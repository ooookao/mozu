//! G2V's shadcn-compatible select built on gpui-base behavior.

use std::rc::Rc;

use gpui::{
    App, Context, ElementId, FocusHandle, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Window,
};
use gpui_base::Select as BaseSelect;

use super::{
    Button, ButtonSize, ButtonVariant, DropdownMenu, DropdownMenuDensity, DropdownMenuRadioGroup,
    DropdownMenuRadioItem, Icon, IconName, PopoverAlign,
};
use crate::theme::{rems_from_design_pixels, sizing::SIZE_4};

type ValueChangeHandler = Rc<dyn Fn(SharedString, &mut Window, &mut App)>;

const DEFAULT_MINIMUM_WIDTH: gpui::Rems = rems_from_design_pixels(128.0);

struct SelectState {
    open: bool,
    trigger_focus: FocusHandle,
    content_focus: FocusHandle,
}

impl SelectState {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            open: false,
            trigger_focus: cx.focus_handle(),
            content_focus: cx.focus_handle(),
        }
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.open == open {
            return;
        }
        self.open = open;
        cx.notify();
    }
}

#[derive(Clone)]
#[must_use]
pub struct SelectItem {
    value: SharedString,
    label: SharedString,
    disabled: bool,
}

impl SelectItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Select {
    id: SharedString,
    value: SharedString,
    label: SharedString,
    accessibility_label: SharedString,
    items: Vec<SelectItem>,
    disabled: bool,
    invalid: bool,
    align: PopoverAlign,
    minimum_width: gpui::AbsoluteLength,
    on_value_change: Option<ValueChangeHandler>,
    group_edges: Option<super::button_group::GroupEdges>,
}

impl Select {
    pub(crate) fn grouped(mut self, edges: super::button_group::GroupEdges) -> Self {
        self.group_edges = Some(edges);
        self
    }
    pub fn new(
        id: impl Into<SharedString>,
        value: impl Into<SharedString>,
        label: impl Into<SharedString>,
        accessibility_label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            label: label.into(),
            accessibility_label: accessibility_label.into(),
            items: Vec::new(),
            disabled: false,
            invalid: false,
            align: PopoverAlign::Start,
            minimum_width: DEFAULT_MINIMUM_WIDTH.into(),
            on_value_change: None,
            group_edges: None,
        }
    }

    pub fn item(mut self, item: SelectItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn align(mut self, align: PopoverAlign) -> Self {
        self.align = align;
        self
    }

    pub fn minimum_width(mut self, width: impl Into<gpui::AbsoluteLength>) -> Self {
        self.minimum_width = width.into();
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

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let state = window.use_keyed_state(
            (ElementId::from(self.id.clone()), "select-state"),
            cx,
            |_, cx| SelectState::new(cx),
        );
        let open = state.read(cx).open;
        let trigger_focus = state.read(cx).trigger_focus.clone();
        let content_focus = state.read(cx).content_focus.clone();

        let trigger = Button::new(format!("{}-trigger", self.id))
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Small)
            .label(self.label)
            .trailing_child(Icon::new(IconName::ChevronDown, SIZE_4))
            .track_focus(&trigger_focus)
            .disabled(self.disabled)
            .invalid(self.invalid);

        let trigger = match self.group_edges {
            Some(edges) => {
                let trigger = trigger.grouped(edges);
                #[cfg(test)]
                let trigger = trigger.debug_selector("button-group-select-trigger");
                trigger
            }
            None => trigger,
        };

        let group =
            self.items
                .into_iter()
                .fold(DropdownMenuRadioGroup::new(self.value), |group, item| {
                    group.item(
                        DropdownMenuRadioItem::new(item.value.clone(), item.label, false)
                            .value(item.value)
                            .disabled(item.disabled),
                    )
                });
        let group = if let Some(handler) = self.on_value_change {
            group.on_value_change(move |value, window, cx| handler(value, window, cx))
        } else {
            group
        };
        let menu_state = state.clone();
        let menu = DropdownMenu::new(format!("{}-menu", self.id), trigger)
            .align(self.align)
            .density(DropdownMenuDensity::Compact)
            .minimum_width(self.minimum_width)
            .controlled_open(open)
            .on_open_change(move |open, _, cx| {
                menu_state.update(cx, |state, cx| state.set_open(open, cx));
            })
            .radio_group(group);

        let base_state = state.clone();
        BaseSelect::new(id)
            .open(open)
            .disabled(self.disabled)
            .focus_handle(&trigger_focus)
            .content_focus_handle(&content_focus)
            .accessibility_label(self.accessibility_label)
            .on_open_change(move |open, _, cx| {
                base_state.update(cx, |state, cx| state.set_open(open, cx));
            })
            .child(menu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SelectHarness;

    impl gpui::Render for SelectHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            Select::new("theme", "system", "System", "Theme")
                .item(SelectItem::new("system", "System"))
                .item(SelectItem::new("dark", "Dark"))
        }
    }

    #[test]
    fn select_uses_shadcn_trigger_and_compact_popup_defaults() {
        let select = Select::new("theme", "light", "Light", "Theme")
            .item(SelectItem::new("light", "Light"))
            .item(SelectItem::new("dark", "Dark"));

        assert_eq!(select.items.len(), 2);
        assert_eq!(select.align, PopoverAlign::Start);
        assert_eq!(select.minimum_width, DEFAULT_MINIMUM_WIDTH.into());
    }

    #[test]
    fn select_items_preserve_disabled_state() {
        let item = SelectItem::new("system", "System").disabled(true);
        assert!(item.disabled);
    }

    #[gpui::test]
    fn pointer_open_transfers_focus_without_updating_it_twice_in_one_frame(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(gpui::WindowAppearance::Light, cx);
        });
        let (_, window) = cx.add_window_view(|_, _| SelectHarness);
        window.update(|window, cx| window.draw(cx).clear(cx));

        window.simulate_click(
            gpui::point(gpui::px(10.0), gpui::px(10.0)),
            Default::default(),
        );
        window.update(|window, cx| window.draw(cx).clear(cx));
    }
}
