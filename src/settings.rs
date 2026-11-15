use crate::chatgpt::{self, Model, Vault};
use std::path::Path;

use gpui::{Context, FocusHandle, IntoElement, Window, div, prelude::*, px, rems};
use mozu_audio::{
    Cancellation, Error, HaasEngine, Result,
    models::HaasVariant,
    vocabulary::{Vocabulary, VocabularyConfig},
};
use mozu_ui::{
    appearance::MozuTheme,
    ui::{
        Button, ButtonSize, ButtonVariant, Empty, EmptyMediaVariant, Icon, IconName, Input, Switch,
    },
};
use objc2_app_kit::NSWorkspace;
use objc2_avf_audio::{AVAudioApplication, AVAudioApplicationRecordPermission};
use objc2_foundation::{NSString, NSURL, NSUserDefaults, ns_string};

use crate::{
    LoadedModels, MozuApp,
    history::{DictionaryEntry, History},
    island::Anchor,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DictionaryForm {
    Closed,
    New,
    Edit(i64),
}

pub(crate) struct DictionaryEditor {
    written: String,
    reading: String,
    search: String,
    written_focus: FocusHandle,
    form: DictionaryForm,
    entries: Vec<DictionaryEntry>,
    error: Option<String>,
    saving: bool,
}

impl DictionaryEditor {
    pub(crate) fn new(cx: &mut Context<MozuApp>) -> Self {
        Self {
            written: String::new(),
            reading: String::new(),
            search: String::new(),
            written_focus: cx.focus_handle(),
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
        self.written = written.to_owned();
        self.reading = reading.to_owned();
        window.focus(&self.written_focus, cx);
        cx.notify();
    }

    fn close(&mut self, _window: &mut Window, cx: &mut Context<MozuApp>) {
        self.form = DictionaryForm::Closed;
        self.error = None;
        self.written.clear();
        self.reading.clear();
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
    let written = view.dictionary.written.trim().to_owned();
    let reading = view.dictionary.reading.trim().to_owned();
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
        .icon_start(Icon::new(IconName::PlusMedium, rems(1.0)))
        .size(ButtonSize::IconSm)
        .variant(ButtonVariant::Ghost)
        .accessibility_label("単語を追加")
        .disabled(view.dictionary.saving || view.run.is_some() || view.models.is_busy())
        .on_click(cx.listener(|view, _, window, cx| {
            view.dictionary.search.clear();
            view.dictionary.open(DictionaryForm::New, window, cx);
        }))
}

fn dictionary_inputs(editor: &DictionaryEditor, cx: &Context<MozuApp>) -> (Input, Input) {
    let submit =
        std::rc::Rc::new(cx.listener(|view, _: &gpui::SharedString, window, cx| {
            save_dictionary(view, window, cx)
        }));
    let submit_written = submit.clone();
    let written = Input::new("dictionary-written", editor.written.clone())
        .placeholder("表記")
        .accessibility_label("表記")
        .track_focus(&editor.written_focus)
        .disabled(editor.saving)
        .invalid(editor.error.is_some())
        .on_change(cx.listener(|view, value: &gpui::SharedString, _, cx| {
            view.dictionary.written = value.to_string();
            view.dictionary.error = None;
            cx.notify();
        }))
        .on_submit(move |value, window, cx| submit_written(value, window, cx));
    let reading = Input::new("dictionary-reading", editor.reading.clone())
        .placeholder("読み（任意）")
        .accessibility_label("読み")
        .disabled(editor.saving)
        .invalid(editor.error.is_some())
        .on_change(cx.listener(|view, value: &gpui::SharedString, _, cx| {
            view.dictionary.reading = value.to_string();
            view.dictionary.error = None;
            cx.notify();
        }))
        .on_submit(move |value, window, cx| submit(value, window, cx));
    (written, reading)
}

fn dictionary_form(
    editor: &DictionaryEditor,
    window: &Window,
    cx: &Context<MozuApp>,
) -> impl IntoElement + use<> {
    use mozu_ui::ui::{
        dialog::dialog_footer,
        field::{FieldLayout, field, field_group, field_label},
    };
    let (written, reading) = dictionary_inputs(editor, cx);
    field_group(cx)
        .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
            if event.keystroke.key == "escape" && !view.dictionary.saving {
                view.dictionary.close(window, cx);
                cx.stop_propagation();
            }
        }))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    field(
                        "new-word-written",
                        written,
                        FieldLayout::Vertical,
                        window,
                        cx,
                    )
                    .label(field_label(
                        "表記",
                        FieldLayout::Vertical,
                        window,
                        cx,
                    )),
                )
                .child(
                    field(
                        "new-word-reading",
                        reading,
                        FieldLayout::Vertical,
                        window,
                        cx,
                    )
                    .label(field_label(
                        "読み",
                        FieldLayout::Vertical,
                        window,
                        cx,
                    )),
                ),
        )
        .child(
            dialog_footer()
                .child(
                    Button::new("cancel-word-edit")
                        .label("キャンセル")
                        .size(ButtonSize::Sm)
                        .variant(ButtonVariant::Ghost)
                        .disabled(editor.saving)
                        .on_click(
                            cx.listener(|view, _, window, cx| view.dictionary.close(window, cx)),
                        ),
                )
                .child(
                    Button::new("save-word")
                        .label(if editor.saving { "確認中" } else { "保存" })
                        .size(ButtonSize::Sm)
                        .disabled(editor.saving || editor.written.trim().is_empty())
                        .on_click(
                            cx.listener(|view, _, window, cx| save_dictionary(view, window, cx)),
                        ),
                ),
        )
}

