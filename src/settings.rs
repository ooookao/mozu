use std::path::Path;

use g2v_ui::{
    theme::{colors::G2VTheme, typography},
    ui::{
        Button, ButtonSize, ButtonVariant, IconName, Input, InputSize, InputState, Switch,
        TextStyleExt as _,
    },
};
use gpui::{Context, Entity, FontWeight, Hsla, IntoElement, Window, div, prelude::*, px};
use gpui_base::input::InputEvent;
use mozu_audio::{
    Cancellation, Error, HaasEngine, Result,
    models::HaasVariant,
    vocabulary::{Vocabulary, VocabularyConfig},
};
use objc2_app_kit::NSWorkspace;
use objc2_avf_audio::{AVAudioApplication, AVAudioApplicationRecordPermission};
use objc2_foundation::{NSURL, NSUserDefaults, ns_string};

use crate::{
    LoadedModels, MozuApp,
    history::{DictionaryEntry, History},
    island::Anchor,
    segment_track,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DictionaryForm {
    Closed,
    New,
    Edit(i64),
}

pub(crate) struct DictionaryEditor {
    written: Entity<InputState>,
    reading: Entity<InputState>,
    search: Entity<InputState>,
    form: DictionaryForm,
    entries: Vec<DictionaryEntry>,
    error: Option<String>,
    saving: bool,
}

impl DictionaryEditor {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<MozuApp>) -> Self {
        let written = cx.new(|cx| InputState::new(window, cx).placeholder("表記"));
        let reading = cx.new(|cx| InputState::new(window, cx).placeholder("読み（任意）"));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("単語を検索"));
        cx.subscribe(&search, |_, _, _: &InputEvent, cx| cx.notify())
            .detach();
        for field in [&written, &reading] {
            cx.subscribe_in(
                field,
                window,
                |view, _, event: &InputEvent, window, cx| match event {
                    InputEvent::PressEnter { .. } => save_dictionary(view, window, cx),
                    InputEvent::Change => {
                        view.dictionary.error = None;
                        cx.notify();
                    }
                    _ => {}
                },
            )
            .detach();
        }
        Self {
            written,
            reading,
            search,
            form: DictionaryForm::Closed,
            entries: Vec::new(),
            error: None,
            saving: false,
        }
    }

    pub(crate) fn refresh(&mut self) {
        match History::open().and_then(|history| history.dictionary()) {
            Ok(entries) => {
                self.entries = entries;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn open(&mut self, form: DictionaryForm, window: &mut Window, cx: &mut Context<MozuApp>) {
        self.form = form;
        self.error = None;
        let entry = match form {
            DictionaryForm::Edit(id) => self.entries.iter().find(|entry| entry.id == id),
            _ => None,
        };
        let written = entry.map(|entry| entry.written.as_str()).unwrap_or("");
        let reading = entry
            .and_then(|entry| entry.reading.as_deref())
            .unwrap_or("");
        self.written
            .update(cx, |state, cx| state.set_value(written, window, cx));
        self.reading
            .update(cx, |state, cx| state.set_value(reading, window, cx));
        self.written.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<MozuApp>) {
        self.form = DictionaryForm::Closed;
        self.error = None;
        self.written
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.reading
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    fn remove(&mut self, id: i64, window: &mut Window, cx: &mut Context<MozuApp>) {
        match History::open().and_then(|history| history.delete_dictionary_entry(id)) {
            Ok(()) => {
                if self.form == DictionaryForm::Edit(id) {
                    self.close(window, cx);
                }
                self.refresh();
            }
            Err(error) => self.error = Some(dictionary_error(error)),
        }
        cx.notify();
    }
}

fn dictionary_error(error: Error) -> String {
    match error {
        Error::Invalid(message) => message,
        error => error.to_string(),
    }
}

fn compile_dictionary(
    engine: &HaasEngine,
    tokenizer: &Path,
    entries: &[DictionaryEntry],
) -> Result<()> {
    let terms = entries
        .iter()
        .map(DictionaryEntry::vocabulary_term)
        .collect::<Result<Vec<_>>>()?;
    let vocabulary = Vocabulary::new(terms, VocabularyConfig::standard())?;
    engine.compile_vocabulary(vocabulary, tokenizer).map(|_| ())
}

fn validate_dictionary(
    engine: &HaasEngine,
    tokenizer: &Path,
    entries: &[DictionaryEntry],
    candidate: &DictionaryEntry,
) -> Result<()> {
    if entries
        .iter()
        .any(|entry| entry.written == candidate.written)
    {
        return Err(Error::Invalid(
            "同じ表記の単語がすでに登録されています。".into(),
        ));
    }
    compile_dictionary(engine, tokenizer, std::slice::from_ref(candidate)).map_err(|error| {
        eprintln!(
            "Dictionary entry cannot be compiled: written={:?}, error={error}",
            candidate.written
        );
        Error::Invalid("この表記または読みは、選択中のモデルでは使用できません。".into())
    })?;
    let mut terms = entries
        .iter()
        .map(DictionaryEntry::vocabulary_term)
        .collect::<Result<Vec<_>>>()?;
    terms.push(candidate.vocabulary_term()?);
    let combined = Vocabulary::new(terms, VocabularyConfig::standard())?;
    let Err(error) = engine.compile_vocabulary(combined, tokenizer) else {
        return Ok(());
    };
    eprintln!(
        "Dictionary validation failed: written={:?}, error={error}",
        candidate.written
    );
    for entry in entries {
        compile_dictionary(engine, tokenizer, std::slice::from_ref(entry))?;
        let pair = Vocabulary::new(
            vec![entry.vocabulary_term()?, candidate.vocabulary_term()?],
            VocabularyConfig::standard(),
        )?;
        if let Err(conflict) = engine.compile_vocabulary(pair, tokenizer) {
            eprintln!(
                "Dictionary entries conflict: first={:?}, second={:?}, error={conflict}",
                entry.written, candidate.written
            );
            return Err(Error::Invalid(format!(
                "「{}」と同じ読みとして認識されます。表記を統一するか、読みを変更してください。",
                entry.written
            )));
        }
    }
    Err(error)
}

fn save_dictionary(view: &mut MozuApp, window: &Window, cx: &mut Context<MozuApp>) {
    if view.dictionary.saving || view.dictionary.form == DictionaryForm::Closed {
        return;
    }
    if view.run.is_some() || view.models.is_busy() {
        view.dictionary.error = Some("音声の処理が終了してから保存してください。".into());
        cx.notify();
        return;
    }
    let written = view.dictionary.written.read(cx).value().trim().to_owned();
    let reading = view.dictionary.reading.read(cx).value().trim().to_owned();
    if written.is_empty() {
        view.dictionary.error = Some("表記を入力してください。".into());
        cx.notify();
        return;
    }
    let id = match view.dictionary.form {
        DictionaryForm::Edit(id) => Some(id),
        _ => None,
    };
    let reading = (!reading.is_empty()).then_some(reading);
    let Some(root) = view.models.root.clone() else {
        view.dictionary.error = Some("モデルの保存先を確認できませんでした。".into());
        cx.notify();
        return;
    };
    if !view.models.encoder_is_installed(view.preferences.variant) {
        view.dictionary.error = Some("モデルをダウンロードしてから保存してください。".into());
        cx.notify();
        return;
    }
    let variant = view.preferences.variant;
    let (sender, receiver) = async_channel::bounded(1);
    view.dictionary.saving = true;
    view.dictionary.error = None;
    let job = Box::new(move |loaded: &mut Option<LoadedModels>| {
        let result = (|| {
            let models = LoadedModels::get(loaded, root, variant, &Cancellation::new())?;
            let tokenizer = models.files.assets.with_file_name("tokenizer.model");
            History::open()?.save_dictionary_entry_validated(
                id,
                &written,
                reading.as_deref(),
                |entries, candidate| {
                    validate_dictionary(&models.engine, &tokenizer, entries, candidate)
                },
            )
        })();
        if sender.try_send(result).is_err() {
            eprintln!("Dictionary validation result receiver was closed");
        }
    });
    if view.recognition_jobs.send(job).is_err() {
        view.dictionary.saving = false;
        view.dictionary.error = Some("辞書を検証する処理を開始できませんでした。".into());
        cx.notify();
        return;
    }
    cx.spawn_in(window, async move |view, cx| {
        let result = receiver.recv().await;
        let _ = view.update_in(cx, |view, window, cx| {
            view.dictionary.saving = false;
            match result {
                Ok(Ok(())) => {
                    view.dictionary.close(window, cx);
                    view.dictionary.refresh();
                }
                Ok(Err(error)) => view.dictionary.error = Some(dictionary_error(error)),
                Err(error) => {
                    view.dictionary.error = Some(format!("辞書の検証が中断されました。{error}"))
                }
            }
            cx.notify();
        });
    })
    .detach();
    cx.notify();
}

pub(crate) fn dictionary_add_button(view: &MozuApp, cx: &Context<MozuApp>) -> Button {
    Button::new("add-dictionary-word")
        .icon(IconName::PlusMedium)
        .size(ButtonSize::IconSmall)
        .variant(ButtonVariant::Ghost)
        .accessibility_label("単語を追加")
        .disabled(view.dictionary.saving || view.run.is_some() || view.models.is_busy())
        .on_click(cx.listener(|view, _, window, cx| {
            view.dictionary
                .search
                .update(cx, |state, cx| state.set_value("", window, cx));
            view.dictionary.open(DictionaryForm::New, window, cx);
        }))
}

fn dictionary_form(
    editor: &DictionaryEditor,
    theme: &G2VTheme,
    cx: &Context<MozuApp>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .py_3()
        .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape" && !view.dictionary.saving {
                view.dictionary.close(window, cx);
                cx.stop_propagation();
            }
        }))
        .child(
            div()
                .flex()
                .gap_3()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("表記"),
                        )
                        .child(
                            Input::new(&editor.written)
                                .size(InputSize::Small)
                                .disabled(editor.saving)
                                .invalid(editor.error.is_some())
                                .accessibility_label("表記"),
                        ),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("読み"),
                        )
                        .child(
                            Input::new(&editor.reading)
                                .size(InputSize::Small)
                                .disabled(editor.saving)
                                .invalid(editor.error.is_some())
                                .accessibility_label("読み"),
                        ),
                ),
        )
        .children(editor.error.as_ref().map(|error| {
            div()
                .text_xs()
                .text_color(theme.destructive)
                .child(error.clone())
        }))
        .child(
            div()
                .flex()
                .justify_end()
                .gap_2()
                .child(
                    Button::new("cancel-word-edit")
                        .label("キャンセル")
                        .size(ButtonSize::Small)
                        .variant(ButtonVariant::Ghost)
                        .disabled(editor.saving)
                        .on_click(
                            cx.listener(|view, _, window, cx| view.dictionary.close(window, cx)),
                        ),
                )
                .child(
                    Button::new("save-word")
                        .label(if editor.saving { "確認中" } else { "保存" })
                        .size(ButtonSize::Small)
                        .disabled(
                            editor.saving || editor.written.read(cx).value().trim().is_empty(),
                        )
                        .on_click(
                            cx.listener(|view, _, window, cx| save_dictionary(view, window, cx)),
                        ),
                ),
        )
}

