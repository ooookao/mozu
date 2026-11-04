//! G2V's canonical native multi-line text input primitive.

use gpui::{
    App, Edges, Entity, Focusable as _, InteractiveElement as _, IntoElement, ParentElement as _,
    Pixels, RenderOnce, Role, SharedString, Styled as _, Window, prelude::FluentBuilder as _, px,
};
use gpui_base::{
    ElementExt as _,
    input::{InputBase, Textarea as TextareaBehavior, TextareaState},
};

use crate::theme::border::BORDER_DEFAULT;
use crate::theme::colors::G2VTheme;
use crate::theme::radius::RADIUS_LG;
use crate::theme::sizing::{SIZE_14, SIZE_16};
use crate::theme::typography;

use super::TextStyleExt as _;
use super::input::{
    InputPresentation, InputVisualState, animated_input_colors, input_text_style, invalid_ring,
    reconcile_presentation, visual_is_dark, with_text_control_pointer_contract,
};

const MIN_HEIGHT: gpui::Rems = SIZE_16;

#[derive(IntoElement)]
#[must_use]
pub struct Textarea {
    state: Entity<TextareaState>,
    chrome: TextareaChrome,
    disabled: bool,
    readonly: bool,
    invalid: bool,
    accessibility_label: Option<SharedString>,
    tab_behavior: TextareaTabBehavior,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TextareaTabBehavior {
    Editor,
    FocusTraversal,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum TextareaChrome {
    #[default]
    Standalone,
    Grouped,
    Composer,
    Integrated,
}

impl Textarea {
    /// Keybinding predicate for application-owned focus traversal in opt-in form fields.
    pub const TAB_NAVIGATION_CONTEXT: &str = "G2VTextareaNavigation > Input";

    /// Lets the application route Tab and Shift-Tab to its focus-trap-aware traversal.
    pub fn tab_navigation(mut self) -> Self {
        self.tab_behavior = TextareaTabBehavior::FocusTraversal;
        self
    }

    /// A multiline editor whose surrounding surface owns the border and background.
    pub fn integrated(state: &Entity<TextareaState>) -> Self {
        Self {
            chrome: TextareaChrome::Integrated,
            ..Self::new(state)
        }
    }

    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            state: state.clone(),
            chrome: TextareaChrome::Standalone,
            disabled: false,
            readonly: false,
            invalid: false,
            accessibility_label: None,
            tab_behavior: TextareaTabBehavior::Editor,
        }
    }

    pub(crate) fn grouped(state: &Entity<TextareaState>) -> Self {
        Self {
            chrome: TextareaChrome::Grouped,
            ..Self::new(state)
        }
    }

    pub(crate) fn composer(state: &Entity<TextareaState>) -> Self {
        Self {
            chrome: TextareaChrome::Composer,
            ..Self::new(state)
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl RenderOnce for Textarea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.render_base(window, cx)
    }
}

impl Textarea {
    pub(super) fn render_base(self, window: &mut Window, cx: &mut App) -> InputBase {
        let theme = G2VTheme::global(cx).clone();
        let is_dark = visual_is_dark(window.appearance());
        let focused = self.state.focus_handle(cx).is_focused(window) && !self.disabled;
        let visual_state =
            InputVisualState::new(window.appearance(), focused, self.disabled, self.invalid);
        let (border, background) =
            animated_input_colors(self.state.entity_id(), &theme, visual_state, window, cx);
        let typography = typography::tokens();
        let text_style = input_text_style(window, &typography);

        let presentation = InputPresentation::new(&theme, self.disabled, self.readonly);
        let presentation_cache = window.use_keyed_state(
            ("g2v-textarea-presentation", self.state.entity_id()),
            cx,
            |_, _| None::<InputPresentation>,
        );
        let textarea_state = self.state.clone();

        let focus_handle = self.state.focus_handle(cx);
        let textarea = InputBase::new(("g2v-textarea", self.state.entity_id()))
            .when(
                self.tab_behavior == TextareaTabBehavior::FocusTraversal,
                |textarea| textarea.key_context("G2VTextareaNavigation"),
            )
            .role(Role::MultilineTextInput)
            .focused(focused)
            .disabled(self.disabled)
            .when_some(self.accessibility_label, |textarea, label| {
                textarea.accessibility_label(label)
            })
            .on_prepaint(move |_, _, cx| {
                reconcile_presentation(&presentation_cache, presentation, cx, |cx| {
                    textarea_state.update(cx, |state, cx| {
                        state.set_editor_style(presentation.editor_style());
                        state.set_editor_paddings(textarea_editor_paddings());
                        state.set_disabled(presentation.disabled, cx);
                        state.set_readonly(presentation.readonly, cx);
                    });
                });
            })
            .flex()
            .items_start()
            .w_full()
            .text_style(text_style)
            .when(self.chrome == TextareaChrome::Standalone, |textarea| {
                textarea
                    .min_h(MIN_HEIGHT)
                    .px_2p5()
                    .py_2()
                    .rounded(RADIUS_LG)
                    .border(BORDER_DEFAULT)
                    .border_color(border)
                    .bg(background)
                    .when(self.disabled, |textarea| textarea.opacity(0.5))
                    .when(self.invalid, |textarea| {
                        textarea.shadow(invalid_ring(theme.destructive, is_dark))
                    })
            })
            .when(self.chrome == TextareaChrome::Grouped, |textarea| {
                textarea
                    .flex_1()
                    .min_h(MIN_HEIGHT)
                    .px_2p5()
                    .py_2()
                    .bg(theme.background.opacity(0.0))
            })
            .when(self.chrome == TextareaChrome::Composer, |textarea| {
                textarea
                    .flex_1()
                    .min_h(SIZE_14)
                    .px_3p5()
                    .pt_3p5()
                    .pb(Pixels::ZERO)
                    .bg(theme.background.opacity(0.0))
            })
            .when(self.chrome == TextareaChrome::Integrated, |textarea| {
                textarea.min_h(MIN_HEIGHT).bg(theme.background.opacity(0.0))
            })
            .child(TextareaBehavior::new(&self.state));

        with_text_control_pointer_contract(textarea, focus_handle, self.disabled)
    }
}

fn textarea_editor_paddings() -> Edges<gpui::Pixels> {
    Edges::all(px(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Context, Modifiers, Render, TestAppContext, VisualTestContext, div, px,
    };

    use crate::ui::input::{TextControlPolicy, install_text_control_test_globals};

    struct TextareaHarness {
        state: Entity<TextareaState>,
        policy: TextControlPolicy,
    }

    impl Render for TextareaHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .debug_selector(|| "textarea-control".into())
                .w(px(240.0))
                .h(MIN_HEIGHT)
                .child(
                    Textarea::new(&self.state)
                        .accessibility_label("Message")
                        .disabled(self.policy.disabled())
                        .readonly(self.policy.readonly()),
                )
        }
    }

