//! G2V's canonical native single-line input primitive.

use gpui::{
    App, BoxShadow, ElementId, Entity, EntityId, FocusHandle, Focusable as _, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, RenderOnce, Role,
    SharedString, Styled as _, Window, prelude::FluentBuilder as _,
};
use gpui_base::{
    ElementExt as _, TextStyleToken, Transition as ValueTransition, TypographyTokens,
    input::{Input as InputBehavior, InputBase, InputEditorStyle, InputState},
    transition,
};

use crate::theme::border::BORDER_DEFAULT;
use crate::theme::colors::G2VTheme;
use crate::theme::motion::MotionTokens;
use crate::theme::radius::RADIUS_MD;
use crate::theme::shadows::{RING_3, focus_ring};
use crate::theme::sizing::{SIZE_7, SIZE_8};
use crate::theme::typography;

use super::{TextStyleExt as _, is_compact};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputSize {
    #[default]
    Default,
    Small,
}

impl InputSize {
    fn height(self) -> gpui::Rems {
        match self {
            Self::Default => SIZE_8,
            Self::Small => SIZE_7,
        }
    }

    fn text_style(self, typography: &TypographyTokens) -> TextStyleToken {
        match self {
            Self::Default => typography.sm,
            Self::Small => TextStyleToken {
                size: gpui::px(12.8),
                ..typography.sm
            },
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Input {
    state: Entity<InputState>,
    chrome: InputChrome,
    size: InputSize,
    disabled: bool,
    readonly: bool,
    invalid: bool,
    accessibility_label: Option<SharedString>,
    group_edges: Option<super::button_group::GroupEdges>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum InputChrome {
    #[default]
    Standalone,
    Grouped,
    Integrated,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct InputPresentation {
    foreground: Hsla,
    muted_foreground: Hsla,
    background: Hsla,
    border: Hsla,
    selection: Hsla,
    caret: Hsla,
    pub(super) disabled: bool,
    pub(super) readonly: bool,
}

impl InputPresentation {
    pub(super) fn new(theme: &G2VTheme, disabled: bool, readonly: bool) -> Self {
        Self {
            foreground: theme.foreground,
            muted_foreground: theme.muted_foreground,
            background: theme.background.opacity(0.0),
            border: theme.border,
            selection: theme.ring.opacity(0.3),
            caret: theme.foreground,
            disabled,
            readonly,
        }
    }

    pub(super) fn editor_style(self) -> InputEditorStyle {
        InputEditorStyle {
            foreground: self.foreground,
            muted_foreground: self.muted_foreground,
            background: self.background,
            border: self.border,
            selection: self.selection,
            caret: self.caret,
            ..InputEditorStyle::default()
        }
    }
}

impl Input {
    /// A single-line editor whose surrounding surface owns border and background.
    pub fn integrated(state: &Entity<InputState>) -> Self {
        Self {
            chrome: InputChrome::Integrated,
            ..Self::new(state)
        }
    }

    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            chrome: InputChrome::Standalone,
            size: InputSize::Default,
            disabled: false,
            readonly: false,
            invalid: false,
            accessibility_label: None,
            group_edges: None,
        }
    }

    pub fn grouped(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            chrome: InputChrome::Grouped,
            group_edges: None,
            size: InputSize::Default,
            disabled: false,
            readonly: false,
            invalid: false,
            accessibility_label: None,
        }
    }

    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
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

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let grouped = self.group_edges.is_some();
        let focus = self.state.focus_handle(cx);
        let input = self.render_base(window, cx);
        if grouped {
            super::button_group::paint_group_control(input, &focus, window)
        } else {
            input.into_any_element()
        }
    }
}

impl Input {
    pub(crate) fn in_button_group(mut self, edges: super::button_group::GroupEdges) -> Self {
        self.chrome = InputChrome::Standalone;
        self.group_edges = Some(edges);
        self
    }

