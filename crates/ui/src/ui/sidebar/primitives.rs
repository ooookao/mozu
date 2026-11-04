//! Render-once building blocks for the native sidebar compound component.

use gpui::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_base::{
    Transition, TypographyTokens, animation::cubic_bezier, input::InputState, transition,
};

use crate::theme::border::BORDER_DEFAULT;
use crate::theme::colors::G2VTheme;
use crate::theme::radius::RADIUS_LG;
use crate::theme::sizing::{SIZE_4, SIZE_5, SIZE_8};
use crate::theme::spacing::{SPACE_1, SPACE_1_5, SPACE_2};
use crate::theme::typography;
use crate::ui::{
    Button, ButtonSize, ButtonVariant, IconName, Input, Separator, Skeleton, TextStyleExt as _,
};

use super::state::SIDEBAR_TRANSITION_DURATION;
use super::state::{SidebarBorder, SidebarSide, SidebarVariant};

macro_rules! sidebar_container {
    ($name:ident, $base:expr) => {
        #[derive(IntoElement, Default)]
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

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
                ($base).children(self.children)
            }
        }
    };
}

sidebar_container!(SidebarHeader, div().flex().flex_col().gap_2().p_2());
#[derive(IntoElement, Default)]
#[must_use]
pub struct SidebarFooter {
    children: Vec<AnyElement>,
    top_border: bool,
}

impl SidebarFooter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn top_border(mut self) -> Self {
        self.top_border = true;
        self
    }
}

impl ParentElement for SidebarFooter {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for SidebarFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .when(self.top_border, |footer| {
                footer
                    .border_t(BORDER_DEFAULT)
                    .border_color(G2VTheme::global(cx).sidebar_border)
            })
            .children(self.children)
    }
}
sidebar_container!(
    SidebarContent,
    div().min_h_0().flex_1().flex().flex_col().overflow_hidden()
);
sidebar_container!(
    SidebarGroup,
    div().relative().w_full().min_w_0().flex().flex_col().p_2()
);
sidebar_container!(
    SidebarMenu,
    div().w_full().min_w_0().flex().flex_col().gap_0p5()
);
sidebar_container!(SidebarMenuItem, div().relative().w_full());
sidebar_container!(SidebarGroupContent, div().w_full());

sidebar_container!(SidebarMenuSubItem, div().relative().w_full());

#[derive(IntoElement)]
#[must_use]
pub struct SidebarInset {
    sidebar_open: bool,
    children: Vec<AnyElement>,
}

impl SidebarInset {
    pub fn new(sidebar_open: bool) -> Self {
        Self {
            sidebar_open,
            children: Vec::new(),
        }
    }
}

