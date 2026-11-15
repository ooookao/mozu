use crate::theme::Theme;
use crate::ui::utils::motion::{MotionProfile, TimedValue, mix_oklab};
use crate::ui::utils::presentation::{CapTrimmed, ComponentTheme, ControlSize, Shape, ThemeScheme};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, BoxShadow, ElementId, FocusHandle, Hsla, IntoElement, Refineable, RenderOnce,
    SharedString, SpringState, StyleRefinement, Styled, Window, div, px,
};
use kinu_ui::{
    ColorScheme, FieldControl, FieldControlBinding, FieldLabelBehavior, SwitchDrag,
    SwitchDragPhase, SwitchPointerKind,
};
use std::time::Instant;

#[derive(Clone, Copy)]
struct Metrics {
    width: f32,
    height: f32,
    thumb: f32,
    press_extend: f32,
    press_shrink: f32,
}
fn metrics(size: ControlSize) -> Metrics {
    match size {
        ControlSize::Default => Metrics {
            width: 34.,
            height: 20.,
            thumb: 16.,
            press_extend: 4.,
            press_shrink: 4.,
        },
        ControlSize::Compact => Metrics {
            width: 28.,
            height: 16.,
            thumb: 12.,
            press_extend: 3.,
            press_shrink: 3.,
        },
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Geometry {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
fn geometry(checked: bool, hovered: bool, pressed: bool, m: Metrics) -> Geometry {
    let extra = if pressed {
        m.press_extend
    } else if hovered {
        2.
    } else {
        0.
    };
    Geometry {
        x: if checked {
            m.width - 2. - m.thumb - extra
        } else {
            2.
        },
        y: if pressed {
            2. + m.press_shrink / 2.
        } else {
            2.
        },
        width: m.thumb + extra,
        height: m.thumb - if pressed { m.press_shrink } else { 0. },
    }
}
struct SpringValue {
    target: f32,
    state: SpringState,
    at: Instant,
}
impl SpringValue {
    fn new(value: f32, at: Instant) -> Self {
        Self {
            target: value,
            state: SpringState {
                position: value,
                velocity: 0.,
            },
            at,
        }
    }
    fn sample(&mut self, target: f32, motion: MotionProfile, now: Instant) -> (f32, bool) {
        self.state = motion.spring.step(
            self.state,
            self.target,
            now.duration_since(self.at).as_secs_f32(),
        );
        self.at = now;
        self.target = target;
        let settled = motion.spring.is_settled(self.state, target, 0.001);
        if settled {
            self.state = SpringState {
                position: target,
                velocity: 0.,
            };
        }
        (self.state.position, settled)
    }
    fn pointer(&mut self, value: f32, now: Instant) {
        let elapsed = now.duration_since(self.at).as_secs_f32();
        self.state.velocity = if elapsed > 0. {
            (value - self.state.position) / elapsed
        } else {
            0.
        };
        self.state.position = value;
        self.target = value;
        self.at = now;
    }
    fn complete(&mut self, target: f32, now: Instant) {
        self.state = SpringState {
            position: target,
            velocity: 0.,
        };
        self.target = target;
        self.at = now;
    }
    fn release(&mut self, now: Instant) {
        self.at = now;
        self.target = self.state.position;
    }
}
struct ColorValue {
    from: Hsla,
    target: Hsla,
    phase: TimedValue,
}
impl ColorValue {
    fn new(value: Hsla, now: Instant) -> Self {
        Self {
            from: value,
            target: value,
            phase: TimedValue::new(1., now),
        }
    }
    fn current(&self, now: Instant, easing: impl Fn(f32) -> f32) -> (Hsla, bool) {
        let (progress, settled) = self.phase.sample(now, &easing);
        (
            mix_oklab(self.from, self.target, f64::from(progress)),
            settled,
        )
    }
    fn sample(
        &mut self,
        target: Hsla,
        motion: MotionProfile,
        now: Instant,
        easing: impl Fn(f32) -> f32,
    ) -> (Hsla, bool) {
        if self.target != target {
            self.from = self.current(now, &easing).0;
            self.target = target;
            self.phase = TimedValue::new(0., now);
            self.phase.retarget(1., now, motion.enter, &easing);
        }
        self.current(now, &easing)
    }
}
struct Presentation {
    focus: FocusHandle,
    hovered: bool,
    pressed: bool,
    dragging: bool,
    disabled: bool,
    x: SpringValue,
    y: SpringValue,
    width: SpringValue,
    height: SpringValue,
    track: ColorValue,
    label: ColorValue,
}
impl Presentation {
    fn new(
        focus: FocusHandle,
        checked: bool,
        disabled: bool,
        tokens: ComponentTheme,
        colors: &Theme,
        now: Instant,
    ) -> Self {
        let g = geometry(checked, false, false, metrics(tokens.size));
        Self {
            focus,
            hovered: false,
            pressed: false,
            dragging: false,
            disabled,
            x: SpringValue::new(g.x, now),
            y: SpringValue::new(g.y, now),
            width: SpringValue::new(g.width, now),
            height: SpringValue::new(g.height, now),
            track: ColorValue::new(
                if checked {
                    gpui::Hsla::from(colors.brand)
                } else {
                    gpui::Hsla::from(colors.accent)
                },
                now,
            ),
            label: ColorValue::new(
                if checked {
                    gpui::Hsla::from(colors.foreground)
                } else {
                    gpui::Hsla::from(colors.muted_foreground)
                },
                now,
            ),
        }
    }
}

/// A controlled, source-owned switch row. The primitive owns activation, dragging, focus and AX.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    primitive: kinu_ui::Switch,
    checked: bool,
    label: Option<SharedString>,
    label_content: Option<AnyElement>,
    tokens: Option<ComponentTheme>,
    thumb_motion: Option<MotionProfile>,
    focus: Option<FocusHandle>,
    style: StyleRefinement,
}
impl Switch {
    pub fn new(id: impl Into<ElementId>, checked: bool) -> Self {
        let id = id.into();
        Self {
            primitive: kinu_ui::Switch::new(id.clone(), checked),
            id,
            checked,
            label: None,
            label_content: None,
            tokens: None,
            thumb_motion: None,
            focus: None,
            style: StyleRefinement::default(),
        }
    }
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.primitive = self.primitive.accessibility_label(label.clone());
        self.label = Some(label);
        self
    }
    /// Replaces the visual label while retaining its explicit accessible name.
    pub fn label_content(mut self, content: impl IntoElement) -> Self {
        self.label_content = Some(content.into_any_element());
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.primitive = self.primitive.accessibility_label(label);
        self
    }
    pub fn tokens(mut self, tokens: ComponentTheme) -> Self {
        self.tokens = Some(tokens.clone());
        self
    }
    pub fn thumb_motion(mut self, motion: MotionProfile) -> Self {
        self.thumb_motion = Some(motion);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.primitive = self.primitive.disabled(disabled);
        self
    }
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.primitive = self.primitive.read_only(read_only);
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.focus = Some(focus.clone());
        self
    }
    pub fn on_change(mut self, change: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.primitive = self.primitive.on_change(change);
        self
    }
}
impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl FieldControl for Switch {
    type Value = bool;
    fn field_value(&self) -> bool {
        self.primitive.field_value()
    }
    fn field_filled(&self) -> bool {
        self.primitive.field_filled()
    }
    fn field_disabled(&self) -> bool {
        self.primitive.field_disabled()
    }
    fn field_label_behavior(&self) -> FieldLabelBehavior {
        self.primitive.field_label_behavior()
    }
    fn with_field(mut self, binding: FieldControlBinding) -> Self {
        self.primitive = self.primitive.with_field(binding);
        self
    }
}
impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ComponentTheme::resolve(self.tokens.clone(), cx);

        let compact = matches!(tokens.size, ControlSize::Compact);
        let m = metrics(tokens.size);
        let body = tokens.size.metrics(&tokens);
        let dark = ColorScheme::current(window, cx) == ColorScheme::Dark;
        let colors = tokens.extras(if dark {
            ThemeScheme::Dark
        } else {
            ThemeScheme::Light
        });
        let disabled = self.primitive.effective_disabled(window);
        let now = cx.background_executor().now();
        let reduce_motion = cx.reduce_motion();
        let focus = self.focus;
        let state = window.use_keyed_state(
            (self.id.clone(), "flow-switch-presentation"),
            cx,
            |_, cx| {
                Presentation::new(
                    focus.clone().unwrap_or_else(|| cx.focus_handle()),
                    self.checked,
                    disabled,
                    tokens.clone(),
                    colors,
                    now,
                )
            },
        );
        let thumb_motion = self.thumb_motion.unwrap_or(tokens.motion.moderate);
        let (focus, g, track_color, label_color, settled) = state.update(cx, |state, _| {
            if let Some(focus) = focus {
                state.focus = focus;
            }
            state.disabled = disabled;
            let hovered = state.hovered && !disabled;
            let target = geometry(self.checked, hovered, state.pressed && !disabled, m);
            if reduce_motion {
                if !state.dragging {
                    state.x.complete(target.x, now);
                }
                state.y.complete(target.y, now);
                state.width.complete(target.width, now);
                state.height.complete(target.height, now);
            }
            let (x, x_settled) = if state.dragging {
                (state.x.state.position, true)
            } else {
                state.x.sample(target.x, thumb_motion, now)
            };
            let (y, y_settled) = state.y.sample(target.y, thumb_motion, now);
            let (width, w_settled) = state.width.sample(target.width, thumb_motion, now);
            let (height, h_settled) = state.height.sample(target.height, thumb_motion, now);
            let base = if self.checked {
                gpui::Hsla::from(colors.brand)
            } else {
                gpui::Hsla::from(colors.accent)
            };
            let track = if hovered {
                mix_oklab(
                    base,
                    if self.checked {
                        gpui::black()
                    } else {
                        gpui::Hsla::from(colors.overlay)
                    },
                    if self.checked { 0.08 } else { 0.1 },
                )
            } else {
                base
            };
            let (track, track_settled) = state.track.sample(
                track,
                tokens.motion.fast,
                now,
                tokens.motion.curves.surface.curve(),
            );
            let (label, label_settled) = state.label.sample(
                if self.checked {
                    gpui::Hsla::from(colors.foreground)
                } else {
                    gpui::Hsla::from(colors.muted_foreground)
                },
                tokens.motion.fast,
                now,
                tokens.motion.curves.surface.curve(),
            );
            (
                state.focus.clone(),
                Geometry {
                    x,
                    y,
                    width,
                    height,
                },
                track,
                label,
                x_settled && y_settled && w_settled && h_settled && track_settled && label_settled,
            )
        });
        if !settled {
            window.request_animation_frame();
        }
        let pointer = state.clone();
        let press = state.clone();
        let hover = state.clone();
        let sharp = matches!(tokens.shape, Shape::Sharp);
        let thumb = div()
            .absolute()
            .left_0()
            .top_0()
            .translate_x(px(g.x))
            .translate_y(px(g.y))
            .w(px(g.width))
            .h(px(g.height))
            .when(!sharp, |thumb| thumb.rounded_full())
            .bg(gpui::Hsla::from(colors.thumb))
            .shadow(vec![
                BoxShadow::new(px(0.), px(1.), gpui::black().opacity(0.1))
                    .blur_radius(px(2.))
                    .spread_radius(px(-1.)),
                BoxShadow::new(px(0.), px(1.), gpui::black().opacity(0.1)).blur_radius(px(3.)),
            ]);
        let track = div()
            .relative()
            .flex_none()
            .w(px(m.width))
            .h(px(m.height))
            .bg(track_color)
            .when(!sharp, |track| track.rounded_full())
            .when(
                focus.is_focused(window) && window.last_input_was_keyboard(),
                |track| {
                    track.shadow(vec![
                        BoxShadow::new(px(0.), px(0.), gpui::Hsla::from(colors.brand))
                            .spread_radius(px(3.)),
                        BoxShadow::new(px(0.), px(0.), gpui::Hsla::from(colors.background))
                            .spread_radius(px(2.)),
                    ])
                },
            )
            .child(thumb);
        let mut primitive = self
            .primitive
            .track_focus(&focus)
            .composite_opacity()
            .drag(SwitchDrag::new(
                px(g.x),
                px(2.)..=px(m.width - 2. - m.thumb - m.press_extend),
                px(2.),
            ))
            .on_drag_change(move |event, _, cx| {
                let now = cx.background_executor().now();
                pointer.update(cx, |state, cx| {
                    match event.phase {
                        SwitchDragPhase::Pressed => {
                            state.pressed = true;
                            state.dragging = false;
                            if event.pointer == SwitchPointerKind::Touch {
                                state.hovered = false;
                            }
                        }
                        SwitchDragPhase::Moved => {
                            state.dragging = true;
                            state.x.pointer(f32::from(event.position), now);
                        }
                        SwitchDragPhase::Released | SwitchDragPhase::Cancelled => {
                            if state.dragging {
                                state.x.release(now);
                            }
                            state.pressed = false;
                            state.dragging = false;
                        }
                    }
                    cx.notify();
                });
            })
            .on_press_change(move |facts, _, cx| {
                press.update(cx, |state, cx| {
                    if state.disabled != facts.disabled {
                        state.disabled = facts.disabled;
                        if facts.disabled {
                            state.hovered = false;
                        }
                        cx.notify();
                    }
                })
            })
            .relative()
            .when(!disabled, |control| control.cursor_pointer())
            .flex()
            .items_center()
            .min_w_0()
            .gap(px(body.gap))
            .px(px(body.horizontal_padding))
            .py(px(if compact { 4. } else { 8. }))
            .font_family(tokens.typography.family.clone())
            .font_variations(tokens.typography.normal.variations())
            .text_size(px(body.text_size))
            .line_height(px(body.line_height))
            .opacity(if disabled { 0.5 } else { 1. })
            .child(track);
        if let Some(content) = self.label_content {
            primitive = primitive.child(div().min_w_0().text_color(label_color).child(content));
        } else if let Some(label) = self.label {
            primitive = primitive.child(
                div()
                    .min_w_0()
                    .text_color(label_color)
                    .child(CapTrimmed::new(label.clone(), label)),
            );
        }
        primitive = primitive.child(
            div()
                .id((self.id, "flow-switch-hover"))
                .absolute()
                .inset_0()
                .hover_listener_mode(gpui::HoverListenerMode::InputModalityIndependent)
                .on_hover(move |inside, window, cx| {
                    hover.update(cx, |state, cx| {
                        let next = *inside && !window.last_input_was_touch() && !state.disabled;
                        if next != state.hovered {
                            state.hovered = next;
                            cx.notify();
                        }
                    })
                }),
        );
        primitive.style().refine(&self.style);
        primitive
    }
}
