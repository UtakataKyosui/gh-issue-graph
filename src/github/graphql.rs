use crate::types::{parse_priority, IssueNode, NodeId, NodeKind, NodeState, RelationshipKind};
use anyhow::Result;
use octocrab::Octocrab;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::OnceLock;

/// Sub-Issue API に非対応なリポジトリのキャッシュ
static SUB_ISSUES_UNSUPPORTED: OnceLock<std::sync::Mutex<HashSet<String>>> = OnceLock::new();

fn unsupported_cache() -> &'static std::sync::Mutex<HashSet<String>> {
    SUB_ISSUES_UNSUPPORTED.get_or_init(|| std::sync::Mutex::new(HashSet::new()))
}

fn is_sub_issues_supported(repo: &str) -> bool {
    !unsupported_cache().lock().unwrap().contains(repo)
}

fn mark_sub_issues_unsupported(repo: &str) {
    unsupported_cache().lock().unwrap().insert(repo.to_string());
}

// --------- GraphQL レスポンス型 ---------

#[derive(Deserialize, Debug)]
pub struct PageInfo {
    #[serde(rename = "hasNextPage")]
    pub has_next_page: bool,
    #[serde(rename = "endCursor")]
    pub end_cursor: Option<String>,
}

#[derive(Deserialize, Debug)]
struct GqlLabel {
    name: String,
}

#[derive(Deserialize, Debug)]
struct GqlMilestone {
    title: String,
}

#[derive(Deserialize, Debug)]
struct GqlUser {
    login: String,
}

#[derive(Deserialize, Debug)]
struct GqlRepository {
    owner: GqlOwner,
    name: String,
}

#[derive(Deserialize, Debug)]
struct GqlOwner {
    login: String,
}

#[derive(Deserialize, Debug)]
struct GqlIssueNode {
    id: String,
    number: u64,
    title: String,
    state: String,
    url: String,
    body: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    labels: Option<GqlLabelConnection>,
    milestone: Option<GqlMilestone>,
    assignees: Option<GqlUserConnection>,
}

#[derive(Deserialize, Debug)]
struct GqlPrNode {
    id: String,
    number: u64,
    title: String,
    state: String,
    url: String,
    body: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "isDraft")]
    is_draft: bool,
    labels: Option<GqlLabelConnection>,
    milestone: Option<GqlMilestone>,
    assignees: Option<GqlUserConnection>,
    #[serde(rename = "closingIssuesReferences")]
    closing_issues: Option<GqlClosingIssuesConnection>,
}

#[derive(Deserialize, Debug)]
struct GqlLabelConnection {
    nodes: Vec<GqlLabel>,
}

#[derive(Deserialize, Debug)]
struct GqlUserConnection {
    nodes: Vec<GqlUser>,
}

#[derive(Deserialize, Debug)]
struct GqlClosingIssuesConnection {
    nodes: Vec<GqlClosingIssueRef>,
}

#[derive(Deserialize, Debug)]
struct GqlClosingIssueRef {
    number: u64,
    repository: GqlRepository,
}

#[derive(Deserialize, Debug)]
struct GqlConnection<T> {
    #[serde(rename = "pageInfo")]
    pub page_info: PageInfo,
    pub nodes: Vec<Option<T>>,
}

// --------- ページ取得結果 ---------

