//! Pure keyboard navigation and typeahead matching for dropdown menus.

use gpui::{KeyDownEvent, SharedString};

use super::DropdownMenuEntry;

pub(super) fn step_active(
    current: Option<&SharedString>,
    navigable: &[SharedString],
    delta: isize,
) -> Option<SharedString> {
    if navigable.is_empty() {
        return None;
    }
    let position = current.and_then(|current| navigable.iter().position(|item| item == current));
    let next = match (position, delta.is_negative()) {
        (None, false) => 0,
        (None | Some(0), true) => navigable.len() - 1,
        (Some(position), true) => position - 1,
        (Some(position), false) => (position + 1) % navigable.len(),
    };
    navigable.get(next).cloned()
}

pub(super) fn typeahead_character(event: &KeyDownEvent) -> Option<String> {
    let modifiers = event.keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return None;
    }
    event
        .keystroke
        .key_char
        .as_ref()
        .filter(|character| character.chars().count() == 1 && !character.trim().is_empty())
        .cloned()
}

pub(super) fn typeahead_match(
    entries: &[DropdownMenuEntry],
    navigable: &[SharedString],
    active: Option<&SharedString>,
    query: &str,
) -> Option<SharedString> {
    if navigable.is_empty() {
        return None;
    }
    let start = active
        .and_then(|active| navigable.iter().position(|id| id == active))
        .map_or(0, |position| (position + 1) % navigable.len());
    (0..navigable.len())
        .map(|offset| &navigable[(start + offset) % navigable.len()])
        .find(|id| {
            entries
                .iter()
                .find(|entry| entry.id() == Some(*id))
                .and_then(entry_label)
                .is_some_and(|label| label.to_lowercase().starts_with(query))
        })
        .cloned()
        .or_else(|| active.cloned())
}

fn entry_label(entry: &DropdownMenuEntry) -> Option<&str> {
    match entry {
        DropdownMenuEntry::Item(item) => Some(item.label.as_ref()),
        DropdownMenuEntry::Checkbox(item) => Some(item.label.as_ref()),
        DropdownMenuEntry::Radio(item) => Some(item.label.as_ref()),
        DropdownMenuEntry::Submenu { label, .. } => Some(label.as_ref()),
        DropdownMenuEntry::Label { .. }
        | DropdownMenuEntry::Separator
        | DropdownMenuEntry::CustomRow(_) => None,
    }
}
