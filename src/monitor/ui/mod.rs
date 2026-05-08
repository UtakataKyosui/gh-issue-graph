pub mod detail;
pub mod effects;
pub mod filter_bar;
pub mod footer;
pub mod header;
pub mod issue_table;
pub mod milestone_picker;
pub mod theme;

use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::Block,
    Frame,
};
use tachyonfx::EffectRenderer as _;

use crate::monitor::app::{App, EffectKey, InputMode};

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let show_filter_bar = app.input_mode == InputMode::FilterInput
        || app.input_mode == InputMode::MilestonePicker
        || !app.filter.is_empty();
    let filter_height = if show_filter_bar { 3u16 } else { 0u16 };

    let [header_area, filter_area, table_area, detail_area, footer_area] =
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(filter_height),
            Constraint::Fill(1),
            Constraint::Length(10),
            Constraint::Length(3),
        ])
        .areas(area);

    frame.render_widget(Block::new().style(Style::new().bg(theme::BASE)), area);

    header::render(frame, header_area, app);
    if show_filter_bar {
        filter_bar::render(frame, filter_area, app);
    }
    issue_table::render(frame, table_area, app);
    detail::render(frame, detail_area, app);
    footer::render(frame, footer_area, app);

    let last_tick = app.last_tick;
    let keys: Vec<EffectKey> = app.effects.keys().cloned().collect();
    for key in keys {
        let eff_area = match key {
            EffectKey::Startup => area,
            EffectKey::TabSwitch | EffectKey::DataLoaded => table_area,
            EffectKey::FilterToggle | EffectKey::MilestoneToggle => filter_area,
        };
        if let Some(effect) = app.effects.get_mut(&key) {
            if effect.running() {
                frame.render_effect(effect, eff_area, last_tick);
            }
        }
    }

    if app.input_mode == InputMode::MilestonePicker {
        milestone_picker::render(frame, app);
    }
}
