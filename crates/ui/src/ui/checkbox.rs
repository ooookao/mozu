use std::rc::Rc;

use gpui::{
    App, ClickEvent, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, SharedString, Styled as _, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_base::{Checkbox as BehaviorCheckbox, CheckboxIndicator, CheckboxState, StateStyle};

use super::{Icon, IconName};
use crate::theme::{
    border::BORDER_DEFAULT,
    colors::G2VTheme,
    shadows::{RING_3, focus_ring},
    sizing::{SIZE_3_5, SIZE_4},
};

const CHECKBOX_RADIUS: gpui::Pixels = px(4.0);

type ChangeHandler = Rc<dyn Fn(bool, &ClickEvent, &mut Window, &mut App)>;

/// Application-owned shadcn-style checkbox composed from gpui-base behavior.
#[derive(IntoElement)]
#[must_use]
pub struct Checkbox {
    id: ElementId,
    label: SharedString,
    checked: bool,
    disabled: bool,
    debug_selector: Option<SharedString>,
    on_change: Option<ChangeHandler>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked: false,
            disabled: false,
            debug_selector: None,
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

    pub fn debug_selector(mut self, selector: impl Into<SharedString>) -> Self {
        self.debug_selector = Some(selector.into());
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

impl RenderOnce for Checkbox {
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
        let selector = self.debug_selector.clone();

        let indicator = CheckboxIndicator::new()
            .checked(checked)
            .disabled(disabled)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(SIZE_4)
            .rounded(CHECKBOX_RADIUS)
            .border(BORDER_DEFAULT)
            .border_color(theme.input)
            .bg(theme.background.opacity(0.0))
            .styles(|styles| {
                styles
                    .checked(|style: StateStyle| {
                        style.border_color(theme.primary).bg(theme.primary)
                    })
                    .disabled(|style: StateStyle| style.opacity(0.5))
            })
            .when(focus_visible, |indicator| {
                indicator.border_color(theme.ring).shadow(ring)
            })
            .when(checked, |indicator| {
                indicator
                    .child(Icon::new(IconName::Check, SIZE_3_5).color(theme.primary_foreground))
            });
        let indicator = div()
            .debug_selector(move || {
                selector
                    .clone()
                    .unwrap_or_else(|| "checkbox-indicator".into())
                    .to_string()
            })
            .size(SIZE_4)
            .child(indicator);

        let mut control = BehaviorCheckbox::new(self.id)
            .checked(checked)
            .disabled(disabled)
            .track_focus(&focus)
            .accessibility_label(self.label.clone())
            .flex()
            .items_center()
            .gap_2()
            .styles(|styles| styles.disabled(|style| style.opacity(0.5).cursor_default()))
            .child(indicator)
            .child(div().child(self.label));
        if let Some(handler) = self.on_change {
            control = control.on_change(move |state, event, window, cx| {
                handler(state == CheckboxState::Checked, event, window, cx);
            });
        }
        div().child(control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_matches_the_shared_checkbox_contract() {
        assert_eq!(CHECKBOX_RADIUS, px(4.0));
        assert_eq!(SIZE_4.to_pixels(px(16.0)), px(16.0));
        assert_eq!(SIZE_3_5.to_pixels(px(16.0)), px(14.0));
    }
}
