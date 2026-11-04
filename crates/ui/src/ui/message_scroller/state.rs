use std::ops::Range;

use gpui::{FollowMode, ListAlignment, ListOffset, ListState, Pixels, SharedString, px};

use crate::ui::ListLayoutMeasurementState;

const POSITION_EPSILON: f32 = 0.5;
const LIST_OVERDRAW: Pixels = px(2048.0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageScrollerDefaultPosition {
    Start,
    #[default]
    End,
    LastAnchor,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageScrollerDirection {
    Start,
    #[default]
    End,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageScrollerAlign {
    #[default]
    Start,
    Center,
    End,
    Nearest,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MessageScrollerConfig {
    pub auto_scroll: bool,
    pub default_position: MessageScrollerDefaultPosition,
    pub edge_threshold: Pixels,
    pub scroll_margin: Pixels,
    pub previous_item_peek: Pixels,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MessageScrollerSnapshot {
    pub start: bool,
    pub end: bool,
    pub current_anchor_id: Option<SharedString>,
    pub visible_message_ids: Vec<SharedString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ItemRegistration {
    id: SharedString,
    scroll_anchor: bool,
    content_version: u64,
}

pub struct MessageScrollerState {
    config: MessageScrollerConfig,
    list: ListState,
    items: Vec<ItemRegistration>,
    initialized: bool,
    preserve_scroll_on_prepend: bool,
    pending_initial_anchor: Option<usize>,
    pending_message_scroll: Option<(usize, MessageScrollerAlign)>,
    layout_measurement: ListLayoutMeasurementState,
    snapshot: MessageScrollerSnapshot,
}

impl MessageScrollerState {
    #[must_use]
    pub fn new(config: MessageScrollerConfig) -> Self {
        let list = ListState::new(0, ListAlignment::Top, LIST_OVERDRAW);
        list.set_follow_mode(if config.auto_scroll {
            FollowMode::Tail
        } else {
            FollowMode::Normal
        });
        Self {
            config,
            list,
            items: Vec::new(),
            initialized: false,
            preserve_scroll_on_prepend: true,
            pending_initial_anchor: None,
            pending_message_scroll: None,
            layout_measurement: ListLayoutMeasurementState::default(),
            snapshot: MessageScrollerSnapshot::default(),
        }
    }

    #[must_use]
    pub fn list(&self) -> ListState {
        self.list.clone()
    }

    #[must_use]
    pub fn snapshot(&self) -> &MessageScrollerSnapshot {
        &self.snapshot
    }

    pub fn set_preserve_scroll_on_prepend(&mut self, preserve: bool) {
        self.preserve_scroll_on_prepend = preserve;
    }

    pub(crate) fn sync_layout(&mut self, width: Pixels, rem_size: Pixels) -> bool {
        self.layout_measurement
            .sync(&self.list, self.items.len(), width, rem_size)
    }

    pub fn set_items(&mut self, items: Vec<(SharedString, bool, u64)>) -> bool {
        let next = items
            .into_iter()
            .map(|(id, scroll_anchor, content_version)| ItemRegistration {
                id,
                scroll_anchor,
                content_version,
            })
            .collect::<Vec<_>>();
        if next == self.items {
            return false;
        }

        let old_len = self.items.len();
        let old_scroll = self.list.logical_scroll_top();
        let prefix = common_prefix_len(&self.items, &next);
        let suffix = common_suffix_len(&self.items[prefix..], &next[prefix..]);
        let is_prepend = old_len > 0 && prefix == 0 && suffix == old_len;
        let old_range = prefix..old_len - suffix;
        let inserted = next.len() - prefix - suffix;
        if old_range.len() == inserted {
            self.list.remeasure_items(old_range);
        } else {
            self.list.splice(old_range, inserted);
        }
        if is_prepend && !self.preserve_scroll_on_prepend {
            self.list.scroll_to(old_scroll);
        }
        self.items = next;

        if !self.initialized && !self.items.is_empty() {
            self.initialized = true;
            self.apply_default_position();
        } else if self.config.auto_scroll && self.list.is_following_tail() {
            self.list.scroll_to_end();
        }
        true
    }

    pub(crate) fn insert_item(
        &mut self,
        index: usize,
        id: SharedString,
        scroll_anchor: bool,
        content_version: u64,
    ) {
        assert!(
            index <= self.items.len(),
            "message scroller insert index {index} exceeds item count {}",
            self.items.len()
        );
        self.items.insert(
            index,
            ItemRegistration {
                id,
                scroll_anchor,
                content_version,
            },
        );
        self.list.splice(index..index, 1);
        self.finish_content_update();
    }

    pub(crate) fn replace_item(
        &mut self,
        index: usize,
        id: SharedString,
        scroll_anchor: bool,
        content_version: u64,
    ) {
        assert!(
            index < self.items.len(),
            "message scroller replace index {index} exceeds item count {}",
            self.items.len()
        );
        let replacement = ItemRegistration {
            id,
            scroll_anchor,
            content_version,
        };
        if self.items[index] == replacement {
            return;
        }
        self.items[index] = replacement;
        self.list.remeasure_items(index..index + 1);
        self.finish_content_update();
    }

    pub(crate) fn remove_item(&mut self, index: usize) {
        assert!(
            index < self.items.len(),
            "message scroller remove index {index} exceeds item count {}",
            self.items.len()
        );
        self.items.remove(index);
        self.list.splice(index..index + 1, 0);
        self.finish_content_update();
    }

    fn finish_content_update(&mut self) {
        if !self.initialized && !self.items.is_empty() {
            self.initialized = true;
            self.apply_default_position();
        } else if self.config.auto_scroll && self.list.is_following_tail() {
            self.list.scroll_to_end();
        }
        self.sync_after_layout();
    }

    pub fn user_scroll_intent(&mut self, visible_range: Range<usize>) -> bool {
        let previous_controls = (self.snapshot.start, self.snapshot.end);
        self.recompute_snapshot_in_range(visible_range);
        previous_controls != (self.snapshot.start, self.snapshot.end)
    }

    pub fn sync_after_layout(&mut self) -> bool {
        let anchor_changed = self.apply_pending_initial_anchor();
        let message_scroll_changed = self.apply_pending_message_scroll();
        self.recompute_snapshot() || anchor_changed || message_scroll_changed
    }

    pub fn scroll_to_start(&mut self) {
        self.list.scroll_to(ListOffset {
            item_ix: 0,
            offset_in_item: px(0.0),
        });
        self.recompute_snapshot();
    }

    pub fn scroll_to_end(&mut self) {
        if self.config.auto_scroll {
            self.list.set_follow_mode(FollowMode::Tail);
        }
        self.list.scroll_to_end();
        self.recompute_snapshot();
    }

    pub fn prepare_for_user_layout_change(&mut self) {
        self.list.set_follow_mode(FollowMode::Normal);
    }

    pub(crate) fn remeasure_items_by_id(&mut self, ids: &[SharedString]) -> usize {
        let mut indices = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| ids.contains(&item.id).then_some(index))
            .collect::<Vec<_>>();
        indices.sort_unstable();
        indices.dedup();
        for index in &indices {
            self.list.remeasure_items(*index..*index + 1);
        }
        indices.len()
    }

    #[must_use]
    pub fn is_following_tail(&self) -> bool {
        self.list.is_following_tail()
    }

    pub fn scroll_to_message(&mut self, id: &SharedString, align: MessageScrollerAlign) -> bool {
        let Some(index) = self.items.iter().position(|item| &item.id == id) else {
            return false;
        };
        if self.list.bounds_for_item(index).is_none() {
            self.list.scroll_to(ListOffset {
                item_ix: index,
                offset_in_item: px(0.0),
            });
            self.pending_message_scroll = Some((index, align));
            return true;
        }
        self.align_item(index, align);
        self.recompute_snapshot();
        true
    }

    fn align_item(&self, index: usize, align: MessageScrollerAlign) {
        let Some(bounds) = self.list.bounds_for_item(index) else {
            return;
        };
        let viewport = self.list.viewport_bounds();
        let target = match align {
            MessageScrollerAlign::Start => {
                bounds.top()
                    - viewport.top()
                    - self.config.scroll_margin
                    - self.config.previous_item_peek
            }
            MessageScrollerAlign::Center => {
                bounds.top() - viewport.top() - (viewport.size.height - bounds.size.height) / 2.0
            }
            MessageScrollerAlign::Nearest
                if bounds.top() >= viewport.top() && bounds.bottom() <= viewport.bottom() =>
            {
                px(0.0)
            }
            MessageScrollerAlign::Nearest if bounds.top() < viewport.top() => {
                bounds.top() - viewport.top() - self.config.scroll_margin
            }
            MessageScrollerAlign::End | MessageScrollerAlign::Nearest => {
                bounds.bottom() - viewport.bottom() + self.config.scroll_margin
            }
        };
        self.list.scroll_by(target);
    }

    fn apply_default_position(&mut self) {
        match self.config.default_position {
            MessageScrollerDefaultPosition::Start => self.list.scroll_to(ListOffset::default()),
            MessageScrollerDefaultPosition::End => self.list.scroll_to_end(),
            MessageScrollerDefaultPosition::LastAnchor => {
                if let Some(index) = self.items.iter().rposition(|item| item.scroll_anchor) {
                    self.list.scroll_to(ListOffset {
                        item_ix: index,
                        offset_in_item: px(0.0),
                    });
                    self.pending_initial_anchor = Some(index);
                } else {
                    self.list.scroll_to_end();
                }
            }
        }
    }

    fn apply_pending_initial_anchor(&mut self) -> bool {
        let Some(index) = self.pending_initial_anchor else {
            return false;
        };
        let Some(bounds) = self.list.bounds_for_item(index) else {
            return false;
        };
        let viewport = self.list.viewport_bounds();
        self.list
            .scroll_by(bounds.top() - viewport.top() - self.config.previous_item_peek);
        self.pending_initial_anchor = None;
        true
    }

    fn apply_pending_message_scroll(&mut self) -> bool {
        let Some((index, align)) = self.pending_message_scroll else {
            return false;
        };
        if self.list.bounds_for_item(index).is_none() {
            return false;
        }
        self.align_item(index, align);
        self.pending_message_scroll = None;
        true
    }

    fn recompute_snapshot(&mut self) -> bool {
        self.recompute_snapshot_in_range(0..self.items.len())
    }

    fn recompute_snapshot_in_range(&mut self, visible_range: Range<usize>) -> bool {
        let viewport = self.list.viewport_bounds();
        let offset = self.list.logical_scroll_top();
        let start = offset.item_ix > 0 || offset.offset_in_item > self.config.edge_threshold;
        let end = self
            .list
            .is_scrolled_to_end()
            .map_or(visible_range.end < self.items.len(), |at_end| !at_end);
        let visible_top = viewport.top() + self.config.scroll_margin;
        let mut visible_message_ids = Vec::new();
        let mut current_anchor_id = None;
        for index in bounded_item_range(visible_range, self.items.len()) {
            let item = &self.items[index];
            let Some(bounds) = self.list.bounds_for_item(index) else {
                continue;
            };
            if bounds.bottom() > viewport.top() && bounds.top() < viewport.bottom() {
                visible_message_ids.push(item.id.clone());
            }
            if item.scroll_anchor && bounds.top() <= visible_top + px(POSITION_EPSILON) {
                current_anchor_id = Some(item.id.clone());
            }
        }
        let next = MessageScrollerSnapshot {
            start,
            end,
            current_anchor_id,
            visible_message_ids,
        };
        if next == self.snapshot {
            false
        } else {
            self.snapshot = next;
            true
        }
    }
}

fn bounded_item_range(requested: Range<usize>, item_count: usize) -> Range<usize> {
    let start = requested.start.min(item_count);
    start..requested.end.min(item_count).max(start)
}

fn common_prefix_len(old: &[ItemRegistration], next: &[ItemRegistration]) -> usize {
    old.iter()
        .zip(next)
        .take_while(|(old, next)| old == next)
        .count()
}

fn common_suffix_len(old: &[ItemRegistration], next: &[ItemRegistration]) -> usize {
    old.iter()
        .rev()
        .zip(next.iter().rev())
        .take_while(|(old, next)| old == next)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> MessageScrollerConfig {
        MessageScrollerConfig {
            auto_scroll: false,
            default_position: MessageScrollerDefaultPosition::Start,
            edge_threshold: px(8.0),
            scroll_margin: px(0.0),
            previous_item_peek: px(64.0),
        }
    }

    fn registration(id: &str) -> ItemRegistration {
        ItemRegistration {
            id: id.into(),
            scroll_anchor: false,
            content_version: 0,
        }
    }

    #[test]
    fn unknown_item_heights_use_visible_range_to_detect_distance_from_end() {
        let mut state = MessageScrollerState::new(config());
        state.set_items(vec![
            (SharedString::from("message-1"), false, 0),
            (SharedString::from("message-2"), false, 0),
            (SharedString::from("message-3"), false, 0),
        ]);

        assert_eq!(state.list().is_scrolled_to_end(), None);
        assert!(state.user_scroll_intent(0..2));
        assert!(state.snapshot().end);
    }

    #[test]
    fn row_diff_keeps_the_unchanged_prefix_and_suffix() {
        let old = vec![registration("a"), registration("b"), registration("c")];
        let next = vec![
            registration("a"),
            registration("x"),
            registration("b"),
            registration("c"),
        ];
        let prefix = common_prefix_len(&old, &next);
        let suffix = common_suffix_len(&old[prefix..], &next[prefix..]);
        assert_eq!(prefix, 1);
        assert_eq!(suffix, 2);
    }

    #[test]
    fn layout_key_invalidates_measurements_for_width_or_rem_changes() {
        let mut state = MessageScrollerState::new(config());
        state.set_items(vec![("message".into(), false, 0)]);

        assert!(!state.sync_layout(px(600.0), px(16.0)));
        assert!(!state.sync_layout(px(600.0), px(16.0)));
        assert!(state.sync_layout(px(620.0), px(16.0)));
        assert!(!state.sync_layout(px(620.0), px(16.0)));
        assert!(state.sync_layout(px(620.0), px(18.0)));
    }

    #[test]
    fn user_layout_change_pauses_tail_following_before_remeasurement() {
        let mut state = MessageScrollerState::new(MessageScrollerConfig {
            auto_scroll: true,
            default_position: MessageScrollerDefaultPosition::End,
            edge_threshold: px(8.0),
            scroll_margin: px(0.0),
            previous_item_peek: px(64.0),
        });
        state.set_items(vec![(SharedString::from("message"), false, 0)]);
        assert!(state.list.is_following_tail());

        state.prepare_for_user_layout_change();

        assert!(!state.list.is_following_tail());
    }

    #[test]
    fn remeasure_by_id_targets_only_registered_rows() {
        let mut state = MessageScrollerState::new(config());
        state.set_items(vec![
            (SharedString::from("first"), false, 0),
            (SharedString::from("second"), false, 0),
            (SharedString::from("third"), false, 0),
        ]);

        let count = state.remeasure_items_by_id(&[
            SharedString::from("second"),
            SharedString::from("missing"),
            SharedString::from("second"),
        ]);

        assert_eq!(count, 1);
    }

    #[test]
    fn prepending_rows_preserves_the_visible_item_by_default() {
        let mut state = MessageScrollerState::new(config());
        state.set_items(vec![("b".into(), false, 0), ("c".into(), false, 0)]);
        state.list.scroll_to(ListOffset {
            item_ix: 1,
            offset_in_item: px(5.0),
        });

        state.set_items(vec![
            ("a".into(), false, 0),
            ("b".into(), false, 0),
            ("c".into(), false, 0),
        ]);

        let offset = state.list.logical_scroll_top();
        assert_eq!(offset.item_ix, 2);
        assert_eq!(offset.offset_in_item, px(5.0));
    }

    #[test]
    fn prepend_preservation_can_be_disabled_explicitly() {
        let mut state = MessageScrollerState::new(config());
        state.set_preserve_scroll_on_prepend(false);
        state.set_items(vec![("b".into(), false, 0), ("c".into(), false, 0)]);
        state.list.scroll_to(ListOffset {
            item_ix: 1,
            offset_in_item: px(5.0),
        });

        state.set_items(vec![
            ("a".into(), false, 0),
            ("b".into(), false, 0),
            ("c".into(), false, 0),
        ]);

        let offset = state.list.logical_scroll_top();
        assert_eq!(offset.item_ix, 1);
        assert_eq!(offset.offset_in_item, px(5.0));
    }

    #[test]
    fn offscreen_message_scroll_is_deferred_until_the_row_is_measured() {
        let mut state = MessageScrollerState::new(config());
        state.set_items(vec![
            ("a".into(), false, 0),
            ("b".into(), false, 0),
            ("c".into(), false, 0),
        ]);

        assert!(state.scroll_to_message(&"c".into(), MessageScrollerAlign::Center));
        assert_eq!(state.list.logical_scroll_top().item_ix, 2);
        assert_eq!(
            state.pending_message_scroll,
            Some((2, MessageScrollerAlign::Center))
        );
    }

    #[test]
    fn scroll_snapshot_scan_is_bounded_to_the_visible_rows() {
        let visible = bounded_item_range(9_995..usize::MAX, 10_000);

        assert_eq!(visible, 9_995..10_000);
        assert_eq!(visible.len(), 5);
    }
}
