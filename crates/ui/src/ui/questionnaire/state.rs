use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable as _, SharedString, Subscription,
    Window,
};
use gpui_base::input::{InputEvent, InputState};

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct QuestionnaireChoiceDefinition {
    pub value: SharedString,
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub disabled: bool,
}

impl QuestionnaireChoiceDefinition {
    pub fn new(value: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
            disabled: false,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct QuestionnaireInputDefinition {
    pub label: SharedString,
    pub placeholder: SharedString,
    pub masked: bool,
}

impl QuestionnaireInputDefinition {
    pub fn new(label: impl Into<SharedString>, placeholder: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            placeholder: placeholder.into(),
            masked: false,
        }
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub struct QuestionnaireItemDefinition {
    pub name: SharedString,
    pub title: SharedString,
    pub description: Option<SharedString>,
    pub choices: Vec<QuestionnaireChoiceDefinition>,
    pub input: Option<QuestionnaireInputDefinition>,
    pub multiple: bool,
    pub required: bool,
    pub disabled: bool,
}

impl QuestionnaireItemDefinition {
    pub fn new(name: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            description: None,
            choices: Vec::new(),
            input: None,
            multiple: false,
            required: false,
            disabled: false,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn choices(mut self, choices: Vec<QuestionnaireChoiceDefinition>) -> Self {
        self.choices = choices;
        self
    }

    pub fn input(mut self, input: QuestionnaireInputDefinition) -> Self {
        self.input = Some(input);
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuestionnaireAnswer {
    pub name: SharedString,
    pub values: Vec<SharedString>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuestionnaireItemStatus {
    Unanswered,
    Answered,
    Skipped,
}

struct ItemState {
    selected: Vec<usize>,
    choice_focus: Vec<FocusHandle>,
    input: Option<Entity<InputState>>,
    skipped: bool,
    invalid: bool,
}

pub struct QuestionnaireState {
    definitions: Vec<QuestionnaireItemDefinition>,
    items: Vec<ItemState>,
    current: usize,
    _input_subscriptions: Vec<Subscription>,
}

impl QuestionnaireState {
    /// Creates questionnaire state and its application-owned input entities.
    ///
    /// # Panics
    /// Panics when `definitions` is empty because a paged questionnaire must
    /// always have a current item.
    pub fn new(
        definitions: Vec<QuestionnaireItemDefinition>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        assert!(
            !definitions.is_empty(),
            "questionnaire requires at least one item"
        );
        let mut items = Vec::with_capacity(definitions.len());
        let mut subscriptions = Vec::new();

        for (index, definition) in definitions.iter().enumerate() {
            let input = definition.input.as_ref().map(|definition| {
                let input = cx.new(|cx| {
                    InputState::new(window, cx)
                        .placeholder(definition.placeholder.clone())
                        .masked(definition.masked)
                });
                subscriptions.push(cx.subscribe(&input, move |this, _, event, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.input_changed(index, cx);
                    }
                }));
                input
            });
            items.push(ItemState {
                selected: Vec::new(),
                choice_focus: definition
                    .choices
                    .iter()
                    .map(|_| cx.focus_handle())
                    .collect(),
                input,
                skipped: false,
                invalid: false,
            });
        }

        Self {
            definitions,
            items,
            current: 0,
            _input_subscriptions: subscriptions,
        }
    }

    pub fn definitions(&self) -> &[QuestionnaireItemDefinition] {
        &self.definitions
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self
            .definitions
            .iter()
            .all(|definition| definition.disabled == disabled)
        {
            return;
        }
        for definition in &mut self.definitions {
            definition.disabled = disabled;
        }
        cx.notify();
    }

    #[must_use]
    pub fn current_index(&self) -> usize {
        self.current
    }

    pub fn current_definition(&self) -> &QuestionnaireItemDefinition {
        &self.definitions[self.current]
    }

    #[must_use]
    pub fn input(&self, index: usize) -> Option<&Entity<InputState>> {
        self.items.get(index).and_then(|item| item.input.as_ref())
    }

    #[must_use]
    pub fn is_selected(&self, item_index: usize, choice_index: usize) -> bool {
        self.items[item_index].selected.contains(&choice_index)
    }

    #[must_use]
    pub fn choice_focus(&self, item_index: usize, choice_index: usize) -> &FocusHandle {
        &self.items[item_index].choice_focus[choice_index]
    }

    #[must_use]
    pub fn input_is_focused(&self, index: usize, window: &Window, cx: &App) -> bool {
        self.items[index]
            .input
            .as_ref()
            .is_some_and(|input| input.read(cx).focus_handle(cx).is_focused(window))
    }

    pub fn focus_relative_choice(
        &mut self,
        delta: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let navigable: Vec<usize> = self.definitions[self.current]
            .choices
            .iter()
            .enumerate()
            .filter_map(|(index, choice)| (!choice.disabled).then_some(index))
            .collect();
        if navigable.is_empty() {
            return;
        }
        let focused = navigable
            .iter()
            .position(|index| self.items[self.current].choice_focus[*index].is_focused(window));
        let next = match (focused, delta.is_negative()) {
            (None, false) => 0,
            (None | Some(0), true) => navigable.len() - 1,
            (Some(position), true) => position - 1,
            (Some(position), false) => (position + 1) % navigable.len(),
        };
        self.items[self.current].choice_focus[navigable[next]].focus(window, cx);
    }

    #[must_use]
    pub fn is_invalid(&self, index: usize) -> bool {
        self.items[index].invalid
    }

    #[must_use]
    pub fn status(&self, index: usize, cx: &App) -> QuestionnaireItemStatus {
        if self.items[index].skipped {
            QuestionnaireItemStatus::Skipped
        } else if self.has_answer(index, cx) {
            QuestionnaireItemStatus::Answered
        } else {
            QuestionnaireItemStatus::Unanswered
        }
    }

    pub fn select(&mut self, choice_index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let definition = &self.definitions[self.current];
        if definition.disabled
            || definition
                .choices
                .get(choice_index)
                .is_none_or(|choice| choice.disabled)
        {
            return;
        }

        let item = &mut self.items[self.current];
        if definition.multiple {
            if let Some(position) = item
                .selected
                .iter()
                .position(|index| *index == choice_index)
            {
                item.selected.remove(position);
            } else {
                item.selected.push(choice_index);
            }
        } else {
            item.selected.clear();
            item.selected.push(choice_index);
            if let Some(input) = &item.input {
                input.update(cx, |input, cx| input.set_value("", window, cx));
            }
        }
        item.skipped = false;
        item.invalid = false;
        cx.notify();
    }

    pub fn previous(&mut self, cx: &mut Context<Self>) {
        if self.current > 0 {
            self.current -= 1;
            cx.notify();
        }
    }

    pub fn next(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.validate(self.current, cx) {
            return false;
        }
        if self.current + 1 < self.definitions.len() {
            self.current += 1;
            cx.notify();
        }
        true
    }

    pub fn skip(&mut self, cx: &mut Context<Self>) -> bool {
        if self.current_definition().required {
            return false;
        }
        self.items[self.current].skipped = true;
        self.items[self.current].invalid = false;
        if self.current + 1 < self.definitions.len() {
            self.current += 1;
        }
        cx.notify();
        true
    }

    pub fn validate_all(&mut self, cx: &mut Context<Self>) -> bool {
        let first_invalid = (0..self.definitions.len()).find(|index| !self.validate(*index, cx));
        if let Some(index) = first_invalid {
            self.current = index;
            cx.notify();
            false
        } else {
            true
        }
    }

    #[must_use]
    pub fn answers(&self, cx: &App) -> Vec<QuestionnaireAnswer> {
        self.definitions
            .iter()
            .enumerate()
            .map(|(index, definition)| QuestionnaireAnswer {
                name: definition.name.clone(),
                values: self.answer_values(index, cx),
            })
            .collect()
    }

    fn input_changed(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.definitions[index].multiple && self.input_has_text(index, cx) {
            self.items[index].selected.clear();
        }
        self.items[index].skipped = false;
        self.items[index].invalid = false;
        cx.notify();
    }

    fn validate(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        let valid = self.items[index].skipped || self.has_answer(index, cx);
        self.items[index].invalid = !valid;
        cx.notify();
        valid
    }

    fn has_answer(&self, index: usize, cx: &App) -> bool {
        !self.items[index].selected.is_empty() || self.input_has_text(index, cx)
    }

    fn input_has_text(&self, index: usize, cx: &App) -> bool {
        self.items[index]
            .input
            .as_ref()
            .is_some_and(|input| !input.read(cx).value().trim().is_empty())
    }

    fn answer_values(&self, index: usize, cx: &App) -> Vec<SharedString> {
        let item = &self.items[index];
        if item.skipped {
            return Vec::new();
        }
        let mut values: Vec<SharedString> = item
            .selected
            .iter()
            .filter_map(|choice_index| self.definitions[index].choices.get(*choice_index))
            .map(|choice| choice.value.clone())
            .collect();
        if let Some(input) = &item.input {
            let value = input.read(cx).value();
            if !value.trim().is_empty() {
                values.push(value);
            }
        }
        values
    }
}
