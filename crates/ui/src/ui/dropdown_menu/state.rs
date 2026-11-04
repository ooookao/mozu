//! Shared state for one dropdown menu surface.

use std::time::{Duration, Instant};

use gpui::{Context, Entity, SharedString};

#[derive(Default)]
pub(super) struct DropdownMenuState {
    pub(super) active: Option<SharedString>,
    pub(super) open_submenu: Option<SharedString>,
    pub(super) pending_submenu: Option<SharedString>,
    pub(super) hover_generation: usize,
    pub(super) typeahead: String,
    pub(super) last_typeahead: Option<Instant>,
}

impl DropdownMenuState {
    pub(super) fn cancel_submenu_interaction(&mut self) {
        self.hover_generation += 1;
        self.pending_submenu = None;
        self.open_submenu = None;
    }
}

#[derive(Default)]
pub(super) struct MenuInteractionTreeState {
    hovered_surfaces: usize,
    leave_generation: usize,
}

impl MenuInteractionTreeState {
    pub(super) fn surface_hover_changed(
        &mut self,
        hovered: bool,
        root: Entity<DropdownMenuState>,
        cx: &mut Context<Self>,
    ) {
        let Some(generation) = self.record_surface_hover(hovered) else {
            return;
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            let _ = this.update(cx, |tree, cx| {
                if !tree.should_clear_after_leave(generation) {
                    return;
                }
                root.update(cx, |state, cx| {
                    state.active = None;
                    state.cancel_submenu_interaction();
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn record_surface_hover(&mut self, hovered: bool) -> Option<usize> {
        self.leave_generation += 1;
        if hovered {
            self.hovered_surfaces += 1;
            return None;
        }
        self.hovered_surfaces = self.hovered_surfaces.saturating_sub(1);
        Some(self.leave_generation)
    }

    fn should_clear_after_leave(&self, generation: usize) -> bool {
        self.leave_generation == generation && self.hovered_surfaces == 0
    }
}

#[cfg(test)]
mod tests {
    use gpui::AppContext as _;

    use super::*;

    #[test]
    fn crossing_from_parent_surface_to_submenu_preserves_highlight() {
        let mut tree = MenuInteractionTreeState::default();
        tree.record_surface_hover(true);
        let parent_leave = tree.record_surface_hover(false).unwrap();
        tree.record_surface_hover(true);

        assert!(!tree.should_clear_after_leave(parent_leave));
    }

    #[test]
    fn leaving_the_entire_menu_tree_clears_highlight() {
        let mut tree = MenuInteractionTreeState::default();
        tree.record_surface_hover(true);
        let leave = tree.record_surface_hover(false).unwrap();

        assert!(tree.should_clear_after_leave(leave));
    }

    #[gpui::test]
    async fn leaving_the_tree_clears_the_explicit_root_state(cx: &mut gpui::TestAppContext) {
        let root = cx.new(|_| DropdownMenuState {
            active: Some("active".into()),
            open_submenu: Some("submenu".into()),
            ..DropdownMenuState::default()
        });
        let tree = cx.new(|_| MenuInteractionTreeState::default());

        tree.update(cx, |tree, cx| {
            tree.surface_hover_changed(true, root.clone(), cx);
            tree.surface_hover_changed(false, root.clone(), cx);
        });
        cx.background_executor
            .timer(Duration::from_millis(17))
            .await;

        let root = root.read_with(cx, |root, _| {
            (root.active.clone(), root.open_submenu.clone())
        });
        assert_eq!(root, (None, None));
    }
}
