//! Conversation scrolling behavior matching G2V's React message scroller composition.

mod model;
mod state;
mod viewport;

use std::rc::Rc;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(debug_assertions)]
use std::time::Instant;

use gpui::{
    AnyElement, App, ElementId, EntityId, InteractiveElement as _, IntoElement, ParentElement,
    Pixels, RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_base::ScrollbarAxis;
use gpui_base::{ElementExt as _, StyledExt as _};

use crate::theme::spacing::SPACE_4;
use crate::ui::scroll_area::{scrollbar_track_width, themed_scrollbar};
use crate::ui::{Button, ButtonSize, ButtonVariant, IconName};

type MessageItemRenderer = dyn Fn(&mut Window, &mut App) -> AnyElement;

#[cfg(debug_assertions)]
static SCROLL_EVENTS: AtomicU64 = AtomicU64::new(0);
#[cfg(debug_assertions)]
static SCROLLER_RENDERS: AtomicU64 = AtomicU64::new(0);
#[cfg(debug_assertions)]
static ROW_RENDERS: AtomicU64 = AtomicU64::new(0);
#[cfg(debug_assertions)]
static ROW_BUILD_NANOS: AtomicU64 = AtomicU64::new(0);
#[cfg(debug_assertions)]
static PREPAINT_PASSES: AtomicU64 = AtomicU64::new(0);
#[cfg(debug_assertions)]
static LAYOUT_REMEASUREMENTS: AtomicU64 = AtomicU64::new(0);

#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MessageScrollerTraceSnapshot {
    pub scroll_events: u64,
    pub scroller_renders: u64,
    pub row_renders: u64,
    pub row_build_nanos: u64,
    pub prepaint_passes: u64,
    pub layout_remeasurements: u64,
}

#[cfg(debug_assertions)]
impl MessageScrollerTraceSnapshot {
    #[must_use]
    pub fn saturating_difference(self, start: Self) -> Self {
        Self {
            scroll_events: self.scroll_events.saturating_sub(start.scroll_events),
            scroller_renders: self.scroller_renders.saturating_sub(start.scroller_renders),
            row_renders: self.row_renders.saturating_sub(start.row_renders),
            row_build_nanos: self.row_build_nanos.saturating_sub(start.row_build_nanos),
            prepaint_passes: self.prepaint_passes.saturating_sub(start.prepaint_passes),
            layout_remeasurements: self
                .layout_remeasurements
                .saturating_sub(start.layout_remeasurements),
        }
    }
}

#[cfg(debug_assertions)]
#[must_use]
pub fn message_scroller_trace_snapshot() -> MessageScrollerTraceSnapshot {
    MessageScrollerTraceSnapshot {
        scroll_events: SCROLL_EVENTS.load(Ordering::Relaxed),
        scroller_renders: SCROLLER_RENDERS.load(Ordering::Relaxed),
        row_renders: ROW_RENDERS.load(Ordering::Relaxed),
        row_build_nanos: ROW_BUILD_NANOS.load(Ordering::Relaxed),
        prepaint_passes: PREPAINT_PASSES.load(Ordering::Relaxed),
        layout_remeasurements: LAYOUT_REMEASUREMENTS.load(Ordering::Relaxed),
    }
}

const fn item_bottom_padding(
    is_last: bool,
    bottom_content_inset: Pixels,
    item_spacing: Pixels,
) -> Pixels {
    if is_last {
        bottom_content_inset
    } else {
        item_spacing
    }
}

const fn item_top_padding(is_first: bool, top_content_inset: Pixels) -> Pixels {
    if is_first {
        top_content_inset
    } else {
        Pixels::ZERO
    }
}

