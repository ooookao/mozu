//! G2V design foundations for the GPUI viewport.

pub mod border;
pub mod colors;
pub mod motion;
pub mod radius;
pub mod shadows;
pub mod sizing;
pub mod spacing;
pub mod typography;

use gpui::{App, Pixels, Rems, WindowAppearance, px, rems};

pub const DEFAULT_BASE_FONT_SIZE: Pixels = px(16.0);

#[must_use]
pub const fn rems_from_design_pixels(pixels: f32) -> Rems {
    rems(pixels / 16.0)
}

#[must_use]
pub fn rems_from_pixels(pixels: Pixels) -> Rems {
    rems(f32::from(pixels) / f32::from(DEFAULT_BASE_FONT_SIZE))
}

pub fn install(appearance: WindowAppearance, cx: &mut App) {
    colors::install(appearance, cx);
    typography::sync_base(cx);
    spacing::sync_base(cx);
    radius::sync_base(cx);
    shadows::sync_base(appearance, cx);
    motion::install(cx);
    crate::ui::sync_base_theme(cx);
}
