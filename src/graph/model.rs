use crate::types::{IssueNode, NodeId, NodeState, RelationshipKind};
use std::collections::HashSet;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use std::collections::HashMap;

pub struct IssueGraph {
    pub graph: StableGraph<IssueNode, RelationshipKind>,
    pub index_map: HashMap<NodeId, NodeIndex>,
    pub milestones: Vec<String>,
    pub labels: Vec<String>,
}

impl IssueGraph {
    pub fn new() -> Self {
        Self {
            graph: StableGraph::new(),
            index_map: HashMap::new(),
            milestones: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: IssueNode) -> NodeIndex {
        if let Some(&idx) = self.index_map.get(&node.id) {
            return idx;
        }
        // ミルストーン/ラベルを収集
        if let Some(ref ms) = node.milestone {
            if !self.milestones.contains(ms) {
                self.milestones.push(ms.clone());
            }
        }
        for label in &node.labels {
            if !self.labels.contains(label) {
                self.labels.push(label.clone());
            }
        }
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.index_map.insert(id, idx);
        idx
    }

    pub fn add_edge(&mut self, from: &NodeId, to: &NodeId, kind: RelationshipKind) -> bool {
        let from_idx = match self.index_map.get(from) {
            Some(&idx) => idx,
            None => return false,
        };
        let to_idx = match self.index_map.get(to) {
            Some(&idx) => idx,
            None => return false,
        };
        // 重複エッジを避ける
        if self
            .graph
            .edges_connecting(from_idx, to_idx)
            .any(|e| *e.weight() == kind)
        {
            return false;
        }
        self.graph.add_edge(from_idx, to_idx, kind);
        true
    }

    pub fn get_node(&self, id: &NodeId) -> Option<&IssueNode> {
        self.index_map
            .get(id)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    pub fn get_node_index(&self, id: &NodeId) -> Option<NodeIndex> {
        self.index_map.get(id).copied()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// 同一マイルストーンのノード間に SameMilestone エッジを追加する
    pub fn add_milestone_edges(&mut self) {
        let milestone_groups: HashMap<String, Vec<NodeId>> = {
            let mut groups: HashMap<String, Vec<NodeId>> = HashMap::new();
            for node in self.graph.node_weights() {
                if let Some(ref ms) = node.milestone {
                    groups.entry(ms.clone()).or_default().push(node.id.clone());
                }
            }
            groups
        };

        for (_ms, ids) in milestone_groups {
            for i in 0..ids.len() {
                for j in (i + 1)..ids.len() {
                    let a = ids[i].clone();
                    let b = ids[j].clone();
                    self.add_edge(&a, &b, RelationshipKind::SameMilestone);
                }
            }
        }
    }

    /// フィルタリングして新しいグラフを返す
    pub fn filter_by_milestone(&self, milestone: &str) -> Self {
        let mut new_graph = Self::new();
        let matching_ids: Vec<NodeId> = self
            .graph
            .node_weights()
            .filter(|n| n.milestone.as_deref() == Some(milestone))
            .map(|n| n.id.clone())
            .collect();

        for id in &matching_ids {
            if let Some(node) = self.get_node(id) {
                new_graph.add_node(node.clone());
            }
        }

        for edge in self.graph.edge_references() {
            let edge: petgraph::stable_graph::EdgeReference<'_, _> = edge;
            let s = self.graph.node_weight(edge.source()).map(|n| n.id.clone());
            let t = self.graph.node_weight(edge.target()).map(|n| n.id.clone());
            if let (Some(s), Some(t)) = (s, t) {
                new_graph.add_edge(&s, &t, edge.weight().clone());
            }
        }
        new_graph
    }

    /// 指定ラベルを持つノードのみ残す
    pub fn filter_by_label(&self, label: &str) -> Self {
        let mut new_graph = Self::new();
        let matching_ids: Vec<NodeId> = self
            .graph
            .node_weights()
            .filter(|n| n.labels.iter().any(|l| l == label))
            .map(|n| n.id.clone())
            .collect();

        for id in &matching_ids {
            if let Some(node) = self.get_node(id) {
                new_graph.add_node(node.clone());
            }
        }
        for edge in self.graph.edge_references() {
            let s = self.graph.node_weight(edge.source()).map(|n| n.id.clone());
            let t = self.graph.node_weight(edge.target()).map(|n| n.id.clone());
            if let (Some(s), Some(t)) = (s, t) {
                if new_graph.index_map.contains_key(&s) && new_graph.index_map.contains_key(&t) {
                    new_graph.add_edge(&s, &t, edge.weight().clone());
                }
            }
        }
        new_graph
    }

    /// 複数ラベルのいずれかを持つノードのみ残す (OR マッチ)
    pub fn filter_by_labels(&self, labels: &HashSet<String>) -> Self {
        if labels.is_empty() {
            return self.clone_graph();
        }
        let mut new_graph = Self::new();
        let matching_ids: Vec<NodeId> = self
            .graph
            .node_weights()
            .filter(|n| n.labels.iter().any(|l| labels.contains(l)))
            .map(|n| n.id.clone())
            .collect();

        for id in &matching_ids {
            if let Some(node) = self.get_node(id) {
                new_graph.add_node(node.clone());
            }
        }
        for edge in self.graph.edge_references() {
            let s = self.graph.node_weight(edge.source()).map(|n| n.id.clone());
            let t = self.graph.node_weight(edge.target()).map(|n| n.id.clone());
            if let (Some(s), Some(t)) = (s, t) {
                if new_graph.index_map.contains_key(&s) && new_graph.index_map.contains_key(&t) {
                    new_graph.add_edge(&s, &t, edge.weight().clone());
                }
            }
        }
        new_graph
    }

    /// グラフ全体をクローンする
    fn clone_graph(&self) -> Self {
        let mut new_graph = Self::new();
        for node in self.graph.node_weights() {
            new_graph.add_node(node.clone());
        }
        for edge in self.graph.edge_references() {
            let s = self.graph.node_weight(edge.source()).map(|n| n.id.clone());
            let t = self.graph.node_weight(edge.target()).map(|n| n.id.clone());
            if let (Some(s), Some(t)) = (s, t) {
                new_graph.add_edge(&s, &t, edge.weight().clone());
            }
        }
        new_graph
    }

    pub fn filter_by_state(&self, states: &[NodeState]) -> Self {
        let mut new_graph = Self::new();
        let matching_ids: Vec<NodeId> = self
            .graph
            .node_weights()
            .filter(|n| states.contains(&n.state))
            .map(|n| n.id.clone())
            .collect();

        for id in &matching_ids {
            if let Some(node) = self.get_node(id) {
                new_graph.add_node(node.clone());
            }
        }
        for edge in self.graph.edge_references() {
            let s = self.graph.node_weight(edge.source()).map(|n| n.id.clone());
            let t = self.graph.node_weight(edge.target()).map(|n| n.id.clone());
            if let (Some(s), Some(t)) = (s, t) {
                if new_graph.index_map.contains_key(&s) && new_graph.index_map.contains_key(&t) {
                    new_graph.add_edge(&s, &t, edge.weight().clone());
                }
            }
        }
        new_graph
    }

    /// 関係性統計を返す
    pub fn relationship_counts(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for edge in self.graph.edge_references() {
            let edge: petgraph::stable_graph::EdgeReference<'_, _> = edge;
            *counts.entry(format!("{}", edge.weight())).or_insert(0) += 1;
        }
        counts
    }

    pub fn all_nodes(&self) -> impl Iterator<Item = &IssueNode> {
        self.graph.node_weights()
    }

    pub fn all_node_indices(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.node_indices()
    }

    /// ノードの隣接ノードと関係性を返す
    pub fn neighbors(&self, id: &NodeId) -> Vec<(&IssueNode, &RelationshipKind)> {
        let idx = match self.index_map.get(id) {
            Some(&i) => i,
            None => return vec![],
        };
        self.graph
            .edges(idx)
            .filter_map(|e: petgraph::stable_graph::EdgeReference<'_, _>| {
                let target_idx = if e.source() == idx {
                    e.target()
                } else {
                    e.source()
                };
                self.graph
                    .node_weight(target_idx)
                    .map(|n| (n, e.weight()))
            })
            .collect()
    }
}

impl Default for IssueGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn make_node(owner: &str, repo: &str, number: u64, kind: NodeKind, state: NodeState) -> IssueNode {
        IssueNode {
            id: NodeId::new(owner, repo, number),
            graphql_id: format!("gid_{}", number),
            kind,
            state,
            title: format!("Issue #{}", number),
            body: String::new(),
            labels: vec![],
            priority: Priority::None,
            milestone: None,
            assignees: vec![],
            url: format!("https://github.com/{}/{}/issues/{}", owner, repo, number),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn add_nodes_and_edges() {
        let mut graph = IssueGraph::new();
        let n1 = make_node("owner", "repo", 1, NodeKind::Issue, NodeState::Open);
        let n2 = make_node("owner", "repo", 2, NodeKind::PullRequest, NodeState::Open);
        let id1 = n1.id.clone();
        let id2 = n2.id.clone();
        graph.add_node(n1);
        graph.add_node(n2);
        assert_eq!(graph.node_count(), 2);
        assert!(graph.add_edge(&id1, &id2, RelationshipKind::ClosingReference));
        assert_eq!(graph.edge_count(), 1);
        // 重複エッジは追加されない
        assert!(!graph.add_edge(&id1, &id2, RelationshipKind::ClosingReference));
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn milestone_edges() {
        let mut graph = IssueGraph::new();
        let mut n1 = make_node("o", "r", 1, NodeKind::Issue, NodeState::Open);
        let mut n2 = make_node("o", "r", 2, NodeKind::Issue, NodeState::Open);
        let mut n3 = make_node("o", "r", 3, NodeKind::Issue, NodeState::Open);
        n1.milestone = Some("v1.0".to_string());
        n2.milestone = Some("v1.0".to_string());
        n3.milestone = Some("v2.0".to_string());
        graph.add_node(n1);
        graph.add_node(n2);
        graph.add_node(n3);
        graph.add_milestone_edges();
        assert_eq!(graph.edge_count(), 1); // only #1--#2
    }
}
