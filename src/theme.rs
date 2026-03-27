use crate::types::Priority;
use catppuccin::PALETTE;
use ratatui::style::{Color, Modifier, Style};

// Catppuccin Mocha パレット
fn mocha_color(name: &str) -> Color {
    let flavor = &PALETTE.mocha;
    let c = match name {
        "rosewater" => flavor.colors.rosewater.rgb,
        "flamingo"  => flavor.colors.flamingo.rgb,
        "pink"      => flavor.colors.pink.rgb,
        "mauve"     => flavor.colors.mauve.rgb,
        "red"       => flavor.colors.red.rgb,
        "maroon"    => flavor.colors.maroon.rgb,
        "peach"     => flavor.colors.peach.rgb,
        "yellow"    => flavor.colors.yellow.rgb,
        "green"     => flavor.colors.green.rgb,
        "teal"      => flavor.colors.teal.rgb,
        "sky"       => flavor.colors.sky.rgb,
        "sapphire"  => flavor.colors.sapphire.rgb,
        "blue"      => flavor.colors.blue.rgb,
        "lavender"  => flavor.colors.lavender.rgb,
        "text"      => flavor.colors.text.rgb,
        "subtext1"  => flavor.colors.subtext1.rgb,
        "subtext0"  => flavor.colors.subtext0.rgb,
        "overlay2"  => flavor.colors.overlay2.rgb,
        "overlay1"  => flavor.colors.overlay1.rgb,
        "overlay0"  => flavor.colors.overlay0.rgb,
        "surface2"  => flavor.colors.surface2.rgb,
        "surface1"  => flavor.colors.surface1.rgb,
        "surface0"  => flavor.colors.surface0.rgb,
        "base"      => flavor.colors.base.rgb,
        "mantle"    => flavor.colors.mantle.rgb,
        "crust"     => flavor.colors.crust.rgb,
        _ => flavor.colors.text.rgb,
    };
    Color::Rgb(c.r, c.g, c.b)
}

// ノード状態別の色
pub fn node_color_open_issue()  -> Color { mocha_color("green") }
pub fn node_color_closed_issue() -> Color { mocha_color("red") }
pub fn node_color_open_pr()     -> Color { mocha_color("teal") }
pub fn node_color_merged_pr()   -> Color { mocha_color("mauve") }
pub fn node_color_draft_pr()    -> Color { mocha_color("yellow") }
pub fn node_color_selected()    -> Color { mocha_color("rosewater") }

// エッジ種別の色
pub fn edge_color_parent_child()     -> Color { mocha_color("blue") }
pub fn edge_color_closing_ref()      -> Color { mocha_color("green") }
pub fn edge_color_cross_ref()        -> Color { mocha_color("overlay1") }
pub fn edge_color_connected()        -> Color { mocha_color("sky") }
pub fn edge_color_body_mention()     -> Color { mocha_color("peach") }
pub fn edge_color_same_milestone()   -> Color { mocha_color("surface1") }
pub fn edge_color_duplicate()        -> Color { mocha_color("red") }

// UI 要素の色
pub fn color_text()      -> Color { mocha_color("text") }
pub fn color_subtext()   -> Color { mocha_color("subtext1") }
pub fn color_surface()   -> Color { mocha_color("surface0") }
pub fn color_overlay()   -> Color { mocha_color("overlay0") }
pub fn color_base()      -> Color { mocha_color("base") }
pub fn color_header_bg() -> Color { mocha_color("mantle") }
pub fn color_accent()    -> Color { mocha_color("lavender") }

// スタイル定義
pub fn style_title() -> Style {
    Style::default()
        .fg(color_accent())
        .add_modifier(Modifier::BOLD)
}

pub fn style_normal() -> Style {
    Style::default().fg(color_text())
}

pub fn style_dimmed() -> Style {
    Style::default().fg(color_subtext())
}

pub fn style_selected() -> Style {
    Style::default()
        .fg(color_base())
        .bg(color_accent())
        .add_modifier(Modifier::BOLD)
}

pub fn style_border() -> Style {
    Style::default().fg(color_overlay())
}

pub fn style_border_focused() -> Style {
    Style::default().fg(color_accent())
}

pub fn style_status_bar() -> Style {
    Style::default().fg(color_text()).bg(color_header_bg())
}

pub fn style_loading() -> Style {
    Style::default()
        .fg(mocha_color("yellow"))
        .add_modifier(Modifier::BOLD)
}

/// 優先度に応じた色
pub fn priority_color(priority: &Priority) -> Color {
    match priority {
        Priority::Critical => mocha_color("red"),
        Priority::High     => mocha_color("peach"),
        Priority::Medium   => mocha_color("yellow"),
        Priority::Low      => mocha_color("overlay2"),
        Priority::None     => mocha_color("text"),
    }
}

/// ラベル名のハッシュから Catppuccin カラーを割り当てる
pub fn label_color(label: &str) -> Color {
    const PALETTE_NAMES: &[&str] = &[
        "flamingo", "pink", "mauve", "peach", "yellow",
        "green", "teal", "sapphire", "lavender",
    ];
    let hash = label.bytes().fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
    mocha_color(PALETTE_NAMES[hash % PALETTE_NAMES.len()])
}
