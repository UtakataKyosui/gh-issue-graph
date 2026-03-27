use crate::graph::cluster::{build_label_clusters, build_milestone_clusters};
use crate::graph::layout::{compute_layout, FRConfig, GraphLayout};
use crate::graph::model::IssueGraph;
use crate::theme;
use crate::types::{NodeState, Priority, RelationshipKind};
use petgraph::stable_graph::NodeIndex;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use ratatui::prelude::*;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, PartialEq)]
pub enum FilterSection {
    Milestones,
    Labels,
    States,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClusterMode {
    Milestone,
    Label,
    None,
}

impl std::fmt::Display for ClusterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterMode::Milestone => write!(f, "milestone"),
            ClusterMode::Label     => write!(f, "label"),
            ClusterMode::None      => write!(f, "none"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    Search,
    Filter,
    Detail,
}

pub struct App {
    pub graph: Arc<RwLock<IssueGraph>>,
    pub layout: Option<GraphLayout>,

    // ビュー状態
    pub mode: AppMode,
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub selected_node: Option<NodeIndex>,
    pub hovered_node: Option<NodeIndex>,

    // フィルタ
    pub milestone_filter: Option<String>,
    pub label_filter: Option<String>,
    pub state_filter: Vec<NodeState>,
    pub active_labels: HashSet<String>,
    pub active_states: HashSet<NodeState>,
    pub search_query: String,

    // フィルタパネル UI 状態
    pub filter_cursor: usize,
    pub filter_section: FilterSection,

    // クラスタリングモード
    pub cluster_mode: ClusterMode,

    // UI 状態
    pub show_legend: bool,
    pub show_detail: bool,
    pub show_filter_panel: bool,
    pub detail_scroll: u16,

    // ローディング
    pub loading: bool,
    pub status_message: String,
    pub graph_loaded: bool,

    // アニメーション用
    pub layout_iter_remaining: usize,
}

impl App {
    pub fn new(graph: Arc<RwLock<IssueGraph>>) -> Self {
        Self {
            graph,
            layout: None,
            mode: AppMode::Normal,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            selected_node: None,
            hovered_node: None,
            milestone_filter: None,
            label_filter: None,
            state_filter: vec![],
            active_labels: HashSet::new(),
            active_states: HashSet::new(),
            search_query: String::new(),
            filter_cursor: 0,
            filter_section: FilterSection::Milestones,
            cluster_mode: ClusterMode::Milestone,
            show_legend: true,
            show_detail: false,
            show_filter_panel: false,
            detail_scroll: 0,
            loading: true,
            status_message: "Loading graph data...".to_string(),
            graph_loaded: false,
            layout_iter_remaining: 0,
        }
    }

    /// グラフデータが更新されたかチェックしてレイアウトを初期化する
    pub fn check_and_update_layout(&mut self, canvas_width: f64, canvas_height: f64) {
        let count = self.graph.read().unwrap().node_count();
        if count == 0 {
            return;
        }
        if self.graph_loaded {
            return; // 既にロード済み
        }
        self.graph_loaded = true;
        self.loading = false;
        self.status_message = format!("{} nodes loaded", count);
        self.recompute_layout(canvas_width, canvas_height);
    }

