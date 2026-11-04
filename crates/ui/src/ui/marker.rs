//! Compact status and separator markers matching G2V's React composition.

use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, Role, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_base::StyledExt as _;

use crate::theme::{border::BORDER_DEFAULT, colors::G2VTheme, sizing::SIZE_4, typography};

use super::TextStyleExt as _;

const MARKER_GAP: gpui::Pixels = px(8.0);
const MARKER_BORDER_PADDING: gpui::Pixels = px(8.0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MarkerVariant {
    #[default]
    Default,
    Separator,
    Border,
}

#[derive(IntoElement)]
#[must_use]
pub struct Marker {
    id: ElementId,
    variant: MarkerVariant,
    role: Option<Role>,
    style: StyleRefinement,
    icon: Option<MarkerIcon>,
    content: MarkerContent,
}

impl Marker {
    pub fn new(id: impl Into<ElementId>, variant: MarkerVariant, content: MarkerContent) -> Self {
        Self {
            id: id.into(),
            variant,
            role: None,
            style: StyleRefinement::default(),
            icon: None,
            content,
        }
    }

    pub fn icon(mut self, icon: MarkerIcon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }
}

impl Styled for Marker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Marker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let separator = self.variant == MarkerVariant::Separator;
        div()
            .id(self.id)
            .relative()
            .min_h(SIZE_4)
            .w_full()
            .flex()
            .items_center()
            .gap(MARKER_GAP)
            .text_left()
            .text_style(typography::tokens().sm)
            .text_color(theme.muted_foreground)
            .when_some(self.role, gpui::StatefulInteractiveElement::role)
            .when(self.variant == MarkerVariant::Border, |marker| {
                marker
                    .border_b(BORDER_DEFAULT)
                    .border_color(theme.border)
                    .pb(MARKER_BORDER_PADDING)
            })
            .refine_style(&self.style)
            .when(separator, |marker| {
                marker.child(separator_line(theme.border, true))
            })
            .when_some(self.icon, gpui::ParentElement::child)
            .child(self.content.render_for_variant(self.variant))
            .when(separator, |marker| {
                marker.child(separator_line(theme.border, false))
            })
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct MarkerIcon {
    child: AnyElement,
}

impl MarkerIcon {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
        }
    }
}

impl RenderOnce for MarkerIcon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .size(SIZE_4)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(self.child)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct MarkerContent {
    child: AnyElement,
}

impl MarkerContent {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
        }
    }

    fn render_for_variant(self, variant: MarkerVariant) -> AnyElement {
        div()
            .min_w_0()
            .when(variant == MarkerVariant::Separator, |content| {
                content.flex_none().text_center()
            })
            .child(self.child)
            .into_any_element()
    }
}

impl RenderOnce for MarkerContent {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.render_for_variant(MarkerVariant::Default)
    }
}

fn separator_line(color: gpui::Hsla, before_content: bool) -> impl IntoElement {
    div()
        .h(BORDER_DEFAULT)
        .min_w_0()
        .flex_1()
        .bg(color)
        .when(before_content, gpui::Styled::mr_1)
        .when(!before_content, gpui::Styled::ml_1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_matches_the_react_marker_contract() {
        assert_eq!(SIZE_4.to_pixels(px(16.0)), px(16.0));
        assert_eq!(MARKER_GAP, px(8.0));
        assert_eq!(MARKER_BORDER_PADDING, px(8.0));
        assert_eq!(BORDER_DEFAULT, px(0.5));
    }

    #[test]
    fn marker_supports_the_roles_used_by_conversation_rows() {
        let status = Marker::new(
            "status-marker",
            MarkerVariant::Default,
            MarkerContent::new("Running"),
        )
        .role(Role::Status);
        let alert = Marker::new(
            "alert-marker",
            MarkerVariant::Default,
            MarkerContent::new("Failed"),
        )
        .role(Role::Alert);

        assert_eq!(status.role, Some(Role::Status));
        assert_eq!(alert.role, Some(Role::Alert));
    }
}
