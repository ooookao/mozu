use crate::ui::field::{
    FieldLayout, field_error_with_tokens as field_error, field_label_with_tokens as field_label,
    field_with_tokens as field,
};
use crate::ui::icon::{Icon, IconName};
use crate::ui::utils::hover::{HoverAxis, HoverItem, HoverMotion, HoverTarget};
use crate::ui::utils::motion::AnimatedArtwork;
use crate::ui::utils::presentation::{ComponentTheme, ControlSize, ThemeScheme};
use gpui::prelude::*;
use gpui::{
    App, Bounds, ElementId, FocusHandle, IntoElement, ParentElement, Refineable, RenderOnce,
    SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use kinu_ui::{
    ColorScheme, FieldErrorMatch, InputGroup as NativeInputGroup, InputGroupAddonKind,
    InputGroupAppearance, InputGroupControlAppearance,
};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

struct MeasuredField {
    id: ElementId,
    bounds: Rc<Cell<Bounds<gpui::Pixels>>>,
    disabled: Rc<Cell<bool>>,
}
struct GroupVisual {
    hover: HoverMotion,
    fields: Vec<MeasuredField>,
}

/// A controlled text field presented inside a shared visual hover group.
/// Native Field and InputGroup retain validation, text services and focus ownership.
pub struct InputField {
    id: ElementId,
    label: SharedString,
    primitive: NativeInputGroup,
    icon: Option<IconName>,
    error: Option<SharedString>,
    label_hidden: bool,
    disabled: bool,
}
impl InputField {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        let id = id.into();
        let label = label.into();
        Self {
            primitive: NativeInputGroup::new((id.clone(), "input"), value)
                .accessibility_label(label.clone()),
            id,
            label,
            icon: None,
            error: None,
            label_hidden: false,
            disabled: false,
        }
    }
    pub fn placeholder(mut self, value: impl Into<SharedString>) -> Self {
        self.primitive = self.primitive.placeholder(value);
        self
    }
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn error(mut self, message: impl Into<SharedString>) -> Self {
        self.error = Some(message.into());
        self
    }
    pub fn label_hidden(mut self, hidden: bool) -> Self {
        self.label_hidden = hidden;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.primitive = self.primitive.disabled(disabled);
        self
    }
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.primitive = self.primitive.read_only(read_only);
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.primitive = self.primitive.track_focus(focus);
        self
    }
    pub fn on_change(
        mut self,
        change: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_change(change);
        self
    }
    pub fn on_submit(
        mut self,
        submit: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_submit(submit);
        self
    }
}

/// An explicit list of fields whose nearest-pointer highlight is presentation only.
#[derive(IntoElement)]
pub struct InputGroup {
    id: ElementId,
    fields: Vec<InputField>,
    tokens: Option<ComponentTheme>,
    style: StyleRefinement,
}
impl InputGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            fields: Vec::new(),
            tokens: None,
            style: StyleRefinement::default(),
        }
    }
    pub fn tokens(mut self, tokens: ComponentTheme) -> Self {
        self.tokens = Some(tokens.clone());
        self
    }
    pub fn field(mut self, field: InputField) -> Self {
        assert!(
            !self.fields.iter().any(|item| item.id == field.id),
            "InputGroup field IDs must be unique"
        );
        self.fields.push(field);
        self
    }
}
impl Styled for InputGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn appearance(
    tokens: ComponentTheme,
    scheme: ThemeScheme,
    active: bool,
    focused: bool,
    invalid: bool,
) -> InputGroupAppearance {
    let colors = tokens.extras(scheme);
    let metrics = tokens.size.metrics(&tokens);
    InputGroupAppearance {
        control: Rc::new({
            let tokens = tokens.clone();
            {
                let __theme_colors_border = colors.border;
                let __theme_colors_destructive = colors.destructive;
                let __theme_colors_foreground = colors.foreground;
                let __theme_colors_hover = colors.hover();
                move |state| {
                    let border = if focused && !state.disabled {
                        gpui::Hsla::from(__theme_colors_foreground).opacity(0.4)
                    } else if invalid {
                        gpui::Hsla::from(__theme_colors_destructive).opacity(0.5)
                    } else {
                        gpui::Hsla::from(__theme_colors_border)
                    };
                    let shadow = |color| {
                        vec![
                            gpui::BoxShadow::new(px(0.), px(0.), color)
                                .spread_radius(px(1.))
                                .inset(),
                        ]
                    };
                    let root = StyleRefinement::default()
                        .h(px(metrics.control_height))
                        .px(px(metrics.horizontal_padding))
                        .gap(px(metrics.gap))
                        .rounded(px(tokens.shape.radii(&tokens).item))
                        .shadow(shadow(border))
                        .bg(if active && !focused && !state.disabled {
                            __theme_colors_hover
                        } else {
                            gpui::transparent_black()
                        })
                        .font_family(tokens.typography.family.clone())
                        .font_weight(tokens.typography.normal.weight)
                        .font_variations(tokens.typography.normal.variations())
                        .text_size(px(metrics.text_size))
                        .line_height(px(metrics.line_height))
                        .text_color(gpui::Hsla::from(__theme_colors_foreground));
                    InputGroupControlAppearance {
                        root,
                        input: StyleRefinement::default()
                            .flex_1()
                            .min_w_0()
                            .h(px(metrics.line_height))
                            .p_0()
                            .border_0()
                            .rounded_none()
                            .bg(gpui::transparent_black())
                            .shadow(Vec::new()),
                        focus: StyleRefinement::default()
                            .bg(gpui::transparent_black())
                            .shadow(shadow(
                                gpui::Hsla::from(__theme_colors_foreground).opacity(0.4),
                            )),
                        transition: Some(gpui::StyleTransition::text_shadow_background(
                            tokens.motion.animations.color.animation(),
                        )),
                    }
                }
            }
        }),
        addon: Rc::new(move |_, _| StyleRefinement::default().p_0().mr(px(metrics.gap))),
        placeholder: Some(gpui::Hsla::from(colors.muted_foreground)),
    }
}