impl ParentElement for SidebarInset {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for SidebarInset {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let target_border = if self.sidebar_open {
            theme.border
        } else {
            theme.border.opacity(0.0)
        };
        let border = transition(
            ("sidebar-inset", "border"),
            target_border,
            Transition::new(SIDEBAR_TRANSITION_DURATION).ease(cubic_bezier(0.34, 1.15, 0.5, 1.0)),
            window,
            cx,
        );
        div()
            .relative()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .border_l(BORDER_DEFAULT)
            .border_color(border)
            .bg(theme.surface)
            .text_color(theme.surface_foreground)
            .children(self.children)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SidebarGroupLabel {
    label: SharedString,
    collapsed: bool,
}

impl SidebarGroupLabel {
    pub fn new(label: impl Into<SharedString>, collapsed: bool) -> Self {
        Self {
            label: label.into(),
            collapsed,
        }
    }
}

impl RenderOnce for SidebarGroupLabel {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let typography = typography::tokens();
        div()
            .h(SIZE_8)
            .flex()
            .items_center()
            .px_2()
            .text_style(typography.xs)
            .text_color(theme.sidebar_foreground.opacity(0.7))
            .when(self.collapsed, |label| {
                label.h_0().opacity(0.0).overflow_hidden()
            })
            .child(self.label)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SidebarSeparator;

impl SidebarSeparator {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SidebarSeparator {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for SidebarSeparator {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().mx_2().child(Separator::horizontal())
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SidebarInput {
    state: Entity<InputState>,
}

impl SidebarInput {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for SidebarInput {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Input::new(&self.state).accessibility_label("Sidebar input")
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SidebarGroupAction {
    id: gpui::ElementId,
    accessibility_label: SharedString,
    disabled: bool,
    icon: IconName,
    on_click: super::ClickHandler,
}

impl SidebarGroupAction {
    pub fn new(
        id: impl Into<gpui::ElementId>,
        accessibility_label: impl Into<SharedString>,
        icon: IconName,
        on_click: super::ClickHandler,
    ) -> Self {
        Self {
            id: id.into(),
            accessibility_label: accessibility_label.into(),
            disabled: false,
            icon,
            on_click,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for SidebarGroupAction {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Button::new(self.id)
            .variant(ButtonVariant::GhostSecondarySubtle)
            .size(ButtonSize::IconExtraSmall)
            .accessibility_label(self.accessibility_label.clone())
            .tooltip(self.accessibility_label)
            .disabled(self.disabled)
            .on_click_shared(self.on_click)
            .icon(self.icon)
            .icon_size(SIZE_4)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SidebarTrigger {
    on_click: super::ClickHandler,
    variant: ButtonVariant,
}

impl SidebarTrigger {
    pub fn new(on_click: super::ClickHandler) -> Self {
        Self {
            on_click,
            variant: ButtonVariant::Ghost,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl RenderOnce for SidebarTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        Button::new("sidebar-trigger")
            .variant(self.variant)
            .size(ButtonSize::IconSmall)
            .accessibility_label("Toggle Sidebar")
            .tooltip("Toggle Sidebar")
            .on_click_shared(self.on_click)
            .icon(IconName::LayoutAlignLeft)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SidebarRail {
    side: SidebarSide,
    on_click: super::ClickHandler,
}

impl SidebarRail {
    pub fn new(side: SidebarSide, on_click: super::ClickHandler) -> Self {
        Self { side, on_click }
    }
}

impl RenderOnce for SidebarRail {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let border = G2VTheme::global(cx).sidebar_border;
        div()
            .id("sidebar-rail")
            .absolute()
            .top_0()
            .bottom_0()
            .w(SPACE_1)
            .cursor(gpui::CursorStyle::ResizeLeftRight)
            .when(self.side == SidebarSide::Left, gpui::Styled::right_0)
            .when(self.side == SidebarSide::Right, gpui::Styled::left_0)
            .hover(move |rail| rail.bg(border))
            .on_click(move |event, window, cx| (self.on_click)(event, window, cx))
    }
}

pub type SidebarMenuAction = SidebarGroupAction;

#[derive(IntoElement)]
#[must_use]
pub struct SidebarMenuBadge {
    label: SharedString,
}

impl SidebarMenuBadge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl RenderOnce for SidebarMenuBadge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let typography = typography::tokens();
        div()
            .absolute()
            .right(SPACE_2)
            .top(SPACE_1_5)
            .h(SIZE_5)
            .min_w(SIZE_5)
            .px_1()
            .flex()
            .items_center()
            .justify_center()
            .rounded(RADIUS_LG)
            .text_style(typography.xs)
            .text_color(theme.sidebar_foreground)
            .child(self.label)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct SidebarMenuSkeleton {
    width: Pixels,
    show_icon: bool,
}

impl SidebarMenuSkeleton {
    pub fn new(width: Pixels, show_icon: bool) -> Self {
        Self { width, show_icon }
    }
}

impl RenderOnce for SidebarMenuSkeleton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .h(SIZE_8)
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .when(self.show_icon, |row| {
                row.child(Skeleton::new().size(SIZE_4))
            })
            .child(Skeleton::new().h(SIZE_4).w(self.width))
    }
}

#[derive(IntoElement, Default)]
#[must_use]
pub struct SidebarMenuSub {
    children: Vec<AnyElement>,
}

impl SidebarMenuSub {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for SidebarMenuSub {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for SidebarMenuSub {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let border = G2VTheme::global(cx).sidebar_border;
        div()
            .mx_3p5()
            .flex()
            .min_w_0()
            .flex_col()
            .gap_0p5()
            .border_l(BORDER_DEFAULT)
            .border_color(border)
            .px_2p5()
            .py_0p5()
            .children(self.children)
    }
}

pub(super) struct SidebarSurfaceConfig {
    pub(super) side: SidebarSide,
    pub(super) variant: SidebarVariant,
    pub(super) width: Pixels,
    pub(super) opacity: f32,
    pub(super) border: SidebarBorder,
}

pub(super) fn sidebar_surface(
    config: &SidebarSurfaceConfig,
    theme: &G2VTheme,
    typography: &TypographyTokens,
    children: Vec<AnyElement>,
) -> impl IntoElement + use<> {
    let side = config.side;
    let variant = config.variant;
    let width = config.width;
    let opacity = config.opacity;
    let border = config.border;

    div()
        .w(width)
        .h_full()
        .flex_none()
        .opacity(opacity)
        .p(if variant == SidebarVariant::Sidebar {
            px(0.0)
        } else {
            SPACE_2
        })
        .text_style(typography.sm)
        .text_color(theme.sidebar_foreground)
        .when(
            side == SidebarSide::Left
                && variant == SidebarVariant::Sidebar
                && border == SidebarBorder::Divider,
            |surface| {
                surface
                    .border_r(BORDER_DEFAULT)
                    .border_color(theme.sidebar_border)
            },
        )
        .when(
            side == SidebarSide::Right
                && variant == SidebarVariant::Sidebar
                && border == SidebarBorder::Divider,
            |surface| {
                surface
                    .border_l(BORDER_DEFAULT)
                    .border_color(theme.sidebar_border)
            },
        )
        .child(
            div()
                .relative()
                .size_full()
                .flex()
                .flex_col()
                .overflow_hidden()
                .bg(theme.sidebar)
                .when(variant != SidebarVariant::Sidebar, |inner| {
                    inner.rounded(RADIUS_LG)
                })
                .children(children),
        )
}
