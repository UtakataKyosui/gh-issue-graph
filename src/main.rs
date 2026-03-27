mod cli;
mod config;
mod error;
mod github;
mod graph;
mod theme;
mod tui;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};
use cli::output::OutputFormat;
use config::Config;
use github::fetcher::FetchConfig;

#[derive(Parser)]
#[command(
    name = "gh-issue-graph",
    about = "Analyze and visualize GitHub Issue/PR relationship graphs",
    long_about = "Analyze Issue, PR, Sub-Issue, Milestone, and cross-reference relationships \
                  and visualize them as an interactive graph (like Obsidian's note clustering graph).\n\n\
                  Without arguments: launches interactive TUI\n\
                  With --repo/--json/--format/--jq: outputs to stdout (CLI mode)",
    version
)]
struct Cli {
    /// Repository in owner/repo format
    #[arg(long, value_name = "OWNER/REPO")]
    repo: Option<String>,

    /// Focus on a specific issue number
    #[arg(long, value_name = "NUMBER", requires = "repo")]
    issue: Option<u64>,

    /// Traversal depth for BFS relationship discovery (default: 2)
    #[arg(long, default_value = "2", value_name = "N")]
    depth: usize,

    /// Output as JSON. Optionally specify fields: --json nodes,edges,stats
    #[arg(long, value_name = "FIELDS", num_args = 0..=1)]
    json: Option<Option<String>>,

    /// jq filter expression (requires --json)
    #[arg(long, value_name = "EXPR", requires = "json")]
    jq: Option<String>,

    /// Output format: list, tree, dot (default: list)
    #[arg(long, value_name = "FORMAT", default_value = "list")]
    format: OutputFormat,

    /// Filter by milestone name
    #[arg(long, value_name = "MILESTONE")]
    milestone: Option<String>,

    /// Filter by label
    #[arg(long, value_name = "LABEL")]
    label: Option<String>,

    /// Disable timeline cross-reference fetching (faster)
    #[arg(long)]
    no_timeline: bool,

    /// Disable sub-issue fetching
    #[arg(long)]
    no_sub_issues: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show or initialize configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Initialize configuration file
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // config サブコマンド
    if let Some(Commands::Config { action }) = cli.command {
        return handle_config_command(action);
    }

    let config = Config::load()?;

    // CLI モード判定:
    // --json / --jq / --issue が指定されている場合は CLI モード
    // --repo のみ = TUI モード
    // 引数なし = TUI モード
    let is_cli_mode = cli.json.is_some()
        || cli.jq.is_some()
        || cli.issue.is_some()
        || (cli.repo.is_some()
            && matches!(cli.format, OutputFormat::Dot | OutputFormat::Tree));

    if is_cli_mode {
        run_cli(cli, config).await
    } else {
        run_tui(cli, config).await
    }
}

async fn run_cli(cli: Cli, config: Config) -> Result<()> {
    let token = config.resolve_token()?;
    let (owner, repo) = parse_repo(cli.repo.as_deref(), &config)?;

    let mut fetch_config = FetchConfig::new(owner.clone(), repo.clone());
    fetch_config.max_depth = cli.depth;
    fetch_config.focus_issue = cli.issue;
    fetch_config.fetch_timeline = !cli.no_timeline;
    fetch_config.fetch_sub_issues = !cli.no_sub_issues;

    eprintln!("Fetching data for {}/{}...", owner, repo);
    let client = github::build_client(&token)?;
    let mut graph = github::fetcher::build_graph(&client, &fetch_config).await?;

    // フィルタ適用
    if let Some(ref ms) = cli.milestone {
        graph = graph.filter_by_milestone(ms);
    }
    if let Some(ref label) = cli.label {
        graph = graph.filter_by_label(label);
    }

    // 出力
    if let Some(fields_opt) = cli.json {
        let fields = fields_opt.as_deref();
        let json_val = cli::output::to_json(&graph, fields)?;

        let output = if let Some(ref jq_expr) = cli.jq {
            cli::filter::apply_jq(&json_val, jq_expr)?
        } else {
            json_val
        };

        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        match cli.format {
            OutputFormat::Tree => {
                let focus = cli.issue.map(|n| crate::types::NodeId::new(&owner, &repo, n));
                print!("{}", cli::output::to_tree(&graph, focus.as_ref()));
            }
            OutputFormat::Dot => {
                print!("{}", cli::output::to_dot(&graph));
            }
            OutputFormat::List | OutputFormat::Json => {
                println!("{}", cli::output::to_list(&graph));
            }
        }
    }

    Ok(())
}

async fn run_tui(cli: Cli, config: Config) -> Result<()> {
    let token = config.resolve_token()?;
    let (owner, repo) = parse_repo(cli.repo.as_deref(), &config)?;

    let mut fetch_config = FetchConfig::new(owner, repo);
    fetch_config.max_depth = cli.depth;
    fetch_config.fetch_timeline = !cli.no_timeline;
    fetch_config.fetch_sub_issues = !cli.no_sub_issues;

    tui::run_tui(fetch_config, token).await
}

fn handle_config_command(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let config = Config::load()?;
            println!("{}", serde_json::to_string_pretty(&config)?);
            let path = Config::config_path();
            eprintln!("Config file: {}", path.display());
        }
        ConfigAction::Init => {
            let config = Config::default();
            config.save()?;
            let path = Config::config_path();
            println!("Config initialized at: {}", path.display());
        }
    }
    Ok(())
}

/// "owner/repo" 文字列を分解する
fn parse_repo(repo_arg: Option<&str>, config: &Config) -> Result<(String, String)> {
    let repo_str = repo_arg
        .or(config.default_repo.as_deref())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No repository specified. Use --repo owner/repo or set default_repo in config."
            )
        })?;

    let parts: Vec<&str> = repo_str.splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        anyhow::bail!("Invalid repository format: '{}'. Expected 'owner/repo'", repo_str);
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}
