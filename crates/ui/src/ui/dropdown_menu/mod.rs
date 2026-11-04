//! G2V's shadcn-compatible dropdown menu composition.

use std::rc::Rc;

mod content;
mod keyboard;
mod presentation;
mod state;

use content::{MenuContentContext, render_menu_content};
use state::{DropdownMenuState, MenuInteractionTreeState};

use gpui::{
    AbsoluteLength, AnyElement, App, ElementId, Entity, IntoElement, Rems, RenderOnce,
    SharedString, Window,
};

use crate::theme::rems_from_design_pixels;

use super::{
    Button, IconName, Popover, PopoverAlign, PopoverSide, PopoverState,
    popover::{
        PopoverFocusOwner, PopoverInteractionMode, PopoverSurfaceStyle, PopoverTriggerLayout,
    },
};

type SelectHandler = Rc<dyn Fn(&mut Window, &mut App)>;
type CheckedChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type ValueChangeHandler = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;
type TriggerBuilder = Rc<dyn Fn(bool, &Window, &App) -> AnyElement>;
type ContextTriggerBuilder = Box<dyn FnOnce(bool, &Window, &App) -> AnyElement>;
type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type ParentMenuHandler = Rc<dyn Fn(&mut App)>;
type CustomRowBuilder = Rc<dyn Fn(&App) -> AnyElement>;

const DEFAULT_SIDE_OFFSET: Rems = rems_from_design_pixels(4.0);
const SUBMENU_SIDE_OFFSET: Rems = Rems::ZERO;
const SUBMENU_ALIGN_OFFSET: Rems = rems_from_design_pixels(-3.0);
const DEFAULT_MINIMUM_WIDTH: Rems = rems_from_design_pixels(128.0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DropdownMenuItemVariant {
    #[default]
    Default,
    Destructive,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DropdownMenuDensity {
    Default,
    #[default]
    Compact,
}

#[derive(Clone)]
#[must_use]
pub struct DropdownMenuItem {
    id: SharedString,
    label: SharedString,
    detail: Option<SharedString>,
    icon: Option<IconName>,
    shortcut: Option<SharedString>,
    selected: bool,
    disabled: bool,
    inset: bool,
    variant: DropdownMenuItemVariant,
    on_select: Option<SelectHandler>,
}

impl DropdownMenuItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            detail: None,
            icon: None,
            shortcut: None,
            selected: false,
            disabled: false,
            inset: false,
            variant: DropdownMenuItemVariant::Default,
            on_select: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    pub fn variant(mut self, variant: DropdownMenuItemVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

#[derive(Clone)]
#[must_use]
pub struct DropdownMenuCheckboxItem {
    id: SharedString,
    label: SharedString,
    checked: bool,
    disabled: bool,
    on_checked_change: Option<CheckedChangeHandler>,
}

impl DropdownMenuCheckboxItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>, checked: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            checked,
            disabled: false,
            on_checked_change: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_checked_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_checked_change = Some(Rc::new(handler));
        self
    }
}

#[derive(Clone)]
#[must_use]
pub struct DropdownMenuRadioItem {
    id: SharedString,
    value: SharedString,
    label: SharedString,
    selected: bool,
    disabled: bool,
    on_select: Option<SelectHandler>,
}

impl DropdownMenuRadioItem {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        selected: bool,
    ) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            value: label.clone(),
            label,
            selected,
            disabled: false,
            on_select: None,
        }
    }

    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = value.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

#[must_use]
pub struct DropdownMenuRadioGroup {
    value: SharedString,
    items: Vec<DropdownMenuRadioItem>,
    on_value_change: Option<ValueChangeHandler>,
}

