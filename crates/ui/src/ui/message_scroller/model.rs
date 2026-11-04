use std::ops::Range;

use gpui::{Pixels, SharedString};

use super::{
    MessageScrollerAlign, MessageScrollerConfig, MessageScrollerDirection, MessageScrollerItem,
    MessageScrollerSnapshot, MessageScrollerState,
};

pub struct MessageScrollerModel {
    state: MessageScrollerState,
    items: Vec<MessageScrollerItem>,
    top_content_inset: Pixels,
    bottom_content_inset: Pixels,
}

impl MessageScrollerModel {
    #[must_use]
    pub fn new(config: MessageScrollerConfig) -> Self {
        Self {
            state: MessageScrollerState::new(config),
            items: Vec::new(),
            top_content_inset: Pixels::ZERO,
            bottom_content_inset: Pixels::ZERO,
        }
    }

    pub fn replace_all(&mut self, items: Vec<MessageScrollerItem>) {
        let registrations = items
            .iter()
            .map(MessageScrollerItem::registration)
            .collect();
        self.state.set_items(registrations);
        self.state.sync_after_layout();
        self.items = items;
    }

    /// Inserts an item at the requested position.
    ///
    /// # Panics
    ///
    /// Panics when `index` is greater than the current item count.
    pub fn insert(&mut self, index: usize, item: MessageScrollerItem) {
        assert!(
            index <= self.items.len(),
            "message scroller insert index {index} exceeds item count {}",
            self.items.len()
        );
        let (id, scroll_anchor, content_version) = item.registration();
        self.state
            .insert_item(index, id, scroll_anchor, content_version);
        self.items.insert(index, item);
    }

    /// Replaces the item at the requested position.
    ///
    /// # Panics
    ///
    /// Panics when `index` does not identify an existing item.
    pub fn replace(&mut self, index: usize, item: MessageScrollerItem) {
        assert!(
            index < self.items.len(),
            "message scroller replace index {index} exceeds item count {}",
            self.items.len()
        );
        let (id, scroll_anchor, content_version) = item.registration();
        self.state
            .replace_item(index, id, scroll_anchor, content_version);
        self.items[index] = item;
    }

    /// Removes the item at the requested position.
    ///
    /// # Panics
    ///
    /// Panics when `index` does not identify an existing item.
    pub fn remove(&mut self, index: usize) {
        assert!(
            index < self.items.len(),
            "message scroller remove index {index} exceeds item count {}",
            self.items.len()
        );
        self.state.remove_item(index);
        let _ = self.items.remove(index);
    }

    pub fn refresh_item(&mut self, id: &SharedString, content_version: u64) -> bool {
        let Some(index) = self.items.iter().position(|item| &item.message_id == id) else {
            return false;
        };
        if self.items[index].content_version == content_version {
            return false;
        }
        self.items[index].content_version = content_version;
        let (id, scroll_anchor, content_version) = self.items[index].registration();
        self.state
            .replace_item(index, id, scroll_anchor, content_version);
        true
    }

    #[must_use]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn item(&self, index: usize) -> MessageScrollerItem {
        self.items[index].clone()
    }

    pub(crate) fn list(&self) -> gpui::ListState {
        self.state.list()
    }

    #[must_use]
    pub fn snapshot(&self) -> &MessageScrollerSnapshot {
        self.state.snapshot()
    }

    pub(crate) fn user_scroll_intent(&mut self, visible_range: Range<usize>) -> bool {
        self.state.user_scroll_intent(visible_range)
    }

    pub(crate) fn sync_layout(&mut self, width: Pixels, rem_size: Pixels) -> bool {
        self.state.sync_layout(width, rem_size)
    }

    pub fn sync_content_insets(&mut self, top: Pixels, bottom: Pixels) -> bool {
        let mut changed = false;
        if self.top_content_inset != top {
            self.top_content_inset = top;
            if !self.items.is_empty() {
                self.state.list().remeasure_items(0..1);
            }
            changed = true;
        }
        if self.bottom_content_inset != bottom {
            self.bottom_content_inset = bottom;
            if let Some(last_index) = self.items.len().checked_sub(1) {
                self.state
                    .list()
                    .remeasure_items(last_index..last_index + 1);
            }
            changed = true;
        }
        changed
    }

    pub(crate) fn content_insets(&self) -> (Pixels, Pixels) {
        (self.top_content_inset, self.bottom_content_inset)
    }

    pub fn set_preserve_scroll_on_prepend(&mut self, preserve: bool) {
        self.state.set_preserve_scroll_on_prepend(preserve);
    }

    pub fn scroll(&mut self, direction: MessageScrollerDirection) {
        match direction {
            MessageScrollerDirection::Start => self.state.scroll_to_start(),
            MessageScrollerDirection::End => self.state.scroll_to_end(),
        }
    }

    pub fn prepare_for_user_layout_change(&mut self) {
        self.state.prepare_for_user_layout_change();
    }

    pub fn remeasure_items(&mut self, ids: &[SharedString]) -> usize {
        self.state.remeasure_items_by_id(ids)
    }

    #[must_use]
    pub fn is_following_tail(&self) -> bool {
        self.state.is_following_tail()
    }

    pub fn scroll_to_message(&mut self, id: &SharedString, align: MessageScrollerAlign) -> bool {
        self.state.scroll_to_message(id, align)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use gpui::{IntoElement as _, div, px};

    use super::*;
    use crate::ui::MessageScrollerDefaultPosition;

    fn config() -> MessageScrollerConfig {
        MessageScrollerConfig {
            auto_scroll: true,
            default_position: MessageScrollerDefaultPosition::End,
            edge_threshold: px(8.0),
            scroll_margin: px(0.0),
            previous_item_peek: px(64.0),
        }
    }

    fn item(id: &'static str, version: u64) -> MessageScrollerItem {
        MessageScrollerItem::new(id, |_, _| div().into_any_element()).content_version(version)
    }

    #[test]
    fn replacing_one_row_preserves_every_unchanged_renderer() {
        let mut model = MessageScrollerModel::new(config());
        model.replace_all(vec![item("first", 1), item("second", 1)]);
        let first_renderer = model.items[0].render_child.clone();

        model.replace(1, item("second", 2));

        assert!(Rc::ptr_eq(&model.items[0].render_child, &first_renderer));
        assert_eq!(model.items[0].content_version, 1);
        assert_eq!(model.items[1].content_version, 2);
    }

    #[test]
    fn content_insets_invalidate_only_when_an_edge_changes() {
        let mut model = MessageScrollerModel::new(config());
        model.replace_all(vec![item("message", 1)]);

        assert!(model.sync_content_insets(px(24.0), px(96.0)));
        assert!(!model.sync_content_insets(px(24.0), px(96.0)));
        assert!(model.sync_content_insets(px(24.0), px(128.0)));
    }

    #[test]
    fn unchanged_scroll_snapshot_does_not_request_invalidation() {
        let mut model = MessageScrollerModel::new(config());

        assert!(!model.user_scroll_intent(0..0));
    }
}
