mod dictation;
mod history;
mod island;
mod material;
mod meeting;
mod menu;
mod models;
mod settings;

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender, SyncSender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

use g2v_ui::{
    theme::{self, colors::G2VTheme, typography},
    ui::{
        Assets, Button, ButtonSize, ButtonVariant, DropdownMenu, DropdownMenuRadioItem, Empty,
        EmptyMediaVariant, Icon, IconName, PopoverAlign, SidebarMenuButton,
    },
};
use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, FontWeight, InteractiveElement as _,
    IntoElement as _, ParentElement as _, PathPromptOptions, Render, ScrollHandle,
    StatefulInteractiveElement as _, Styled, TitlebarOptions, Window, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, div, point, prelude::FluentBuilder as _, px, rems, size,
};
use mozu_audio::{
    Cancellation, ComputeConfig, Error, HaasEngine, Result,
    audio::{AudioSource, WavSource},
    batch::{self, BatchProgress},
    models::{HaasModelFiles, HaasVariant, ModelCache},
    recording::{RateConverter, Recording, SAMPLE_RATE},
    streaming::StreamingSession,
    vad::SileroVad,
    vocabulary::{Vocabulary, VocabularyConfig},
    window::AsrConfig,
};

use objc2_avf_audio::AVAudioApplicationRecordPermission;

use dictation::{Key, KeyMonitor, PasteTarget};
use history::{History, HistoryKind, HistoryView};
use material::WindowMaterial;
use meeting::MeetingRecording;
use settings::{DictionaryEditor, Preferences};

const TITLEBAR_HEIGHT: f32 = 54.0;
const HOLD_THRESHOLD: Duration = Duration::from_millis(320);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Dictation,
    Transcription,
}

impl Mode {
    fn title(self) -> &'static str {
        match self {
            Self::Dictation => "音声入力",
            Self::Transcription => "文字起こし",
        }
    }

    fn history_kind(self) -> HistoryKind {
        match self {
            Self::Dictation => HistoryKind::Dictation,
            Self::Transcription => HistoryKind::Transcription,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Recognition(Mode),
    Dictionary,
    Models,
    Settings,
}

struct MozuApp {
    mode: Mode,
    screen: Screen,
    other_screen: Screen,
    material: WindowMaterial,
    preferences: Preferences,
    page: Page,
    dictionary: DictionaryEditor,
    models: models::ModelManager,
    recording_source: RecordingSource,
    recognition_jobs: Sender<RecognitionJob>,
    warmup: Cancellation,
    run: Option<Run>,
    key_monitor: Option<KeyMonitor>,
    key_press: Option<KeyPress>,
}

#[derive(Default)]
struct Screen {
    history: HistoryView,
    transcript: Transcript,
    scroll: ScrollHandle,
    phase: Phase,
    seconds: f64,
    spectrum: island::Spectrum,
    copied: bool,
    file_name: Option<String>,
}

impl Screen {
    fn clear_result(&mut self) {
        self.transcript = Transcript::default();
        self.file_name = None;
        self.seconds = 0.0;
        self.spectrum = island::Spectrum::default();
        self.copied = false;
        self.phase = Phase::Idle;
    }
}

struct Run {
    input: Input,
    stop: Sender<()>,
    cancellation: Cancellation,
    target: Option<PasteTarget>,
    discard: bool,
    model_ready: bool,
}

struct KeyPress {
    started: Instant,
    interrupted: bool,
    recording: bool,
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Preparing,
    Listening,
    Transcribing(BatchProgress),
    Stopping,
    Finished,
    Failed(String),
}

impl MozuApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut screen = Screen::default();
        screen.history.refresh(HistoryKind::Dictation);
        let (recognition_jobs, jobs) = mpsc::channel::<RecognitionJob>();
        thread::spawn(move || {
            let mut loaded = None;
            for job in jobs {
                job(&mut loaded);
            }
        });
        Self {
            mode: Mode::Dictation,
            screen,
            other_screen: Screen::default(),
            material: WindowMaterial::install(window),
            preferences: Preferences::load(),
            page: Page::Recognition(Mode::Dictation),
            dictionary: DictionaryEditor::new(window, cx),
            models: models::ModelManager::load(),
            recording_source: RecordingSource::Microphone,
            recognition_jobs,
            warmup: Cancellation::new(),
            run: None,
            key_monitor: None,
            key_press: None,
        }
    }
}

impl Drop for MozuApp {
    fn drop(&mut self) {
        if let Some(run) = &self.run {
            run.cancellation.cancel();
        }
        self.models.cancel();
        self.warmup.cancel();
    }
}

#[derive(Default)]
struct Transcript {
    text: String,
    provisional: String,
}

impl Transcript {
    fn update(&mut self, confirmed: String, provisional: String) {
        self.text = confirmed;
        self.provisional = provisional;
    }

