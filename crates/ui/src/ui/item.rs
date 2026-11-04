//! Shadcn-compatible composition for dense content and settings rows.

use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    Role, StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _,
};

use crate::theme::{
    border::BORDER_DEFAULT,
    colors::G2VTheme,
    radius::{RADIUS_LG, RADIUS_SM},
    rems_from_pixels,
    sizing::{SIZE_6, SIZE_8, SIZE_10},
    spacing::{SPACE_2, SPACE_2_5, SPACE_3, SPACE_4},
    typography,
};

use super::{Separator, TextStyleExt as _};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ItemVariant {
    #[default]
    Default,
    Outline,
    Muted,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ItemSize {
    #[default]
    Default,
    Small,
    ExtraSmall,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ItemMediaVariant {
    #[default]
    Default,
    Icon,
    Image,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ItemGroupVariant {
    #[default]
    Default,
    Muted,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ItemSpacing {
    gap: gpui::Rems,
    padding_x: gpui::Rems,
    padding_y: gpui::Rems,
}

fn item_spacing(size: ItemSize) -> ItemSpacing {
    match size {
        ItemSize::Default | ItemSize::Small => ItemSpacing {
            gap: rems_from_pixels(SPACE_2_5),
            padding_x: rems_from_pixels(SPACE_3),
            padding_y: rems_from_pixels(SPACE_2_5),
        },
        ItemSize::ExtraSmall => ItemSpacing {
            gap: rems_from_pixels(SPACE_2),
            padding_x: rems_from_pixels(SPACE_2_5),
            padding_y: rems_from_pixels(SPACE_2),
        },
    }
}

fn group_gap(size: ItemSize) -> gpui::Rems {
    match size {
        ItemSize::Default => rems_from_pixels(SPACE_4),
        ItemSize::Small => rems_from_pixels(SPACE_2_5),
        ItemSize::ExtraSmall => rems_from_pixels(SPACE_2),
    }
}

fn image_size(size: ItemSize) -> gpui::Rems {
    match size {
        ItemSize::Default => SIZE_10,
        ItemSize::Small => SIZE_8,
        ItemSize::ExtraSmall => SIZE_6,
    }
}

macro_rules! item_container {
    ($name:ident) => {
        #[derive(Default, IntoElement)]
        #[must_use]
        pub struct $name {
            children: Vec<AnyElement>,
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }
    };
}

item_container!(ItemTitle);

#[derive(IntoElement)]
#[must_use]
pub struct ItemGroup {
    id: ElementId,
    size: ItemSize,
    variant: ItemGroupVariant,
    children: Vec<AnyElement>,
}

impl ItemGroup {
    pub fn new(id: impl Into<ElementId>, size: ItemSize) -> Self {
        Self {
            id: id.into(),
            size,
            variant: ItemGroupVariant::Default,
            children: Vec::new(),
        }
    }

    pub fn variant(mut self, variant: ItemGroupVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl ParentElement for ItemGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ItemGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let grouped = self.variant == ItemGroupVariant::Muted;
        div()
            .id(self.id)
            .role(Role::List)
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(if grouped {
                gpui::Rems::ZERO
            } else {
                group_gap(self.size)
            })
            .when(grouped, |group| {
                group
                    .overflow_hidden()
                    .rounded(rems_from_pixels(RADIUS_LG))
                    .bg(G2VTheme::global(cx).muted.opacity(0.5))
            })
            .children(self.children)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Item {
    id: ElementId,
    variant: ItemVariant,
    size: ItemSize,
    children: Vec<AnyElement>,
}

impl Item {
    pub fn new(id: impl Into<ElementId>, variant: ItemVariant, size: ItemSize) -> Self {
        Self {
            id: id.into(),
            variant,
            size,
            children: Vec::new(),
        }
    }
}

impl ParentElement for Item {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Item {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let spacing = item_spacing(self.size);
        let border_color = match self.variant {
            ItemVariant::Outline => theme.border,
            ItemVariant::Default | ItemVariant::Muted => theme.border.opacity(0.0),
        };

        div()
            .id(self.id)
            .role(Role::ListItem)
            .w_full()
            .min_w_0()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(spacing.gap)
            .rounded(rems_from_pixels(RADIUS_LG))
            .border(BORDER_DEFAULT)
            .border_color(border_color)
            .when(self.variant == ItemVariant::Muted, |item| {
                item.bg(theme.muted.opacity(0.5))
            })
            .px(spacing.padding_x)
            .py(spacing.padding_y)
            .text_style(typography::tokens().sm)
            .text_color(theme.foreground)
            .children(self.children)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct ItemMedia {
    variant: ItemMediaVariant,
    size: ItemSize,
    child: AnyElement,
}

impl ItemMedia {
    pub fn new(variant: ItemMediaVariant, size: ItemSize, child: impl IntoElement) -> Self {
        Self {
            variant,
            size,
            child: child.into_any_element(),
        }
    }
}

impl RenderOnce for ItemMedia {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(rems_from_pixels(SPACE_2))
            .when(self.variant == ItemMediaVariant::Image, |media| {
                media
                    .size(image_size(self.size))
                    .overflow_hidden()
                    .rounded(rems_from_pixels(RADIUS_SM))
            })
            .child(self.child)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct ItemContent {
    size: ItemSize,
    children: Vec<AnyElement>,
}

impl ItemContent {
    pub fn new(size: ItemSize) -> Self {
        Self {
            size,
            children: Vec::new(),
        }
    }
}

impl ParentElement for ItemContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ItemContent {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .items_start()
            .when(self.size != ItemSize::ExtraSmall, |content| {
                content.gap(rems_from_pixels(crate::theme::spacing::SPACE_1))
            })
            .children(self.children)
    }
}

impl RenderOnce for ItemTitle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .min_w_0()
            .flex_none()
            .flex()
            .items_center()
            .gap(rems_from_pixels(SPACE_2))
            .truncate()
            .text_style(typography::tokens().sm)
            .font_weight(typography::FONT_WEIGHT.medium)
            .children(self.children)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct ItemDescription {
    children: Vec<AnyElement>,
}

impl ItemDescription {
    pub fn new(_size: ItemSize) -> Self {
        Self {
            children: Vec::new(),
        }
    }
}

impl ParentElement for ItemDescription {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ItemDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_style = typography::tokens().xs;
        div()
            .min_w_0()
            .max_h(text_style.line_height * 2.0)
            .overflow_hidden()
            .whitespace_normal()
            .text_left()
            .text_style(text_style)
            .text_color(G2VTheme::global(cx).muted_foreground)
            .children(self.children)
    }
}

#[derive(Default, IntoElement)]
#[must_use]
pub struct ItemActions {
    full_width: bool,
    children: Vec<AnyElement>,
}

impl ItemActions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }
}

impl ParentElement for ItemActions {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ItemActions {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .when(self.full_width, gpui::Styled::w_full)
            .when(!self.full_width, gpui::Styled::flex_none)
            .flex()
            .items_center()
            .gap(rems_from_pixels(SPACE_2))
            .children(self.children)
    }
}

#[derive(IntoElement, Default)]
#[must_use]
pub struct ItemSeparator {
    inset: Option<ItemSize>,
}

impl ItemSeparator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inset(mut self, size: ItemSize) -> Self {
        self.inset = Some(size);
        self
    }
}

impl RenderOnce for ItemSeparator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .when_some(self.inset, |separator, size| {
                separator.mx(item_spacing(size).padding_x)
            })
            .child(Separator::horizontal())
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::*;

    #[test]
    fn nova_item_spacing_matches_each_supported_size() {
        assert_eq!(
            item_spacing(ItemSize::Default),
            ItemSpacing {
                gap: gpui::rems(0.625),
                padding_x: gpui::rems(0.75),
                padding_y: gpui::rems(0.625),
            }
        );
        assert_eq!(
            item_spacing(ItemSize::Small),
            item_spacing(ItemSize::Default)
        );
        assert_eq!(
            item_spacing(ItemSize::ExtraSmall),
            ItemSpacing {
                gap: gpui::rems(0.5),
                padding_x: gpui::rems(0.625),
                padding_y: gpui::rems(0.5),
            }
        );
    }

    #[test]
    fn group_density_tracks_item_size() {
        assert_eq!(group_gap(ItemSize::Default), gpui::rems(1.0));
        assert_eq!(group_gap(ItemSize::Small), gpui::rems(0.625));
        assert_eq!(group_gap(ItemSize::ExtraSmall), gpui::rems(0.5));
    }

    #[test]
    fn item_spacing_scales_with_the_window_rem_size() {
        let default = item_spacing(ItemSize::Default);
        assert_eq!(default.gap.to_pixels(px(16.0)), px(10.0));
        assert_eq!(default.padding_x.to_pixels(px(16.0)), px(12.0));
        assert_eq!(default.padding_y.to_pixels(px(16.0)), px(10.0));
        assert_eq!(default.gap.to_pixels(px(18.0)), px(11.25));
        assert_eq!(default.padding_x.to_pixels(px(18.0)), px(13.5));
        assert_eq!(default.padding_y.to_pixels(px(18.0)), px(11.25));
    }

    #[test]
    fn contained_group_and_inset_separator_are_explicit() {
        let group = ItemGroup::new("settings", ItemSize::Default).variant(ItemGroupVariant::Muted);
        let separator = ItemSeparator::new().inset(ItemSize::Default);

        assert_eq!(group.variant, ItemGroupVariant::Muted);
        assert_eq!(separator.inset, Some(ItemSize::Default));
    }

    #[test]
    fn actions_expand_only_when_requested() {
        assert!(!ItemActions::new().full_width);
        assert!(ItemActions::new().full_width().full_width);
    }

    #[test]
    fn image_media_size_tracks_item_size_and_rem_scale() {
        assert_eq!(image_size(ItemSize::Default).to_pixels(px(16.0)), px(40.0));
        assert_eq!(image_size(ItemSize::Small).to_pixels(px(16.0)), px(32.0));
        assert_eq!(
            image_size(ItemSize::ExtraSmall).to_pixels(px(16.0)),
            px(24.0)
        );
        assert_eq!(image_size(ItemSize::Default).to_pixels(px(18.0)), px(45.0));
    }
}
