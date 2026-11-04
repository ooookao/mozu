use gpui::{App, ElementId, IntoElement, RenderOnce, Window, div, prelude::*};
use gpui_base::{StateStyle, Switch as BehaviorSwitch, SwitchThumb, SwitchTrack};

use crate::theme::{colors::G2VTheme, radius::RADIUS_FULL};

type SwitchChangeHandler = std::rc::Rc<dyn Fn(bool, &gpui::ClickEvent, &mut Window, &mut App)>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SwitchSize {
    Small,
    #[default]
    Default,
}

impl SwitchSize {
    const fn metrics(self) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels, gpui::Pixels) {
        match self {
            Self::Small => (
                gpui::px(24.0),
                gpui::px(14.0),
                gpui::px(12.0),
                gpui::px(1.0),
            ),
            Self::Default => (
                gpui::px(32.0),
                gpui::px(18.4),
                gpui::px(16.0),
                gpui::px(1.2),
            ),
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Switch {
    id: ElementId,
    checked: bool,
    disabled: bool,
    size: SwitchSize,
    label: gpui::SharedString,
    on_change: Option<SwitchChangeHandler>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>, label: impl Into<gpui::SharedString>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            size: SwitchSize::Default,
            label: label.into(),
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
    pub fn size(mut self, size: SwitchSize) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(bool, &gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let checked = self.checked;
        let (width, height, thumb_size, padding) = self.size.metrics();
        let unchecked_background = if theme.is_dark {
            theme.input.opacity(0.8)
        } else {
            theme.input
        };
        let thumb_background = if theme.is_dark {
            if checked {
                theme.primary_foreground
            } else {
                theme.foreground
            }
        } else {
            theme.background_solid
        };
        let thumb_shadow = gpui_base::Theme::global(cx).tokens.shadow.sm.clone();
        let track = SwitchTrack::new((self.id.clone(), "track"))
            .checked(checked)
            .disabled(self.disabled)
            .flex()
            .flex_shrink_0()
            .items_center()
            .w(width)
            .h(height)
            .p(padding)
            .rounded(RADIUS_FULL)
            .bg(unchecked_background)
            .styles(|styles| {
                styles
                    .checked(|style: StateStyle| style.bg(theme.primary))
                    .disabled(|style: StateStyle| style.opacity(0.5))
            })
            .child(
                SwitchThumb::new(checked)
                    .disabled(self.disabled)
                    .size(thumb_size)
                    .rounded(RADIUS_FULL)
                    .bg(thumb_background)
                    .shadow(thumb_shadow)
                    .when(checked, gpui::Styled::ml_auto),
            );
        let mut control = BehaviorSwitch::new(self.id)
            .checked(checked)
            .disabled(self.disabled)
            .accessibility_label(self.label)
            .child(track);
        if let Some(handler) = self.on_change {
            control = control
                .on_change(move |value, event, window, cx| handler(value, event, window, cx));
        }
        div().child(control)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_match_the_shared_switch_contract() {
        assert_eq!(
            SwitchSize::Default.metrics(),
            (
                gpui::px(32.0),
                gpui::px(18.4),
                gpui::px(16.0),
                gpui::px(1.2)
            )
        );
        assert_eq!(
            SwitchSize::Small.metrics(),
            (
                gpui::px(24.0),
                gpui::px(14.0),
                gpui::px(12.0),
                gpui::px(1.0)
            )
        );
    }
}
