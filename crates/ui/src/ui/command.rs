//! Cmdk-compatible command state with shadcn presentation.

use std::{collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, InteractiveElement as _,
    IntoElement, KeyDownEvent, ParentElement as _, Rems, RenderOnce, SharedString, Styled as _,
    Subscription, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_base::{Button as ButtonBehavior, TextStyleToken, input::InputEvent};

use crate::theme::{
    border::BORDER_DEFAULT,
    colors::G2VTheme,
    radius::{RADIUS_LG, RADIUS_SM},
    rems_from_design_pixels, rems_from_pixels,
    sizing::{SIZE_3_5, SIZE_4},
    spacing::{SPACE_1, SPACE_1_5, SPACE_2},
    typography,
};

use super::{
    Icon, IconName, InputGroup, InputGroupAddon, InputGroupAddonAlign, InputState, Kbd, ScrollArea,
    TextStyleExt as _, dialog::command_dialog_trigger, input_group::InputGroupSize,
};

type SelectHandler = Rc<dyn Fn(&mut Window, &mut App)>;

const COMMAND_LIST_MAX_HEIGHT: Rems = rems_from_design_pixels(288.0);
const COMMAND_ITEM_HEIGHT: Rems = rems_from_design_pixels(32.0);
const COMMAND_GROUP_PADDING: Rems = rems_from_design_pixels(8.0);
const COMMAND_GROUP_HEADING_HEIGHT: Rems = rems_from_design_pixels(28.0);
const COMMAND_EMPTY_HEIGHT: Rems = rems_from_design_pixels(68.0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CommandDensity {
    #[default]
    Default,
    Compact,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CommandSearchStyle {
    #[default]
    Boxed,
    Integrated,
}

#[derive(Clone, Copy)]
struct CommandMetrics {
    input_size: InputGroupSize,
    input_icon_size: gpui::Rems,
    item_height: Rems,
    item_padding_x: Rems,
    item_padding_y: Rems,
    item_gap: Rems,
    item_icon_size: gpui::Rems,
    compact_text: bool,
    group_padding: Rems,
    heading_height: Rems,
    heading_padding_x: Rems,
    heading_padding_y: Rems,
    empty_height: Rems,
}

impl CommandDensity {
    fn metrics(self) -> CommandMetrics {
        match self {
            Self::Default => CommandMetrics {
                input_size: InputGroupSize::Default,
                input_icon_size: SIZE_4,
                item_height: COMMAND_ITEM_HEIGHT,
                item_padding_x: rems_from_pixels(SPACE_2),
                item_padding_y: rems_from_pixels(SPACE_1_5),
                item_gap: rems_from_pixels(SPACE_2),
                item_icon_size: SIZE_4,
                compact_text: false,
                group_padding: COMMAND_GROUP_PADDING,
                heading_height: COMMAND_GROUP_HEADING_HEIGHT,
                heading_padding_x: rems_from_pixels(SPACE_2),
                heading_padding_y: rems_from_pixels(SPACE_1_5),
                empty_height: COMMAND_EMPTY_HEIGHT,
            },
            Self::Compact => CommandMetrics {
                input_size: InputGroupSize::Small,
                input_icon_size: SIZE_3_5,
                item_height: rems_from_design_pixels(28.0),
                item_padding_x: rems_from_pixels(SPACE_1_5),
                item_padding_y: rems_from_pixels(SPACE_1),
                item_gap: rems_from_pixels(SPACE_1_5),
                item_icon_size: SIZE_3_5,
                compact_text: true,
                group_padding: Rems::ZERO,
                heading_height: rems_from_design_pixels(24.0),
                heading_padding_x: rems_from_pixels(SPACE_1_5),
                heading_padding_y: rems_from_pixels(SPACE_1),
                empty_height: rems_from_design_pixels(56.0),
            },
        }
    }
}

fn command_text_style(metrics: CommandMetrics) -> TextStyleToken {
    if metrics.compact_text {
        TextStyleToken {
            size: px(12.8),
            ..typography::tokens().sm
        }
    } else {
        typography::tokens().sm
    }
}

pub struct CommandState {
    query: Entity<InputState>,
    active: usize,
    _query_subscription: Subscription,
}

impl CommandState {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        placeholder: impl Into<SharedString>,
    ) -> Self {
        let placeholder = placeholder.into();
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let query_subscription = cx.subscribe(&query, |this, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.active = 0;
                cx.notify();
            }
        });
        Self {
            query,
            active: 0,
            _query_subscription: query_subscription,
        }
    }

    pub(crate) fn query(&self, cx: &App) -> String {
        self.query.read(cx).text().to_string()
    }

    #[cfg(test)]
    pub(crate) fn set_query(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.query
            .update(cx, |query, cx| query.set_value(value, window, cx));
    }

    fn move_selection(&mut self, direction: CommandDirection, item_count: usize) {
        self.active = next_selection(self.active, direction, item_count);
    }

    pub(crate) fn clear_query(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.active = 0;
        self.query
            .update(cx, |query, cx| query.set_value("", window, cx));
        cx.notify();
    }
}

