use crate::graph::model::IssueGraph;
use crate::types::{NodeId, NodeKind, NodeState, Priority, RelationshipKind};
use anyhow::Result;
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use petgraph::Direction;
use serde_json::{json, Value};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub enum OutputFormat {
    List,
    Tree,
    Json,
    Dot,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "list" => Ok(Self::List),
            "tree" => Ok(Self::Tree),
            "json" => Ok(Self::Json),
            "dot" => Ok(Self::Dot),
            other => Err(format!("Unknown format: {}", other)),
        }
    }
}

/// グラフを JSON 形式にシリアライズする
pub fn to_json(graph: &IssueGraph, fields: Option<&str>) -> Result<Value> {
    let nodes: Vec<Value> = graph
        .all_nodes()
        .map(|n| {
            json!({
                "id": n.id.to_string(),
                "number": n.id.number,
                "kind": match n.kind { NodeKind::Issue => "issue", NodeKind::PullRequest => "pull_request" },
                "state": match n.state {
                    NodeState::Open => "open",
                    NodeState::Closed => "closed",
                    NodeState::Merged => "merged",
                    NodeState::Draft => "draft",
                },
                "title": n.title,
                "labels": n.labels,
                "priority": match n.priority {
                    Priority::Critical => "critical",
                    Priority::High     => "high",
                    Priority::Medium   => "medium",
                    Priority::Low      => "low",
                    Priority::None     => "",
                },
                "milestone": n.milestone,
                "assignees": n.assignees,
                "url": n.url,
                "repo": n.id.repo_full(),
            })
        })
        .collect();

    let edges: Vec<Value> = graph
        .graph
        .edge_references()
        .filter_map(|e: petgraph::stable_graph::EdgeReference<'_, _>| {
            let src = graph.graph.node_weight(e.source())?;
            let tgt = graph.graph.node_weight(e.target())?;
            Some(json!({
                "source": src.id.to_string(),
                "target": tgt.id.to_string(),
                "kind": e.weight().to_string(),
            }))
        })
        .collect();

    let counts = graph.relationship_counts();
    let result = json!({
        "nodes": nodes,
        "edges": edges,
        "stats": {
            "node_count": graph.node_count(),
            "edge_count": graph.edge_count(),
            "relationship_counts": counts,
        }
    });

    // フィールド選択
    if let Some(field_str) = fields {
        if !field_str.is_empty() {
            let selected_fields: Vec<&str> = field_str.split(',').collect();
            let mut filtered = serde_json::Map::new();
            if let Some(obj) = result.as_object() {
                for field in selected_fields {
                    let field = field.trim();
                    if let Some(val) = obj.get(field) {
                        filtered.insert(field.to_string(), val.clone());
                    }
                }
            }
            return Ok(Value::Object(filtered));
        }
    }

    Ok(result)
}

