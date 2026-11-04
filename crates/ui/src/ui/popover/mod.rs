//! G2V's canonical shadcn-compatible anchored popover.

use std::rc::Rc;

mod motion;
mod positioner;
mod primitive;

pub use primitive::PopoverState;
use primitive::{OpenChangeHandler, PopoverPhase};

use crate::theme::border::BORDER_DEFAULT;
use crate::theme::colors::G2VTheme;
use crate::theme::radius::RADIUS_LG;
use crate::theme::shadows;
use crate::theme::spacing::{SPACE_1, SPACE_2_5};
use crate::theme::typography;
use crate::theme::{rems_from_design_pixels, rems_from_pixels};
use gpui::{
    AbsoluteLength, Animation, AnimationExt as _, AnyElement, App, Bounds, ElementId, Entity,
    FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _,
    Pixels, Rems, RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    div, ease_in_out, prelude::FluentBuilder as _,
};
use gpui_base::{
    Align as PositionerAlign, ElementExt as _, Placement, Positioner, actions::Cancel,
};

use super::{Button, TextStyleExt as _, floating_surface};

const CONTEXT: &str = "G2VPopover";
const CONTENT_WIDTH: Rems = rems_from_design_pixels(288.0);
const DEFAULT_SIDE_OFFSET: Rems = rems_from_design_pixels(4.0);
const VIEWPORT_MARGIN: Rems = rems_from_design_pixels(5.0);
use motion::{MOTION_DURATION, animate_surface};
use positioner::{resolved_side_from_bounds, shifted_trigger_bounds};

type TriggerBuilder = Box<dyn FnOnce(bool, &Window, &App) -> AnyElement>;
type ContentBuilder = Rc<dyn Fn(&Entity<PopoverState>, &mut Window, &mut App) -> AnyElement>;

#[derive(Clone, Copy)]
pub(crate) enum PopoverWidth {
    Fixed(Rems),
    ContentWithAnchorMinimum(Pixels),
}

