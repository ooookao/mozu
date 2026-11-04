use gpui::{App, Entity, KeyDownEvent, Window};

use super::{QuestionnaireItemDefinition, QuestionnaireState, SubmitHandler};

pub(super) fn handle(
    event: &KeyDownEvent,
    state: &Entity<QuestionnaireState>,
    definition: &QuestionnaireItemDefinition,
    is_last: bool,
    submit: Option<&SubmitHandler>,
    window: &mut Window,
    cx: &mut App,
) {
    let modifiers = event.keystroke.modifiers;
    let current = state.read(cx).current_index();
    let input_focused = state.read(cx).input_is_focused(current, window, cx);
    if event.keystroke.key == "enter" && (modifiers.platform || modifiers.control) {
        cx.stop_propagation();
        advance_or_submit(state, is_last, submit, window, cx);
        return;
    }
    if input_focused {
        return;
    }
    match event.keystroke.key.as_str() {
        "up" | "down" => {
            cx.stop_propagation();
            let delta = if event.keystroke.key == "up" { -1 } else { 1 };
            state.update(cx, |state, cx| {
                state.focus_relative_choice(delta, window, cx);
            });
        }
        "left" if current > 0 => {
            cx.stop_propagation();
            state.update(cx, QuestionnaireState::previous);
        }
        "right" => {
            cx.stop_propagation();
            advance_or_submit(state, is_last, submit, window, cx);
        }
        _ if shortcut_allowed(event) => {
            if let Some(choice_index) =
                shortcut_choice_index(event.keystroke.key_char.as_deref(), definition)
            {
                cx.stop_propagation();
                state.update(cx, |state, cx| state.select(choice_index, window, cx));
            }
        }
        _ => {}
    }
}

pub(super) fn advance_or_submit(
    state: &Entity<QuestionnaireState>,
    is_last: bool,
    handler: Option<&SubmitHandler>,
    window: &mut Window,
    cx: &mut App,
) {
    if !is_last {
        state.update(cx, QuestionnaireState::next);
        return;
    }
    let valid = state.update(cx, QuestionnaireState::validate_all);
    if valid && let Some(handler) = handler {
        handler(state.read(cx).answers(cx), window, cx);
    }
}

fn shortcut_allowed(event: &KeyDownEvent) -> bool {
    let modifiers = event.keystroke.modifiers;
    !modifiers.platform && !modifiers.control && !modifiers.alt && !modifiers.function
}

fn shortcut_choice_index(
    key: Option<&str>,
    definition: &QuestionnaireItemDefinition,
) -> Option<usize> {
    let key = key?.to_ascii_lowercase();
    definition
        .choices
        .iter()
        .enumerate()
        .filter(|(_, choice)| !choice.disabled)
        .enumerate()
        .find_map(|(shortcut_index, (choice_index, _))| {
            let number = (shortcut_index < 9).then(|| (shortcut_index + 1).to_string());
            let letter = u8::try_from(shortcut_index)
                .ok()
                .and_then(|index| b'a'.checked_add(index))
                .map(char::from)
                .map(|character| character.to_string());
            (number.as_deref() == Some(key.as_str()) || letter.as_deref() == Some(key.as_str()))
                .then_some(choice_index)
        })
}
