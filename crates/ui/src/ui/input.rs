use crate::theme::Theme;
use crate::ui::icon::{Icon, IconName};
use crate::ui::utils::input_surface::{animated_appearance, motion_state};
use crate::ui::utils::presentation::{ComponentTheme, ControlSize, ThemeScheme, nested_radius};
use gpui::{
    App, ElementId, FocusHandle, IntoElement, ParentElement, Refineable, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, px,
};
use kinu_ui::{
    ColorScheme, Input as InputPrimitive, NumberField, NumberFieldChange, NumberFieldFormat,
    NumberFieldGroup, NumberFieldInput, NumberFieldStepper,
};

/// An editable single-line field. Native Input owns text, selection, IME and Field binding.
#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    primitive: InputPrimitive,
    tokens: Option<ComponentTheme>,
    style: StyleRefinement,
}
impl Input {
    pub fn new(id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            primitive: InputPrimitive::new(id, value),
            tokens: None,
            style: StyleRefinement::default(),
        }
    }
    pub fn tokens(mut self, tokens: ComponentTheme) -> Self {
        self.tokens = Some(tokens.clone());
        self
    }
    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.primitive = self.primitive.placeholder(text);
        self
    }
    pub fn accessibility_label(mut self, text: impl Into<SharedString>) -> Self {
        self.primitive = self.primitive.accessibility_label(text);
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
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.primitive = self.primitive.invalid(invalid);
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.primitive = self.primitive.track_focus(focus);
        self
    }
    pub fn on_change(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_change(handler);
        self
    }
    pub fn on_submit(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_submit(handler);
        self
    }
}
impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ComponentTheme::resolve(self.tokens.clone(), cx);
        let now = cx.background_executor().now();
        let motion = motion_state(self.id, window, cx);
        let mut primitive =
            self.primitive
                .appearance(animated_appearance(tokens.clone(), motion.clone(), now));
        primitive.style().refine(&self.style);
        let element: gpui::AnyElement = primitive.render(window, cx).into_any_element();
        element
    }
}
impl kinu_ui::FieldControl for Input {
    type Value = SharedString;
    fn field_value(&self) -> Self::Value {
        kinu_ui::FieldControl::field_value(&self.primitive)
    }
    fn field_filled(&self) -> bool {
        kinu_ui::FieldControl::field_filled(&self.primitive)
    }
    fn field_disabled(&self) -> bool {
        kinu_ui::FieldControl::field_disabled(&self.primitive)
    }
    fn field_requires_value(&self) -> bool {
        kinu_ui::FieldControl::field_requires_value(&self.primitive)
    }
    fn field_label_behavior(&self) -> kinu_ui::FieldLabelBehavior {
        kinu_ui::FieldControl::field_label_behavior(&self.primitive)
    }
    fn with_field(mut self, binding: kinu_ui::FieldControlBinding) -> Self {
        self.primitive = kinu_ui::FieldControl::with_field(self.primitive, binding);
        self
    }
}

#[derive(Clone, Copy)]
struct NumberLayout {
    button_size: f32,
    inset: f32,
    text_padding_end: f32,
    icon_size: f32,
}

fn number_layout(size: ControlSize) -> NumberLayout {
    match size {
        ControlSize::Default => NumberLayout {
            button_size: 28.,
            inset: 4.,
            text_padding_end: 68.,
            icon_size: 16.,
        },
        ControlSize::Compact => NumberLayout {
            button_size: 24.,
            inset: 2.,
            text_padding_end: 54.,
            icon_size: 14.,
        },
    }
}

fn number_stepper(
    label: &'static str,
    icon: IconName,
    right: f32,
    layout: NumberLayout,
    tokens: ComponentTheme,
    colors: &Theme,
) -> NumberFieldStepper {
    NumberFieldStepper::new(label)
        .absolute()
        .top(px(layout.inset))
        .right(px(right))
        .size(px(layout.button_size))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(nested_radius(
            tokens.shape.radii(&tokens).item,
            layout.inset,
            0.,
        )))
        .text_color(gpui::Hsla::from(colors.muted_foreground))
        .hover({
            let __theme_colors_foreground = colors.foreground;
            let __theme_colors_hover = colors.hover();
            move |style| {
                style
                    .bg(__theme_colors_hover)
                    .text_color(gpui::Hsla::from(__theme_colors_foreground))
            }
        })
        .transition(gpui::StyleTransition::colors(
            tokens.motion.animations.color.animation(),
        ))
        .child(Icon::new(icon).size(layout.icon_size).stroke_width(1.5))
}