    fn render_base(self, window: &mut Window, cx: &mut App) -> InputBase {
        let theme = G2VTheme::global(cx).clone();
        let is_dark = visual_is_dark(window.appearance());
        let focused = self.state.focus_handle(cx).is_focused(window) && !self.disabled;
        let visual_state =
            InputVisualState::new(window.appearance(), focused, self.disabled, self.invalid);
        let (border, background) =
            animated_input_colors(self.state.entity_id(), &theme, visual_state, window, cx);
        let typography = typography::tokens();
        let text_style = if is_compact(window) {
            typography.xs
        } else {
            self.size.text_style(&typography)
        };

        let presentation = InputPresentation::new(&theme, self.disabled, self.readonly);
        let presentation_cache = window.use_keyed_state(
            ("g2v-input-presentation", self.state.entity_id()),
            cx,
            |_, _| None::<InputPresentation>,
        );
        let input_state = self.state.clone();

        let focus_handle = self.state.focus_handle(cx);
        let input = InputBase::new(("g2v-input", self.state.entity_id()))
            .role(Role::TextInput)
            .focused(focused)
            .disabled(self.disabled)
            .when_some(self.accessibility_label, |input, label| {
                input.accessibility_label(label)
            })
            .on_prepaint(move |_, _, cx| {
                reconcile_presentation(&presentation_cache, presentation, cx, |cx| {
                    input_state.update(cx, |state, cx| {
                        state.set_editor_style(presentation.editor_style());
                        state.set_disabled(presentation.disabled, cx);
                        state.set_readonly(presentation.readonly, cx);
                    });
                });
            })
            .flex()
            .items_center()
            .w_full()
            .min_w_0()
            .h(self.size.height())
            .text_style(text_style)
            .when(self.chrome == InputChrome::Standalone, |input| {
                input
                    .px_2p5()
                    .py_1()
                    .rounded(RADIUS_MD)
                    .border(BORDER_DEFAULT)
                    .border_color(border)
                    .bg(background)
                    .when(self.disabled, |input| input.opacity(0.5))
                    .when(self.invalid, |input| {
                        input.shadow(invalid_ring(theme.destructive, is_dark))
                    })
            })
            .when(self.chrome == InputChrome::Grouped, |input| {
                input
                    .flex_1()
                    .min_w_0()
                    .px_1p5()
                    .bg(theme.background.opacity(0.0))
            })
            .when(self.chrome == InputChrome::Integrated, |input| {
                input.bg(theme.background.opacity(0.0))
            })
            .when_some(self.group_edges, |input, edges| {
                edges
                    .apply(input)
                    .flex_1()
                    .debug_selector(|| "button-group-input".into())
            })
            .child(InputBehavior::new(&self.state));

        with_text_control_pointer_contract(input, focus_handle, self.disabled)
    }
}

pub(super) fn with_text_control_pointer_contract(
    control: InputBase,
    focus_handle: FocusHandle,
    disabled: bool,
) -> InputBase {
    control
        .when(disabled, |control| {
            control.capture_any_mouse_down(|_, _, cx| cx.stop_propagation())
        })
        .when(!disabled, |control| {
            control.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                focus_handle.focus(window, cx);
            })
        })
}

pub(super) fn reconcile_presentation<P, F>(
    cache: &Entity<Option<P>>,
    desired: P,
    cx: &mut App,
    apply: F,
) where
    P: Clone + PartialEq + 'static,
    F: FnOnce(&mut App),
{
    if cache.read(cx).as_ref() == Some(&desired) {
        return;
    }
    apply(cx);
    cache.update(cx, |current, _| *current = Some(desired));
}

pub(super) const fn visual_is_dark(appearance: gpui::WindowAppearance) -> bool {
    matches!(
        appearance,
        gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
    )
}

pub(super) fn input_background(theme: &G2VTheme, is_dark: bool, disabled: bool) -> Hsla {
    if is_dark {
        theme.input.opacity(if disabled { 0.8 } else { 0.3 })
    } else if disabled {
        theme.input.opacity(0.5)
    } else {
        theme.background.opacity(0.0)
    }
}

pub(super) fn input_border(theme: &G2VTheme, is_dark: bool, focused: bool, invalid: bool) -> Hsla {
    if invalid {
        return theme.destructive.opacity(if is_dark { 0.5 } else { 1.0 });
    }
    if focused {
        return theme.ring;
    }
    theme.input
}

pub(super) fn invalid_ring(color: Hsla, is_dark: bool) -> Vec<BoxShadow> {
    focus_ring(color.opacity(if is_dark { 0.4 } else { 0.2 }), RING_3)
}

