//! Searchable single-action picker composed from G2V Popover and Command.

use std::rc::Rc;

use gpui::{App, AppContext as _, Context, Entity, IntoElement, RenderOnce, SharedString, Window};

use super::{
    Button, Command, CommandDensity, CommandGroup, CommandItem, CommandSearchStyle, CommandState,
    IconName, Popover, PopoverAlign, PopoverSide, popover::PopoverSurfaceStyle,
};

type SelectHandler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Clone)]
#[must_use]
pub struct SearchablePopoverItem {
    value: SharedString,
    label: SharedString,
    keywords: Vec<SharedString>,
    icon: Option<IconName>,
    selected: bool,
    disabled: bool,
    on_select: Option<SelectHandler>,
}

impl SearchablePopoverItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            keywords: Vec::new(),
            icon: None,
            selected: false,
            disabled: false,
            on_select: None,
        }
    }

    pub fn keyword(mut self, keyword: impl Into<SharedString>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
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

    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

pub struct SearchablePopoverState {
    command: Entity<CommandState>,
}

impl SearchablePopoverState {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        search_placeholder: impl Into<SharedString>,
    ) -> Self {
        Self {
            command: cx.new(|cx| CommandState::new(window, cx, search_placeholder)),
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SearchablePopover {
    id: SharedString,
    state: Entity<SearchablePopoverState>,
    trigger: Button,
    items: Vec<SearchablePopoverItem>,
    actions: Vec<SearchablePopoverItem>,
    empty_text: SharedString,
    group_label: Option<SharedString>,
    disabled: bool,
    command_density: CommandDensity,
    side: PopoverSide,
    align: PopoverAlign,
}

impl SearchablePopover {
    pub fn new(
        id: impl Into<SharedString>,
        state: &Entity<SearchablePopoverState>,
        trigger: Button,
    ) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            trigger,
            items: Vec::new(),
            actions: Vec::new(),
            empty_text: "No results found.".into(),
            group_label: None,
            disabled: false,
            command_density: CommandDensity::Default,
            side: PopoverSide::Top,
            align: PopoverAlign::Start,
        }
    }

    pub fn item(mut self, item: SearchablePopoverItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn action(mut self, action: SearchablePopoverItem) -> Self {
        self.actions.push(action);
        self
    }

    pub fn empty_text(mut self, empty_text: impl Into<SharedString>) -> Self {
        self.empty_text = empty_text.into();
        self
    }

    pub fn group_label(mut self, group_label: impl Into<SharedString>) -> Self {
        self.group_label = Some(group_label.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn command_density(mut self, density: CommandDensity) -> Self {
        self.command_density = density;
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
}

impl RenderOnce for SearchablePopover {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let command_state = self.state.read(cx).command.clone();
        let close_state = self.state.clone();
        let content_id = self.id.clone();
        let empty_text = self.empty_text.clone();
        let group_label = self.group_label.clone();
        let items = self.items.clone();
        let actions = self.actions.clone();
        let command_density = self.command_density;

        Popover::new(self.id)
            .trigger_button(self.trigger.disabled(self.disabled))
            .side(self.side)
            .align(self.align)
            .surface_style(PopoverSurfaceStyle::command())
            .on_open_change(move |open, window, cx| {
                if !open {
                    let command = close_state.read(cx).command.clone();
                    command.update(cx, |command, cx| {
                        command.clear_query(window, cx);
                    });
                }
            })
            .content(move |popover, _, _| {
                command_content(&SearchableCommandContent {
                    id: &content_id,
                    state: &command_state,
                    popover,
                    items: &items,
                    actions: &actions,
                    group_label: group_label.as_ref(),
                    empty_text: &empty_text,
                    density: command_density,
                })
            })
    }
}

struct SearchableCommandContent<'a> {
    id: &'a SharedString,
    state: &'a Entity<CommandState>,
    popover: &'a Entity<super::PopoverState>,
    items: &'a [SearchablePopoverItem],
    actions: &'a [SearchablePopoverItem],
    group_label: Option<&'a SharedString>,
    empty_text: &'a SharedString,
    density: CommandDensity,
}

fn command_content(content: &SearchableCommandContent<'_>) -> gpui::AnyElement {
    let mut group = CommandGroup::new();
    if let Some(label) = content.group_label {
        group = group.heading(label.clone());
    }
    for item in content.items {
        group = group.item(command_item(item, content.popover));
    }
    let mut command = Command::new(format!("{}-command", content.id), content.state)
        .density(content.density)
        .search_style(CommandSearchStyle::Integrated)
        .empty_text(content.empty_text.clone())
        .group(group);
    if !content.actions.is_empty() {
        let action_group = content
            .actions
            .iter()
            .fold(CommandGroup::new(), |group, action| {
                group.item(command_item(action, content.popover))
            });
        command = command.separator().group(action_group);
    }
    command.into_any_element()
}

fn command_item(
    item: &SearchablePopoverItem,
    popover: &Entity<super::PopoverState>,
) -> CommandItem {
    let mut command = CommandItem::new(item.value.clone(), item.label.clone())
        .disabled(item.disabled)
        .checked(item.selected);
    for keyword in &item.keywords {
        command = command.keyword(keyword.clone());
    }
    if let Some(icon) = item.icon {
        command = command.icon(icon);
    }
    let handler = item.on_select.clone();
    let popover = popover.clone();
    command.on_select(move |window, cx| {
        if let Some(handler) = &handler {
            handler(window, cx);
        }
        popover.update(cx, |popover, cx| popover.dismiss(window, cx));
    })
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{AppContext as _, Context, Modifiers, Render, WindowAppearance};

    use super::*;

    struct Harness {
        state: Entity<SearchablePopoverState>,
        selections: Rc<Cell<usize>>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let selections = self.selections.clone();
            SearchablePopover::new(
                "test-searchable-popover",
                &self.state,
                Button::new("test-searchable-trigger")
                    .label("Select")
                    .debug_selector("test-searchable-trigger"),
            )
            .group_label("Providers")
            .item(
                SearchablePopoverItem::new("codex", "Codex")
                    .keyword("OpenAI")
                    .selected(true)
                    .on_select(move |_, _| selections.set(selections.get() + 1)),
            )
        }
    }

    #[gpui::test]
    fn selection_closes_and_resets_the_search_query(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let selections = Rc::new(Cell::new(0));
        let (harness, window) = cx.add_window_view({
            let selections = selections.clone();
            move |window, cx| Harness {
                state: cx.new(|cx| SearchablePopoverState::new(window, cx, "Search providers...")),
                selections,
            }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let trigger = window.debug_bounds("test-searchable-trigger").unwrap();
        window.simulate_click(trigger.center(), Modifiers::default());
        window.update(|window, cx| window.draw(cx).clear(cx));

        window.update(|window, cx| {
            let command = harness.read(cx).state.read(cx).command.clone();
            command.update(cx, |command, cx| command.set_query("cod", window, cx));
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let item = window
            .debug_bounds("command-item-test-searchable-popover-command-codex")
            .unwrap();
        window.simulate_click(item.center(), Modifiers::default());

        assert_eq!(selections.get(), 1);
        window.update(|_, cx| {
            let command = harness.read(cx).state.read(cx).command.clone();
            assert!(command.read(cx).query(cx).is_empty());
        });
    }
}
