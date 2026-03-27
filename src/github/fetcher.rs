use crate::graph::model::IssueGraph;
use crate::types::{NodeId, RelationshipKind};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use octocrab::Octocrab;
use regex::Regex;
use std::collections::{HashSet, VecDeque};

/// フェッチ設定
pub struct FetchConfig {
    pub owner: String,
    pub repo: String,
    /// BFS 探索の最大深度
    pub max_depth: usize,
    /// 特定 Issue にフォーカスする場合
    pub focus_issue: Option<u64>,
    /// タイムライン参照を取得するか
    pub fetch_timeline: bool,
    /// Sub-Issue を取得するか
    pub fetch_sub_issues: bool,
}

impl FetchConfig {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            max_depth: 2,
            focus_issue: None,
            fetch_timeline: true,
            fetch_sub_issues: true,
        }
    }
}

/// 本文テキストから #N 参照を抽出する
fn extract_body_mentions(body: &str, owner: &str, repo: &str) -> Vec<NodeId> {
    let re = Regex::new(r"(?:^|[^/&])#(\d+)").unwrap();
    re.captures_iter(body)
        .filter_map(|cap| cap[1].parse::<u64>().ok())
        .map(|n| NodeId::new(owner, repo, n))
        .collect()
}

/// "Duplicate of #N" パターンを検出する
fn detect_duplicate(body: &str, owner: &str, repo: &str) -> Option<NodeId> {
    let re = Regex::new(r"(?i)duplicate\s+of\s+#(\d+)").unwrap();
    re.captures(body)
        .and_then(|cap| cap[1].parse::<u64>().ok())
        .map(|n| NodeId::new(owner, repo, n))
}

