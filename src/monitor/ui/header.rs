use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Tabs},
    Frame,
};

use crate::monitor::app::App;

use super::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.monitor_state.read().unwrap();

    let tab_titles: Vec<Line> = state
        .results
        .iter()
        .map(|r| {
            if r.error.is_some() {
                Line::from(vec![
                    ratatui::text::Span::styled(
                        format!("{} ", theme::ERROR_ICON),
                        theme::error_title(),
                    ),
                    ratatui::text::Span::raw(format!("{}/{}", r.owner, r.name)),
                ])
            } else {
                let active_count = r.issues.iter().filter(|i| i.has_active_work()).count();
                let label = if active_count > 0 {
                    format!("{}/{} ({})", r.owner, r.name, active_count)
                } else {
                    format!("{}/{}", r.owner, r.name)
                };
                Line::from(label)
            }
        })
        .collect();

    let filter_indicator = if !app.filter.is_empty() { " [filtered]" } else { "" };
    let title = format!("gh issue-monitor{filter_indicator}");

    let show_spinner = state.loading;
    let [tabs_area, spinner_area] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(if show_spinner { 3 } else { 0 }),
    ])
    .areas(area);

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::bordered()
                .border_type(theme::BORDER_TYPE)
                .border_style(theme::border_style())
                .title(title)
                .title_style(theme::title_style()),
        )
        .highlight_style(theme::tab_active())
        .style(theme::tab_inactive())
        .select(app.selected_tab)
        .divider("│")
        .padding(" ", " ");

    frame.render_widget(tabs, tabs_area);

    if show_spinner {
        let throbber = throbber_widgets_tui::Throbber::default()
            .throbber_style(theme::spinner_style())
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX);
        let mut state = app.throbber_state.clone();
        frame.render_stateful_widget(throbber, spinner_area, &mut state);
    }
}