pub(super) fn input_text_style(
    window: &Window,
    typography: &gpui_base::TypographyTokens,
) -> TextStyleToken {
    if is_compact(window) {
        typography.md
    } else {
        typography.sm
    }
}

#[derive(Clone, Copy)]
pub(super) struct InputVisualState {
    appearance: gpui::WindowAppearance,
    focused: bool,
    disabled: bool,
    invalid: bool,
}

impl InputVisualState {
    pub(super) const fn new(
        appearance: gpui::WindowAppearance,
        focused: bool,
        disabled: bool,
        invalid: bool,
    ) -> Self {
        Self {
            appearance,
            focused,
            disabled,
            invalid,
        }
    }

    const fn is_dark(self) -> bool {
        matches!(
            self.appearance,
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        )
    }
}

pub(super) fn animated_input_colors(
    id: EntityId,
    theme: &G2VTheme,
    state: InputVisualState,
    window: &mut Window,
    cx: &mut App,
) -> (Hsla, Hsla) {
    let policy = ValueTransition::new(MotionTokens::global(cx).fast);
    let element_id = ElementId::from(("input", id));
    let is_dark = state.is_dark();
    let border = transition(
        (element_id.clone(), "border"),
        input_border(theme, is_dark, state.focused, state.invalid),
        policy.clone(),
        window,
        cx,
    );
    let background = transition(
        (element_id, "background"),
        input_background(theme, is_dark, state.disabled),
        policy,
        window,
        cx,
    );
    (border, background)
}

#[cfg(test)]
pub(super) fn install_text_control_test_globals(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        gpui_base::init(cx);
        crate::ui::init(cx);
        crate::theme::typography::install(cx);
        crate::theme::install(gpui::WindowAppearance::Light, cx);
    });
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) enum TextControlPolicy {
    Enabled,
    Disabled,
    Readonly,
}

#[cfg(test)]
impl TextControlPolicy {
    pub(super) const fn disabled(self) -> bool {
        matches!(self, Self::Disabled)
    }

    pub(super) const fn readonly(self) -> bool {
        matches!(self, Self::Readonly)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        AppContext as _, Context, Element as _, Modifiers, Render, Role, TestAppContext,
        VisualTestContext, WindowAppearance, accesskit, canvas, div, px,
    };
    use std::sync::{Arc, Mutex};

    struct InputHarness {
        state: Entity<InputState>,
        policy: TextControlPolicy,
    }