    fn finish(&mut self, text: String) {
        self.text = text;
        self.provisional.clear();
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RecordingSource {
    Microphone,
    Meeting,
}

impl RecordingSource {
    fn label(self) -> &'static str {
        match self {
            Self::Microphone => "マイク",
            Self::Meeting => "マイク＋システム音声",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum Input {
    Sample(PathBuf),
    Recording(RecordingSource),
    Dictation,
    File(PathBuf),
}

impl Input {
    fn recording_source(&self) -> Option<RecordingSource> {
        match self {
            Self::Recording(source) => Some(*source),
            Self::Dictation => Some(RecordingSource::Microphone),
            Self::Sample(_) | Self::File(_) => None,
        }
    }
}

enum Update {
    Listening,
    ModelReady,
    Position {
        seconds: f64,
        spectrum: island::Spectrum,
    },
    Transcript {
        confirmed: String,
        provisional: String,
    },
    FileProgress {
        progress: BatchProgress,
        text: String,
    },
    Finished {
        text: String,
        history_id: Result<Option<i64>>,
    },
    Cancelled,
    Failed(String),
}

impl MozuApp {
    fn select_mode(&mut self, mode: Mode) {
        if self.run.is_none() && self.mode != mode {
            std::mem::swap(&mut self.screen, &mut self.other_screen);
            self.mode = mode;
        }
    }

    fn screen_for(&self, mode: Mode) -> &Screen {
        if self.mode == mode {
            &self.screen
        } else {
            &self.other_screen
        }
    }

    fn screen_for_mut(&mut self, mode: Mode) -> &mut Screen {
        if self.mode == mode {
            &mut self.screen
        } else {
            &mut self.other_screen
        }
    }

    fn navigate(&mut self, page: Page, cx: &mut Context<Self>) {
        match page {
            Page::Recognition(mode) => {
                self.screen_for_mut(mode)
                    .history
                    .refresh(mode.history_kind());
            }
            Page::Dictionary => self.dictionary.refresh(),
            Page::Models => {}
            Page::Settings => {}
        }
        self.page = page;
        cx.notify();
    }

    fn open_file(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.run.is_some() || self.models.is_busy() {
            return;
        }
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("音声ファイルを開く".into()),
        });
        cx.spawn_in(window, async move |view, cx| {
            let result = receiver.await;
            let _ = view.update_in(cx, |view, window, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        view.start(Input::File(path), None, window, cx);
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(error)) => {
                    view.screen_for_mut(Mode::Transcription).phase =
                        Phase::Failed(format!("ファイルを開けませんでした。{error}"));
                    cx.notify();
                }
                Err(_) => {}
            });
        })
        .detach();
    }

    fn invalidate_recognition_models(&mut self) {
        self.warmup.cancel();
        let _ = self
            .recognition_jobs
            .send(Box::new(|loaded| *loaded = None));
    }

    fn warm_recognition(&mut self) {
        self.warmup.cancel();
        self.warmup = Cancellation::new();
        let Some(root) = self.models.root.clone() else {
            return;
        };
        let variant = self.preferences.variant;
        let cancellation = self.warmup.clone();
        let _ = self.recognition_jobs.send(Box::new(move |loaded| {
            if !models::cached_for_streaming(&root, variant) {
                return;
            }
            let result = LoadedModels::get(loaded, root, variant, &cancellation)
                .and_then(|models| models.vad(&cancellation).map(|_| ()));
            if let Err(error) = result
                && !matches!(error, Error::Cancelled)
            {
                eprintln!("Could not prepare cached recognition models: {error}");
            }
        }));
    }

    fn start(
        &mut self,
        input: Input,
        target: Option<PasteTarget>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if self.run.is_some() || self.models.is_busy() {
            return;
        }
        match &input {
            Input::Dictation => self.select_mode(Mode::Dictation),
            Input::File(_) | Input::Sample(_) | Input::Recording(_) => {
                self.select_mode(Mode::Transcription)
            }
        }
        if input != Input::Dictation {
            self.key_press = None;
            self.screen.history.close_detail();
            if matches!(self.page, Page::Recognition(_)) {
                self.page = Page::Recognition(Mode::Transcription);
            }
        }
        self.screen.clear_result();
        self.screen.file_name = match &input {
            Input::File(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned()),
            _ => None,
        };
        self.screen.scroll.set_offset(point(px(0.0), px(0.0)));
        if !self.models.encoder_is_installed(self.preferences.variant) {
            self.screen.phase = Phase::Failed(
                "「モデル」でエンコーダーをダウンロードし、「設定」で選択してください".into(),
            );
            cx.notify();
            return;
        }
        if input.recording_source().is_some() {
            let permission = settings::microphone_permission();
            if permission != AVAudioApplicationRecordPermission::Granted {
                self.screen.phase =
                    Phase::Failed("設定でマイクへのアクセスを許可してください".into());
                if permission == AVAudioApplicationRecordPermission::Undetermined {
                    settings::request_microphone_permission(cx);
                }
                cx.notify();
                return;
            }
        }
        self.screen.phase = Phase::Preparing;
        let (stop, requests) = mpsc::channel();
        let (updates, receiver) = mpsc::channel();
        let cancellation = Cancellation::new();
        self.run = Some(Run {
            input: input.clone(),
            stop,
            cancellation: cancellation.clone(),
            target,
            discard: false,
            model_ready: false,
        });
        let variant = self.preferences.variant;
        let cache_path = self.models.root.clone();
        let jobs = self.recognition_jobs.clone();
        thread::spawn(move || {
            let feed = match open_feed(&input, requests, &updates, &cancellation) {
                Ok(feed) => feed,
                Err(error) => {
                    report_recognition_failure(error, &updates);
                    return;
                }
            };
            let failure_updates = updates.clone();
            if jobs
                .send(Box::new(move |loaded| {
                    let result = recognize(
                        cache_path,
                        input,
                        variant,
                        feed,
                        &updates,
                        &cancellation,
                        loaded,
                    );
                    if let Err(error) = result {
                        report_recognition_failure(error, &updates);
                    }
                }))
                .is_err()
            {
                let _ =
                    failure_updates.send(Update::Failed("認識処理を開始できませんでした".into()));
            }
        });
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(30))
                    .await;
                let running = view.update_in(cx, |view, window, cx| {
                    let mut changed = false;
                    loop {
                        match receiver.try_recv() {
                            Ok(Update::Listening) => {
                                if matches!(view.screen.phase, Phase::Preparing) {
                                    view.screen.phase = Phase::Listening;
                                }
                            }
                            Ok(Update::ModelReady) => {
                                if let Some(run) = &mut view.run {
                                    run.model_ready = true;
                                }
                            }
                            Ok(Update::Position { seconds, spectrum }) => {
                                view.screen.seconds = seconds;
                                view.screen.spectrum = spectrum;
                            }
                            Ok(Update::Transcript {
                                confirmed,
                                provisional,
                            }) => {
                                if view.mode == Mode::Transcription {
                                    if view.screen.transcript.text != confirmed {
                                        view.follow_transcript();
                                        view.screen.copied = false;
                                    }
                                    view.screen.transcript.update(confirmed, provisional);
                                }
                            }
                            Ok(Update::FileProgress { progress, text }) => {
                                if !view.run.as_ref().is_some_and(|run| run.discard) {
                                    view.follow_transcript();
                                    view.screen.transcript.finish(text);
                                    view.screen.seconds =
                                        progress.processed_samples as f64 / SAMPLE_RATE as f64;
                                    view.screen.phase = Phase::Transcribing(progress);
                                }
                            }
                            Ok(Update::Finished { text, history_id }) => {
                                if view.run.as_ref().is_some_and(|run| run.discard) {
                                    if let Ok(Some(id)) = history_id {
                                        thread::spawn(move || {
                                            if let Err(error) = History::open()
                                                .and_then(|history| history.discard(id))
                                            {
                                                eprintln!("Could not discard dictation: {error}");
                                            }
                                        });
                                    }
                                    view.clear_cancelled();
                                } else {
                                    view.follow_transcript();
                                    view.screen.transcript.finish(text);
                                    view.screen.copied = false;
                                    view.finish(history_id, cx);
                                }
                            }
                            Ok(Update::Cancelled) => view.clear_cancelled(),
                            Ok(Update::Failed(_))
                                if view.run.as_ref().is_some_and(|run| run.discard) =>
                            {
                                view.clear_cancelled();
                            }
                            Ok(Update::Failed(message)) => view.end(Phase::Failed(message)),
                            Err(TryRecvError::Empty) => break,
                            Err(TryRecvError::Disconnected) => {
                                if view.run.is_some() {
                                    view.end(Phase::Failed("認識処理が中断されました".into()));
                                    changed = true;
                                }
                                break;
                            }
                        }
                        changed = true;
                    }
                    if changed {
                        cx.notify();
                        window.refresh();
                    }
                    view.run.is_some()
                });
                if !matches!(running, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
        cx.notify();
    }

