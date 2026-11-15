use crate::ui::utils::presentation::{ComponentTheme, ThemeScheme};
pub use crate::ui::utils::scroll_fade::paint_scroll_fade;
use crate::ui::utils::scroll_fade::scroll_fade as viewport_fade;
use gpui::{
    Animation, App, Axis, ElementId, Hsla, StyleRefinement, StyleTransition, Styled, Window, px,
};
use kinu_ui::{
    ColorScheme, ScrollArea as NativeScrollArea, ScrollAreaCorner, ScrollAreaScrollbar,
    ScrollAreaScrollbarState, ScrollAreaThumb, ScrollAreaViewport,
};
pub use kinu_ui::{ScrollAreaAxes, ScrollAreaContent, ScrollAreaState};
use std::time::Duration;

fn visible(state: &ScrollAreaScrollbarState) -> bool {
    let overflow = match state.orientation {
        Axis::Vertical => state.area.has_overflow_y,
        Axis::Horizontal => state.area.has_overflow_x,
    };
    overflow
        && (state.area.hovering
            || state.scrolling()
            || state.area.dragging == Some(state.orientation))
}

fn visibility_animation(
    shown: bool,
    enter: Duration,
    hold: Duration,
    exit: Duration,
    ease_out: impl Fn(f32) -> f32 + 'static,
) -> Animation {
    let delay = hold.as_millis() as f32;
    let fade = exit.as_millis() as f32;
    let total = (hold + exit).as_millis() as f32;
    if shown {
        Animation::new(enter).with_easing(ease_out)
    } else {
        Animation::new(hold + exit).with_easing(move |progress| {
            ease_out(((progress * total - delay) / fade).clamp(0., 1.))
        })
    }
}

fn thumb_style(state: &ScrollAreaScrollbarState, overlay: Hsla, radius: f32) -> StyleRefinement {
    let dragging = state.area.dragging == Some(state.orientation);
    let opacity = if dragging {
        0.16
    } else if state.hovered {
        0.12
    } else {
        0.08
    };
    let thickness = if state.hovered { 6. } else { 4. };
    let style = StyleRefinement::default()
        .relative()
        .bg(overlay.opacity(opacity))
        .rounded(px(radius));
    match state.orientation {
        Axis::Vertical => style.w(px(thickness)).translate_x(px(-2.)),
        Axis::Horizontal => style.h(px(thickness)).translate_y(px(-2.)),
    }
}

/// Shares scrollbar appearance with an existing native scrolling viewport.
pub fn scrollbar(
    orientation: Axis,
    tokens: ComponentTheme,
    scheme: ThemeScheme,
) -> ScrollAreaScrollbar {
    let overlay = gpui::Hsla::from((tokens.extras(scheme)).overlay);
    let radius = tokens.shape.radii(&tokens).item;
    let timing = tokens.settings.timings;
    let animations = tokens.motion.animations;
    let thumb = ScrollAreaThumb::new()
        .state_style(move |state| thumb_style(state, overlay, radius))
        .transition(StyleTransition::new(
            Animation::new(Duration::from_millis(
                animations.scrollbar_enter.duration_ms,
            ))
            .with_easing(tokens.motion.curves.in_out.curve()),
        ));
    let bar = ScrollAreaScrollbar::new(orientation, thumb)
        .keep_mounted(true)
        .state_style(|state| {
            StyleRefinement::default().opacity(if visible(state) { 1. } else { 0. })
        })
        .state_transition(move |state| {
            let shown = visible(state);
            let easing = if shown {
                animations.scrollbar_enter.easing
            } else {
                animations.scrollbar_exit.easing
            };
            StyleTransition::opacity(visibility_animation(
                shown,
                Duration::from_millis(animations.scrollbar_enter.duration_ms),
                Duration::from_millis(timing.scroll_fade_hold_ms),
                Duration::from_millis(animations.scrollbar_exit.duration_ms),
                easing.curve(),
            ))
        })
        .interactive_when(visible);
    match orientation {
        Axis::Vertical => bar.w(px(10.)).py(px(4.)),
        Axis::Horizontal => bar.h(px(10.)).px(px(4.)),
    }
}

/// Styles a native scroll area without duplicating scroll state, input, or geometry.
/// Configure content, focus, labels, and masks on the viewport before passing it in.
pub fn scroll_area_with_tokens(
    id: impl Into<ElementId>,
    mut viewport: ScrollAreaViewport,
    axes: ScrollAreaAxes,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> NativeScrollArea {
    viewport = viewport.axes(axes).size_full();
    let scheme = match ColorScheme::current(window, cx) {
        ColorScheme::Light => ThemeScheme::Light,
        ColorScheme::Dark => ThemeScheme::Dark,
    };
    let mut area = NativeScrollArea::new(id, viewport)
        .relative()
        .overflow_hidden();
    for orientation in [Axis::Vertical, Axis::Horizontal] {
        let enabled = match axes {
            ScrollAreaAxes::Vertical => orientation == Axis::Vertical,
            ScrollAreaAxes::Horizontal => orientation == Axis::Horizontal,
            ScrollAreaAxes::Both => true,
            ScrollAreaAxes::None => false,
        };
        if !enabled {
            continue;
        }
        area = area.scrollbar(scrollbar(orientation, tokens.clone(), scheme));
    }
    if axes == ScrollAreaAxes::Both {
        area = area.corner(ScrollAreaCorner::new());
    }
    area
}

/// Fades the 48px edge bands according to actual scroll distance on one axis.
pub fn scroll_fade(viewport: ScrollAreaViewport, axis: Axis) -> ScrollAreaViewport {
    viewport.scroll_fade(viewport_fade(axis))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn scroll_area(
    id: impl Into<ElementId>,
    viewport: ScrollAreaViewport,
    axes: ScrollAreaAxes,
    window: &Window,
    cx: &App,
) -> NativeScrollArea {
    scroll_area_with_tokens(id, viewport, axes, ComponentTheme::current(cx), window, cx)
}
