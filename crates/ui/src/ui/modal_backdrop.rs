//! Shared low-level backdrop painting for native modal primitives.

use gpui::{Corners, InteractiveElement as _, ParentElement as _, Styled as _, canvas, div, px};

pub(super) const MODAL_BACKDROP_BLUR_RADIUS: gpui::Pixels = px(4.0);
pub(super) const MODAL_BACKDROP_TINT_OPACITY: f32 = 0.05;
pub(super) const DIALOG_BACKDROP_TINT_OPACITY: f32 = 0.08;

pub(super) fn standard_modal_backdrop(viewport: gpui::Size<gpui::Pixels>) -> gpui::Div {
    modal_backdrop(
        viewport,
        MODAL_BACKDROP_BLUR_RADIUS,
        MODAL_BACKDROP_TINT_OPACITY,
    )
}

pub(super) fn dialog_modal_backdrop(viewport: gpui::Size<gpui::Pixels>) -> gpui::Div {
    modal_backdrop(
        viewport,
        MODAL_BACKDROP_BLUR_RADIUS,
        DIALOG_BACKDROP_TINT_OPACITY,
    )
}

pub(super) fn modal_backdrop(
    viewport: gpui::Size<gpui::Pixels>,
    blur_radius: gpui::Pixels,
    tint_opacity: f32,
) -> gpui::Div {
    div()
        .w(viewport.width)
        .h(viewport.height)
        .occlude()
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, (), window, _| {
                    window.paint_backdrop_blur(bounds, Corners::default(), blur_radius);
                },
            )
            .absolute()
            .inset_0(),
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .bg(gpui::hsla(0.0, 0.0, 0.0, tint_opacity)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_backdrop_matches_the_shared_modal_treatment() {
        assert_eq!(MODAL_BACKDROP_BLUR_RADIUS, px(4.0));
        assert!((MODAL_BACKDROP_TINT_OPACITY - 0.05).abs() < f32::EPSILON);
        assert!((DIALOG_BACKDROP_TINT_OPACITY - 0.08).abs() < f32::EPSILON);
    }
}