pub(crate) fn render_dictionary(
    view: &MozuApp,
    window: &Window,
    cx: &Context<MozuApp>,
) -> impl IntoElement + use<> {
    use kinu_ui::{TableColumn, TableEmpty, TableRow};
    use mozu_ui::ui::{
        data_table::data_table,
        field::{field_errors, field_group},
        input_group::{InputField, InputGroup},
    };
    let editor = &view.dictionary;
    let query = editor.search.trim().to_lowercase();
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
        })
        .cloned()
        .collect::<Vec<_>>();
    let row_count = entries.len();
    let saving = editor.saving;
    let edit = std::rc::Rc::new(cx.listener(|view, id: &i64, window, cx| {
        view.dictionary.open(DictionaryForm::Edit(*id), window, cx)
    }));
    let remove = std::rc::Rc::new(
        cx.listener(|view, id: &i64, window, cx| view.dictionary.remove(*id, window, cx)),
    );
    let table = data_table(
        "dictionary-table",
        row_count,
        move |index, _, _| {
            let entry = &entries[index];
            let id = entry.id;
            let cells = {
                let edit = edit.clone();
                let remove = remove.clone();
                vec![
                    div()
                        .truncate()
                        .child(entry.written.clone())
                        .into_any_element(),
                    div()
                        .truncate()
                        .child(entry.reading.clone().unwrap_or_else(|| "—".into()))
                        .into_any_element(),
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            Button::new(("edit-word", id as u64))
                                .icon_start(Icon::new(IconName::PencilEdit, rems(1.0)))
                                .size(ButtonSize::IconSm)
                                .variant(ButtonVariant::Ghost)
                                .accessibility_label(format!("{}を編集", entry.written))
                                .disabled(saving)
                                .on_click(move |_, window, cx| edit(&id, window, cx)),
                        )
                        .child(
                            Button::new(("delete-word", id as u64))
                                .icon_start(Icon::new(IconName::Trash, rems(1.0)))
                                .size(ButtonSize::IconSm)
                                .variant(ButtonVariant::Ghost)
                                .accessibility_label(format!("{}を削除", entry.written))
                                .disabled(saving)
                                .on_click(move |_, window, cx| remove(&id, window, cx)),
                        )
                        .into_any_element(),
                ]
            };
            TableRow::new(id as u64, cells)
        },
        window,
        cx,
    )
    .columns([
        TableColumn::new("written", "表記").flex(1.),
        TableColumn::new("reading", "読み").flex(1.),
        TableColumn::new("actions", "操作").width(px(88.)),
    ])
    .accessibility_label("辞書の単語一覧")
    .scroll_fade(mozu_ui::ui::utils::scroll_fade::scroll_fade(
        gpui::Axis::Vertical,
    ))
    .viewport_height((window.viewport_size().height - px(158.)).max(px(100.)))
    .empty(TableEmpty::new(
        Empty::new()
            .title(if query.is_empty() {
                "単語はまだありません"
            } else {
                "一致する単語がありません"
            })
            .media(
                EmptyMediaVariant::Default,
                div().child(Icon::new(IconName::Dictionary, rems(1.5))),
            ),
    ));
    let search = InputGroup::new("dictionary-search-group").w_full().field(
        InputField::new("dictionary-search", "辞書を検索", editor.search.clone())
            .label_hidden(true)
            .icon(mozu_ui::ui::icon::IconName::Search)
            .placeholder("単語を検索")
            .on_change(cx.listener(|view, value: &gpui::SharedString, _, cx| {
                view.dictionary.search = value.to_string();
                cx.notify();
            })),
    );
    let errors = editor
        .error
        .as_ref()
        .map(|error| error.clone().into())
        .into_iter()
        .collect::<Vec<gpui::SharedString>>();
    field_group(cx)
        .id("dictionary")
        .flex_1()
        .min_h_0()
        .px_6()
        .py_3()
        .child(search)
        .children(
            (editor.form == DictionaryForm::Closed)
                .then(|| field_errors("dictionary-errors", &errors, window, cx))
                .flatten(),
        )
        .child(table)
}

