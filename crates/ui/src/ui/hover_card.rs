//! G2V's shadcn-compatible hover card built on gpui-base behavior.

use std::{rc::Rc, time::Duration};

use gpui::{
    AbsoluteLength, Animation, AnimationExt as _, AnyElement, App, Bounds, Context, ElementId,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, Rems, Render, RenderOnce,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Task, Window, div, ease_in_out,
    prelude::FluentBuilder as _, px,
};
use gpui_base::{
    Align, DeferredPopover, ElementExt as _, GlobalState, Placement, Positioner, StyledExt as _,
};

use crate::theme::{
    colors::G2VTheme,
    motion::{MotionPolicy, elevated_surface_opacity},
    radius::RADIUS_LG,
    rems_from_design_pixels, shadows,
    spacing::SPACE_2,
    typography,
};

use super::{TextStyleExt as _, floating_surface};

type TriggerBuilder = Box<dyn FnOnce(&mut Window, &App) -> AnyElement>;
type ContentBuilder = Rc<dyn Fn(&App) -> AnyElement>;
type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

const CONTENT_WIDTH: Rems = rems_from_design_pixels(256.0);
const DEFAULT_SIDE_OFFSET: Rems = rems_from_design_pixels(4.0);
const ENTER_MOTION_DURATION: Duration = Duration::from_millis(100);
const EXIT_MOTION_DURATION: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HoverCardSide {
    Top,
    Left,
    Right,
    #[default]
    Bottom,
}

impl HoverCardSide {
    const fn placement(self) -> Placement {
        match self {
            Self::Top => Placement::Top,
            Self::Left => Placement::Left,
            Self::Right => Placement::Right,
            Self::Bottom => Placement::Bottom,
        }
    }

    const fn motion(self) -> (f32, f32) {
        match self {
            Self::Top => (0.0, 1.0),
            Self::Left => (1.0, 0.0),
            Self::Right => (-1.0, 0.0),
            Self::Bottom => (0.0, -1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HoverCardAlign {
    Start,
    #[default]
    Center,
    End,
}

impl HoverCardAlign {
    const fn positioner_align(self) -> Align {
        match self {
            Self::Start => Align::Start,
            Self::Center => Align::Center,
            Self::End => Align::End,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HoverCardPhase {
    Hidden,
    Opening,
    Open,
    Closing,
}

struct HoverCardState {
    phase: HoverCardPhase,
    epoch: usize,
    task: Option<Task<()>>,
    hovering_trigger: bool,
    hovering_content: bool,
    trigger_bounds: Bounds<Pixels>,
    resolved_side: HoverCardSide,
    deferred_context: Option<DeferredPopover>,
}

impl HoverCardState {
    fn new(side: HoverCardSide) -> Self {
        Self {
            phase: HoverCardPhase::Hidden,
            epoch: 0,
            task: None,
            hovering_trigger: false,
            hovering_content: false,
            trigger_bounds: Bounds::default(),
            resolved_side: side,
            deferred_context: None,
        }
    }

    fn is_mounted(&self) -> bool {
        self.phase != HoverCardPhase::Hidden
    }

    fn cancel_task(&mut self) -> usize {
        self.epoch += 1;
        self.task = None;
        self.epoch
    }

    fn schedule_open(
        &mut self,
        delay: Duration,
        side: HoverCardSide,
        on_open_change: Option<OpenChangeHandler>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let epoch = self.cancel_task();
        self.resolved_side = side;
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update_in(cx, |state, window, cx| {
                if state.epoch != epoch || !state.hovering_trigger {
                    return;
                }
                state.phase = HoverCardPhase::Opening;
                state.deferred_context = Some(GlobalState::register_deferred_popover(cx));
                if let Some(callback) = on_open_change {
                    callback(true, window, cx);
                }
                cx.notify();
                state.finish_opening(window, cx);
            });
        }));
    }

    fn finish_opening(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.cancel_task();
        let motion_duration = MotionPolicy::global(cx).duration(ENTER_MOTION_DURATION);
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(motion_duration).await;
            let _ = this.update_in(cx, |state, _, cx| {
                if state.epoch == epoch && state.phase == HoverCardPhase::Opening {
                    state.phase = HoverCardPhase::Open;
                    state.task = None;
                    cx.notify();
                }
            });
        }));
    }

    fn schedule_close(
        &mut self,
        delay: Duration,
        on_open_change: Option<OpenChangeHandler>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let epoch = self.cancel_task();
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update_in(cx, |state, window, cx| {
                if state.epoch != epoch || state.hovering_trigger || state.hovering_content {
                    return;
                }
                state.phase = HoverCardPhase::Closing;
                if let Some(callback) = on_open_change {
                    callback(false, window, cx);
                }
                cx.notify();
                state.finish_closing(window, cx);
            });
        }));
    }

    fn finish_closing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let epoch = self.cancel_task();
        let motion_duration = MotionPolicy::global(cx).duration(EXIT_MOTION_DURATION);
        self.task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(motion_duration).await;
            let _ = this.update_in(cx, |state, _, cx| {
                if state.epoch == epoch && state.phase == HoverCardPhase::Closing {
                    state.phase = HoverCardPhase::Hidden;
                    state.task = None;
                    state.deferred_context = None;
                    cx.notify();
                }
            });
        }));
    }

    fn reopen_from_closing(
        &mut self,
        on_open_change: Option<OpenChangeHandler>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.phase = HoverCardPhase::Open;
        self.cancel_task();
        if let Some(callback) = on_open_change {
            callback(true, window, cx);
        }
        cx.notify();
    }

    fn on_trigger_hover(
        &mut self,
        hovering: bool,
        behavior: &HoverCardBehavior,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hovering_trigger = hovering;
        if hovering {
            if self.phase == HoverCardPhase::Closing {
                self.reopen_from_closing(behavior.on_open_change.clone(), window, cx);
            } else if self.phase == HoverCardPhase::Hidden {
                self.schedule_open(
                    behavior.open_delay,
                    behavior.side,
                    behavior.on_open_change.clone(),
                    window,
                    cx,
                );
            }
        } else if !self.hovering_content && self.is_mounted() {
            self.schedule_close(
                behavior.close_delay,
                behavior.on_open_change.clone(),
                window,
                cx,
            );
        } else if !self.hovering_content {
            self.cancel_task();
        }
    }

    fn on_content_hover(
        &mut self,
        hovering: bool,
        behavior: &HoverCardBehavior,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hovering_content = hovering;
        if hovering {
            if self.phase == HoverCardPhase::Closing {
                self.reopen_from_closing(behavior.on_open_change.clone(), window, cx);
            } else {
                self.cancel_task();
            }
        } else if !self.hovering_trigger {
            self.schedule_close(
                behavior.close_delay,
                behavior.on_open_change.clone(),
                window,
                cx,
            );
        }
    }
}

