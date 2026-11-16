use std::{
    fs,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use gpui::{ClipboardItem, Context, IntoElement, Window, div, prelude::*, px, rems};
use mozu_audio::{Error, Result, models::HaasVariant, vocabulary::VocabularyTerm};
use mozu_ui::{
    appearance::MozuTheme,
    ui::{Button, ButtonSize, ButtonVariant, Empty, Icon, IconName},
};
use rusqlite::{Connection, named_params, params};

use crate::{Mode, MozuApp};

pub struct History(Connection);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum HistoryKind {
    Dictation,
    Transcription,
}

impl HistoryKind {
    fn key(self) -> &'static str {
        match self {
            Self::Dictation => "dictation",
            Self::Transcription => "transcription",
        }
    }
}

const HISTORY_PAGE_SIZE: usize = 50;

#[derive(Clone)]
pub(crate) struct HistoryEntry {
    pub(crate) id: i64,
    pub(crate) preview: String,
    pub(crate) source_name: Option<String>,
    pub(crate) created_at: String,
}

struct HistoryPage {
    entries: Vec<HistoryEntry>,
    has_more: bool,
}

#[derive(Default)]
pub(crate) struct HistoryView {
    entries: Vec<HistoryEntry>,
    detail: Option<(i64, String)>,
    error: Option<String>,
    has_more: bool,
    page_cursors: Vec<Option<i64>>,
    scroll: gpui::UniformListScrollHandle,
}

