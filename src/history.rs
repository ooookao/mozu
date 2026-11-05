use std::{
    fs,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::Duration,
};

use g2v_ui::{
    theme::colors::G2VTheme,
    ui::{Button, ButtonSize, ButtonVariant, IconName},
};
use gpui::{ClipboardItem, Context, IntoElement, div, prelude::*, px};
use mozu_audio::{Error, Result, models::HaasVariant, vocabulary::VocabularyTerm};
use rusqlite::{Connection, params};

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

pub(crate) struct HistoryEntry {
    pub(crate) id: i64,
    pub(crate) preview: String,
    pub(crate) source_name: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Default)]
pub(crate) struct HistoryView {
    entries: Vec<HistoryEntry>,
    detail: Option<(i64, String)>,
    error: Option<String>,
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
        match History::open().and_then(|history| history.entries(kind)) {
            Ok(entries) => {
                self.entries = entries;
                self.error = None;
            }
            Err(error) => {
                self.entries.clear();
                self.error = Some(error.to_string());
            }
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
        theme: &G2VTheme,
        cx: &Context<MozuApp>,
    ) -> impl IntoElement {
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
                                .icon(IconName::ArrowLeft)
                                .label("戻る")
                                .size(ButtonSize::Small)
                                .variant(ButtonVariant::Ghost)
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.screen_for_mut(mode).history.close_detail();
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new(("copy-history", *id as u64))
                                .icon(IconName::Copy)
                                .label("コピー")
                                .size(ButtonSize::Small)
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
            panel = panel.child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("履歴はありません"),
            );
        } else {
            panel = panel.children(self.entries.iter().map(|entry| {
                let id = entry.id;
                let label = entry.source_name.as_ref().unwrap_or(&entry.preview);
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_t_1()
                    .border_color(theme.border)
                    .py_2()
                    .child(
                        div().flex_1().min_w_0().overflow_hidden().child(
                            Button::new(("history-entry", id as u64))
                                .label(label.clone())
                                .full_width()
                                .content_alignment(g2v_ui::ui::ButtonContentAlignment::Start)
                                .variant(ButtonVariant::Ghost)
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.screen_for_mut(mode).history.open_entry(id);
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(entry.created_at.clone()),
                    )
            }));
        }
        div()
            .id("history")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_2()
            .py_3()
            .child(panel)
    }
}

#[derive(Debug, PartialEq, Eq)]
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
        if !(0..=3).contains(&version) {
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

    pub(crate) fn entries(&self, kind: HistoryKind) -> Result<Vec<HistoryEntry>> {
        let mut statement = self.0.prepare(
            "SELECT id, substr(raw_text, 1, 120), source_name, datetime(created_at, 'localtime')
             FROM dictations WHERE kind = ?1 ORDER BY id DESC",
        ).map_err(database_error)?;
        statement
            .query_map([kind.key()], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    preview: row.get(1)?,
                    source_name: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(database_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(database_error)
    }

    pub(crate) fn text(&self, id: i64) -> Result<String> {
        self.0
            .query_row(
                "SELECT raw_text FROM dictations WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .map_err(database_error)
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

fn database_error(error: rusqlite::Error) -> Error {
    Error::Invalid(format!("Dictation history database: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                "SELECT raw_text FROM dictations WHERE id = ?1",
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
        let dictation = history.entries(HistoryKind::Dictation).unwrap();
        let transcription = history.entries(HistoryKind::Transcription).unwrap();
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
