//! Typed shadcn-style semantic color tokens for G2V's native UI.

use std::f32::consts::PI;

use gpui::{App, Global, Hsla, Pixels, Rgba, WindowAppearance, px};
use gpui_base::ColorTokens;

use super::DEFAULT_BASE_FONT_SIZE;

#[derive(Clone)]
pub struct G2VTheme {
    pub is_dark: bool,
    pub base_font_size: Pixels,
    pub background: Hsla,
    pub background_solid: Hsla,
    pub composer: Hsla,
    pub composer_hover: Hsla,
    pub composer_selected: Hsla,
    pub composer_rail: Hsla,
    pub foreground: Hsla,
    pub card: Hsla,
    pub card_hover: Hsla,
    pub card_selected: Hsla,
    pub card_foreground: Hsla,
    pub popover: Hsla,
    pub popover_hover: Hsla,
    pub popover_selected: Hsla,
    pub popover_border: Hsla,
    pub popover_foreground: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_hover: Hsla,
    pub secondary_foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub subtle_foreground: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub bubble_tinted: Hsla,
    pub bubble_tinted_hover: Hsla,
    pub border: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
    pub tab: Hsla,
    pub surface: Hsla,
    pub surface_hover: Hsla,
    pub surface_selected: Hsla,
    pub surface_foreground: Hsla,
    pub surface_border: Hsla,
    pub chart_1: Hsla,
    pub chart_2: Hsla,
    pub chart_3: Hsla,
    pub chart_4: Hsla,
    pub chart_5: Hsla,
    pub blue_500: Hsla,
    pub violet_500: Hsla,
    pub green_600: Hsla,
    pub orange_600: Hsla,
    pub radius: Pixels,
    pub sidebar: Hsla,
    pub sidebar_foreground: Hsla,
    pub sidebar_primary: Hsla,
    pub sidebar_primary_foreground: Hsla,
    pub sidebar_accent: Hsla,
    pub sidebar_accent_foreground: Hsla,
    pub sidebar_border: Hsla,
    pub sidebar_divider: Hsla,
    pub sidebar_ring: Hsla,
    pub window_material: Hsla,
}

impl Global for G2VTheme {}

