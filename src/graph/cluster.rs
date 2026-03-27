use crate::graph::model::IssueGraph;
use petgraph::stable_graph::NodeIndex;
use std::collections::HashMap;

/// グラフのノードにクラスタ ID を割り当てる
/// 同一マイルストーン: 同じ ID を持つ
/// マイルストーンがない: 0 (未分類)
pub fn build_milestone_clusters(graph: &IssueGraph) -> HashMap<NodeIndex, usize> {
    let mut milestone_ids: HashMap<String, usize> = HashMap::new();
    let mut next_id = 1usize;

    let mut clusters = HashMap::new();
    for (_node_id_in_map, &idx) in &graph.index_map {
        if let Some(node) = graph.graph.node_weight(idx) {
            let cluster_id = if let Some(ref ms) = node.milestone {
                *milestone_ids.entry(ms.clone()).or_insert_with(|| {
                    let id = next_id;
                    next_id += 1;
                    id
                })
            } else {
                0
            };
            clusters.insert(idx, cluster_id);
        }
    }
    clusters
}

/// ラベルベースのクラスタリング (最初のラベルを使用)
pub fn build_label_clusters(graph: &IssueGraph) -> HashMap<NodeIndex, usize> {
    let mut label_ids: HashMap<String, usize> = HashMap::new();
    let mut next_id = 1usize;

    let mut clusters = HashMap::new();
    for (_node_id, &idx) in &graph.index_map {
        if let Some(node) = graph.graph.node_weight(idx) {
            let cluster_id = if let Some(first_label) = node.labels.first() {
                *label_ids.entry(first_label.clone()).or_insert_with(|| {
                    let id = next_id;
                    next_id += 1;
                    id
                })
            } else {
                0
            };
            clusters.insert(idx, cluster_id);
        }
    }
    clusters
}
