//! G2V's styled source-code editor built on the unstyled GPUI Base behavior.

use std::{
    cell::{Cell, RefCell},
    ops::Range,
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use gpui::{
    App, Context, Entity, FontStyle, FontWeight, HighlightStyle, Hsla, IntoElement, RenderOnce,
    SharedString, Task, Window, div, prelude::*, rgb,
};
use gpui_base::{
    ElementExt as _,
    input::{
        DiagnosticColors, Editor as EditorBehavior, EditorState, FoldRange, HighlightStyleResolver,
        InputEdit, InputEditorStyle, InputHighlighter, Rope,
    },
};
use gpui_component::highlighter::{LanguageRegistry, SyntaxHighlighter};

use crate::theme::{colors::G2VTheme, typography};

use super::{TextStyleTokenExt as _, input::reconcile_presentation};

pub fn code_editor_state(
    language: &str,
    window: &mut Window,
    cx: &mut Context<EditorState>,
) -> EditorState {
    let mut state = EditorState::new(window, cx)
        .language(language)
        .line_number(true)
        .folding(true)
        .soft_wrap(true);
    state.set_highlighter_factory(tree_sitter_highlighter_factory(), cx);
    state
}

#[derive(IntoElement)]
pub struct CodeEditor {
    state: Entity<EditorState>,
    readonly: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct CodeEditorPresentation {
    appearance: gpui::WindowAppearance,
    foreground: Hsla,
    muted_foreground: Hsla,
    surface: Hsla,
    border: Hsla,
    ring: Hsla,
    destructive: Hsla,
    warning: Hsla,
    info: Hsla,
    active_line: Hsla,
    readonly: bool,
}

impl CodeEditorPresentation {
    fn new(theme: &G2VTheme, appearance: gpui::WindowAppearance, readonly: bool) -> Self {
        Self {
            appearance,
            foreground: theme.foreground,
            muted_foreground: theme.muted_foreground,
            surface: theme.surface,
            border: theme.border,
            ring: theme.ring,
            destructive: theme.destructive,
            warning: theme.orange_600,
            info: theme.blue_500,
            active_line: theme.surface_hover,
            readonly,
        }
    }
}

impl CodeEditor {
    #[must_use]
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
            readonly: false,
        }
    }

    #[must_use]
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }
}

impl RenderOnce for CodeEditor {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let appearance = window.appearance();
        let editor_style = code_editor_style(&theme, appearance);
        let presentation = CodeEditorPresentation::new(&theme, appearance, self.readonly);
        let presentation_cache = window.use_keyed_state(
            ("g2v-code-editor-presentation", self.state.entity_id()),
            cx,
            |_, _| None::<CodeEditorPresentation>,
        );
        let editor_state = self.state.clone();
        let typography = typography::tokens();
        let editor_text = typography::CODE_EDITOR_TEXT;

        div()
            .size_full()
            .min_h_0()
            .min_w_0()
            .flex()
            .font_family(typography.mono.clone())
            .text_size(editor_text.font_size_rems())
            .line_height(editor_text.line_height_rems())
            .bg(theme.surface)
            .on_prepaint(move |_, _, cx| {
                reconcile_presentation(&presentation_cache, presentation, cx, |cx| {
                    editor_state.update(cx, |state, cx| {
                        state.set_editor_style(editor_style.clone());
                        state.set_readonly(presentation.readonly, cx);
                    });
                });
            })
            .child(EditorBehavior::new(&self.state))
    }
}

fn code_editor_style(theme: &G2VTheme, appearance: gpui::WindowAppearance) -> InputEditorStyle {
    let highlight_styles: Arc<dyn HighlightStyleResolver> = match appearance {
        gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => {
            Arc::new(G2VSyntaxPalette::dark())
        }
        gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => {
            Arc::new(G2VSyntaxPalette::light())
        }
    };
    InputEditorStyle {
        foreground: theme.foreground,
        muted_foreground: theme.muted_foreground,
        background: theme.surface,
        border: theme.border,
        selection: theme.ring.opacity(0.3),
        caret: theme.foreground,
        diagnostics: DiagnosticColors {
            error: theme.destructive,
            warning: theme.orange_600,
            info: theme.blue_500,
            hint: theme.muted_foreground,
        },
        highlight_styles,
        editor_active_line: Some(theme.surface_hover),
        editor_gutter_background: Some(theme.surface),
        ..InputEditorStyle::default()
    }
}

/// G2V's syntax colors mapped onto GPUI Base's Tree-sitter highlight names.
#[derive(Clone, Copy)]
struct G2VSyntaxPalette {
    foreground: Hsla,
    comment: Hsla,
    string: Hsla,
    number: Hsla,
    constant: Hsla,
    variable: Hsla,
    keyword: Hsla,
    function: Hsla,
    r#type: Hsla,
    punctuation: Hsla,
    tag: Hsla,
    regex: Hsla,
    escape: Hsla,
    attribute: Hsla,
}