impl G2VTheme {
    #[must_use]
    pub fn for_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
        }
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn control_track_fill(&self) -> Hsla {
        if self.is_dark {
            // Solid fills keep the contrast stable over the translucent macOS backdrop.
            gpui::rgb(0x303030).into()
        } else {
            gpui::hsla(0.0, 0.0, 0.0, 0.04)
        }
    }

    pub fn control_hover_fill(&self) -> Hsla {
        if self.is_dark {
            gpui::rgb(0x474747).into()
        } else {
            self.control_track_fill()
        }
    }

    pub fn control_selected_fill(&self) -> Hsla {
        if self.is_dark {
            gpui::rgb(0x565656).into()
        } else {
            gpui::hsla(0.0, 0.0, 0.0, 0.07)
        }
    }

    fn light() -> Self {
        let foreground = oklch(0.2046, 0.0, 0.0, 1.0);
        let surface = oklch(1.0, 0.0, 0.0, 1.0);
        let muted = oklch(0.9642, 0.0, 0.0, 1.0);
        let border = oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.85);
        Self {
            is_dark: false,
            base_font_size: DEFAULT_BASE_FONT_SIZE,
            background: oklch(1.0, 0.0, 0.0, 0.5),
            background_solid: oklch(1.0, 0.0, 0.0, 1.0),
            composer: oklch(1.0, 0.0, 0.0, 1.0),
            composer_hover: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.95),
            composer_selected: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.9),
            composer_rail: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.9642, 0.0, 0.0, 1.0), 0.6),
            foreground,
            card: oklch(1.0, 0.0, 0.0, 1.0),
            card_hover: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.95),
            card_selected: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.9),
            card_foreground: foreground,
            popover: oklch(1.0, 0.0, 0.0, 1.0),
            popover_hover: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.95),
            popover_selected: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.9),
            popover_border: foreground.opacity(0.12),
            popover_foreground: foreground,
            primary: foreground,
            primary_foreground: oklch(0.9791, 0.0, 0.0, 1.0),
            secondary: muted,
            secondary_hover: oklch_mix((0.9642, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.95),
            secondary_foreground: foreground,
            muted,
            muted_foreground: oklch(0.5417, 0.0, 0.0, 1.0),
            subtle_foreground: oklch(0.5784, 0.0, 0.0, 1.0),
            accent: muted,
            accent_foreground: foreground,
            destructive: oklch(0.5318, 0.1976, 27.905, 1.0),
            bubble_tinted: oklch(0.93, 0.0, 0.0, 1.0),
            bubble_tinted_hover: oklch(0.88, 0.0, 0.0, 1.0),
            border,
            input: border,
            ring: oklch(0.546, 0.245, 262.881, 1.0),
            tab: muted.opacity(0.6),
            chart_1: oklch(0.87, 0.0, 0.0, 1.0),
            chart_2: oklch(0.556, 0.0, 0.0, 1.0),
            chart_3: oklch(0.439, 0.0, 0.0, 1.0),
            chart_4: oklch(0.371, 0.0, 0.0, 1.0),
            chart_5: oklch(0.269, 0.0, 0.0, 1.0),
            blue_500: oklch(0.623, 0.214, 259.815, 1.0),
            violet_500: oklch(0.606, 0.25, 292.717, 1.0),
            green_600: oklch(0.627, 0.194, 149.214, 1.0),
            orange_600: oklch(0.646, 0.222, 41.116, 1.0),
            radius: px(13.2),
            surface,
            surface_hover: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.95),
            surface_selected: oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.9),
            surface_foreground: foreground,
            surface_border: border,
            sidebar: oklch(0.0, 0.0, 0.0, 0.0),
            sidebar_foreground: foreground,
            sidebar_primary: foreground,
            sidebar_primary_foreground: oklch(0.9791, 0.0, 0.0, 1.0),
            sidebar_accent: oklch(0.0, 0.0, 0.0, 0.07),
            sidebar_accent_foreground: foreground,
            sidebar_border: border,
            sidebar_divider: border,
            sidebar_ring: oklch(0.546, 0.245, 262.881, 1.0),
            window_material: oklch(0.98, 0.0, 0.0, 0.9),
        }
    }

    fn dark() -> Self {
        let foreground = oklch(0.9461, 0.0, 0.0, 1.0);
        let surface = oklch(0.2103, 0.0059, 285.885, 1.0);
        let popover = oklch_mix((0.2103, 0.0059, 285.885, 1.0), (0.9461, 0.0, 0.0, 1.0), 0.9);
        let popover_oklch = (0.28388, 0.00531, 285.885, 1.0);
        let muted = oklch(0.2768, 0.0, 0.0, 1.0);
        let border = oklch_mix(
            (0.2103, 0.0059, 285.885, 1.0),
            (0.9461, 0.0, 0.0, 1.0),
            0.85,
        );
        Self {
            is_dark: true,
            base_font_size: DEFAULT_BASE_FONT_SIZE,
            background: oklch(0.2478, 0.0, 0.0, 0.6),
            background_solid: oklch(0.1452, 0.0021, 286.131, 1.0),
            composer: muted,
            composer_hover: oklch_mix((0.2768, 0.0, 0.0, 1.0), (0.9461, 0.0, 0.0, 1.0), 0.95),
            composer_selected: oklch_mix((0.2768, 0.0, 0.0, 1.0), (0.9461, 0.0, 0.0, 1.0), 0.9),
            composer_rail: oklch_mix((0.2103, 0.0059, 285.885, 1.0), (0.2768, 0.0, 0.0, 1.0), 0.6),
            foreground,
            card: oklch(0.2264, 0.0, 0.0, 1.0),
            card_hover: oklch_mix((0.2264, 0.0, 0.0, 1.0), (0.9461, 0.0, 0.0, 1.0), 0.95),
            card_selected: oklch_mix((0.2264, 0.0, 0.0, 1.0), (0.9461, 0.0, 0.0, 1.0), 0.9),
            card_foreground: foreground,
            popover,
            popover_hover: oklch_mix(popover_oklch, (0.9461, 0.0, 0.0, 1.0), 0.95),
            popover_selected: oklch_mix(popover_oklch, (0.9461, 0.0, 0.0, 1.0), 0.9),
            popover_border: foreground.opacity(0.12),
            popover_foreground: foreground,
            primary: foreground,
            primary_foreground: oklch(0.2002, 0.0, 0.0, 1.0),
            secondary: oklch(0.2562, 0.0, 0.0, 1.0),
            secondary_hover: oklch_mix((0.2562, 0.0, 0.0, 1.0), (0.9461, 0.0, 0.0, 1.0), 0.95),
            secondary_foreground: foreground,
            muted,
            muted_foreground: oklch(0.7058, 0.0, 0.0, 1.0),
            subtle_foreground: oklch(0.6662, 0.0, 0.0, 1.0),
            accent: muted,
            accent_foreground: foreground,
            destructive: oklch(0.7449, 0.1269, 25.829, 1.0),
            bubble_tinted: oklch(0.3, 0.0, 0.0, 1.0),
            bubble_tinted_hover: oklch(0.35, 0.0, 0.0, 1.0),
            border,
            input: border,
            ring: oklch(0.623, 0.214, 259.815, 1.0),
            tab: muted.opacity(0.6),
            chart_1: oklch(0.87, 0.0, 0.0, 1.0),
            chart_2: oklch(0.556, 0.0, 0.0, 1.0),
            chart_3: oklch(0.439, 0.0, 0.0, 1.0),
            chart_4: oklch(0.371, 0.0, 0.0, 1.0),
            chart_5: oklch(0.269, 0.0, 0.0, 1.0),
            blue_500: oklch(0.623, 0.214, 259.815, 1.0),
            violet_500: oklch(0.606, 0.25, 292.717, 1.0),
            green_600: oklch(0.627, 0.194, 149.214, 1.0),
            orange_600: oklch(0.646, 0.222, 41.116, 1.0),
            radius: px(13.2),
            surface,
            surface_hover: oklch_mix(
                (0.2103, 0.0059, 285.885, 1.0),
                (0.9461, 0.0, 0.0, 1.0),
                0.95,
            ),
            surface_selected: oklch_mix(
                (0.2103, 0.0059, 285.885, 1.0),
                (0.9461, 0.0, 0.0, 1.0),
                0.9,
            ),
            surface_foreground: foreground,
            surface_border: border,
            sidebar: oklch(0.0, 0.0, 0.0, 0.0),
            sidebar_foreground: foreground,
            sidebar_primary: foreground,
            sidebar_primary_foreground: oklch(0.2002, 0.0, 0.0, 1.0),
            sidebar_accent: oklch(1.0, 0.0, 0.0, 0.10),
            sidebar_accent_foreground: foreground,
            sidebar_border: border,
            sidebar_divider: border,
            sidebar_ring: oklch(0.623, 0.214, 259.815, 1.0),
            window_material: oklch(0.2478, 0.0, 0.0, 0.9),
        }
    }

    fn apply_to_base_theme(&self, cx: &mut App) {
        debug_assert_eq!(self.sidebar_ring, self.ring);
        gpui_base::Theme::global_mut(cx).tokens.colors = ColorTokens {
            background: self.background,
            foreground: self.foreground,
            surface: self.popover,
            surface_foreground: self.popover_foreground,
            primary: self.primary,
            primary_foreground: self.primary_foreground,
            secondary: self.secondary,
            secondary_foreground: self.secondary_foreground,
            muted: self.muted,
            muted_foreground: self.muted_foreground,
            accent: self.accent,
            accent_foreground: self.accent_foreground,
            destructive: self.destructive,
            destructive_foreground: self.primary_foreground,
            border: self.border,
            input: self.input,
            ring: self.ring,
        };
    }
}

