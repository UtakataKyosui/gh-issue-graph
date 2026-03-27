use crate::theme;
use crate::tui::app::App;
use ratatui::{prelude::*, widgets::*};

pub fn render_detail(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Detail ")
        .border_style(theme::style_border_focused());

    if let Some(lines) = app.selected_node_detail() {
        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .scroll((app.detail_scroll, 0));
        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("No node selected")
            .block(block)
            .style(theme::style_dimmed());
        f.render_widget(paragraph, area);
    }
}