impl G2VSyntaxPalette {
    fn dark() -> Self {
        Self {
            foreground: rgb(0x00f1_f1f1).into(),
            comment: rgb(0x008b_8b94).into(),
            string: rgb(0x006f_cf97).into(),
            number: rgb(0x0060_a5fa).into(),
            constant: rgb(0x00f4_bf75).into(),
            variable: rgb(0x00f5_9e75).into(),
            keyword: rgb(0x00f4_72b6).into(),
            function: rgb(0x00a7_8bfa).into(),
            r#type: rgb(0x00c0_84fc).into(),
            punctuation: rgb(0x0071_717a).into(),
            tag: rgb(0x00fb_7185).into(),
            regex: rgb(0x0067_e8f9).into(),
            escape: rgb(0x005e_ead4).into(),
            attribute: rgb(0x0086_efac).into(),
        }
    }

    fn light() -> Self {
        Self {
            foreground: rgb(0x0020_2024).into(),
            comment: rgb(0x006b_6b75).into(),
            string: rgb(0x0027_864a).into(),
            number: rgb(0x0025_63a8).into(),
            constant: rgb(0x009a_6700).into(),
            variable: rgb(0x00a1_4f21).into(),
            keyword: rgb(0x00be_185d).into(),
            function: rgb(0x006d_28d9).into(),
            r#type: rgb(0x007e_22ce).into(),
            punctuation: rgb(0x0052_525b).into(),
            tag: rgb(0x00b4_233f).into(),
            regex: rgb(0x000e_7490).into(),
            escape: rgb(0x000f_766e).into(),
            attribute: rgb(0x0016_834a).into(),
        }
    }

    fn colored(color: Hsla) -> HighlightStyle {
        HighlightStyle {
            color: Some(color),
            ..HighlightStyle::default()
        }
    }
}

impl HighlightStyleResolver for G2VSyntaxPalette {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        let style = match name {
            "attribute" => Self::colored(self.attribute),
            "boolean" | "number" => Self::colored(self.number),
            "comment" | "comment.doc" | "hint" => Self::colored(self.comment),
            "constant" | "label" | "variable.special" => Self::colored(self.constant),
            "constructor" | "function" | "link_text" => Self::colored(self.function),
            "enum" | "type" | "variant" => Self::colored(self.r#type),
            "keyword" | "preproc" | "tag.doctype" => Self::colored(self.keyword),
            "operator"
            | "punctuation"
            | "punctuation.bracket"
            | "punctuation.delimiter"
            | "punctuation.list_marker" => Self::colored(self.punctuation),
            "property" | "variable" => Self::colored(self.variable),
            "punctuation.special" | "tag" | "title" => Self::colored(self.tag),
            "string"
            | "string.special"
            | "string.special.symbol"
            | "text.code.span"
            | "text.literal"
            | "link_uri" => Self::colored(self.string),
            "string.escape" => Self::colored(self.escape),
            "string.regex" => Self::colored(self.regex),
            "emphasis" => HighlightStyle {
                color: Some(self.keyword),
                font_style: Some(FontStyle::Italic),
                ..HighlightStyle::default()
            },
            "emphasis.strong" => HighlightStyle {
                color: Some(self.constant),
                font_weight: Some(FontWeight::BOLD),
                ..HighlightStyle::default()
            },
            "embedded" | "predictive" | "primary" => Self::colored(self.foreground),
            _ => return None,
        };
        Some(style)
    }
}

fn tree_sitter_highlighter_factory() -> gpui_base::input::InputHighlighterFactory {
    Rc::new(|language| {
        LanguageRegistry::singleton()
            .language(language)
            .filter(gpui_component::highlighter::LanguageConfig::has_grammar)
            .map(|_| Box::new(TreeSitterHighlighter::new(language)) as Box<dyn InputHighlighter>)
    })
}

pub type SourceLineHighlights = Vec<Vec<(Range<usize>, HighlightStyle)>>;

#[must_use]
pub fn highlight_source_lines(
    source: &str,
    language: &str,
    appearance: gpui::WindowAppearance,
) -> SourceLineHighlights {
    let Some(configuration) = LanguageRegistry::singleton().language(language) else {
        return source.lines().map(|_| Vec::new()).collect();
    };
    if !configuration.has_grammar() {
        return source.lines().map(|_| Vec::new()).collect();
    }
    let resolver = match appearance {
        gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => {
            G2VSyntaxPalette::dark()
        }
        gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight => {
            G2VSyntaxPalette::light()
        }
    };
    let rope = Rope::from(source);
    let mut highlighter = SyntaxHighlighter::new(language);
    highlighter.update(None, &rope, None);
    let mut offset = 0;
    source
        .split_terminator('\n')
        .map(|line| {
            let range = offset..offset + line.len();
            let highlights = highlighter
                .styles(&range, &resolver)
                .into_iter()
                .filter_map(|(highlight, style)| {
                    let start = highlight.start.max(range.start) - range.start;
                    let end = highlight.end.min(range.end) - range.start;
                    (start < end).then_some((start..end, style))
                })
                .collect();
            offset += line.len() + 1;
            highlights
        })
        .collect()
}