fn oklch_mix(first: (f32, f32, f32, f32), second: (f32, f32, f32, f32), first_weight: f32) -> Hsla {
    let second_weight = 1.0 - first_weight;
    let hue = if first.1.abs() < f32::EPSILON {
        second.2
    } else if second.1.abs() < f32::EPSILON {
        first.2
    } else {
        first.2 * first_weight + second.2 * second_weight
    };
    oklch(
        first.0 * first_weight + second.0 * second_weight,
        first.1 * first_weight + second.1 * second_weight,
        hue,
        first.3 * first_weight + second.3 * second_weight,
    )
}

pub fn install(appearance: WindowAppearance, cx: &mut App) {
    let theme = G2VTheme::for_appearance(appearance);
    theme.apply_to_base_theme(cx);
    cx.set_global(theme);
}

fn oklch(lightness: f32, chroma: f32, hue_degrees: f32, alpha: f32) -> Hsla {
    let hue = hue_degrees * PI / 180.0;
    let oklab_a = chroma * hue.cos();
    let oklab_b = chroma * hue.sin();
    let l_root = lightness + 0.396_337_78 * oklab_a + 0.215_803_76 * oklab_b;
    let m_root = lightness - 0.105_561_346 * oklab_a - 0.063_854_17 * oklab_b;
    let s_root = lightness - 0.089_484_18 * oklab_a - 1.291_485_5 * oklab_b;
    let linear_l = l_root.powi(3);
    let linear_m = m_root.powi(3);
    let linear_s = s_root.powi(3);
    let red =
        linear_to_srgb(4.076_741_7 * linear_l - 3.307_711_6 * linear_m + 0.230_969_94 * linear_s);
    let green =
        linear_to_srgb(-1.268_438 * linear_l + 2.609_757_4 * linear_m - 0.341_319_4 * linear_s);
    let blue = linear_to_srgb(
        -0.004_196_086_3 * linear_l - 0.703_418_6 * linear_m + 1.707_614_7 * linear_s,
    );
    Rgba {
        r: red.clamp(0.0, 1.0),
        g: green.clamp(0.0, 1.0),
        b: blue.clamp(0.0, 1.0),
        a: alpha.clamp(0.0, 1.0),
    }
    .into()
}

