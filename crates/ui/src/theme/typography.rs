//! Tailwind-compatible typography tokens for G2V's native UI.

use std::borrow::Cow;

use gpui::{App, FontWeight, SharedString, px};
use gpui_base::{TextStyleToken, TypographyTokens};

static FONT_INTER_REGULAR: &[u8] = include_bytes!("../../assets/fonts/Inter-Regular.ttf");
static FONT_INTER_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/Inter-SemiBold.ttf");
static FONT_JETBRAINS_MONO_LIGHT: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Light.ttf");
static FONT_JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontWeightTokens {
    pub thin: FontWeight,
    pub extra_light: FontWeight,
    pub light: FontWeight,
    pub normal: FontWeight,
    pub medium: FontWeight,
    pub semibold: FontWeight,
    pub bold: FontWeight,
    pub extra_bold: FontWeight,
    pub black: FontWeight,
}

pub const FONT_WEIGHT: FontWeightTokens = FontWeightTokens {
    thin: FontWeight::THIN,
    extra_light: FontWeight::EXTRA_LIGHT,
    light: FontWeight::LIGHT,
    normal: FontWeight::LIGHT,
    medium: FontWeight::MEDIUM,
    semibold: FontWeight::SEMIBOLD,
    bold: FontWeight::BOLD,
    extra_bold: FontWeight::EXTRA_BOLD,
    black: FontWeight::BLACK,
};

pub const TEXT_3XL: TextStyleToken = TextStyleToken {
    size: px(30.0),
    line_height: px(36.0),
    weight: FONT_WEIGHT.normal,
};

pub const STARTER_CARD_TEXT: TextStyleToken = TextStyleToken {
    size: px(13.0),
    line_height: px(18.85),
    weight: FONT_WEIGHT.normal,
};

pub const TEXT_13: TextStyleToken = TextStyleToken {
    size: px(13.0),
    line_height: px(18.0),
    weight: FONT_WEIGHT.normal,
};

pub const CODE_EDITOR_TEXT: TextStyleToken = TextStyleToken {
    size: px(12.0),
    line_height: px(18.0),
    weight: FONT_WEIGHT.normal,
};

#[must_use]
pub fn tokens() -> TypographyTokens {
    TypographyTokens {
        sans: sans_font_family(),
        mono: mono_font_family(),
        xs: text_style(12.0, 16.0),
        sm: text_style(14.0, 20.0),
        md: text_style(16.0, 24.0),
        lg: text_style(18.0, 28.0),
        xl: text_style(20.0, 28.0),
        mono_md: text_style(13.0, 20.0),
    }
}

pub fn install(cx: &mut App) {
    gpui_base::Theme::global_mut(cx).tokens.typography = tokens();
}

/// Registers G2V's bundled UI font before any window begins shaping text.
/// Registers the fonts bundled with G2V.
///
/// # Panics
///
/// Panics when GPUI cannot register one of the embedded font resources.
pub fn register_fonts(cx: &App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(FONT_INTER_REGULAR),
            Cow::Borrowed(FONT_INTER_SEMIBOLD),
            Cow::Borrowed(FONT_JETBRAINS_MONO_LIGHT),
            Cow::Borrowed(FONT_JETBRAINS_MONO_REGULAR),
        ])
        .expect("register bundled Inter and JetBrains Mono fonts");
}

pub fn sync_base(cx: &mut App) {
    gpui_base::Theme::global_mut(cx).tokens.typography = tokens();
}

const fn text_style(size: f32, line_height: f32) -> TextStyleToken {
    TextStyleToken {
        size: px(size),
        line_height: px(line_height),
        weight: FONT_WEIGHT.normal,
    }
}

fn sans_font_family() -> SharedString {
    "Inter".into()
}

fn mono_font_family() -> SharedString {
    "JetBrains Mono".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_scale_matches_tailwind_defaults() {
        let typography = tokens();

        assert_eq!(typography.sans, SharedString::from("Inter"));
        assert_eq!(typography.mono, SharedString::from("JetBrains Mono"));
        assert_eq!(typography.xs, text_style(12.0, 16.0));
        assert_eq!(typography.sm, text_style(14.0, 20.0));
        assert_eq!(typography.md, text_style(16.0, 24.0));
        assert_eq!(typography.lg, text_style(18.0, 28.0));
        assert_eq!(typography.xl, text_style(20.0, 28.0));
        assert_eq!(TEXT_3XL, text_style(30.0, 36.0));
        assert_eq!(TEXT_13, text_style(13.0, 18.0));
        assert_eq!(CODE_EDITOR_TEXT, text_style(12.0, 18.0));
    }

    #[test]
    fn font_weights_use_the_g2v_native_scale() {
        let expected = [
            100.0, 200.0, 300.0, 300.0, 500.0, 600.0, 700.0, 800.0, 900.0,
        ];
        let actual = [
            FONT_WEIGHT.thin.0,
            FONT_WEIGHT.extra_light.0,
            FONT_WEIGHT.light.0,
            FONT_WEIGHT.normal.0,
            FONT_WEIGHT.medium.0,
            FONT_WEIGHT.semibold.0,
            FONT_WEIGHT.bold.0,
            FONT_WEIGHT.extra_bold.0,
            FONT_WEIGHT.black.0,
        ];

        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() < f32::EPSILON);
        }
    }
}