    fn follow_transcript(&self) {
        if self.screen.scroll.max_offset().y + self.screen.scroll.offset().y <= px(24.0) {
            self.screen.scroll.scroll_to_bottom();
        }
    }

    fn finish(&mut self, history_id: Result<Option<i64>>, cx: &mut Context<Self>) {
        let mut phase = Phase::Finished;
        match history_id {
            Err(error) => phase = Phase::Failed(error.to_string()),
            Ok(Some(id)) => {
                if let Some(run) = self
                    .run
                    .as_mut()
                    .filter(|run| run.input == Input::Dictation)
                {
                    let result = run
                        .target
                        .take()
                        .ok_or("元の入力欄を確認できませんでした")
                        .and_then(|target| target.paste(&self.screen.transcript.text));
                    let sent = result.is_ok();
                    match result {
                        Ok(clipboard) => cx
                            .spawn(async move |_, cx| {
                                cx.background_executor()
                                    .timer(Duration::from_millis(350))
                                    .await;
                                clipboard.restore();
                            })
                            .detach(),
                        Err(message) => phase = Phase::Failed(message.into()),
                    }
                    thread::spawn(move || {
                        if let Err(error) =
                            History::open().and_then(|history| history.record_paste(id, sent))
                        {
                            eprintln!("Could not save paste status: {error}");
                        }
                    });
                }
            }
            Ok(None) => {}
        }
        self.end(phase);
        self.screen.history.refresh(self.mode.history_kind());
    }

    fn end(&mut self, phase: Phase) {
        self.run = None;
        self.screen.phase = phase;
        self.screen.spectrum = island::Spectrum::default();
    }

    fn clear_cancelled(&mut self) {
        self.run = None;
        self.screen.clear_result();
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        if matches!(
            self.screen.phase,
            Phase::Preparing | Phase::Transcribing(_) | Phase::Stopping
        ) {
            self.cancel(cx);
        } else {
            self.request_finish(cx);
        }
    }

    fn request_finish(&mut self, cx: &mut Context<Self>) {
        let Some(run) = &self.run else { return };
        if !matches!(
            self.screen.phase,
            Phase::Preparing | Phase::Listening | Phase::Transcribing(_)
        ) {
            return;
        }
        let _ = run.stop.send(());
        self.screen.phase = Phase::Stopping;
        self.screen.spectrum = island::Spectrum::default();
        cx.notify();
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        if let Some(run) = &mut self.run {
            run.discard = true;
            run.cancellation.cancel();
            self.request_finish(cx);
        }
    }

