mod actions;
mod choice;
mod content;
mod keyboard;
mod state;

use std::rc::Rc;

use gpui::{
    App, Entity, InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement as _,
    RenderOnce, SharedString, Styled as _, Window, div,
};

use crate::theme::{border::BORDER_DEFAULT, colors::G2VTheme, radius::RADIUS_XL};

pub use state::{
    QuestionnaireAnswer, QuestionnaireChoiceDefinition, QuestionnaireInputDefinition,
    QuestionnaireItemDefinition, QuestionnaireItemStatus, QuestionnaireState,
};

type SubmitHandler = Rc<dyn Fn(Vec<QuestionnaireAnswer>, &mut Window, &mut App)>;

#[derive(IntoElement)]
#[must_use]
pub struct Questionnaire {
    id: SharedString,
    state: Entity<QuestionnaireState>,
    on_submit: Option<SubmitHandler>,
}

impl Questionnaire {
    pub fn new(id: impl Into<SharedString>, state: &Entity<QuestionnaireState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            on_submit: None,
        }
    }

    pub fn on_submit(
        mut self,
        handler: impl Fn(Vec<QuestionnaireAnswer>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_submit = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Questionnaire {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let (current, count, definition, invalid, input) = {
            let state = self.state.read(cx);
            let current = state.current_index();
            (
                current,
                state.definitions().len(),
                state.current_definition().clone(),
                state.is_invalid(current),
                state.input(current).cloned(),
            )
        };

        let choices = definition
            .choices
            .iter()
            .enumerate()
            .map(|(choice_index, choice)| {
                choice::render(
                    &choice::ChoiceContext {
                        id: &self.id,
                        state: &self.state,
                        item_index: current,
                        multiple: definition.multiple,
                        item_disabled: definition.disabled,
                        theme: &theme,
                    },
                    choice_index,
                    choice,
                    cx,
                )
            })
            .collect::<Vec<_>>();

        let keyboard_state = self.state.clone();
        let keyboard_submit_handler = self.on_submit.clone();
        let keyboard_definition = definition.clone();
        let is_last = current + 1 == count;

        div()
            .id(format!("questionnaire-{}", self.id))
            .debug_selector(|| "questionnaire-surface".into())
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            .px_4()
            .py_3()
            .rounded(RADIUS_XL)
            .border(BORDER_DEFAULT)
            .border_color(if invalid {
                theme.destructive
            } else {
                theme.input
            })
            .bg(theme.composer)
            .on_key_down(move |event: &KeyDownEvent, window, cx| {
                keyboard::handle(
                    event,
                    &keyboard_state,
                    &keyboard_definition,
                    is_last,
                    keyboard_submit_handler.as_ref(),
                    window,
                    cx,
                );
            })
            .child(content::render(
                content::Content {
                    current,
                    count,
                    definition: definition.clone(),
                    invalid,
                    input,
                    choices,
                },
                &theme,
            ))
            .child(actions::render(actions::Actions {
                id: self.id,
                state: self.state,
                current,
                is_last,
                required: definition.required,
                disabled: definition.disabled,
                submit: self.on_submit,
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::spacing::{SPACE_3, SPACE_4};
    use crate::theme::{border::BORDER_DEFAULT, sizing::SIZE_10};
    use gpui::{AppContext as _, Context, Render, WindowAppearance, px};

    struct QuestionnaireHarness {
        state: Entity<QuestionnaireState>,
    }

    impl Render for QuestionnaireHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(px(768.0))
                .child(Questionnaire::new("test-questionnaire", &self.state))
        }
    }

    fn definitions() -> Vec<QuestionnaireItemDefinition> {
        vec![
            QuestionnaireItemDefinition::new("provider", "Provider")
                .choices(vec![
                    QuestionnaireChoiceDefinition::new("codex", "Codex"),
                    QuestionnaireChoiceDefinition::new("claude", "Claude"),
                ])
                .required(true),
            QuestionnaireItemDefinition::new("context", "Context")
                .input(QuestionnaireInputDefinition::new("Context", "Add context")),
        ]
    }

    #[gpui::test]
    fn selection_validation_and_skip_follow_the_react_contract(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (harness, window) = cx.add_window_view(|window, cx| QuestionnaireHarness {
            state: cx.new(|cx| QuestionnaireState::new(definitions(), window, cx)),
        });
        window.update(|window, cx| window.draw(cx).clear(cx));

        window.update(|window, cx| {
            let state = harness.read(cx).state.clone();
            assert!(!state.update(cx, QuestionnaireState::next));
            assert_eq!(state.read(cx).current_index(), 0);
            state.update(cx, |state, cx| state.select(1, window, cx));
            assert!(state.update(cx, QuestionnaireState::next));
            assert_eq!(state.read(cx).current_index(), 1);
            assert!(state.update(cx, QuestionnaireState::skip));
            assert_eq!(
                state.read(cx).answers(cx),
                vec![
                    QuestionnaireAnswer {
                        name: "provider".into(),
                        values: vec!["claude".into()],
                    },
                    QuestionnaireAnswer {
                        name: "context".into(),
                        values: Vec::new(),
                    },
                ]
            );
        });
    }

    #[test]
    fn dimensions_match_the_compact_questionnaire_contract() {
        assert_eq!(SIZE_10.to_pixels(px(16.0)), px(40.0));
        assert_eq!(SPACE_3, px(12.0));
        assert_eq!(SPACE_4, px(16.0));
    }

    #[gpui::test]
    fn conversation_questionnaire_uses_compact_composer_metrics(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (_, window) = cx.add_window_view(|window, cx| QuestionnaireHarness {
            state: cx.new(|cx| {
                QuestionnaireState::new(
                    vec![
                        QuestionnaireItemDefinition::new("scope", "Verification scope")
                            .description("Which verification scope should the fixture use?")
                            .choices(vec![
                                QuestionnaireChoiceDefinition::new("focused", "Focused")
                                    .description("Run the conversation checks only."),
                                QuestionnaireChoiceDefinition::new("full", "Full")
                                    .description("Run the full desktop verification."),
                            ])
                            .input(QuestionnaireInputDefinition::new(
                                "Answer",
                                "Type your answer",
                            ))
                            .required(true),
                    ],
                    window,
                    cx,
                )
            }),
        });

        for _ in 0..2 {
            window.update(|window, cx| window.draw(cx).clear(cx));
        }
        let surface = window
            .debug_bounds("questionnaire-surface")
            .expect("questionnaire surface bounds must be available");
        let heading = window
            .debug_bounds("questionnaire-heading")
            .expect("questionnaire heading bounds must be available");
        let title = window
            .debug_bounds("questionnaire-title")
            .expect("questionnaire title bounds must be available");
        let choice = window
            .debug_bounds("questionnaire-choice-0-0")
            .expect("questionnaire choice bounds must be available");

        assert!(surface.size.height <= px(300.0), "surface: {surface:?}");
        assert!(heading.size.height <= px(16.0), "heading: {heading:?}");
        assert!(title.size.height <= px(20.0), "title: {title:?}");
        assert!(
            choice.size.height <= SIZE_10.to_pixels(px(16.0)) + BORDER_DEFAULT * 2.0,
            "choice: {choice:?}"
        );
    }

    #[gpui::test]
    fn secret_inputs_are_masked_and_pending_submission_can_disable_the_form(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (harness, _window) = cx.add_window_view(|window, cx| QuestionnaireHarness {
            state: cx.new(|cx| {
                QuestionnaireState::new(
                    vec![
                        QuestionnaireItemDefinition::new("token", "Token")
                            .input(
                                QuestionnaireInputDefinition::new("Token", "Enter token")
                                    .masked(true),
                            )
                            .required(true),
                    ],
                    window,
                    cx,
                )
            }),
        });

        cx.update(|cx| {
            let state = harness.read(cx).state.clone();
            let input = state
                .read(cx)
                .input(0)
                .cloned()
                .expect("secret questionnaire item must own an input");
            assert!(input.read(cx).presentation().is_masked());
            state.update(cx, |state, cx| state.set_disabled(true, cx));
            assert!(state.read(cx).definitions()[0].disabled);
        });
    }
}
