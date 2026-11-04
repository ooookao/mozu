use std::rc::Rc;

use gpui::{
    App, Axis, ClickEvent, ElementId, IntoElement, ParentElement as _, RenderOnce, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _,
};
use gpui_base::{Radio as RadioBehavior, RadioGroup as RadioGroupBehavior, StateStyle};

use crate::theme::{
    border::BORDER_DEFAULT,
    colors::G2VTheme,
    shadows::{RING_3, focus_ring},
    sizing::{SIZE_2, SIZE_4},
};

type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// Application-owned radio group composed from gpui-base semantics.
#[derive(IntoElement)]
#[must_use]
pub struct RadioGroup {
    behavior: RadioGroupBehavior,
}

impl RadioGroup {
    pub fn vertical(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::new(id, label, Axis::Vertical)
    }

    pub fn horizontal(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self::new(id, label, Axis::Horizontal)
    }

    fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, axis: Axis) -> Self {
        Self {
            behavior: RadioGroupBehavior::new(id).axis(axis).aria_label(label),
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.behavior = self.behavior.child(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.behavior = self.behavior.children(children);
        self
    }
}

impl Styled for RadioGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        self.behavior.style()
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.behavior
    }
}

/// Shadcn-style radio item backed by gpui-base behavior.
#[derive(IntoElement)]
#[must_use]
pub struct RadioGroupItem {
    id: ElementId,
    label: SharedString,
    checked: bool,
    disabled: bool,
    on_change: Option<ChangeHandler>,
}

impl RadioGroupItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked: false,
            disabled: false,
            on_change: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for RadioGroupItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let checked = self.checked;
        let disabled = self.disabled;
        let focus = window
            .use_keyed_state((self.id.clone(), "focus"), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let focus_visible = focus.is_focused(window) && window.last_input_was_keyboard();
        let ring = focus_ring(theme.ring.opacity(0.5), RING_3);

        let indicator = div()
            .flex()
            .items_center()
            .justify_center()
            .size(SIZE_4)
            .rounded_full()
            .border(BORDER_DEFAULT)
            .border_color(if checked { theme.primary } else { theme.input })
            .when(checked, |indicator| {
                indicator.bg(theme.primary).child(
                    div()
                        .size(SIZE_2)
                        .rounded_full()
                        .bg(theme.primary_foreground),
                )
            })
            .when(focus_visible, |indicator| {
                indicator.border_color(theme.ring).shadow(ring)
            });

        let mut radio = RadioBehavior::new(self.id)
            .checked(checked)
            .disabled(disabled)
            .track_focus(&focus)
            .accessibility_label(self.label)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(SIZE_4)
            .styles(|styles| styles.disabled(|style: StateStyle| style.opacity(0.5)))
            .child(indicator);
        if let Some(handler) = self.on_change {
            radio = radio.on_change(move |next, event, window, cx| {
                handler(next, event, window, cx);
            });
        }
        radio
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn indicator_geometry_matches_the_shared_radio_contract() {
        assert_eq!(SIZE_4.to_pixels(px(16.0)), px(16.0));
        assert_eq!(SIZE_2.to_pixels(px(16.0)), px(8.0));
        assert_eq!(BORDER_DEFAULT, px(0.5));
    }
}