impl HistoryView {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.detail.is_none() && self.error.is_none()
    }

    pub(crate) fn has_detail(&self) -> bool {
        self.detail.is_some()
    }

    pub(crate) fn close_detail(&mut self) {
        self.detail = None;
    }

    pub(crate) fn refresh(&mut self, kind: HistoryKind) {
        match History::open().and_then(|history| history.entries_page(kind, None)) {
            Ok(page) => {
                self.apply_page(page);
                self.page_cursors = vec![None];
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn apply_page(&mut self, page: HistoryPage) {
        self.entries = page.entries;
        self.has_more = page.has_more;
        self.error = None;
        self.scroll
            .scroll_to_item_strict(0, gpui::ScrollStrategy::Top);
    }

    fn next_page(&mut self, kind: HistoryKind) {
        if !self.has_more {
            return;
        }
        let Some(cursor) = self.entries.last().map(|entry| entry.id) else {
            return;
        };
        match History::open().and_then(|history| history.entries_page(kind, Some(cursor))) {
            Ok(page) => {
                self.apply_page(page);
                self.page_cursors.push(Some(cursor));
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn previous_page(&mut self, kind: HistoryKind) {
        if self.page_cursors.len() < 2 {
            return;
        }
        let cursor = self.page_cursors[self.page_cursors.len() - 2];
        match History::open().and_then(|history| history.entries_page(kind, cursor)) {
            Ok(page) => {
                self.apply_page(page);
                self.page_cursors.pop();
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn open_entry(&mut self, id: i64) {
        match History::open().and_then(|history| history.text(id)) {
            Ok(text) => {
                self.detail = Some((id, text));
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(crate) fn render(
        &self,
        mode: Mode,
        theme: &MozuTheme,
        window: &Window,
        cx: &Context<MozuApp>,
    ) -> gpui::AnyElement {
        if self.detail.is_none() && !self.entries.is_empty() {
            return self.render_page(mode, window, cx);
        }
        let mut panel = div()
            .flex()
            .flex_col()
            .gap_3()
            .max_w(px(640.0))
            .mx_auto()
            .children(
                self.error
                    .clone()
                    .map(|error| div().text_sm().text_color(theme.destructive).child(error)),
            );
        if let Some((id, text)) = &self.detail {
            let clipboard_text = text.clone();
            panel = panel
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap_2()
                        .child(
                            Button::new("back-to-history")
                                .icon_start(Icon::new(IconName::ArrowLeft, rems(1.0)))
                                .label("戻る")
                                .size(ButtonSize::Sm)
                                .variant(ButtonVariant::Ghost)
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.screen_for_mut(mode).history.close_detail();
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new(("copy-history", *id as u64))
                                .icon_start(Icon::new(IconName::Copy, rems(1.0)))
                                .label("コピー")
                                .size(ButtonSize::Sm)
                                .variant(ButtonVariant::Outline)
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        clipboard_text.clone(),
                                    ));
                                }),
                        ),
                )
                .child(div().text_sm().line_height(px(24.0)).child(text.clone()));
        } else if self.entries.is_empty() && self.error.is_none() {
            panel = panel.child(Empty::new().title("履歴はありません"));
        }
        let viewport = kinu_ui::ScrollAreaViewport::new()
            .px_2()
            .py_3()
            .child(panel);
        mozu_ui::ui::scroll_area::scroll_area(
            "history-scroll",
            mozu_ui::ui::scroll_area::scroll_fade(viewport, gpui::Axis::Vertical),
            kinu_ui::ScrollAreaAxes::Vertical,
            window,
            cx,
        )
        .flex_1()
        .min_h_0()
        .into_any_element()
    }

    fn render_page(&self, mode: Mode, window: &Window, cx: &Context<MozuApp>) -> gpui::AnyElement {
        use kinu_ui::{TableColumn, TableRow};
        use mozu_ui::ui::{
            data_table::data_table,
            pagination::Pagination,
            utils::presentation::{ComponentTheme, ControlSize, TextRole},
        };
        let entries = self.entries.clone();
        let open = std::rc::Rc::new(cx.listener(move |view, id: &i64, _, cx| {
            view.screen_for_mut(mode).history.open_entry(*id);
            cx.notify();
        }));
        let tokens = ComponentTheme::current(cx);
        let text = TextRole::Body.metrics(tokens.size, &tokens);
        let header_height = text.line_height
            + if tokens.size == ControlSize::Compact {
                11.
            } else {
                17.
            };
        let pager_height = ControlSize::Compact.metrics(&tokens).control_height;
        let error_height = if self.error.is_some() {
            text.line_height + 12.
        } else {
            0.
        };
        let table = data_table(
            "history-table",
            entries.len(),
            move |index, _, _| {
                let entry = &entries[index];
                let id = entry.id;
                let open = open.clone();
                TableRow::new(
                    id as u64,
                    vec![
                        Button::new(("history-entry", id as u64))
                            .label(entry.source_name.as_ref().unwrap_or(&entry.preview).clone())
                            .size(ButtonSize::Compact)
                            .variant(ButtonVariant::Ghost)
                            .w_full()
                            .h_full()
                            .justify_start()
                            .px_0()
                            .on_click(move |_, window, cx| open(&id, window, cx))
                            .into_any_element(),
                        div().child(entry.created_at.clone()).into_any_element(),
                    ],
                )
            },
            window,
            cx,
        )
        .columns([
            TableColumn::new("content", "内容").flex(1.),
            TableColumn::new("created", "日時").width(px(150.)),
        ])
        .track_scroll(&self.scroll)
        .accessibility_label("音声入力・文字起こしの履歴")
        .scroll_fade(mozu_ui::ui::utils::scroll_fade::scroll_fade(
            gpui::Axis::Vertical,
        ))
        .viewport_height(
            (window.viewport_size().height
                - px(crate::TITLEBAR_HEIGHT
                    + 24.
                    + 12.
                    + header_height
                    + pager_height
                    + error_height))
            .max(px(40.)),
        );
        let pager = Pagination::new("history-pagination")
            .accessibility_label("履歴のページ移動")
            .item(
                Button::new("history-previous")
                    .label("前へ")
                    .size(ButtonSize::Compact)
                    .variant(ButtonVariant::Ghost)
                    .disabled(self.page_cursors.len() < 2)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.screen_for_mut(mode)
                            .history
                            .previous_page(mode.history_kind());
                        cx.notify();
                    })),
            )
            .item(div().child(format!("{}ページ", self.page_cursors.len().max(1))))
            .item(
                Button::new("history-next")
                    .label("次へ")
                    .size(ButtonSize::Compact)
                    .variant(ButtonVariant::Ghost)
                    .disabled(!self.has_more)
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.screen_for_mut(mode)
                            .history
                            .next_page(mode.history_kind());
                        cx.notify();
                    })),
            );
        div()
            .id("history-page")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .px_6()
            .py_3()
            .gap_3()
            .children(self.error.as_ref().map(|error| {
                div()
                    .text_color(mozu_ui::appearance::MozuTheme::global(cx).destructive)
                    .child(error.clone())
            }))
            .child(table)
            .child(pager)
            .into_any_element()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DictionaryEntry {
    pub(crate) id: i64,
    pub(crate) written: String,
    pub(crate) reading: Option<String>,
}

impl DictionaryEntry {
    pub(crate) fn vocabulary_term(&self) -> Result<VocabularyTerm> {
        VocabularyTerm::new(&self.written, self.reading.clone().into_iter().collect())
    }
}

impl History {
    pub fn open() -> Result<Self> {
        let directory = std::env::var_os("MOZU_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::home_dir().map(|home| home.join("Library/Application Support/Mozu"))
            })
            .ok_or_else(|| Error::Invalid("Cannot locate the Mozu data directory".into()))?;
        fs::create_dir_all(&directory).map_err(|source| Error::Io {
            path: directory.clone(),
            source,
        })?;
        Self::open_file(&directory.join("history.sqlite3"))
    }

    fn open_file(path: &Path) -> Result<Self> {
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(path)
            .map_err(|source| Error::Io {
                path: path.to_path_buf(),
                source,
            })?;
        let connection = Connection::open(path).map_err(|error| {
            Error::Invalid(format!("Open history database {}: {error}", path.display()))
        })?;
        Self::initialize(connection)
    }

    fn initialize(mut connection: Connection) -> Result<Self> {
        connection
            .busy_timeout(Duration::from_secs(2))
            .map_err(database_error)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .map_err(database_error)?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(database_error)?;
        let version: i64 = transaction
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(database_error)?;
        if !(0..=4).contains(&version) {
            return Err(Error::Invalid(format!(
                "Unsupported dictation history version: {version}"
            )));
        }
        if version == 0 {
            transaction.execute_batch(
            "CREATE TABLE IF NOT EXISTS dictations (
                id INTEGER PRIMARY KEY,
                raw_text TEXT NOT NULL,
                audio_samples INTEGER NOT NULL,
                encoder_frames INTEGER NOT NULL CHECK(encoder_frames IN (1500, 3072)),
                paste_status TEXT NOT NULL CHECK(paste_status IN ('saved', 'pending', 'shortcut_sent', 'skipped')),
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
             );",
            ).map_err(database_error)?;
        }
        if version < 2 {
            transaction
                .execute_batch(
                    "CREATE TABLE dictionary (
                    id INTEGER PRIMARY KEY,
                    written TEXT NOT NULL UNIQUE,
                    reading TEXT
                 );
                 PRAGMA user_version = 2;",
                )
                .map_err(database_error)?;
        }
        if version < 3 {
            transaction
                .execute_batch(
                    "ALTER TABLE dictations ADD COLUMN kind TEXT NOT NULL DEFAULT 'dictation'
                     CHECK(kind IN ('dictation', 'transcription'));
                 ALTER TABLE dictations ADD COLUMN source_name TEXT;
                 CREATE INDEX dictations_by_kind ON dictations (kind, id DESC);
                 PRAGMA user_version = 3;",
                )
                .map_err(database_error)?;
        }
        if version < 4 {
            transaction
                .execute_batch(
                    "ALTER TABLE dictations ADD COLUMN formatted_text TEXT;
                ALTER TABLE dictations ADD COLUMN format_model TEXT;
                PRAGMA user_version = 4;",
                )
                .map_err(database_error)?;
        }
        transaction.commit().map_err(database_error)?;
        Ok(Self(connection))
    }

    pub fn save(
        &self,
        text: &str,
        audio_samples: u64,
        variant: HaasVariant,
        kind: HistoryKind,
        source_name: Option<&str>,
        paste_requested: bool,
    ) -> Result<i64> {
        let frames = match variant {
            HaasVariant::Frames1500 => 1500,
            HaasVariant::Frames3072 => 3072,
        };
        let status = if paste_requested { "pending" } else { "saved" };
        let audio_samples = i64::try_from(audio_samples)
            .map_err(|_| Error::Invalid("Audio length exceeds the SQLite integer range".into()))?;
        self.0
            .query_row(
                "INSERT INTO dictations (raw_text, audio_samples, encoder_frames, paste_status, kind, source_name)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id",
                params![text, audio_samples, frames, status, kind.key(), source_name],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    fn entries_page(&self, kind: HistoryKind, before: Option<i64>) -> Result<HistoryPage> {
        let sql = history_page_query(before);
        let mut statement = self.0.prepare(&sql).map_err(database_error)?;
        let read = |row: &rusqlite::Row<'_>| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                preview: row.get(1)?,
                source_name: row.get(2)?,
                created_at: row.get(3)?,
            })
        };
        let limit = (HISTORY_PAGE_SIZE + 1) as i64;
        let rows = match before {
            Some(cursor) => statement.query_map(
                named_params! {":kind": kind.key(), ":before": cursor, ":limit": limit},
                read,
            ),
            None => statement.query_map(named_params! {":kind": kind.key(), ":limit": limit}, read),
        }
        .map_err(database_error)?;
        let mut entries = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database_error)?;
        let has_more = entries.len() > HISTORY_PAGE_SIZE;
        entries.truncate(HISTORY_PAGE_SIZE);
        Ok(HistoryPage { entries, has_more })
    }

    pub(crate) fn text(&self, id: i64) -> Result<String> {
        self.0
            .query_row(
                "SELECT COALESCE(formatted_text, raw_text) FROM dictations WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .map_err(database_error)
    }

    pub(crate) fn save_formatted(&self, id: i64, text: &str, model: &str) -> Result<()> {
        let changed = self
            .0
            .execute(
                "UPDATE dictations SET formatted_text = ?1, format_model = ?2 WHERE id = ?3",
                params![text, model, id],
            )
            .map_err(database_error)?;
        if changed != 1 {
            return Err(Error::Invalid(format!(
                "AI整形の保存対象がありません: dictation_id={id}"
            )));
        }
        Ok(())
    }

    pub fn record_paste(&self, id: i64, sent: bool) -> Result<()> {
        let status = if sent { "shortcut_sent" } else { "skipped" };
        self.0.execute(
            "UPDATE dictations SET paste_status = ?1 WHERE id = ?2 AND paste_status = 'pending'",
            params![status, id],
        ).map_err(database_error)?;
        Ok(())
    }

    pub fn discard(&self, id: i64) -> Result<()> {
        self.0
            .execute("DELETE FROM dictations WHERE id = ?1", [id])
            .map_err(database_error)?;
        Ok(())
    }

    pub(crate) fn dictionary(&self) -> Result<Vec<DictionaryEntry>> {
        let mut statement = self
            .0
            .prepare("SELECT id, written, reading FROM dictionary ORDER BY written")
            .map_err(database_error)?;
        statement
            .query_map([], |row| {
                Ok(DictionaryEntry {
                    id: row.get(0)?,
                    written: row.get(1)?,
                    reading: row.get(2)?,
                })
            })
            .map_err(database_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database_error)
    }

    pub(crate) fn save_dictionary_entry_validated(
        &self,
        id: Option<i64>,
        written: &str,
        reading: Option<&str>,
        validate: impl FnOnce(&[DictionaryEntry], &DictionaryEntry) -> Result<()>,
    ) -> Result<()> {
        let transaction =
            rusqlite::Transaction::new_unchecked(&self.0, rusqlite::TransactionBehavior::Immediate)
                .map_err(database_error)?;
        let mut entries = self.dictionary()?;
        entries.retain(|entry| Some(entry.id) != id);
        let candidate = DictionaryEntry {
            id: id.unwrap_or(-1),
            written: written.to_owned(),
            reading: reading.map(str::to_owned),
        };
        validate(&entries, &candidate)?;
        self.save_dictionary_entry(id, written, reading)?;
        transaction.commit().map_err(database_error)
    }

    pub(crate) fn save_dictionary_entry(
        &self,
        id: Option<i64>,
        written: &str,
        reading: Option<&str>,
    ) -> Result<()> {
        match id {
            Some(id) => self
                .0
                .query_row(
                    "UPDATE dictionary SET written = ?1, reading = ?2 WHERE id = ?3 RETURNING id",
                    params![written, reading, id],
                    |row| row.get::<_, i64>(0),
                )
                .map(|_| ()),
            None => self
                .0
                .execute(
                    "INSERT INTO dictionary (written, reading) VALUES (?1, ?2)",
                    params![written, reading],
                )
                .map(|_| ()),
        }
        .map_err(|error| {
            if error
                .sqlite_error()
                .is_some_and(|code| code.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE)
            {
                Error::Invalid("同じ表記の単語がすでに登録されています。".into())
            } else {
                database_error(error)
            }
        })
    }

    pub(crate) fn delete_dictionary_entry(&self, id: i64) -> Result<()> {
        self.0
            .execute("DELETE FROM dictionary WHERE id = ?1", [id])
            .map_err(database_error)?;
        Ok(())
    }
}

