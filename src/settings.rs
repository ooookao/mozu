use g2v_ui::{
    theme::colors::G2VTheme,
    ui::{Button, ButtonSize, ButtonVariant, IconName, Input, InputState, Switch},
};
use gpui::{Context, Entity, FontWeight, Hsla, IntoElement, Window, div, prelude::*, px};
use mozu_audio::{Error, Result, models::HaasVariant, vocabulary::VocabularyTerm};
use objc2_app_kit::NSWorkspace;
use objc2_avf_audio::{AVAudioApplication, AVAudioApplicationRecordPermission};
use objc2_foundation::{NSURL, NSUserDefaults, ns_string};

use crate::{
    MozuApp,
    history::{DictionaryEntry, History},
    island::Anchor,
    segment_track,
};

pub(crate) struct DictionaryEditor {
    written: Entity<InputState>,
    reading: Entity<InputState>,
    editing: Option<i64>,
    entries: Vec<DictionaryEntry>,
    error: Option<String>,
}

impl DictionaryEditor {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<MozuApp>) -> Self {
        Self {
            written: cx.new(|cx| InputState::new(window, cx).placeholder("表記")),
            reading: cx.new(|cx| InputState::new(window, cx).placeholder("読み（任意）")),
            editing: None,
            entries: Vec::new(),
            error: None,
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

    fn set_fields(&mut self, id: Option<i64>, window: &mut Window, cx: &mut Context<MozuApp>) {
        self.editing = id;
        self.error = None;
        let entry = self.entries.iter().find(|entry| Some(entry.id) == id);
        let written = entry.map(|entry| entry.written.as_str()).unwrap_or("");
        let reading = entry
            .and_then(|entry| entry.reading.as_deref())
            .unwrap_or("");
        self.written
            .update(cx, |state, cx| state.set_value(written, window, cx));
        self.reading
            .update(cx, |state, cx| state.set_value(reading, window, cx));
        cx.notify();
    }

    fn save(&self, cx: &Context<MozuApp>) -> Result<()> {
        let written = self.written.read(cx).value();
        let reading = self.reading.read(cx).value();
        let written = written.trim();
        let reading = reading.trim();
        if written.is_empty() {
            return Err(Error::Invalid("表記を入力してください。".into()));
        }
        let reading = (!reading.is_empty()).then_some(reading);
        VocabularyTerm::new(written, reading.map(str::to_owned).into_iter().collect())?;
        History::open()?.save_dictionary_entry(self.editing, written, reading)
    }

    fn save_or_report(&mut self, window: &mut Window, cx: &mut Context<MozuApp>) {
        match self.save(cx) {
            Ok(()) => {
                self.set_fields(None, window, cx);
                self.refresh();
            }
            Err(error) => self.error = Some(dictionary_error(error)),
        }
        cx.notify();
    }

    fn remove(&mut self, id: i64, window: &mut Window, cx: &mut Context<MozuApp>) {
        match History::open().and_then(|history| history.delete_dictionary_entry(id)) {
            Ok(()) => {
                if self.editing == Some(id) {
                    self.set_fields(None, window, cx);
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

pub(crate) fn render_dictionary(
    editor: &DictionaryEditor,
    theme: &G2VTheme,
    cx: &Context<MozuApp>,
) -> impl IntoElement {
    let entries = editor.entries.iter().map(|entry| {
        let id = entry.id;
        div()
            .flex()
            .items_center()
            .gap_2()
            .py_2()
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .child(entry.written.clone()),
            )
            .children(entry.reading.clone().map(|reading| {
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(theme.muted_foreground)
                    .child(reading)
            }))
            .child(
                Button::new(("edit-word", id as u64))
                    .icon(IconName::PencilEdit)
                    .size(ButtonSize::IconSmall)
                    .variant(ButtonVariant::Ghost)
                    .accessibility_label(format!("{}を編集", entry.written))
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.dictionary.set_fields(Some(id), window, cx)
                    })),
            )
            .child(
                Button::new(("delete-word", id as u64))
                    .icon(IconName::Trash)
                    .size(ButtonSize::IconSmall)
                    .variant(ButtonVariant::Ghost)
                    .accessibility_label(format!("{}を削除", entry.written))
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.dictionary.remove(id, window, cx)
                    })),
            )
    });
    div()
        .id("dictionary")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .px_6()
        .py_3()
        .child(
            div()
                .max_w(px(520.0))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_lg()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("辞書"),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&editor.written).accessibility_label("表記")),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(Input::new(&editor.reading).accessibility_label("読み")),
                        )
                        .child(
                            Button::new("save-word")
                                .label(if editor.editing.is_some() {
                                    "保存"
                                } else {
                                    "追加"
                                })
                                .on_click(cx.listener(|view, _, window, cx| {
                                    view.dictionary.save_or_report(window, cx)
                                })),
                        ),
                )
                .children(editor.editing.map(|_| {
                    div().flex().justify_end().child(
                        Button::new("cancel-word-edit")
                            .label("キャンセル")
                            .size(ButtonSize::Small)
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.dictionary.set_fields(None, window, cx)
                            })),
                    )
                }))
                .children(
                    editor
                        .error
                        .clone()
                        .map(|error| div().text_sm().text_color(theme.destructive).child(error)),
                )
                .child(div().flex().flex_col().children(entries)),
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
