use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, prelude::*};
use gpui_base::{StateStyle, Toggle, ToggleGroup as ToggleGroupBehavior};

use super::TextStyleExt as _;
use crate::theme::{colors::G2VTheme, radius::RADIUS_FULL, sizing::SIZE_7, typography};

type ValueChangeHandler = std::rc::Rc<dyn Fn(SharedString, &mut Window, &mut App)>;

#[derive(Clone)]
pub struct ToggleGroupItem {
    pub value: SharedString,
    pub label: SharedString,
}

impl ToggleGroupItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct ToggleGroup {
    id: SharedString,
    value: SharedString,
    label: SharedString,
    items: Vec<ToggleGroupItem>,
    on_value_change: Option<ValueChangeHandler>,
}

impl ToggleGroup {
    pub fn new(
        id: impl Into<SharedString>,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            label: label.into(),
            items: Vec::new(),
            on_value_change: None,
        }
    }
    pub fn item(mut self, item: ToggleGroupItem) -> Self {
        self.items.push(item);
        self
    }
    pub fn on_value_change(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_value_change = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let selected = self.value;
        let handler = self.on_value_change;
        let group_id = self.id.clone();
        ToggleGroupBehavior::new(self.id)
            .aria_label(self.label)
            .flex()
            .items_center()
            .gap_1()
            .children(self.items.into_iter().map(move |item| {
                let pressed = item.value == selected;
                let value = item.value.clone();
                let handler = handler.clone();
                Toggle::new((ElementId::from(group_id.clone()), item.value.clone()))
                    .pressed(pressed)
                    .h(SIZE_7)
                    .px_3()
                    .rounded(RADIUS_FULL)
                    .text_style(typography::tokens().sm)
                    .text_color(theme.muted_foreground)
                    .styles(|styles| {
                        styles.pressed(|style: StateStyle| {
                            style.bg(theme.muted).text_color(theme.foreground)
                        })
                    })
                    .child(item.label)
                    .on_change(move |next, _, window, cx| {
                        if next && let Some(handler) = &handler {
                            handler(value.clone(), window, cx);
                        }
                    })
            }))
    }
}
