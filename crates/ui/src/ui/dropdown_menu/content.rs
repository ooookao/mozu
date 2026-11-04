//! Runtime composition and interaction wiring for dropdown menu surfaces.

use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, ElementId, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    KeyDownEvent, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _,
    Window, div, prelude::FluentBuilder as _,
};

use crate::theme::{border::BORDER_DEFAULT, colors::G2VTheme, typography};

use super::super::TextStyleExt as _;
use super::{
    DropdownMenu, DropdownMenuDensity, DropdownMenuEntry, MenuInteractionTreeState,
    ParentMenuHandler, PopoverState,
    keyboard::{step_active, typeahead_character, typeahead_match},
    presentation::{
        SubmenuRowSpec, checkbox_row_spec, item_row_spec, menu_row, radio_row_spec, submenu_row,
    },
    state::DropdownMenuState,
};

struct MenuKeyboardContext {
    entries: Vec<DropdownMenuEntry>,
    navigable: Vec<SharedString>,
    state: Entity<DropdownMenuState>,
    popover: Entity<PopoverState>,
    on_left: Option<ParentMenuHandler>,
}

#[derive(Clone)]
pub(super) struct MenuContentContext {
    pub(super) id: SharedString,
    pub(super) entries: Vec<DropdownMenuEntry>,
    pub(super) state: Entity<DropdownMenuState>,
    pub(super) root_state: Entity<DropdownMenuState>,
    pub(super) popover: Entity<PopoverState>,
    pub(super) on_left: Option<ParentMenuHandler>,
    pub(super) interaction_tree: Entity<MenuInteractionTreeState>,
    pub(super) density: DropdownMenuDensity,
}

struct EntryRenderContext<'a> {
    menu_id: &'a SharedString,
    state: &'a Entity<DropdownMenuState>,
    popover: &'a Entity<PopoverState>,
    interaction_tree: &'a Entity<MenuInteractionTreeState>,
    root_state: &'a Entity<DropdownMenuState>,
    density: DropdownMenuDensity,
}

