//! Responsive layout queries derived from the current native viewport.

use gpui::{Pixels, Window, px};

pub struct Breakpoint;

#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the complete Tailwind breakpoint scale is a design API"
    )
)]
impl Breakpoint {
    pub const SMALL: Pixels = px(640.0);
    pub const MEDIUM: Pixels = px(768.0);
    pub const LARGE: Pixels = px(1024.0);
    pub const EXTRA_LARGE: Pixels = px(1280.0);
    pub const TWO_EXTRA_LARGE: Pixels = px(1536.0);
}

pub const COMPACT_BREAKPOINT: Pixels = Breakpoint::MEDIUM;

#[must_use]
pub fn is_compact(window: &Window) -> bool {
    is_compact_width(window.viewport_size().width)
}

fn is_compact_width(width: Pixels) -> bool {
    width < COMPACT_BREAKPOINT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_below_the_breakpoint_are_compact() {
        assert!(is_compact_width(px(767.0)));
    }

    #[test]
    fn breakpoint_and_wider_widths_are_not_compact() {
        assert!(!is_compact_width(px(768.0)));
        assert!(!is_compact_width(px(1024.0)));
    }

    #[test]
    fn breakpoints_match_the_tailwind_scale() {
        assert_eq!(Breakpoint::SMALL, px(640.0));
        assert_eq!(Breakpoint::MEDIUM, px(768.0));
        assert_eq!(Breakpoint::LARGE, px(1024.0));
        assert_eq!(Breakpoint::EXTRA_LARGE, px(1280.0));
        assert_eq!(Breakpoint::TWO_EXTRA_LARGE, px(1536.0));
    }
}
