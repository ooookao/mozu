use crate::theme::Theme;
use crate::ui::button::{Button, ButtonSize, ButtonVariant};
use gpui::{
    AnyElement, App, ElementId, FocusHandle, IntoElement, ParentElement, Refineable, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, px,
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToggleVariant {
    #[default]
    Default,
    Outline,
}

/// Compatibility sizes mapped to the common compact/default density pair.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToggleSize {
    #[default]
    Default,
    Sm,
    Lg,
}

type ChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

/// A controlled toggle using the shared button surface and press presentation.
#[derive(IntoElement)]
pub struct Toggle {
    button: Button,
    pressed: bool,
    tab_stop: bool,
    invalid: bool,
    change: Option<ChangeHandler>,
    style: StyleRefinement,
}
impl Toggle {
    pub fn new(id: impl Into<ElementId>, pressed: bool) -> Self {
        Self {
            button: Button::new(id)
                .variant(ButtonVariant::Ghost)
                .active(pressed),
            pressed,
            tab_stop: true,
            invalid: false,
            change: None,
            style: StyleRefinement::default(),
        }
    }
    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.button = self.button.variant(match variant {
            ToggleVariant::Default => ButtonVariant::Ghost,
            ToggleVariant::Outline => ButtonVariant::Tertiary,
        });
        self
    }
    pub fn size(mut self, size: ToggleSize) -> Self {
        self.button = self.button.size(match size {
            ToggleSize::Sm => ButtonSize::Compact,
            ToggleSize::Default => ButtonSize::Default,
            ToggleSize::Lg => ButtonSize::Lg,
        });
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.button = self.button.disabled(disabled);
        self
    }
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self.button = self.button.invalid(invalid);
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.button = self.button.accessibility_label(label);
        self
    }
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.button = self.button.track_focus(focus);
        self
    }
    /// Uses the shared square button geometry for an icon-only toggle.
    pub fn icon_only(mut self, icon_only: bool) -> Self {
        self.button = self.button.icon_only(icon_only);
        self
    }
    pub fn icon_start(mut self, icon: impl IntoElement + 'static) -> Self {
        self.button = self.button.icon_start(icon);
        self
    }
    pub fn icon_end(mut self, icon: impl IntoElement + 'static) -> Self {
        self.button = self.button.icon_end(icon);
        self
    }
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.change = Some(Rc::new(handler));
        self
    }
}
impl Styled for Toggle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for Toggle {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.button.extend(children);
    }
}
impl Toggle {
    /// Exposes the shared presentation for controlled group composition.
    pub fn into_primitive(mut self, window: &mut Window, cx: &mut App) -> kinu_ui::Toggle {
        let theme = Theme::current(window, cx);
        self.button = self.button.text_color(if self.pressed {
            theme.foreground
        } else {
            theme.muted_foreground
        });
        if self.invalid {
            self.button = self
                .button
                .outline_width(px(1.))
                .outline_color(theme.destructive);
        }
        self.button.style().refine(&self.style);
        let mut toggle =
            kinu_ui::Toggle::from_button(self.button.into_primitive(window, cx), self.pressed)
                .tab_stop(self.tab_stop);
        if self.invalid {
            toggle = toggle.focus_visible(|style| {
                style.outline_width(px(1.)).outline_color(theme.destructive)
            });
        }
        if let Some(change) = self.change {
            toggle = toggle.on_change(move |value, window, cx| change(value, window, cx));
        }
        toggle
    }
}

impl RenderOnce for Toggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.into_primitive(window, cx)
    }
}
