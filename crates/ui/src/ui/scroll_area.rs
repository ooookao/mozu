//! Native scroll area matching G2V's Base UI scrollbar contract.

use std::time::Duration;

use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement, Rems,
    RenderOnce, ScrollHandle, StatefulInteractiveElement as _, StyleRefinement, Styled, Window,
    div, prelude::FluentBuilder as _,
};
use gpui_base::{
    InteractiveElementExt as _, Scrollbar, ScrollbarAxis, ScrollbarEntrance, ScrollbarHandle,
    ScrollbarMode, ScrollbarMotion, ScrollbarTheme, StyledExt as _,
};

use crate::theme::colors::G2VTheme;
use crate::theme::radius::RADIUS_FULL;
use crate::theme::rems_from_design_pixels;

const TRACK_WIDTH: Rems = rems_from_design_pixels(10.0);
const THUMB_WIDTH: Rems = rems_from_design_pixels(4.0);
const THUMB_HOVER_WIDTH: Rems = rems_from_design_pixels(6.0);
const THUMB_INSET: Rems = rems_from_design_pixels(2.0);
const THUMB_OPACITY: f32 = 0.08;
const THUMB_HOVER_OPACITY: f32 = 0.12;
const THUMB_ACTIVE_OPACITY: f32 = 0.16;
const ENTER_DURATION: Duration = Duration::from_millis(160);
const EXIT_DURATION: Duration = Duration::from_millis(120);
const EXPAND_DURATION: Duration = Duration::from_millis(160);
const EXIT_DELAY: Duration = Duration::from_millis(160);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollAreaOrientation {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

impl From<ScrollAreaOrientation> for ScrollbarAxis {
    fn from(orientation: ScrollAreaOrientation) -> Self {
        match orientation {
            ScrollAreaOrientation::Vertical => Self::Vertical,
            ScrollAreaOrientation::Horizontal => Self::Horizontal,
            ScrollAreaOrientation::Both => Self::Both,
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct ScrollArea {
    id: ElementId,
    orientation: ScrollAreaOrientation,
    scroll_handle: Option<ScrollHandle>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl ScrollArea {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            orientation: ScrollAreaOrientation::Vertical,
            scroll_handle: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn orientation(mut self, orientation: ScrollAreaOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn scroll_handle(mut self, scroll_handle: ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle);
        self
    }
}

impl Styled for ScrollArea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = self.scroll_handle.unwrap_or_else(|| {
            window
                .use_keyed_state((self.id.clone(), "scroll-handle"), cx, |_, _| {
                    ScrollHandle::new()
                })
                .read(cx)
                .clone()
        });
        let axis = ScrollbarAxis::from(self.orientation);
        let scrollbar = themed_scrollbar(
            (self.id.clone(), "scrollbar"),
            &scroll_handle,
            axis,
            window,
            cx,
        );
        let viewport = div()
            .id((self.id.clone(), "viewport"))
            .size_full()
            .track_scroll(&scroll_handle)
            .lock_scroll_axis()
            .map(|viewport| match self.orientation {
                ScrollAreaOrientation::Vertical => viewport.overflow_y_scroll(),
                ScrollAreaOrientation::Horizontal => viewport.overflow_x_scroll(),
                ScrollAreaOrientation::Both => viewport.overflow_scroll(),
            })
            .child(
                div()
                    .id((self.id.clone(), "content"))
                    .map(|content| match self.orientation {
                        ScrollAreaOrientation::Vertical => content.min_h_full(),
                        ScrollAreaOrientation::Horizontal => content.min_w_full().w_auto(),
                        ScrollAreaOrientation::Both => content.min_size_full().size_auto(),
                    })
                    .children(self.children),
            );

        let viewport_scroll_handle = scroll_handle.clone();
        div()
            .id(self.id)
            .relative()
            .size_full()
            .overflow_hidden()
            .on_scroll_wheel(move |_, _, cx| {
                let maximum = viewport_scroll_handle.max_offset();
                let contains_scroll = match axis {
                    ScrollbarAxis::Vertical => maximum.y > gpui::Pixels::ZERO,
                    ScrollbarAxis::Horizontal => maximum.x > gpui::Pixels::ZERO,
                    ScrollbarAxis::Both => {
                        maximum.x > gpui::Pixels::ZERO || maximum.y > gpui::Pixels::ZERO
                    }
                };
                if contains_scroll {
                    cx.stop_propagation();
                }
            })
            .refine_style(&self.style)
            .child(viewport)
            .child(scrollbar)
    }
}

pub(crate) fn themed_scrollbar<H: ScrollbarHandle + Clone>(
    id: impl Into<ElementId>,
    scroll_handle: &H,
    axis: ScrollbarAxis,
    window: &Window,
    cx: &App,
) -> Scrollbar {
    let foreground = G2VTheme::global(cx).foreground;
    let rem_size = window.rem_size();
    let track_width = TRACK_WIDTH.to_pixels(rem_size);
    let thumb_width = THUMB_WIDTH.to_pixels(rem_size);
    let thumb_hover_width = THUMB_HOVER_WIDTH.to_pixels(rem_size);
    let thumb_inset = THUMB_INSET.to_pixels(rem_size);
    Scrollbar::new(scroll_handle)
        .id(id)
        .axis(axis)
        .mode(ScrollbarMode::Scrolling)
        .styles(|styles| {
            styles
                .track(|style| style.width(track_width))
                .track_hover(|style| style.width(track_width))
                .track_active(|style| style.width(track_width))
                .thumb(|style| {
                    style
                        .bg(foreground.opacity(THUMB_OPACITY))
                        .width(thumb_width)
                        .inset(thumb_inset)
                        .radius(RADIUS_FULL)
                })
                .thumb_hover(|style| {
                    style
                        .bg(foreground.opacity(THUMB_HOVER_OPACITY))
                        .width(thumb_hover_width)
                })
                .thumb_active(|style| {
                    style
                        .bg(foreground.opacity(THUMB_ACTIVE_OPACITY))
                        .width(thumb_hover_width)
                })
        })
}

#[must_use]
pub fn scrollbar_track_width(rem_size: gpui::Pixels) -> gpui::Pixels {
    TRACK_WIDTH.to_pixels(rem_size)
}

pub(crate) fn sync_base_theme(cx: &mut App) {
    let motion = ScrollbarMotion::default()
        .with_idle(EXIT_DELAY)
        .with_enter(ENTER_DURATION)
        .with_exit(EXIT_DURATION)
        .with_expand(EXPAND_DURATION)
        .with_entrance(ScrollbarEntrance::Fade)
        .with_thumb_hover_entrance(ScrollbarEntrance::Fade);
    gpui_base::Theme::global_mut(cx).scrollbar = ScrollbarTheme::new()
        .with_mode(ScrollbarMode::Scrolling)
        .with_motion(motion);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, Render, ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement,
        WindowAppearance, point, px,
    };

    #[test]
    fn motion_matches_the_g2v_scroll_area_contract() {
        assert_eq!(ENTER_DURATION, Duration::from_millis(160));
        assert_eq!(EXIT_DURATION, Duration::from_millis(120));
        assert_eq!(EXIT_DELAY, Duration::from_millis(160));
        assert_eq!(EXPAND_DURATION, Duration::from_millis(160));
    }

    #[test]
    fn scrollbar_geometry_matches_the_g2v_scroll_area_contract() {
        assert_eq!(TRACK_WIDTH.to_pixels(px(16.0)), px(10.0));
        assert_eq!(TRACK_WIDTH.to_pixels(px(18.0)), px(11.25));
        assert_eq!(THUMB_WIDTH.to_pixels(px(16.0)), px(4.0));
        assert_eq!(THUMB_WIDTH.to_pixels(px(18.0)), px(4.5));
        assert_eq!(THUMB_HOVER_WIDTH.to_pixels(px(16.0)), px(6.0));
        assert_eq!(THUMB_HOVER_WIDTH.to_pixels(px(18.0)), px(6.75));
        assert_eq!(THUMB_INSET.to_pixels(px(16.0)), px(2.0));
        assert_eq!(THUMB_INSET.to_pixels(px(18.0)), px(2.25));
    }

    struct NestedScrollHarness {
        page_scroll: ScrollHandle,
        content_height: gpui::Pixels,
    }

    impl Render for NestedScrollHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .id("page-scroll-test")
                .size_full()
                .overflow_y_scroll()
                .track_scroll(&self.page_scroll)
                .child(
                    div().h(px(1600.0)).child(
                        div().w(px(160.0)).h(px(80.0)).child(
                            ScrollArea::new("nested-scroll-test")
                                .child(div().h(self.content_height).child("Nested content")),
                        ),
                    ),
                )
        }
    }

    #[gpui::test]
    fn nested_scroll_area_contains_wheel_events(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let page_scroll = ScrollHandle::new();
        let (_, window) = cx.add_window_view({
            let page_scroll = page_scroll.clone();
            move |_, _| NestedScrollHarness {
                page_scroll,
                content_height: px(320.0),
            }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));

        window.simulate_event(ScrollWheelEvent {
            position: point(px(40.0), px(40.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-30.0))),
            ..Default::default()
        });

        assert_eq!(page_scroll.offset(), point(px(0.0), px(0.0)));
    }

    #[gpui::test]
    fn non_overflowing_output_does_not_block_parent_scroll(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let page_scroll = ScrollHandle::new();
        let (_, window) = cx.add_window_view({
            let page_scroll = page_scroll.clone();
            move |_, _| NestedScrollHarness {
                page_scroll,
                content_height: px(40.0),
            }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.simulate_event(ScrollWheelEvent {
            position: point(px(40.0), px(40.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-30.0))),
            ..Default::default()
        });
        assert_eq!(page_scroll.offset().y, px(-30.0));
    }
}