pub(crate) fn dictionary_dialog(
    view: &MozuApp,
    window: &mut Window,
    cx: &mut Context<MozuApp>,
) -> impl IntoElement + use<> {
    use mozu_ui::ui::{
        dialog::{DialogStyle, dialog, dialog_header},
        field::field_errors,
    };
    let editor = &view.dictionary;
    let title = match editor.form {
        DictionaryForm::Edit(_) => "単語を編集",
        _ => "単語を登録",
    };
    let errors = editor
        .error
        .iter()
        .cloned()
        .map(gpui::SharedString::from)
        .collect::<Vec<_>>();
    let content = div()
        .flex()
        .flex_col()
        .gap_0()
        .child(dialog_header(title, "", window, cx))
        .child(dictionary_form(editor, window, cx))
        .children(field_errors(
            "dictionary-dialog-errors",
            &errors,
            window,
            cx,
        ));
    let close = cx.listener(|view, open: &bool, window, cx| {
        if !*open && !view.dictionary.saving {
            view.dictionary.close(window, cx);
        }
    });
    dialog(
        "dictionary-dialog",
        editor.form != DictionaryForm::Closed,
        content,
        DialogStyle::current(cx),
        window,
        cx,
    )
    .title(title)
    .on_open_change(move |open, _, window, cx| close(&open, window, cx))
}

