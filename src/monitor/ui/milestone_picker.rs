use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::monitor::app::App;

use super::theme;

pub fn render(frame: &mut Frame, app: &App) {
    let area = popup_area(frame.area(), 50, 60);

    frame.render_widget(Clear, area);

    let title = if app.filter.milestone.is_some() {
        format!(
            " Milestone: {} ",
            app.filter.milestone.as_deref().unwrap_or("")
        )
    } else {
        " Select Milestone ".to_string()
    };

    let block = Block::bordered()
        .border_type(theme::BORDER_TYPE)
        .border_style(theme::popup_border())
        .title(title)
        .title_alignment(Alignment::Center)
        .title_style(theme::title_style())
        .style(theme::popup_bg());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::new(
        Direction::Vertical,
        [Constraint::Fill(1), Constraint::Length(1)],
    )
    .split(inner);

    let state = app.monitor_state.read().unwrap();
    let repo = state.results.get(app.selected_tab);

    let mut items: Vec<ListItem> = Vec::new();

    items.push(ListItem::new(Line::from(vec![
        Span::styled("  All milestones", theme::popup_item()),
    ])));

    for ms_title in &app.milestone_list {
        let progress = repo
            .and_then(|r| {
                r.issues.iter().find_map(|i| {
                    i.milestone
                        .as_ref()
                        .filter(|m| m.title == *ms_title)
                        .map(|m| m.progress_percentage)
                })
            })
            .unwrap_or(0.0);

        let bar = progress_bar(progress, 10);
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  ", theme::popup_dim()),
            Span::styled(ms_title.clone(), theme::popup_item()),
            Span::styled(format!("  {bar} {:.0}%", progress), theme::popup_dim()),
        ])));
    }

    if items.len() == 1 {
        items.push(ListItem::new(Line::from(Span::styled(
            "  (no milestones found)",
            theme::popup_dim(),
        ))));
    }

    let mut list_state = ListState::default();
    list_state.select(Some(app.milestone_selected));

    let list = List::new(items)
        .highlight_style(theme::popup_highlight())
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, sections[0], &mut list_state);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled(" j/k", theme::key_badge()),
        Span::styled(" navigate  ", theme::key_desc()),
        Span::styled("Enter", theme::key_badge()),
        Span::styled(" select  ", theme::key_desc()),
        Span::styled("Esc", theme::key_badge()),
        Span::styled(" cancel", theme::key_desc()),
    ]));
    frame.render_widget(hint, sections[1]);
}

fn progress_bar(percent: f64, width: usize) -> String {
    let filled = ((percent / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::new(
        Direction::Vertical,
        [
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ],
    )
    .split(area);

    Layout::new(
        Direction::Horizontal,
        [
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ],
    )
    .split(vertical[1])[1]
}