impl DropdownMenuRadioGroup {
    pub fn new(value: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            items: Vec::new(),
            on_value_change: None,
        }
    }

    pub fn item(mut self, item: DropdownMenuRadioItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn on_value_change(
        mut self,
        handler: impl Fn(SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_value_change = Some(Rc::new(handler));
        self
    }
}

#[derive(Clone)]
enum DropdownMenuEntry {
    Label {
        label: SharedString,
        inset: bool,
    },
    Item(DropdownMenuItem),
    Checkbox(DropdownMenuCheckboxItem),
    Radio(DropdownMenuRadioItem),
    Separator,
    CustomRow(CustomRowBuilder),
    Submenu {
        id: SharedString,
        label: SharedString,
        detail: Option<SharedString>,
        inset: bool,
        minimum_width: AbsoluteLength,
        entries: Vec<DropdownMenuEntry>,
    },
}

impl DropdownMenuEntry {
    fn id(&self) -> Option<&SharedString> {
        match self {
            Self::Item(item) => Some(&item.id),
            Self::Checkbox(item) => Some(&item.id),
            Self::Radio(item) => Some(&item.id),
            Self::Submenu { id, .. } => Some(id),
            Self::Label { .. } | Self::Separator | Self::CustomRow(_) => None,
        }
    }

    const fn disabled(&self) -> bool {
        match self {
            Self::Item(item) => item.disabled,
            Self::Checkbox(item) => item.disabled,
            Self::Radio(item) => item.disabled,
            Self::Label { .. } | Self::Separator | Self::CustomRow(_) => true,
            Self::Submenu { .. } => false,
        }
    }

    const fn navigable(&self) -> bool {
        !matches!(
            self,
            Self::Label { .. } | Self::Separator | Self::CustomRow(_)
        ) && !self.disabled()
    }
}

#[must_use]
pub struct DropdownMenuSub {
    id: SharedString,
    label: SharedString,
    detail: Option<SharedString>,
    inset: bool,
    minimum_width: AbsoluteLength,
    entries: Vec<DropdownMenuEntry>,
}

impl DropdownMenuSub {
    /// Adds non-interactive diagnostic text without treating it as a selectable action.
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        let message = message.into();
        self.entries
            .push(DropdownMenuEntry::CustomRow(Rc::new(move |cx| {
                presentation::menu_message(message.clone(), cx)
            })));
        self
    }

    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            detail: None,
            inset: false,
            minimum_width: DEFAULT_MINIMUM_WIDTH.into(),
            entries: Vec::new(),
        }
    }

    pub fn inset(mut self, inset: bool) -> Self {
        self.inset = inset;
        self
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn minimum_width(mut self, width: impl Into<AbsoluteLength>) -> Self {
        self.minimum_width = width.into();
        self
    }

    pub fn item(mut self, item: DropdownMenuItem) -> Self {
        self.entries.push(DropdownMenuEntry::Item(item));
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(DropdownMenuEntry::Separator);
        self
    }
}

enum DropdownMenuTrigger {
    Button(Box<Button>),
    Custom(TriggerBuilder),
    Context(ContextTriggerBuilder),
}

#[derive(IntoElement)]
#[must_use]
pub struct DropdownMenu {
    id: SharedString,
    trigger: DropdownMenuTrigger,
    entries: Vec<DropdownMenuEntry>,
    side: PopoverSide,
    align: PopoverAlign,
    side_offset: AbsoluteLength,
    align_offset: AbsoluteLength,
    open: Option<bool>,
    on_open_change: Option<OpenChangeHandler>,
    on_left: Option<ParentMenuHandler>,
    interaction_tree: Option<Entity<MenuInteractionTreeState>>,
    root_state: Option<Entity<DropdownMenuState>>,
    density: DropdownMenuDensity,
    minimum_width: AbsoluteLength,
}

impl DropdownMenu {
    pub(crate) fn grouped(mut self, edges: super::button_group::GroupEdges) -> Self {
        if let DropdownMenuTrigger::Button(button) = self.trigger {
            self.trigger = DropdownMenuTrigger::Button(Box::new(button.grouped(edges)));
        }
        self
    }

