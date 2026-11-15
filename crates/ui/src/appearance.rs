use crate::theme::{Theme, ThemePalette};
use gpui::{App, Hsla, WindowAppearance, rgb};

/// Mozu's native window material and status colors derived from its Kinu palette.
#[derive(Clone)]
pub struct MozuTheme {
    pub is_dark: bool,
    pub foreground: Hsla,
    pub muted_foreground: Hsla,
    pub border: Hsla,
    pub sidebar_divider: Hsla,
    pub destructive: Hsla,
    pub background_solid: Hsla,
    pub window_material: Hsla,
    pub blue_500: Hsla,
    pub orange_600: Hsla,
}
impl MozuTheme {
    pub fn global(cx: &App) -> Self {
        let palette = ThemePalette::current(cx);
        Self::from_palette(cx.window_appearance(), palette)
    }
    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        let palette = crate::theme::load(include_str!("theme.toml"), "theme.toml")
            .expect("Mozu Kinu theme must be valid");
        Self::from_palette(appearance, &palette)
    }
    fn from_palette(appearance: WindowAppearance, palette: &ThemePalette) -> Self {
        let is_dark = matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let colors: &Theme = if is_dark {
            &palette.dark
        } else {
            &palette.light
        };
        Self {
            is_dark,
            foreground: colors.foreground.into(),
            muted_foreground: colors.muted_foreground.into(),
            border: colors.border.into(),
            sidebar_divider: colors.sidebar_border.into(),
            destructive: colors.destructive.into(),
            background_solid: colors.background.into(),
            window_material: colors.card.into(),
            blue_500: colors.info.into(),
            orange_600: colors.warning.into(),
        }
    }
    pub fn control_track_fill(&self) -> Hsla {
        if self.is_dark {
            rgb(0x303030).into()
        } else {
            gpui::hsla(0., 0., 0., 0.04)
        }
    }
    pub fn control_hover_fill(&self) -> Hsla {
        if self.is_dark {
            rgb(0x474747).into()
        } else {
            self.control_track_fill()
        }
    }
    pub fn control_selected_fill(&self) -> Hsla {
        if self.is_dark {
            rgb(0x565656).into()
        } else {
            gpui::hsla(0., 0., 0., 0.07)
        }
    }
}
