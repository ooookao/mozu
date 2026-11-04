//! Presentational grouping of independently operable controls.

use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};

use super::{Button, DropdownMenu, Input, Select, TextStyleExt};
use crate::theme::{
    border::BORDER_DEFAULT, colors::G2VTheme, radius::RADIUS_LG, rems_from_pixels, typography,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonGroupOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GroupEdges {
    orientation: ButtonGroupOrientation,
    first: bool,
    last: bool,
}

impl GroupEdges {
    pub(crate) fn apply<T: Styled + FluentBuilder>(self, element: T) -> T {
        let radius = rems_from_pixels(RADIUS_LG);
        match self.orientation {
            ButtonGroupOrientation::Horizontal => element
                .rounded_r(gpui::Pixels::ZERO)
                .when(self.last, |element| element.rounded_r(radius))
                .when(!self.first, |element| {
                    element.rounded_l(gpui::Pixels::ZERO).border_l_0()
                }),
            ButtonGroupOrientation::Vertical => element
                .rounded_b(gpui::Pixels::ZERO)
                .when(self.last, |element| element.rounded_b(radius))
                .when(!self.first, |element| {
                    element.rounded_t(gpui::Pixels::ZERO).border_t_0()
                }),
        }
    }
}

enum GroupPart {
    Button(Button),
    Menu(DropdownMenu),
    Input(Input),
    Select(Select),
    Text(ButtonGroupText),
    Separator(ButtonGroupSeparator),
    Group(Box<ButtonGroup>),
}

/// Layout only: each button retains its own action, focus and disabled state.
#[derive(IntoElement)]
#[must_use]
pub struct ButtonGroup {
    id: ElementId,
    orientation: ButtonGroupOrientation,
    label: Option<SharedString>,
    parts: Vec<GroupPart>,
    style: gpui::StyleRefinement,
}

impl ButtonGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            orientation: ButtonGroupOrientation::Horizontal,
            label: None,
            parts: Vec::new(),
            style: gpui::StyleRefinement::default(),
        }
    }

    pub fn orientation(mut self, orientation: ButtonGroupOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn button(mut self, button: Button) -> Self {
        self.parts.push(GroupPart::Button(button));
        self
    }

    pub fn dropdown_menu(mut self, menu: DropdownMenu) -> Self {
        self.parts.push(GroupPart::Menu(menu));
        self
    }

    pub fn input(mut self, input: Input) -> Self {
        self.parts.push(GroupPart::Input(input));
        self
    }

    pub fn select(mut self, select: Select) -> Self {
        self.parts.push(GroupPart::Select(select));
        self
    }

    pub fn text(mut self, text: ButtonGroupText) -> Self {
        self.parts.push(GroupPart::Text(text));
        self
    }

    pub fn separator(mut self, separator: ButtonGroupSeparator) -> Self {
        self.parts.push(GroupPart::Separator(separator));
        self
    }

    pub fn group(mut self, group: ButtonGroup) -> Self {
        self.parts.push(GroupPart::Group(Box::new(group)));
        self
    }
}

impl Styled for ButtonGroup {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ButtonGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let count = self.parts.len();
        let nested = self
            .parts
            .iter()
            .any(|part| matches!(part, GroupPart::Group(_)));
        div()
            .id(self.id)
            .role(gpui::Role::Group)
            .when_some(self.label, gpui::StatefulInteractiveElement::aria_label)
            .flex()
            .flex_none()
            .items_stretch()
            .when(nested, gpui::Styled::gap_2)
            .when(
                self.orientation == ButtonGroupOrientation::Vertical,
                gpui::Styled::flex_col,
            )
            .map(|mut group| {
                use gpui::Refineable;
                group.style().refine(&self.style);
                group
            })
            .children(self.parts.into_iter().enumerate().map(|(index, part)| {
                let edges = GroupEdges {
                    orientation: self.orientation,
                    first: index == 0,
                    last: index + 1 == count,
                };
                match part {
                    GroupPart::Button(button) => button.grouped(edges).into_any_element(),
                    GroupPart::Menu(menu) => menu.grouped(edges).into_any_element(),
                    GroupPart::Input(input) => input.in_button_group(edges).into_any_element(),
                    GroupPart::Select(select) => select.grouped(edges).into_any_element(),
                    GroupPart::Text(mut text) => {
                        text.edges = Some(edges);
                        text.into_any_element()
                    }
                    GroupPart::Separator(separator) => separator.into_any_element(),
                    GroupPart::Group(group) => (*group).into_any_element(),
                }
            }))
    }
}

