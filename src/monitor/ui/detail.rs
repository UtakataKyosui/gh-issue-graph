use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame,
};

use crate::monitor::app::App;
use crate::monitor::core::build_issue_tree;
use crate::monitor::types::{IssueState, IssueStatus, IssueRelationship, LinkedPr, PrState, RelationshipType};

use super::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.monitor_state.read().unwrap();

    let Some(repo_result) = state.results.get(app.selected_tab) else {
        frame.render_widget(
            Block::bordered()
                .border_type(theme::BORDER_TYPE)
                .border_style(theme::border_style())
                .title("Detail")
                .title_style(theme::title_style()),
            area,
        );
        return;
    };

    if repo_result.error.is_some() {
        let para = Paragraph::new("See error details above")
            .block(
                Block::bordered()
                    .border_type(theme::BORDER_TYPE)
                    .border_style(theme::border_style())
                    .title("Detail")
                    .title_style(theme::title_style()),
            )
            .style(theme::dimmed());
        frame.render_widget(para, area);
        return;
    }

    let filtered_issues: Vec<IssueStatus> = repo_result
        .issues
        .iter()
        .filter(|i| app.filter.matches(i))
        .cloned()
        .collect();

    let tree_entries = build_issue_tree(&filtered_issues);

    let selected_issue = app
        .table_state
        .selected()
        .and_then(|i| tree_entries.get(i))
        .map(|e| &filtered_issues[e.issue_index]);

    let Some(issue) = selected_issue else {
        let para = Paragraph::new("Select an issue to see details")
            .block(
                Block::bordered()
                    .border_type(theme::BORDER_TYPE)
                    .border_style(theme::border_style())
                    .title("Detail")
                    .title_style(theme::title_style()),
            )
            .style(theme::dimmed());
        frame.render_widget(para, area);
        return;
    };

    let title = format!("Issue #{}: {}", issue.number, issue.title);
    let lines = build_detail_lines(issue);
    let total_lines = lines.len();

    let para = Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_type(theme::BORDER_TYPE)
                .border_style(theme::border_style())
                .title(title)
                .title_style(theme::title_style()),
        )
        .wrap(Wrap { trim: true })
        .scroll((app.detail_scroll, 0));

    frame.render_widget(para, area);

    if total_lines > 0 {
        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(app.detail_scroll as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).style(theme::dimmed()),
            area,
            &mut scrollbar_state,
        );
    }
}

fn build_detail_lines(issue: &IssueStatus) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let has_prs = !issue.linked_prs.is_empty();
    let has_branches = issue.matched_branches.iter().any(|b| !b.has_pr);
    let has_sub_issues = issue.sub_issues_summary.as_ref().map(|s| s.total > 0).unwrap_or(false)
        || !issue.sub_issues.is_empty();
    let has_relationships = issue.has_relationships();

    if !has_prs && !has_branches && !has_sub_issues && !has_relationships {
        lines.push(Line::from(Span::styled(
            "No linked PRs, branches, sub-issues, or relationships found",
            theme::dimmed(),
        )));
        return lines;
    }

    for pr in &issue.linked_prs {
        lines.push(pr_line(pr));
    }
    for branch in &issue.matched_branches {
        if branch.has_pr {
            continue;
        }
        let author_span = branch
            .likely_author
            .as_deref()
            .map(|a| format!(" (likely @{a})"))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {}", theme::ICON_BRANCH),
                Style::new().fg(theme::TEAL),
            ),
            Span::raw(branch.name.clone()),
            Span::styled(author_span, theme::dimmed()),
            Span::styled(
                format!(" {} no PR", theme::ICON_NO_PR),
                theme::dimmed(),
            ),
        ]));
    }

    if has_sub_issues {
        if has_prs || has_branches {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            "Sub-issues",
            theme::title_style(),
        )));
        if let Some(summary) = &issue.sub_issues_summary {
            if summary.total > 0 {
                lines.push(Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled(
                        format!(
                            "{}/{} completed ({:.0}%)",
                            summary.completed, summary.total, summary.percent_completed
                        ),
                        if summary.completed == summary.total {
                            theme::sub_issue_closed()
                        } else {
                            theme::sub_issue_open()
                        },
                    ),
                ]));
            }
        }
        for sub in &issue.sub_issues {
            let (symbol, style) = match sub.state {
                IssueState::Open => (theme::SUB_ISSUE_OPEN_SYMBOL, theme::sub_issue_open()),
                IssueState::Closed => (theme::SUB_ISSUE_CLOSED_SYMBOL, theme::sub_issue_closed()),
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {symbol} "), style),
                Span::styled(format!("#{} ", sub.number), theme::dimmed()),
                Span::styled(sub.title.clone(), theme::table_row_normal()),
            ]));
        }
    }

    if has_relationships {
        if has_prs || has_branches || has_sub_issues {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            "Relationships",
            theme::title_style(),
        )));
        for rel in &issue.relationships {
            lines.push(relationship_line(rel));
        }
    }

    lines
}

fn relationship_line(rel: &IssueRelationship) -> Line<'static> {
    let (icon, label) = match rel.relationship_type {
        RelationshipType::Tracks => (theme::ICON_TRACKS, "tracks"),
        RelationshipType::TrackedBy => (theme::ICON_TRACKED_BY, "tracked by"),
        RelationshipType::Duplicate => (theme::ICON_DUPLICATE, "duplicate of"),
    };
    let state_style = match rel.issue.state {
        IssueState::Open => theme::status_active(),
        IssueState::Closed => theme::dimmed(),
    };
    Line::from(vec![
        Span::styled(format!("  {icon} "), theme::relationship_style()),
        Span::styled(format!("{label} "), theme::dimmed()),
        Span::styled(format!("#{} ", rel.issue.number), theme::accent()),
        Span::styled(rel.issue.title.clone(), state_style),
    ])
}

fn pr_line(pr: &LinkedPr) -> Line<'static> {
    let (state_style, state_str) = match pr.state {
        PrState::Open => (theme::pr_open(), "open"),
        PrState::Merged => (theme::pr_merged(), "merged"),
        PrState::Closed => (theme::pr_closed(), "closed"),
    };

    let draft_span = if pr.is_draft {
        Span::styled(
            format!(" {} draft", theme::ICON_DRAFT),
            theme::dimmed(),
        )
    } else {
        Span::raw("")
    };

    let close_span = if pr.will_close {
        Span::styled(
            format!(" {} will close", theme::ICON_WILL_CLOSE),
            Style::new().fg(theme::YELLOW).add_modifier(ratatui::style::Modifier::BOLD),
        )
    } else {
        Span::raw("")
    };

    Line::from(vec![
        Span::styled(
            format!("  {} PR ", theme::ICON_PR),
            Style::new().fg(theme::TEAL),
        ),
        Span::styled(format!("#{} ", pr.number), theme::dimmed()),
        Span::styled(format!("by @{} ", pr.author), theme::accent()),
        Span::styled(format!("({state_str}"), state_style),
        draft_span,
        Span::styled(")", state_style),
        Span::styled(
            format!(" {} ", theme::ICON_BRANCH),
            theme::dimmed(),
        ),
        Span::raw(pr.head_branch.clone()),
        close_span,
    ])
}