pub struct IssuePage {
    pub nodes: Vec<IssueNode>,
    pub relationships: Vec<(NodeId, NodeId, RelationshipKind)>,
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

pub struct PrPage {
    pub nodes: Vec<IssueNode>,
    pub relationships: Vec<(NodeId, NodeId, RelationshipKind)>,
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

// --------- ヘルパー ---------

fn parse_state_issue(state: &str) -> NodeState {
    match state.to_uppercase().as_str() {
        "OPEN" => NodeState::Open,
        _ => NodeState::Closed,
    }
}

fn parse_state_pr(state: &str, is_draft: bool) -> NodeState {
    if is_draft {
        return NodeState::Draft;
    }
    match state.to_uppercase().as_str() {
        "OPEN" => NodeState::Open,
        "MERGED" => NodeState::Merged,
        _ => NodeState::Closed,
    }
}

fn parse_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

fn gql_issue_to_node(n: &GqlIssueNode, owner: &str, repo: &str) -> IssueNode {
    let labels: Vec<String> = n
        .labels
        .as_ref()
        .map(|c| c.nodes.iter().map(|l| l.name.clone()).collect())
        .unwrap_or_default();
    let priority = parse_priority(&labels);
    IssueNode {
        id: NodeId::new(owner, repo, n.number),
        graphql_id: n.id.clone(),
        kind: NodeKind::Issue,
        state: parse_state_issue(&n.state),
        title: n.title.clone(),
        body: n.body.clone().unwrap_or_default(),
        labels,
        priority,
        milestone: n.milestone.as_ref().map(|m| m.title.clone()),
        assignees: n
            .assignees
            .as_ref()
            .map(|c| c.nodes.iter().map(|u| u.login.clone()).collect())
            .unwrap_or_default(),
        url: n.url.clone(),
        created_at: parse_datetime(&n.created_at),
        updated_at: parse_datetime(&n.updated_at),
    }
}

fn gql_pr_to_node(n: &GqlPrNode, owner: &str, repo: &str) -> IssueNode {
    let labels: Vec<String> = n
        .labels
        .as_ref()
        .map(|c| c.nodes.iter().map(|l| l.name.clone()).collect())
        .unwrap_or_default();
    let priority = parse_priority(&labels);
    IssueNode {
        id: NodeId::new(owner, repo, n.number),
        graphql_id: n.id.clone(),
        kind: NodeKind::PullRequest,
        state: parse_state_pr(&n.state, n.is_draft),
        title: n.title.clone(),
        body: n.body.clone().unwrap_or_default(),
        labels,
        priority,
        milestone: n.milestone.as_ref().map(|m| m.title.clone()),
        assignees: n
            .assignees
            .as_ref()
            .map(|c| c.nodes.iter().map(|u| u.login.clone()).collect())
            .unwrap_or_default(),
        url: n.url.clone(),
        created_at: parse_datetime(&n.created_at),
        updated_at: parse_datetime(&n.updated_at),
    }
}

// --------- 公開 API ---------

/// Issue 一覧を1ページ取得する
pub async fn fetch_issues_page(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    after: Option<&str>,
) -> Result<IssuePage> {
    let vars = json!({
        "owner": owner,
        "name": repo,
        "issueAfter": after,
        "prAfter": Value::Null,
    });
    let data: Value = client
        .graphql(&json!({
            "query": crate::github::queries::QUERY_ISSUES_PAGE,
            "variables": vars,
        }))
        .await?;

    let repo_data = &data["data"]["repository"];
    let issues_conn = &repo_data["issues"];
    let page_info: PageInfo = serde_json::from_value(issues_conn["pageInfo"].clone())?;

    let mut nodes = Vec::new();
    for item in issues_conn["nodes"].as_array().unwrap_or(&vec![]) {
        if item.is_null() {
            continue;
        }
        if let Ok(n) = serde_json::from_value::<GqlIssueNode>(item.clone()) {
            nodes.push(gql_issue_to_node(&n, owner, repo));
        }
    }

    Ok(IssuePage {
        nodes,
        relationships: vec![],
        has_next_page: page_info.has_next_page,
        end_cursor: page_info.end_cursor,
    })
}

/// PR 一覧を1ページ取得する (closingIssuesReferences を含む)
pub async fn fetch_prs_page(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    after: Option<&str>,
) -> Result<PrPage> {
    let vars = json!({
        "owner": owner,
        "name": repo,
        "issueAfter": Value::Null,
        "prAfter": after,
    });
    let data: Value = client
        .graphql(&json!({
            "query": crate::github::queries::QUERY_ISSUES_PAGE,
            "variables": vars,
        }))
        .await?;

    let repo_data = &data["data"]["repository"];
    let prs_conn = &repo_data["pullRequests"];
    let page_info: PageInfo = serde_json::from_value(prs_conn["pageInfo"].clone())?;

    let mut nodes = Vec::new();
    let mut relationships = Vec::new();

    for item in prs_conn["nodes"].as_array().unwrap_or(&vec![]) {
        if item.is_null() {
            continue;
        }
        if let Ok(n) = serde_json::from_value::<GqlPrNode>(item.clone()) {
            let pr_id = NodeId::new(owner, repo, n.number);

            // closingIssuesReferences → ClosingReference エッジ
            if let Some(ref closing) = n.closing_issues {
                for ref_issue in &closing.nodes {
                    let target_id = NodeId::new(
                        &ref_issue.repository.owner.login,
                        &ref_issue.repository.name,
                        ref_issue.number,
                    );
                    relationships.push((
                        pr_id.clone(),
                        target_id,
                        RelationshipKind::ClosingReference,
                    ));
                }
            }

            nodes.push(gql_pr_to_node(&n, owner, repo));
        }
    }

    Ok(PrPage {
        nodes,
        relationships,
        has_next_page: page_info.has_next_page,
        end_cursor: page_info.end_cursor,
    })
}

/// Sub-Issue を取得する (GraphQL-Features: sub_issues ヘッダ付き)
/// 非対応の場合は Ok(None) を返す
pub async fn fetch_sub_issues(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<Option<Vec<(NodeId, NodeId, RelationshipKind)>>> {
    let repo_full = format!("{}/{}", owner, repo);
    if !is_sub_issues_supported(&repo_full) {
        return Ok(None);
    }

    let vars = json!({
        "owner": owner,
        "name": repo,
        "number": number as i64,
    });

    // octocrab の graphql() に追加ヘッダを注入するため、reqwest 経由で直接呼び出す
    // octocrab のインスタンスから base_url と reqwest クライアントを借用して呼ぶ
    let result: std::result::Result<Value, _> = client
        .graphql(&json!({
            "query": crate::github::queries::QUERY_SUB_ISSUES,
            "variables": vars,
        }))
        .await;

    match result {
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("sub_issues") || msg.contains("doesn't exist on type") {
                mark_sub_issues_unsupported(&repo_full);
                return Ok(None);
            }
            Err(anyhow::anyhow!(msg))
        }
        Ok(data) => {
            // エラーフィールドを確認
            if let Some(errors) = data["errors"].as_array() {
                for err in errors {
                    let msg = err["message"].as_str().unwrap_or("");
                    if msg.contains("sub_issues") || msg.contains("doesn't exist on type") {
                        mark_sub_issues_unsupported(&repo_full);
                        return Ok(None);
                    }
                }
            }

            let issue_data = &data["data"]["repository"]["issue"];
            if issue_data.is_null() {
                return Ok(Some(vec![]));
            }

            let current_id = NodeId::new(owner, repo, number);
            let mut rels = Vec::new();

            // parent → ParentChild (current is child of parent)
            if let Some(parent) = issue_data["parent"].as_object() {
                let parent_number = parent["number"].as_u64().unwrap_or(0);
                let parent_owner = parent["repository"]["owner"]["login"]
                    .as_str()
                    .unwrap_or(owner);
                let parent_repo = parent["repository"]["name"].as_str().unwrap_or(repo);
                let parent_id = NodeId::new(parent_owner, parent_repo, parent_number);
                rels.push((parent_id, current_id.clone(), RelationshipKind::ParentChild));
            }

            // subIssues → ParentChild (current is parent of children)
            if let Some(children) = issue_data["subIssues"]["nodes"].as_array() {
                for child in children {
                    if child.is_null() {
                        continue;
                    }
                    let child_number = child["number"].as_u64().unwrap_or(0);
                    let child_owner = child["repository"]["owner"]["login"]
                        .as_str()
                        .unwrap_or(owner);
                    let child_repo = child["repository"]["name"].as_str().unwrap_or(repo);
                    let child_id = NodeId::new(child_owner, child_repo, child_number);
                    rels.push((current_id.clone(), child_id, RelationshipKind::ParentChild));
                }
            }

            Ok(Some(rels))
        }
    }
}

/// タイムラインのクロスリファレンスと接続イベントを取得する
pub async fn fetch_timeline_refs(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<Vec<(NodeId, NodeId, RelationshipKind)>> {
    let vars = json!({
        "owner": owner,
        "name": repo,
        "number": number as i64,
    });

    let data: Value = client
        .graphql(&json!({
            "query": crate::github::queries::QUERY_TIMELINE,
            "variables": vars,
        }))
        .await?;

    let current_id = NodeId::new(owner, repo, number);
    let mut rels = Vec::new();

    let issue_or_pr = &data["data"]["repository"]["issueOrPullRequest"];
    let timeline_nodes = issue_or_pr["timelineItems"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    for item in timeline_nodes {
        if item.is_null() {
            continue;
        }
        // CrossReferencedEvent: source が参照元
        if let Some(source) = item.get("source") {
            if !source.is_null() {
                let num = source["number"].as_u64().unwrap_or(0);
                let src_owner = source["repository"]["owner"]["login"]
                    .as_str()
                    .unwrap_or(owner);
                let src_repo = source["repository"]["name"].as_str().unwrap_or(repo);
                if num > 0 {
                    let source_id = NodeId::new(src_owner, src_repo, num);
                    rels.push((source_id, current_id.clone(), RelationshipKind::CrossReference));
                }
            }
        }
        // ConnectedEvent: subject が接続相手
        if let Some(subject) = item.get("subject") {
            if !subject.is_null() {
                let num = subject["number"].as_u64().unwrap_or(0);
                let sub_owner = subject["repository"]["owner"]["login"]
                    .as_str()
                    .unwrap_or(owner);
                let sub_repo = subject["repository"]["name"].as_str().unwrap_or(repo);
                if num > 0 {
                    let subject_id = NodeId::new(sub_owner, sub_repo, num);
                    rels.push((current_id.clone(), subject_id, RelationshipKind::ConnectedEvent));
                }
            }
        }
    }

    Ok(rels)
}