    fn install_key_monitor(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !dictation::is_trusted() {
            self.key_monitor = None;
            self.key_press = None;
            return;
        }
        if self.key_monitor.is_some() {
            return;
        }
        let (sender, receiver) = async_channel::unbounded();
        self.key_monitor = KeyMonitor::install(sender);
        if self.key_monitor.is_none() {
            return;
        }
        cx.spawn_in(window, async move |view, cx| {
            while let Ok(key) = receiver.recv().await {
                if view
                    .update_in(cx, |view, window, cx| view.handle_key(key, window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn handle_key(&mut self, key: Key, window: &Window, cx: &mut Context<Self>) {
        if self.models.is_busy()
            || self
                .run
                .as_ref()
                .is_some_and(|run| run.input != Input::Dictation)
        {
            self.key_press = None;
            return;
        }
        match key {
            Key::Down if self.key_press.is_none() => {
                let started = Instant::now();
                self.key_press = Some(KeyPress {
                    started,
                    interrupted: false,
                    recording: false,
                });
                if self.run.is_none() {
                    cx.spawn_in(window, async move |view, cx| {
                        cx.background_executor().timer(HOLD_THRESHOLD).await;
                        let _ = view.update_in(cx, |view, window, cx| {
                            if view.run.is_none()
                                && let Some(press) = view.key_press.as_mut()
                                && press.started == started
                                && !press.interrupted
                            {
                                press.recording = true;
                                let target = view
                                    .key_monitor
                                    .as_ref()
                                    .and_then(KeyMonitor::capture_target);
                                view.start(Input::Dictation, target, window, cx);
                            }
                        });
                    })
                    .detach();
                }
            }
            Key::Up => {
                let Some(press) = self.key_press.take() else {
                    return;
                };
                if press.recording {
                    self.request_finish(cx);
                } else if !press.interrupted && press.started.elapsed() < HOLD_THRESHOLD {
                    match self.run.as_ref().map(|run| &run.input) {
                        None => {
                            let target = self
                                .key_monitor
                                .as_ref()
                                .and_then(KeyMonitor::capture_target);
                            self.start(Input::Dictation, target, window, cx);
                        }
                        Some(Input::Dictation) => self.request_finish(cx),
                        _ => {}
                    }
                }
            }
            Key::Escape => {
                if let Some(press) = &mut self.key_press {
                    press.interrupted = true;
                }
                if self
                    .run
                    .as_ref()
                    .is_some_and(|run| run.input == Input::Dictation)
                {
                    self.cancel(cx);
                }
            }
            Key::Other => {
                if let Some(press) = &mut self.key_press {
                    press.interrupted = true;
                }
            }
            Key::Down => {}
        }
    }
}

type RecognitionJob = Box<dyn FnOnce(&mut Option<LoadedModels>) + Send>;

struct LoadedModels {
    root: PathBuf,
    variant: HaasVariant,
    files: HaasModelFiles,
    engine: HaasEngine,
    vad: Option<SileroVad>,
}

impl LoadedModels {
    fn get<'a>(
        loaded: &'a mut Option<Self>,
        root: PathBuf,
        variant: HaasVariant,
        cancellation: &Cancellation,
    ) -> Result<&'a mut Self> {
        if loaded
            .as_ref()
            .is_some_and(|models| models.root == root && models.variant == variant)
        {
            return Ok(loaded.as_mut().expect("matching models are loaded"));
        }
        *loaded = None;
        let files =
            ModelCache::new(root.clone()).prepare_haas_variant(variant, cancellation, |_| {})?;
        let engine = files.load(ComputeConfig::standard())?;
        Ok(loaded.insert(Self {
            root,
            variant,
            files,
            engine,
            vad: None,
        }))
    }

    fn vad(&mut self, cancellation: &Cancellation) -> Result<&SileroVad> {
        if self.vad.is_none() {
            let path = ModelCache::new(self.root.clone()).prepare_silero(cancellation, |_| {})?;
            self.vad = Some(SileroVad::load(&path)?);
        }
        Ok(self.vad.as_ref().expect("voice detector is loaded"))
    }
}

enum Feed {
    Sample(PathBuf, Receiver<()>),
    Microphone(Vec<f32>, Receiver<Result<Vec<f32>>>),
    File(AudioSource),
}

fn open_feed(
    input: &Input,
    stop: Receiver<()>,
    updates: &Sender<Update>,
    cancellation: &Cancellation,
) -> Result<Feed> {
    Ok(match input {
        Input::Sample(path) => Feed::Sample(path.clone(), stop),
        Input::File(path) => Feed::File(AudioSource::open(path, cancellation)?),
        Input::Recording(_) | Input::Dictation => {
            let source = input.recording_source().expect("live recording input");
            let (sender, receiver) = mpsc::sync_channel(768);
            let updates = updates.clone();
            thread::spawn(move || {
                if let Err(error) = capture_audio(source, &stop, &updates, &sender) {
                    let _ = sender.send(Err(error));
                }
            });
            let first_audio = receiver
                .recv()
                .map_err(|_| Error::Invalid("マイクから音声を取得できませんでした".into()))??;
            Feed::Microphone(first_audio, receiver)
        }
    })
}

fn report_recognition_failure(error: Error, updates: &Sender<Update>) {
    let update = match error {
        Error::Cancelled => Update::Cancelled,
        error => {
            eprintln!("Recognition failed: {error}");
            Update::Failed(error.to_string())
        }
    };
    let _ = updates.send(update);
}

fn recognize(
    cache_path: Option<PathBuf>,
    input: Input,
    variant: HaasVariant,
    feed: Feed,
    updates: &Sender<Update>,
    cancellation: &Cancellation,
    loaded: &mut Option<LoadedModels>,
) -> Result<()> {
    let cache_path =
        cache_path.ok_or_else(|| Error::Invalid("Cannot locate the Mozu model cache".into()))?;
    let models = LoadedModels::get(loaded, cache_path, variant, cancellation)?;
    if !matches!(feed, Feed::File(_)) {
        models.vad(cancellation)?;
    }
    let engine = &models.engine;
    let terms = History::open()?
        .dictionary()?
        .iter()
        .map(history::DictionaryEntry::vocabulary_term)
        .collect::<Result<Vec<_>>>()?;
    let vocabulary = if terms.is_empty() {
        None
    } else {
        let dictionary = Vocabulary::new(terms, VocabularyConfig::standard())?;
        Some(Arc::new(engine.compile_vocabulary(
            dictionary,
            &models.files.assets.with_file_name("tokenizer.model"),
        )?))
    };
    let feed = match feed {
        Feed::File(source) => {
            let _ = updates.send(Update::FileProgress {
                progress: BatchProgress {
                    processed_samples: 0,
                    total_samples: 0,
                },
                text: String::new(),
            });
            let report = |update: batch::BatchUpdate<'_>| {
                updates
                    .send(Update::FileProgress {
                        progress: update.progress,
                        text: update.text.to_owned(),
                    })
                    .map_err(|_| Error::Cancelled)
            };
            let recognition = match vocabulary {
                Some(vocabulary) => batch::transcribe_quiet_source_with_vocabulary(
                    engine,
                    source,
                    vocabulary,
                    cancellation,
                    report,
                ),
                None => batch::transcribe_quiet_source_with_updates(
                    engine,
                    source,
                    cancellation,
                    report,
                ),
            }?;
            let source_name = match &input {
                Input::File(path) => path.file_name().map(|name| name.to_string_lossy()),
                _ => unreachable!("the file feed carries a file input"),
            };
            let history_id = History::open()
                .and_then(|history| {
                    history.save(
                        &recognition.result.text,
                        recognition.result.metadata.audio_samples,
                        variant,
                        HistoryKind::Transcription,
                        source_name.as_deref(),
                        false,
                    )
                })
                .map(Some);
            let _ = updates.send(Update::Finished {
                text: recognition.result.text,
                history_id,
            });
            return Ok(());
        }
        feed => feed,
    };
    let vad = models.vad.as_ref().expect("streaming models are loaded");
    let config = AsrConfig::interactive_for(engine.input_limits())?;
    let mut session = match vocabulary {
        Some(vocabulary) => StreamingSession::new_with_vocabulary(engine, vad, config, vocabulary),
        None => StreamingSession::new(engine, vad, config),
    }?;
    if updates.send(Update::ModelReady).is_err() {
        return Ok(());
    }
    match feed {
        Feed::File(_) => unreachable!("file transcription completes before streaming"),
        Feed::Sample(path, stop) => {
            replay(&path, &mut session, &stop, updates, cancellation)?;
        }
        Feed::Microphone(first_audio, capture) => {
            for audio in std::iter::once(Ok(first_audio)).chain(capture) {
                if !publish_audio(&mut session, &audio?, updates, cancellation)? {
                    return Ok(());
                }
            }
        }
    }
    let (result, _) = session.finish(cancellation)?;
    let empty = result.text.trim().is_empty();
    if empty && input.recording_source().is_some() {
        return Err(Error::Invalid("音声を認識できませんでした".into()));
    }
    let history_id = if matches!(input, Input::Sample(_)) {
        Ok(None)
    } else {
        History::open()
            .and_then(|history| {
                history.save(
                    &result.text,
                    result.audio_samples,
                    variant,
                    match input {
                        Input::Recording(_) => HistoryKind::Transcription,
                        _ => HistoryKind::Dictation,
                    },
                    None,
                    input == Input::Dictation,
                )
            })
            .map(Some)
    };
    let _ = updates.send(Update::Finished {
        text: result.text,
        history_id,
    });
    Ok(())
}

fn replay(
    path: &Path,
    session: &mut StreamingSession<'_>,
    stop: &Receiver<()>,
    updates: &Sender<Update>,
    cancellation: &Cancellation,
) -> Result<()> {
    let mut source = WavSource::open(path)?;
    let rate = SAMPLE_RATE as usize;
    let audio = source.read_block(24 * rate)?;
    if updates.send(Update::Listening).is_err() {
        return Ok(());
    }
    let start = Instant::now();
    let mut samples = 0;
    let mut meter = island::AudioMeter::new();
    for chunk in audio.chunks(rate / 10) {
        samples += chunk.len();
        let seconds = samples as f64 / rate as f64;
        let wait = Duration::from_secs_f64(seconds).saturating_sub(start.elapsed());
        if !matches!(
            stop.recv_timeout(wait),
            Err(mpsc::RecvTimeoutError::Timeout)
        ) {
            break;
        }
        if !meter.send(chunk, updates)? || !publish_audio(session, chunk, updates, cancellation)? {
            break;
        }
    }
    Ok(())
}

enum LiveRecording {
    Microphone(Recording),
    Meeting(MeetingRecording),
}

impl LiveRecording {
    fn start(source: RecordingSource) -> Result<Self> {
        match source {
            RecordingSource::Microphone => Recording::start().map(Self::Microphone),
            RecordingSource::Meeting => MeetingRecording::start().map(Self::Meeting),
        }
    }

    fn sample_rate(&self) -> u32 {
        match self {
            Self::Microphone(recording) => recording.device().sample_rate,
            Self::Meeting(recording) => recording.sample_rate(),
        }
    }

    fn drain_input(&mut self, maximum: usize) -> Result<Vec<f32>> {
        match self {
            Self::Microphone(recording) => recording.drain_input(maximum),
            Self::Meeting(recording) => recording.drain_input(maximum),
        }
    }

    fn stop_input(self) -> Result<Vec<f32>> {
        match self {
            Self::Microphone(recording) => recording.stop_input(),
            Self::Meeting(recording) => recording.stop_input(),
        }
    }
}

fn capture_audio(
    source: RecordingSource,
    stop: &Receiver<()>,
    updates: &Sender<Update>,
    audio: &SyncSender<Result<Vec<f32>>>,
) -> Result<()> {
    match stop.try_recv() {
        Ok(()) | Err(TryRecvError::Disconnected) => return Ok(()),
        Err(TryRecvError::Empty) => {}
    }
    let mut recording = LiveRecording::start(source)?;
    let source_rate = recording.sample_rate();
    let mut converter = RateConverter::new(source_rate)?;
    let mut meter = island::AudioMeter::new();
    if updates.send(Update::Listening).is_err() {
        return Ok(());
    }
    loop {
        match stop.recv_timeout(Duration::from_millis(50)) {
            Ok(()) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        let native = recording.drain_input(source_rate as usize)?;
        let samples = converter.push(&native)?;
        if !meter.send(&samples, updates)? || !send_audio(audio, samples)? {
            return Ok(());
        }
    }
    for native in recording.stop_input()?.chunks(source_rate as usize) {
        let samples = converter.push(native)?;
        if !meter.send(&samples, updates)? || !send_audio(audio, samples)? {
            return Ok(());
        }
    }
    let tail = converter.finish()?;
    if meter.send(&tail, updates)? {
        send_audio(audio, tail)?;
    }
    Ok(())
}

fn send_audio(audio: &SyncSender<Result<Vec<f32>>>, samples: Vec<f32>) -> Result<bool> {
    if samples.is_empty() {
        return Ok(true);
    }
    match audio.try_send(Ok(samples)) {
        Ok(()) => Ok(true),
        Err(mpsc::TrySendError::Disconnected(_)) => Ok(false),
        Err(mpsc::TrySendError::Full(_)) => Err(Error::Invalid(
            "認識の準備中に録音バッファがいっぱいになりました".into(),
        )),
    }
}

fn publish_audio(
    session: &mut StreamingSession<'_>,
    audio: &[f32],
    updates: &Sender<Update>,
    cancellation: &Cancellation,
) -> Result<bool> {
    if audio.is_empty() {
        return Ok(true);
    }
    for chunk in audio.chunks(SAMPLE_RATE as usize) {
        for event in session.push_audio(chunk, cancellation)? {
            if updates
                .send(Update::Transcript {
                    confirmed: event.confirmed_text,
                    provisional: event.provisional_text,
                })
                .is_err()
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

impl Render for MozuApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let material = self.material.tint(&theme);
        let chrome_border = theme.border.opacity(0.72);
        let active = self.run.is_some();
        let page_mode = match self.page {
            Page::Recognition(mode) => Some(mode),
            _ => None,
        };
        let mode = page_mode.unwrap_or(self.mode);
        let screen = self.screen_for(mode);
        let dictation = mode == Mode::Dictation;
        let transcribing = active && self.mode == Mode::Transcription;
        let dictating = active && self.mode == Mode::Dictation;
        let show_history = dictation || matches!(screen.phase, Phase::Idle);
        let can_copy = !dictation && !transcribing && !screen.transcript.text.is_empty();
        let show_start_actions = page_mode == Some(Mode::Transcription)
            && !transcribing
            && show_history
            && !screen.history.has_detail();
        let compact = window.viewport_size().width < px(680.0);
        let dictation_badge = dictating.then(|| {
            let stopping = matches!(self.screen.phase, Phase::Stopping);
            div()
                .flex()
                .items_center()
                .justify_center()
                .gap_2()
                .h(px(26.0))
                .px_2()
                .rounded_full()
                .bg(theme.control_track_fill())
                .text_xs()
                .text_color(theme.foreground)
                .child(
                    self.screen
                        .spectrum
                        .wave(island::WaveSize::Badge, theme.foreground),
                )
                .children((!compact).then_some(if stopping {
                    "処理中"
                } else {
                    "音声入力中"
                }))
        });
        let nav_item = |label, page, icon| {
            SidebarMenuButton::new(label, label)
                .icon(icon, rems(1.0))
                .active(self.page == page)
                .collapsed(compact)
                .on_click(std::rc::Rc::new(cx.listener(move |view, _, _, cx| {
                    view.navigate(page, cx);
                })))
        };
        let sidebar = div()
            .w(px(if compact { 58.0 } else { 172.0 }))
            .bg(material::sidebar_tint(material, &theme))
            .flex_none()
            .flex()
            .flex_col()
            .when(compact, |sidebar| sidebar.items_center())
            .gap_1()
            .pt(px(TITLEBAR_HEIGHT))
            .pb_3()
            .px_2()
            .border_r_1()
            .border_color(theme.sidebar_divider)
            .child(nav_item(
                Mode::Dictation.title(),
                Page::Recognition(Mode::Dictation),
                IconName::Microphone,
            ))
            .child(nav_item(
                Mode::Transcription.title(),
                Page::Recognition(Mode::Transcription),
                IconName::Files,
            ))
            .child(nav_item("辞書", Page::Dictionary, IconName::Plan))
            .child(div().flex_1())
            .children(dictation_badge)
            .child(nav_item("モデル", Page::Models, IconName::Components))
            .child(nav_item("設定", Page::Settings, IconName::Setting));
        let unavailable = active || self.models.is_busy();
        let open_button = show_start_actions.then(|| {
            Button::new("open-file")
                .icon(IconName::FolderOpen)
                .label("ファイルを開く")
                .disabled(unavailable)
                .on_click(cx.listener(|view, _, window, cx| view.open_file(window, cx)))
        });
        let source_picker = show_start_actions.then(|| {
            let view = cx.entity().downgrade();
            let choice = |source: RecordingSource| {
                let view = view.clone();
                DropdownMenuRadioItem::new(
                    source.label(),
                    source.label(),
                    self.recording_source == source,
                )
                .on_select(move |_, cx| {
                    let _ = view.update(cx, |view, cx| {
                        view.recording_source = source;
                        cx.notify();
                    });
                })
            };
            DropdownMenu::new(
                "recording-source",
                Button::new("recording-source-button")
                    .label(self.recording_source.label())
                    .icon(IconName::ChevronDown)
                    .variant(ButtonVariant::Outline)
                    .disabled(unavailable),
            )
            .align(PopoverAlign::End)
            .radio_item(choice(RecordingSource::Microphone))
            .radio_item(choice(RecordingSource::Meeting))
        });
        let record = show_start_actions.then(|| {
            Button::new("record")
                .icon(IconName::Microphone)
                .label("録音")
                .disabled(unavailable)
                .variant(ButtonVariant::Outline)
                .on_click(cx.listener(|view, _, window, cx| {
                    view.start(Input::Recording(view.recording_source), None, window, cx)
                }))
        });
        let stop = match screen.phase {
            Phase::Preparing | Phase::Transcribing(_) | Phase::Stopping => Button::new("cancel")
                .label("キャンセル")
                .variant(ButtonVariant::Outline)
                .disabled(self.run.as_ref().is_some_and(|run| run.discard))
                .on_click(cx.listener(|view, _, _, cx| view.stop(cx))),
            _ => Button::new("stop")
                .icon(IconName::Stop)
                .label("停止")
                .variant(ButtonVariant::Destructive)
                .on_click(cx.listener(|view, _, _, cx| view.stop(cx))),
        };
        let copy = can_copy.then(|| {
            Button::new("copy-transcript")
                .icon(if screen.copied {
                    IconName::Check
                } else {
                    IconName::Copy
                })
                .label(if screen.copied {
                    "コピー済み"
                } else {
                    "コピー"
                })
                .size(ButtonSize::Small)
                .variant(ButtonVariant::GhostSecondary)
                .on_click(cx.listener(|view, _, _, cx| {
                    let screen = view.screen_for_mut(Mode::Transcription);
                    cx.write_to_clipboard(ClipboardItem::new_string(
                        screen.transcript.text.clone(),
                    ));
                    screen.copied = true;
                    cx.notify();
                }))
        });
        let settings = (self.page == Page::Settings).then(|| {
            let shortcut = self.key_monitor.is_none().then(|| {
                Button::new("dictation-shortcut")
                    .label("Fnキーを有効にする")
                    .size(ButtonSize::Small)
                    .variant(ButtonVariant::Outline)
                    .on_click(cx.listener(|view, _, window, cx| {
                        dictation::request_accessibility_permission();
                        view.install_key_monitor(window, cx);
                        cx.notify();
                    }))
            });
            settings::render(self, shortcut, &theme, cx)
        });
        let dictionary =
            (self.page == Page::Dictionary).then(|| settings::render_dictionary(self, &theme, cx));
        let history = show_history.then(|| {
            if dictation && screen.history.is_empty() {
                let empty = Empty::new("音声入力の履歴はありません").media(
                    EmptyMediaVariant::Default,
                    Icon::new(IconName::Microphone, rems(1.5)).color(theme.muted_foreground),
                );
                if self.key_monitor.is_some() {
                    empty.description("Fnキーを押して話す").into_any_element()
                } else {
                    empty
                        .content(
                            Button::new("open-voice-input-settings")
                                .label("Fnキーの設定を開く")
                                .size(ButtonSize::Small)
                                .variant(ButtonVariant::Ghost)
                                .on_click(
                                    cx.listener(|view, _, _, cx| view.navigate(Page::Settings, cx)),
                                ),
                        )
                        .into_any_element()
                }
            } else {
                screen.history.render(mode, &theme, cx).into_any_element()
            }
        });
        let back_to_list = (!transcribing && !show_history).then(|| {
            Button::new("back-to-recognition-list")
                .icon(IconName::ArrowLeft)
                .label("一覧に戻る")
                .size(ButtonSize::Small)
                .variant(ButtonVariant::Ghost)
                .on_click(cx.listener(|view, _, _, cx| {
                    view.screen_for_mut(Mode::Transcription).clear_result();
                    cx.notify();
                }))
        });
        let models = (self.page == Page::Models).then(|| self.models.render(active, &theme, cx));
        let transcript = (!show_history).then(|| {
            div()
                .id("transcript")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&screen.scroll)
                .px(px(8.0))
                .py(px(20.0))
                .text_size(px(18.0))
                .line_height(px(32.0))
                .child(screen.transcript.text.clone())
        });
        let preview = (!dictation && matches!(screen.phase, Phase::Listening)).then(|| {
            div()
                .flex()
                .items_center()
                .gap(px(12.0))
                .h(px(48.0))
                .flex_none()
                .overflow_hidden()
                .px(px(8.0))
                .py(px(12.0))
                .border_t_1()
                .border_color(chrome_border)
                .child(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child("認識中"),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .whitespace_nowrap()
                        .text_ellipsis_start()
                        .text_sm()
                        .line_height(px(22.0))
                        .text_color(theme.muted_foreground)
                        .child(screen.transcript.provisional.clone()),
                )
        });
        let error = match &screen.phase {
            Phase::Failed(message) => Some(
                div()
                    .px(px(8.0))
                    .py(px(10.0))
                    .text_sm()
                    .text_color(theme.destructive)
                    .child(message.clone()),
            ),
            _ => None,
        };
        let file_name = screen.file_name.clone();
        let recording_status = (!dictation && transcribing).then(|| {
            let seconds = screen.seconds as u64;
            let (status, indicator) = match &screen.phase {
                Phase::Preparing => ("準備中".into(), theme.orange_600),
                Phase::Listening => ("録音中".into(), theme.destructive),
                Phase::Transcribing(progress) if progress.total_samples > 0 => (
                    format!(
                        "文字起こし中 · {:.0}%",
                        100.0 * progress.processed_samples as f64 / progress.total_samples as f64
                    ),
                    theme.blue_500,
                ),
                Phase::Transcribing(_) => ("文字起こし中".into(), theme.blue_500),
                Phase::Stopping
                    if self
                        .run
                        .as_ref()
                        .is_some_and(|run| !run.model_ready && !run.discard && seconds > 0) =>
                {
                    ("モデルを準備中".into(), theme.orange_600)
                }
                _ => ("処理中".into(), theme.orange_600),
            };
            let listening = matches!(screen.phase, Phase::Listening);
            div()
                .flex()
                .items_center()
                .gap(px(9.0))
                .text_sm()
                .children(
                    listening.then(|| screen.spectrum.wave(island::WaveSize::Badge, indicator)),
                )
                .children((!listening).then(|| div().size(px(7.0)).rounded_full().bg(indicator)))
                .child(status)
                .child(div().text_color(theme.muted_foreground).child(format!(
                    "{:02}:{:02}",
                    seconds / 60,
                    seconds % 60
                )))
        });
        let recognition_title = page_mode.map(Mode::title);
        let page_title = match self.page {
            Page::Dictionary => Some("辞書"),
            _ => recognition_title,
        };
        let dictionary_add =
            (self.page == Page::Dictionary).then(|| settings::dictionary_add_button(self, cx));
        let recognition = recognition_title.map(|_| {
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .px(px(16.0))
                .overflow_hidden()
                .children(history)
                .children(
                    (file_name.is_some() || can_copy || back_to_list.is_some()).then(|| {
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(8.0))
                            .flex_none()
                            .h(px(46.0))
                            .px(px(8.0))
                            .border_b_1()
                            .border_color(chrome_border)
                            .children(back_to_list)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .children(file_name),
                            )
                            .children(copy)
                    }),
                )
                .children(error)
                .children(transcript)
                .children(preview)
        });
        div()
            .size_full()
            .text_color(theme.foreground)
            .flex()
            .child(sidebar)
            .child(
                div()
                    .flex_1()
                    .bg(material)
                    .min_w_0()
                    .min_h_0()
                    .when(page_title.is_none(), |content| {
                        content.pt(px(TITLEBAR_HEIGHT))
                    })
                    .flex()
                    .flex_col()
                    .children(page_title.map(|title| {
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .h(px(TITLEBAR_HEIGHT))
                            .px(px(24.0))
                            .justify_between()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .flex_none()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(12.0))
                                    .flex_none()
                                    .children(recording_status)
                                    .children(dictionary_add)
                                    .children(open_button)
                                    .children(source_picker)
                                    .children(record)
                                    .children((!dictation && transcribing).then_some(stop)),
                            )
                    }))
                    .children(settings)
                    .children(dictionary)
                    .children(models)
                    .children(recognition),
            )
    }
}

