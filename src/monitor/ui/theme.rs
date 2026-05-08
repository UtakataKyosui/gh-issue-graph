use ratatui::{
    style::{Color, Modifier, Style},
    widgets::BorderType,
};

const fn cc(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

pub const BASE: Color = cc(30, 30, 46);
pub const MANTLE: Color = cc(24, 24, 37);
pub const CRUST: Color = cc(17, 17, 27);
pub const SURFACE0: Color = cc(49, 50, 68);
#[allow(dead_code)]
pub const SURFACE1: Color = cc(69, 71, 90);
pub const OVERLAY0: Color = cc(108, 112, 134);
pub const SUBTEXT0: Color = cc(166, 173, 200);
pub const SUBTEXT1: Color = cc(186, 194, 222);
pub const TEXT: Color = cc(205, 214, 244);

pub const LAVENDER: Color = cc(180, 190, 254);
pub const BLUE: Color = cc(137, 180, 250);
pub const SAPPHIRE: Color = cc(116, 199, 236);
pub const TEAL: Color = cc(148, 226, 213);
pub const GREEN: Color = cc(166, 227, 161);
pub const YELLOW: Color = cc(249, 226, 175);
pub const PEACH: Color = cc(250, 179, 135);
pub const RED: Color = cc(243, 139, 168);
pub const MAUVE: Color = cc(203, 166, 247);

pub const BORDER_TYPE: BorderType = BorderType::Rounded;

pub fn border_style() -> Style {
    Style::new().fg(OVERLAY0)
}

pub fn table_header() -> Style {
    Style::new()
        .fg(SUBTEXT1)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
}

pub fn table_row_normal() -> Style {
    Style::new().fg(TEXT)
}

pub fn table_row_alt() -> Style {
    Style::new().fg(TEXT).bg(MANTLE)
}

pub fn table_highlight() -> Style {
    Style::new()
        .fg(BASE)
        .bg(LAVENDER)
        .add_modifier(Modifier::BOLD)
}

pub const TABLE_HIGHLIGHT_SYMBOL: &str = " ▶ ";

pub fn status_active() -> Style {
    Style::new().fg(GREEN)
}

pub fn status_idle() -> Style {
    Style::new().fg(OVERLAY0)
}

pub const STATUS_ACTIVE_SYMBOL: &str = "◉";
pub const STATUS_IDLE_SYMBOL: &str = "◌";

pub fn pr_open() -> Style {
    Style::new().fg(GREEN)
}

pub fn pr_merged() -> Style {
    Style::new().fg(MAUVE)
}

pub fn pr_closed() -> Style {
    Style::new().fg(RED)
}

pub const ICON_BRANCH: &str = "⎇ ";
pub const ICON_PR: &str = "◨ ";
pub const ICON_WILL_CLOSE: &str = "↗";
pub const ICON_DRAFT: &str = "✏";
pub const ICON_NO_PR: &str = "∅";

pub fn key_badge() -> Style {
    Style::new()
        .fg(BASE)
        .bg(LAVENDER)
        .add_modifier(Modifier::BOLD)
}

pub fn key_desc() -> Style {
    Style::new().fg(SUBTEXT0)
}

pub fn gauge_healthy() -> Style {
    Style::new().fg(GREEN)
}

pub fn gauge_warning() -> Style {
    Style::new().fg(YELLOW)
}

pub fn gauge_danger() -> Style {
    Style::new().fg(RED)
}

pub fn tab_active() -> Style {
    Style::new()
        .fg(BASE)
        .bg(LAVENDER)
        .add_modifier(Modifier::BOLD)
}

pub fn tab_inactive() -> Style {
    Style::new().fg(SUBTEXT0)
}

pub fn filter_label() -> Style {
    Style::new().fg(PEACH).add_modifier(Modifier::BOLD)
}

pub fn filter_cursor() -> Style {
    Style::new().fg(PEACH)
}

pub fn filter_border_active() -> Style {
    Style::new().fg(PEACH)
}

pub fn filter_border_inactive() -> Style {
    Style::new().fg(TEAL)
}

pub fn tree_connector() -> Style {
    Style::new().fg(OVERLAY0)
}

pub fn sub_issue_open() -> Style {
    Style::new().fg(GREEN)
}

pub fn sub_issue_closed() -> Style {
    Style::new().fg(OVERLAY0)
}

pub const SUB_ISSUE_OPEN_SYMBOL: &str = "○";
pub const SUB_ISSUE_CLOSED_SYMBOL: &str = "✓";

pub fn milestone_style() -> Style {
    Style::new().fg(SAPPHIRE)
}

pub fn popup_bg() -> Style {
    Style::new().bg(SURFACE0)
}

pub fn popup_border() -> Style {
    Style::new().fg(LAVENDER)
}

pub fn popup_highlight() -> Style {
    Style::new()
        .fg(BASE)
        .bg(SAPPHIRE)
        .add_modifier(Modifier::BOLD)
}

pub fn popup_item() -> Style {
    Style::new().fg(TEXT)
}

pub fn popup_dim() -> Style {
    Style::new().fg(OVERLAY0)
}

pub fn relationship_style() -> Style {
    Style::new().fg(MAUVE)
}

pub const ICON_TRACKS: &str = "→";
pub const ICON_TRACKED_BY: &str = "←";
pub const ICON_DUPLICATE: &str = "≡";
pub const ICON_RELATIONSHIP: &str = "⇆";

pub fn error_title() -> Style {
    Style::new()
        .fg(RED)
        .add_modifier(Modifier::BOLD)
}

pub fn error_body() -> Style {
    Style::new().fg(SUBTEXT0)
}

pub fn error_hint() -> Style {
    Style::new().fg(OVERLAY0)
}

pub const ERROR_ICON: &str = "✗";

pub fn spinner_style() -> Style {
    Style::new().fg(SAPPHIRE).add_modifier(Modifier::BOLD)
}

pub fn title_style() -> Style {
    Style::new().fg(SUBTEXT1)
}

pub fn dimmed() -> Style {
    Style::new().fg(OVERLAY0)
}

pub fn accent() -> Style {
    Style::new().fg(BLUE)
}