impl RenderOnce for InputGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ComponentTheme::resolve(self.tokens.clone(), cx);

        let scheme = if ColorScheme::current(window, cx) == ColorScheme::Dark {
            ThemeScheme::Dark
        } else {
            ThemeScheme::Light
        };
        let visual =
            window.use_keyed_state((self.id.clone(), "field-hover"), cx, |_, _| GroupVisual {
                hover: HoverMotion::new(),
                fields: Vec::new(),
            });
        visual.update(cx, |state, _| {
            let mut previous = std::mem::take(&mut state.fields);
            state.fields = self
                .fields
                .iter()
                .map(|field| {
                    if let Some(index) = previous.iter().position(|row| row.id == field.id) {
                        previous.remove(index)
                    } else {
                        MeasuredField {
                            id: field.id.clone(),
                            bounds: Rc::new(Cell::new(Bounds::default())),
                            disabled: Rc::new(Cell::new(field.disabled)),
                        }
                    }
                })
                .collect();
        });
        let active = visual.read(cx).hover.snapshot().active;
        let mut root = div()
            .id(self.id)
            .relative()
            .flex()
            .flex_col()
            .gap(px(12.))
            .w(px(288.))
            .max_w_full();
        for item in self.fields {
            let measured = visual
                .read(cx)
                .fields
                .iter()
                .find(|row| row.id == item.id)
                .expect("registered field");
            let bounds = measured.bounds.clone();
            let disabled = measured.disabled.clone();
            let active = active.as_ref() == Some(&item.id);
            let icon_id = item.id.clone();
            let icon = item.icon;
            let invalid = item
                .error
                .as_ref()
                .is_some_and(|message| !message.is_empty());
            let mut native = field(
                item.id.clone(),
                item.primitive,
                FieldLayout::Vertical,
                tokens.clone(),
                window,
                cx,
            )
            .disabled(item.disabled)
            .invalid(invalid)
            .state_style(|state| {
                StyleRefinement::default().opacity(if state.disabled { 0.5 } else { 1. })
            })
            .present_control({
                let tokens = tokens.clone();
                move |mut control, state, _| {
                    disabled.set(state.disabled);
                    control = control.appearance(appearance(
                        tokens.clone(),
                        scheme,
                        active,
                        state.focused,
                        state.invalid(),
                    ));
                    if let Some(icon) = icon {
                        control = control.start(
                            {
                                let emphasized = (active || state.focused) && !state.disabled;
                                let colors = tokens.extras(scheme);
                                let size = if tokens.size == ControlSize::Compact {
                                    14.
                                } else {
                                    16.
                                };
                                AnimatedArtwork::new(
                                    icon_id,
                                    if emphasized {
                                        gpui::Hsla::from(colors.foreground)
                                    } else {
                                        gpui::Hsla::from(colors.muted_foreground)
                                    },
                                    if emphasized { 2. } else { 1.5 },
                                    Duration::from_millis(
                                        tokens.motion.animations.color.duration_ms,
                                    ),
                                    tokens.motion.animations.color.easing,
                                    move |color, stroke| {
                                        Icon::new(icon)
                                            .size(size)
                                            .stroke_width(stroke)
                                            .text_color(color)
                                            .into_any_element()
                                    },
                                )
                            },
                            InputGroupAddonKind::Content,
                        );
                    }
                    control
                }
            });
            if !item.label_hidden {
                native = native.label(field_label(
                    item.label,
                    FieldLayout::Vertical,
                    tokens.clone(),
                    window,
                    cx,
                ));
            }
            if let Some(error) = item.error.filter(|message| !message.is_empty()) {
                native = native.error(
                    field_error(FieldErrorMatch::Always, tokens.clone(), window, cx).message(error),
                );
            }
            root = root.child(
                div()
                    .id((item.id, "measurement"))
                    .child(native)
                    .on_prepainted(move |rect, window, _| {
                        bounds.set(window.element_to_window(rect))
                    }),
            );
        }
        root.style().refine(&self.style);
        let pointer = visual.clone();
        root.on_mouse_move(move |event, _, cx| {
            pointer.update(cx, |state, cx| {
                let items = state
                    .fields
                    .iter()
                    .map(|row| HoverItem {
                        id: row.id.clone(),
                        bounds: row.bounds.get(),
                        enabled: !row.disabled.get(),
                    })
                    .collect::<Vec<_>>();
                if state.hover.update(
                    event.position,
                    &items,
                    HoverAxis::Vertical,
                    HoverTarget::Nearest,
                ) {
                    cx.notify();
                }
            });
        })
        .on_hover(move |hovered, _, cx| {
            if !hovered {
                visual.update(cx, |state, cx| {
                    if state.hover.clear() {
                        cx.notify();
                    }
                });
            }
        })
    }
}
