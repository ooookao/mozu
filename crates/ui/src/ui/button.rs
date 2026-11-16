use crate::theme::Theme;
use crate::ui::utils::motion::{TimedValue, mix_oklab};
use crate::ui::utils::presentation::{ComponentTheme, ControlSize, ThemeScheme};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, BoxShadow, ClickEvent, ElementId, FocusHandle, Hsla, IntoElement,
    ParentElement, Refineable, RenderOnce, SharedString, StyleRefinement, Styled, Window, div, px,
};
use kinu_ui::{Button as ButtonPrimitive, ButtonType, ColorScheme};
use std::time::{Duration, Instant};

/// The current frame's position within a visual button group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonGroupPosition {
    pub first: bool,
    pub last: bool,
    pub vertical: bool,
}

fn group_shape(radius: gpui::Pixels, position: Option<ButtonGroupPosition>) -> StyleRefinement {
    match position {
        None => StyleRefinement::default().rounded(radius),
        Some(position) => StyleRefinement::default()
            .rounded_tl(if position.first { radius } else { px(0.) })
            .rounded_tr(
                if if position.vertical {
                    position.first
                } else {
                    position.last
                } {
                    radius
                } else {
                    px(0.)
                },
            )
            .rounded_bl(
                if if position.vertical {
                    position.last
                } else {
                    position.first
                } {
                    radius
                } else {
                    px(0.)
                },
            )
            .rounded_br(if position.last { radius } else { px(0.) }),
    }
}

/// Flow's five button treatments, independent of activation behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Tertiary,
    Ghost,
    Destructive,
    /// Kinu compatibility name for Flow's Primary treatment.
    Default,
    /// Kinu compatibility name for Flow's Tertiary treatment.
    Outline,
    /// Kinu compatibility name for Flow's Ghost treatment.
    Link,
}

/// Flow's two size steps plus Kinu compatibility names for installed callers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    #[default]
    Default,
    Compact,
    Icon,
    IconCompact,
    /// Resolves to Flow's Compact size.
    Xs,
    /// Resolves to Flow's Compact size.
    Sm,
    /// Resolves to Flow's Default size.
    Lg,
    /// Resolves to Flow's compact icon size.
    IconXs,
    /// Resolves to Flow's compact icon size.
    IconSm,
    /// Resolves to Flow's default icon size.
    IconLg,
}

impl From<ControlSize> for ButtonSize {
    fn from(size: ControlSize) -> Self {
        match size {
            ControlSize::Default => Self::Default,
            ControlSize::Compact => Self::Compact,
        }
    }
}

