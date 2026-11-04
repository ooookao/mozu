//! Theme-aware loading spinner primitive.

use std::{f32::consts::TAU, time::Duration};

use gpui::{
    Animation, AnimationExt as _, App, Hsla, IntoElement, Pixels, RenderOnce, Transformation,
    Window, prelude::*, radians, svg,
};

use crate::theme::colors::G2VTheme;

use super::IconName;

const SPIN_DURATION: Duration = Duration::from_secs(1);

#[derive(IntoElement)]
#[must_use]
pub struct Spinner {
    size: Pixels,
    color: Option<Hsla>,
}

impl Spinner {
    pub fn new(size: Pixels) -> Self {
        Self { size, color: None }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Spinner {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self
            .color
            .unwrap_or_else(|| G2VTheme::global(cx).muted_foreground);
        let spinner = svg()
            .path(IconName::Loading.path())
            .size(self.size)
            .text_color(color)
            .flex_none()
            .with_transformation(Transformation::rotate(radians(0.0)));

        if cx.reduce_motion() {
            spinner.into_any_element()
        } else {
            spinner
                .with_animation(
                    "spinner-rotate",
                    Animation::new(SPIN_DURATION).repeat(),
                    |spinner, delta| {
                        spinner.with_transformation(Transformation::rotate(radians(TAU * delta)))
                    },
                )
                .into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_uses_a_one_second_rotation_cycle() {
        assert_eq!(SPIN_DURATION, Duration::from_secs(1));
    }
}
