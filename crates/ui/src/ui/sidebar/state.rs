//! State and variants shared by native sidebar primitives.

use std::time::Duration;

use gpui::Pixels;
use gpui::px;

pub const SIDEBAR_WIDTH: Pixels = px(272.0);
pub const SIDEBAR_WIDTH_MOBILE: Pixels = px(288.0);
pub const SIDEBAR_WIDTH_ICON: Pixels = px(48.0);
pub const SIDEBAR_TRANSITION_DURATION: Duration = Duration::from_millis(400);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SidebarSide {
    #[default]
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SidebarVariant {
    #[default]
    Sidebar,
    Floating,
    Inset,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SidebarBorder {
    None,
    #[default]
    Divider,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SidebarCollapsible {
    #[default]
    Offcanvas,
    Icon,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SidebarState {
    open: bool,
    open_mobile: bool,
}

impl SidebarState {
    #[must_use]
    pub const fn new(open: bool) -> Self {
        Self {
            open,
            open_mobile: false,
        }
    }

    #[must_use]
    pub const fn is_open(self) -> bool {
        self.open
    }

    #[must_use]
    pub const fn is_mobile_open(self) -> bool {
        self.open_mobile
    }

    #[must_use]
    pub const fn is_collapsed(self) -> bool {
        !self.open
    }

    pub fn toggle_desktop(&mut self) {
        self.open = !self.open;
    }

    pub fn toggle_mobile(&mut self) {
        self.open_mobile = !self.open_mobile;
    }

    pub fn close_mobile(&mut self) {
        self.open_mobile = false;
    }
}

impl Default for SidebarState {
    fn default() -> Self {
        Self::new(true)
    }
}

pub(super) fn width(open: bool, compact: bool, collapsible: SidebarCollapsible) -> Pixels {
    if compact {
        return if open {
            SIDEBAR_WIDTH_MOBILE
        } else {
            Pixels::ZERO
        };
    }
    if open || collapsible == SidebarCollapsible::None {
        SIDEBAR_WIDTH
    } else {
        match collapsible {
            SidebarCollapsible::Offcanvas => Pixels::ZERO,
            SidebarCollapsible::Icon => SIDEBAR_WIDTH_ICON,
            SidebarCollapsible::None => SIDEBAR_WIDTH,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_match_the_shadcn_sidebar_contract() {
        assert_eq!(SIDEBAR_WIDTH, px(272.0));
        assert_eq!(SIDEBAR_WIDTH_MOBILE, px(288.0));
        assert_eq!(SIDEBAR_WIDTH_ICON, px(48.0));
        assert_eq!(SIDEBAR_TRANSITION_DURATION, Duration::from_millis(400));
        assert_eq!(width(false, false, SidebarCollapsible::Offcanvas), px(0.0));
        assert_eq!(width(false, false, SidebarCollapsible::Icon), px(48.0));
        assert_eq!(width(false, true, SidebarCollapsible::None), px(0.0));
    }

    #[test]
    fn desktop_and_mobile_visibility_are_independent() {
        let mut state = SidebarState::new(true);
        state.toggle_mobile();
        assert!(state.is_open());
        assert!(state.is_mobile_open());

        state.toggle_desktop();
        assert!(state.is_collapsed());
        assert!(state.is_mobile_open());
    }
}
