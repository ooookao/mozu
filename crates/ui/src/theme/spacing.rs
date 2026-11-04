//! Tailwind-compatible spacing scale for native layout.

use gpui::{App, Pixels, px};
use gpui_base::SpacingTokens;

pub const SPACE_0_5: Pixels = px(2.0);
pub const SPACE_1: Pixels = px(4.0);
pub const SPACE_1_5: Pixels = px(6.0);
pub const SPACE_2: Pixels = px(8.0);
pub const SPACE_2_5: Pixels = px(10.0);
pub const SPACE_3: Pixels = px(12.0);
pub const SPACE_3_5: Pixels = px(14.0);
pub const SPACE_4: Pixels = px(16.0);
#[allow(dead_code)]
pub const SPACE_5: Pixels = px(20.0);
pub const SPACE_6: Pixels = px(24.0);
pub const SPACE_7: Pixels = px(28.0);
pub const SPACE_8: Pixels = px(32.0);
pub const SPACE_10: Pixels = px(40.0);
pub const SPACE_4_5: Pixels = px(18.0);
#[allow(dead_code)]
pub const SPACE_18: Pixels = px(72.0);

#[must_use]
pub fn tokens() -> SpacingTokens {
    SpacingTokens {
        xxs: px(2.0),
        xs: px(4.0),
        sm: px(8.0),
        md: px(12.0),
        lg: SPACE_4,
        xl: px(24.0),
        xxl: px(32.0),
    }
}

pub fn sync_base(cx: &mut App) {
    gpui_base::Theme::global_mut(cx).tokens.spacing = tokens();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_uses_tailwind_four_pixel_grid() {
        assert_eq!(SPACE_0_5, px(2.0));
        assert_eq!(SPACE_1, px(4.0));
        assert_eq!(SPACE_1_5, px(6.0));
        assert_eq!(SPACE_2_5, px(10.0));
        assert_eq!(SPACE_3_5, px(14.0));
        assert_eq!(SPACE_5, px(20.0));
        assert_eq!(SPACE_6, px(24.0));
        assert_eq!(SPACE_7, px(28.0));
        assert_eq!(SPACE_8, px(32.0));
        assert_eq!(SPACE_10, px(40.0));
        assert_eq!(SPACE_18, px(72.0));
    }
}