    fn textarea_harness(
        cx: &mut TestAppContext,
        policy: TextControlPolicy,
    ) -> (Entity<TextareaHarness>, &mut VisualTestContext) {
        install_text_control_test_globals(cx);
        let (harness, window) = cx.add_window_view(move |window, cx| TextareaHarness {
            state: cx.new(|cx| TextareaState::new(window, cx)),
            policy,
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.update(|window, cx| window.draw(cx).clear(cx));
        (harness, window)
    }

    #[gpui::test]
    fn pointer_focus_and_keyboard_edit_the_enabled_textarea(cx: &mut TestAppContext) {
        let (harness, window) = textarea_harness(cx, TextControlPolicy::Enabled);
        let bounds = window.debug_bounds("textarea-control").unwrap();

        window.simulate_click(bounds.center(), Modifiers::default());
        window.simulate_keystrokes("g2v");

        window.update(|window, cx| {
            let state = harness.read(cx).state.clone();
            assert!(
                state.focus_handle(cx).is_focused(window),
                "textarea did not focus after clicking bounds={bounds:?}, value={:?}",
                state.read(cx).value()
            );
            assert_eq!(state.read(cx).value(), "g2v", "bounds={bounds:?}");
        });
    }

    #[gpui::test]
    fn disabled_and_readonly_textarea_policies_are_inert(cx: &mut TestAppContext) {
        let (disabled, window) = textarea_harness(cx, TextControlPolicy::Disabled);
        let bounds = window.debug_bounds("textarea-control").unwrap();
        window.simulate_click(bounds.center(), Modifiers::default());
        window.simulate_keystrokes("blocked");
        window.update(|window, cx| {
            let state = disabled.read(cx).state.clone();
            assert!(!state.focus_handle(cx).is_focused(window));
            assert!(state.read(cx).value().is_empty());
        });

        let (readonly, window) = textarea_harness(cx, TextControlPolicy::Readonly);
        let bounds = window.debug_bounds("textarea-control").unwrap();
        window.simulate_click(bounds.center(), Modifiers::default());
        window.simulate_keystrokes("blocked");
        window.update(|window, cx| {
            let state = readonly.read(cx).state.clone();
            assert!(state.focus_handle(cx).is_focused(window));
            assert!(state.read(cx).value().is_empty());
        });
    }

    #[test]
    fn dimensions_match_the_react_textarea_contract() {
        assert_eq!(MIN_HEIGHT.to_pixels(px(16.0)), px(64.0));
        assert_eq!(RADIUS_LG, px(13.2));
        assert_eq!(BORDER_DEFAULT, px(0.5));
    }

    #[test]
    fn base_editor_adds_no_second_layer_of_padding() {
        let paddings = textarea_editor_paddings();

        assert_eq!(paddings.top, px(0.0));
        assert_eq!(paddings.right, px(0.0));
        assert_eq!(paddings.bottom, px(0.0));
        assert_eq!(paddings.left, px(0.0));
    }
}
