use crate::theme;
use crate::tui::app::App;
use ratatui::{prelude::*, widgets::*};

pub fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let graph = app.graph.read().unwrap();
    let status = if app.loading {
        "[ Loading... ]".to_string()
    } else {
        format!(
            "[ nodes: {}  edges: {} ]",
            graph.node_count(),
            graph.edge_count()
        )
    };
    drop(graph);

    let text = Line::from(vec![
        Span::styled(" gh-issue-graph ", theme::style_title()),
        Span::raw("  "),
        Span::styled(status, theme::style_dimmed()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::style_border())
        .style(Style::default().bg(theme::color_header_bg()));

    let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);
    f.render_widget(paragraph, area);
}