struct TreeSitterHighlighter {
    language: SharedString,
    highlighter: Rc<RefCell<SyntaxHighlighter>>,
    parse_task: Rc<RefCell<Option<Task<()>>>>,
    parse_revision: Rc<Cell<u64>>,
}

impl TreeSitterHighlighter {
    fn new(language: &str) -> Self {
        Self {
            language: language.to_owned().into(),
            highlighter: Rc::new(RefCell::new(SyntaxHighlighter::new(language))),
            parse_task: Rc::new(RefCell::new(None)),
            parse_revision: Rc::new(Cell::new(0)),
        }
    }
}

impl InputHighlighter for TreeSitterHighlighter {
    fn language(&self) -> SharedString {
        self.language.clone()
    }

    fn update(
        &mut self,
        edit: Option<InputEdit>,
        text: &Rope,
        _: bool,
        window: &mut Window,
        cx: &mut Context<EditorState>,
    ) {
        const FOREGROUND_PARSE_BUDGET: Duration = Duration::from_millis(2);
        const EDIT_PARSE_DEBOUNCE: Duration = Duration::from_millis(150);

        let initial_parse = self.highlighter.borrow().tree().is_none();
        let completed = !initial_parse
            && self.highlighter.borrow_mut().update(
                edit.map(tree_sitter_input_edit),
                text,
                Some(FOREGROUND_PARSE_BUDGET),
            );
        self.parse_revision
            .set(self.parse_revision.get().wrapping_add(1));
        let revision = self.parse_revision.get();
        if completed {
            self.parse_task.borrow_mut().take();
            return;
        }

        let language = self.language.clone();
        let text = text.clone();
        let highlighter = self.highlighter.clone();
        let parse_revision = self.parse_revision.clone();
        let delay = (!initial_parse).then_some(EDIT_PARSE_DEBOUNCE);
        let task = cx.spawn_in(window, async move |editor, cx| {
            if let Some(delay) = delay {
                cx.background_executor().timer(delay).await;
            }
            let parsed = cx
                .background_executor()
                .spawn(async move {
                    let mut highlighter = SyntaxHighlighter::new(&language);
                    highlighter.update(None, &text, None);
                    highlighter
                })
                .await;
            if parse_revision.get() != revision {
                return;
            }
            *highlighter.borrow_mut() = parsed;
            let _ = editor.update(cx, |state, cx| {
                state.apply_highlighter_fold_candidates(Vec::new(), cx);
            });
        });
        self.parse_task.borrow_mut().replace(task);
    }

    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        self.highlighter.borrow().styles(range, resolver)
    }

    fn fold_ranges(&self, _: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
}