    /// レイアウトを再計算する
    pub fn recompute_layout(&mut self, canvas_width: f64, canvas_height: f64) {
        let graph = self.graph.read().unwrap();
        if graph.node_count() == 0 {
            return;
        }

        let node_indices: Vec<NodeIndex> = graph.all_node_indices().collect();
        let mut layout = GraphLayout::new(canvas_width, canvas_height);
        layout.initialize_random(&node_indices);

        let edges: Vec<(NodeIndex, NodeIndex)> = graph
            .graph
            .edge_references()
            .map(|e: petgraph::stable_graph::EdgeReference<'_, _>| (e.source(), e.target()))
            .collect();

        let clusters = match self.cluster_mode {
            ClusterMode::Milestone => build_milestone_clusters(&graph),
            ClusterMode::Label     => build_label_clusters(&graph),
            ClusterMode::None      => std::collections::HashMap::new(),
        };

        compute_layout(&mut layout, &edges, &clusters, &FRConfig::default());

        self.layout = Some(layout);
    }

    /// アニメーション用の段階的レイアウト (1フレーム分)
    pub fn advance_layout(&mut self, canvas_width: f64, canvas_height: f64) {
        if self.layout_iter_remaining == 0 {
            return;
        }
        let graph = self.graph.read().unwrap();
        if let Some(ref mut layout) = self.layout {
            let edges: Vec<(NodeIndex, NodeIndex)> = graph
                .graph
                .edge_references()
                .map(|e: petgraph::stable_graph::EdgeReference<'_, _>| (e.source(), e.target()))
                .collect();
            let clusters = match self.cluster_mode {
                ClusterMode::Milestone => build_milestone_clusters(&graph),
                ClusterMode::Label     => build_label_clusters(&graph),
                ClusterMode::None      => std::collections::HashMap::new(),
            };
            let config = FRConfig {
                iterations: 10,
                cluster_strength: 0.05,
            };
            compute_layout(layout, &edges, &clusters, &config);
        }
        self.layout_iter_remaining = self.layout_iter_remaining.saturating_sub(10);
    }

    /// ズームをリセットしてグラフ全体を表示する
    pub fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan_x = 0.0;
        self.pan_y = 0.0;
    }

    /// 検索クエリにマッチするノードを返す
    pub fn search_results(&self) -> Vec<NodeIndex> {
        if self.search_query.is_empty() {
            return vec![];
        }
        let q = self.search_query.to_lowercase();
        let graph = self.graph.read().unwrap();
        graph
            .graph
            .node_indices()
            .filter(|&idx| {
                if let Some(node) = graph.graph.node_weight(idx) {
                    node.title.to_lowercase().contains(&q)
                        || node.id.number.to_string().contains(&q)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Tab キーで次のノードを選択する
    pub fn select_next_node(&mut self) {
        let graph = self.graph.read().unwrap();
        let indices: Vec<NodeIndex> = graph.all_node_indices().collect();
        if indices.is_empty() {
            return;
        }
        self.selected_node = Some(match self.selected_node {
            None => indices[0],
            Some(current) => {
                let pos = indices.iter().position(|&i| i == current).unwrap_or(0);
                indices[(pos + 1) % indices.len()]
            }
        });
        self.detail_scroll = 0;
    }

    /// 現在選択中のノードの詳細を styled Lines で返す
    pub fn selected_node_detail(&self) -> Option<Vec<Line<'static>>> {
        let idx = self.selected_node?;
        let graph = self.graph.read().unwrap();
        let node = graph.graph.node_weight(idx)?.clone();

        let mut lines: Vec<Line<'static>> = Vec::new();

        // タイトル行
        lines.push(Line::from(vec![
            Span::styled(node.display_number(), theme::style_title()),
            Span::raw("  "),
            Span::styled(node.title.clone(), theme::style_title()),
        ]));

        // State
        let state_color = match node.state {
            NodeState::Open   => theme::node_color_open_issue(),
            NodeState::Closed => theme::node_color_closed_issue(),
            NodeState::Merged => theme::node_color_merged_pr(),
            NodeState::Draft  => theme::node_color_draft_pr(),
        };
        lines.push(Line::from(vec![
            Span::styled("State: ", theme::style_dimmed()),
            Span::styled(node.state.to_string(), Style::default().fg(state_color)),
        ]));

        // Priority (None 以外のみ)
        if node.priority != Priority::None {
            lines.push(Line::from(vec![
                Span::styled("Priority: ", theme::style_dimmed()),
                Span::styled(
                    format!("{} ({})", node.priority.icon(), node.priority),
                    Style::default().fg(theme::priority_color(&node.priority)).add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // URL
        lines.push(Line::from(vec![
            Span::styled("URL: ", theme::style_dimmed()),
            Span::styled(node.url.clone(), theme::style_normal()),
        ]));

        // Milestone
        if let Some(ref ms) = node.milestone {
            lines.push(Line::from(vec![
                Span::styled("Milestone: ", theme::style_dimmed()),
                Span::styled(ms.clone(), Style::default().fg(theme::color_accent())),
            ]));
        }

        // Labels
        if !node.labels.is_empty() {
            let mut label_spans = vec![Span::styled("Labels: ", theme::style_dimmed())];
            for (i, label) in node.labels.iter().enumerate() {
                if i > 0 {
                    label_spans.push(Span::raw("  "));
                }
                label_spans.push(Span::styled(
                    format!("[{}]", label),
                    Style::default().fg(theme::label_color(label)),
                ));
            }
            lines.push(Line::from(label_spans));
        }

        // Assignees
        if !node.assignees.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Assignees: ", theme::style_dimmed()),
                Span::styled(node.assignees.join(", "), theme::style_normal()),
            ]));
        }

        // 隣接関係
        let neighbors: Vec<_> = graph
            .neighbors(&node.id)
            .into_iter()
            .map(|(n, k)| (n.clone(), k.clone()))
            .collect();

        if !neighbors.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "── Relationships ──────────────────",
                theme::style_dimmed(),
            )));

            for (neighbor, kind) in &neighbors {
                let edge_color = match kind {
                    RelationshipKind::ParentChild      => theme::edge_color_parent_child(),
                    RelationshipKind::ClosingReference => theme::edge_color_closing_ref(),
                    RelationshipKind::CrossReference   => theme::edge_color_cross_ref(),
                    RelationshipKind::ConnectedEvent   => theme::edge_color_connected(),
                    RelationshipKind::BodyMention      => theme::edge_color_body_mention(),
                    RelationshipKind::SameMilestone    => theme::edge_color_same_milestone(),
                    RelationshipKind::Duplicate        => theme::edge_color_duplicate(),
                };

                // 関係種別 → 相手ノード
                let mut rel_spans = vec![
                    Span::styled(format!("  {:>14}  ", kind.to_string()), Style::default().fg(edge_color)),
                    Span::styled(
                        neighbor.display_number(),
                        Style::default().fg(edge_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        // タイトルを 30 文字で切り詰め (マルチバイト対応)
                        {
                            let chars: Vec<char> = neighbor.title.chars().collect();
                            if chars.len() > 29 {
                                format!("{}…", chars[..29].iter().collect::<String>())
                            } else {
                                neighbor.title.clone()
                            }
                        },
                        theme::style_normal(),
                    ),
                ];

                // 優先度インジケータ
                if neighbor.priority != Priority::None {
                    rel_spans.push(Span::raw(" "));
                    rel_spans.push(Span::styled(
                        neighbor.priority.icon(),
                        Style::default().fg(theme::priority_color(&neighbor.priority)),
                    ));
                }

                lines.push(Line::from(rel_spans));

                // 共有ラベル
                let shared_labels: Vec<&String> = node
                    .labels
                    .iter()
                    .filter(|l| neighbor.labels.contains(l))
                    .collect();
                if !shared_labels.is_empty() {
                    let mut shared_spans = vec![
                        Span::styled("  shared: ", theme::style_dimmed()),
                    ];
                    for (i, label) in shared_labels.iter().enumerate() {
                        if i > 0 {
                            shared_spans.push(Span::raw(" "));
                        }
                        shared_spans.push(Span::styled(
                            format!("[{}]", label),
                            Style::default().fg(theme::label_color(label)),
                        ));
                    }
                    lines.push(Line::from(shared_spans));
                }

                // 相手ノードのラベルのみ (共有以外)
                let own_labels: Vec<&String> = neighbor
                    .labels
                    .iter()
                    .filter(|l| !node.labels.contains(l))
                    .collect();
                if !own_labels.is_empty() {
                    let mut own_spans = vec![
                        Span::styled("  labels: ", theme::style_dimmed()),
                    ];
                    for (i, label) in own_labels.iter().enumerate() {
                        if i > 0 {
                            own_spans.push(Span::raw(" "));
                        }
                        own_spans.push(Span::styled(
                            format!("[{}]", label),
                            Style::default()
                                .fg(theme::label_color(label))
                                .add_modifier(Modifier::DIM),
                        ));
                    }
                    lines.push(Line::from(own_spans));
                }
            }
        }

        Some(lines)
    }
}
