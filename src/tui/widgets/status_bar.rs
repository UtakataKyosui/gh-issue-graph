use crate::theme;
use crate::tui::app::{App, AppMode};
use ratatui::{prelude::*, widgets::*};

pub fn render_status(f: &mut Frame, area: Rect, app: &App) {
    let mode_hint = match app.mode {
        AppMode::Normal => "[h/j/k/l] Pan  [+/-] Zoom  [Tab] Next  [Enter] Detail  [/] Search  [f] Filter  [L] Legend  [c] Cluster  [q] Quit",
        AppMode::Search => "[Type] Search  [Enter] Jump  [Esc] Cancel",
        AppMode::Filter => "[Tab] Section  [j/k] Move  [Space] Toggle  [c] Clear  [Esc] Close",
        AppMode::Detail => "[j/k] Scroll  [o] Open in browser  [Esc] Close",
    };

    let cluster_info = format!("Cluster:{}", app.cluster_mode);

    let status = if !app.status_message.is_empty() {
        format!("{}  {}  {}", app.status_message, cluster_info, mode_hint)
    } else {
        format!("{}  {}", cluster_info, mode_hint)
    };

    let paragraph = Paragraph::new(status)
        .style(theme::style_status_bar())
        .alignment(Alignment::Left);

    f.render_widget(paragraph, area);
}