    impl Render for InputHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .debug_selector(|| "input-control".into())
                .w(px(240.0))
                .h(InputSize::Default.height())
                .child(
                    Input::new(&self.state)
                        .accessibility_label("Project name")
                        .disabled(self.policy.disabled())
                        .readonly(self.policy.readonly()),
                )
        }
    }

    fn input_harness(
        cx: &mut TestAppContext,
        policy: TextControlPolicy,
    ) -> (Entity<InputHarness>, &mut VisualTestContext) {
        install_text_control_test_globals(cx);
        let (harness, window) = cx.add_window_view(move |window, cx| InputHarness {
            state: cx.new(|cx| InputState::new(window, cx)),
            policy,
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.update(|window, cx| window.draw(cx).clear(cx));
        (harness, window)
    }

    #[gpui::test]
    fn pointer_focus_and_keyboard_edit_the_enabled_input(cx: &mut TestAppContext) {
        let (harness, window) = input_harness(cx, TextControlPolicy::Enabled);
        let bounds = window.debug_bounds("input-control").unwrap();

        window.simulate_click(bounds.center(), Modifiers::default());
        window.simulate_keystrokes("g2v");

        window.update(|window, cx| {
            let state = harness.read(cx).state.clone();
            assert!(
                state.focus_handle(cx).is_focused(window),
                "input did not focus after clicking bounds={bounds:?}, value={:?}",
                state.read(cx).value()
            );
            assert_eq!(state.read(cx).value(), "g2v", "bounds={bounds:?}");
        });
    }

    #[gpui::test]
    fn disabled_and_readonly_input_policies_are_inert(cx: &mut TestAppContext) {
        let (disabled, window) = input_harness(cx, TextControlPolicy::Disabled);
        let bounds = window.debug_bounds("input-control").unwrap();
        window.simulate_click(bounds.center(), Modifiers::default());
        window.simulate_keystrokes("blocked");
        window.update(|window, cx| {
            let state = disabled.read(cx).state.clone();
            assert!(!state.focus_handle(cx).is_focused(window));
            assert!(state.read(cx).value().is_empty());
        });

        let (readonly, window) = input_harness(cx, TextControlPolicy::Readonly);
        let bounds = window.debug_bounds("input-control").unwrap();
        window.simulate_click(bounds.center(), Modifiers::default());
        window.simulate_keystrokes("blocked");
        window.update(|window, cx| {
            let state = readonly.read(cx).state.clone();
            assert!(state.focus_handle(cx).is_focused(window));
            assert!(state.read(cx).value().is_empty());
        });
    }

    #[gpui::test]
    fn input_and_textarea_facades_expose_roles_and_labels(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<(accesskit::Node, accesskit::Node)>>>;

        struct AccessibilityHarness {
            input: Entity<InputState>,
            textarea: Entity<gpui_base::input::TextareaState>,
            captured: Captured,
        }

        impl Render for AccessibilityHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let input = self.input.clone();
                let textarea = self.textarea.clone();
                let captured = self.captured.clone();
                canvas(
                    move |_, window, cx| {
                        let mut input_node = accesskit::Node::new(Role::TextInput);
                        Input::new(&input)
                            .accessibility_label("Project name")
                            .render_base(window, cx)
                            .render(window, cx)
                            .into_element()
                            .write_a11y_info(&mut input_node);

                        let mut textarea_node = accesskit::Node::new(Role::MultilineTextInput);
                        crate::ui::textarea::Textarea::new(&textarea)
                            .accessibility_label("Message")
                            .render_base(window, cx)
                            .render(window, cx)
                            .into_element()
                            .write_a11y_info(&mut textarea_node);
                        *captured.lock().unwrap() = Some((input_node, textarea_node));
                    },
                    |_, (), _, _| {},
                )
            }
        }

        install_text_control_test_globals(cx);
        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, window) = cx.add_window_view(move |window, cx| AccessibilityHarness {
            input: cx.new(|cx| InputState::new(window, cx)),
            textarea: cx.new(|cx| gpui_base::input::TextareaState::new(window, cx)),
            captured,
        });
        window.update(|window, cx| window.draw(cx).clear(cx));

        let (input, textarea) = result.lock().unwrap().take().unwrap();
        assert_eq!(input.role(), Role::TextInput);
        assert_eq!(input.label(), Some("Project name"));
        assert_eq!(textarea.role(), Role::MultilineTextInput);
        assert_eq!(textarea.label(), Some("Message"));
    }

    #[test]
    fn sizes_match_the_react_input_contract() {
        assert_eq!(InputSize::Default.height().to_pixels(px(16.0)), px(32.0));
        assert_eq!(InputSize::Small.height().to_pixels(px(16.0)), px(28.0));
        assert_eq!(InputSize::Default.height().to_pixels(px(18.0)), px(36.0));
    }

    #[test]
    fn radius_and_border_match_the_react_input_contract() {
        assert_eq!(RADIUS_MD, px(10.56));
        assert_eq!(BORDER_DEFAULT, px(0.5));
    }

    #[test]
    fn editor_caret_uses_foreground_in_dark_mode() {
        let theme = G2VTheme::for_appearance(WindowAppearance::Dark);
        let style = InputPresentation::new(&theme, false, false).editor_style();

        assert_eq!(style.caret, theme.foreground);
        assert_eq!(style.muted_foreground, theme.muted_foreground);
    }

    #[test]
    fn invalid_state_takes_precedence_over_focus() {
        let theme = G2VTheme::for_appearance(WindowAppearance::Light);

        assert_eq!(input_border(&theme, false, true, true), theme.destructive);
    }

    #[test]
    fn dark_background_matches_the_shadcn_opacity_contract() {
        let theme = G2VTheme::for_appearance(WindowAppearance::Dark);

        assert_eq!(
            input_background(&theme, true, false),
            theme.input.opacity(0.3)
        );
        assert_eq!(
            input_background(&theme, true, true),
            theme.input.opacity(0.8)
        );
    }
}
