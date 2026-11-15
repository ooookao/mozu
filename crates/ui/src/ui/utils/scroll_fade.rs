use gpui::{Axis, Bounds, Pixels, Window, px};
use kinu_ui::ScrollAreaState;
pub use kinu_ui::ScrollFade;

/// The common 48px content fade for either axis of a native viewport.
///
/// Apply with `ScrollAreaViewport::scroll_fade` or
/// `ScrollViewportPresentation::scroll_fade`. Keep the surrounding surface,
/// border, shadow, and scrollbar outside that viewport's alpha mask.
/// Use `ScrollFade::new` for a caller-owned length, or
/// `ScrollFade::overflow_extent` to grow transparent edge bands with overflow.
pub fn scroll_fade(axis: Axis) -> ScrollFade {
    ScrollFade::new(axis, px(48.))
}

/// Paints alpha inside an existing viewport mask using that viewport's state.
/// This does not create a scroll owner or paint a foreground color overlay.
pub fn paint_scroll_fade(
    bounds: Bounds<Pixels>,
    state: &ScrollAreaState,
    axis: Axis,
    length: Pixels,
    window: &mut Window,
) {
    ScrollFade::new(axis, length).paint(bounds, state, window);
}
