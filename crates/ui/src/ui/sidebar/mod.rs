//! Native counterpart to the shared shadcn sidebar compound component.

mod conversation_menu;
mod hover_card;
mod menu_buttons;
mod primitives;
mod project_menu;
mod state;

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _,
};
use gpui_base::{Transition, animation::cubic_bezier, transition};

use crate::theme::colors::G2VTheme;
use crate::theme::motion::cubic_bezier_with_overshoot;
use crate::theme::typography;
use crate::ui::floating_surface;
use crate::ui::modal_backdrop::standard_modal_backdrop;

pub use crate::ui::CollapsibleContent as SidebarCollapsibleContent;
pub use conversation_menu::{
    SidebarConversationActions, SidebarConversationDetails, SidebarConversationMenu,
};
pub use hover_card::SidebarHoverCard;
pub use menu_buttons::{
    SidebarConversationMenuButton, SidebarMenuButton, SidebarProjectButton,
    SidebarProjectMenuButton,
};
pub use primitives::{
    SidebarContent, SidebarFooter, SidebarGroup, SidebarGroupAction, SidebarGroupContent,
    SidebarGroupLabel, SidebarHeader, SidebarInput, SidebarInset, SidebarMenu, SidebarMenuAction,
    SidebarMenuBadge, SidebarMenuItem, SidebarMenuSkeleton, SidebarMenuSub, SidebarMenuSubItem,
    SidebarRail, SidebarSeparator, SidebarTrigger,
};
pub use project_menu::{SidebarProjectActions, SidebarProjectDetails, SidebarProjectMenu};

gpui::actions!(sidebar, [ToggleSidebar]);

pub(crate) fn init(cx: &mut App) {
    #[cfg(target_os = "macos")]
    cx.bind_keys([gpui::KeyBinding::new("cmd-b", ToggleSidebar, None)]);
    #[cfg(not(target_os = "macos"))]
    cx.bind_keys([gpui::KeyBinding::new("ctrl-b", ToggleSidebar, None)]);
}
pub use state::{
    SIDEBAR_TRANSITION_DURATION, SIDEBAR_WIDTH, SIDEBAR_WIDTH_ICON, SIDEBAR_WIDTH_MOBILE,
    SidebarBorder, SidebarCollapsible, SidebarSide, SidebarState, SidebarVariant,
};

pub(crate) type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub type SidebarProvider = SidebarState;

#[derive(IntoElement)]
#[must_use]
pub struct Sidebar {
    side: SidebarSide,
    variant: SidebarVariant,
    collapsible: SidebarCollapsible,
    open: bool,
    compact: bool,
    border: SidebarBorder,
    on_mobile_close: Option<ClickHandler>,
    children: Vec<AnyElement>,
}

impl Sidebar {
    pub fn new(
        side: SidebarSide,
        variant: SidebarVariant,
        collapsible: SidebarCollapsible,
        open: bool,
        compact: bool,
    ) -> Self {
        Self {
            side,
            variant,
            collapsible,
            open,
            compact,
            border: SidebarBorder::Divider,
            on_mobile_close: None,
            children: Vec::new(),
        }
    }

    pub fn border(mut self, border: SidebarBorder) -> Self {
        self.border = border;
        self
    }

    pub fn on_mobile_close(mut self, handler: ClickHandler) -> Self {
        self.on_mobile_close = Some(handler);
        self
    }
}

impl ParentElement for Sidebar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Sidebar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let typography = typography::tokens();
        let target_width = state::width(self.open, self.compact, self.collapsible);
        let target_opacity = if self.open { 1.0 } else { 0.0 };
        let width_transition = || {
            Transition::new(SIDEBAR_TRANSITION_DURATION)
                .ease(cubic_bezier_with_overshoot(0.34, 1.15, 0.5, 1.0))
        };
        let visual_transition = || {
            Transition::new(SIDEBAR_TRANSITION_DURATION).ease(cubic_bezier(0.34, 1.15, 0.5, 1.0))
        };
        let width = if self.compact {
            target_width
        } else {
            let animated_width = transition(
                ("application-sidebar", "width"),
                target_width,
                width_transition(),
                window,
                cx,
            );
            if animated_width < gpui::Pixels::ZERO {
                gpui::Pixels::ZERO
            } else {
                animated_width
            }
        };
        let opacity = if self.compact {
            target_opacity
        } else {
            transition(
                ("application-sidebar", "opacity"),
                target_opacity,
                visual_transition(),
                window,
                cx,
            )
        };
        let surface = primitives::sidebar_surface(
            &primitives::SidebarSurfaceConfig {
                side: self.side,
                variant: self.variant,
                width,
                opacity,
                border: self.border,
            },
            &theme,
            &typography,
            self.children,
        );

        if !self.compact {
            return surface.into_any_element();
        }
        if !self.open {
            return div().w_0().h_full().into_any_element();
        }

        let overlay = standard_modal_backdrop(window.viewport_size())
            .id("sidebar-mobile-overlay")
            .when_some(self.on_mobile_close, |overlay, close| {
                overlay.on_click(move |event, window, cx| close(event, window, cx))
            });
        floating_surface(
            div().absolute().inset_0().child(overlay).child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .bottom_0()
                    .occlude()
                    .child(surface),
            ),
            100,
        )
        .into_any_element()
    }
}