    pub fn new(id: impl Into<SharedString>, trigger: Button) -> Self {
        Self {
            id: id.into(),
            trigger: DropdownMenuTrigger::Button(Box::new(trigger)),
            entries: Vec::new(),
            side: PopoverSide::Bottom,
            align: PopoverAlign::Start,
            side_offset: DEFAULT_SIDE_OFFSET.into(),
            align_offset: Rems::ZERO.into(),
            open: None,
            on_open_change: None,
            on_left: None,
            interaction_tree: None,
            root_state: None,
            density: DropdownMenuDensity::Compact,
            minimum_width: DEFAULT_MINIMUM_WIDTH.into(),
        }
    }

    fn custom(
        id: impl Into<SharedString>,
        trigger: impl Fn(bool, &Window, &App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            trigger: DropdownMenuTrigger::Custom(Rc::new(trigger)),
            entries: Vec::new(),
            side: PopoverSide::Right,
            align: PopoverAlign::Start,
            side_offset: SUBMENU_SIDE_OFFSET.into(),
            align_offset: SUBMENU_ALIGN_OFFSET.into(),
            open: None,
            on_open_change: None,
            on_left: None,
            interaction_tree: None,
            root_state: None,
            density: DropdownMenuDensity::Compact,
            minimum_width: DEFAULT_MINIMUM_WIDTH.into(),
        }
    }