fn history_page_query(before: Option<i64>) -> String {
    let cursor_filter = if before.is_some() {
        "AND id < :before"
    } else {
        ""
    };
    format!("SELECT id, substr(COALESCE(formatted_text, raw_text), 1, 120), source_name, datetime(created_at, 'localtime')
        FROM dictations WHERE kind = :kind {cursor_filter} ORDER BY id DESC LIMIT :limit")
}

fn database_error(error: rusqlite::Error) -> Error {
    Error::Invalid(format!("Dictation history database: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_pages_have_no_duplicates_or_missing_entries_after_an_insert() {
        let history = History::initialize(Connection::open_in_memory().unwrap()).unwrap();
        history.0.execute_batch("WITH RECURSIVE rows(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM rows WHERE n<302)
            INSERT INTO dictations(raw_text,audio_samples,encoder_frames,paste_status,kind)
            SELECT '検証用履歴 ' || n,16000,3072,'saved',CASE WHEN n%2=0 THEN 'dictation' ELSE 'transcription' END FROM rows;").unwrap();
        let mut page = history.entries_page(HistoryKind::Dictation, None).unwrap();
        assert!(page.has_more);
        let first_cursor = page.entries.last().unwrap().id;
        history
            .save(
                "途中で追加された履歴",
                16000,
                HaasVariant::Frames3072,
                HistoryKind::Dictation,
                None,
                false,
            )
            .unwrap();
        let mut ids = page
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        let mut sizes = vec![page.entries.len()];
        while page.has_more {
            let cursor = page.entries.last().unwrap().id;
            page = history
                .entries_page(HistoryKind::Dictation, Some(cursor))
                .unwrap();
            sizes.push(page.entries.len());
            ids.extend(page.entries.iter().map(|entry| entry.id));
        }
        assert_eq!(sizes, vec![50, 50, 50, 1]);
        assert_eq!(ids, (1..=151).rev().map(|n| n * 2).collect::<Vec<_>>());
        let previous_page = history
            .entries_page(HistoryKind::Dictation, Some(first_cursor))
            .unwrap();
        assert_eq!(previous_page.entries.first().unwrap().id, 202);
        let refreshed = history.entries_page(HistoryKind::Dictation, None).unwrap();
        assert_eq!(refreshed.entries.first().unwrap().id, 303);
    }

    #[test]
    fn exact_page_size_has_no_empty_following_page() {
        let history = History::initialize(Connection::open_in_memory().unwrap()).unwrap();
        assert!(
            !history
                .entries_page(HistoryKind::Dictation, None)
                .unwrap()
                .has_more
        );
        history
            .0
            .execute_batch(
                "WITH RECURSIVE rows(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM rows WHERE n<50)
            INSERT INTO dictations(raw_text,audio_samples,encoder_frames,paste_status)
            SELECT '検証用履歴',16000,3072,'saved' FROM rows;",
            )
            .unwrap();
        let page = history.entries_page(HistoryKind::Dictation, None).unwrap();
        assert_eq!(page.entries.len(), HISTORY_PAGE_SIZE);
        assert!(!page.has_more);
        let last = page.entries.last().unwrap().id;
        assert!(
            history
                .entries_page(HistoryKind::Dictation, Some(last))
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn deep_page_seeks_the_existing_kind_and_id_index() {
        let history = History::initialize(Connection::open_in_memory().unwrap()).unwrap();
        let sql = format!("EXPLAIN QUERY PLAN {}", history_page_query(Some(10_000)));
        let mut statement = history.0.prepare(&sql).unwrap();
        let plan = statement
            .query_map(
                named_params! {":kind":"dictation",":before":10_000,":limit":51},
                |row| row.get::<_, String>(3),
            )
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
            .join("\n");
        assert!(plan.contains("dictations_by_kind"), "{plan}");
        assert!(plan.contains("id<?") || plan.contains("rowid<?"), "{plan}");
        assert!(!plan.contains("USE TEMP B-TREE"), "{plan}");
    }

    #[test]
    fn history_list_fetches_only_one_page() {
        let history = History::initialize(Connection::open_in_memory().unwrap()).unwrap();
        history
            .0
            .execute_batch(
                "WITH RECURSIVE rows(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM rows WHERE n<151)
            INSERT INTO dictations(raw_text,audio_samples,encoder_frames,paste_status,kind)
            SELECT '検証用履歴 ' || n,16000,3072,'saved','dictation' FROM rows;",
            )
            .unwrap();
        let entries = history
            .entries_page(HistoryKind::Dictation, None)
            .unwrap()
            .entries;
        assert_eq!(entries.len(), 50);
        assert_eq!(entries.first().unwrap().id, 151);
        assert_eq!(entries.last().unwrap().id, 102);
    }

    #[test]
    fn rejected_dictionary_validation_preserves_existing_entries() {
        let directory = tempfile::tempdir().unwrap();
        let history = History::open_file(&directory.path().join("history.sqlite3")).unwrap();
        history
            .save_dictionary_entry(None, "BaseUI", Some("べーすゆーあい"))
            .unwrap();
        let existing = history.dictionary().unwrap().pop().unwrap();

        let rejected = history.save_dictionary_entry_validated(
            None,
            "Base UI",
            Some("べーすゆーあい"),
            |entries, candidate| {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0], existing);
                assert_eq!(candidate.written, "Base UI");
                Err(Error::Invalid("conflicting reading".into()))
            },
        );
        assert!(rejected.is_err());
        assert_eq!(history.dictionary().unwrap(), vec![existing]);

        let id = history.dictionary().unwrap()[0].id;
        let rejected = history.save_dictionary_entry_validated(
            Some(id),
            "Changed",
            Some("へんこう"),
            |entries, _| {
                assert!(entries.is_empty());
                Err(Error::Invalid("rejected update".into()))
            },
        );
        assert!(rejected.is_err());
        let unchanged = history.dictionary().unwrap().pop().unwrap();
        assert_eq!(unchanged.written, "BaseUI");
        assert_eq!(unchanged.reading.as_deref(), Some("べーすゆーあい"));
    }

    #[test]
    fn formatting_migration_preserves_original_and_records_model() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let history = History::open_file(&path).unwrap();
        let id = history
            .save(
                "えっと明日です",
                16000,
                HaasVariant::Frames3072,
                HistoryKind::Dictation,
                None,
                false,
            )
            .unwrap();
        history
            .0
            .execute_batch(
                "ALTER TABLE dictations DROP COLUMN formatted_text;
            ALTER TABLE dictations DROP COLUMN format_model; PRAGMA user_version = 3;",
            )
            .unwrap();
        drop(history);
        let history = History::open_file(&path).unwrap();
        assert_eq!(history.text(id).unwrap(), "えっと明日です");
        history
            .save_formatted(id, "明日です。", "chosen-model")
            .unwrap();
        assert_eq!(history.text(id).unwrap(), "明日です。");
        let original: (String, String) = history
            .0
            .query_row(
                "SELECT raw_text, format_model FROM dictations WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(original, ("えっと明日です".into(), "chosen-model".into()));
    }

    #[test]
    fn dictionary_migration_keeps_existing_history() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let history = History::open_file(&path).unwrap();
        let id = history
            .save(
                "前の録音",
                16000,
                HaasVariant::Frames3072,
                HistoryKind::Dictation,
                None,
                false,
            )
            .unwrap();
        history
            .0
            .execute_batch(
                "DROP TABLE dictionary; DROP INDEX dictations_by_kind;
                 ALTER TABLE dictations DROP COLUMN kind;
                 ALTER TABLE dictations DROP COLUMN source_name;
                 ALTER TABLE dictations DROP COLUMN formatted_text;
                 ALTER TABLE dictations DROP COLUMN format_model;
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        drop(history);

        let history = History::open_file(&path).unwrap();
        history
            .save_dictionary_entry(None, "Mozu", Some("もず"))
            .unwrap();
        let entry = history.dictionary().unwrap().pop().unwrap();
        assert!(history.save_dictionary_entry(None, "Mozu", None).is_err());
        history
            .save_dictionary_entry(Some(entry.id), "Mozu Audio", None)
            .unwrap();
        drop(history);

        let history = History::open_file(&path).unwrap();
        let entry = history.dictionary().unwrap().pop().unwrap();
        assert_eq!(entry.written, "Mozu Audio");
        assert_eq!(entry.reading, None);
        history.delete_dictionary_entry(entry.id).unwrap();
        assert!(history.dictionary().unwrap().is_empty());
        let text: String = history
            .0
            .query_row(
                "SELECT COALESCE(formatted_text, raw_text) FROM dictations WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(text, "前の録音");
        let transcription_id = history
            .save(
                "音声ファイルの結果",
                16000,
                HaasVariant::Frames3072,
                HistoryKind::Transcription,
                Some("meeting.wav"),
                false,
            )
            .unwrap();
        let dictation = history
            .entries_page(HistoryKind::Dictation, None)
            .unwrap()
            .entries;
        let transcription = history
            .entries_page(HistoryKind::Transcription, None)
            .unwrap()
            .entries;
        assert_eq!(
            dictation.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            vec![id]
        );
        assert_eq!(
            transcription
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            vec![transcription_id]
        );
        assert_eq!(transcription[0].source_name.as_deref(), Some("meeting.wav"));
    }

    #[test]
    fn recognition_survives_a_skipped_paste() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("history.sqlite3");
        let history = History::open_file(&path).unwrap();
        let id = history
            .save(
                "テストです",
                24000,
                HaasVariant::Frames1500,
                HistoryKind::Dictation,
                None,
                true,
            )
            .unwrap();
        drop(history);
        let history = History::open_file(&path).unwrap();
        history.record_paste(id, false).unwrap();
        drop(history);
        let history = History::open_file(&path).unwrap();
        let row: (String, i64, String) = history
            .0
            .query_row(
                "SELECT raw_text, encoder_frames, paste_status FROM dictations WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(row, ("テストです".into(), 1500, "skipped".into()));
    }
}
