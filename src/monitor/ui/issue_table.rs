use ratatui::{
    layout::{Alignment, Constraint, Rect},
    text::{Line, Span},
    widgets::{Block, Cell, HighlightSpacing, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::monitor::app::App;
use crate::monitor::core::build_issue_tree;
use crate::monitor::types::IssueStatus;

use super::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.monitor_state.read().unwrap();

    let Some(repo_result) = state.results.get(app.selected_tab) else {
        let block = Block::bordered()
            .border_type(theme::BORDER_TYPE)
            .border_style(theme::border_style())
            .title("Issues")
            .title_style(theme::title_style());
        frame.render_widget(block, area);
        return;
    };

    if let Some(err) = &repo_result.error {
        render_error_panel(frame, area, &repo_result.repo_name(), err);
        return;
    }

    let filtered_issues: Vec<&IssueStatus> = repo_result
        .issues
        .iter()
        .filter(|i| app.filter.matches(i))
        .collect();

    let total = repo_result.issues.len();
    let shown = filtered_issues.len();
    let active_count = filtered_issues.iter().filter(|i| i.has_active_work()).count();

    let title = if shown < total {
        format!("Issues — {shown} shown ({total} total), {active_count} with active work")
    } else {
        format!("Issues — {total} open, {active_count} with active work")
    };

    let filtered_owned: Vec<IssueStatus> = filtered_issues.iter().map(|i| (*i).clone()).collect();
    let tree_entries = build_issue_tree(&filtered_owned);

    let header = Row::new(vec![
        Cell::from("#").style(theme::table_header()),
        Cell::from("Title").style(theme::table_header()),
        Cell::from("Milestone").style(theme::table_header()),
        Cell::from("Sub").style(theme::table_header()),
        Cell::from("PRs").style(theme::table_header()),
        Cell::from("Branches").style(theme::table_header()),
        Cell::from("Rel").style(theme::table_header()),
        Cell::from("Status").style(theme::table_header()),
    ])
    .bottom_margin(1);

    let rows: Vec<Row> = tree_entries
        .iter()
        .enumerate()
        .map(|(row_idx, entry)| {
            let issue = &filtered_owned[entry.issue_index];
            let row = issue_row(issue, entry.connector().as_str());
            if row_idx % 2 == 1 {
                row.style(theme::table_row_alt())
            } else {
                row.style(theme::table_row_normal())
            }
        })
        .collect();

    let widths = [
        Constraint::Length(10),
        Constraint::Fill(1),
        Constraint::Length(14),
        Constraint::Length(5),
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Length(5),
        Constraint::Length(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::bordered()
                .border_type(theme::BORDER_TYPE)
                .border_style(theme::border_style())
                .title(title)
                .title_style(theme::title_style()),
        )
        .row_highlight_style(theme::table_highlight())
        .highlight_symbol(theme::TABLE_HIGHLIGHT_SYMBOL)
        .highlight_spacing(HighlightSpacing::Always);

    let mut table_state = app.table_state;
    frame.render_stateful_widget(table, area, &mut table_state);
}

fn issue_row<'a>(issue: &'a IssueStatus, tree_prefix: &str) -> Row<'a> {
    let pr_count = issue.linked_prs.len();
    let branch_count = issue.matched_branches.iter().filter(|b| !b.has_pr).count();

    let (status_symbol, status_text, status_style) = if issue.has_active_work() {
        (theme::STATUS_ACTIVE_SYMBOL, "active", theme::status_active())
    } else {
        (theme::STATUS_IDLE_SYMBOL, "idle", theme::status_idle())
    };

    let number_cell = if tree_prefix.is_empty() {
        Cell::from(Span::styled(format!("#{}", issue.number), theme::dimmed()))
    } else {
        Cell::from(Line::from(vec![
            Span::styled(tree_prefix.to_string(), theme::tree_connector()),
            Span::styled(format!("#{}", issue.number), theme::dimmed()),
        ]))
    };

    let milestone_cell = match &issue.milestone {
        None => Cell::from(""),
        Some(ms) => {
            let truncated = truncate_str(&ms.title, 13);
            Cell::from(Span::styled(truncated, theme::milestone_style()))
        }
    };

    let sub_cell = match &issue.sub_issues_summary {
        None => Cell::from(""),
        Some(s) if s.total == 0 => Cell::from(""),
        Some(s) => Cell::from(Span::styled(
            format!("{}/{}", s.completed, s.total),
            if s.completed == s.total {
                theme::sub_issue_closed()
            } else {
                theme::sub_issue_open()
            },
        )),
    };

    let rel_cell = if issue.has_relationships() {
        let count = issue.relationships.len();
        Cell::from(Span::styled(
            format!("{} {count}", theme::ICON_RELATIONSHIP),
            theme::relationship_style(),
        ))
    } else {
        Cell::from("")
    };

    Row::new(vec![
        number_cell,
        Cell::from(issue.title.clone()),
        milestone_cell,
        sub_cell,
        Cell::from(pr_count.to_string()),
        Cell::from(branch_count.to_string()),
        rel_cell,
        Cell::from(Span::styled(
            format!("{status_symbol} {status_text}"),
            status_style,
        )),
    ])
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let mut result: String = chars[..max_chars - 1].iter().collect();
        result.push('…');
        result
    }
}