    pub fn context(
        id: impl Into<SharedString>,
        trigger: impl FnOnce(bool, &Window, &App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            trigger: DropdownMenuTrigger::Context(Box::new(trigger)),
            entries: Vec::new(),
            side: PopoverSide::Bottom,
            align: PopoverAlign::Start,
            side_offset: DEFAULT_SIDE_OFFSET.into(),
            align_offset: Rems::ZERO.into(),
            open: None,
            on_open_change: None,
            on_left: None,
            interaction_tree: None,
            root_state: None,
            density: DropdownMenuDensity::Compact,
            minimum_width: DEFAULT_MINIMUM_WIDTH.into(),
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(DropdownMenuEntry::Label {
            label: label.into(),
            inset: false,
        });
        self
    }

    pub fn inset_label(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(DropdownMenuEntry::Label {
            label: label.into(),
            inset: true,
        });
        self
    }

    pub fn item(mut self, item: DropdownMenuItem) -> Self {
        self.entries.push(DropdownMenuEntry::Item(item));
        self
    }

    pub fn checkbox_item(mut self, item: DropdownMenuCheckboxItem) -> Self {
        self.entries.push(DropdownMenuEntry::Checkbox(item));
        self
    }

    pub fn radio_item(mut self, item: DropdownMenuRadioItem) -> Self {
        self.entries.push(DropdownMenuEntry::Radio(item));
        self
    }

    pub fn radio_group(mut self, group: DropdownMenuRadioGroup) -> Self {
        for mut item in group.items {
            item.selected = item.value == group.value;
            if let Some(handler) = group.on_value_change.clone() {
                let value = item.value.clone();
                item.on_select = Some(Rc::new(move |window, cx| {
                    handler(value.clone(), window, cx);
                }));
            }
            self.entries.push(DropdownMenuEntry::Radio(item));
        }
        self
    }

    pub fn separator(mut self) -> Self {
        self.entries.push(DropdownMenuEntry::Separator);
        self
    }

    pub fn custom_row(mut self, render: impl Fn(&App) -> AnyElement + 'static) -> Self {
        self.entries
            .push(DropdownMenuEntry::CustomRow(Rc::new(render)));
        self
    }

    pub fn submenu(mut self, submenu: DropdownMenuSub) -> Self {
        self.entries.push(DropdownMenuEntry::Submenu {
            id: submenu.id,
            label: submenu.label,
            detail: submenu.detail,
            inset: submenu.inset,
            minimum_width: submenu.minimum_width,
            entries: submenu.entries,
        });
        self
    }

    pub fn side(mut self, side: PopoverSide) -> Self {
        self.side = side;
        self
    }

    pub fn align(mut self, align: PopoverAlign) -> Self {
        self.align = align;
        self
    }

    pub fn density(mut self, density: DropdownMenuDensity) -> Self {
        self.density = density;
        self
    }

    pub fn minimum_width(mut self, width: impl Into<AbsoluteLength>) -> Self {
        self.minimum_width = width.into();
        self
    }

    pub fn on_open_change(
        mut self,
        callback: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    pub fn controlled_open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    fn controlled(
        mut self,
        open: bool,
        on_open_change: OpenChangeHandler,
        on_left: ParentMenuHandler,
    ) -> Self {
        self.open = Some(open);
        self.on_open_change = Some(on_open_change);
        self.on_left = Some(on_left);
        self
    }

    fn with_interaction_tree(
        mut self,
        tree: Entity<MenuInteractionTreeState>,
        root_state: Entity<DropdownMenuState>,
    ) -> Self {
        self.interaction_tree = Some(tree);
        self.root_state = Some(root_state);
        self
    }
}

impl RenderOnce for DropdownMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let interaction_mode = match &self.trigger {
            DropdownMenuTrigger::Button(_) => PopoverInteractionMode::Modal,
            DropdownMenuTrigger::Custom(_) => PopoverInteractionMode::NonModal,
            DropdownMenuTrigger::Context(_) => PopoverInteractionMode::Modal,
        };
        let id = self.id.clone();
        let interaction_tree = self.interaction_tree.unwrap_or_else(|| {
            window.use_keyed_state(
                (ElementId::from(id.clone()), "interaction-tree"),
                cx,
                |_, _| MenuInteractionTreeState::default(),
            )
        });
        let menu_state =
            window.use_keyed_state((ElementId::from(id.clone()), "menu-state"), cx, |_, _| {
                DropdownMenuState::default()
            });
        let root_state = self.root_state.unwrap_or_else(|| menu_state.clone());
        let entries = self.entries.clone();
        let on_left = self.on_left.clone();
        let density = self.density;
        let minimum_width = self.minimum_width.to_pixels(window.rem_size());
        let content_tree = interaction_tree.clone();
        let content_state = menu_state.clone();
        let content_root_state = root_state.clone();
        let mut popover = Popover::new(self.id)
            .side(self.side)
            .align(self.align)
            .side_offset(self.side_offset)
            .align_offset(self.align_offset)
            .interaction_mode(interaction_mode)
            .focus_owner(PopoverFocusOwner::Content)
            .surface_style(PopoverSurfaceStyle::menu(minimum_width))
            .content(move |popover_state, window, cx| {
                let context = MenuContentContext {
                    id: id.clone(),
                    entries: entries.clone(),
                    state: content_state.clone(),
                    root_state: content_root_state.clone(),
                    popover: popover_state.clone(),
                    on_left: on_left.clone(),
                    interaction_tree: content_tree.clone(),
                    density,
                };
                render_menu_content(&context, window, cx)
            });
        if let Some(open) = self.open {
            popover = popover.open(open);
        }
        if let Some(on_open_change) = self.on_open_change {
            popover = popover.on_open_change(move |open, window, cx| {
                on_open_change(open, window, cx);
            });
        }
        popover = match self.trigger {
            DropdownMenuTrigger::Button(button) => popover.trigger_button(*button),
            DropdownMenuTrigger::Custom(trigger) => popover
                .trigger_layout(PopoverTriggerLayout::Fill)
                .trigger(move |open, window, cx| trigger(open, window, cx)),
            DropdownMenuTrigger::Context(trigger) => popover
                .trigger_layout(PopoverTriggerLayout::Fill)
                .passive_trigger()
                .trigger(move |open, window, cx| trigger(open, window, cx)),
        };
        popover
    }
}

#[cfg(test)]
mod tests;
