use crate::theme::Theme;
use crate::ui::dropdown_menu::{self as dropdown, DropdownStyle};
use crate::ui::utils::motion::popup_motion;
use crate::ui::utils::presentation::ComponentTheme;
use gpui::{App, Styled, Window, px};
use kinu_ui::{MenuAppearance, MenuRowAppearance, MenuRowKind, MenuRowState, PopupMotion};

/// The caller's menu role; all popup menus share one visual recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuKind {
    Dropdown,
    Context,
    Menubar,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItemVariant {
    Default,
    Destructive,
}

/// Refines semantic command ink while preserving the common row geometry.
pub fn item_variant(
    _kind: MenuKind,
    variant: MenuItemVariant,
    window: &Window,
    cx: &App,
) -> impl Fn(MenuRowState, MenuRowAppearance) -> MenuRowAppearance + 'static {
    let color = Theme::current(window, cx).destructive;
    move |state, mut parts| {
        if variant == MenuItemVariant::Destructive && matches!(state.kind, MenuRowKind::Command) {
            parts.row = parts.row.text_color(color);
            parts.label = parts.label.text_color(color);
            parts.icon = parts.icon.text_color(color);
        }
        parts
    }
}
pub fn appearance(_kind: MenuKind, window: &Window, cx: &App) -> MenuAppearance {
    dropdown::appearance(DropdownStyle::current(cx), dropdown::scheme(window, cx))
}
pub fn motion(cx: &App) -> PopupMotion {
    popup_motion(ComponentTheme::current(cx).motion.fast, px(4.))
}
