use gpui::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Role,
    SharedString, Styled as _, div, prelude::FluentBuilder as _,
};
use gpui_base::Button as ButtonBehavior;

use crate::{
    theme::{
        border::BORDER_DEFAULT,
        colors::G2VTheme,
        radius::{RADIUS_FULL, RADIUS_LG, RADIUS_SM},
        sizing::{SIZE_2, SIZE_3_5, SIZE_4, SIZE_10},
        typography,
    },
    ui::{Icon, IconName, TextStyleExt as _},
};

use super::{QuestionnaireChoiceDefinition, QuestionnaireState};

const RADIO_DOT_SIZE: gpui::Rems = SIZE_2;

pub(super) struct ChoiceContext<'a> {
    pub id: &'a SharedString,
    pub state: &'a Entity<QuestionnaireState>,
    pub item_index: usize,
    pub multiple: bool,
    pub item_disabled: bool,
    pub theme: &'a G2VTheme,
}

pub(super) fn render(
    context: &ChoiceContext<'_>,
    choice_index: usize,
    choice: &QuestionnaireChoiceDefinition,
    cx: &App,
) -> AnyElement {
    let selected = context
        .state
        .read(cx)
        .is_selected(context.item_index, choice_index);
    let disabled = context.item_disabled || choice.disabled;
    let click_state = context.state.clone();
    let theme = context.theme.clone();
    let focus = context
        .state
        .read(cx)
        .choice_focus(context.item_index, choice_index)
        .clone();
    let indicator = indicator(context.multiple, selected, &theme);
    let label = label(choice, &theme);
    let id = format!(
        "{}-choice-{}-{choice_index}",
        context.id, context.item_index
    );

    ButtonBehavior::new(id)
        .w_full()
        .min_h(SIZE_10)
        .flex()
        .items_center()
        .gap_2p5()
        .px_3()
        .py_1()
        .rounded(RADIUS_LG)
        .border(BORDER_DEFAULT)
        .border_color(if selected {
            theme.primary.opacity(0.4)
        } else {
            theme.input
        })
        .bg(if selected {
            theme.muted
        } else {
            theme.background.opacity(0.0)
        })
        .text_color(theme.foreground)
        .cursor_pointer()
        .role(if context.multiple {
            Role::CheckBox
        } else {
            Role::RadioButton
        })
        .track_focus(&focus)
        .disabled(disabled)
        .debug_selector(move || {
            format!("questionnaire-choice-{}-{choice_index}", context.item_index)
        })
        .styles(|styles| styles.disabled(|style| style.opacity(0.5).cursor_default()))
        .when(!disabled, |row| {
            row.hover(|style| style.bg(theme.muted.opacity(0.5)))
        })
        .on_click(move |_, window, cx| {
            click_state.update(cx, |state, cx| state.select(choice_index, window, cx));
        })
        .child(indicator)
        .child(label)
        .into_any_element()
}

fn indicator(multiple: bool, selected: bool, theme: &G2VTheme) -> impl IntoElement {
    div()
        .size(SIZE_4)
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(if multiple { RADIUS_SM } else { RADIUS_FULL })
        .border(BORDER_DEFAULT)
        .border_color(if selected { theme.primary } else { theme.input })
        .bg(if selected {
            theme.primary
        } else {
            theme.background.opacity(0.0)
        })
        .when(selected, |indicator| {
            if multiple {
                indicator
                    .child(Icon::new(IconName::Check, SIZE_3_5).color(theme.primary_foreground))
            } else {
                indicator.child(
                    div()
                        .size(RADIO_DOT_SIZE)
                        .rounded(RADIUS_FULL)
                        .bg(theme.primary_foreground),
                )
            }
        })
}

fn label(choice: &QuestionnaireChoiceDefinition, theme: &G2VTheme) -> impl IntoElement {
    let typography = typography::tokens();
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .items_start()
        .child(div().text_style(typography.xs).child(choice.label.clone()))
        .when_some(choice.description.clone(), |content, description| {
            content.child(
                div()
                    .text_style(typography.xs)
                    .text_color(theme.muted_foreground)
                    .child(description),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn indicator_dimensions_follow_the_window_rem_size() {
        assert_eq!(SIZE_4.to_pixels(px(16.0)), px(16.0));
        assert_eq!(RADIO_DOT_SIZE.to_pixels(px(16.0)), px(8.0));
        assert_eq!(RADIO_DOT_SIZE.to_pixels(px(18.0)), px(9.0));
    }
}
