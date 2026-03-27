use crate::theme;
use crate::tui::app::{App, FilterSection};
use crate::types::NodeState;
use ratatui::{prelude::*, widgets::*};

const STATES: &[NodeState] = &[
    NodeState::Open,
    NodeState::Closed,
    NodeState::Merged,
    NodeState::Draft,
];

fn state_label(s: &NodeState) -> &'static str {
    match s {
        NodeState::Open   => "open",
        NodeState::Closed => "closed",
        NodeState::Merged => "merged",
        NodeState::Draft  => "draft",
    }
}

pub fn render_filter(f: &mut Frame, area: Rect, app: &App) {
    let graph = app.graph.read().unwrap();
    let milestones = &graph.milestones;
    let labels = &graph.labels;

    // アクティブフィルタ数
    let active_count = app.milestone_filter.is_some() as usize
        + app.active_labels.len()
        + app.active_states.len();
    let title = if active_count > 0 {
        format!(" Filter ({} active) [Esc to close] ", active_count)
    } else {
        " Filter [Esc to close] ".to_string()
    };

    // パネル高さをコンテンツに合わせる
    let ms_count = milestones.len().min(6);
    let lb_count = labels.len().min(6);
    let height = (3 + ms_count + 1 + lb_count + 1 + STATES.len() + 2) as u16;
    let width = 44u16;

    let panel_area = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width: width.min(area.width),
        height: height.min(area.height),
    };

    // ---- Milestones セクション ----
    // cursor は全フラット項目リストの位置
    // flat list: milestones (0..ms_count), labels (ms_count..ms_count+lb_count), states
    let ms_end = ms_count;
    let lb_end = ms_end + lb_count;

    let mut lines: Vec<Line> = Vec::new();

    // Section header: Milestones
    let ms_header_style = if app.filter_section == FilterSection::Milestones {
        theme::style_title()
    } else {
        theme::style_dimmed()
    };
    lines.push(Line::from(Span::styled("Milestones:", ms_header_style)));

    if milestones.is_empty() {
        lines.push(Line::from(Span::styled("  (none)", theme::style_dimmed())));
    } else {
        for (i, ms) in milestones.iter().take(6).enumerate() {
            let is_cursor = app.filter_section == FilterSection::Milestones && app.filter_cursor == i;
            let is_active = app.milestone_filter.as_deref() == Some(ms.as_str());
            let checkbox = if is_active { "[x]" } else { "[ ]" };
            let line_style = if is_cursor {
                theme::style_selected()
            } else if is_active {
                Style::default().fg(theme::color_accent()).add_modifier(Modifier::BOLD)
            } else {
                theme::style_normal()
            };
            lines.push(Line::from(Span::styled(
                format!(" {} {}", checkbox, ms),
                line_style,
            )));
        }
    }

    lines.push(Line::from(""));

    // Section header: Labels
    let lb_header_style = if app.filter_section == FilterSection::Labels {
        theme::style_title()
    } else {
        theme::style_dimmed()
    };
    lines.push(Line::from(Span::styled("Labels:", lb_header_style)));

    if labels.is_empty() {
        lines.push(Line::from(Span::styled("  (none)", theme::style_dimmed())));
    } else {
        for (i, label) in labels.iter().take(6).enumerate() {
            let cursor_i = i;
            let is_cursor = app.filter_section == FilterSection::Labels && app.filter_cursor == cursor_i;
            let is_active = app.active_labels.contains(label);
            let checkbox = if is_active { "[x]" } else { "[ ]" };
            let label_color = theme::label_color(label);
            let line_style = if is_cursor {
                theme::style_selected()
            } else if is_active {
                Style::default().fg(label_color).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(label_color)
            };
            lines.push(Line::from(Span::styled(
                format!(" {} {}", checkbox, label),
                line_style,
            )));
        }
    }

    lines.push(Line::from(""));

    // Section header: States
    let st_header_style = if app.filter_section == FilterSection::States {
        theme::style_title()
    } else {
        theme::style_dimmed()
    };
    lines.push(Line::from(Span::styled("States:", st_header_style)));

    for (i, state) in STATES.iter().enumerate() {
        let is_cursor = app.filter_section == FilterSection::States && app.filter_cursor == i;
        let is_active = app.active_states.contains(state);
        let checkbox = if is_active { "[x]" } else { "[ ]" };
        let line_style = if is_cursor {
            theme::style_selected()
        } else if is_active {
            Style::default().fg(theme::color_accent()).add_modifier(Modifier::BOLD)
        } else {
            theme::style_normal()
        };
        lines.push(Line::from(Span::styled(
            format!(" {} {}", checkbox, state_label(state)),
            line_style,
        )));
    }

    // フッター
    lines.push(Line::from(Span::styled(
        "[Tab] Section  [j/k] Move  [Space] Toggle  [c] Clear  [Esc] Close",
        theme::style_dimmed(),
    )));

    // 使わない変数を suppress
    let _ = ms_end;
    let _ = lb_end;

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme::style_border_focused())
        .style(Style::default().bg(theme::color_base()));

    let paragraph = Paragraph::new(lines).block(block);

    f.render_widget(Clear, panel_area);
    f.render_widget(paragraph, panel_area);
}
