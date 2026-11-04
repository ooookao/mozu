//! Shared adapters that apply typed design tokens to GPUI elements.

use gpui::{Pixels, Rems, Styled, rems};
use gpui_base::TextStyleToken;

use crate::theme::DEFAULT_BASE_FONT_SIZE;

pub trait TextStyleTokenExt {
    fn font_size_rems(self) -> Rems;
    fn line_height_rems(self) -> Rems;
    fn font_size_pixels(self, rem_size: Pixels) -> Pixels;
}

impl TextStyleTokenExt for TextStyleToken {
    fn font_size_rems(self) -> Rems {
        rems(f32::from(self.size) / f32::from(DEFAULT_BASE_FONT_SIZE))
    }

    fn line_height_rems(self) -> Rems {
        rems(f32::from(self.line_height) / f32::from(DEFAULT_BASE_FONT_SIZE))
    }

    fn font_size_pixels(self, rem_size: Pixels) -> Pixels {
        self.font_size_rems().to_pixels(rem_size)
    }
}

pub trait TextStyleExt: Styled + Sized {
    #[must_use]
    fn text_style(self, style: TextStyleToken) -> Self {
        self.text_size(style.font_size_rems())
            .line_height(style.line_height_rems())
            .font_weight(style.weight)
    }
}

impl<T> TextStyleExt for T where T: Styled + Sized {}

#[cfg(test)]
mod tests {
    use gpui::{FontWeight, px};

    use super::*;

    #[test]
    fn text_tokens_follow_the_window_rem_size() {
        let token = TextStyleToken {
            size: px(14.0),
            line_height: px(20.0),
            weight: FontWeight::NORMAL,
        };

        assert_eq!(token.font_size_pixels(px(16.0)), px(14.0));
        assert_eq!(token.font_size_pixels(px(18.0)), px(15.75));
        assert_eq!(token.line_height_rems().to_pixels(px(18.0)), px(22.5));
    }
}
