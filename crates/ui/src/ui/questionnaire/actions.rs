use gpui::{
    Entity, IntoElement, ParentElement as _, SharedString, Styled as _, div,
    prelude::FluentBuilder as _,
};

use crate::ui::{Button, ButtonSize, ButtonVariant};

use super::{QuestionnaireState, SubmitHandler};

pub(super) struct Actions {
    pub id: SharedString,
    pub state: Entity<QuestionnaireState>,
    pub current: usize,
    pub is_last: bool,
    pub required: bool,
    pub disabled: bool,
    pub submit: Option<SubmitHandler>,
}

pub(super) fn render(actions: Actions) -> impl IntoElement {
    let Actions {
        id,
        state,
        current,
        is_last,
        required,
        disabled,
        submit,
    } = actions;
    let previous_state = state.clone();
    let skip_state = state.clone();
    let advance_state = state;

    div()
        .w_full()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(div().child(if current > 0 {
            Button::new(format!("{id}-previous"))
                .label("Previous")
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Small)
                .on_click(move |_, _, cx| previous_state.update(cx, QuestionnaireState::previous))
                .into_any_element()
        } else {
            gpui::Empty.into_any_element()
        }))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .when(!required, |buttons| {
                    buttons.child(
                        Button::new(format!("{id}-skip"))
                            .label("Skip")
                            .variant(ButtonVariant::Ghost)
                            .size(ButtonSize::Small)
                            .disabled(disabled)
                            .on_click(move |_, _, cx| {
                                skip_state.update(cx, QuestionnaireState::skip);
                            }),
                    )
                })
                .child(if is_last {
                    Button::new(format!("{id}-submit"))
                        .label("Submit")
                        .size(ButtonSize::Small)
                        .disabled(disabled)
                        .on_click(move |_, window, cx| {
                            super::keyboard::advance_or_submit(
                                &advance_state,
                                true,
                                submit.as_ref(),
                                window,
                                cx,
                            );
                        })
                        .into_any_element()
                } else {
                    Button::new(format!("{id}-next"))
                        .label("Next")
                        .size(ButtonSize::Small)
                        .disabled(disabled)
                        .on_click(move |_, _, cx| {
                            advance_state.update(cx, QuestionnaireState::next);
                        })
                        .into_any_element()
                }),
        )
}
