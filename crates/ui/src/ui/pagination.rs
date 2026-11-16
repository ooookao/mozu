use crate::ui::button::{ButtonSize, ButtonVariant, button_variants};
use crate::ui::icon::{Icon, IconName};
use crate::ui::utils::presentation::{ComponentTheme, ControlSize};
use gpui::{
    AnyElement, App, Div, ElementId, InteractiveElement, IntoElement, Link, ParentElement,
    Refineable, RenderOnce, Role, SharedString, StatefulInteractiveElement, StyleRefinement,
    Styled, Window, div, px,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageState {
    Other,
    Current,
}
pub(crate) struct Presentation {
    pub(crate) content: StyleRefinement,
    pub(crate) previous: StyleRefinement,
    pub(crate) next: StyleRefinement,
    pub(crate) ellipsis: StyleRefinement,
    pub(crate) ellipsis_icon: f32,
}
pub(crate) fn presentation(_window: &Window, cx: &App) -> Presentation {
    let tokens = ComponentTheme::current(cx);
    let compact = tokens.size == ControlSize::Compact;
    let metrics = tokens.size.metrics(&tokens);
    Presentation {
        content: StyleRefinement::default()
            .flex()
            .items_center()
            .gap(px(if compact { 2. } else { 4. })),
        previous: StyleRefinement::default().pl(px(if compact { 8. } else { 12. })),
        next: StyleRefinement::default().pr(px(if compact { 8. } else { 12. })),
        ellipsis: StyleRefinement::default()
            .flex()
            .items_center()
            .justify_center()
            .size(px(metrics.control_height)),
        ellipsis_icon: if compact { 14. } else { 16. },
    }
}
fn icon_button_size(cx: &App) -> ButtonSize {
    match ComponentTheme::current(cx).size {
        ControlSize::Default => ButtonSize::Icon,
        ControlSize::Compact => ButtonSize::IconCompact,
    }
}
/// Styles a real URL link; page selection and page-number generation remain caller-owned.
pub fn pagination_link(
    id: impl Into<ElementId>,
    url: impl Into<SharedString>,
    label: impl Into<SharedString>,
    state: PageState,
    window: &Window,
    cx: &App,
) -> Link {
    let label = label.into();
    let variant = match state {
        PageState::Other => ButtonVariant::Ghost,
        PageState::Current => ButtonVariant::Outline,
    };
    let mut link = styled_link(
        id,
        url,
        label.clone(),
        variant,
        icon_button_size(cx),
        window,
        cx,
    )
    .child(label);
    if state == PageState::Current {
        link = link.aria_current(gpui::accesskit::AriaCurrent::Page);
    }
    link
}
fn styled_link(
    id: impl Into<ElementId>,
    url: impl Into<SharedString>,
    label: SharedString,
    variant: ButtonVariant,
    size: ButtonSize,
    window: &Window,
    cx: &App,
) -> Link {
    let p = button_variants(variant, size, window, cx);
    let mut link = Link::new(id, url)
        .aria_label(label)
        .hover(|_| p.hover)
        .focus_visible(|_| p.focus)
        .active(|_| p.active)
        .transition(gpui::StyleTransition::colors(
            ComponentTheme::current(cx)
                .motion
                .animations
                .color
                .animation(),
        ));
    link.style().refine(&p.base);
    link
}
enum PageDirection {
    Previous,
    Next,
}
fn directional_link(
    id: impl Into<ElementId>,
    url: impl Into<SharedString>,
    text: SharedString,
    direction: PageDirection,
    window: &Window,
    cx: &App,
) -> Link {
    let recipe = presentation(window, cx);
    let icon_size = button_variants(
        ButtonVariant::Ghost,
        ComponentTheme::current(cx).size.into(),
        window,
        cx,
    )
    .icon_size;
    let (label, rotation, style) = match direction {
        PageDirection::Previous => ("Go to previous page", std::f32::consts::PI, recipe.previous),
        PageDirection::Next => ("Go to next page", 0., recipe.next),
    };
    let mut link = styled_link(
        id,
        url,
        label.into(),
        ButtonVariant::Ghost,
        ComponentTheme::current(cx).size.into(),
        window,
        cx,
    );
    let icon = Icon::new(IconName::ChevronRight)
        .rotate(gpui::radians(rotation))
        .size(icon_size);
    let show_text = window.viewport_size().width >= px(640.);
    let text = show_text.then(|| text.into_any_element());
    link = match direction {
        PageDirection::Previous => link.child(icon).children(text),
        PageDirection::Next => link.children(text).child(icon),
    };
    if show_text {
        link.style().refine(&style);
    } else {
        let tokens = ComponentTheme::current(cx);
        link = link
            .w(px(tokens.size.metrics(&tokens).control_height))
            .p(px(0.));
    }
    link
}
/// Creates the previous-page link without calculating a destination or changing selection.
pub fn pagination_previous(
    id: impl Into<ElementId>,
    url: impl Into<SharedString>,
    text: impl Into<SharedString>,
    window: &Window,
    cx: &App,
) -> Link {
    directional_link(id, url, text.into(), PageDirection::Previous, window, cx)
}
/// Creates the next-page link without calculating a destination or changing selection.
pub fn pagination_next(
    id: impl Into<ElementId>,
    url: impl Into<SharedString>,
    text: impl Into<SharedString>,
    window: &Window,
    cx: &App,
) -> Link {
    directional_link(id, url, text.into(), PageDirection::Next, window, cx)
}
/// Returns decorative omitted-page artwork; it does not represent an interactive page menu.
pub fn pagination_ellipsis(window: &Window, cx: &App) -> Div {
    let p = presentation(window, cx);
    let mut element = div().child(Icon::new(IconName::MoreHorizontal).size(p.ellipsis_icon));
    element.style().refine(&p.ellipsis);
    element
}
/// A named navigation landmark that lays out caller-supplied native controls in a list.
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    label: SharedString,
    items: Vec<AnyElement>,
    style: StyleRefinement,
}
impl Pagination {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: "pagination".into(),
            items: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
    pub fn item(mut self, item: impl IntoElement) -> Self {
        self.items.push(item.into_any_element());
        self
    }
}
impl Styled for Pagination {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for Pagination {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut list = div().id("pagination-list").role(Role::List);
        list.style().refine(&presentation(window, cx).content);
        for (index, item) in self.items.into_iter().enumerate() {
            list = list.child(div().id(("page", index)).role(Role::ListItem).child(item));
        }
        let mut root = div()
            .id(self.id)
            .role(Role::Navigation)
            .aria_label(self.label)
            .mx_auto()
            .flex()
            .w_full()
            .justify_center()
            .child(list);
        root.style().refine(&self.style);
        root
    }
}