fn linear_to_srgb(channel: f32) -> f32 {
    if channel <= 0.003_130_8 {
        12.92 * channel
    } else {
        1.055 * channel.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_hsla_close(actual: Hsla, expected: Hsla) {
        const TOLERANCE: f32 = 1.0e-6;
        assert!((actual.h - expected.h).abs() < TOLERANCE);
        assert!((actual.s - expected.s).abs() < TOLERANCE);
        assert!((actual.l - expected.l).abs() < TOLERANCE);
        assert!((actual.a - expected.a).abs() < TOLERANCE);
    }

    #[test]
    fn oklch_neutral_endpoints_map_to_black_and_white() {
        let black: Rgba = oklch(0.0, 0.0, 0.0, 1.0).into();
        let white: Rgba = oklch(1.0, 0.0, 0.0, 1.0).into();
        assert!(black.r.abs() < 0.0001 && black.g.abs() < 0.0001 && black.b.abs() < 0.0001);
        assert!(
            (white.r - 1.0).abs() < 0.0001
                && (white.g - 1.0).abs() < 0.0001
                && (white.b - 1.0).abs() < 0.0001
        );
        assert!((white.a - 1.0).abs() < 0.0001);
    }

    #[test]
    fn window_material_preserves_translucency_in_both_themes() {
        let light: Rgba = G2VTheme::light().window_material.into();
        let dark: Rgba = G2VTheme::dark().window_material.into();
        assert_eq!(
            G2VTheme::light().window_material,
            oklch(0.98, 0.0, 0.0, 0.9)
        );
        assert!((light.a - 0.9).abs() < 0.0001);
        assert!((dark.a - 0.9).abs() < 0.0001);
    }

    #[test]
    fn desktop_surface_is_opaque_and_sidebar_uses_the_root_frost() {
        let light = G2VTheme::light();
        let dark = G2VTheme::dark();
        let light_sidebar: Rgba = light.sidebar.into();
        let dark_sidebar: Rgba = dark.sidebar.into();

        assert_eq!(light.surface, oklch(1.0, 0.0, 0.0, 1.0));
        assert_eq!(dark.surface, oklch(0.2103, 0.0059, 285.885, 1.0));
        assert_eq!(light.surface_foreground, light.foreground);
        assert_eq!(dark.surface_foreground, dark.foreground);
        assert_eq!(light.sidebar, oklch(0.0, 0.0, 0.0, 0.0));
        assert_eq!(dark.sidebar, oklch(0.0, 0.0, 0.0, 0.0));
        assert_eq!(light.sidebar_accent, oklch(0.0, 0.0, 0.0, 0.07));
        assert_eq!(dark.sidebar_accent, oklch(1.0, 0.0, 0.0, 0.10));
        assert!(light_sidebar.a.abs() < 0.0001);
        assert!(dark_sidebar.a.abs() < 0.0001);
    }

    #[test]
    fn theme_records_its_resolved_appearance() {
        assert!(!G2VTheme::for_appearance(WindowAppearance::Light).is_dark);
        assert!(G2VTheme::for_appearance(WindowAppearance::Dark).is_dark);
    }

    #[test]
    fn theme_owns_the_application_rem_baseline() {
        assert_eq!(G2VTheme::light().base_font_size, DEFAULT_BASE_FONT_SIZE);
        assert_eq!(G2VTheme::dark().base_font_size, DEFAULT_BASE_FONT_SIZE);
    }

    #[test]
    fn ring_colors_match_the_react_desktop_theme() {
        let light = G2VTheme::light();
        let dark = G2VTheme::dark();

        assert_eq!(light.ring, oklch(0.546, 0.245, 262.881, 1.0));
        assert_eq!(dark.ring, oklch(0.623, 0.214, 259.815, 1.0));
        assert_eq!(light.sidebar_ring, light.ring);
        assert_eq!(dark.sidebar_ring, dark.ring);
    }

    #[test]
    fn semantic_colors_match_the_react_desktop_theme() {
        let light = G2VTheme::light();
        let dark = G2VTheme::dark();

        assert_eq!(light.background, oklch(1.0, 0.0, 0.0, 0.5));
        assert_eq!(light.foreground, oklch(0.2046, 0.0, 0.0, 1.0));
        assert_eq!(light.secondary, oklch(0.9642, 0.0, 0.0, 1.0));
        assert_eq!(light.muted_foreground, oklch(0.5417, 0.0, 0.0, 1.0));
        assert_eq!(light.subtle_foreground, oklch(0.5784, 0.0, 0.0, 1.0));
        assert_eq!(light.surface_border, light.border);
        assert_eq!(light.popover_border, light.foreground.opacity(0.12));
        assert_eq!(light.sidebar_divider, light.border);

        assert_eq!(dark.background, oklch(0.2478, 0.0, 0.0, 0.6));
        assert_eq!(dark.background_solid, oklch(0.1452, 0.0021, 286.131, 1.0));
        assert_eq!(dark.card, oklch(0.2264, 0.0, 0.0, 1.0));
        assert_hsla_close(
            dark.popover,
            oklch_mix((0.2103, 0.0059, 285.885, 1.0), (0.9461, 0.0, 0.0, 1.0), 0.9),
        );
        assert_eq!(dark.secondary, oklch(0.2562, 0.0, 0.0, 1.0));
        assert_eq!(dark.muted, oklch(0.2768, 0.0, 0.0, 1.0));
        assert_eq!(dark.muted_foreground, oklch(0.7058, 0.0, 0.0, 1.0));
        assert_eq!(dark.subtle_foreground, oklch(0.6662, 0.0, 0.0, 1.0));
        assert_eq!(dark.surface_border, dark.border);
        assert_eq!(dark.popover_border, dark.foreground.opacity(0.12));
        assert_eq!(dark.sidebar_divider, dark.border);
    }

    #[test]
    fn elevated_surfaces_derive_interaction_states_from_their_own_base() {
        let light = G2VTheme::light();
        let dark = G2VTheme::dark();

        assert_eq!(light.popover, light.surface);
        assert_eq!(
            light.popover_hover,
            oklch_mix((1.0, 0.0, 0.0, 1.0), (0.2046, 0.0, 0.0, 1.0), 0.95)
        );
        assert_eq!(
            dark.surface_hover,
            oklch_mix(
                (0.2103, 0.0059, 285.885, 1.0),
                (0.9461, 0.0, 0.0, 1.0),
                0.95
            )
        );
        assert_hsla_close(
            dark.popover_selected,
            oklch_mix(
                (0.28388, 0.00531, 285.885, 1.0),
                (0.9461, 0.0, 0.0, 1.0),
                0.9,
            ),
        );
    }

    #[test]
    fn oklch_mix_preserves_the_chromatic_hue_when_mixed_with_a_neutral() {
        let actual: Rgba = oklch_mix(
            (0.2103, 0.0059, 285.885, 1.0),
            (0.9461, 0.0, 0.0, 1.0),
            0.85,
        )
        .into();
        let expected: Rgba = oklch(0.320_67, 0.005_015, 285.885, 1.0).into();
        assert!((actual.r - expected.r).abs() < 0.0001);
        assert!((actual.g - expected.g).abs() < 0.0001);
        assert!((actual.b - expected.b).abs() < 0.0001);
        assert!((actual.a - expected.a).abs() < 0.0001);
    }
}
