use crate::ui::utils::input_surface::{animated_appearance, motion_state};
use crate::ui::utils::presentation::{ComponentTheme, ControlSize, Shape};
use gpui::{
    App, ElementId, FocusHandle, IntoElement, Refineable, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, px,
};
use kinu_ui::{Input as InputPrimitive, TextInputAppearance};

fn multiline_appearance(
    tokens: ComponentTheme,
    mut appearance: TextInputAppearance,
) -> TextInputAppearance {
    let (minimum, maximum, padding) = match tokens.size {
        ControlSize::Default => (76., 216., 7.),
        ControlSize::Compact => (64., 190., 4.),
    };
    appearance.style.size.height = Some(gpui::Length::Auto);
    appearance.style = appearance
        .style
        .min_h(px(minimum))
        .max_h(px(maximum))
        .py(px(padding))
        .rounded(px(match tokens.shape {
            Shape::Pill => tokens.shape.radii(&tokens).merged,
            _ => tokens.shape.radii(&tokens).item,
        }));
    appearance.resize = Some(kinu_ui::InputResize::None);
    appearance
}

/// An editable multiline field. Native Input owns text, selection, IME and Field binding.
#[derive(IntoElement)]
pub struct Textarea {
    id: ElementId,
    primitive: InputPrimitive,
    tokens: Option<ComponentTheme>,
    style: StyleRefinement,
}
impl Textarea {
    pub fn new(id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            primitive: InputPrimitive::new(id, value)
                .mode(kinu_ui::InputMode::Multiline)
                .preferred_rows(3)
                .resize(kinu_ui::InputResize::None),
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
impl Styled for Textarea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for Textarea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ComponentTheme::resolve(self.tokens.clone(), cx);
        let now = cx.background_executor().now();
        let motion = motion_state(self.id, window, cx);

        let appearance = animated_appearance(tokens.clone(), motion, now);
        let mut primitive = self.primitive.appearance({
            let tokens = tokens.clone();
            move |state, window, cx| {
                let appearance = appearance(state, window, cx);
                multiline_appearance(tokens.clone(), appearance)
            }
        });
        primitive.style().refine(&self.style);
        let element: gpui::AnyElement = primitive.render(window, cx).into_any_element();
        element
    }
}
impl kinu_ui::FieldControl for Textarea {
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