#[derive(Clone, Copy)]
enum CommandDirection {
    Previous,
    Next,
}

fn next_selection(current: usize, direction: CommandDirection, item_count: usize) -> usize {
    if item_count == 0 {
        return 0;
    }
    match direction {
        CommandDirection::Previous => current.checked_sub(1).unwrap_or(item_count - 1),
        CommandDirection::Next => (current + 1) % item_count,
    }
}

#[derive(Clone)]
#[must_use]
pub struct CommandItem {
    value: SharedString,
    label: SharedString,
    keywords: Vec<SharedString>,
    icon: Option<IconName>,
    shortcut: Option<SharedString>,
    disabled: bool,
    checked: bool,
    on_select: Option<SelectHandler>,
}

impl CommandItem {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            keywords: Vec::new(),
            icon: None,
            shortcut: None,
            disabled: false,
            checked: false,
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

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    fn score(&self, query: &str) -> f32 {
        if query.is_empty() {
            return 1.0;
        }
        command_score(
            &format!("{} {}", self.value, self.label),
            query,
            &self.keywords,
        )
    }
}

#[derive(Clone)]
#[must_use]
pub struct CommandGroup {
    heading: Option<SharedString>,
    items: Vec<CommandItem>,
}

impl CommandGroup {
    pub fn new() -> Self {
        Self {
            heading: None,
            items: Vec::new(),
        }
    }

    pub fn heading(mut self, heading: impl Into<SharedString>) -> Self {
        self.heading = Some(heading.into());
        self
    }

    pub fn item(mut self, item: CommandItem) -> Self {
        self.items.push(item);
        self
    }
}

impl Default for CommandGroup {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
enum CommandSection {
    Group(CommandGroup),
    Separator,
}

#[derive(Clone, IntoElement)]
#[must_use]
pub struct Command {
    id: SharedString,
    state: Entity<CommandState>,
    empty_text: SharedString,
    sections: Vec<CommandSection>,
    density: CommandDensity,
    search_style: CommandSearchStyle,
}

impl Command {
    pub fn new(id: impl Into<SharedString>, state: &Entity<CommandState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            empty_text: "No results found.".into(),
            sections: Vec::new(),
            density: CommandDensity::Default,
            search_style: CommandSearchStyle::Boxed,
        }
    }

    pub fn empty_text(mut self, empty_text: impl Into<SharedString>) -> Self {
        self.empty_text = empty_text.into();
        self
    }

    pub fn group(mut self, group: CommandGroup) -> Self {
        self.sections.push(CommandSection::Group(group));
        self
    }

    pub fn separator(mut self) -> Self {
        self.sections.push(CommandSection::Separator);
        self
    }

    pub fn density(mut self, density: CommandDensity) -> Self {
        self.density = density;
        self
    }

    pub fn search_style(mut self, style: CommandSearchStyle) -> Self {
        self.search_style = style;
        self
    }
}