pub use model::MessageScrollerModel;
pub use state::{
    MessageScrollerAlign, MessageScrollerConfig, MessageScrollerDefaultPosition,
    MessageScrollerDirection, MessageScrollerSnapshot, MessageScrollerState,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageScrollerScrollbarPlacement {
    #[default]
    Internal,
    External,
}

#[derive(IntoElement)]
#[must_use]
pub struct MessageScrollerScrollbar {
    id: ElementId,
    model: gpui::Entity<MessageScrollerModel>,
}

impl MessageScrollerScrollbar {
    pub fn new(id: impl Into<ElementId>, model: gpui::Entity<MessageScrollerModel>) -> Self {
        Self {
            id: id.into(),
            model,
        }
    }
}

impl RenderOnce for MessageScrollerScrollbar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let list_state = self.model.read(cx).list();
        themed_scrollbar(self.id, &list_state, ScrollbarAxis::Vertical, window, cx)
            .viewport_from_layout()
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct MessageScroller {
    id: ElementId,
    model: gpui::Entity<MessageScrollerModel>,
    invalidation_target: Option<EntityId>,
    buttons: Vec<MessageScrollerDirection>,
    scrollbar_placement: MessageScrollerScrollbarPlacement,
    style: StyleRefinement,
}

impl MessageScroller {
    pub fn new(id: impl Into<ElementId>, model: gpui::Entity<MessageScrollerModel>) -> Self {
        Self {
            id: id.into(),
            model,
            invalidation_target: None,
            buttons: Vec::new(),
            scrollbar_placement: MessageScrollerScrollbarPlacement::Internal,
            style: StyleRefinement::default(),
        }
    }

    pub fn invalidation_target(mut self, target: EntityId) -> Self {
        self.invalidation_target = Some(target);
        self
    }

    pub fn button(mut self, direction: MessageScrollerDirection) -> Self {
        self.buttons.push(direction);
        self
    }

    pub fn scrollbar_placement(mut self, placement: MessageScrollerScrollbarPlacement) -> Self {
        self.scrollbar_placement = placement;
        self
    }
}

impl Styled for MessageScroller {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageScroller {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        #[cfg(debug_assertions)]
        SCROLLER_RENDERS.fetch_add(1, Ordering::Relaxed);
        let list_state = self.model.read(cx).list();
        let snapshot = self.model.read(cx).snapshot().clone();
        let invalidation_target = self.invalidation_target;
        let weak_model = self.model.downgrade();
        list_state.set_scroll_handler(move |event, _, cx| {
            #[cfg(debug_assertions)]
            SCROLL_EVENTS.fetch_add(1, Ordering::Relaxed);
            let visible_range = event.visible_range.clone();
            let weak_model = weak_model.clone();
            cx.defer(move |cx| {
                let snapshot_changed = weak_model
                    .update(cx, |model, _| model.user_scroll_intent(visible_range))
                    .unwrap_or(false);
                if snapshot_changed && let Some(target) = invalidation_target {
                    cx.notify(target);
                }
            });
        });

        let (_, bottom_content_inset) = self.model.read(cx).content_insets();
        let viewport = viewport::render(&self.model, cx);

        let scrollbar =
            MessageScrollerScrollbar::new((self.id.clone(), "scrollbar"), self.model.clone());
        let scrollbar_overlay = div()
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(scrollbar_track_width(window.rem_size()))
            .child(scrollbar);
        let scrollbar_is_internal =
            self.scrollbar_placement == MessageScrollerScrollbarPlacement::Internal;

        let root_id = self.id.clone();
        let measured_model = self.model.downgrade();
        let buttons = self.buttons.into_iter().filter_map(|direction| {
            let active = match direction {
                MessageScrollerDirection::Start => snapshot.start,
                MessageScrollerDirection::End => snapshot.end,
            };
            active.then(|| {
                render_scroll_button(
                    self.id.clone(),
                    direction,
                    self.model.clone(),
                    invalidation_target,
                    bottom_content_inset,
                )
            })
        });

        div()
            .id(root_id)
            .relative()
            .size_full()
            .min_h_0()
            .min_w_0()
            .overflow_hidden()
            .refine_style(&self.style)
            .on_prepaint(move |bounds, window, cx| {
                #[cfg(debug_assertions)]
                PREPAINT_PASSES.fetch_add(1, Ordering::Relaxed);
                let remeasured = measured_model
                    .update(cx, |model, cx| {
                        let remeasured = model.sync_layout(bounds.size.width, window.rem_size());
                        if remeasured {
                            cx.notify();
                        }
                        remeasured
                    })
                    .unwrap_or(false);
                #[cfg(debug_assertions)]
                if remeasured {
                    LAYOUT_REMEASUREMENTS.fetch_add(1, Ordering::Relaxed);
                }
                if remeasured && let Some(target) = invalidation_target {
                    cx.notify(target);
                }
            })
            .child(viewport)
            .when(scrollbar_is_internal, |root| root.child(scrollbar_overlay))
            .children(buttons)
    }
}

fn render_scroll_button(
    id: ElementId,
    direction: MessageScrollerDirection,
    model: gpui::Entity<MessageScrollerModel>,
    invalidation_target: Option<EntityId>,
    bottom_content_inset: Pixels,
) -> AnyElement {
    let (suffix, icon, label) = match direction {
        MessageScrollerDirection::Start => ("start", IconName::ChevronUp, "Scroll to start"),
        MessageScrollerDirection::End => ("end", IconName::ChevronDown, "Scroll to end"),
    };
    let button = Button::new((id, suffix))
        .variant(ButtonVariant::Secondary)
        .size(ButtonSize::IconSmall)
        .icon(icon)
        .accessibility_label(label)
        .tooltip(label)
        .on_click(move |_, _window, cx| {
            model.update(cx, |model, _| model.scroll(direction));
            if let Some(target) = invalidation_target {
                cx.notify(target);
            }
        });

    div()
        .absolute()
        .left_0()
        .right_0()
        .flex()
        .justify_center()
        .when(direction == MessageScrollerDirection::Start, |container| {
            container.top(SPACE_4)
        })
        .when(direction == MessageScrollerDirection::End, |container| {
            container.bottom(scroll_button_bottom_offset(bottom_content_inset))
        })
        .child(button)
        .into_any_element()
}

const fn scroll_button_bottom_offset(bottom_content_inset: Pixels) -> Pixels {
    bottom_content_inset
}

#[must_use]
#[derive(Clone)]
pub struct MessageScrollerItem {
    message_id: SharedString,
    scroll_anchor: bool,
    content_version: u64,
    bottom_spacing: Pixels,
    render_child: Rc<MessageItemRenderer>,
}

impl MessageScrollerItem {
    pub fn new(
        message_id: impl Into<SharedString>,
        render_child: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            message_id: message_id.into(),
            scroll_anchor: false,
            content_version: 0,
            bottom_spacing: Pixels::ZERO,
            render_child: Rc::new(render_child),
        }
    }

    pub fn scroll_anchor(mut self, scroll_anchor: bool) -> Self {
        self.scroll_anchor = scroll_anchor;
        self
    }

    pub fn content_version(mut self, content_version: u64) -> Self {
        self.content_version = content_version;
        self
    }

    pub fn bottom_spacing(mut self, bottom_spacing: Pixels) -> Self {
        self.bottom_spacing = bottom_spacing;
        self
    }

    fn registration(&self) -> (SharedString, bool, u64) {
        (
            self.message_id.clone(),
            self.scroll_anchor,
            self.content_version,
        )
    }

    fn render(
        &self,
        is_first: bool,
        is_last: bool,
        top_content_inset: Pixels,
        bottom_content_inset: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        div()
            .id(self.message_id.clone())
            .min_w_0()
            .flex_shrink_0()
            .pt(item_top_padding(is_first, top_content_inset))
            .pb(item_bottom_padding(
                is_last,
                bottom_content_inset,
                self.bottom_spacing,
            ))
            .child((self.render_child)(window, cx))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::HashSet, rc::Rc};

    use super::*;

    #[test]
    fn trace_snapshot_reports_only_the_measurement_interval() {
        let start = MessageScrollerTraceSnapshot {
            scroll_events: 4,
            scroller_renders: 7,
            row_renders: 28,
            row_build_nanos: 2_800,
            prepaint_passes: 7,
            layout_remeasurements: 1,
        };
        let finish = MessageScrollerTraceSnapshot {
            scroll_events: 10,
            scroller_renders: 9,
            row_renders: 40,
            row_build_nanos: 4_600,
            prepaint_passes: 10,
            layout_remeasurements: 1,
        };

        assert_eq!(
            finish.saturating_difference(start),
            MessageScrollerTraceSnapshot {
                scroll_events: 6,
                scroller_renders: 2,
                row_renders: 12,
                row_build_nanos: 1_800,
                prepaint_passes: 3,
                layout_remeasurements: 0,
            }
        );
    }
    use gpui::{
        AppContext as _, Context, ListOffset, PlatformInput, Render, ScrollDelta, ScrollWheelEvent,
        WindowAppearance, point, px,
    };

    #[test]
    fn generic_items_do_not_invent_inter_message_spacing() {
        assert_eq!(
            item_bottom_padding(false, px(96.0), Pixels::ZERO),
            Pixels::ZERO
        );
        assert_eq!(item_bottom_padding(true, px(96.0), Pixels::ZERO), px(96.0));
    }

    #[test]
    fn item_spacing_applies_only_before_another_transcript_row() {
        assert_eq!(item_bottom_padding(false, px(96.0), px(8.0)), px(8.0));
        assert_eq!(item_bottom_padding(true, px(96.0), px(8.0)), px(96.0));
    }

    #[test]
    fn only_the_first_message_reserves_the_transcript_top_spacing() {
        assert_eq!(item_top_padding(false, px(24.0)), Pixels::ZERO);
        assert_eq!(item_top_padding(true, px(24.0)), px(24.0));
    }

    #[test]
    fn scroll_to_bottom_clears_the_composer_and_its_content_spacing() {
        assert_eq!(scroll_button_bottom_offset(px(112.0)), px(112.0));
    }

    struct MessageScrollerWheelHarness {
        model: gpui::Entity<MessageScrollerModel>,
    }

    #[gpui::test]
    fn expanded_row_reentering_viewport_keeps_its_height_on_first_frame(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (view, window) = cx.add_window_view(|_, cx| {
            let model = cx.new(|_| {
                let mut model = MessageScrollerModel::new(MessageScrollerConfig {
                    auto_scroll: false,
                    default_position: MessageScrollerDefaultPosition::Start,
                    edge_threshold: px(8.0),
                    scroll_margin: Pixels::ZERO,
                    previous_item_peek: Pixels::ZERO,
                });
                model.replace_all(vec![
                    MessageScrollerItem::new("expanded-row", |_, _| {
                        crate::ui::CollapsibleContent::new("expanded-row-content", true)
                            .child(div().h(px(36.0)))
                            .into_any_element()
                    }),
                    MessageScrollerItem::new("long-row", |_, _| {
                        div().h(px(4000.0)).into_any_element()
                    }),
                ]);
                model
            });
            MessageScrollerWheelHarness { model }
        });
        for _ in 0..3 {
            window.update(|window, cx| window.draw(cx).clear(cx));
        }
        window.update(|_, cx| {
            let list = view.read(cx).model.read(cx).list();
            assert_eq!(list.bounds_for_item(0).unwrap().size.height, px(36.0));
            list.scroll_to(ListOffset {
                item_ix: 1,
                offset_in_item: px(2500.0),
            });
        });
        for _ in 0..3 {
            window.update(|window, cx| window.draw(cx).clear(cx));
        }
        window.update(|_, cx| {
            view.read(cx).model.read(cx).list().scroll_to(ListOffset {
                item_ix: 0,
                offset_in_item: px(20.0),
            });
        });
        window.update(|window, cx| {
            window.draw(cx).clear(cx);
            let list = view.read(cx).model.read(cx).list();
            assert_eq!(
                list.bounds_for_item(0).unwrap().size.height,
                px(36.0),
                "already-open content must not become zero-height when returning from virtualization"
            );
        });
    }

    impl Render for MessageScrollerWheelHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            MessageScroller::new("message-scroller-wheel-regression", self.model.clone())
                .invalidation_target(cx.entity_id())
        }
    }

    #[gpui::test]
    fn visible_row_changes_do_not_request_parent_invalidation_when_controls_are_stable(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (view, window) = cx.add_window_view(|_, cx| {
            let model = cx.new(|_| {
                let mut model = MessageScrollerModel::new(MessageScrollerConfig {
                    auto_scroll: false,
                    default_position: MessageScrollerDefaultPosition::Start,
                    edge_threshold: px(8.0),
                    scroll_margin: Pixels::ZERO,
                    previous_item_peek: Pixels::ZERO,
                });
                model.replace_all(
                    (0..50)
                        .map(|index| {
                            MessageScrollerItem::new(format!("message-{index}"), |_, _| {
                                div().h(px(100.0)).into_any_element()
                            })
                        })
                        .collect(),
                );
                model
            });
            MessageScrollerWheelHarness { model }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.update(|_, cx| {
            view.read(cx).model.read(cx).list().scroll_to(ListOffset {
                item_ix: 20,
                offset_in_item: Pixels::ZERO,
            });
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.update(|_, cx| {
            let model = view.read(cx).model.clone();
            model.update(cx, |model, _| {
                assert!(model.user_scroll_intent(0..50));
            });
        });
        let before = window.update(|_, cx| view.read(cx).model.read(cx).snapshot().clone());
        assert!(before.start && before.end);
        let first_visible_id = before
            .visible_message_ids
            .first()
            .expect("the middle viewport must contain visible messages")
            .to_string();
        let first_visible_index = first_visible_id
            .strip_prefix("message-")
            .expect("test message id prefix")
            .parse::<usize>()
            .expect("test message id index");

        let parent_invalidation_requested = window.update(|_, cx| {
            let model = view.read(cx).model.clone();
            model.update(cx, |model, _| {
                model.user_scroll_intent(first_visible_index + 1..50)
            })
        });
        let after = window.update(|_, cx| view.read(cx).model.read(cx).snapshot().clone());
        assert_eq!((after.start, after.end), (before.start, before.end));
        assert_ne!(after.visible_message_ids, before.visible_message_ids);
        assert!(
            !parent_invalidation_requested,
            "visible row changes must not request a parent render while scroll controls are unchanged"
        );
    }

    #[gpui::test]
    fn wheel_event_does_not_call_a_paint_only_window_api(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (_, window) = cx.add_window_view(|_, cx| {
            let model = cx.new(|_| {
                let config = MessageScrollerConfig {
                    auto_scroll: true,
                    default_position: MessageScrollerDefaultPosition::End,
                    edge_threshold: px(8.0),
                    scroll_margin: px(0.0),
                    previous_item_peek: px(64.0),
                };
                let mut model = MessageScrollerModel::new(config);
                model.replace_all(vec![
                    MessageScrollerItem::new("message-1", |_, _| {
                        div().h(px(200.0)).into_any_element()
                    }),
                    MessageScrollerItem::new("message-2", |_, _| {
                        div().h(px(200.0)).into_any_element()
                    }),
                    MessageScrollerItem::new("message-3", |_, _| {
                        div().h(px(200.0)).into_any_element()
                    }),
                ]);
                model
            });
            MessageScrollerWheelHarness { model }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));

        window.simulate_event(ScrollWheelEvent {
            position: point(px(40.0), px(40.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(-30.0))),
            ..Default::default()
        });
    }

    #[gpui::test]
    fn handled_wheel_scroll_changes_offset_while_remaining_propagatable(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (view, window) = cx.add_window_view(|_, cx| {
            let model = cx.new(|_| {
                let mut model = MessageScrollerModel::new(MessageScrollerConfig {
                    auto_scroll: true,
                    default_position: MessageScrollerDefaultPosition::End,
                    edge_threshold: px(8.0),
                    scroll_margin: px(0.0),
                    previous_item_peek: px(64.0),
                });
                model.replace_all(vec![
                    MessageScrollerItem::new("message-1", |_, _| {
                        div().h(px(400.0)).into_any_element()
                    }),
                    MessageScrollerItem::new("message-2", |_, _| {
                        div().h(px(400.0)).into_any_element()
                    }),
                    MessageScrollerItem::new("message-3", |_, _| {
                        div().h(px(400.0)).into_any_element()
                    }),
                ]);
                model
            });
            MessageScrollerWheelHarness { model }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let before =
            window.update(|_, cx| view.read(cx).model.read(cx).list().logical_scroll_top());

        let result = window.update(|window, cx| {
            window.dispatch_event(
                PlatformInput::ScrollWheel(ScrollWheelEvent {
                    position: point(px(40.0), px(40.0)),
                    delta: ScrollDelta::Pixels(point(px(0.0), px(72.0))),
                    ..Default::default()
                }),
                cx,
            )
        });
        let after = window.update(|_, cx| view.read(cx).model.read(cx).list().logical_scroll_top());
        window.run_until_parked();
        let snapshot = window.update(|_, cx| view.read(cx).model.read(cx).snapshot().clone());

        assert!(result.propagate);
        assert!(
            after.item_ix != before.item_ix || after.offset_in_item != before.offset_in_item,
            "wheel event must change the list's logical scroll offset"
        );
        assert!(
            snapshot.end,
            "wheel event toward the start must update the semantic snapshot"
        );
    }

    struct VariableHeightMessage {
        height: Pixels,
    }

    impl Render for VariableHeightMessage {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().h(self.height)
        }
    }

    struct MessageScrollerRemeasureHarness {
        model: gpui::Entity<MessageScrollerModel>,
        message: gpui::Entity<VariableHeightMessage>,
    }

    impl Render for MessageScrollerRemeasureHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            MessageScroller::new("message-scroller-remeasure-regression", self.model.clone())
        }
    }

    #[gpui::test]
    fn expansion_after_measurement_frame_preserves_header_position(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (view, window) = cx.add_window_view(|_, cx| {
            let message = cx.new(|_| VariableHeightMessage { height: px(0.0) });
            let child = message.clone();
            let model = cx.new(|_| {
                let mut model = MessageScrollerModel::new(MessageScrollerConfig {
                    auto_scroll: true,
                    default_position: MessageScrollerDefaultPosition::End,
                    edge_threshold: px(8.0),
                    scroll_margin: Pixels::ZERO,
                    previous_item_peek: Pixels::ZERO,
                });
                model.replace_all(vec![
                    MessageScrollerItem::new("history", |_, _| {
                        div().h(px(1500.0)).into_any_element()
                    }),
                    MessageScrollerItem::new("header", |_, _| div().h(px(40.0)).into_any_element()),
                    MessageScrollerItem::new("work", move |_, _| child.clone().into_any_element()),
                    MessageScrollerItem::new("answer", |_, _| {
                        div().h(px(160.0)).into_any_element()
                    }),
                ]);
                model
            });
            MessageScrollerRemeasureHarness { model, message }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let before = window.update(|_, cx| {
            let model = view.read(cx).model.clone();
            let y = model.read(cx).list().bounds_for_item(1).unwrap().origin.y;
            model.update(cx, |model, _| {
                model.prepare_for_user_layout_change();
                model.remeasure_items(&["work".into()]);
            });
            y
        });
        // The collapsible measures its natural height before growing on a later frame.
        window.update(|window, cx| window.draw(cx).clear(cx));
        for height in [px(60.0), px(180.0), px(300.0)] {
            window.update(|_, cx| {
                let message = view.read(cx).message.clone();
                message.update(cx, |message, cx| {
                    message.height = height;
                    cx.notify();
                });
            });
            window.update(|window, cx| window.draw(cx).clear(cx));
            let after = window.update(|_, cx| {
                view.read(cx)
                    .model
                    .read(cx)
                    .list()
                    .bounds_for_item(1)
                    .unwrap()
                    .origin
                    .y
            });
            assert_eq!(before, after, "header moved while work grew to {height:?}");
        }
        window.update(|_, cx| {
            let model = view.read(cx).model.clone();
            model.update(cx, |model, _| model.scroll(MessageScrollerDirection::End));
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.update(|_, cx| {
            let model = view.read(cx).model.read(cx);
            assert!(model.is_following_tail());
            assert_eq!(
                model.list().bounds_for_item(1).unwrap().origin.y,
                before - px(300.0)
            );
        });
    }

    #[gpui::test]
    fn nested_message_height_change_updates_the_list_row_measurement(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (view, window) = cx.add_window_view(|_, cx| {
            let message = cx.new(|_| VariableHeightMessage { height: px(80.0) });
            let rendered_message = message.clone();
            let model = cx.new(|_| {
                let mut model = MessageScrollerModel::new(MessageScrollerConfig {
                    auto_scroll: false,
                    default_position: MessageScrollerDefaultPosition::Start,
                    edge_threshold: px(8.0),
                    scroll_margin: Pixels::ZERO,
                    previous_item_peek: Pixels::ZERO,
                });
                model.replace_all(vec![MessageScrollerItem::new("message", move |_, _| {
                    rendered_message.clone().into_any_element()
                })]);
                model
            });
            MessageScrollerRemeasureHarness { model, message }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let initial_height = window.update(|_, cx| {
            view.read(cx)
                .model
                .read(cx)
                .list()
                .bounds_for_item(0)
                .expect("initial message bounds")
                .size
                .height
        });

        window.update(|_, cx| {
            let message = view.read(cx).message.clone();
            message.update(cx, |message, cx| {
                message.height = px(240.0);
                cx.notify();
            });
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let updated_height = window.update(|_, cx| {
            view.read(cx)
                .model
                .read(cx)
                .list()
                .bounds_for_item(0)
                .expect("updated message bounds")
                .size
                .height
        });

        assert_eq!(initial_height, px(80.0));
        assert_eq!(updated_height, px(240.0));
    }

    #[gpui::test]
    fn long_timeline_renders_only_visible_and_overdraw_rows(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let rendered_indices = Rc::new(RefCell::new(Vec::new()));
        let rendered_indices_for_view = rendered_indices.clone();
        let (_, window) = cx.add_window_view(move |_, cx| {
            let rendered_indices = rendered_indices_for_view.clone();
            let model = cx.new(|_| {
                let mut model = MessageScrollerModel::new(MessageScrollerConfig {
                    auto_scroll: false,
                    default_position: MessageScrollerDefaultPosition::Start,
                    edge_threshold: px(8.0),
                    scroll_margin: Pixels::ZERO,
                    previous_item_peek: Pixels::ZERO,
                });
                model.replace_all(
                    (0..1_000)
                        .map(|index| {
                            let rendered_indices = rendered_indices.clone();
                            MessageScrollerItem::new(format!("message-{index}"), move |_, _| {
                                rendered_indices.borrow_mut().push(index);
                                div().h(px(80.0)).into_any_element()
                            })
                        })
                        .collect(),
                );
                model
            });
            MessageScrollerWheelHarness { model }
        });

        window.update(|window, cx| window.draw(cx).clear(cx));

        let rendered = rendered_indices
            .borrow()
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        assert!(!rendered.is_empty());
        assert!(
            rendered.len() >= 32,
            "the timeline must preload enough adjacent rows to avoid mount churn during scrolling; rendered_rows={}",
            rendered.len()
        );
        assert!(
            rendered.len() < 1_000,
            "virtualized timeline must not render every message row"
        );
    }

    #[test]
    fn items_default_to_non_anchor_rows() {
        let item = MessageScrollerItem::new("message-1", |_, _| div().into_any_element());
        assert!(!item.scroll_anchor);
    }

    #[test]
    fn anchor_rows_are_explicit() {
        let item = MessageScrollerItem::new("message-1", |_, _| div().into_any_element())
            .scroll_anchor(true);
        assert!(item.scroll_anchor);
    }
}