fn segment_track(theme: &G2VTheme) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(px(2.0))
        .p(px(2.0))
        .rounded_full()
        .bg(theme.control_track_fill())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_before_capture_reports_missing_audio_without_loading_models() {
        let (stop, requests) = mpsc::channel();
        stop.send(()).unwrap();
        let (updates, _) = mpsc::channel();

        let error = open_feed(&Input::Dictation, requests, &updates, &Cancellation::new())
            .err()
            .expect("capture stopped without audio");

        assert!(
            matches!(error, Error::Invalid(message) if message == "マイクから音声を取得できませんでした")
        );
    }

    #[test]
    fn dark_control_fills_keep_their_contrast_over_window_material() {
        let theme = G2VTheme::for_appearance(gpui::WindowAppearance::Dark);
        let track = theme.control_track_fill();
        let hover = theme.control_hover_fill();
        let selected = theme.control_selected_fill();
        assert_eq!([track.a, hover.a, selected.a], [1.0; 3]);
        assert!(track.l < hover.l && hover.l < selected.l);
    }

    #[test]
    fn provisional_revision_preserves_confirmed_text_and_final_text_replaces_it() {
        let mut transcript = Transcript::default();
        transcript.update("お茶です。".into(), "あした".into());
        transcript.update("お茶です。".into(), "明日".into());
        assert_eq!(transcript.text, "お茶です。");
        assert_eq!(transcript.provisional, "明日");
        transcript.finish("お茶です。明日。".into());
        assert_eq!(transcript.text, "お茶です。明日。");
        assert!(transcript.provisional.is_empty());
    }
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let command = args.next();
    if command.as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let sample = if command.as_deref() == Some(std::ffi::OsStr::new("--sample")) {
        let Some(path) = args.next() else {
            eprintln!("Usage: mozu --sample <wav>");
            std::process::exit(2);
        };
        Some(PathBuf::from(path))
    } else {
        None
    };
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            gpui_base::init(cx);
            typography::register_fonts(cx);
            typography::install(cx);
            theme::install(cx.window_appearance(), cx);
            let bounds = Bounds::centered(None, size(px(760.0), px(540.0)), cx);
            let console = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(600.0), px(360.0))),
                        window_background: WindowBackgroundAppearance::Opaque,
                        titlebar: Some(TitlebarOptions {
                            title: None,
                            appears_transparent: true,
                            traffic_light_position: Some(point(
                                px(16.0),
                                px((TITLEBAR_HEIGHT - 14.0) / 2.0),
                            )),
                        }),
                        ..Default::default()
                    },
                    |window, cx| {
                        cx.new(|cx| {
                            let mut demo = MozuApp::new(window, cx);
                            demo.install_key_monitor(window, cx);
                            cx.observe_window_activation(window, |view, window, cx| {
                                view.material.sync(window);
                                view.install_key_monitor(window, cx);
                                cx.notify();
                            })
                            .detach();
                            cx.observe_window_appearance(window, |view, window, cx| {
                                theme::install(window.appearance(), cx);
                                view.material.sync(window);
                                cx.notify();
                            })
                            .detach();
                            if let Some(path) = &sample {
                                demo.start(Input::Sample(path.clone()), None, window, cx);
                            } else {
                                demo.warm_recognition();
                            }
                            demo
                        })
                    },
                )
                .expect("open Mozu window");
            menu::install(console, cx);
            cx.activate(true);
            if sample.is_none() {
                if let Err(error) = island::open(console, cx) {
                    eprintln!("Could not open the dictation island: {error}");
                }
                dictation::request_accessibility_permission_on_first_launch();
            }
        });
}