impl RenderOnce for Command {
    #[expect(
        clippy::too_many_lines,
        reason = "the declarative command composition keeps keyboard and visual order together"
    )]
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let metrics = self.density.metrics();
        let query = self.state.read(cx).query(cx);
        let normalized_query = query.trim().to_lowercase();
        let visible_values = visible_values(&self.sections, &normalized_query);
        let active = self
            .state
            .read(cx)
            .active
            .min(visible_values.len().saturating_sub(1));
        let active_value = visible_values.get(active).cloned();
        let item_count = visible_values.len();
        let key_state = self.state.clone();
        let enter_sections = self.sections.clone();
        let enter_query = normalized_query.clone();
        let enter_value = active_value.clone();
        let rendered_sections = render_sections(
            &CommandRenderContext {
                command_id: &self.id,
                state: &self.state,
                active_value: active_value.as_ref(),
                visible_values: &visible_values,
                theme: &theme,
                metrics,
            },
            &self.sections,
            &normalized_query,
        );
        let list_height = command_list_height(&self.sections, &normalized_query, metrics);

        let query_state = self.state.read(cx).query.clone();
        let integrated_search = self.search_style == CommandSearchStyle::Integrated;
        let input_group = if integrated_search {
            InputGroup::command_integrated(("command-input", query_state.entity_id()), &query_state)
        } else {
            InputGroup::command(("command-input", query_state.entity_id()), &query_state)
        };
        let search_addon = InputGroupAddon::new(
            InputGroupAddonAlign::InlineStart,
            Icon::new(IconName::Search, metrics.input_icon_size).color(theme.muted_foreground),
        );
        let search_addon = if integrated_search && self.density == CommandDensity::Compact {
            search_addon.inline_padding(metrics.item_padding_x, Rems::ZERO)
        } else {
            search_addon
        };
        div()
            .id(self.id)
            .w_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(RADIUS_LG)
            .bg(theme.popover)
            .p_1()
            .text_color(theme.popover_foreground)
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                let direction = match event.keystroke.key.as_str() {
                    "up" => Some(CommandDirection::Previous),
                    "down" => Some(CommandDirection::Next),
                    _ => None,
                };
                if let Some(direction) = direction {
                    cx.stop_propagation();
                    key_state.update(cx, |state, cx| {
                        state.move_selection(direction, item_count);
                        cx.notify();
                    });
                    return;
                }
                if event.keystroke.key == "enter" {
                    cx.stop_propagation();
                    if let Some(value) = &enter_value
                        && let Some(item) = find_item(&enter_sections, &enter_query, value)
                        && let Some(handler) = &item.on_select
                    {
                        handler(window, cx);
                    }
                }
            })
            .child(
                div()
                    .when(!integrated_search, |search| search.p_1().pb(Rems::ZERO))
                    .child(
                        input_group
                            .size(metrics.input_size)
                            .accessibility_label("Search commands")
                            .addon(search_addon),
                    ),
            )
            .child(
                ScrollArea::new(("command-list", query_state.entity_id()))
                    .h(list_height)
                    .max_h(COMMAND_LIST_MAX_HEIGHT)
                    .child(
                        div()
                            .max_h(COMMAND_LIST_MAX_HEIGHT)
                            .when(rendered_sections.is_empty(), |list| {
                                list.child(
                                    div()
                                        .h(metrics.empty_height)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_style(command_text_style(metrics))
                                        .text_center()
                                        .child(self.empty_text),
                                )
                            })
                            .children(rendered_sections),
                    ),
            )
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct CommandDialog {
    trigger: AnyElement,
    title: SharedString,
    description: SharedString,
    command: Command,
}

impl CommandDialog {
    pub fn new(
        trigger: impl IntoElement,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        command: Command,
    ) -> Self {
        Self {
            trigger: trigger.into_any_element(),
            title: title.into(),
            description: description.into(),
            command,
        }
    }
}