pub(crate) fn render_dictionary(
    view: &MozuApp,
    theme: &G2VTheme,
    cx: &Context<MozuApp>,
) -> impl IntoElement {
    let editor = &view.dictionary;
    let query = editor.search.read(cx).value().trim().to_lowercase();
    let entries = editor
        .entries
        .iter()
        .filter(|entry| {
            query.is_empty()
                || entry.written.to_lowercase().contains(&query)
                || entry
                    .reading
                    .as_ref()
                    .is_some_and(|reading| reading.to_lowercase().contains(&query))
                || editor.form == DictionaryForm::Edit(entry.id)
        })
        .collect::<Vec<_>>();
    let empty = entries.is_empty();
    let rows = entries.into_iter().map(|entry| {
        let id = entry.id;
        let editing = editor.form == DictionaryForm::Edit(id);
        div()
            .border_b_1()
            .border_color(theme.border.opacity(0.5))
            .when(editing, |row| row.child(dictionary_form(editor, theme, cx)))
            .when(!editing, |row| {
                row.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .min_h(px(42.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .child(entry.written.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(theme.muted_foreground)
                                .child(entry.reading.clone().unwrap_or_else(|| "—".into())),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .gap_1()
                                .child(
                                    Button::new(("edit-word", id as u64))
                                        .icon(IconName::PencilEdit)
                                        .size(ButtonSize::IconTiny)
                                        .variant(ButtonVariant::Ghost)
                                        .accessibility_label(format!("{}を編集", entry.written))
                                        .disabled(editor.saving)
                                        .on_click(cx.listener(move |view, _, window, cx| {
                                            view.dictionary.open(
                                                DictionaryForm::Edit(id),
                                                window,
                                                cx,
                                            )
                                        })),
                                )
                                .child(
                                    Button::new(("delete-word", id as u64))
                                        .icon(IconName::Trash)
                                        .size(ButtonSize::IconTiny)
                                        .variant(ButtonVariant::Ghost)
                                        .accessibility_label(format!("{}を削除", entry.written))
                                        .disabled(editor.saving)
                                        .on_click(cx.listener(move |view, _, window, cx| {
                                            view.dictionary.remove(id, window, cx)
                                        })),
                                ),
                        ),
                )
            })
    });
    div()
        .id("dictionary")
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .px_6()
        .text_style(typography::TEXT_13)
        .font_weight(FontWeight::NORMAL)
        .child(
            div().flex_none().pb_3().child(
                Input::new(&editor.search)
                    .size(InputSize::Small)
                    .accessibility_label("辞書を検索"),
            ),
        )
        .children((editor.form == DictionaryForm::New).then(|| {
            div()
                .flex_none()
                .border_b_1()
                .border_color(theme.border)
                .child(dictionary_form(editor, theme, cx))
        }))
        .children(
            (editor.form == DictionaryForm::Closed)
                .then(|| {
                    editor.error.as_ref().map(|error| {
                        div()
                            .text_xs()
                            .py_2()
                            .text_color(theme.destructive)
                            .child(error.clone())
                    })
                })
                .flatten(),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap_3()
                .py_2()
                .text_xs()
                .text_color(theme.muted_foreground)
                .child(div().flex_1().child("表記"))
                .child(div().flex_1().child("読み"))
                .child(div().w(px(52.0))),
        )
        .child(
            div()
                .id("dictionary-list")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .children(rows)
                .children(empty.then(|| {
                    div()
                        .py_6()
                        .text_color(theme.muted_foreground)
                        .child(if query.is_empty() {
                            "単語はまだありません"
                        } else {
                            "一致する単語がありません"
                        })
                })),
        )
}
#[derive(Clone, Copy)]
pub(crate) struct Preferences {
    pub(crate) variant: HaasVariant,
    pub(crate) show_island: bool,
    pub(crate) anchor: Anchor,
}

impl Preferences {
    pub(crate) fn load() -> Self {
        let defaults = NSUserDefaults::standardUserDefaults();
        let variant = match defaults.integerForKey(ns_string!("EncoderFrames")) {
            1500 => HaasVariant::Frames1500,
            _ => HaasVariant::Frames3072,
        };
        let show_island = defaults
            .objectForKey(ns_string!("ShowDictationIsland"))
            .is_none()
            || defaults.boolForKey(ns_string!("ShowDictationIsland"));
        let anchor =
            Anchor::from_saved(defaults.integerForKey(ns_string!("DictationIslandAnchor")));
        Self {
            variant,
            show_island,
            anchor,
        }
    }

    pub(crate) fn save(self) {
        let defaults = NSUserDefaults::standardUserDefaults();
        let frames = match self.variant {
            HaasVariant::Frames1500 => 1500,
            HaasVariant::Frames3072 => 3072,
        };
        defaults.setInteger_forKey(frames, ns_string!("EncoderFrames"));
        defaults.setBool_forKey(self.show_island, ns_string!("ShowDictationIsland"));
        defaults.setInteger_forKey(self.anchor as isize, ns_string!("DictationIslandAnchor"));
    }
}

pub(crate) fn microphone_permission() -> AVAudioApplicationRecordPermission {
    // The shared application reports permission for this process.
    unsafe { AVAudioApplication::sharedInstance().recordPermission() }
}

pub(crate) fn request_microphone_permission(cx: &mut Context<MozuApp>) {
    let (sender, receiver) = async_channel::bounded(1);
    let completion = block2::RcBlock::new(move |_: objc2::runtime::Bool| {
        let _ = sender.try_send(());
    });
    // AVFAudio copies the block and may run it off the main thread.
    unsafe { AVAudioApplication::requestRecordPermissionWithCompletionHandler(&completion) };
    cx.spawn(async move |view, cx| {
        if receiver.recv().await.is_ok() {
            let _ = view.update(cx, |_, cx| cx.notify());
        }
    })
    .detach();
}

pub(crate) fn render(
    demo: &MozuApp,
    shortcut: Option<Button>,
    theme: &G2VTheme,
    cx: &Context<MozuApp>,
) -> impl IntoElement {
    let preferences = demo.preferences;
    let active = demo.run.is_some();
    let microphone = match microphone_permission() {
        AVAudioApplicationRecordPermission::Granted => div().text_sm().child("許可済み"),
        AVAudioApplicationRecordPermission::Undetermined => div().child(
            Button::new("microphone-permission")
                .label("マイクを許可する")
                .size(ButtonSize::Small)
                .variant(ButtonVariant::Outline)
                .on_click(cx.listener(|_, _, _, cx| request_microphone_permission(cx))),
        ),
        _ => div().child(
            Button::new("microphone-settings")
                .label("システム設定を開く")
                .size(ButtonSize::Small)
                .variant(ButtonVariant::Outline)
                .on_click(cx.listener(|_, _, _, _| {
                    let url = NSURL::URLWithString(ns_string!(
                        "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
                    ))
                    .expect("valid microphone settings URL");
                    if !NSWorkspace::sharedWorkspace().openURL(&url) {
                        eprintln!("Could not open macOS microphone settings");
                    }
                })),
        ),
    };
    let encoders = [
        ("15 秒", HaasVariant::Frames1500),
        ("30.72 秒", HaasVariant::Frames3072),
    ]
    .into_iter()
    .map(|(label, variant)| {
        let installed = demo.models.encoder_is_installed(variant);
        Button::new(label)
            .label(label)
            .size(ButtonSize::Small)
            .rounded_full()
            .variant(ButtonVariant::SurfaceOption)
            .selected(installed && preferences.variant == variant)
            .disabled(active || !installed || demo.models.is_busy())
            .on_click(cx.listener(move |view, _, _, cx| {
                if view.run.is_none()
                    && !view.models.is_busy()
                    && view.models.encoder_is_installed(variant)
                    && view.preferences.variant != variant
                {
                    view.preferences.variant = variant;
                    view.preferences.save();
                    view.invalidate_recognition_models();
                    view.warm_recognition();
                    cx.notify();
                }
            }))
    });

    let set_island = cx.listener(|view, checked: &bool, _, cx| {
        view.preferences.show_island = *checked;
        view.preferences.save();
        cx.notify();
    });

    div()
        .id("settings")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px(px(28.0))
        .py(px(14.0))
        .child(
            div()
                .w_full()
                .max_w(px(520.0))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .pb(px(12.0))
                        .child("設定"),
                )
                .child(
                    row("エンコーダー", theme.border)
                        .child(segment_track(theme).children(encoders)),
                )
                .child(
                    row("音声入力インジケータ", theme.border).child(
                        Switch::new("show-dictation-island", "音声入力インジケータ")
                            .checked(preferences.show_island)
                            .on_change(move |checked, _, window, cx| {
                                set_island(&checked, window, cx)
                            }),
                    ),
                )
                .child(row("マイクへのアクセス", theme.border).child(microphone))
                .child(
                    row("音声入力のショートカット", theme.border).child(
                        div()
                            .text_sm()
                            .children(shortcut)
                            .children(demo.key_monitor.is_some().then(|| "Fn")),
                    ),
                ),
        )
}

fn row(label: &'static str, border: Hsla) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .py(px(12.0))
        .border_t_1()
        .border_color(border)
        .child(div().text_sm().child(label))
}
