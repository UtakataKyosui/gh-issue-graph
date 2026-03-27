pub mod app;
pub mod event;
pub mod widgets;

use crate::graph::model::IssueGraph;
use crate::github::fetcher::FetchConfig;
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use std::sync::{Arc, RwLock};

pub async fn run_tui(
    fetch_config: FetchConfig,
    token: String,
) -> Result<()> {
    // ターミナル初期化
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let shared_graph: Arc<RwLock<IssueGraph>> = Arc::new(RwLock::new(IssueGraph::new()));
    let graph_clone = Arc::clone(&shared_graph);

    // データ取得をバックグラウンドで実行
    let fetch_handle = tokio::spawn(async move {
        let client = crate::github::build_client(&token)?;
        let graph = crate::github::fetcher::build_graph(&client, &fetch_config).await?;
        *graph_clone.write().unwrap() = graph;
        Ok::<(), anyhow::Error>(())
    });

    let mut app = app::App::new(Arc::clone(&shared_graph));

    let result = event::run_event_loop(&mut terminal, &mut app).await;

    // ターミナル復元
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // バックグラウンドタスクのエラーを確認
    if let Err(e) = fetch_handle.await? {
        eprintln!("Data fetch error: {}", e);
    }

    result
}