impl RenderOnce for CommandDialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let title = self.title;
        let description = self.description;
        let command = self.command;
        command_dialog_trigger(self.trigger, move |_, _| {
            div()
                .relative()
                .child(
                    div()
                        .absolute()
                        .w(px(1.0))
                        .h(px(1.0))
                        .overflow_hidden()
                        .child(title.clone())
                        .child(description.clone()),
                )
                .child(command.clone())
                .into_any_element()
        })
    }
}

struct CommandRenderContext<'a> {
    command_id: &'a SharedString,
    state: &'a Entity<CommandState>,
    active_value: Option<&'a SharedString>,
    visible_values: &'a [SharedString],
    theme: &'a G2VTheme,
    metrics: CommandMetrics,
}

fn render_sections(
    context: &CommandRenderContext<'_>,
    sections: &[CommandSection],
    query: &str,
) -> Vec<AnyElement> {
    let mut rendered = Vec::new();
    for section in sections {
        match section {
            CommandSection::Group(group) => {
                let items = ranked_items(group, query)
                    .into_iter()
                    .map(|item| {
                        render_item(
                            context.command_id,
                            context.state,
                            item,
                            context
                                .visible_values
                                .iter()
                                .position(|value| value == &item.value),
                            context.active_value == Some(&item.value),
                            context.theme,
                            context.metrics,
                        )
                    })
                    .collect::<Vec<_>>();
                if !items.is_empty() {
                    rendered.push(render_group(
                        group.heading.clone(),
                        items,
                        context.theme,
                        context.metrics,
                    ));
                }
            }
            CommandSection::Separator if !rendered.is_empty() => rendered.push(
                div()
                    .mx(-SPACE_1)
                    .my_1()
                    .h(BORDER_DEFAULT)
                    .bg(context.theme.border)
                    .into_any_element(),
            ),
            CommandSection::Separator => {}
        }
    }
    rendered
}

fn find_item<'a>(
    sections: &'a [CommandSection],
    query: &str,
    value: &SharedString,
) -> Option<&'a CommandItem> {
    sections
        .iter()
        .filter_map(|section| match section {
            CommandSection::Group(group) => Some(&group.items),
            CommandSection::Separator => None,
        })
        .flatten()
        .find(|item| !item.disabled && item.score(query) > 0.0 && item.value == *value)
}

fn visible_values(sections: &[CommandSection], query: &str) -> Vec<SharedString> {
    sections
        .iter()
        .filter_map(|section| match section {
            CommandSection::Group(group) => Some(ranked_items(group, query)),
            CommandSection::Separator => None,
        })
        .flatten()
        .filter(|item| !item.disabled)
        .map(|item| item.value.clone())
        .collect()
}

fn command_list_height(sections: &[CommandSection], query: &str, metrics: CommandMetrics) -> Rems {
    let mut height = Rems::ZERO;
    let mut has_content = false;
    for section in sections {
        match section {
            CommandSection::Group(group) => {
                let item_count = ranked_items(group, query).len();
                if item_count > 0 {
                    let heading_height = if group.heading.is_some() {
                        metrics.heading_height
                    } else {
                        Rems::ZERO
                    };
                    let item_count = u16::try_from(item_count)
                        .expect("command group item count must fit in u16");
                    height += metrics.group_padding
                        + heading_height
                        + metrics.item_height * f32::from(item_count);
                    has_content = true;
                }
            }
            CommandSection::Separator if has_content => {
                height += rems_from_pixels(BORDER_DEFAULT + SPACE_1 * 2.0);
            }
            CommandSection::Separator => {}
        }
    }
    if !has_content {
        return metrics.empty_height;
    }
    if height.0 < COMMAND_LIST_MAX_HEIGHT.0 {
        height
    } else {
        COMMAND_LIST_MAX_HEIGHT
    }
}

fn ranked_items<'a>(group: &'a CommandGroup, query: &str) -> Vec<&'a CommandItem> {
    let mut ranked = group
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let score = item.score(query);
            (score > 0.0).then_some((index, score, item))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    ranked.into_iter().map(|(_, _, item)| item).collect()
}

