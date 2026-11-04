//! Tailwind-compatible fixed-size and maximum-width scales.

use gpui::{Pixels, Rems, px};

use super::rems_from_design_pixels;

pub const SIZE_2: Rems = rems_from_design_pixels(8.0);
pub const SIZE_3: Rems = rems_from_design_pixels(12.0);
pub const SIZE_3_5: Rems = rems_from_design_pixels(14.0);
pub const SIZE_4: Rems = rems_from_design_pixels(16.0);
pub const SIZE_5: Rems = rems_from_design_pixels(20.0);
pub const SIZE_6: Rems = rems_from_design_pixels(24.0);
pub const SIZE_7: Rems = rems_from_design_pixels(28.0);
pub const SIZE_8: Rems = rems_from_design_pixels(32.0);
pub const SIZE_9: Rems = rems_from_design_pixels(36.0);
pub const SIZE_10: Rems = rems_from_design_pixels(40.0);
pub const SIZE_12: Rems = rems_from_design_pixels(48.0);
pub const SIZE_14: Rems = rems_from_design_pixels(56.0);
pub const SIZE_16: Rems = rems_from_design_pixels(64.0);
pub const SIZE_26: Rems = rems_from_design_pixels(104.0);

pub struct MaxWidth;

impl MaxWidth {
    pub const EXTRA_SMALL: Pixels = px(320.0);
    pub const SMALL: Pixels = px(384.0);
    pub const MEDIUM: Pixels = px(448.0);
    pub const LARGE: Pixels = px(512.0);
    pub const EXTRA_LARGE: Pixels = px(576.0);
    pub const TWO_EXTRA_LARGE: Pixels = px(672.0);
    pub const THREE_EXTRA_LARGE: Pixels = px(768.0);
    pub const FOUR_EXTRA_LARGE: Pixels = px(896.0);
    pub const FIVE_EXTRA_LARGE: Pixels = px(1024.0);
    pub const SIX_EXTRA_LARGE: Pixels = px(1152.0);
    pub const SEVEN_EXTRA_LARGE: Pixels = px(1280.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_sizes_follow_the_window_rem_scale() {
        assert_eq!(SIZE_2.to_pixels(px(16.0)), px(8.0));
        assert_eq!(SIZE_3.to_pixels(px(16.0)), px(12.0));
        assert_eq!(SIZE_3_5.to_pixels(px(16.0)), px(14.0));
        assert_eq!(SIZE_4.to_pixels(px(16.0)), px(16.0));
        assert_eq!(SIZE_5.to_pixels(px(16.0)), px(20.0));
        assert_eq!(SIZE_6.to_pixels(px(16.0)), px(24.0));
        assert_eq!(SIZE_7.to_pixels(px(16.0)), px(28.0));
        assert_eq!(SIZE_8.to_pixels(px(16.0)), px(32.0));
        assert_eq!(SIZE_9.to_pixels(px(16.0)), px(36.0));
        assert_eq!(SIZE_10.to_pixels(px(16.0)), px(40.0));
        assert_eq!(SIZE_12.to_pixels(px(16.0)), px(48.0));
        assert_eq!(SIZE_14.to_pixels(px(16.0)), px(56.0));
        assert_eq!(SIZE_16.to_pixels(px(16.0)), px(64.0));
        assert_eq!(SIZE_26.to_pixels(px(16.0)), px(104.0));

        assert_eq!(SIZE_4.to_pixels(px(18.0)), px(18.0));
        assert_eq!(SIZE_8.to_pixels(px(18.0)), px(36.0));
    }

    #[test]
    fn maximum_widths_match_the_tailwind_scale() {
        assert_eq!(MaxWidth::TWO_EXTRA_LARGE, px(672.0));
        assert_eq!(MaxWidth::THREE_EXTRA_LARGE, px(768.0));
        assert_eq!(MaxWidth::SEVEN_EXTRA_LARGE, px(1280.0));
    }
}
