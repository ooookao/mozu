//! G2V's canonical native loading placeholder primitive.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, App, IntoElement, RenderOnce, StyleRefinement, Styled, Window,
    bounce, div,
};
use gpui_base::StyledExt as _;

use crate::theme::colors::G2VTheme;
use crate::theme::motion::cubic_bezier_with_overshoot;
use crate::theme::radius::RADIUS_MD;

const PULSE_DURATION: Duration = Duration::from_secs(2);

#[derive(IntoElement, Default)]
#[must_use]
pub struct Skeleton {
    style: StyleRefinement,
}

impl Skeleton {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let background = G2VTheme::global(cx).muted;

        div()
            .rounded(RADIUS_MD)
            .bg(background)
            .refine_style(&self.style)
            .with_animation(
                "skeleton-pulse",
                Animation::new(PULSE_DURATION)
                    .repeat()
                    .with_easing(bounce(cubic_bezier_with_overshoot(0.4, 0.0, 0.6, 1.0))),
                |element, delta| element.opacity(pulse_opacity(delta)),
            )
    }
}

fn pulse_opacity(delta: f32) -> f32 {
    1.0 - delta * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_matches_tailwind_opacity_range() {
        assert!((pulse_opacity(0.0) - 1.0).abs() < f32::EPSILON);
        assert!((pulse_opacity(1.0) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn pulse_matches_tailwind_duration() {
        assert_eq!(PULSE_DURATION, Duration::from_secs(2));
    }

    #[test]
    fn pulse_uses_the_tailwind_timing_curve() {
        let easing = bounce(cubic_bezier_with_overshoot(0.4, 0.0, 0.6, 1.0));
        assert!((easing(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((easing(0.5) - 1.0).abs() < f32::EPSILON);
        assert!((easing(1.0) - 0.0).abs() < f32::EPSILON);
    }
}