/// ツリー形式で出力する (focus_issue が起点、親子関係を表示)
pub fn to_tree(graph: &IssueGraph, focus: Option<&NodeId>) -> String {
    let mut output = String::new();

    // 起点ノードを決定
    let root_indices: Vec<NodeIndex> = if let Some(focus_id) = focus {
        if let Some(idx) = graph.get_node_index(focus_id) {
            vec![idx]
        } else {
            graph.all_node_indices().collect()
        }
    } else {
        // 親がないノードをルートにする
        graph
            .all_node_indices()
            .filter(|&idx| {
                !graph
                    .graph
                    .edges_directed(idx, Direction::Incoming)
                    .any(|e: petgraph::stable_graph::EdgeReference<'_, _>| *e.weight() == RelationshipKind::ParentChild)
            })
            .collect()
    };

    let mut visited = HashSet::new();
    for root in root_indices {
        render_tree_node(graph, root, "", true, &mut visited, &mut output);
    }

    if output.is_empty() {
        output.push_str("(no nodes)\n");
    }
    output
}

fn render_tree_node(
    graph: &IssueGraph,
    idx: NodeIndex,
    prefix: &str,
    is_last: bool,
    visited: &mut HashSet<NodeIndex>,
    output: &mut String,
) {
    if visited.contains(&idx) {
        return;
    }
    visited.insert(idx);

    let connector = if is_last { "└── " } else { "├── " };
    let child_prefix = if is_last { "    " } else { "│   " };

    if let Some(node) = graph.graph.node_weight(idx) {
        let state_icon = match node.state {
            NodeState::Open => "●",
            NodeState::Closed => "○",
            NodeState::Merged => "⬡",
            NodeState::Draft => "◌",
        };
        let kind_prefix = match node.kind {
            NodeKind::Issue => "",
            NodeKind::PullRequest => "PR ",
        };
        let milestone = node
            .milestone
            .as_deref()
            .map(|m| format!(" [{}]", m))
            .unwrap_or_default();
        let prio = node.priority.icon();
        let prio_str = if prio.is_empty() { String::new() } else { format!(" {}", prio) };
        let label_str = if node.labels.is_empty() {
            String::new()
        } else {
            format!(" ({})", node.labels.join(", "))
        };
        output.push_str(&format!(
            "{}{}{} {}#{} {}{}{}{}\n",
            prefix,
            connector,
            state_icon,
            kind_prefix,
            node.id.number,
            node.title,
            milestone,
            prio_str,
            label_str,
        ));

        // ParentChild 関係の子ノードを再帰的に表示
        let children: Vec<NodeIndex> = graph
            .graph
            .edges_directed(idx, Direction::Outgoing)
            .filter(|e: &petgraph::stable_graph::EdgeReference<'_, _>| *e.weight() == RelationshipKind::ParentChild)
            .map(|e: petgraph::stable_graph::EdgeReference<'_, _>| e.target())
            .collect();

        for (i, child_idx) in children.iter().enumerate() {
            let is_last_child = i == children.len() - 1;
            render_tree_node(
                graph,
                *child_idx,
                &format!("{}{}", prefix, child_prefix),
                is_last_child,
                visited,
                output,
            );
        }

        // 参照関係をフラット表示
        let refs: Vec<String> = graph
            .graph
            .edges_directed(idx, Direction::Outgoing)
            .filter(|e: &petgraph::stable_graph::EdgeReference<'_, _>| {
                !matches!(
                    e.weight(),
                    RelationshipKind::ParentChild | RelationshipKind::SameMilestone
                )
            })
            .filter_map(|e: petgraph::stable_graph::EdgeReference<'_, _>| {
                graph.graph.node_weight(e.target()).map(|n| {
                    format!("{}#{}", match n.kind { NodeKind::PullRequest => "PR ", _ => "" }, n.id.number)
                })
            })
            .collect();

        if !refs.is_empty() {
            output.push_str(&format!(
                "{}{}refs: {}\n",
                prefix,
                child_prefix,
                refs.join(", ")
            ));
        }
    }
}

/// リスト形式で出力する
pub fn to_list(graph: &IssueGraph) -> String {
    let mut lines: Vec<String> = graph
        .all_nodes()
        .map(|n| {
            let state = match n.state {
                NodeState::Open => "open",
                NodeState::Closed => "closed",
                NodeState::Merged => "merged",
                NodeState::Draft => "draft",
            };
            let kind = match n.kind {
                NodeKind::Issue => "",
                NodeKind::PullRequest => "PR ",
            };
            let neighbors = graph.neighbors(&n.id);
            let refs: Vec<String> = neighbors
                .iter()
                .filter(|(_, kind)| !matches!(kind, RelationshipKind::SameMilestone))
                .map(|(n, kind)| format!("{}#{} ({})", match n.kind { NodeKind::PullRequest => "PR ", _ => "" }, n.id.number, kind))
                .collect();

            let ref_str = if refs.is_empty() {
                String::new()
            } else {
                format!(" → {}", refs.join(", "))
            };

            // 優先度インジケータ
            let prio = n.priority.icon();
            let prio_str = if prio.is_empty() { String::new() } else { format!(" {}", prio) };

            // ラベル
            let label_str = if n.labels.is_empty() {
                String::new()
            } else {
                format!(" ({})", n.labels.join(", "))
            };

            format!(
                "{}#{} {} [{}]{}{}{}",
                kind, n.id.number, n.title, state, prio_str, label_str, ref_str
            )
        })
        .collect();

    lines.sort();
    lines.join("\n")
}

/// Graphviz DOT 形式で出力する
pub fn to_dot(graph: &IssueGraph) -> String {
    let mut dot = String::from("digraph issue_graph {\n");
    dot.push_str("  node [shape=box fontname=\"monospace\"];\n");
    dot.push_str("  rankdir=LR;\n\n");

    for node in graph.all_nodes() {
        let label = format!(
            "{}#{}: {}",
            match node.kind { NodeKind::PullRequest => "PR ", _ => "" },
            node.id.number,
            node.title.replace('"', "'")
        );
        let color = match node.state {
            NodeState::Open => "green",
            NodeState::Closed => "red",
            NodeState::Merged => "purple",
            NodeState::Draft => "yellow",
        };
        dot.push_str(&format!(
            "  \"{}\" [label=\"{}\" color={} style=filled fillcolor=\"{}22\"];\n",
            node.id, label, color, color
        ));
    }

    dot.push('\n');

    for edge in graph.graph.edge_references() {
        let edge: petgraph::stable_graph::EdgeReference<'_, _> = edge;
        if let (Some(src), Some(tgt)) = (
            graph.graph.node_weight(edge.source()),
            graph.graph.node_weight(edge.target()),
        ) {
            let (style, color) = match edge.weight() {
                RelationshipKind::ParentChild => ("solid", "blue"),
                RelationshipKind::ClosingReference => ("dashed", "green"),
                RelationshipKind::CrossReference => ("dotted", "gray"),
                RelationshipKind::ConnectedEvent => ("dashed", "cyan"),
                RelationshipKind::BodyMention => ("dotted", "orange"),
                RelationshipKind::SameMilestone => ("dotted", "lightgray"),
                RelationshipKind::Duplicate => ("dotted", "red"),
            };
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\" style={} color={}];\n",
                src.id,
                tgt.id,
                edge.weight(),
                style,
                color
            ));
        }
    }

    dot.push_str("}\n");
    dot
}