impl ButtonSize {
    fn resolved(self) -> (ControlSize, bool) {
        match self {
            Self::Default | Self::Lg => (ControlSize::Default, false),
            Self::Compact | Self::Xs | Self::Sm => (ControlSize::Compact, false),
            Self::Icon | Self::IconLg => (ControlSize::Default, true),
            Self::IconCompact | Self::IconXs | Self::IconSm => (ControlSize::Compact, true),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSpacing {
    InlineStart,
    InlineEnd,
}

/// Static component presentation for consumers that retain their own element semantics.
/// Animated button surfaces are supplied by `Button::into_primitive`.
pub struct ButtonPresentation {
    pub base: StyleRefinement,
    pub hover: StyleRefinement,
    pub focus: StyleRefinement,
    pub active: StyleRefinement,
    pub icon_size: f32,
}

pub fn button_variants(
    variant: ButtonVariant,
    size: ButtonSize,
    window: &Window,
    cx: &App,
) -> ButtonPresentation {
    let tokens = ComponentTheme::current(cx);
    let scheme = if ColorScheme::current(window, cx) == ColorScheme::Dark {
        ThemeScheme::Dark
    } else {
        ThemeScheme::Light
    };
    let colors = tokens.extras(scheme);
    let (size, icon_only) = size.resolved();
    let metrics = size.metrics(&tokens);
    let compact = matches!(size, ControlSize::Compact);
    let rest = surface(variant, colors, scheme, false, false, false);
    let hover = surface(variant, colors, scheme, true, false, false);
    let active = surface(variant, colors, scheme, false, true, false);
    let mut base = base_style(tokens.clone(), size, rest.foreground).bg(rest.fill);
    base = if icon_only {
        base.w(px(metrics.control_height)).p(px(0.))
    } else {
        base.px(px(if compact { 12. } else { 16. }))
            .gap(px(if compact { 4. } else { 6. }))
    };
    if rest.inset {
        base = base.border_1().border_color(rest.ring).bg_clip_padding();
    }
    ButtonPresentation {
        base,
        hover: StyleRefinement::default()
            .bg(hover.fill)
            .text_color(hover.foreground),
        focus: StyleRefinement::default()
            .outline_width(px(1.))
            .outline_offset(px(0.))
            .outline_color(gpui::Hsla::from(colors.brand)),
        active: StyleRefinement::default()
            .bg(active.fill)
            .text_color(active.foreground),
        icon_size: if compact { 14. } else { 16. },
    }
}

fn base_style(tokens: ComponentTheme, size: ControlSize, foreground: Hsla) -> StyleRefinement {
    let metrics = size.metrics(&tokens);
    StyleRefinement::default()
        .flex()
        .flex_none()
        .relative()
        .items_center()
        .justify_center()
        .h(px(metrics.control_height))
        .rounded(px(tokens.shape.radii(&tokens).item))
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.normal.weight)
        .font_variations(tokens.typography.normal.variations())
        .text_size(px(metrics.text_size))
        .line_height(px(metrics.line_height))
        .text_color(foreground)
        .whitespace_nowrap()
        .cursor_pointer()
}

/// Size and animated stroke weight supplied to an application-owned icon slot.
#[derive(Clone, Copy)]
pub struct ButtonIconStyle {
    pub size: gpui::Pixels,
    pub stroke_width: f32,
}

type IconSlot = Box<dyn FnOnce(ButtonIconStyle) -> AnyElement>;

fn icon_slot(icon: IconSlot, style: ButtonIconStyle) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .size(style.size)
        .child(icon(style))
}

/// Source-owned component presentation; the primitive owns activation, focus and accessibility.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    primitive: ButtonPrimitive,
    label: Option<SharedString>,
    tooltip: Option<SharedString>,
    tokens: Option<ComponentTheme>,
    variant: ButtonVariant,
    size: Option<ButtonSize>,
    disabled: bool,
    loading: bool,
    active: bool,
    icon_only: bool,
    icon_spacing: Option<IconSpacing>,
    leading_icon: Option<IconSlot>,
    trailing_icon: Option<IconSlot>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            primitive: ButtonPrimitive::new(id.clone()),
            id,
            label: None,
            tooltip: None,
            tokens: None,
            variant: ButtonVariant::Primary,
            size: None,
            disabled: false,
            loading: false,
            active: false,
            icon_only: false,
            icon_spacing: None,
            leading_icon: None,
            trailing_icon: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Sets visible text and the accessible name from the same application-owned value.
    /// Shows a Kinu tooltip while preserving the native button's input and focus ownership.
    pub fn tooltip(mut self, description: impl Into<SharedString>) -> Self {
        self.tooltip = Some(description.into());
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.primitive = self.primitive.accessibility_label(label.clone());
        self.label = Some(label);
        self
    }
    pub fn focusable_when_disabled(mut self, focusable: bool) -> Self {
        self.primitive = self.primitive.focusable_when_disabled(focusable);
        self
    }
    pub fn icon_start(self, icon: impl IntoElement + 'static) -> Self {
        self.leading_icon(move |style| {
            gpui::ElementContextScope::new(kinu_ui::IconSize(f32::from(style.size)), icon)
                .into_any_element()
        })
    }
    pub fn icon_end(self, icon: impl IntoElement + 'static) -> Self {
        self.trailing_icon(move |style| {
            gpui::ElementContextScope::new(kinu_ui::IconSize(f32::from(style.size)), icon)
                .into_any_element()
        })
    }
    pub fn icon_spacing(mut self, side: IconSpacing) -> Self {
        self.icon_spacing = Some(side);
        self
    }
    pub fn tokens(mut self, tokens: ComponentTheme) -> Self {
        self.tokens = Some(tokens.clone());
        self
    }
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn size(mut self, size: impl Into<ButtonSize>) -> Self {
        self.size = Some(size.into());
        self
    }
    pub fn icon_only(mut self, icon_only: bool) -> Self {
        self.icon_only = icon_only;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }
    /// Holds the engaged colors without forcing the physical press inset.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
    /// Exposes validity without inventing an invalid treatment absent from Flow's button.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.primitive = self.primitive.invalid(invalid);
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.primitive = self.primitive.accessibility_label(label);
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.primitive = self.primitive.track_focus(focus);
        self
    }
    pub fn button_type(mut self, button_type: ButtonType) -> Self {
        self.primitive = self.primitive.button_type(button_type);
        self
    }
    pub fn has_popup(mut self, popup: Option<gpui::accesskit::HasPopup>) -> Self {
        self.primitive = self.primitive.has_popup(popup);
        self
    }
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.primitive = self.primitive.expanded(expanded);
        self
    }
    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_click(listener);
        self
    }
    pub fn leading_icon(
        mut self,
        icon: impl FnOnce(ButtonIconStyle) -> AnyElement + 'static,
    ) -> Self {
        self.leading_icon = Some(Box::new(icon));
        self
    }
    pub fn trailing_icon(
        mut self,
        icon: impl FnOnce(ButtonIconStyle) -> AnyElement + 'static,
    ) -> Self {
        self.trailing_icon = Some(Box::new(icon));
        self
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

struct PresentationState {
    hovered: bool,
    pressed: bool,
    disabled: bool,
    spread: TimedValue,
    stroke: TimedValue,
    spinner_started: Instant,
    surface: Option<SurfaceTween>,
}

impl PresentationState {
    fn new(now: Instant) -> Self {
        Self {
            hovered: false,
            pressed: false,
            disabled: false,
            spread: TimedValue::new(1., now),
            stroke: TimedValue::new(1.5, now),
            spinner_started: now,
            surface: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Surface {
    fill: Hsla,
    foreground: Hsla,
    ring: Hsla,
    inset: bool,
}

#[derive(Clone, Copy)]
struct SurfaceTween {
    from: Surface,
    target: Surface,
    started: Instant,
    duration: Duration,
}

impl SurfaceTween {
    fn sample(self, now: Instant, easing: impl Fn(f32) -> f32) -> (Surface, bool) {
        let progress = if self.duration.is_zero() {
            1.
        } else {
            (now.duration_since(self.started).as_secs_f32() / self.duration.as_secs_f32()).min(1.)
        };
        let share = easing(progress) as f64;
        (
            Surface {
                fill: mix_oklab(self.from.fill, self.target.fill, share),
                foreground: mix_oklab(self.from.foreground, self.target.foreground, share),
                ring: mix_oklab(self.from.ring, self.target.ring, share),
                inset: self.target.inset,
            },
            progress >= 1.,
        )
    }
}

fn surface(
    variant: ButtonVariant,
    colors: &Theme,
    scheme: ThemeScheme,
    hovered: bool,
    held: bool,
    active: bool,
) -> Surface {
    let transparent = gpui::transparent_black();
    let engaged = held || active;
    match variant {
        ButtonVariant::Primary | ButtonVariant::Default => Surface {
            fill: mix_oklab(
                gpui::Hsla::from(colors.foreground),
                gpui::Hsla::from(colors.background),
                if engaged {
                    0.2
                } else if hovered {
                    0.1
                } else {
                    0.
                },
            ),
            foreground: gpui::Hsla::from(colors.background),
            ring: transparent,
            inset: false,
        },
        ButtonVariant::Secondary => Surface {
            fill: if held || active {
                colors.tint()
            } else if hovered {
                colors.tint_hover()
            } else {
                colors.tint()
            },
            foreground: gpui::Hsla::from(colors.foreground),
            ring: transparent,
            inset: false,
        },
        ButtonVariant::Tertiary | ButtonVariant::Outline => Surface {
            fill: if engaged {
                colors.active()
            } else if hovered {
                colors.hover()
            } else {
                transparent
            },
            foreground: gpui::Hsla::from(colors.foreground),
            ring: gpui::Hsla::from(colors.border),
            inset: true,
        },
        ButtonVariant::Ghost | ButtonVariant::Link => Surface {
            fill: if engaged {
                colors.active()
            } else if hovered {
                colors.hover()
            } else {
                transparent
            },
            foreground: if hovered {
                gpui::Hsla::from(colors.foreground)
            } else {
                gpui::Hsla::from(colors.muted_foreground)
            },
            ring: transparent,
            inset: false,
        },
        ButtonVariant::Destructive => {
            let dark = matches!(scheme, ThemeScheme::Dark);
            let alpha = if active || (hovered && !held) {
                if dark { 0.3 } else { 0.25 }
            } else if dark {
                0.2
            } else {
                0.15
            };
            Surface {
                fill: gpui::Hsla::from(colors.destructive_fill).opacity(alpha),
                foreground: gpui::Hsla::from(colors.destructive_foreground),
                ring: transparent,
                inset: false,
            }
        }
    }
}

fn surface_shadows(surface: Surface, spread: f32) -> Vec<BoxShadow> {
    if surface.inset {
        vec![
            BoxShadow::new(px(0.), px(0.), surface.ring).spread_radius(px(spread)),
            BoxShadow::new(px(0.), px(0.), surface.ring)
                .spread_radius(px(1. - spread))
                .inset(),
        ]
    } else {
        vec![BoxShadow::new(px(0.), px(0.), surface.fill).spread_radius(px(spread))]
    }
}

impl Button {
    /// Retains Flow surfaces while exposing the primitive for semantic composition.
    pub fn into_primitive(mut self, window: &mut Window, cx: &mut App) -> ButtonPrimitive {
        let tokens = ComponentTheme::resolve(self.tokens.take(), cx);
        let now = cx.background_executor().now();
        let state = window.use_keyed_state((self.id.clone(), "flow-presentation"), cx, |_, _| {
            PresentationState::new(now)
        });
        let scheme = if ColorScheme::current(window, cx) == ColorScheme::Dark {
            ThemeScheme::Dark
        } else {
            ThemeScheme::Light
        };
        let colors = tokens.extras(scheme);
        let (size, sized_icon) = self.size.unwrap_or_else(|| tokens.size.into()).resolved();
        self.icon_only |= sized_icon;
        let compact = matches!(size, ControlSize::Compact);
        let metrics = size.metrics(&tokens);
        let explicit_disabled = self.disabled || self.loading;
        let reduce_motion = cx.reduce_motion();
        let (hovered, held, spread, stroke, running, elapsed) = state.update(cx, |state, _| {
            let unavailable = explicit_disabled || state.disabled;
            let hovered = state.hovered && !unavailable;
            let held = state.pressed && !unavailable;
            let press = if held {
                tokens.motion.animations.press_enter
            } else {
                tokens.motion.animations.press_exit
            };
            let press_easing = press.curve();
            state.spread.retarget(
                if held { 0. } else { 1. },
                now,
                Duration::from_millis(press.duration_ms),
                press_easing,
            );
            state.stroke.retarget(
                if hovered { 2. } else { 1.5 },
                now,
                tokens.motion.fast.enter,
                tokens.motion.curves.surface.curve(),
            );
            let (spread, spread_done) = state.spread.sample(now, press.curve());
            let (stroke, stroke_done) = state
                .stroke
                .sample(now, tokens.motion.curves.surface.curve());
            (
                hovered,
                held,
                spread,
                stroke,
                !spread_done || !stroke_done,
                now.duration_since(state.spinner_started),
            )
        });
        let target_surface = surface(self.variant, colors, scheme, hovered, held, self.active);
        let (surface, colors_done) = state.update(cx, |state, _| {
            let transition = state.surface.get_or_insert(SurfaceTween {
                from: target_surface,
                target: target_surface,
                started: now,
                duration: Duration::ZERO,
            });
            if transition.target != target_surface {
                transition.from = transition
                    .sample(now, tokens.motion.curves.surface.curve())
                    .0;
                transition.target = target_surface;
                transition.started = now;
                transition.duration = tokens.motion.fast.enter;
            }
            transition.sample(now, tokens.motion.curves.surface.curve())
        });
        if running || !colors_done || self.loading {
            window.request_animation_frame();
        }
        let radius = tokens.shape.radii(&tokens).item;
        let group_position = window.element_context::<ButtonGroupPosition>().copied();
        let shape = group_shape(px(radius), group_position);
        let mut background = div()
            .id("surface")
            .absolute()
            .inset(px(1.))
            .rounded(px(radius))
            .bg(surface.fill)
            .shadow(surface_shadows(surface, spread));
        if surface.inset
            && let Some(position) = group_position
        {
            background = background
                .inset(px(1. - spread))
                .shadow(Vec::<BoxShadow>::new())
                .border_1()
                .border_color(surface.ring);
            if !position.first {
                background = if position.vertical {
                    background.border_t(px(0.))
                } else {
                    background.border_l(px(0.))
                };
            }
        }
        background.style().refine(&shape);
        let icon_style = ButtonIconStyle {
            size: px(if compact { 14. } else { 16. }),
            stroke_width: stroke,
        };
        let mut content = div()
            .relative()
            .min_w_0()
            .max_w_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(if compact { 4. } else { 6. }));
        let has_leading = self
            .icon_spacing
            .map_or(self.leading_icon.is_some(), |side| {
                side == IconSpacing::InlineStart
            });
        let has_trailing = self
            .icon_spacing
            .map_or(self.trailing_icon.is_some(), |side| {
                side == IconSpacing::InlineEnd
            });
        if let Some(icon) = self.leading_icon.take() {
            content = content.child(icon_slot(icon, icon_style));
        }
        if !self.icon_only {
            content = content.when_some(self.label, |content, label| {
                content.child(div().min_w_0().truncate().child(label))
            });
        }
        content = content.children(self.children);
        if let Some(icon) = self.trailing_icon.take() {
            content = content.child(icon_slot(icon, icon_style));
        }
        let contents = div()
            .relative()
            .min_w_0()
            .max_w_full()
            .child(content.opacity(if self.loading { 0. } else { 1. }))
            .when(self.loading, |content| {
                content.child(loading_spinner(
                    icon_style.size,
                    surface.foreground,
                    elapsed,
                    reduce_motion,
                    &tokens,
                ))
            });
        let press_state = state.clone();
        let mut primitive = self
            .primitive
            .disabled(explicit_disabled)
            .on_press_change(move |pressed, _, cx| {
                press_state.update(cx, |state, cx| {
                    if state.pressed != pressed.pressed || state.disabled != pressed.disabled {
                        state.pressed = pressed.pressed;
                        state.disabled = pressed.disabled;
                        cx.notify();
                    }
                });
            })
            .busy(self.loading)
            .disabled_style(move |style| style.opacity(if self.loading { 1. } else { 0.5 }))
            .composite_opacity()
            .focus_visible({
                let __theme_colors_brand = colors.brand;
                move |style| {
                    style
                        .outline_width(px(1.))
                        .outline_offset(px(0.))
                        .outline_color(gpui::Hsla::from(__theme_colors_brand))
                }
            })
            .child(background)
            .child(contents);
        primitive
            .style()
            .refine(&base_style(tokens.clone(), size, surface.foreground));
        primitive.style().refine(&shape);
        primitive = if self.icon_only {
            primitive.w(px(metrics.control_height)).p(px(0.))
        } else {
            primitive
                .pl(px(if compact {
                    if has_leading { 8. } else { 12. }
                } else if has_leading {
                    12.
                } else {
                    16.
                }))
                .pr(px(if compact {
                    if has_trailing { 8. } else { 12. }
                } else if has_trailing {
                    12.
                } else {
                    16.
                }))
        };
        primitive.style().refine(&self.style);
        let hover_state = state.clone();
        let observer = div()
            .id((self.id, "flow-button-presentation"))
            .absolute()
            .inset_0()
            .on_hover(move |hovered, _, cx| {
                hover_state.update(cx, |state, cx| {
                    if state.hovered != *hovered {
                        state.hovered = *hovered;
                        cx.notify();
                    }
                });
            });
        primitive.child(observer)
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let tooltip = self.tooltip.clone();
        let primitive = self.into_primitive(window, cx);
        match tooltip {
            Some(description) => {
                let state = window.use_keyed_state((id.clone(), "tooltip-open"), cx, |_, _| false);
                let open = *state.read(cx);
                crate::ui::tooltip::Tooltip::new((id, "tooltip"), open, primitive, description)
                    .on_open_change(move |open, _, _, cx| {
                        state.update(cx, |value, cx| {
                            *value = open;
                            cx.notify();
                        })
                    })
                    .into_any_element()
            }
            None => primitive.into_any_element(),
        }
    }
}

fn loading_spinner(
    size: gpui::Pixels,
    color: Hsla,
    elapsed: Duration,
    reduce_motion: bool,
    tokens: &ComponentTheme,
) -> impl IntoElement {
    let profile = if reduce_motion {
        tokens.motion.animations.loading_spinner_reduced
    } else {
        tokens.motion.animations.loading_spinner
    };
    let seconds = Duration::from_millis(profile.duration_ms).as_secs_f32();
    let phase = elapsed.as_secs_f32() % seconds / seconds;
    let easing = profile.curve();
    let rotation = easing(phase) * std::f32::consts::TAU;
    let growth = easing(if phase < 0.5 {
        phase * 2.
    } else {
        (1. - phase) * 2.
    });
    let dash = 0.01 + growth * 25.99;
    let offset = 25. - growth * 13.;
    div()
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            gpui::canvas(
                |_, _, _| (),
                move |bounds, (), window, _| {
                    let scale = f32::from(size) / 16.;
                    let radius = 6.25 * scale;
                    for opposite in [0., 0.5] {
                        let start = (offset / 100. + opposite) * std::f32::consts::TAU
                            - std::f32::consts::FRAC_PI_2
                            + rotation;
                        let end = start + dash / 100. * std::f32::consts::TAU;
                        let center = bounds.center();
                        let point_at = |angle: f32| {
                            center + gpui::point(px(radius * angle.cos()), px(radius * angle.sin()))
                        };
                        let mut path = gpui::PathBuilder::stroke(px(1.5 * scale)).with_style(
                            gpui::PathStyle::Stroke(
                                gpui::StrokeOptions::default()
                                    .with_line_width(1.5 * scale)
                                    .with_line_cap(gpui::LineCap::Round),
                            ),
                        );
                        path.move_to(point_at(start));
                        path.arc_to(
                            gpui::point(px(radius), px(radius)),
                            px(0.),
                            false,
                            true,
                            point_at(end),
                        );
                        window.paint_path(
                            path.build()
                                .expect("Finite spinner arc geometry must tessellate"),
                            color,
                        );
                    }
                },
            )
            .size(size),
        )
}
