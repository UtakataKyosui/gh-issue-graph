use crate::theme;
use crate::tui::app::App;
use ratatui::{prelude::*, widgets::*};

pub fn render_search(f: &mut Frame, area: Rect, app: &App) {
    let search_height = 3u16;
    let search_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(search_height + 1),
        width: area.width.saturating_sub(4).min(60),
        height: search_height,
    };

    let text = format!("Search: {}_", app.search_query);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" / Search ")
        .border_style(theme::style_border_focused());

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(theme::style_normal());

    f.render_widget(Clear, search_area);
    f.render_widget(paragraph, search_area);
}