fn tree_sitter_input_edit(edit: InputEdit) -> tree_sitter::InputEdit {
    tree_sitter::InputEdit {
        start_byte: edit.start_byte,
        old_end_byte: edit.old_end_byte,
        new_end_byte: edit.new_end_byte,
        start_position: tree_sitter::Point::new(
            edit.start_position.row,
            edit.start_position.column,
        ),
        old_end_position: tree_sitter::Point::new(
            edit.old_end_position.row,
            edit.old_end_position.column,
        ),
        new_end_position: tree_sitter::Point::new(
            edit.new_end_position.row,
            edit.new_end_position.column,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Render, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    struct SyntaxParseHarness {
        editor: Entity<EditorState>,
    }

    impl Render for SyntaxParseHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn initial_syntax_parse_does_not_complete_on_the_foreground(cx: &mut TestAppContext) {
        let highlighter = Rc::new(RefCell::new(TreeSitterHighlighter::new("rust")));
        let (harness, window) = cx.add_window_view(|window, cx| SyntaxParseHarness {
            editor: cx.new(|cx| EditorState::new(window, cx).language("rust")),
        });

        harness.update_in(window, |harness, window, cx| {
            harness.editor.update(cx, |_, cx| {
                highlighter.borrow_mut().update(
                    None,
                    &Rope::from("fn main() { println!(\"hello\"); }"),
                    false,
                    window,
                    cx,
                );
            });
        });

        assert!(
            highlighter.borrow().highlighter.borrow().tree().is_none(),
            "the initial file-preview parse must be deferred to the background"
        );
    }

    #[gpui::test]
    fn background_syntax_parse_applies_only_the_latest_revision(cx: &mut TestAppContext) {
        let highlighter = Rc::new(RefCell::new(TreeSitterHighlighter::new("rust")));
        let (harness, window) = cx.add_window_view(|window, cx| SyntaxParseHarness {
            editor: cx.new(|cx| EditorState::new(window, cx).language("rust")),
        });
        let stale = Rope::from("fn stale() {}");
        let latest = Rope::from("fn latest_revision_is_longer() {}");

        harness.update_in(window, |harness, window, cx| {
            harness.editor.update(cx, |_, cx| {
                highlighter
                    .borrow_mut()
                    .update(None, &stale, false, window, cx);
                highlighter
                    .borrow_mut()
                    .update(None, &latest, false, window, cx);
            });
        });
        window.run_until_parked();

        assert_eq!(
            highlighter
                .borrow()
                .highlighter
                .borrow()
                .tree()
                .expect("the latest background parse must complete")
                .root_node()
                .end_byte(),
            latest.len()
        );
    }

    #[test]
    fn editor_edits_are_preserved_for_incremental_tree_sitter_updates() {
        let edit = InputEdit {
            start_byte: 4,
            old_end_byte: 7,
            new_end_byte: 9,
            start_position: gpui_base::input::Point::new(1, 2),
            old_end_position: gpui_base::input::Point::new(1, 5),
            new_end_position: gpui_base::input::Point::new(2, 1),
        };

        let converted = tree_sitter_input_edit(edit);

        assert_eq!(converted.start_byte, 4);
        assert_eq!(converted.old_end_byte, 7);
        assert_eq!(converted.new_end_byte, 9);
        assert_eq!(converted.start_position, tree_sitter::Point::new(1, 2));
        assert_eq!(converted.old_end_position, tree_sitter::Point::new(1, 5));
        assert_eq!(converted.new_end_position, tree_sitter::Point::new(2, 1));
    }

    #[test]
    fn tree_sitter_factory_supports_primary_workspace_languages() {
        let factory = tree_sitter_highlighter_factory();
        assert!(factory("rust").is_some());
        assert!(factory("typescript").is_some());
        assert!(factory("tsx").is_some());
        assert!(factory("text").is_none());
    }

    #[test]
    fn syntax_palette_matches_representative_light_and_dark_tokens() {
        let dark = G2VSyntaxPalette::dark();
        let light = G2VSyntaxPalette::light();

        assert_eq!(
            dark.style("keyword").unwrap().color,
            Some(rgb(0x00f4_72b6).into())
        );
        assert_eq!(
            dark.style("string").unwrap().color,
            Some(rgb(0x006f_cf97).into())
        );
        assert_eq!(
            light.style("keyword").unwrap().color,
            Some(rgb(0x00be_185d).into())
        );
        assert_eq!(
            light.style("string").unwrap().color,
            Some(rgb(0x0027_864a).into())
        );
    }

    #[test]
    fn syntax_palette_preserves_markup_emphasis() {
        let palette = G2VSyntaxPalette::dark();

        assert_eq!(
            palette.style("emphasis").unwrap().font_style,
            Some(FontStyle::Italic)
        );
        assert_eq!(
            palette.style("emphasis.strong").unwrap().font_weight,
            Some(FontWeight::BOLD)
        );
        assert!(palette.style("unsupported.scope").is_none());
    }

    #[test]
    fn syntax_palette_covers_editor_highlight_names() {
        let palette = G2VSyntaxPalette::dark();
        let names = [
            "attribute",
            "boolean",
            "comment",
            "comment.doc",
            "constant",
            "constructor",
            "embedded",
            "emphasis",
            "emphasis.strong",
            "enum",
            "function",
            "hint",
            "keyword",
            "label",
            "link_text",
            "link_uri",
            "number",
            "operator",
            "predictive",
            "preproc",
            "primary",
            "property",
            "punctuation",
            "punctuation.bracket",
            "punctuation.delimiter",
            "punctuation.list_marker",
            "punctuation.special",
            "string",
            "string.escape",
            "string.regex",
            "string.special",
            "string.special.symbol",
            "tag",
            "tag.doctype",
            "text.code.span",
            "text.literal",
            "title",
            "type",
            "variable",
            "variable.special",
            "variant",
        ];

        for name in names {
            assert!(
                palette.style(name).is_some(),
                "missing syntax mapping for {name}"
            );
        }
    }

    #[test]
    fn source_highlights_are_local_to_each_rendered_line() {
        let source = "fn first() {}\nlet value = 1;";
        let lines = highlight_source_lines(source, "rust", gpui::WindowAppearance::Dark);

        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| !line.is_empty()));
        for (source_line, highlights) in source.lines().zip(lines) {
            assert!(
                highlights
                    .iter()
                    .all(|(range, _)| range.start < range.end && range.end <= source_line.len())
            );
        }
    }
}
