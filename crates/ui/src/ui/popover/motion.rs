//! Popover surface motion independent from positioning and interaction state.

use std::time::Duration;

use gpui::{BoxShadow, Div, Stateful, Styled as _, px};

use crate::theme::motion::{
    FLOATING_SURFACE_TRANSITION_DURATION, FLOATING_SURFACE_TRANSITION_OFFSET,
    elevated_surface_opacity, floating_surface_visibility,
};

use super::{PopoverSide, primitive::PopoverPhase};

pub(super) const MOTION_DURATION: Duration = FLOATING_SURFACE_TRANSITION_DURATION;

pub(super) fn animate_surface(
    surface: Stateful<Div>,
    phase: PopoverPhase,
    side: PopoverSide,
    shadow: &[BoxShadow],
    delta: f32,
) -> Stateful<Div> {
    let visible = floating_surface_visibility(phase == PopoverPhase::Closing, delta);
    let (x, y) = side.motion();
    surface
        .left(px(x
            * f32::from(FLOATING_SURFACE_TRANSITION_OFFSET)
            * (1.0 - visible)))
        .top(px(y
            * f32::from(FLOATING_SURFACE_TRANSITION_OFFSET)
            * (1.0 - visible)))
        .opacity(popover_surface_opacity(visible))
        .shadow(faded_shadow(shadow, visible))
}

fn popover_surface_opacity(progress: f32) -> f32 {
    elevated_surface_opacity(progress)
}

fn faded_shadow(shadow: &[BoxShadow], opacity: f32) -> Vec<BoxShadow> {
    shadow
        .iter()
        .cloned()
        .map(|mut shadow| {
            shadow.color.a *= opacity;
            shadow
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_fades_with_the_surface() {
        let shadows = crate::theme::shadows::tokens(gpui::WindowAppearance::Light).md;
        let faded = faded_shadow(&shadows, 0.25);

        assert_eq!(faded.len(), shadows.len());
        assert!((faded[0].color.a - shadows[0].color.a * 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn occluding_popover_surface_stays_opaque_during_motion() {
        assert_eq!(popover_surface_opacity(0.0), 1.0);
        assert_eq!(popover_surface_opacity(0.5), 1.0);
        assert_eq!(popover_surface_opacity(1.0), 1.0);
    }
}
