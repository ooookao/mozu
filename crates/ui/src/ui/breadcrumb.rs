//! shadcn-compatible breadcrumb navigation built on gpui-base links.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, div, prelude::FluentBuilder as _,
};
use gpui_base::Link as LinkBehavior;

use crate::theme::{
    colors::G2VTheme,
    sizing::{SIZE_3_5, SIZE_4, SIZE_5},
    spacing::{SPACE_1, SPACE_1_5},
    typography,
};

use super::{Icon, IconName, TextStyleExt as _};

type ActivationHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
const SEPARATOR_ICON_SIZE: gpui::Rems = SIZE_3_5;
const ELLIPSIS_SIZE: gpui::Rems = SIZE_5;
const ELLIPSIS_ICON_SIZE: gpui::Rems = SIZE_4;

enum BreadcrumbItemKind {
    Link(ActivationHandler),
    Label,
    Page,
    Ellipsis,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BreadcrumbDensity {
    #[default]
    Default,
    Compact,
}

pub struct BreadcrumbItem {
    id: SharedString,
    label: SharedString,
    kind: BreadcrumbItemKind,
}

impl BreadcrumbItem {
    pub fn link(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        on_activate: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: BreadcrumbItemKind::Link(Rc::new(on_activate)),
        }
    }

    pub fn page(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: BreadcrumbItemKind::Page,
        }
    }

    pub fn label(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: BreadcrumbItemKind::Label,
        }
    }

    #[must_use]
    pub fn ellipsis(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: SharedString::from("More"),
            kind: BreadcrumbItemKind::Ellipsis,
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Breadcrumb {
    id: SharedString,
    items: Vec<BreadcrumbItem>,
    density: BreadcrumbDensity,
}

impl Breadcrumb {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            density: BreadcrumbDensity::Default,
        }
    }

    pub fn density(mut self, density: BreadcrumbDensity) -> Self {
        self.density = density;
        self
    }

    pub fn item(mut self, item: BreadcrumbItem) -> Self {
        self.items.push(item);
        self
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let mut children = Vec::with_capacity(self.items.len().saturating_mul(2));
        let item_count = self.items.len();
        for (index, item) in self.items.into_iter().enumerate() {
            let item_id = item.id.clone();
            children.push(render_item(&self.id, item, self.density, theme));
            if index + 1 < item_count {
                children.push(
                    div()
                        .id((
                            ElementId::from(self.id.clone()),
                            format!("separator-after-{item_id}"),
                        ))
                        .role(Role::ListItem)
                        .child(Icon::new(IconName::ChevronRight, SEPARATOR_ICON_SIZE))
                        .into_any_element(),
                );
            }
        }

        let compact = self.density == BreadcrumbDensity::Compact;
        div()
            .id(self.id.clone())
            .role(Role::Navigation)
            .aria_label("breadcrumb")
            .min_w_0()
            .when(compact, |breadcrumb| breadcrumb.flex_1().overflow_hidden())
            .child(
                div()
                    .id(format!("{}-list", self.id))
                    .role(Role::List)
                    .flex()
                    .when(!compact, gpui::Styled::flex_wrap)
                    .when(compact, |list| {
                        list.min_w_0().overflow_hidden().whitespace_nowrap()
                    })
                    .items_center()
                    .gap(if compact { SPACE_1 } else { SPACE_1_5 })
                    .text_style(if compact {
                        typography::tokens().xs
                    } else {
                        typography::tokens().sm
                    })
                    .text_color(theme.muted_foreground)
                    .children(children),
            )
    }
}

fn render_item(
    breadcrumb_id: &SharedString,
    item: BreadcrumbItem,
    density: BreadcrumbDensity,
    theme: &G2VTheme,
) -> AnyElement {
    let compact = density == BreadcrumbDensity::Compact;
    let item_id = item.id.clone();
    let content = match item.kind {
        BreadcrumbItemKind::Link(on_activate) => {
            let foreground = theme.foreground;
            LinkBehavior::new((ElementId::from(breadcrumb_id.clone()), item_id.clone()))
                .on_activate(move |event, window, cx| on_activate(event, window, cx))
                .hover(move |style| style.text_color(foreground))
                .child(item.label)
                .into_any_element()
        }
        BreadcrumbItemKind::Label => div().child(item.label).into_any_element(),
        BreadcrumbItemKind::Page => div()
            .id((
                ElementId::from(breadcrumb_id.clone()),
                format!("page-{item_id}"),
            ))
            .role(Role::Link)
            .aria_label(format!("{} (current page)", item.label))
            .when(compact, |page| {
                page.min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
            })
            .text_color(theme.foreground)
            .child(item.label)
            .into_any_element(),
        BreadcrumbItemKind::Ellipsis => div()
            .id((
                ElementId::from(breadcrumb_id.clone()),
                format!("ellipsis-{item_id}"),
            ))
            .role(Role::ListItem)
            .aria_label("More")
            .size(ELLIPSIS_SIZE)
            .flex()
            .items_center()
            .justify_center()
            .child(Icon::new(IconName::Dots, ELLIPSIS_ICON_SIZE))
            .into_any_element(),
    };

    div()
        .id((
            ElementId::from(breadcrumb_id.clone()),
            format!("item-{item_id}"),
        ))
        .role(Role::ListItem)
        .flex()
        .when(compact, gpui::Styled::min_w_0)
        .items_center()
        .gap_1()
        .child(content)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn geometry_matches_the_react_breadcrumb_contract() {
        assert_eq!(SPACE_1_5, px(6.0));
        assert_eq!(SEPARATOR_ICON_SIZE.to_pixels(px(16.0)), px(14.0));
        assert_eq!(ELLIPSIS_SIZE.to_pixels(px(16.0)), px(20.0));
        assert_eq!(ELLIPSIS_ICON_SIZE.to_pixels(px(16.0)), px(16.0));
    }

    #[test]
    fn compact_density_matches_the_workspace_file_header() {
        assert_eq!(SPACE_1, px(4.0));
        assert_eq!(typography::tokens().xs.size, px(12.0));
    }

    #[test]
    fn item_identity_survives_reordering() {
        let first = BreadcrumbItem::label("workspace", "Workspace");
        let second = BreadcrumbItem::page("src", "src");
        let reordered = [second, first];

        assert_eq!(reordered[0].id.as_ref(), "src");
        assert_eq!(reordered[1].id.as_ref(), "workspace");
    }
}
