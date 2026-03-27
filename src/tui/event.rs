use crate::tui::app::{App, AppMode, ClusterMode, FilterSection};
use crate::types::NodeState;
use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use ratatui::prelude::*;
use std::time::Duration;
use tokio::time::interval;

const TICK_RATE: Duration = Duration::from_millis(50);

pub async fn run_event_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut reader = EventStream::new();
    let mut tick = interval(TICK_RATE);
    let mut canvas_size = (0.0f64, 0.0f64);

    loop {
        tokio::select! {
            _ = tick.tick() => {
                // レイアウト更新チェック
                if !app.graph_loaded {
                    let size = terminal.size()?;
                    canvas_size = (size.width as f64 * 2.0, size.height as f64 * 4.0);
                    app.check_and_update_layout(canvas_size.0, canvas_size.1);
                }

                // アニメーション
                if app.layout_iter_remaining > 0 {
                    app.advance_layout(canvas_size.0, canvas_size.1);
                }

                // 描画
                terminal.draw(|f| render(f, app))?;
            }

            maybe_event = reader.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => {
                        if handle_key(app, key.code, key.modifiers) {
                            break;
                        }
                    }
                    Some(Ok(Event::Resize(w, h))) => {
                        canvas_size = (w as f64 * 2.0, h as f64 * 4.0);
                        if app.graph_loaded {
                            app.recompute_layout(canvas_size.0, canvas_size.1);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

/// キー入力を処理する。終了ならtrue
fn handle_key(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    match app.mode {
        AppMode::Normal => handle_normal_key(app, code, modifiers),
        AppMode::Search => handle_search_key(app, code),
        AppMode::Filter => handle_filter_key(app, code),
        AppMode::Detail => handle_detail_key(app, code),
    }
}

fn handle_normal_key(app: &mut App, code: KeyCode, _modifiers: KeyModifiers) -> bool {
    const PAN_STEP: f64 = 10.0;
    match code {
        KeyCode::Char('q') | KeyCode::Esc => return true,
        KeyCode::Char('h') | KeyCode::Left  => app.pan_x -= PAN_STEP / app.zoom,
        KeyCode::Char('l') | KeyCode::Right => app.pan_x += PAN_STEP / app.zoom,
        KeyCode::Char('k') | KeyCode::Up    => app.pan_y -= PAN_STEP / app.zoom,
        KeyCode::Char('j') | KeyCode::Down  => app.pan_y += PAN_STEP / app.zoom,
        KeyCode::Char('+') => app.zoom = (app.zoom * 1.2).min(10.0),
        KeyCode::Char('-') => app.zoom = (app.zoom / 1.2).max(0.1),
        KeyCode::Char('0') => app.reset_view(),
        KeyCode::Tab       => app.select_next_node(),
        KeyCode::Enter => {
            if app.selected_node.is_some() {
                app.show_detail = true;
                app.mode = AppMode::Detail;
            }
        }
        KeyCode::Char('/') => {
            app.mode = AppMode::Search;
            app.search_query.clear();
        }
        KeyCode::Char('f') => {
            app.show_filter_panel = !app.show_filter_panel;
            if app.show_filter_panel {
                app.mode = AppMode::Filter;
            }
        }
        KeyCode::Char('L') => app.show_legend = !app.show_legend,
        KeyCode::Char('c') => {
            // クラスタモードを Milestone → Label → None → Milestone と切替
            let size = (app.layout.as_ref().map(|l| l.width).unwrap_or(200.0),
                        app.layout.as_ref().map(|l| l.height).unwrap_or(100.0));
            app.cluster_mode = match app.cluster_mode {
                ClusterMode::Milestone => ClusterMode::Label,
                ClusterMode::Label     => ClusterMode::None,
                ClusterMode::None      => ClusterMode::Milestone,
            };
            app.status_message = format!("Cluster: {}", app.cluster_mode);
            app.recompute_layout(size.0, size.1);
        }
        KeyCode::Char('r') => {
            app.graph_loaded = false;
            app.loading = true;
            app.status_message = "Refreshing...".to_string();
        }
        _ => {}
    }
    false
}

fn handle_search_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.search_query.clear();
        }
        KeyCode::Enter => {
            // 最初のマッチにジャンプ
            let results = app.search_results();
            if let Some(&first) = results.first() {
                app.selected_node = Some(first);
            }
            app.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
        }
        _ => {}
    }
    false
}

fn handle_filter_key(app: &mut App, code: KeyCode) -> bool {
    const STATES: &[NodeState] = &[NodeState::Open, NodeState::Closed, NodeState::Merged, NodeState::Draft];

    // 各セクションの項目数を取得
    let (ms_count, lb_count) = {
        let graph = app.graph.read().unwrap();
        (
            graph.milestones.len().min(6),
            graph.labels.len().min(6),
        )
    };

    let section_max = match app.filter_section {
        FilterSection::Milestones => ms_count.saturating_sub(1),
        FilterSection::Labels     => lb_count.saturating_sub(1),
        FilterSection::States     => STATES.len() - 1,
    };

    match code {
        KeyCode::Esc | KeyCode::Char('f') => {
            app.show_filter_panel = false;
            app.mode = AppMode::Normal;
            // フィルタ変更でレイアウトを再計算
            let size = (app.layout.as_ref().map(|l| l.width).unwrap_or(200.0),
                        app.layout.as_ref().map(|l| l.height).unwrap_or(100.0));
            app.recompute_layout(size.0, size.1);
        }
        KeyCode::Tab => {
            app.filter_section = match app.filter_section {
                FilterSection::Milestones => FilterSection::Labels,
                FilterSection::Labels     => FilterSection::States,
                FilterSection::States     => FilterSection::Milestones,
            };
            app.filter_cursor = 0;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if app.filter_cursor < section_max {
                app.filter_cursor += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.filter_cursor = app.filter_cursor.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            match app.filter_section {
                FilterSection::Milestones => {
                    let ms = {
                        let graph = app.graph.read().unwrap();
                        graph.milestones.get(app.filter_cursor).cloned()
                    };
                    if let Some(ms) = ms {
                        if app.milestone_filter.as_deref() == Some(&ms) {
                            app.milestone_filter = None;
                        } else {
                            app.milestone_filter = Some(ms);
                        }
                    }
                }
                FilterSection::Labels => {
                    let label = {
                        let graph = app.graph.read().unwrap();
                        graph.labels.get(app.filter_cursor).cloned()
                    };
                    if let Some(label) = label {
                        if app.active_labels.contains(&label) {
                            app.active_labels.remove(&label);
                        } else {
                            app.active_labels.insert(label);
                        }
                    }
                }
                FilterSection::States => {
                    if let Some(state) = STATES.get(app.filter_cursor) {
                        if app.active_states.contains(state) {
                            app.active_states.remove(state);
                        } else {
                            app.active_states.insert(state.clone());
                        }
                    }
                }
            }
        }
        KeyCode::Char('c') => {
            // 全フィルタクリア
            app.milestone_filter = None;
            app.active_labels.clear();
            app.active_states.clear();
        }
        _ => {}
    }
    false
}

fn handle_detail_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.show_detail = false;
            app.mode = AppMode::Normal;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.detail_scroll = app.detail_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.detail_scroll = app.detail_scroll.saturating_sub(1);
        }
        KeyCode::Char('o') => {
            // ブラウザで開く
            if let Some(idx) = app.selected_node {
                let graph = app.graph.read().unwrap();
                if let Some(node) = graph.graph.node_weight(idx) {
                    let url = node.url.clone();
                    drop(graph);
                    let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" })
                        .arg(&url)
                        .spawn();
                }
            }
        }
        _ => {}
    }
    false
}

// --------- レンダリング ---------

fn render(f: &mut Frame, app: &App) {
    use crate::tui::widgets::*;

    let area = f.area();

    // ヘッダ (3行) + メイン + ステータスバー (1行)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // ヘッダ
    header::render_header(f, chunks[0], app);

    // メインエリア: グラフ + 詳細パネル
    let main_chunks = if app.show_detail {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
            .split(chunks[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[1])
    };

    // グラフキャンバス
    graph_canvas::render_graph(f, main_chunks[0], app);

    // 詳細パネル
    if app.show_detail {
        node_detail::render_detail(f, main_chunks[1], app);
    }

    // 凡例 (グラフキャンバス右下にオーバーレイ)
    if app.show_legend {
        legend::render_legend(f, main_chunks[0]);
    }

    // フィルタパネル (モーダル)
    if app.show_filter_panel {
        filter_panel::render_filter(f, area, app);
    }

    // 検索バー (モード時のみ)
    if app.mode == AppMode::Search {
        search::render_search(f, chunks[1], app);
    }

    // ステータスバー
    status_bar::render_status(f, chunks[2], app);
}