#[derive(Clone)]
struct HoverCardBehavior {
    open_delay: Duration,
    close_delay: Duration,
    side: HoverCardSide,
    on_open_change: Option<OpenChangeHandler>,
}

impl Render for HoverCardState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn resolved_side_from_bounds(trigger: Bounds<Pixels>, content: Bounds<Pixels>) -> HoverCardSide {
    let delta_x = content.center().x - trigger.center().x;
    let delta_y = content.center().y - trigger.center().y;
    if delta_x.abs() >= delta_y.abs() {
        if delta_x >= px(0.0) {
            HoverCardSide::Right
        } else {
            HoverCardSide::Left
        }
    } else if delta_y >= px(0.0) {
        HoverCardSide::Bottom
    } else {
        HoverCardSide::Top
    }
}

/// A delayed preview surface associated with a pointer trigger.
///
/// `gpui-base` owns hover timing and the trigger-to-content hover bridge.
/// G2V owns only the shadcn-compatible presentation of the content surface.
#[derive(IntoElement)]
#[must_use]
pub struct HoverCard {
    id: ElementId,
    side: HoverCardSide,
    align: HoverCardAlign,
    side_offset: AbsoluteLength,
    trigger: Option<TriggerBuilder>,
    content: Option<ContentBuilder>,
    children: Vec<AnyElement>,
    trigger_style: StyleRefinement,
    content_style: StyleRefinement,
    open_delay: Duration,
    close_delay: Duration,
    on_open_change: Option<OpenChangeHandler>,
}

impl HoverCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            side: HoverCardSide::Bottom,
            align: HoverCardAlign::Center,
            side_offset: DEFAULT_SIDE_OFFSET.into(),
            trigger: None,
            content: None,
            children: Vec::new(),
            trigger_style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            open_delay: Duration::from_millis(600),
            close_delay: Duration::from_millis(300),
            on_open_change: None,
        }
    }

    pub fn side(mut self, side: HoverCardSide) -> Self {
        self.side = side;
        self
    }

    pub fn align(mut self, align: HoverCardAlign) -> Self {
        self.align = align;
        self
    }

    pub fn side_offset(mut self, offset: impl Into<AbsoluteLength>) -> Self {
        self.side_offset = offset.into();
        self
    }

    pub fn trigger(mut self, trigger: impl IntoElement + 'static) -> Self {
        self.trigger = Some(Box::new(move |_, _| trigger.into_any_element()));
        self
    }

    /// Refines layout on both internal trigger hosts without changing hover-card behavior.
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.trigger_style = style;
        self
    }

    pub fn trigger_with(
        mut self,
        trigger: impl FnOnce(&mut Window, &App) -> AnyElement + 'static,
    ) -> Self {
        self.trigger = Some(Box::new(trigger));
        self
    }

    pub fn content(mut self, content: impl Fn(&App) -> AnyElement + 'static) -> Self {
        self.content = Some(Rc::new(content));
        self
    }

    pub fn open_delay(mut self, duration: Duration) -> Self {
        self.open_delay = duration;
        self
    }

    pub fn close_delay(mut self, duration: Duration) -> Self {
        self.close_delay = duration;
        self
    }

    pub fn on_open_change(
        mut self,
        callback: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }
}

