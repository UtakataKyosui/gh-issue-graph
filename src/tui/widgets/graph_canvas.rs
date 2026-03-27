use crate::theme;
use crate::tui::app::App;
use crate::types::{NodeKind, NodeState, Priority, RelationshipKind};

/// 文字数で切り詰める (マルチバイト対応)
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_chars {
        let t: String = chars[..max_chars].iter().collect();
        format!("{}…", t)
    } else {
        s.to_string()
    }
}
use ratatui::{
    prelude::*,
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Circle, Line as CanvasLine},
        Block, Borders,
    },
};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};

pub fn render_graph(f: &mut Frame, area: Rect, app: &App) {
    let layout = match &app.layout {
        Some(l) => l,
        None => {
            // ローディング表示
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" gh-issue-graph ")
                .border_style(theme::style_border());
            let inner = block.inner(area);
            f.render_widget(block, area);
            let loading = ratatui::widgets::Paragraph::new(if app.loading {
                "Loading graph data..."
            } else {
                "No data"
            })
            .style(theme::style_loading())
            .alignment(Alignment::Center);
            f.render_widget(loading, inner);
            return;
        }
    };

    let graph = app.graph.read().unwrap();
    if graph.node_count() == 0 {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" gh-issue-graph ")
            .border_style(theme::style_border());
        f.render_widget(block, area);
        return;
    }

    // キャンバス座標: レイアウト座標をズーム/パンで変換
    let lw = layout.width;
    let lh = layout.height;
    let zoom = app.zoom;
    let cx = lw / 2.0 + app.pan_x;
    let cy = lh / 2.0 + app.pan_y;
    let half_w = lw / (2.0 * zoom);
    let half_h = lh / (2.0 * zoom);
    let x_min = cx - half_w;
    let x_max = cx + half_w;
    let y_min = cy - half_h;
    let y_max = cy + half_h;

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " gh-issue-graph  nodes:{} edges:{} zoom:{:.1}x ",
                    graph.node_count(),
                    graph.edge_count(),
                    zoom
                ))
                .border_style(if app.mode == crate::tui::app::AppMode::Normal {
                    theme::style_border_focused()
                } else {
                    theme::style_border()
                }),
        )
        .x_bounds([x_min, x_max])
        .y_bounds([y_min, y_max])
        .marker(Marker::Braille)
        .paint(|ctx| {
            // --- エッジ描画 ---
            for edge in graph.graph.edge_references() {
                let edge: petgraph::stable_graph::EdgeReference<'_, _> = edge;
                let src_idx = edge.source();
                let tgt_idx = edge.target();
                let src_pos = layout.positions.get(&src_idx);
                let tgt_pos = layout.positions.get(&tgt_idx);
                if let (Some(sp), Some(tp)) = (src_pos, tgt_pos) {
                    let color = match edge.weight() {
                        RelationshipKind::ParentChild      => theme::edge_color_parent_child(),
                        RelationshipKind::ClosingReference => theme::edge_color_closing_ref(),
                        RelationshipKind::CrossReference   => theme::edge_color_cross_ref(),
                        RelationshipKind::ConnectedEvent   => theme::edge_color_connected(),
                        RelationshipKind::BodyMention      => theme::edge_color_body_mention(),
                        RelationshipKind::SameMilestone    => theme::edge_color_same_milestone(),
                        RelationshipKind::Duplicate        => theme::edge_color_duplicate(),
                    };
                    ctx.draw(&CanvasLine {
                        x1: sp.x,
                        y1: sp.y,
                        x2: tp.x,
                        y2: tp.y,
                        color,
                    });

                    // ズーム >= 2.5 でエッジ中点に関係種別ラベルを表示
                    // SameMilestone は除外 (ノイズが多い)
                    if zoom >= 2.5 && !matches!(edge.weight(), RelationshipKind::SameMilestone) {
                        let mid_x = (sp.x + tp.x) / 2.0;
                        let mid_y = (sp.y + tp.y) / 2.0;
                        let label = match edge.weight() {
                            RelationshipKind::ParentChild      => "parent",
                            RelationshipKind::ClosingReference => "closes",
                            RelationshipKind::CrossReference   => "ref",
                            RelationshipKind::ConnectedEvent   => "linked",
                            RelationshipKind::BodyMention      => "mention",
                            RelationshipKind::Duplicate        => "dup",
                            RelationshipKind::SameMilestone    => "",
                        };
                        ctx.print(mid_x, mid_y, Span::styled(label, Style::default().fg(color)));
                    }
                }
            }

            // --- ノード描画 ---
            for idx in graph.all_node_indices() {
                if let (Some(pos), Some(node)) = (
                    layout.positions.get(&idx),
                    graph.graph.node_weight(idx),
                ) {
                    let is_selected = app.selected_node == Some(idx);
                    let base_color = if is_selected {
                        theme::node_color_selected()
                    } else {
                        match (&node.kind, &node.state) {
                            (NodeKind::Issue, NodeState::Open)       => theme::node_color_open_issue(),
                            (NodeKind::Issue, _)                     => theme::node_color_closed_issue(),
                            (NodeKind::PullRequest, NodeState::Open) => theme::node_color_open_pr(),
                            (NodeKind::PullRequest, NodeState::Merged) => theme::node_color_merged_pr(),
                            (NodeKind::PullRequest, NodeState::Draft)  => theme::node_color_draft_pr(),
                            (NodeKind::PullRequest, _)               => theme::node_color_closed_issue(),
                        }
                    };

                    // 優先度によってノードサイズを変える
                    let priority_scale = match node.priority {
                        Priority::Critical => 1.5,
                        Priority::High     => 1.25,
                        _                  => 1.0,
                    };
                    let base_radius = if is_selected { 3.0 } else { 2.0 };
                    let radius = base_radius * priority_scale;

                    ctx.draw(&Circle {
                        x: pos.x,
                        y: pos.y,
                        radius,
                        color: base_color,
                    });

                    // ラベル表示 (段階的)
                    if zoom >= 0.5 || graph.node_count() < 20 {
                        let number_label = match node.kind {
                            NodeKind::Issue       => format!("#{}", node.id.number),
                            NodeKind::PullRequest => format!("PR#{}", node.id.number),
                        };

                        // 優先度アイコン (None 以外)
                        let prio_icon = node.priority.icon();

                        if zoom >= 2.0 {
                            // 高ズーム: "#N タイトル [P0]"
                            let title_truncated = truncate_chars(&node.title, 29);
                            let full_label = if prio_icon.is_empty() {
                                format!("{} {}", number_label, title_truncated)
                            } else {
                                format!("{} {} {}", number_label, title_truncated, prio_icon)
                            };
                            ctx.print(pos.x + radius + 1.0, pos.y, Span::styled(full_label, Style::default().fg(base_color)));

                            // ズーム >= 3.0 で最初の2ラベルをその下に表示
                            if zoom >= 3.0 && !node.labels.is_empty() {
                                let label_str = node.labels.iter().take(2)
                                    .map(|l| format!("[{}]", l))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                ctx.print(pos.x + radius + 1.0, pos.y - 4.0, Span::styled(label_str, Style::default().fg(theme::color_subtext())));
                            }
                        } else {
                            // 標準ズーム: "#N [P0]"
                            let short_label = if prio_icon.is_empty() {
                                number_label
                            } else {
                                format!("{} {}", number_label, prio_icon)
                            };
                            ctx.print(pos.x + radius + 1.0, pos.y, Span::styled(short_label, Style::default().fg(base_color)));
                        }
                    }
                }
            }
        });

    f.render_widget(canvas, area);
}