fn render_group(
    heading: Option<SharedString>,
    items: Vec<AnyElement>,
    theme: &G2VTheme,
    metrics: CommandMetrics,
) -> AnyElement {
    div()
        .overflow_hidden()
        .p(metrics.group_padding / 2.0)
        .text_color(theme.foreground)
        .when_some(heading, |group, heading| {
            group.child(
                div()
                    .px(metrics.heading_padding_x)
                    .py(metrics.heading_padding_y)
                    .text_style(typography::tokens().xs)
                    .text_color(theme.muted_foreground)
                    .child(heading),
            )
        })
        .children(items)
        .into_any_element()
}

fn render_item(
    command_id: &SharedString,
    state: &Entity<CommandState>,
    item: &CommandItem,
    visible_index: Option<usize>,
    selected: bool,
    theme: &G2VTheme,
    metrics: CommandMetrics,
) -> AnyElement {
    let hover_state = state.clone();
    let click_handler = item.on_select.clone();
    let selected_background = theme.popover_hover;
    let selected_foreground = theme.foreground;

    ButtonBehavior::new((ElementId::from(command_id.clone()), item.value.clone()))
        .debug_selector({
            let command_id = command_id.clone();
            let value = item.value.clone();
            move || format!("command-item-{command_id}-{value}")
        })
        .accessibility_label(item.label.clone())
        .disabled(item.disabled)
        .selected(selected)
        .focusable(false)
        .styles(|styles| {
            styles
                .selected(|style| {
                    style
                        .bg(selected_background)
                        .text_color(selected_foreground)
                })
                .disabled(|style| style.opacity(0.5))
        })
        .relative()
        .w_full()
        .flex()
        .items_center()
        .gap(metrics.item_gap)
        .rounded(RADIUS_SM)
        .px(metrics.item_padding_x)
        .py(metrics.item_padding_y)
        .text_style(command_text_style(metrics))
        .text_color(theme.foreground)
        .on_mouse_move(move |_, _, cx| {
            if let Some(index) = visible_index {
                hover_state.update(cx, |state, cx| {
                    if state.active != index {
                        state.active = index;
                        cx.notify();
                    }
                });
            }
        })
        .on_click(move |_, window, cx| {
            if let Some(handler) = &click_handler {
                handler(window, cx);
            }
        })
        .children(
            item.icon
                .map(|icon| Icon::new(icon, metrics.item_icon_size).into_any_element()),
        )
        .child(div().min_w_0().flex_1().child(item.label.clone()))
        .when_some(item.shortcut.clone(), |row, shortcut| {
            row.child(Kbd::new().label(shortcut).ml_auto())
        })
        .when(item.checked && item.shortcut.is_none(), |row| {
            row.child(
                Icon::new(IconName::Check, metrics.item_icon_size)
                    .color(theme.foreground)
                    .into_any_element(),
            )
        })
        .into_any_element()
}