impl Styled for HoverCard {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.content_style
    }
}

impl ParentElement for HoverCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for HoverCard {
    #[expect(
        clippy::too_many_lines,
        reason = "the declarative hover-card composition keeps its trigger and surface lifecycle together"
    )]
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let mut shadow = shadows::tokens(window.appearance()).md;
        shadow.extend(shadows::focus_ring(
            theme.foreground.opacity(0.1),
            shadows::OUTLINE_DEFAULT,
        ));

        let trigger = self
            .trigger
            .map_or_else(|| div().into_any_element(), |trigger| trigger(window, cx));
        let content = self.content;
        let children = self.children;
        let trigger_style = self.trigger_style;
        let content_style = self.content_style;
        let content_id = self.id.clone();
        let preferred_side = self.side;
        let behavior = HoverCardBehavior {
            open_delay: self.open_delay,
            close_delay: self.close_delay,
            side: self.side,
            on_open_change: self.on_open_change,
        };
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| {
            HoverCardState::new(preferred_side)
        });
        let trigger_behavior = behavior.clone();
        let trigger = div()
            .id((self.id.clone(), "trigger"))
            .debug_selector(|| "hover-card-trigger-host".into())
            .flex()
            .flex_none()
            .refine_style(&trigger_style)
            .child(trigger)
            .on_prepaint({
                let state = state.clone();
                move |bounds, _, cx| {
                    state.update(cx, |state, _| state.trigger_bounds = bounds);
                }
            })
            .on_hover(
                window.listener_for(&state, move |state, hovering, window, cx| {
                    state.on_trigger_hover(*hovering, &trigger_behavior, window, cx);
                }),
            );
        let (phase, trigger_bounds, side, mounted) = {
            let snapshot = state.read(cx);
            (
                snapshot.phase,
                snapshot.trigger_bounds,
                snapshot.resolved_side,
                snapshot.is_mounted(),
            )
        };
        let root = div()
            .id(self.id)
            .relative()
            .flex()
            .flex_none()
            .refine_style(&trigger_style)
            .child(trigger);
        if !mounted {
            return root;
        }
        let body = content.as_ref().map(|content| content(cx));
        let content_behavior = behavior.clone();
        let hover = window.listener_for(&state, move |state, hovering, window, cx| {
            state.on_content_hover(*hovering, &content_behavior, window, cx);
        });
        let animated_shadow = shadow.clone();
        let side_offset = self.side_offset.to_pixels(window.rem_size());
        let surface = div()
            .id((content_id, "content"))
            .debug_selector(|| "hover-card-surface".into())
            .w(CONTENT_WIDTH)
            .flex()
            .flex_col()
            .rounded(RADIUS_LG)
            .bg(theme.popover)
            .p_2p5()
            .text_style(typography::tokens().sm)
            .text_color(theme.popover_foreground)
            .shadow(shadow)
            .when_some(body, gpui::ParentElement::child)
            .children(children)
            .refine_style(&content_style)
            .on_prepaint({
                let state = state.clone();
                move |bounds, window, cx| {
                    let resolved = resolved_side_from_bounds(trigger_bounds, bounds);
                    if state.read(cx).resolved_side != resolved {
                        state.update(cx, |state, _| state.resolved_side = resolved);
                        window.request_animation_frame();
                    }
                }
            })
            .on_hover(hover)
            .with_animation(
                match phase {
                    HoverCardPhase::Closing => "hover-card-exit",
                    _ => "hover-card-enter",
                },
                Animation::new(if phase == HoverCardPhase::Closing {
                    EXIT_MOTION_DURATION
                } else {
                    ENTER_MOTION_DURATION
                })
                .with_easing(ease_in_out),
                move |surface, delta| {
                    let visible = if phase == HoverCardPhase::Closing {
                        1.0 - delta
                    } else {
                        delta
                    };
                    let distance = SPACE_2 * (1.0 - visible);
                    let (x, y) = side.motion();
                    surface
                        .relative()
                        .left(distance * x)
                        .top(distance * y)
                        .opacity(elevated_surface_opacity(visible))
                        .shadow(
                            animated_shadow
                                .iter()
                                .cloned()
                                .map(|mut shadow| {
                                    shadow.color.a *= visible;
                                    shadow
                                })
                                .collect::<Vec<_>>(),
                        )
                },
            );
        root.child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_0()
                .child(floating_surface(
                    div().child(
                        Positioner::side(trigger_bounds)
                            .placement(preferred_side.placement())
                            .align(self.align.positioner_align())
                            .offset(side_offset)
                            .child(surface),
                    ),
                    gpui_base::POPUP_PRIORITY,
                )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, Render, TestAppContext, WindowAppearance, point, size};

    struct Harness;

    struct RightStartHarness;

    impl Render for RightStartHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().relative().pt(px(200.0)).pl(px(200.0)).child(
                HoverCard::new("right-start-hover-card")
                    .side(HoverCardSide::Right)
                    .align(HoverCardAlign::Start)
                    .open_delay(Duration::ZERO)
                    .trigger(
                        div()
                            .debug_selector(|| "right-start-trigger".into())
                            .size(px(40.0)),
                    )
                    .content(|_| {
                        div()
                            .debug_selector(|| "right-start-content".into())
                            .h(px(100.0))
                            .into_any_element()
                    }),
            )
        }
    }

    #[test]
    fn defaults_match_the_shadcn_positioning_contract() {
        let hover_card = HoverCard::new("defaults");
        assert_eq!(hover_card.side, HoverCardSide::Bottom);
        assert_eq!(hover_card.align, HoverCardAlign::Center);
        assert_eq!(hover_card.side_offset.to_pixels(px(16.0)), px(4.0));
        assert_eq!(hover_card.side_offset.to_pixels(px(18.0)), px(4.5));
        assert_eq!(CONTENT_WIDTH.to_pixels(px(16.0)), px(256.0));
        assert_eq!(CONTENT_WIDTH.to_pixels(px(18.0)), px(288.0));
        assert_eq!(hover_card.open_delay, Duration::from_millis(600));
        assert_eq!(hover_card.close_delay, Duration::from_millis(300));
        assert_eq!(ENTER_MOTION_DURATION, Duration::from_millis(100));
        assert_eq!(EXIT_MOTION_DURATION, Duration::from_millis(50));
    }

    #[test]
    fn motion_direction_tracks_the_resolved_side_after_flipping() {
        let trigger = Bounds::new(point(px(100.0), px(100.0)), size(px(40.0), px(20.0)));
        assert_eq!(
            resolved_side_from_bounds(
                trigger,
                Bounds::new(point(px(144.0), px(90.0)), size(px(80.0), px(40.0))),
            ),
            HoverCardSide::Right
        );
        assert_eq!(
            resolved_side_from_bounds(
                trigger,
                Bounds::new(point(px(16.0), px(90.0)), size(px(80.0), px(40.0))),
            ),
            HoverCardSide::Left
        );
    }

    #[gpui::test]
    fn right_start_uses_the_same_zero_sized_overlay_contract_as_popover(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (_, cx) = cx.add_window_view(|_, _| RightStartHarness);
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_mouse_move(
            point(px(210.0), px(210.0)),
            None,
            gpui::Modifiers::default(),
        );
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.draw(cx).clear(cx);
        });

        let trigger = cx
            .debug_bounds("right-start-trigger")
            .expect("trigger must render");
        let trigger_host = cx
            .debug_bounds("hover-card-trigger-host")
            .expect("trigger host must render");
        let content = cx
            .debug_bounds("right-start-content")
            .expect("content must render");
        let surface = cx
            .debug_bounds("hover-card-surface")
            .expect("surface must render");
        assert_eq!(
            trigger.top(),
            surface.top(),
            "trigger={trigger:?} trigger_host={trigger_host:?} surface={surface:?} content={content:?}"
        );
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            HoverCard::new("hover-card")
                .open_delay(Duration::from_millis(100))
                .close_delay(Duration::from_millis(100))
                .trigger(
                    div()
                        .debug_selector(|| "hover-card-trigger".into())
                        .size(px(20.0)),
                )
                .content(|_| {
                    div()
                        .debug_selector(|| "hover-card-body".into())
                        .child("Preview")
                        .into_any_element()
                })
        }
    }

    #[gpui::test]
    fn opens_and_closes_after_the_configured_delays(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let delay = Duration::from_millis(100);
        let (_, cx) = cx.add_window_view(|_, _| Harness);
        cx.update(|window, cx| window.draw(cx).clear(cx));

        cx.simulate_mouse_move(point(px(10.0), px(10.0)), None, gpui::Modifiers::default());
        cx.executor().advance_clock(delay);
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("hover-card-body").is_some());

        cx.simulate_mouse_move(
            point(px(400.0), px(400.0)),
            None,
            gpui::Modifiers::default(),
        );
        cx.executor().advance_clock(delay);
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("hover-card-body").is_some());

        cx.executor().advance_clock(EXIT_MOTION_DURATION);
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(cx.debug_bounds("hover-card-body").is_none());
    }
}
