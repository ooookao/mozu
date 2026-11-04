//! Shared elevation shadows for application-owned native components.

use gpui::{App, BoxShadow, Hsla, WindowAppearance, hsla, point, px};
use gpui_base::ShadowTokens;

pub const RING_2: gpui::Pixels = px(2.0);
pub const RING_3: gpui::Pixels = px(3.0);
pub const OUTLINE_DEFAULT: gpui::Pixels = super::border::BORDER_DEFAULT;

#[must_use]
pub fn tokens(appearance: WindowAppearance) -> ShadowTokens {
    let color = match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => hsla(0.0, 0.0, 0.0, 0.03),
        WindowAppearance::Dark | WindowAppearance::VibrantDark => hsla(0.0, 0.0, 0.0, 0.05),
    };

    ShadowTokens {
        sm: shadow_pair((2.0, 7.0, -2.0), (0.0, 4.0, -1.0), color),
        md: shadow_pair((3.0, 12.0, -3.0), (0.0, 7.0, -2.0), color),
        lg: shadow_pair((4.0, 18.0, -4.0), (0.0, 10.0, -3.0), color),
    }
}

pub fn sync_base(appearance: WindowAppearance, cx: &mut App) {
    gpui_base::Theme::global_mut(cx).tokens.shadow = tokens(appearance);
}

#[must_use]
pub fn focus_ring(color: Hsla, width: gpui::Pixels) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color,
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: width,
        inset: false,
    }]
}

#[cfg(test)]
#[must_use]
pub fn inset_outline(color: Hsla, width: gpui::Pixels) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color,
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: width,
        inset: true,
    }]
}

fn shadow_pair(first: (f32, f32, f32), second: (f32, f32, f32), color: Hsla) -> Vec<BoxShadow> {
    vec![
        box_shadow(0.0, first.0, first.1, first.2, color),
        box_shadow(0.0, second.0, second.1, second.2, color),
    ]
}

fn box_shadow(x: f32, y: f32, blur: f32, spread: f32, color: Hsla) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(x), px(y)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_shadows_are_more_opaque_than_light_shadows() {
        let light = tokens(WindowAppearance::Light);
        let dark = tokens(WindowAppearance::Dark);

        assert!(dark.sm[0].color.a > light.sm[0].color.a);
        assert!((light.sm[0].color.a - 0.03).abs() < f32::EPSILON);
        assert!((dark.sm[0].color.a - 0.05).abs() < f32::EPSILON);
    }

    #[test]
    fn small_shadow_uses_a_soft_two_layer_profile() {
        let shadow = tokens(WindowAppearance::Dark).sm;

        assert_eq!(shadow[0].offset.y, px(2.0));
        assert_eq!(shadow[0].blur_radius, px(7.0));
        assert_eq!(shadow[0].spread_radius, px(-2.0));
        assert_eq!(shadow[1].offset.y, px(0.0));
        assert_eq!(shadow[1].blur_radius, px(4.0));
        assert_eq!(shadow[1].spread_radius, px(-1.0));
    }

    #[test]
    fn focus_and_outline_shadows_preserve_their_paint_direction() {
        let ring = focus_ring(hsla(0.0, 0.0, 0.0, 1.0), RING_3);
        let outline = inset_outline(hsla(0.0, 0.0, 0.0, 1.0), OUTLINE_DEFAULT);

        assert!(!ring[0].inset);
        assert!(outline[0].inset);
    }
}
