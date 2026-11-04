use gpui::{
    AnyElement, Entity, InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, div,
    prelude::FluentBuilder as _,
};
use gpui_base::input::InputState;

use crate::{
    theme::{colors::G2VTheme, typography},
    ui::{Input, TextStyleExt as _},
};

use super::QuestionnaireItemDefinition;

pub(super) struct Content {
    pub current: usize,
    pub count: usize,
    pub definition: QuestionnaireItemDefinition,
    pub invalid: bool,
    pub input: Option<Entity<InputState>>,
    pub choices: Vec<AnyElement>,
}

pub(super) fn render(content: Content, theme: &G2VTheme) -> impl IntoElement {
    let typography = typography::tokens();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .debug_selector(|| "questionnaire-heading".into())
                .child(
                    div()
                        .text_style(typography.xs)
                        .text_color(theme.muted_foreground)
                        .child(format!(
                            "Question {} of {}",
                            content.current + 1,
                            content.count
                        )),
                )
                .child(
                    div()
                        .text_style(typography.xs)
                        .text_color(theme.muted_foreground)
                        .child(if content.definition.required {
                            "Required"
                        } else {
                            "Optional"
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .debug_selector(|| "questionnaire-title".into())
                        .text_style(typography.sm)
                        .text_color(theme.foreground)
                        .child(content.definition.title.clone()),
                )
                .when_some(
                    content.definition.description.clone(),
                    |element, description| {
                        element.child(
                            div()
                                .text_style(typography.xs)
                                .text_color(theme.muted_foreground)
                                .child(description),
                        )
                    },
                ),
        )
        .child(div().flex().flex_col().gap_2().children(content.choices))
        .when_some(
            content.input.zip(content.definition.input.clone()),
            |root, (input, definition)| {
                root.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_style(typography.xs)
                                .text_color(theme.foreground)
                                .child(definition.label.clone()),
                        )
                        .child(
                            Input::new(&input)
                                .invalid(content.invalid)
                                .disabled(content.definition.disabled)
                                .accessibility_label(definition.label),
                        ),
                )
            },
        )
        .when(content.invalid, |root| {
            root.child(
                div()
                    .text_style(typography.sm)
                    .text_color(theme.destructive)
                    .child("Choose or enter an answer before continuing."),
            )
        })
}
