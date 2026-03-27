use petgraph::stable_graph::NodeIndex;
use rand::Rng;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub struct GraphLayout {
    pub positions: HashMap<NodeIndex, Position>,
    pub width: f64,
    pub height: f64,
}

impl GraphLayout {
    pub fn new(width: f64, height: f64) -> Self {
        Self {
            positions: HashMap::new(),
            width,
            height,
        }
    }

    /// 全ノードを (0,0)-(width,height) の範囲内にランダム配置する
    pub fn initialize_random(&mut self, node_indices: &[NodeIndex]) {
        let mut rng = rand::thread_rng();
        for &idx in node_indices {
            self.positions.insert(
                idx,
                Position {
                    x: rng.gen_range(0.1 * self.width..0.9 * self.width),
                    y: rng.gen_range(0.1 * self.height..0.9 * self.height),
                },
            );
        }
    }

    /// ノードを bounds 内にクランプする
    fn clamp(&self, pos: &mut Position) {
        pos.x = pos.x.clamp(0.0, self.width);
        pos.y = pos.y.clamp(0.0, self.height);
    }

    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        if self.positions.is_empty() {
            return (0.0, 0.0, self.width, self.height);
        }
        let xs: Vec<f64> = self.positions.values().map(|p| p.x).collect();
        let ys: Vec<f64> = self.positions.values().map(|p| p.y).collect();
        let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (min_x, min_y, max_x, max_y)
    }
}

/// Fruchterman-Reingold 力学モデルのパラメータ
pub struct FRConfig {
    /// イテレーション回数
    pub iterations: usize,
    /// クラスタ引力の強さ (0.0-1.0)
    pub cluster_strength: f64,
}

impl Default for FRConfig {
    fn default() -> Self {
        Self {
            iterations: 300,
            cluster_strength: 0.05,
        }
    }
}

/// Fruchterman-Reingold アルゴリズムでレイアウトを計算する
///
/// - `layout`: 初期位置が設定済みの GraphLayout (in/out)
/// - `edges`: (source_idx, target_idx) のエッジリスト
/// - `clusters`: ノードごとのクラスタ ID (同一 ID のノードは引き合う)
/// - `config`: アルゴリズムパラメータ
pub fn compute_layout(
    layout: &mut GraphLayout,
    edges: &[(NodeIndex, NodeIndex)],
    clusters: &HashMap<NodeIndex, usize>,
    config: &FRConfig,
) {
    let area = layout.width * layout.height;
    let n = layout.positions.len();
    if n == 0 {
        return;
    }
    let k = (area / n as f64).sqrt();
    let mut temperature = layout.width / 10.0;
    let cooling = temperature / config.iterations as f64;

    let node_indices: Vec<NodeIndex> = layout.positions.keys().cloned().collect();

    for _iter in 0..config.iterations {
        let mut displacements: HashMap<NodeIndex, (f64, f64)> = node_indices
            .iter()
            .map(|&idx| (idx, (0.0, 0.0)))
            .collect();

        // 1. 斥力 (全ノードペア)
        for i in 0..node_indices.len() {
            for j in (i + 1)..node_indices.len() {
                let ui = node_indices[i];
                let vj = node_indices[j];
                let pos_u = &layout.positions[&ui];
                let pos_v = &layout.positions[&vj];
                let dx = pos_u.x - pos_v.x;
                let dy = pos_u.y - pos_v.y;
                let distance = (dx * dx + dy * dy).sqrt().max(0.01);
                let force = k * k / distance;
                let fx = dx / distance * force;
                let fy = dy / distance * force;
                displacements.get_mut(&ui).map(|d| { d.0 += fx; d.1 += fy; });
                displacements.get_mut(&vj).map(|d| { d.0 -= fx; d.1 -= fy; });
            }
        }

        // 2. 引力 (エッジのみ)
        for &(src, tgt) in edges {
            if !layout.positions.contains_key(&src) || !layout.positions.contains_key(&tgt) {
                continue;
            }
            let pos_s = &layout.positions[&src];
            let pos_t = &layout.positions[&tgt];
            let dx = pos_s.x - pos_t.x;
            let dy = pos_s.y - pos_t.y;
            let distance = (dx * dx + dy * dy).sqrt().max(0.01);
            let force = distance * distance / k;
            let fx = dx / distance * force;
            let fy = dy / distance * force;
            displacements.get_mut(&src).map(|d| { d.0 -= fx; d.1 -= fy; });
            displacements.get_mut(&tgt).map(|d| { d.0 += fx; d.1 += fy; });
        }

        // 3. クラスタ重力 (同一クラスタの重心へ引き寄せ)
        if config.cluster_strength > 0.0 {
            let mut cluster_centroids: HashMap<usize, (f64, f64, usize)> = HashMap::new();
            for (&idx, &cluster_id) in clusters {
                if let Some(pos) = layout.positions.get(&idx) {
                    let entry = cluster_centroids.entry(cluster_id).or_insert((0.0, 0.0, 0));
                    entry.0 += pos.x;
                    entry.1 += pos.y;
                    entry.2 += 1;
                }
            }
            let centroids: HashMap<usize, (f64, f64)> = cluster_centroids
                .into_iter()
                .filter(|(_, (_, _, count))| *count > 1)
                .map(|(id, (sx, sy, count))| (id, (sx / count as f64, sy / count as f64)))
                .collect();

            for (&idx, &cluster_id) in clusters {
                if let (Some(centroid), Some(disp)) =
                    (centroids.get(&cluster_id), displacements.get_mut(&idx))
                {
                    if let Some(pos) = layout.positions.get(&idx) {
                        let dx = centroid.0 - pos.x;
                        let dy = centroid.1 - pos.y;
                        disp.0 += dx * config.cluster_strength;
                        disp.1 += dy * config.cluster_strength;
                    }
                }
            }
        }

        // 4. 変位を適用 (温度でクランプ)
        for &idx in &node_indices {
            let (dx, dy) = displacements[&idx];
            let disp_len = (dx * dx + dy * dy).sqrt().max(0.01);
            let scale = (disp_len.min(temperature)) / disp_len;
            if let Some(pos) = layout.positions.get_mut(&idx) {
                pos.x += dx * scale;
                pos.y += dy * scale;
                // 境界クランプ (パディング付き)
                let pad = k * 0.5;
                pos.x = pos.x.clamp(pad, layout.width - pad);
                pos.y = pos.y.clamp(pad, layout.height - pad);
            }
        }

        temperature -= cooling;
        if temperature < 0.0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::stable_graph::StableGraph;

    #[test]
    fn layout_converges() {
        let mut g: StableGraph<(), ()> = StableGraph::new();
        let nodes: Vec<NodeIndex> = (0..10).map(|_| g.add_node(())).collect();

        let mut layout = GraphLayout::new(100.0, 100.0);
        layout.initialize_random(&nodes);

        let edges: Vec<(NodeIndex, NodeIndex)> = vec![
            (nodes[0], nodes[1]),
            (nodes[1], nodes[2]),
            (nodes[2], nodes[0]),
        ];

        let clusters = HashMap::new();
        compute_layout(&mut layout, &edges, &clusters, &FRConfig::default());

        // 全ノードが境界内にあることを確認
        for pos in layout.positions.values() {
            assert!(pos.x >= 0.0 && pos.x <= 100.0);
            assert!(pos.y >= 0.0 && pos.y <= 100.0);
        }
    }
}
