use tachyonfx::{fx, Effect, Interpolation, Motion};

use super::theme;

pub fn startup() -> Effect {
    fx::fade_from(theme::CRUST, theme::CRUST, (800, Interpolation::CubicOut))
}

pub fn tab_switch() -> Effect {
    fx::parallel(&[
        fx::fade_from_fg(theme::SURFACE0, (300, Interpolation::CubicOut)),
        fx::slide_in(Motion::LeftToRight, 3, 0, theme::BASE, (300, Interpolation::CubicOut)),
    ])
}

pub fn data_loaded() -> Effect {
    fx::coalesce((450, Interpolation::SineOut))
}

pub fn filter_toggle() -> Effect {
    fx::parallel(&[
        fx::fade_from(theme::BASE, theme::BASE, (220, Interpolation::CubicOut)),
        fx::slide_in(Motion::UpToDown, 2, 0, theme::BASE, (220, Interpolation::CubicOut)),
    ])
}