/// Keeps layout in place while painting a keyboard-focused control above its siblings.
pub(crate) fn paint_group_control(
    element: impl IntoElement,
    focus: &gpui::FocusHandle,
    window: &Window,
) -> AnyElement {
    if focus.is_focused(window) && window.last_input_was_keyboard() {
        gpui::deferred(element).with_priority(1).into_any_element()
    } else {
        element.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{ButtonVariant, DropdownMenuItem};
    use gpui::{Modifiers, Render, TestAppContext, px};
    use std::{cell::Cell, rc::Rc};

    struct EditableHarness {
        input: gpui::Entity<gpui_base::input::InputState>,
        selected: Rc<std::cell::RefCell<String>>,
    }

    impl Render for EditableHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let selected = self.selected.clone();
            div().flex().items_start().child(
                ButtonGroup::new("editable-group")
                    .w_80()
                    .input(
                        Input::new(&self.input)
                            .size(super::super::InputSize::Small)
                            .accessibility_label("Name"),
                    )
                    .select(
                        Select::new("group-select", "one", "One", "Choice")
                            .item(super::super::SelectItem::new("one", "One"))
                            .item(super::super::SelectItem::new("two", "Two"))
                            .on_value_change(move |value, _, _| {
                                *selected.borrow_mut() = value.to_string()
                            }),
                    ),
            )
        }
    }

    #[gpui::test]
    fn grouped_input_edits_and_select_changes_value(cx: &mut TestAppContext) {
        use gpui::AppContext;
        init(cx);
        let selected = Rc::new(std::cell::RefCell::new(String::new()));
        let (harness, window) = cx.add_window_view(|window, cx| EditableHarness {
            input: cx.new(|cx| gpui_base::input::InputState::new(window, cx)),
            selected: selected.clone(),
        });
        for _ in 0..3 {
            window.update(|window, cx| window.draw(cx).clear(cx));
        }
        let input = window.debug_bounds("button-group-input").unwrap();
        let select = window.debug_bounds("button-group-select-trigger").unwrap();
        assert!(input.size.width > px(100.0));
        assert_eq!(input.right(), select.origin.x);
        assert_eq!(input.size.height, select.size.height);
        window.simulate_click(input.center(), Modifiers::default());
        window.simulate_keystrokes("hello");
        window.update(|_, cx| {
            assert_eq!(harness.read(cx).input.read(cx).text().to_string(), "hello")
        });
        window.simulate_click(select.center(), Modifiers::default());
        window.update(|window, cx| window.draw(cx).clear(cx));
        let option = window.debug_bounds("dropdown-menu-row-two").unwrap();
        window.simulate_click(option.center(), Modifiers::default());
        assert_eq!(&*selected.borrow(), "two");
    }

    struct FocusOrderHarness {
        focus: gpui::FocusHandle,
        order: Rc<std::cell::RefCell<Vec<&'static str>>>,
    }

    impl Render for FocusOrderHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let first = self.order.clone();
            let second = self.order.clone();
            div().flex().child(
                ButtonGroup::new("focus-order")
                    .button(
                        Button::new("first")
                            .debug_selector("focus-order-first")
                            .label("First")
                            .track_focus(&self.focus)
                            .child(gpui::canvas(
                                |_, _, _| {},
                                move |_, (), _, _| first.borrow_mut().push("first"),
                            )),
                    )
                    .button(
                        Button::new("second")
                            .debug_selector("focus-order-second")
                            .label("Second")
                            .child(gpui::canvas(
                                |_, _, _| {},
                                move |_, (), _, _| second.borrow_mut().push("second"),
                            )),
                    ),
            )
        }
    }

    #[gpui::test]
    fn keyboard_focus_paints_after_adjacent_controls_without_moving_them(cx: &mut TestAppContext) {
        init(cx);
        let order = Rc::new(std::cell::RefCell::new(Vec::new()));
        let (harness, window) = cx.add_window_view(|_, cx| FocusOrderHarness {
            focus: cx.focus_handle(),
            order: order.clone(),
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let first_bounds = window.debug_bounds("focus-order-first").unwrap();
        let second_bounds = window.debug_bounds("focus-order-second").unwrap();
        order.borrow_mut().clear();
        window.simulate_keystrokes("a");
        window.update(|window, cx| {
            let focus = harness.read(cx).focus.clone();
            focus.focus(window, cx);
            window.draw(cx).clear(cx);
        });
        let order = order.borrow();
        assert_eq!(&order[order.len() - 2..], &["second", "first"]);
        assert_eq!(
            window.debug_bounds("focus-order-first").unwrap(),
            first_bounds
        );
        assert_eq!(
            window.debug_bounds("focus-order-second").unwrap(),
            second_bounds
        );
    }

    struct GroupHarness {
        orientation: ButtonGroupOrientation,
        clicks: Rc<Cell<usize>>,
    }

    struct TextSeparatorHarness;

    impl Render for TextSeparatorHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div().flex().items_start().child(
                ButtonGroup::new("labelled-actions")
                    .text(ButtonGroupText::new().child("Action"))
                    .separator(ButtonGroupSeparator::new())
                    .button(
                        Button::new("labelled-save")
                            .label("Save")
                            .debug_selector("labelled-save"),
                    ),
            )
        }
    }

    #[gpui::test]
    fn text_and_separator_stretch_with_controls_in_both_themes(cx: &mut TestAppContext) {
        init(cx);
        let (_, window) = cx.add_window_view(|_, _| TextSeparatorHarness);
        for appearance in [gpui::WindowAppearance::Light, gpui::WindowAppearance::Dark] {
            window.update(|window, cx| {
                crate::theme::install(appearance, cx);
                window.draw(cx).clear(cx);
            });
            let text = window.debug_bounds("button-group-text").unwrap();
            let separator = window.debug_bounds("button-group-separator").unwrap();
            let button = window.debug_bounds("labelled-save").unwrap();
            assert_eq!(text.size.height, button.size.height);
            assert_eq!(separator.size.width, BORDER_DEFAULT);
            assert_eq!(separator.size.height, button.size.height - px(2.0));
            assert_eq!(text.right(), separator.origin.x);
            assert_eq!(separator.right(), button.origin.x);
        }
    }

    impl Render for GroupHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let clicks = self.clicks.clone();
            div().flex().items_start().child(
                ButtonGroup::new("actions")
                    .orientation(self.orientation)
                    .accessibility_label("Actions")
                    .button(
                        Button::new("primary")
                            .debug_selector("group-primary")
                            .variant(ButtonVariant::Outline)
                            .label("Save")
                            .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
                    )
                    .dropdown_menu(
                        DropdownMenu::new(
                            "group-options",
                            Button::new("options")
                                .debug_selector("group-options-trigger")
                                .variant(ButtonVariant::Outline)
                                .label("Options"),
                        )
                        .item(DropdownMenuItem::new("other", "Other action")),
                    ),
            )
        }
    }

    fn init(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(gpui::WindowAppearance::Light, cx);
        });
    }

    #[test]
    fn joining_removes_only_internal_edges() {
        for orientation in [
            ButtonGroupOrientation::Horizontal,
            ButtonGroupOrientation::Vertical,
        ] {
            for index in 0..3 {
                let mut element = GroupEdges {
                    orientation,
                    first: index == 0,
                    last: index == 2,
                }
                .apply(
                    div()
                        .border(BORDER_DEFAULT)
                        .rounded(rems_from_pixels(crate::theme::radius::RADIUS_MD)),
                );
                let style = element.style();
                match orientation {
                    ButtonGroupOrientation::Horizontal => {
                        assert_eq!(
                            style.border_widths.left,
                            Some(if index == 0 { BORDER_DEFAULT } else { px(0.0) }.into())
                        );
                        assert_eq!(style.border_widths.top, Some(BORDER_DEFAULT.into()));
                    }
                    ButtonGroupOrientation::Vertical => {
                        assert_eq!(
                            style.border_widths.top,
                            Some(if index == 0 { BORDER_DEFAULT } else { px(0.0) }.into())
                        );
                        assert_eq!(style.border_widths.left, Some(BORDER_DEFAULT.into()));
                    }
                }
                assert_eq!(
                    style.corner_radii.top_left,
                    Some(if index == 0 {
                        rems_from_pixels(crate::theme::radius::RADIUS_MD).into()
                    } else {
                        px(0.0).into()
                    })
                );
                assert_eq!(
                    style.corner_radii.bottom_right,
                    Some(if index == 2 {
                        rems_from_pixels(RADIUS_LG).into()
                    } else {
                        px(0.0).into()
                    })
                );
            }
        }
    }

    #[gpui::test]
    fn horizontal_controls_touch_and_actions_remain_independent(cx: &mut TestAppContext) {
        init(cx);
        let clicks = Rc::new(Cell::new(0));
        let (_, window) = cx.add_window_view(|_, _| GroupHarness {
            orientation: ButtonGroupOrientation::Horizontal,
            clicks: clicks.clone(),
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let primary = window.debug_bounds("group-primary").unwrap();
        let trigger = window.debug_bounds("group-options-trigger").unwrap();
        assert_eq!(primary.right(), trigger.origin.x);
        assert_eq!(primary.origin.y, trigger.origin.y);
        window.simulate_click(primary.center(), Modifiers::default());
        assert_eq!(clicks.get(), 1);
        window.simulate_click(trigger.center(), Modifiers::default());
        window.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(clicks.get(), 1);
        assert!(window.debug_bounds("dropdown-menu-content").is_some());
    }

    #[gpui::test]
    fn vertical_controls_touch(cx: &mut TestAppContext) {
        init(cx);
        let (_, window) = cx.add_window_view(|_, _| GroupHarness {
            orientation: ButtonGroupOrientation::Vertical,
            clicks: Rc::new(Cell::new(0)),
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        let primary = window.debug_bounds("group-primary").unwrap();
        let trigger = window.debug_bounds("group-options-trigger").unwrap();
        assert_eq!(primary.bottom(), trigger.origin.y);
        assert_eq!(primary.origin.x, trigger.origin.x);
    }
}

#[derive(IntoElement, Default)]
#[must_use]
pub struct ButtonGroupText {
    children: Vec<AnyElement>,
    edges: Option<GroupEdges>,
}

impl ButtonGroupText {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for ButtonGroupText {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ButtonGroupText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        div()
            .debug_selector(|| "button-group-text".into())
            .flex()
            .items_center()
            .gap_2()
            .px_2p5()
            .rounded(rems_from_pixels(RADIUS_LG))
            .border(BORDER_DEFAULT)
            .border_color(theme.border)
            .bg(theme.muted)
            .text_style(typography::tokens().sm)
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.foreground)
            .when_some(self.edges, |text, edges| edges.apply(text))
            .children(self.children)
    }
}

/// A one-pixel separator stretched across the group's cross axis.
#[derive(IntoElement)]
#[must_use]
pub struct ButtonGroupSeparator {
    orientation: super::SeparatorOrientation,
}

impl Default for ButtonGroupSeparator {
    fn default() -> Self {
        Self {
            orientation: super::SeparatorOrientation::Vertical,
        }
    }
}

impl ButtonGroupSeparator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn orientation(mut self, orientation: super::SeparatorOrientation) -> Self {
        self.orientation = orientation;
        self
    }
}

impl RenderOnce for ButtonGroupSeparator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        super::Separator::grouped(self.orientation)
    }
}