#[derive(Clone)]
pub(crate) struct Preferences {
    pub(crate) variant: HaasVariant,
    pub(crate) show_island: bool,
    pub(crate) anchor: Anchor,
    pub(crate) ai_enabled: bool,
    pub(crate) ai_prompt: String,
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
            ai_enabled: defaults.boolForKey(ns_string!("AIFormattingEnabled")),
            ai_prompt: defaults
                .stringForKey(ns_string!("AIFormattingPrompt"))
                .map_or_else(
                    || crate::format::DEFAULT_PROMPT.to_owned(),
                    |value| value.to_string(),
                ),
        }
    }

    pub(crate) fn save(&self) {
        let defaults = NSUserDefaults::standardUserDefaults();
        let frames = match self.variant {
            HaasVariant::Frames1500 => 1500,
            HaasVariant::Frames3072 => 3072,
        };
        defaults.setInteger_forKey(frames, ns_string!("EncoderFrames"));
        defaults.setBool_forKey(self.ai_enabled, ns_string!("AIFormattingEnabled"));
        unsafe {
            defaults.setObject_forKey(
                Some(&NSString::from_str(&self.ai_prompt)),
                ns_string!("AIFormattingPrompt"),
            );
        }
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

fn setting_item(id: &'static str, title: &'static str) -> mozu_ui::ui::item::Item {
    use mozu_ui::ui::{
        item::{Item, ItemSize, ItemVariant},
        utils::presentation::TextRole,
    };
    Item::new(id, title)
        .size(ItemSize::Default)
        .variant(ItemVariant::Muted)
        .title_role(TextRole::Body)
        .description_role(TextRole::Caption)
}

pub(crate) fn render(
    demo: &MozuApp,
    shortcut: Option<Button>,
    theme: &MozuTheme,
    window: &mut Window,
    cx: &mut Context<MozuApp>,
) -> impl IntoElement + use<> {
    let preferences = &demo.preferences;
    let active = demo.run.is_some();
    let microphone = match microphone_permission() {
        AVAudioApplicationRecordPermission::Granted => div().text_sm().child("許可済み"),
        AVAudioApplicationRecordPermission::Undetermined => div().child(
            Button::new("microphone-permission")
                .label("マイクを許可する")
                .size(ButtonSize::Sm)
                .variant(ButtonVariant::Outline)
                .on_click(cx.listener(|_, _, _, cx| request_microphone_permission(cx))),
        ),
        _ => div().child(
            Button::new("microphone-settings")
                .label("システム設定を開く")
                .size(ButtonSize::Sm)
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
    let ai = render_chatgpt(demo, theme, window, cx);
    use mozu_ui::ui::{
        field::field_group,
        toggle_group::{ToggleGroup, ToggleGroupItem},
    };
    let encoders = ToggleGroup::new(
        "encoder-choice",
        kinu_ui::Selection::Single(Some(preferences.variant)),
    )
    .accessibility_label("エンコーダー")
    .disabled(active || demo.models.is_busy())
    .item(
        ToggleGroupItem::new("encoder-1500", HaasVariant::Frames1500)
            .accessibility_label("15 秒")
            .disabled(!demo.models.encoder_is_installed(HaasVariant::Frames1500))
            .child("15 秒"),
    )
    .item(
        ToggleGroupItem::new("encoder-3072", HaasVariant::Frames3072)
            .accessibility_label("30.72 秒")
            .disabled(!demo.models.encoder_is_installed(HaasVariant::Frames3072))
            .child("30.72 秒"),
    )
    .on_change(cx.listener(|view, selection, _, cx| {
        if let kinu_ui::Selection::Single(Some(variant)) = selection
            && view.preferences.variant != *variant
        {
            view.preferences.variant = *variant;
            view.preferences.save();
            view.invalidate_recognition_models();
            view.warm_recognition();
            cx.notify();
        }
    }));
    let island = setting_item("island-preference", "音声入力インジケータ")
        .media(
            mozu_ui::ui::item::ItemMediaVariant::Icon,
            mozu_ui::ui::icon::Icon::new(mozu_ui::ui::icon::IconName::Indicator),
        )
        .actions(
            Switch::new("show-dictation-island", preferences.show_island)
                .accessibility_label("音声入力インジケータ")
                .on_change(cx.listener(|view, checked: &bool, _, cx| {
                    view.preferences.show_island = *checked;
                    view.preferences.save();
                    cx.notify();
                })),
        );
    let content = field_group(cx)
        .max_w(px(520.))
        .mx_auto()
        .child(
            setting_item("encoder-setting", "エンコーダー")
                .media(
                    mozu_ui::ui::item::ItemMediaVariant::Icon,
                    mozu_ui::ui::icon::Icon::new(mozu_ui::ui::icon::IconName::Encoder),
                )
                .actions(encoders),
        )
        .child(island)
        .child(ai)
        .child(
            setting_item("microphone-setting", "マイクへのアクセス")
                .media(
                    mozu_ui::ui::item::ItemMediaVariant::Icon,
                    mozu_ui::ui::icon::Icon::new(mozu_ui::ui::icon::IconName::Microphone),
                )
                .actions(microphone),
        )
        .child(
            setting_item("shortcut-setting", "音声入力のショートカット")
                .media(
                    mozu_ui::ui::item::ItemMediaVariant::Icon,
                    mozu_ui::ui::icon::Icon::new(mozu_ui::ui::icon::IconName::Shortcut),
                )
                .actions(
                    div()
                        .flex()
                        .items_center()
                        .children(shortcut)
                        .children(demo.key_monitor.is_some().then_some("Fn")),
                ),
        );
    let viewport = kinu_ui::ScrollAreaViewport::new()
        .px_6()
        .py_3()
        .child(content);
    mozu_ui::ui::scroll_area::scroll_area(
        "settings-scroll",
        mozu_ui::ui::scroll_area::scroll_fade(viewport, gpui::Axis::Vertical),
        kinu_ui::ScrollAreaAxes::Vertical,
        window,
        cx,
    )
    .flex_1()
    .min_h_0()
}

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub(crate) struct ChatGptUi {
    pub(crate) vault: Option<Vault>,
    pub(crate) models: Vec<Model>,
    pub(crate) model: String,
    pub(crate) busy: bool,
    pub(crate) error: Option<String>,
    cancellation: Arc<AtomicBool>,
    model_open: bool,
    account_open: bool,
    welcome: bool,
}

impl ChatGptUi {
    pub(crate) fn new() -> Self {
        Self {
            vault: None,
            models: Vec::new(),
            model: String::new(),
            busy: true,
            error: None,
            cancellation: Arc::new(AtomicBool::new(false)),
            model_open: false,
            account_open: false,
            welcome: false,
        }
    }
    pub(crate) fn connected(&self) -> bool {
        self.vault.as_ref().is_some_and(chatgpt::is_connected)
    }
    pub(crate) fn ready(&self) -> bool {
        self.connected() && !self.busy && self.models.iter().any(|m| m.slug == self.model)
    }
    pub(crate) fn cancel(&self) {
        self.cancellation.store(true, Ordering::SeqCst);
    }
    fn load_model(&mut self) {
        self.model = self
            .vault
            .as_ref()
            .and_then(|v| v.active.as_ref())
            .and_then(|id| {
                NSUserDefaults::standardUserDefaults().stringForKey(
                    &objc2_foundation::NSString::from_str(&format!("ChatGPTModel.{id}")),
                )
            })
            .map(|s| s.to_string())
            .unwrap_or_default();
    }
    fn save_model(&self) {
        if let Some(id) = self.vault.as_ref().and_then(|v| v.active.as_ref()) {
            unsafe {
                NSUserDefaults::standardUserDefaults().setObject_forKey(
                    Some(&objc2_foundation::NSString::from_str(&self.model)),
                    &objc2_foundation::NSString::from_str(&format!("ChatGPTModel.{id}")),
                );
            }
        }
    }
}

enum ChatGptOutcome {
    Loaded(Vault),
    SignedIn(Vault),
    Models(Vault, Vec<Model>),
    SignedOut(Vault, Option<String>),
}
enum ChatGptEvent {
    Browser(String),
    Done(chatgpt::Result<ChatGptOutcome>),
}

impl MozuApp {
    fn chatgpt_task(
        &mut self,
        task: impl FnOnce(
            Arc<AtomicBool>,
            async_channel::Sender<ChatGptEvent>,
        ) -> chatgpt::Result<ChatGptOutcome>
        + Send
        + 'static,
        cx: &mut Context<Self>,
    ) {
        self.chatgpt.busy = true;
        self.chatgpt.error = None;
        self.chatgpt.cancellation = Arc::new(AtomicBool::new(false));
        let cancel = self.chatgpt.cancellation.clone();
        let (tx, rx) = async_channel::unbounded();
        let activity = crate::updater::protect(cx);
        std::thread::spawn(move || {
            let outcome = task(cancel, tx.clone());
            let _ = tx.send_blocking(ChatGptEvent::Done(outcome));
            drop(activity);
        });
        cx.spawn(async move |view, cx| {
            while let Ok(event) = rx.recv().await {
                let done = matches!(event, ChatGptEvent::Done(_));
                let _ = view.update(cx, |view, cx| {
                    match event {
                        ChatGptEvent::Browser(url) => {
                            if !view.chatgpt.cancellation.load(Ordering::SeqCst)
                                && let Err(error) = open_url(&url)
                            {
                                view.chatgpt.cancel();
                                view.chatgpt.error = Some(error);
                            }
                        }
                        ChatGptEvent::Done(result) => {
                            view.chatgpt.busy = false;
                            match result {
                                Ok(ChatGptOutcome::Loaded(vault)) => {
                                    view.chatgpt.vault = Some(vault);
                                    view.chatgpt.load_model();
                                    if view.chatgpt.connected() {
                                        view.refresh_chatgpt_models(cx);
                                    }
                                }
                                Ok(ChatGptOutcome::SignedIn(vault)) => {
                                    view.chatgpt.vault = Some(vault);
                                    view.chatgpt.models.clear();
                                    view.chatgpt.load_model();
                                    if !NSUserDefaults::standardUserDefaults()
                                        .boolForKey(ns_string!("ChatGPTWelcomeSeen"))
                                    {
                                        view.chatgpt.welcome = true;
                                    }
                                    view.refresh_chatgpt_models(cx);
                                }
                                Ok(ChatGptOutcome::Models(vault, models)) => {
                                    view.chatgpt.vault = Some(vault);
                                    view.chatgpt.models = models;
                                    if !view
                                        .chatgpt
                                        .models
                                        .iter()
                                        .any(|m| m.slug == view.chatgpt.model)
                                    {
                                        view.chatgpt.model.clear();
                                    }
                                }
                                Ok(ChatGptOutcome::SignedOut(vault, warning)) => {
                                    view.chatgpt.vault = Some(vault);
                                    view.chatgpt.models.clear();
                                    view.chatgpt.error = warning;
                                }
                                Err(error) => {
                                    if !view.chatgpt.cancellation.load(Ordering::SeqCst)
                                        || view.chatgpt.error.is_none()
                                    {
                                        view.chatgpt.error = Some(error);
                                    }
                                }
                            }
                        }
                    }
                    cx.notify();
                });
                if done {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }
    pub(crate) fn load_chatgpt(&mut self, cx: &mut Context<Self>) {
        self.chatgpt_task(|_, _| Vault::load().map(ChatGptOutcome::Loaded), cx);
    }
    fn sign_in_chatgpt(&mut self, returning: Option<String>, cx: &mut Context<Self>) {
        if self.chatgpt.busy || self.run.is_some() {
            return;
        }
        let Some(_vault) = self.chatgpt.vault.clone() else {
            self.load_chatgpt(cx);
            return;
        };
        self.chatgpt_task(
            move |cancel, tx| {
                chatgpt::sign_in(&Vault::load()?, returning.as_deref(), &cancel, |url| {
                    tx.send_blocking(ChatGptEvent::Browser(url))
                        .map_err(|_| "Mozuの認証画面が閉じられました".into())
                })
                .map(ChatGptOutcome::SignedIn)
            },
            cx,
        );
    }
    fn refresh_chatgpt_models(&mut self, cx: &mut Context<Self>) {
        if self.chatgpt.busy || self.run.is_some() {
            return;
        }
        let Some(_vault) = self.chatgpt.vault.clone() else {
            return;
        };
        self.chatgpt_task(
            move |_, _| {
                chatgpt::list_models(&Vault::load()?).map(|(v, m)| ChatGptOutcome::Models(v, m))
            },
            cx,
        );
    }
    fn sign_out_chatgpt(&mut self, cx: &mut Context<Self>) {
        if self.chatgpt.busy || self.run.is_some() {
            return;
        }
        let Some(_vault) = self.chatgpt.vault.clone() else {
            return;
        };
        self.chatgpt_task(
            move |_, _| {
                chatgpt::sign_out(&Vault::load()?)
                    .map(|(v, warning)| ChatGptOutcome::SignedOut(v, warning))
            },
            cx,
        );
    }
}

fn open_url(value: &str) -> std::result::Result<(), String> {
    let url = NSURL::URLWithString(&objc2_foundation::NSString::from_str(value))
        .ok_or("リンクのURLが不正です")?;
    if NSWorkspace::sharedWorkspace().openURL(&url) {
        Ok(())
    } else {
        Err("ブラウザを開けませんでした".into())
    }
}

fn render_chatgpt(
    view: &MozuApp,
    _theme: &MozuTheme,
    window: &mut Window,
    cx: &mut Context<MozuApp>,
) -> impl IntoElement + use<> {
    use mozu_ui::ui::dropdown_menu::{DropdownStyle, dropdown_menu_with_trigger};
    let state = &view.chatgpt;
    let locked = state.busy || view.run.is_some();
    let connected = state.connected();
    let status = if state.busy {
        "接続を確認中"
    } else if connected {
        "連携済み"
    } else {
        "未連携"
    };
    let account = state.vault.as_ref().and_then(|v| v.selected());
    let account_name = account
        .map(|a| a.email.clone())
        .unwrap_or_else(|| "ChatGPT".into());
    let returning = account.map(|a| a.client_id.clone());
    let login = Button::new("chatgpt-login")
        .label(if connected {
            "再ログイン"
        } else {
            "Continue with ChatGPT"
        })
        .icon_start(mozu_ui::icons::BrandIcon::new(
            mozu_ui::icons::BrandIconName::OpenAi,
            rems(1.0),
        ))
        .accessibility_label("ChatGPTでログイン")
        .size(ButtonSize::Sm)
        .variant(ButtonVariant::Outline)
        .disabled(locked)
        .on_click(cx.listener(move |view, _, _, cx| view.sign_in_chatgpt(returning.clone(), cx)));
    let model_label = state
        .models
        .iter()
        .find(|m| m.slug == state.model)
        .map(|m| m.display_name.as_str())
        .unwrap_or("モデルを選択");
    let trigger = Button::new("chatgpt-model-trigger")
        .label(model_label.to_owned())
        .icon_end(Icon::new(IconName::ChevronDown, rems(1.0)))
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .disabled(locked || !connected || state.models.is_empty())
        .into_primitive(window, cx);
    let mut model_menu = dropdown_menu_with_trigger::<String>(
        "chatgpt-model-menu",
        state.model_open,
        trigger,
        DropdownStyle::current(cx),
        window,
        cx,
    )
    .on_open_change({
        let set = cx.listener(|view, open: &bool, _, cx| {
            view.chatgpt.model_open = *open;
            cx.notify();
        });
        move |open, _, w, cx| set(&open, w, cx)
    })
    .on_invoke(cx.listener(|view, slug: &String, _, cx| {
        view.chatgpt.model = slug.clone();
        view.chatgpt.model_open = false;
        view.chatgpt.save_model();
        cx.notify();
    }));
    for model in &state.models {
        model_menu = model_menu.item(kinu_ui::MenuItem::new(
            model.slug.clone(),
            model.slug.clone(),
            model.display_name.clone(),
        ));
    }
    let account_trigger = Button::new("chatgpt-account-trigger")
        .label(account_name.clone())
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Sm)
        .disabled(locked || state.vault.is_none())
        .into_primitive(window, cx);
    let mut account_menu = dropdown_menu_with_trigger::<String>(
        "chatgpt-account-menu",
        state.account_open,
        account_trigger,
        DropdownStyle::current(cx),
        window,
        cx,
    )
    .on_open_change({
        let set = cx.listener(|view, open: &bool, _, cx| {
            view.chatgpt.account_open = *open;
            cx.notify();
        });
        move |open, _, w, cx| set(&open, w, cx)
    })
    .on_invoke(cx.listener(|view, id: &String, _, cx| {
        view.chatgpt.account_open = false;
        if id == "add-account" {
            view.sign_in_chatgpt(None, cx);
        } else if view.chatgpt.vault.is_some() {
            let id = id.clone();
            view.chatgpt_task(
                move |_, _| {
                    let mut next = Vault::load()?;
                    if !next.accounts.iter().any(|a| a.client_id == id) {
                        return Err("ChatGPTアカウントが見つかりません".into());
                    }
                    next.active = Some(id);
                    next.save()?;
                    Ok(ChatGptOutcome::Loaded(next))
                },
                cx,
            );
        }
        cx.notify();
    }));
    if let Some(vault) = &state.vault {
        for (index, account) in vault.accounts.iter().enumerate() {
            account_menu = account_menu.item(kinu_ui::MenuItem::new(
                account.client_id.clone(),
                account.client_id.clone(),
                format!("{} · {}", account.email, index + 1),
            ));
        }
    }
    account_menu = account_menu.item(kinu_ui::MenuItem::new(
        "add-account",
        "add-account".to_owned(),
        "別のアカウントでログイン",
    ));
    use mozu_ui::ui::{
        field::field_errors,
        item::{ItemGroup, ItemVariant},
    };
    let issues = state
        .error
        .as_ref()
        .map(|error| error.clone().into())
        .into_iter()
        .collect::<Vec<gpui::SharedString>>();
    let errors = field_errors("chatgpt-errors", &issues, window, cx);
    let connection_actions = div()
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .when(!state.busy, |actions| actions.child(login))
        .when(state.busy, |actions| {
            actions.child(
                Button::new("chatgpt-cancel-login")
                    .label("キャンセル")
                    .size(ButtonSize::Compact)
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.chatgpt.cancel();
                        cx.notify();
                    })),
            )
        })
        .when(connected && !state.busy, |actions| {
            actions.child(
                Button::new("chatgpt-sign-out")
                    .label("ログアウト")
                    .size(ButtonSize::Compact)
                    .variant(ButtonVariant::Ghost)
                    .disabled(locked)
                    .on_click(cx.listener(|view, _, _, cx| view.sign_out_chatgpt(cx))),
            )
        });
    let connection = setting_item("chatgpt-connection", "ChatGPT連携")
        .variant(ItemVariant::Outline)
        .description(status)
        .actions(connection_actions)
        .when(connected, |item| item.footer(account_menu));
    let model = setting_item("chatgpt-model-setting", "モデル")
        .variant(ItemVariant::Outline)
        .actions(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .child(model_menu)
                .child(
                    Button::new("chatgpt-refresh-models")
                        .label("更新")
                        .accessibility_label("モデルを更新")
                        .size(ButtonSize::Compact)
                        .variant(ButtonVariant::Ghost)
                        .disabled(locked || !connected)
                        .on_click(cx.listener(|view, _, _, cx| view.refresh_chatgpt_models(cx))),
                ),
        );
    let prompt = setting_item("ai-formatting-prompt-setting", "プロンプト")
        .variant(ItemVariant::Outline)
        .footer(
            mozu_ui::ui::textarea::Textarea::new(
                "ai-formatting-prompt",
                view.preferences.ai_prompt.clone(),
            )
            .accessibility_label("AI整形のプロンプト")
            .w_full()
            .disabled(view.run.is_some())
            .invalid(view.preferences.ai_prompt.trim().is_empty())
            .on_change(cx.listener(|view, text: &gpui::SharedString, _, cx| {
                view.preferences.ai_prompt = text.to_string();
                view.preferences.save();
                cx.notify();
            })),
        );
    let usage = setting_item("chatgpt-usage", "利用状況")
        .variant(ItemVariant::Outline)
        .actions(
            Button::new("chatgpt-manage-usage")
                .label("管理画面を開く")
                .accessibility_label("ChatGPTの利用状況を管理")
                .size(ButtonSize::Compact)
                .variant(ButtonVariant::Outline)
                .on_click(cx.listener(|view, _, _, cx| {
                    if let Err(error) = open_url("https://chatgpt.com/settings/usage") {
                        view.chatgpt.error = Some(error);
                        cx.notify();
                    }
                })),
        );
    setting_item("ai-formatting-setting", "AI整形")
        .media(
            mozu_ui::ui::item::ItemMediaVariant::Icon,
            mozu_ui::ui::icon::Icon::new(mozu_ui::ui::icon::IconName::OpenAI),
        )
        .description("OpenAIへ文字起こしを送り、ChatGPTプランで整形します。")
        .actions(
            Switch::new("ai-formatting-enabled", view.preferences.ai_enabled)
                .accessibility_label("AI整形")
                .disabled(view.run.is_some())
                .on_change(cx.listener(|view, checked: &bool, _, cx| {
                    view.preferences.ai_enabled = *checked;
                    view.preferences.save();
                    cx.notify();
                })),
        )
        .footer(
            div()
                .w_full()
                .child(
                    ItemGroup::new("chatgpt-settings")
                        .item(connection)
                        .item(model)
                        .item(prompt)
                        .item(usage),
                )
                .children(errors),
        )
}

pub(crate) fn chatgpt_welcome(
    view: &MozuApp,
    window: &Window,
    cx: &Context<MozuApp>,
) -> impl IntoElement + use<> {
    use mozu_ui::ui::dialog::{dialog_footer, dialog_header, dialog_shell};
    let close = cx.listener(|view, _, _, cx| {
        view.chatgpt.welcome = false;
        NSUserDefaults::standardUserDefaults()
            .setBool_forKey(true, ns_string!("ChatGPTWelcomeSeen"));
        cx.notify();
    });
    dialog_shell("chatgpt-welcome", view.chatgpt.welcome, window, cx)
        .content(div().flex().flex_col().p_6().rounded_lg()
            .bg(mozu_ui::theme::Theme::current(window, cx).popover)
            .child(dialog_header("ChatGPTプランを利用します", "AI整形はChatGPTプランの利用枠を使用します。利用状況と上限はChatGPTの設定で管理できます。", window, cx))
            .child(dialog_footer().child(Button::new("chatgpt-welcome-dismiss").label("わかりました").on_click(close))))
        .on_open_change({ let close = cx.listener(|view, _, _, cx| {
            view.chatgpt.welcome = false;
            NSUserDefaults::standardUserDefaults().setBool_forKey(true, ns_string!("ChatGPTWelcomeSeen"));
            cx.notify();
        }); move |_, _, window, cx| close(&(), window, cx) })
}
