use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::monitor::app::{App, InputMode};

use super::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans: Vec<Span> = Vec::new();

    match app.input_mode {
        InputMode::FilterInput => {
            spans.push(Span::styled("Search: ", theme::filter_label()));
            let before = &app.filter_input[..app.filter_cursor];
            let after = &app.filter_input[app.filter_cursor..];
            spans.push(Span::raw(before.to_string()));
            spans.push(Span::styled("█", theme::filter_cursor()));
            spans.push(Span::raw(after.to_string()));
        }
        InputMode::Normal | InputMode::MilestonePicker => {
            if !app.filter.labels.is_empty() {
                spans.push(Span::styled("Labels: ", theme::filter_label()));
                spans.push(Span::raw(app.filter.labels.join(", ")));
                if !app.filter.keywords.is_empty() || app.filter.milestone.is_some() {
                    spans.push(Span::raw("  "));
                }
            }
            if !app.filter.keywords.is_empty() {
                spans.push(Span::styled("Keyword: ", theme::filter_label()));
                spans.push(Span::raw(app.filter.keywords.join(", ")));
                if app.filter.milestone.is_some() {
                    spans.push(Span::raw("  "));
                }
            }
            if let Some(ms) = &app.filter.milestone {
                spans.push(Span::styled("Milestone: ", theme::filter_label()));
                spans.push(Span::styled(ms.clone(), theme::milestone_style()));
            }
        }
    }

    let border_style = if app.input_mode == InputMode::FilterInput {
        theme::filter_border_active()
    } else {
        theme::filter_border_inactive()
    };

    let block = Block::bordered()
        .border_type(theme::BORDER_TYPE)
        .border_style(border_style)
        .title("Filter")
        .title_style(theme::title_style());

    let para = Paragraph::new(Line::from(spans)).block(block);
    frame.render_widget(para, area);
}
