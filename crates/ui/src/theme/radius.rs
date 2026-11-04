//! shadcn-compatible radius scale derived from the base radius.

use gpui::{App, Pixels, px};
use gpui_base::RadiusTokens;

pub const RADIUS_SM: Pixels = px(7.92);
pub const RADIUS_MD: Pixels = px(10.56);
pub const RADIUS_LG: Pixels = px(13.2);
pub const RADIUS_XL: Pixels = px(18.48);
pub const RADIUS_2XL: Pixels = px(23.76);
pub const RADIUS_3XL: Pixels = px(29.04);
pub const RADIUS_4XL: Pixels = px(34.32);
pub const RADIUS_FULL: Pixels = px(9999.0);

#[must_use]
pub fn tokens() -> RadiusTokens {
    debug_assert!(RADIUS_2XL < RADIUS_3XL);
    debug_assert!(RADIUS_3XL < RADIUS_4XL);

    RadiusTokens {
        none: px(0.0),
        sm: RADIUS_SM,
        md: RADIUS_MD,
        lg: RADIUS_LG,
        xl: RADIUS_XL,
        full: RADIUS_FULL,
    }
}

pub fn sync_base(cx: &mut App) {
    gpui_base::Theme::global_mut(cx).tokens.radius = tokens();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_scale_matches_shadcn_formula() {
        assert_eq!(RADIUS_SM, px(7.92));
        assert_eq!(RADIUS_MD, px(10.56));
        assert_eq!(RADIUS_LG, px(13.2));
        assert_eq!(RADIUS_XL, px(18.48));
        assert_eq!(RADIUS_2XL, px(23.76));
        assert_eq!(RADIUS_3XL, px(29.04));
        assert_eq!(RADIUS_4XL, px(34.32));
        assert_eq!(RADIUS_FULL, px(9999.0));
    }
}