fn render_error_panel(frame: &mut Frame, area: Rect, repo_name: &str, raw_error: &str) {
    let (title, message, hint) = classify_error(raw_error);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {} {title}", theme::ERROR_ICON),
        theme::error_title(),
    )));
    lines.push(Line::from(""));
    for line in message.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {line}"),
            theme::error_body(),
        )));
    }
    lines.push(Line::from(""));
    for line in hint.lines() {
        lines.push(Line::from(Span::styled(
            format!("  {line}"),
            theme::error_hint(),
        )));
    }

    let block_title = format!("Issues — {repo_name}");
    let block = Block::bordered()
        .border_type(theme::BORDER_TYPE)
        .border_style(theme::error_title())
        .title(block_title)
        .title_style(theme::error_title());

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);

    frame.render_widget(para, area);
}

fn classify_error(raw: &str) -> (&'static str, String, &'static str) {
    let lower = raw.to_lowercase();

    if lower.contains("not found") || lower.contains("could not resolve") {
        (
            "Repository not found",
            "The repository could not be found.\nThis may be a typo in the name, or you may not have permission to access it.".to_string(),
            "Check the repository name in your config file.\nEnsure your token has the `repo` scope for private repositories.",
        )
    } else if lower.contains("bad credentials") || lower.contains("401") || lower.contains("unauthorized") {
        (
            "Authentication failed",
            "Your GitHub token was rejected.".to_string(),
            "Run `gh auth login` to re-authenticate, or check that\nGH_TOKEN / GITHUB_TOKEN is set to a valid token.",
        )
    } else if lower.contains("rate limit") || lower.contains("403") || lower.contains("secondary rate") {
        (
            "Rate limit exceeded",
            "You've hit the GitHub API rate limit.\nThe monitor will retry on the next poll cycle.".to_string(),
            "Consider increasing the poll interval with --interval.\nRate limits reset every hour.",
        )
    } else if lower.contains("timed out") || lower.contains("timeout") {
        (
            "Request timed out",
            "The GitHub API request took too long to respond.".to_string(),
            "Check your network connection.\nThe monitor will retry automatically on the next poll.",
        )
    } else if lower.contains("dns") || lower.contains("resolve host") || lower.contains("network") || lower.contains("connection") {
        (
            "Network error",
            "Could not connect to GitHub.".to_string(),
            "Check your internet connection and proxy settings.\nThe monitor will retry automatically on the next poll.",
        )
    } else if lower.contains("graphql") {
        let clean = raw
            .strip_prefix("GraphQL errors for ")
            .and_then(|s| s.split_once(": "))
            .map(|(_, json_part)| summarize_graphql_errors(json_part))
            .unwrap_or_else(|| raw.to_string());
        (
            "API error",
            clean,
            "This may be a temporary issue. The monitor will retry automatically.",
        )
    } else {
        (
            "Unexpected error",
            raw.to_string(),
            "Press `r` to retry. If this persists, check your config and network.",
        )
    }
}

fn summarize_graphql_errors(json_str: &str) -> String {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(arr) = val.as_array() {
            let msgs: Vec<&str> = arr
                .iter()
                .filter_map(|e| e.get("message").and_then(|m| m.as_str()))
                .collect();
            if !msgs.is_empty() {
                return msgs.join("\n");
            }
        }
    }
    if json_str.len() > 120 {
        format!("{}...", &json_str[..120])
    } else {
        json_str.to_string()
    }
}