#[derive(Clone, Copy, Default)]
pub(crate) enum PopoverTriggerLayout {
    #[default]
    Intrinsic,
    Fill,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(crate) enum PopoverInteractionMode {
    #[default]
    NonModal,
    Modal,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(crate) enum PopoverFocusOwner {
    #[default]
    Surface,
    Content,
}

#[derive(Clone, Copy)]
pub(crate) struct PopoverSurfaceStyle {
    width: PopoverWidth,
    gap: Rems,
    padding: Rems,
    radius: Pixels,
    border: bool,
}

impl PopoverSurfaceStyle {
    fn standard() -> Self {
        Self {
            width: PopoverWidth::Fixed(CONTENT_WIDTH),
            gap: rems_from_pixels(SPACE_2_5),
            padding: rems_from_pixels(SPACE_2_5),
            radius: RADIUS_LG,
            border: true,
        }
    }

    pub(crate) fn menu(minimum_width: Pixels) -> Self {
        Self {
            width: PopoverWidth::ContentWithAnchorMinimum(minimum_width),
            gap: Rems::ZERO,
            padding: rems_from_design_pixels(4.0),
            radius: RADIUS_LG,
            border: true,
        }
    }

    pub(crate) fn command() -> Self {
        Self {
            width: PopoverWidth::Fixed(CONTENT_WIDTH),
            gap: Rems::ZERO,
            padding: Rems::ZERO,
            radius: RADIUS_LG,
            border: true,
        }
    }

    fn compact() -> Self {
        Self {
            padding: rems_from_pixels(SPACE_1),
            ..Self::standard()
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PopoverSide {
    Top,
    Left,
    Right,
    InlineStart,
    InlineEnd,
    #[default]
    Bottom,
}

impl PopoverSide {
    const fn placement(self) -> Placement {
        match self {
            Self::Top => Placement::Top,
            Self::Left | Self::InlineStart => Placement::Left,
            Self::Right | Self::InlineEnd => Placement::Right,
            Self::Bottom => Placement::Bottom,
        }
    }

    const fn motion(self) -> (f32, f32) {
        match self {
            Self::Top => (0.0, 1.0),
            Self::Left | Self::InlineStart => (1.0, 0.0),
            Self::Right | Self::InlineEnd => (-1.0, 0.0),
            Self::Bottom => (0.0, -1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PopoverAlign {
    Start,
    #[default]
    Center,
    End,
}

impl PopoverAlign {
    const fn positioner(self) -> PositionerAlign {
        match self {
            Self::Start => PositionerAlign::Start,
            Self::Center => PositionerAlign::Center,
            Self::End => PositionerAlign::End,
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Popover {
    id: ElementId,
    trigger: Option<TriggerBuilder>,
    button_trigger: Option<Button>,
    content: Option<ContentBuilder>,
    side: PopoverSide,
    align: PopoverAlign,
    side_offset: AbsoluteLength,
    align_offset: AbsoluteLength,
    default_open: bool,
    open: Option<bool>,
    overlay_closable: bool,
    on_open_change: Option<OpenChangeHandler>,
    surface_style: PopoverSurfaceStyle,
    trigger_layout: PopoverTriggerLayout,
    trigger_activates: bool,
    disabled: bool,
    interaction_mode: PopoverInteractionMode,
    focus_owner: PopoverFocusOwner,
    initial_focus: Option<FocusHandle>,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            trigger: None,
            button_trigger: None,
            content: None,
            side: PopoverSide::Bottom,
            align: PopoverAlign::Center,
            side_offset: DEFAULT_SIDE_OFFSET.into(),
            align_offset: Rems::ZERO.into(),
            default_open: false,
            open: None,
            overlay_closable: true,
            on_open_change: None,
            surface_style: PopoverSurfaceStyle::standard(),
            trigger_layout: PopoverTriggerLayout::Intrinsic,
            trigger_activates: true,
            disabled: false,
            interaction_mode: PopoverInteractionMode::NonModal,
            focus_owner: PopoverFocusOwner::Surface,
            initial_focus: None,
        }
    }

    pub fn trigger(
        mut self,
        trigger: impl FnOnce(bool, &Window, &App) -> AnyElement + 'static,
    ) -> Self {
        self.trigger = Some(Box::new(trigger));
        self
    }

    pub fn trigger_button(mut self, trigger: Button) -> Self {
        self.button_trigger = Some(trigger);
        self
    }

    pub fn content(
        mut self,
        content: impl Fn(&Entity<PopoverState>, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.content = Some(Rc::new(content));
        self
    }

    pub fn side(mut self, side: PopoverSide) -> Self {
        self.side = side;
        self
    }

    pub fn align(mut self, align: PopoverAlign) -> Self {
        self.align = align;
        self
    }

    pub fn compact(mut self) -> Self {
        self.surface_style = PopoverSurfaceStyle::compact();
        self
    }

    pub fn width(mut self, width: Rems) -> Self {
        self.surface_style.width = PopoverWidth::Fixed(width);
        self
    }

    pub fn radius(mut self, radius: Pixels) -> Self {
        self.surface_style.radius = radius;
        self
    }

    pub fn content_padding(mut self, padding: Rems) -> Self {
        self.surface_style.padding = padding;
        self
    }

    pub fn side_offset(mut self, offset: impl Into<AbsoluteLength>) -> Self {
        self.side_offset = offset.into();
        self
    }

    pub fn align_offset(mut self, offset: impl Into<AbsoluteLength>) -> Self {
        self.align_offset = offset.into();
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    pub fn on_open_change(
        mut self,
        callback: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    pub(crate) fn surface_style(mut self, style: PopoverSurfaceStyle) -> Self {
        self.surface_style = style;
        self
    }

    pub(crate) fn trigger_layout(mut self, layout: PopoverTriggerLayout) -> Self {
        self.trigger_layout = layout;
        self
    }

    pub(crate) fn passive_trigger(mut self) -> Self {
        self.trigger_activates = false;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub(crate) fn interaction_mode(mut self, mode: PopoverInteractionMode) -> Self {
        self.interaction_mode = mode;
        self
    }

    pub(crate) fn focus_owner(mut self, owner: PopoverFocusOwner) -> Self {
        self.focus_owner = owner;
        self
    }

    pub fn initial_focus(mut self, focus: FocusHandle) -> Self {
        self.initial_focus = Some(focus);
        self
    }
}

impl RenderOnce for Popover {
    #[expect(
        clippy::too_many_lines,
        reason = "the declarative popover composition keeps trigger and controlled-state wiring together"
    )]
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let surface_config = PopoverSurfaceConfig {
            id: self.id.clone(),
            overlay_closable: self.overlay_closable,
            style: self.surface_style,
            focus_owner: self.focus_owner,
        };
        let state = window.use_keyed_state(self.id.clone(), cx, |_, cx| {
            PopoverState::new(self.side, cx)
        });

        let snapshot = PopoverSnapshot::read(&state, cx);
        let parent_view_id = window.current_view();
        let wrapper_owns_activation = self.button_trigger.is_none() && self.trigger_activates;
        let trigger = build_trigger(
            self.trigger,
            self.button_trigger,
            snapshot.open,
            &state,
            parent_view_id,
            self.disabled,
            window,
            cx,
        );
        let trigger_host = render_trigger(
            self.id.clone(),
            trigger,
            state.clone(),
            parent_view_id,
            wrapper_owns_activation,
            self.trigger_layout,
        );
        let controlled_open = self.open;
        let default_open = self.default_open;
        let on_open_change = self.on_open_change;
        let initial_focus = self.initial_focus;
        let root = div()
            .id(self.id.clone())
            .relative()
            .flex()
            .flex_none()
            .when(
                matches!(self.trigger_layout, PopoverTriggerLayout::Fill),
                gpui::Styled::w_full,
            )
            .key_context(CONTEXT)
            .on_prepaint({
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, cx| {
                        state.set_initial_focus(initial_focus.clone());
                        state.reconcile(
                            controlled_open,
                            default_open,
                            on_open_change.clone(),
                            window,
                            cx,
                        );
                    });
                }
            })
            .on_action(window.listener_for(&state, PopoverState::on_cancel))
            .on_key_down({
                let state = state.clone();
                move |event: &gpui::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" && state.read(cx).is_mounted() {
                        cx.stop_propagation();
                        state.update(cx, |state, cx| state.dismiss(window, cx));
                    }
                }
            })
            .child(trigger_host);
        if !snapshot.mounted || !snapshot.trigger_captured {
            return root.into_any_element();
        }

        let content = self.content.map(|content| content(&state, window, cx));
        let Some(content) = content else {
            return root.into_any_element();
        };
        let viewport = window.viewport_size();
        let rem_size = window.rem_size();
        let surface = render_surface(
            &surface_config,
            content,
            &state,
            snapshot,
            parent_view_id,
            window,
            cx,
        );

        let positioner = position_surface(
            self.side,
            self.align,
            self.side_offset.to_pixels(rem_size),
            self.align_offset.to_pixels(rem_size),
            VIEWPORT_MARGIN.to_pixels(rem_size),
            snapshot.trigger_bounds,
            surface.into_any_element(),
        );
        root.child(pinned_popover_layer(
            self.interaction_mode,
            snapshot,
            state.clone(),
            viewport,
            positioner,
        ))
        .into_any_element()
    }
}

fn build_trigger(
    custom: Option<TriggerBuilder>,
    button: Option<Button>,
    open: bool,
    state: &Entity<PopoverState>,
    parent_view_id: gpui::EntityId,
    disabled: bool,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    button.map_or_else(
        || {
            custom.map_or_else(
                || div().into_any_element(),
                |trigger| trigger(open, window, cx),
            )
        },
        |trigger| {
            trigger
                .track_focus(&state.read(cx).trigger_focus_handle)
                .selected(open)
                .has_popup(true)
                .when(!disabled, |trigger| {
                    trigger.on_click({
                        let state = state.clone();
                        move |_, window, cx| {
                            state.update(cx, |state, cx| state.handle_trigger_press(window, cx));
                            cx.notify(parent_view_id);
                        }
                    })
                })
                .into_any_element()
        },
    )
}

fn position_surface(
    side: PopoverSide,
    align: PopoverAlign,
    side_offset: Pixels,
    align_offset: Pixels,
    viewport_margin: Pixels,
    trigger_bounds: Bounds<Pixels>,
    surface: AnyElement,
) -> Positioner {
    let trigger_bounds = shifted_trigger_bounds(trigger_bounds, side, align_offset);
    Positioner::side(trigger_bounds)
        .placement(side.placement())
        .align(align.positioner())
        .offset(side_offset)
        .margin(viewport_margin)
        .child(surface)
}

#[derive(Clone, Copy)]
struct PopoverSnapshot {
    open: bool,
    phase: PopoverPhase,
    generation: usize,
    mounted: bool,
    trigger_bounds: Bounds<Pixels>,
    trigger_captured: bool,
    resolved_side: PopoverSide,
}

struct PopoverSurfaceConfig {
    id: ElementId,
    overlay_closable: bool,
    style: PopoverSurfaceStyle,
    focus_owner: PopoverFocusOwner,
}

impl PopoverSnapshot {
    fn read(state: &Entity<PopoverState>, cx: &App) -> Self {
        let state = state.read(cx);
        Self {
            open: state.is_open(),
            phase: state.phase,
            generation: state.generation(),
            mounted: state.is_mounted(),
            trigger_bounds: state.trigger_bounds,
            trigger_captured: state.trigger_captured,
            resolved_side: state.resolved_side,
        }
    }
}

fn render_trigger(
    id: ElementId,
    trigger: AnyElement,
    state: Entity<PopoverState>,
    parent_view_id: gpui::EntityId,
    wrapper_owns_activation: bool,
    layout: PopoverTriggerLayout,
) -> impl IntoElement {
    div()
        .id((id, "trigger"))
        .flex()
        .flex_none()
        .when(matches!(layout, PopoverTriggerLayout::Fill), |trigger| {
            trigger.w_full()
        })
        .child(trigger)
        .when(wrapper_owns_activation, |trigger| {
            trigger.on_mouse_down(gpui::MouseButton::Left, {
                let state = state.clone();
                move |_, window, cx| {
                    cx.stop_propagation();
                    state.update(cx, |state, cx| state.handle_trigger_press(window, cx));
                    cx.notify(parent_view_id);
                }
            })
        })
        .on_prepaint(move |bounds, window, cx| {
            let changed =
                state.read(cx).trigger_bounds != bounds || !state.read(cx).trigger_captured;
            if changed {
                state.update(cx, |state, _| {
                    state.trigger_bounds = bounds;
                    state.trigger_captured = true;
                });
                window.request_animation_frame();
            }
        })
}

fn pinned_popover_layer(
    interaction_mode: PopoverInteractionMode,
    snapshot: PopoverSnapshot,
    state: Entity<PopoverState>,
    viewport: gpui::Size<Pixels>,
    positioner: Positioner,
) -> AnyElement {
    let layer = floating_surface(
        div()
            .when(interaction_mode == PopoverInteractionMode::Modal, |layer| {
                layer.child(
                    div()
                        .debug_selector(|| "popover-modal-layer".into())
                        .absolute()
                        .left(-snapshot.trigger_bounds.origin.x)
                        .top(-snapshot.trigger_bounds.origin.y)
                        .w(viewport.width)
                        .h(viewport.height)
                        .occlude()
                        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
                        .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            state.update(cx, |state, cx| state.dismiss(window, cx));
                        }),
                )
            })
            .child(positioner),
        gpui_base::POPUP_PRIORITY,
    );
    div()
        .absolute()
        .top_0()
        .left_0()
        .size_0()
        .child(layer)
        .into_any_element()
}

#[expect(
    clippy::too_many_lines,
    reason = "the popover surface builder follows paint order and has no independent state"
)]
fn render_surface(
    config: &PopoverSurfaceConfig,
    content: AnyElement,
    state: &Entity<PopoverState>,
    snapshot: PopoverSnapshot,
    parent_view_id: gpui::EntityId,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let theme = G2VTheme::global(cx).clone();
    let mut shadow = shadows::tokens(window.appearance()).md;
    if !config.style.border {
        shadow.extend(shadows::focus_ring(
            theme.foreground.opacity(0.1),
            shadows::OUTLINE_DEFAULT,
        ));
    }
    let (width, minimum_width) = match config.style.width {
        PopoverWidth::Fixed(width) => (Some(width), None),
        PopoverWidth::ContentWithAnchorMinimum(minimum) => {
            (None, Some(snapshot.trigger_bounds.size.width.max(minimum)))
        }
    };
    let animation_name = match snapshot.phase {
        PopoverPhase::Closing => "close",
        PopoverPhase::Opening | PopoverPhase::Open | PopoverPhase::Hidden => "open",
    };
    let animation_id = (
        config.id.clone(),
        SharedString::from(format!("{animation_name}-{}", snapshot.generation)),
    );
    let animated_shadow = shadow.clone();
    div()
        .id((config.id.clone(), "content"))
        .role(gpui::Role::Dialog)
        .occlude()
        .tab_group()
        .when(
            config.focus_owner == PopoverFocusOwner::Surface,
            |surface| surface.track_focus(&state.read(cx).focus_handle(cx)),
        )
        .key_context(CONTEXT)
        .on_action(window.listener_for(state, PopoverState::on_cancel))
        .on_key_down({
            let state = state.clone();
            move |event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    state.update(cx, |state, cx| state.dismiss(window, cx));
                }
            }
        })
        .when_some(width, gpui::Styled::w)
        .when_some(minimum_width, gpui::Styled::min_w)
        .flex()
        .flex_col()
        .gap(config.style.gap)
        .overflow_hidden()
        .rounded(config.style.radius)
        .when(config.style.border, |surface| {
            surface
                .border(BORDER_DEFAULT)
                .border_color(theme.popover_border)
        })
        .bg(theme.popover)
        .p(config.style.padding)
        .text_style(typography::tokens().sm)
        .text_color(theme.popover_foreground)
        .shadow(shadow)
        .child(content)
        .when(snapshot.phase == PopoverPhase::Closing, |surface| {
            surface
                .relative()
                .child(div().absolute().inset_0().occlude())
        })
        .on_prepaint({
            let state = state.clone();
            move |bounds, window, cx| {
                let side = resolved_side_from_bounds(
                    snapshot.trigger_bounds,
                    bounds,
                    snapshot.resolved_side,
                );
                if state.read(cx).resolved_side != side {
                    state.update(cx, |state, _| state.resolved_side = side);
                    window.request_animation_frame();
                }
            }
        })
        .when(config.overlay_closable, |surface| {
            surface.on_mouse_down_out({
                let state = state.clone();
                move |_, window, cx| {
                    state.update(cx, |state, cx| state.dismiss(window, cx));
                    cx.notify(parent_view_id);
                }
            })
        })
        .with_animation(
            animation_id,
            Animation::new(MOTION_DURATION).with_easing(ease_in_out),
            move |surface, delta| {
                animate_surface(
                    surface,
                    snapshot.phase,
                    snapshot.resolved_side,
                    &animated_shadow,
                    delta,
                )
            },
        )
}

#[derive(IntoElement, Default)]
#[must_use]
pub struct PopoverHeader {
    children: Vec<AnyElement>,
}

impl PopoverHeader {
    pub fn new() -> Self {
        Self::default()
    }
}

impl gpui::ParentElement for PopoverHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for PopoverHeader {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_0p5()
            .text_style(typography::tokens().sm)
            .children(self.children)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct PopoverTitle(SharedString);

impl PopoverTitle {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self(title.into())
    }
}

impl RenderOnce for PopoverTitle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child(self.0)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct PopoverDescription(SharedString);

impl PopoverDescription {
    pub fn new(description: impl Into<SharedString>) -> Self {
        Self(description.into())
    }
}

impl RenderOnce for PopoverDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .text_color(G2VTheme::global(cx).muted_foreground)
            .child(self.0)
    }
}

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, WindowAppearance, point,
        px,
    };
    use std::{
        cell::{Cell, RefCell},
        time::Duration,
    };

    #[test]
    fn defaults_match_the_react_popover_contract() {
        let popover = Popover::new("popover");
        assert_eq!(popover.side, PopoverSide::Bottom);
        assert_eq!(popover.align, PopoverAlign::Center);
        assert_eq!(popover.side_offset.to_pixels(px(16.0)), px(4.0));
        assert_eq!(popover.side_offset.to_pixels(px(18.0)), px(4.5));
        assert_eq!(popover.align_offset.to_pixels(px(16.0)), px(0.0));
        assert_eq!(CONTENT_WIDTH.to_pixels(px(16.0)), px(288.0));
        assert_eq!(CONTENT_WIDTH.to_pixels(px(18.0)), px(324.0));
        assert_eq!(VIEWPORT_MARGIN.to_pixels(px(16.0)), px(5.0));
        assert_eq!(VIEWPORT_MARGIN.to_pixels(px(18.0)), px(5.625));
        assert_eq!(MOTION_DURATION, Duration::from_millis(100));
        let command = PopoverSurfaceStyle::command();
        assert!(
            matches!(command.width, PopoverWidth::Fixed(width) if width.to_pixels(px(16.0)) == px(288.0))
        );
        assert_eq!(command.padding, gpui::Rems::ZERO);
        assert_eq!(command.gap, gpui::Rems::ZERO);
        assert!(command.border);

        let menu = PopoverSurfaceStyle::menu(px(128.0));
        assert!(menu.border);

        let compact = Popover::new("compact-popover").compact();
        assert_eq!(compact.surface_style.padding.to_pixels(px(16.0)), px(4.0));
        let flush = Popover::new("flush-popover").content_padding(Rems::ZERO);
        assert_eq!(flush.surface_style.padding, Rems::ZERO);
        assert_eq!(compact.surface_style.radius, RADIUS_LG);
    }

    struct PositionHarness;

    impl Render for PositionHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().relative().child(
                div().absolute().left(px(400.0)).top(px(100.0)).child(
                    Popover::new("position-popover")
                        .default_open(true)
                        .trigger(|_, _, _| {
                            div()
                                .debug_selector(|| "position-trigger".into())
                                .w(px(100.0))
                                .h(px(30.0))
                                .into_any_element()
                        })
                        .content(|_, _, _| {
                            div()
                                .debug_selector(|| "position-content".into())
                                .w_full()
                                .h(px(40.0))
                                .into_any_element()
                        }),
                ),
            )
        }
    }

    struct InitialFocusHarness {
        content_focus: FocusHandle,
    }

    impl Render for InitialFocusHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Popover::new("initial-focus-popover")
                .default_open(true)
                .initial_focus(self.content_focus.clone())
                .trigger(|_, _, _| div().w(px(40.0)).h(px(20.0)).into_any_element())
                .content({
                    let content_focus = self.content_focus.clone();
                    move |_, _, _| {
                        div()
                            .track_focus(&content_focus)
                            .w(px(120.0))
                            .h(px(40.0))
                            .into_any_element()
                    }
                })
        }
    }

    struct KeyboardTriggerHarness {
        changes: Rc<RefCell<Vec<bool>>>,
        trigger_focused_while_closing: Rc<Cell<Option<bool>>>,
        disabled: bool,
    }

    fn simulate_keyboard_click(window: &mut gpui::VisualTestContext, key: &str) {
        let keystroke = Keystroke::parse(key).unwrap();
        window.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        window.simulate_event(KeyUpEvent { keystroke });
    }

    impl Render for KeyboardTriggerHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let changes = self.changes.clone();
            let trigger_focused_while_closing = self.trigger_focused_while_closing.clone();
            Popover::new("keyboard-popover")
                .disabled(self.disabled)
                .trigger_button(Button::new("keyboard-trigger").label("Open"))
                .content(move |state, window, cx| {
                    if state.read(cx).phase == PopoverPhase::Closing {
                        trigger_focused_while_closing
                            .set(Some(state.read(cx).trigger_focus_handle.is_focused(window)));
                    }
                    div()
                        .debug_selector(|| "keyboard-content".into())
                        .h(px(40.0))
                        .into_any_element()
                })
                .on_open_change(move |open, _, _| changes.borrow_mut().push(open))
        }
    }

    #[gpui::test]
    fn content_is_anchored_to_the_trigger_bounds(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (_, window) = cx.add_window_view(|_, _| PositionHarness);

        for _ in 0..2 {
            window.update(|window, cx| window.draw(cx).clear(cx));
        }
        std::thread::sleep(MOTION_DURATION * 2);
        window.update(|window, cx| window.draw(cx).clear(cx));

        let trigger = window.debug_bounds("position-trigger").unwrap();
        let content = window.debug_bounds("position-content").unwrap();
        let surface_center_x = content.origin.x + content.size.width / 2.0;

        assert_eq!(trigger.origin, point(px(400.0), px(100.0)));
        assert_eq!(surface_center_x, trigger.center().x);
        assert_eq!(
            content.origin.y,
            trigger.bottom() + DEFAULT_SIDE_OFFSET.to_pixels(px(16.0)) + BORDER_DEFAULT + SPACE_2_5
        );
    }

    #[gpui::test]
    fn initial_focus_targets_popover_content(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (harness, window) = cx.add_window_view(|_, cx| InitialFocusHarness {
            content_focus: cx.focus_handle(),
        });

        window.update(|window, cx| window.draw(cx).clear(cx));
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.run_until_parked();

        window.update(|window, cx| {
            assert!(harness.read(cx).content_focus.is_focused(window));
        });
    }

    #[gpui::test]
    fn button_trigger_opens_with_enter_and_space(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let changes = Rc::new(RefCell::new(Vec::new()));
        let trigger_focused_while_closing = Rc::new(Cell::new(None));
        let (_, window) = cx.add_window_view({
            let changes = changes.clone();
            let trigger_focused_while_closing = trigger_focused_while_closing.clone();
            move |_, _| KeyboardTriggerHarness {
                changes,
                trigger_focused_while_closing,
                disabled: false,
            }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));

        window.simulate_click(point(px(10.0), px(10.0)), Modifiers::default());
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("keyboard-content").is_some());

        window.simulate_keystrokes("escape");
        assert_eq!(&*changes.borrow(), &[true, false]);
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(trigger_focused_while_closing.get(), Some(true));
        window.executor().advance_clock(MOTION_DURATION * 2);
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("keyboard-content").is_none());
        simulate_keyboard_click(window, "enter");
        assert_eq!(&*changes.borrow(), &[true, false, true]);
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("keyboard-content").is_some());

        window.simulate_keystrokes("escape");
        window.executor().advance_clock(MOTION_DURATION * 2);
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("keyboard-content").is_none());

        simulate_keyboard_click(window, "space");
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert!(window.debug_bounds("keyboard-content").is_some());
    }

    #[gpui::test]
    fn disabled_button_trigger_does_not_open(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let changes = Rc::new(RefCell::new(Vec::new()));
        let trigger_focused_while_closing = Rc::new(Cell::new(None));
        let (_, window) = cx.add_window_view({
            let changes = changes.clone();
            let trigger_focused_while_closing = trigger_focused_while_closing.clone();
            move |_, _| KeyboardTriggerHarness {
                changes,
                trigger_focused_while_closing,
                disabled: true,
            }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));

        window.simulate_click(point(px(10.0), px(10.0)), Modifiers::default());
        window.update(|window, cx| window.draw(cx).clear(cx));

        assert!(changes.borrow().is_empty());
        assert!(window.debug_bounds("keyboard-content").is_none());
    }
}