/// A controlled numeric field. Native NumberField owns parsing, stepping and text services.
#[derive(IntoElement)]
pub struct NumberInput {
    id: ElementId,
    primitive: NumberField,
    placeholder: SharedString,
    tokens: Option<ComponentTheme>,
    input_style: StyleRefinement,
    style: StyleRefinement,
}

impl NumberInput {
    pub fn new(id: impl Into<ElementId>, value: Option<f64>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            primitive: NumberField::new(id, value),
            placeholder: "".into(),
            tokens: None,
            input_style: StyleRefinement::default(),
            style: StyleRefinement::default(),
        }
    }
    pub fn tokens(mut self, tokens: ComponentTheme) -> Self {
        self.tokens = Some(tokens.clone());
        self
    }
    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }
    pub fn accessibility_label(mut self, text: impl Into<SharedString>) -> Self {
        self.primitive = self.primitive.accessibility_label(text);
        self
    }
    pub fn min(mut self, value: f64) -> Self {
        self.primitive = self.primitive.min(value);
        self
    }
    pub fn max(mut self, value: f64) -> Self {
        self.primitive = self.primitive.max(value);
        self
    }
    pub fn step(mut self, value: f64) -> Self {
        self.primitive = self.primitive.step(value);
        self
    }
    pub fn format(mut self, format: NumberFieldFormat) -> Self {
        self.primitive = self.primitive.format(format);
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
        self.primitive = self.primitive.track_focus(focus);
        self
    }
    pub fn input_style(mut self, style: StyleRefinement) -> Self {
        self.input_style = style;
        self
    }
    pub fn on_change(
        mut self,
        handler: impl Fn(&NumberFieldChange, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_change(handler);
        self
    }
    pub fn on_value_committed(
        mut self,
        handler: impl Fn(&NumberFieldChange, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_value_committed(handler);
        self
    }
}

impl Styled for NumberInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NumberInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ComponentTheme::resolve(self.tokens.clone(), cx);

        let scheme = if ColorScheme::current(window, cx) == ColorScheme::Dark {
            ThemeScheme::Dark
        } else {
            ThemeScheme::Light
        };
        let colors = tokens.extras(scheme);
        let layout = number_layout(tokens.size);
        let now = cx.background_executor().now();
        let motion = motion_state(self.id, window, cx);
        let appearance = animated_appearance(tokens.clone(), motion.clone(), now);
        let mut input = NumberFieldInput::new()
            .placeholder(self.placeholder)
            .appearance(move |state, window, cx| {
                let mut appearance = appearance(state, window, cx);
                // The group owns disabled opacity so the overlaid steppers dim once with the field.
                appearance.style = appearance.style.opacity(1.).pr(px(layout.text_padding_end));
                appearance
            });
        input.style().refine(&self.input_style);
        let mut primitive = self
            .primitive
            .input(input)
            .group(
                NumberFieldGroup::new()
                    .relative()
                    .w_full()
                    .min_w(px(0.))
                    .when_disabled(|style| style.opacity(0.5)),
            )
            .decrement(number_stepper(
                "Decrease",
                IconName::Minus,
                layout.inset + layout.button_size,
                layout,
                tokens.clone(),
                colors,
            ))
            .increment(number_stepper(
                "Increase",
                IconName::Plus,
                layout.inset,
                layout,
                tokens.clone(),
                colors,
            ))
            .w_full()
            .min_w(px(0.));
        primitive.style().refine(&self.style);
        let element: gpui::AnyElement = primitive.render(window, cx).into_any_element();
        element
    }
}

impl kinu_ui::FieldControl for NumberInput {
    type Value = Option<f64>;
    fn field_value(&self) -> Self::Value {
        kinu_ui::FieldControl::field_value(&self.primitive)
    }
    fn field_filled(&self) -> bool {
        kinu_ui::FieldControl::field_filled(&self.primitive)
    }
    fn field_disabled(&self) -> bool {
        kinu_ui::FieldControl::field_disabled(&self.primitive)
    }
    fn field_requires_value(&self) -> bool {
        kinu_ui::FieldControl::field_requires_value(&self.primitive)
    }
    fn field_label_behavior(&self) -> kinu_ui::FieldLabelBehavior {
        kinu_ui::FieldControl::field_label_behavior(&self.primitive)
    }
    fn with_field(mut self, binding: kinu_ui::FieldControlBinding) -> Self {
        self.primitive = kinu_ui::FieldControl::with_field(self.primitive, binding);
        self
    }
}
