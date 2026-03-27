use crate::theme;
use ratatui::{prelude::*, widgets::*};

pub fn render_legend(f: &mut Frame, area: Rect) {
    let legend_items = vec![
        ("● Open Issue",      theme::node_color_open_issue()),
        ("○ Closed Issue",    theme::node_color_closed_issue()),
        ("● Open PR",         theme::node_color_open_pr()),
        ("⬡ Merged PR",       theme::node_color_merged_pr()),
        ("◌ Draft PR",        theme::node_color_draft_pr()),
        ("─ Parent-Child",    theme::edge_color_parent_child()),
        ("- Closes",          theme::edge_color_closing_ref()),
        ("· Cross-ref",       theme::edge_color_cross_ref()),
        ("· Body mention",    theme::edge_color_body_mention()),
    ];

    let text: Vec<Line> = legend_items
        .iter()
        .map(|(label, color)| {
            Line::from(Span::styled(*label, Style::default().fg(*color)))
        })
        .collect();

    let width = 20u16;
    let height = (legend_items.len() as u16) + 2;

    let legend_area = Rect {
        x: area.x + area.width.saturating_sub(width + 1),
        y: area.y + area.height.saturating_sub(height + 1),
        width: width.min(area.width),
        height: height.min(area.height),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Legend ")
        .border_style(theme::style_border())
        .style(Style::default().bg(theme::color_base()));

    let paragraph = Paragraph::new(text)
        .block(block)
        .style(theme::style_normal());

    f.render_widget(paragraph, legend_area);
}
