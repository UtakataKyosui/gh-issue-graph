pub mod app;
pub mod config;
pub mod core;
pub mod github;
pub mod json_output;
pub mod types;
pub mod ui;

use anyhow::Result;

/// Entry point for the `monitor` subcommand: load config, resolve token, run TUI.
pub async fn run() -> Result<()> {
    let cfg = config::load_config()?;

    if cfg.repositories.is_empty() {
        anyhow::bail!(
            "No repositories configured for the monitor.\n\
             Create ~/.config/gh-issue-monitor/config.json or set GH_ISSUE_MONITOR_CONFIG.\n\
             Example:\n\
             {{\n  \"repositories\": [{{\"owner\": \"my-org\", \"name\": \"my-repo\"}}]\n}}"
        );
    }

    let token = config::resolve_token(&cfg)?;
    let client = github::build_client(&token)?;
    let filter = core::FilterOptions::default();

    app::run(client, cfg, filter).await
}
