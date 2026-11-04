//! Dropdown menu row presentation and typed visual contracts.

use std::time::Duration;

use gpui::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _,
};

use crate::theme::{
    colors::G2VTheme,
    radius::RADIUS_MD,
    rems_from_pixels,
    sizing::{SIZE_3_5, SIZE_4},
    spacing::{SPACE_2, SPACE_8},
    typography,
};

use super::super::{Icon, Kbd, TextStyleExt as _};
use super::content::activate_entry;
use super::state::DropdownMenuState;
use super::{
    DropdownMenuCheckboxItem, DropdownMenuDensity, DropdownMenuEntry, DropdownMenuItem,
    DropdownMenuItemVariant, DropdownMenuRadioItem, IconName, PopoverState,
};

pub(super) struct MenuRowSpec {
    label: SharedString,
    detail: Option<SharedString>,
    icon: Option<IconName>,
    shortcut: Option<SharedString>,
    trailing_icon: Option<IconName>,
    trailing: MenuRowTrailing,
    indent: MenuRowIndent,
    availability: MenuRowAvailability,
    variant: DropdownMenuItemVariant,
    active: bool,
    role: gpui::Role,
    selected: Option<bool>,
    kind: MenuRowKind,
    density: DropdownMenuDensity,
}

pub(super) fn menu_message(message: SharedString, cx: &App) -> AnyElement {
    div()
        .debug_selector(|| "dropdown-menu-message".into())
        .w_80()
        .whitespace_normal()
        .px_2()
        .py_1()
        .text_style(density_text_style(DropdownMenuDensity::Compact))
        .text_color(G2VTheme::global(cx).muted_foreground)
        .child(message)
        .into_any_element()
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum MenuRowTrailing {
    None,
    Reserved,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum MenuRowIndent {
    Normal,
    Inset,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum MenuRowAvailability {
    Enabled,
    Disabled,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum MenuRowKind {
    Item,
    Submenu,
}

pub(super) fn item_row_spec(
    item: &DropdownMenuItem,
    active: bool,
    density: DropdownMenuDensity,
) -> MenuRowSpec {
    MenuRowSpec {
        label: item.label.clone(),
        detail: item.detail.clone(),
        icon: item.icon,
        shortcut: item.shortcut.clone(),
        trailing_icon: item.selected.then_some(IconName::Check),
        trailing: if item.selected {
            MenuRowTrailing::Reserved
        } else {
            MenuRowTrailing::None
        },
        indent: row_indent(item.inset),
        availability: row_availability(item.disabled),
        variant: item.variant,
        active,
        role: gpui::Role::MenuItem,
        selected: item.selected.then_some(true),
        kind: MenuRowKind::Item,
        density,
    }
}

pub(super) fn checkbox_row_spec(
    item: &DropdownMenuCheckboxItem,
    active: bool,
    density: DropdownMenuDensity,
) -> MenuRowSpec {
    MenuRowSpec {
        label: item.label.clone(),
        detail: None,
        icon: None,
        shortcut: None,
        trailing_icon: item.checked.then_some(IconName::Check),
        trailing: MenuRowTrailing::Reserved,
        indent: MenuRowIndent::Normal,
        availability: row_availability(item.disabled),
        variant: DropdownMenuItemVariant::Default,
        active,
        role: gpui::Role::MenuItemCheckBox,
        selected: Some(item.checked),
        kind: MenuRowKind::Item,
        density,
    }
}

pub(super) fn radio_row_spec(
    item: &DropdownMenuRadioItem,
    active: bool,
    density: DropdownMenuDensity,
) -> MenuRowSpec {
    MenuRowSpec {
        label: item.label.clone(),
        detail: None,
        icon: None,
        shortcut: None,
        trailing_icon: item.selected.then_some(IconName::Check),
        trailing: MenuRowTrailing::Reserved,
        indent: MenuRowIndent::Normal,
        availability: row_availability(item.disabled),
        variant: DropdownMenuItemVariant::Default,
        active,
        role: gpui::Role::MenuItemRadio,
        selected: Some(item.selected),
        kind: MenuRowKind::Item,
        density,
    }
}

const fn row_indent(inset: bool) -> MenuRowIndent {
    if inset {
        MenuRowIndent::Inset
    } else {
        MenuRowIndent::Normal
    }
}

const fn row_availability(disabled: bool) -> MenuRowAvailability {
    if disabled {
        MenuRowAvailability::Disabled
    } else {
        MenuRowAvailability::Enabled
    }
}

pub(super) fn menu_row(
    menu_id: &SharedString,
    item_id: &SharedString,
    spec: MenuRowSpec,
    state: &Entity<DropdownMenuState>,
    popover_state: &Entity<PopoverState>,
    activation: Option<DropdownMenuEntry>,
    cx: &App,
) -> AnyElement {
    let mut row = menu_row_visual(menu_id, item_id, &spec, state, cx);
    if let Some(shortcut) = spec.shortcut {
        row = row.child(Kbd::new().label(shortcut).ml_auto());
    }
    if spec.availability == MenuRowAvailability::Disabled {
        return row.into_any_element();
    }
    let popover_state = popover_state.clone();
    row.when_some(activation, |row, activation| {
        row.on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            activate_entry(&activation, &popover_state, window, cx);
        })
    })
    .into_any_element()
}

fn menu_row_visual(
    menu_id: &SharedString,
    item_id: &SharedString,
    spec: &MenuRowSpec,
    state: &Entity<DropdownMenuState>,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    let theme = G2VTheme::global(cx);
    let (foreground, background) = menu_row_colors(spec, theme);
    let hover_state = state.clone();
    let row_kind = spec.kind;
    let text_style = density_text_style(spec.density);
    let icon_size = density_icon_size(spec.density);
    let item_id = item_id.clone();
    let debug_id = item_id.clone();
    div()
        .id((gpui::ElementId::from(menu_id.clone()), item_id.clone()))
        .debug_selector(move || format!("dropdown-menu-row-{debug_id}"))
        .relative()
        .role(spec.role)
        .aria_label(spec.label.clone())
        .when_some(
            spec.selected,
            gpui::StatefulInteractiveElement::aria_selected,
        )
        .w_full()
        .flex()
        .items_center()
        .gap_1p5()
        .rounded(RADIUS_MD)
        .px_1p5()
        .py_1()
        .when(spec.indent == MenuRowIndent::Inset, gpui::Styled::pl_7)
        .text_style(text_style)
        .text_color(foreground)
        .bg(background)
        .cursor_default()
        .when(spec.availability == MenuRowAvailability::Disabled, |row| {
            row.opacity(0.5)
        })
        .when_some(spec.icon, |row, icon| {
            row.child(Icon::new(icon, icon_size).color(foreground))
        })
        .child(menu_row_text(spec, theme))
        .when(spec.trailing == MenuRowTrailing::Reserved, |row| {
            row.child(
                div()
                    .ml_auto()
                    .w(rems_from_pixels(SPACE_8))
                    .flex()
                    .justify_end()
                    .when_some(spec.trailing_icon, |indicator, icon| {
                        indicator.child(Icon::new(icon, SIZE_4).color(foreground))
                    }),
            )
        })
        .when(spec.availability == MenuRowAvailability::Enabled, |row| {
            row.on_mouse_move(move |_, _, cx| {
                hover_state.update(cx, |state, cx| {
                    if row_kind == MenuRowKind::Submenu {
                        if state.open_submenu.as_ref() == Some(&item_id)
                            || state.pending_submenu.as_ref() == Some(&item_id)
                        {
                            return;
                        }
                        state.active = Some(item_id.clone());
                        state.pending_submenu = Some(item_id.clone());
                        state.hover_generation += 1;
                        let generation = state.hover_generation;
                        let delayed_item_id = item_id.clone();
                        cx.spawn(async move |state, cx| {
                            cx.background_executor()
                                .timer(Duration::from_millis(100))
                                .await;
                            let _ = state.update(cx, |state, cx| {
                                if state.hover_generation == generation
                                    && state.pending_submenu.as_ref() == Some(&delayed_item_id)
                                {
                                    state.pending_submenu = None;
                                    state.open_submenu = Some(delayed_item_id.clone());
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                        cx.notify();
                    } else if state.active.as_ref() != Some(&item_id)
                        || state.open_submenu.is_some()
                    {
                        state.hover_generation += 1;
                        state.pending_submenu = None;
                        state.active = Some(item_id.clone());
                        state.open_submenu = None;
                        cx.notify();
                    }
                });
            })
        })
}

fn menu_row_text(spec: &MenuRowSpec, theme: &G2VTheme) -> impl IntoElement {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .items_center()
        .gap(SPACE_2)
        .whitespace_nowrap()
        .child(
            div()
                .debug_selector(|| "dropdown-menu-row-label".into())
                .child(spec.label.clone()),
        )
        .when_some(spec.detail.clone(), |text, detail| {
            text.child(
                div()
                    .debug_selector(|| "dropdown-menu-row-detail".into())
                    .ml_auto()
                    .when(spec.kind == MenuRowKind::Item, |detail| {
                        detail.text_style(typography::tokens().xs)
                    })
                    .text_color(theme.muted_foreground)
                    .child(detail),
            )
        })
}

pub(super) fn density_text_style(density: DropdownMenuDensity) -> gpui_base::TextStyleToken {
    match density {
        DropdownMenuDensity::Default => typography::tokens().sm,
        DropdownMenuDensity::Compact => typography::TEXT_13,
    }
}

pub(super) const fn density_icon_size(density: DropdownMenuDensity) -> gpui::Rems {
    match density {
        DropdownMenuDensity::Default => SIZE_4,
        DropdownMenuDensity::Compact => SIZE_3_5,
    }
}

pub(super) struct SubmenuRowSpec {
    pub(super) item_id: SharedString,
    pub(super) label: SharedString,
    pub(super) detail: Option<SharedString>,
    pub(super) inset: bool,
    pub(super) active: bool,
    pub(super) density: DropdownMenuDensity,
}

pub(super) fn submenu_row(
    menu_id: &SharedString,
    spec: SubmenuRowSpec,
    state: &Entity<DropdownMenuState>,
    cx: &App,
) -> gpui::Stateful<gpui::Div> {
    menu_row_visual(
        menu_id,
        &spec.item_id,
        &MenuRowSpec {
            label: spec.label,
            detail: spec.detail,
            icon: None,
            shortcut: None,
            trailing_icon: Some(IconName::ChevronRight),
            trailing: MenuRowTrailing::Reserved,
            indent: row_indent(spec.inset),
            availability: MenuRowAvailability::Enabled,
            variant: DropdownMenuItemVariant::Default,
            active: spec.active,
            role: gpui::Role::MenuItem,
            selected: None,
            kind: MenuRowKind::Submenu,
            density: spec.density,
        },
        state,
        cx,
    )
}

fn menu_row_colors(spec: &MenuRowSpec, theme: &G2VTheme) -> (gpui::Hsla, gpui::Hsla) {
    let foreground = match (spec.variant, spec.active) {
        (DropdownMenuItemVariant::Destructive, _) => theme.destructive,
        (DropdownMenuItemVariant::Default, true) => theme.accent_foreground,
        (DropdownMenuItemVariant::Default, false) => theme.popover_foreground,
    };
    let background = match (spec.variant, spec.active) {
        (DropdownMenuItemVariant::Destructive, true) => theme.destructive.opacity(0.1),
        (DropdownMenuItemVariant::Default, true) => theme.popover_hover,
        (_, false) => theme.accent.opacity(0.0),
    };
    (foreground, background)
}
