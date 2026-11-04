//! Sidebar-owned hover-card composition and layout defaults.

use std::time::Duration;

use gpui::{
    AnyElement, App, ElementId, IntoElement, RenderOnce, StyleRefinement, Styled as _, Window,
};

use crate::theme::{rems_from_pixels, spacing::SPACE_3};
use crate::ui::{HoverCard, HoverCardAlign, HoverCardSide};

const SIDEBAR_HOVER_CARD_OPEN_DELAY: Duration = Duration::from_millis(750);
const SIDEBAR_HOVER_CARD_CLOSE_DELAY: Duration = Duration::ZERO;

#[derive(IntoElement)]
#[must_use]
pub struct SidebarHoverCard {
    hover_card: HoverCard,
}

impl SidebarHoverCard {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            hover_card: HoverCard::new(id)
                .open_delay(SIDEBAR_HOVER_CARD_OPEN_DELAY)
                .close_delay(SIDEBAR_HOVER_CARD_CLOSE_DELAY)
                .side(HoverCardSide::Right)
                .align(HoverCardAlign::Start)
                .side_offset(rems_from_pixels(SPACE_3))
                .trigger_style(StyleRefinement::default().w_full()),
        }
    }

    pub fn trigger(mut self, trigger: impl IntoElement + 'static) -> Self {
        self.hover_card = self.hover_card.trigger(trigger);
        self
    }

    /// Uses the same compact outer inset as searchable popover menus.
    pub fn menu_surface(mut self) -> Self {
        self.hover_card = self.hover_card.p_1();
        self
    }

    pub fn content(mut self, content: impl Fn(&App) -> AnyElement + 'static) -> Self {
        self.hover_card = self.hover_card.content(content);
        self
    }
}

impl RenderOnce for SidebarHoverCard {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.hover_card
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidebar_hover_card_opens_deliberately_and_closes_immediately() {
        assert_eq!(SIDEBAR_HOVER_CARD_OPEN_DELAY, Duration::from_millis(750));
        assert_eq!(SIDEBAR_HOVER_CARD_CLOSE_DELAY, Duration::ZERO);
    }
}