// Port of cmdk 1.1.1's command-score constants and recursive matching model.
fn command_score(value: &str, query: &str, keywords: &[SharedString]) -> f32 {
    let candidate = if keywords.is_empty() {
        value.to_owned()
    } else {
        format!(
            "{} {}",
            value,
            keywords
                .iter()
                .map(SharedString::as_ref)
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let original = candidate.chars().collect::<Vec<_>>();
    let normalized = normalize_score_text(&candidate);
    let query_original = query.chars().collect::<Vec<_>>();
    let query_normalized = normalize_score_text(query);
    score_recursive(
        &original,
        &query_original,
        &normalized,
        &query_normalized,
        0,
        0,
        &mut HashMap::new(),
    )
}

fn normalize_score_text(value: &str) -> Vec<char> {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_whitespace() || character == '-' {
                ' '
            } else {
                character
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn score_recursive(
    original: &[char],
    query_original: &[char],
    normalized: &[char],
    query: &[char],
    candidate_index: usize,
    query_index: usize,
    memo: &mut HashMap<(usize, usize), f32>,
) -> f32 {
    if query_index == query.len() {
        return if candidate_index == original.len() {
            1.0
        } else {
            0.99
        };
    }
    if let Some(score) = memo.get(&(candidate_index, query_index)) {
        return *score;
    }

    let mut best = 0.0_f32;
    for found in candidate_index..normalized.len() {
        if normalized[found] != query[query_index] {
            continue;
        }
        let mut score = score_recursive(
            original,
            query_original,
            normalized,
            query,
            found + 1,
            query_index + 1,
            memo,
        );
        score *= score_position(original, candidate_index, found);
        if original.get(found) != query_original.get(query_index) {
            score *= 0.9999;
        }
        best = best.max(score);
    }
    memo.insert((candidate_index, query_index), best);
    best
}

fn score_position(original: &[char], start: usize, found: usize) -> f32 {
    if found == start {
        return 1.0;
    }
    let previous = original[found - 1];
    let gap = found.saturating_sub(start);
    let gap_penalty = 0.999_f32.powi(i32::try_from(gap).unwrap_or(i32::MAX));
    if matches!(
        previous,
        '\\' | '/' | '_' | '+' | '.' | '#' | '"' | '@' | '[' | '(' | '{' | '&'
    ) {
        0.8 * gap_penalty
    } else if previous.is_whitespace() || previous == '-' {
        0.9 * gap_penalty
    } else {
        0.17 * gap_penalty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtering_matches_value_label_and_keywords_case_insensitively() {
        let item = CommandItem::new("project-open", "Open project").keyword("workspace");

        assert!(item.score("project") > 0.0);
        assert!(item.score("open") > 0.0);
        assert!(item.score("workspace") > 0.0);
        assert!(item.score("branch").abs() < f32::EPSILON);
    }

    #[test]
    fn cmdk_scoring_supports_fuzzy_subsequences_and_prefers_prefixes() {
        let prefix = command_score("Open project", "open", &[]);
        let fuzzy = command_score("Open project", "opj", &[]);

        assert!(prefix > fuzzy);
        assert!(fuzzy > 0.0);
    }

    #[test]
    fn command_dimensions_match_the_react_contract() {
        assert_eq!(RADIUS_LG, px(13.2));
        assert_eq!(COMMAND_LIST_MAX_HEIGHT.to_pixels(px(16.0)), px(288.0));
        assert_eq!(COMMAND_LIST_MAX_HEIGHT.to_pixels(px(18.0)), px(324.0));
    }

    #[test]
    fn keyboard_selection_wraps_and_handles_an_empty_list() {
        assert_eq!(next_selection(0, CommandDirection::Previous, 3), 2);
        assert_eq!(next_selection(2, CommandDirection::Next, 3), 0);
        assert_eq!(next_selection(4, CommandDirection::Next, 0), 0);
    }

    #[test]
    fn list_height_is_intrinsic_and_capped_for_auto_height_dialogs() {
        let sections = vec![CommandSection::Group(
            CommandGroup::new()
                .heading("Suggestions")
                .item(CommandItem::new("one", "One"))
                .item(CommandItem::new("two", "Two")),
        )];

        assert_eq!(
            command_list_height(&sections, "", CommandDensity::Default.metrics())
                .to_pixels(px(16.0)),
            px(100.0)
        );
        assert_eq!(
            command_list_height(&sections, "missing", CommandDensity::Default.metrics())
                .to_pixels(px(16.0)),
            px(68.0)
        );
        assert_eq!(
            command_list_height(&sections, "", CommandDensity::Compact.metrics())
                .to_pixels(px(16.0)),
            px(80.0)
        );
        assert_eq!(
            command_list_height(&sections, "missing", CommandDensity::Compact.metrics())
                .to_pixels(px(16.0)),
            px(56.0)
        );
        assert_eq!(
            command_list_height(&sections, "", CommandDensity::Default.metrics())
                .to_pixels(px(18.0)),
            px(112.5)
        );
    }
}