pub(super) fn render_menu_content(
    context: &MenuContentContext,
    _window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let element_id = ElementId::from(context.id.clone());
    let state = context.state.clone();
    let navigable = context
        .entries
        .iter()
        .filter_map(|entry| {
            if entry.navigable() {
                entry.id().cloned()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let active = state.read(cx).active.clone();
    let keyboard = MenuKeyboardContext {
        entries: context.entries.clone(),
        navigable: navigable.clone(),
        state: state.clone(),
        popover: context.popover.clone(),
        on_left: context.on_left.clone(),
    };
    let hover_tree = context.interaction_tree.clone();
    let hover_root = context.root_state.clone();
    let entry_context = EntryRenderContext {
        menu_id: &context.id,
        state: &state,
        popover: &context.popover,
        interaction_tree: &context.interaction_tree,
        root_state: &context.root_state,
        density: context.density,
    };

    div()
        .id((element_id, "menu-content"))
        .debug_selector(|| "dropdown-menu-content".into())
        .w_full()
        .flex()
        .flex_col()
        .whitespace_nowrap()
        .role(gpui::Role::Menu)
        .track_focus(&context.popover.read(cx).focus_handle(cx))
        .key_context("G2VDropdownMenu")
        .on_hover(move |hovered, _, cx| {
            hover_tree.update(cx, |tree, cx| {
                tree.surface_hover_changed(*hovered, hover_root.clone(), cx);
            });
        })
        .on_key_down(move |event, window, cx| {
            handle_menu_key(event, &keyboard, window, cx);
        })
        .children(context.entries.iter().map(|entry| {
            render_entry(
                entry,
                entry.id().is_some_and(|id| active.as_ref() == Some(id)),
                &entry_context,
                cx,
            )
        }))
        .into_any_element()
}

fn handle_menu_key(
    event: &KeyDownEvent,
    keyboard: &MenuKeyboardContext,
    window: &mut Window,
    cx: &mut App,
) {
    match event.keystroke.key.as_str() {
        "up" => move_menu_active(keyboard, -1, cx),
        "down" => move_menu_active(keyboard, 1, cx),
        "home" => set_menu_active(keyboard, keyboard.navigable.first().cloned(), cx),
        "end" => set_menu_active(keyboard, keyboard.navigable.last().cloned(), cx),
        "enter" | "space" => activate_current_entry(keyboard, window, cx),
        "right" => open_current_submenu(keyboard, cx),
        "left" => close_to_parent_menu(keyboard, cx),
        _ => handle_menu_typeahead(event, keyboard, cx),
    }
}

fn move_menu_active(keyboard: &MenuKeyboardContext, delta: isize, cx: &mut App) {
    cx.stop_propagation();
    keyboard.state.update(cx, |state, cx| {
        state.active = step_active(state.active.as_ref(), &keyboard.navigable, delta);
        cancel_submenu_interaction(state);
        cx.notify();
    });
}

fn set_menu_active(keyboard: &MenuKeyboardContext, active: Option<SharedString>, cx: &mut App) {
    cx.stop_propagation();
    keyboard.state.update(cx, |state, cx| {
        state.active = active;
        cancel_submenu_interaction(state);
        cx.notify();
    });
}

fn activate_current_entry(keyboard: &MenuKeyboardContext, window: &mut Window, cx: &mut App) {
    if let Some(active) = keyboard.state.read(cx).active.as_ref() {
        let Some(entry) = keyboard
            .entries
            .iter()
            .find(|entry| entry.id() == Some(active))
        else {
            return;
        };
        cx.stop_propagation();
        activate_entry(entry, &keyboard.popover, window, cx);
    }
}

fn open_current_submenu(keyboard: &MenuKeyboardContext, cx: &mut App) {
    let Some(active) = keyboard.state.read(cx).active.clone() else {
        return;
    };
    if !keyboard.entries.iter().any(|entry| {
        entry.id() == Some(&active) && matches!(entry, DropdownMenuEntry::Submenu { .. })
    }) {
        return;
    }
    cx.stop_propagation();
    keyboard.state.update(cx, |state, cx| {
        state.open_submenu = Some(active);
        state.pending_submenu = None;
        cx.notify();
    });
}

fn close_to_parent_menu(keyboard: &MenuKeyboardContext, cx: &mut App) {
    if let Some(on_left) = &keyboard.on_left {
        cx.stop_propagation();
        on_left(cx);
    }
}

fn handle_menu_typeahead(event: &KeyDownEvent, keyboard: &MenuKeyboardContext, cx: &mut App) {
    let Some(character) = typeahead_character(event) else {
        return;
    };
    cx.stop_propagation();
    keyboard.state.update(cx, |state, cx| {
        update_typeahead_query(state, &character, Instant::now());
        state.active = typeahead_match(
            &keyboard.entries,
            &keyboard.navigable,
            state.active.as_ref(),
            &state.typeahead,
        );
        cx.notify();
    });
}

fn cancel_submenu_interaction(state: &mut DropdownMenuState) {
    state.cancel_submenu_interaction();
}

pub(super) fn update_typeahead_query(state: &mut DropdownMenuState, character: &str, now: Instant) {
    if state
        .last_typeahead
        .is_none_or(|last| now.duration_since(last) > Duration::from_millis(500))
    {
        state.typeahead.clear();
    }
    state.typeahead.push_str(&character.to_lowercase());
    state.last_typeahead = Some(now);
}

fn render_entry(
    entry: &DropdownMenuEntry,
    active: bool,
    context: &EntryRenderContext<'_>,
    cx: &App,
) -> AnyElement {
    let theme = G2VTheme::global(cx);
    match entry {
        DropdownMenuEntry::Label { label, inset } => div()
            .w_full()
            .px_1p5()
            .py_1()
            .when(*inset, gpui::Styled::pl_7)
            .text_style(typography::tokens().xs)
            .text_color(theme.muted_foreground)
            .child(label.clone())
            .into_any_element(),
        DropdownMenuEntry::Separator => div()
            .debug_selector(|| "dropdown-menu-separator".into())
            .w_full()
            .h(BORDER_DEFAULT)
            .my_1()
            .bg(theme.popover_border)
            .into_any_element(),
        DropdownMenuEntry::CustomRow(render) => render(cx),
        DropdownMenuEntry::Item(item) => menu_row(
            context.menu_id,
            &item.id,
            item_row_spec(item, active, context.density),
            context.state,
            context.popover,
            Some(DropdownMenuEntry::Item(item.clone())),
            cx,
        ),
        DropdownMenuEntry::Checkbox(item) => menu_row(
            context.menu_id,
            &item.id,
            checkbox_row_spec(item, active, context.density),
            context.state,
            context.popover,
            Some(DropdownMenuEntry::Checkbox(item.clone())),
            cx,
        ),
        DropdownMenuEntry::Radio(item) => menu_row(
            context.menu_id,
            &item.id,
            radio_row_spec(item, active, context.density),
            context.state,
            context.popover,
            Some(DropdownMenuEntry::Radio(item.clone())),
            cx,
        ),
        DropdownMenuEntry::Submenu { .. } => render_submenu_entry(entry, active, context, cx),
    }
}

fn render_submenu_entry(
    entry: &DropdownMenuEntry,
    active: bool,
    context: &EntryRenderContext<'_>,
    cx: &App,
) -> AnyElement {
    let DropdownMenuEntry::Submenu {
        id,
        label,
        detail,
        inset,
        minimum_width,
        entries,
    } = entry
    else {
        unreachable!("render_submenu_entry requires a submenu entry");
    };
    let parent_state = context.state.clone();
    let submenu_open = context.state.read(cx).open_submenu.as_ref() == Some(id);
    let submenu_id = id.clone();
    let trigger_id = submenu_id.clone();
    let open_id = submenu_id.clone();
    let label = label.clone();
    let detail = detail.clone();
    let menu_id = context.menu_id.clone();
    let submenu_entries = entries.clone();
    let inset = *inset;
    let density = context.density;
    let minimum_width = *minimum_width;
    DropdownMenu::custom(submenu_id, move |open, _, cx| {
        submenu_row(
            &menu_id,
            SubmenuRowSpec {
                item_id: trigger_id.clone(),
                label: label.clone(),
                detail: detail.clone(),
                inset,
                active: active || open,
                density,
            },
            &parent_state,
            cx,
        )
        .into_any_element()
    })
    .with_entries(submenu_entries)
    .minimum_width(minimum_width)
    .density(context.density)
    .with_interaction_tree(context.interaction_tree.clone(), context.root_state.clone())
    .controlled(
        submenu_open,
        {
            let state = context.state.clone();
            Rc::new(move |open, _, cx| {
                state.update(cx, |state, cx| {
                    state.open_submenu = open.then(|| open_id.clone());
                    state.pending_submenu = None;
                    cx.notify();
                });
            })
        },
        {
            let state = context.state.clone();
            Rc::new(move |cx| {
                state.update(cx, |state, cx| {
                    state.open_submenu = None;
                    cx.notify();
                });
            })
        },
    )
    .into_any_element()
}

impl DropdownMenu {
    fn with_entries(mut self, entries: Vec<DropdownMenuEntry>) -> Self {
        self.entries = entries;
        self
    }
}

pub(super) fn activate_entry(
    entry: &DropdownMenuEntry,
    popover_state: &Entity<PopoverState>,
    window: &mut Window,
    cx: &mut App,
) {
    if matches!(
        entry,
        DropdownMenuEntry::Label { .. }
            | DropdownMenuEntry::Separator
            | DropdownMenuEntry::CustomRow(_)
            | DropdownMenuEntry::Submenu { .. }
    ) {
        return;
    }
    popover_state.update(cx, |state, cx| state.dismiss(window, cx));
    match entry {
        DropdownMenuEntry::Item(item) => {
            if let Some(handler) = &item.on_select {
                handler(window, cx);
            }
        }
        DropdownMenuEntry::Checkbox(item) => {
            if let Some(handler) = &item.on_checked_change {
                handler(!item.checked, window, cx);
            }
        }
        DropdownMenuEntry::Radio(item) => {
            if let Some(handler) = &item.on_select {
                handler(window, cx);
            }
        }
        DropdownMenuEntry::Label { .. }
        | DropdownMenuEntry::Separator
        | DropdownMenuEntry::CustomRow(_)
        | DropdownMenuEntry::Submenu { .. } => unreachable!("non-activatable entries return first"),
    }
}
