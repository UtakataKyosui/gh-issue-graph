use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::monitor::app::{App, InputMode};

use super::theme;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [keys_area, gauge_area] =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(30)]).areas(area);

    render_keybindings(frame, keys_area, app);
    render_rate_gauge(frame, gauge_area, app);
}

fn render_keybindings(frame: &mut Frame, area: Rect, app: &App) {
    let line = match app.input_mode {
        InputMode::FilterInput => Line::from(vec![
            key_span("Enter"),
            desc_span(" apply "),
            key_span("Esc"),
            desc_span(" cancel"),
        ]),
        InputMode::MilestonePicker => Line::from(vec![
            key_span("j/k"),
            desc_span(" navigate "),
            key_span("Enter"),
            desc_span(" select "),
            key_span("Esc"),
            desc_span(" cancel"),
        ]),
        InputMode::Normal => Line::from(vec![
            key_span("j/k"),
            desc_span(" nav "),
            key_span("Tab"),
            desc_span(" repo "),
            key_span("/"),
            desc_span(" search "),
            key_span("m"),
            desc_span(" ms "),
            key_span("C"),
            desc_span(" clear "),
            key_span("r"),
            desc_span(" reload "),
            key_span("Esc"),
            desc_span(" quit"),
        ]),
    };

    let para = Paragraph::new(line).block(
        Block::bordered()
            .border_type(theme::BORDER_TYPE)
            .border_style(theme::border_style()),
    );
    frame.render_widget(para, area);
}

fn key_span(key: &'static str) -> Span<'static> {
    Span::styled(format!(" {key} "), theme::key_badge())
}

fn desc_span(text: &'static str) -> Span<'static> {
    Span::styled(text, theme::key_desc())
}

fn render_rate_gauge(frame: &mut Frame, area: Rect, app: &App) {
    let state = app.monitor_state.read().unwrap();

    let rate_limit = state
        .results
        .iter()
        .filter_map(|r| r.rate_limit.as_ref())
        .min_by_key(|rl| rl.remaining);

    let next_poll_secs = app.next_poll_secs();

    let line = if let Some(rl) = rate_limit {
        let ratio = rl.remaining as f64 / rl.limit.max(1) as f64;
        let count_style = if ratio > 0.5 {
            theme::gauge_healthy()
        } else if ratio > 0.2 {
            theme::gauge_warning()
        } else {
            theme::gauge_danger()
        };
        Line::from(vec![
            Span::styled(format!("{}", rl.remaining), count_style),
            Span::styled(format!("/{} ", rl.limit), theme::dimmed()),
            Span::styled(format!("· {next_poll_secs}s"), theme::dimmed()),
        ])
    } else {
        Line::from(Span::styled(format!("· {next_poll_secs}s"), theme::dimmed()))
    };

    let para = Paragraph::new(line).block(
        Block::bordered()
            .border_type(theme::BORDER_TYPE)
            .border_style(theme::border_style())
            .title("API")
            .title_style(theme::title_style()),
    );
    frame.render_widget(para, area);
}