/// 1 ノードの sub-issues + timeline を並列フェッチして関係リストを返す
async fn fetch_relations_for_node(
    client: &Octocrab,
    fetch_sub_issues: bool,
    fetch_timeline: bool,
    node_id: &NodeId,
) -> Vec<(NodeId, NodeId, RelationshipKind)> {
    let owner = &node_id.owner;
    let repo = &node_id.repo;
    let number = node_id.number;

    // sub-issues と timeline を tokio::join! で同時実行
    let (sub_result, timeline_result) = tokio::join!(
        async {
            if fetch_sub_issues {
                crate::github::graphql::fetch_sub_issues(client, owner, repo, number)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or_default()
            } else {
                vec![]
            }
        },
        async {
            if fetch_timeline {
                crate::github::graphql::fetch_timeline_refs(client, owner, repo, number)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    );

    let mut rels = sub_result;
    rels.extend(timeline_result);
    rels
}

/// Issue/PR 一覧と関係性を取得して IssueGraph を構築する
pub async fn build_graph(client: &Octocrab, config: &FetchConfig) -> Result<IssueGraph> {
    let mut graph = IssueGraph::new();
    let owner = &config.owner;
    let repo = &config.repo;

    // Step 1: シード取得
    if let Some(focus) = config.focus_issue {
        // 特定 Issue にフォーカスする場合は BFS で深度制限
        let seed_id = NodeId::new(owner.clone(), repo.clone(), focus);
        fetch_bfs(client, config, &mut graph, seed_id).await?;
    } else {
        // リポジトリ全体の Issue/PR を取得
        fetch_all_issues(client, owner, repo, &mut graph).await?;
        fetch_all_prs(client, owner, repo, &mut graph).await?;

        // Step 2: 各ノードの関係性を並列取得
        let all_ids: Vec<NodeId> = graph.index_map.keys().cloned().collect();

        // buffer_unordered で最大 CONCURRENCY ノードを並列フェッチ
        const CONCURRENCY: usize = 8;
        let fetch_sub = config.fetch_sub_issues;
        let fetch_tl = config.fetch_timeline;

        let all_rels: Vec<Vec<(NodeId, NodeId, RelationshipKind)>> = stream::iter(all_ids)
            .map(|node_id| {
                let client = client.clone();
                async move {
                    fetch_relations_for_node(&client, fetch_sub, fetch_tl, &node_id).await
                }
            })
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await;

        // 収集した関係をグラフに一括追加
        for rels in all_rels {
            for (src, tgt, kind) in rels {
                graph.add_edge(&src, &tgt, kind);
            }
        }
    }

    // Step 3: 後処理
    add_body_mention_edges(&mut graph, owner, repo);
    graph.add_milestone_edges();

    Ok(graph)
}

/// 全 Issue をページネーションで取得する
async fn fetch_all_issues(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    graph: &mut IssueGraph,
) -> Result<()> {
    let mut cursor: Option<String> = None;
    loop {
        let page = crate::github::graphql::fetch_issues_page(
            client,
            owner,
            repo,
            cursor.as_deref(),
        )
        .await?;

        for node in page.nodes {
            graph.add_node(node);
        }

        if !page.has_next_page {
            break;
        }
        cursor = page.end_cursor;
    }
    Ok(())
}

/// 全 PR をページネーションで取得する (closingIssuesReferences も含む)
async fn fetch_all_prs(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    graph: &mut IssueGraph,
) -> Result<()> {
    let mut cursor: Option<String> = None;
    loop {
        let page = crate::github::graphql::fetch_prs_page(
            client,
            owner,
            repo,
            cursor.as_deref(),
        )
        .await?;

        for node in page.nodes {
            graph.add_node(node);
        }
        for (src, tgt, kind) in page.relationships {
            graph.add_edge(&src, &tgt, kind);
        }

        if !page.has_next_page {
            break;
        }
        cursor = page.end_cursor;
    }
    Ok(())
}

/// BFS で特定 Issue を起点に depth まで探索する
async fn fetch_bfs(
    client: &Octocrab,
    config: &FetchConfig,
    graph: &mut IssueGraph,
    start: NodeId,
) -> Result<()> {
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut queue: VecDeque<(NodeId, usize)> = VecDeque::new();
    queue.push_back((start, 0));

    while let Some((node_id, depth)) = queue.pop_front() {
        if visited.contains(&node_id) {
            continue;
        }
        visited.insert(node_id.clone());

        // ノードをグラフに追加 (まだなければフェッチ)
        if graph.get_node(&node_id).is_none() {
            let page = crate::github::graphql::fetch_issues_page(
                client,
                &node_id.owner,
                &node_id.repo,
                None,
            )
            .await?;
            for node in page.nodes {
                graph.add_node(node);
            }
        }

        if depth < config.max_depth {
            let rels = fetch_relations_for_node(
                client,
                config.fetch_sub_issues,
                config.fetch_timeline,
                &node_id,
            )
            .await;
            for (src, tgt, kind) in rels {
                graph.add_edge(&src, &tgt, kind.clone());
                if !visited.contains(&tgt) {
                    queue.push_back((tgt, depth + 1));
                }
                if !visited.contains(&src) {
                    queue.push_back((src, depth + 1));
                }
            }
        }
    }
    Ok(())
}

/// グラフ内の全ノードの本文から BodyMention エッジを追加する
fn add_body_mention_edges(graph: &mut IssueGraph, owner: &str, repo: &str) {
    let nodes: Vec<(NodeId, String)> = graph
        .graph
        .node_weights()
        .map(|n| (n.id.clone(), n.body.clone()))
        .collect();

    for (node_id, body) in nodes {
        let mentions = extract_body_mentions(&body, owner, repo);
        for target_id in mentions {
            if target_id != node_id && graph.get_node(&target_id).is_some() {
                graph.add_edge(&node_id, &target_id, RelationshipKind::BodyMention);
            }
        }
        if let Some(dup_id) = detect_duplicate(&body, owner, repo) {
            if dup_id != node_id && graph.get_node(&dup_id).is_some() {
                graph.add_edge(&node_id, &dup_id, RelationshipKind::Duplicate);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_body_mentions() {
        let body = "This is related to #123 and also #456.\nSee #789.";
        let mentions = extract_body_mentions(body, "owner", "repo");
        let numbers: Vec<u64> = mentions.iter().map(|id| id.number).collect();
        assert!(numbers.contains(&123));
        assert!(numbers.contains(&456));
        assert!(numbers.contains(&789));
    }

    #[test]
    fn test_detect_duplicate() {
        let body = "Duplicate of #42";
        let dup = detect_duplicate(body, "owner", "repo");
        assert_eq!(dup.map(|id| id.number), Some(42));

        let body2 = "No duplicate here";
        assert!(detect_duplicate(body2, "owner", "repo").is_none());
    }
}
