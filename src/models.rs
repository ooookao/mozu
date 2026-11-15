use std::{
    fs::{self, OpenOptions, TryLockError},
    path::{Path, PathBuf},
};

use gpui::{Context, IntoElement, Window, div, prelude::*, px, rems};
use mozu_audio::{
    Cancellation, Error, Result,
    models::{HaasVariant, ModelCache},
};
use mozu_ui::{
    appearance::MozuTheme,
    ui::{BrandIcon, BrandIconName, Button, ButtonSize, ButtonVariant, Icon, IconName},
};
use objc2_foundation::{NSUserDefaults, ns_string};

use crate::MozuApp;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Model {
    Haas3072,
    Haas1500,
    Silero,
}

impl Model {
    const ALL: [Self; 3] = [Self::Haas3072, Self::Haas1500, Self::Silero];

    fn encoder(variant: HaasVariant) -> Self {
        match variant {
            HaasVariant::Frames1500 => Self::Haas1500,
            HaasVariant::Frames3072 => Self::Haas3072,
        }
    }

    fn is_installed(self, root: &Path) -> bool {
        let (directory, revision) = self.cache_layout();
        root.join(directory).join(revision).is_dir()
    }

    fn repository(self) -> &'static str {
        match self {
            Self::Haas3072 | Self::Haas1500 => "ooookao/haas-ja-v1.0-coreml",
            Self::Silero => "FluidInference/silero-vad-coreml",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Haas3072 => "haas-ja-v1.0-coreml-3072",
            Self::Haas1500 => "haas-ja-v1.0-coreml-1500",
            Self::Silero => "silero-vad-v6.2.1-coreml-256ms",
        }
    }

    // These directory names and revisions follow the pinned Mozu Audio SDK's cache layout.
    fn cache_layout(self) -> (&'static str, &'static str) {
        match self {
            Self::Haas3072 => (
                "haas-ja-v1.0-int8",
                "e2f7a376a517494ba3903fb42bd70c89883c83ca",
            ),
            Self::Haas1500 => (
                "haas-ja-v1.0-int8-1500",
                "e54549fdd81dadabd9776d51fb6f1b3a7180663f",
            ),
            Self::Silero => (
                "silero-vad-v6.2.1",
                "b419383c55c110e2c9271fa6ee0ea83d03c70d96",
            ),
        }
    }

    fn download(self, root: PathBuf, cancellation: &Cancellation) -> Result<()> {
        let cache = ModelCache::new(root);
        let variant = match self {
            Self::Haas3072 => HaasVariant::Frames3072,
            Self::Haas1500 => HaasVariant::Frames1500,
            Self::Silero => return cache.prepare_silero(cancellation, |_| {}).map(|_| ()),
        };
        cache
            .prepare_haas_variant(variant, cancellation, |_| {})
            .map(|_| ())
    }

    fn delete(self, root: &Path) -> Result<()> {
        let (directory, revision) = self.cache_layout();
        let parent = root.join(directory);
        require_directory(&parent)?;
        let lock_path = parent.join("download.lock");
        let lock = at(
            &lock_path,
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock_path),
        )?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(Error::Invalid(
                    "別の処理がこのモデルを使用しています。".into(),
                ));
            }
            Err(TryLockError::Error(source)) => {
                return Err(Error::Io {
                    path: lock_path,
                    source,
                });
            }
        }
        let target = parent.join(revision);
        require_directory(&target)?;
        at(&target, fs::remove_dir_all(&target))
    }
}

pub(crate) fn cached_for_streaming(root: &Path, variant: HaasVariant) -> bool {
    Model::encoder(variant).is_installed(root) && Model::Silero.is_installed(root)
}

fn at<T>(path: &Path, result: std::io::Result<T>) -> Result<T> {
    result.map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn require_directory(path: &Path) -> Result<()> {
    let metadata = at(path, fs::symlink_metadata(path))?;
    if !metadata.file_type().is_dir() {
        return Err(Error::Invalid(format!(
            "モデルフォルダを確認できませんでした。{}",
            path.display()
        )));
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModelAction {
    Download,
    Delete,
}

struct ModelJob {
    model: Model,
    action: ModelAction,
    cancellation: Cancellation,
    _update_activity: crate::updater::Activity,
}

pub(crate) struct ModelManager {
    pub(crate) root: Option<PathBuf>,
    job: Option<ModelJob>,
    confirm_delete: Option<Model>,
    error: Option<String>,
}

impl ModelManager {
    pub(crate) fn load() -> Self {
        let override_path = std::env::var_os("MOZU_CACHE").map(PathBuf::from);
        let root = override_path
            .or_else(|| {
                NSUserDefaults::standardUserDefaults()
                    .stringForKey(ns_string!("ModelCachePath"))
                    .map(|value| PathBuf::from(value.to_string()))
            })
            .or_else(|| std::env::home_dir().map(|home| home.join("Library/Caches/Mozu/Models")));
        Self {
            root,
            job: None,
            confirm_delete: None,
            error: None,
        }
    }

    pub(crate) fn encoder_is_installed(&self, variant: HaasVariant) -> bool {
        self.root
            .as_ref()
            .is_some_and(|root| Model::encoder(variant).is_installed(root))
    }

    pub(crate) fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    pub(crate) fn cancel(&self) {
        if let Some(job) = &self.job {
            job.cancellation.cancel();
        }
    }

    fn operate(
        demo: &mut MozuApp,
        model: Model,
        action: ModelAction,
        window: &Window,
        cx: &mut Context<MozuApp>,
    ) {
        if demo.run.is_some() || demo.models.is_busy() || crate::updater::restart_requested(cx) {
            return;
        }
        let Some(root) = demo.models.root.clone() else {
            return;
        };
        if action == ModelAction::Delete {
            demo.invalidate_recognition_models();
        }
        let cancellation = Cancellation::new();
        demo.models.job = Some(ModelJob {
            model,
            action,
            cancellation: cancellation.clone(),
            _update_activity: crate::updater::protect(cx),
        });
        demo.models.confirm_delete = None;
        demo.models.error = None;
        demo.key_press = None;
        let work = cx.background_executor().spawn(async move {
            match action {
                ModelAction::Delete => model.delete(&root),
                ModelAction::Download => model.download(root, &cancellation),
            }
        });
        cx.spawn_in(window, async move |view, cx| {
            let outcome = work.await;
            let _ = view.update_in(cx, |demo, _, cx| {
                demo.models.job = None;
                if outcome.is_ok() && action == ModelAction::Download {
                    demo.warm_recognition();
                }
                if let Err(error) = outcome
                    && !matches!(error, Error::Cancelled)
                {
                    demo.models.error = Some(error.to_string());
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn render(
        &self,
        active: bool,
        theme: &MozuTheme,
        window: &Window,
        cx: &Context<MozuApp>,
    ) -> impl IntoElement {
        use mozu_ui::ui::item::{Item, ItemGroup, ItemSize};
        let busy = active || self.is_busy();
        let folder = self.root.as_ref().filter(|root| root.is_dir()).cloned();
        let path = self
            .root
            .as_ref()
            .map(|root| root.display().to_string())
            .unwrap_or_else(|| "保存先を取得できませんでした".into());
        let rows = Model::ALL.into_iter().map(|model| {
            let (_, revision) = model.cache_layout();
            let repository = model.repository();
            let url = format!("https://huggingface.co/{repository}/tree/{revision}");
            let name = model.display_name();
            let detail = match model {
                Model::Haas3072 => "30.72 秒エンコーダー",
                Model::Haas1500 => "15 秒エンコーダー",
                Model::Silero => "音声区間検出",
            };
            let installed = self
                .root
                .as_ref()
                .is_some_and(|root| model.is_installed(root));
            let job = self.job.as_ref().filter(|job| job.model == model);
            let status = match job {
                Some(job) if job.action == ModelAction::Delete => "削除中",
                Some(_) => "ダウンロード中",
                None if installed => "保存済み",
                None => "未ダウンロード",
            };
            let confirm_delete = self.confirm_delete == Some(model);
            let action = if job.is_some_and(|job| job.action == ModelAction::Download) {
                Button::new(("cancel-download", model as usize))
                    .icon_start(Icon::new(IconName::X, rems(1.0)))
                    .accessibility_label(format!("{name}のダウンロードをキャンセル"))
                    .accessibility_label("ダウンロードをキャンセル")
                    .tooltip("ダウンロードをキャンセル")
                    .on_click(cx.listener(|demo, _, _, _| demo.models.cancel()))
            } else if installed && !confirm_delete {
                Button::new(("ask-remove-model", model as usize))
                    .icon_start(Icon::new(IconName::Trash, rems(1.0)))
                    .accessibility_label(format!("{name}を削除"))
                    .accessibility_label("削除")
                    .tooltip("削除")
                    .disabled(busy)
                    .on_click(cx.listener(move |demo, _, _, cx| {
                        demo.models.confirm_delete = Some(model);
                        cx.notify();
                    }))
            } else {
                let (icon, label) = if confirm_delete {
                    (IconName::Trash, "削除を確定")
                } else {
                    (IconName::ArrowDown, "ダウンロード")
                };
                Button::new(("operate-model", model as usize))
                    .icon_start(Icon::new(icon, rems(1.0)))
                    .accessibility_label(format!("{name}の{label}"))
                    .accessibility_label(label)
                    .tooltip(label)
                    .disabled(busy || self.root.is_none())
                    .on_click(cx.listener(move |demo, _, window, cx| {
                        let action = if confirm_delete {
                            ModelAction::Delete
                        } else {
                            ModelAction::Download
                        };
                        Self::operate(demo, model, action, window, cx);
                    }))
            }
            .size(ButtonSize::IconSm)
            .variant(if confirm_delete {
                ButtonVariant::Destructive
            } else {
                ButtonVariant::Outline
            });
            Item::new(("model-row", model as usize), name)
                .size(ItemSize::Xs)
                .description(format!("Core ML · {detail} · {status}"))
                .actions(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Button::new(("model-hugging-face", model as usize))
                                .size(ButtonSize::IconSm)
                                .variant(ButtonVariant::Ghost)
                                .icon_end(BrandIcon::new(BrandIconName::HuggingFace, rems(1.125)))
                                .accessibility_label(format!("{name}をHugging Faceで開く"))
                                .accessibility_label("Hugging Faceで開く")
                                .tooltip("Hugging Faceで開く")
                                .on_click(move |_, _, cx| cx.open_url(&url)),
                        )
                        .children(confirm_delete.then(|| {
                            Button::new(("keep-model", model as usize))
                                .label("キャンセル")
                                .size(ButtonSize::Sm)
                                .variant(ButtonVariant::Ghost)
                                .on_click(cx.listener(|demo, _, _, cx| {
                                    demo.models.confirm_delete = None;
                                    cx.notify();
                                }))
                        }))
                        .child(action),
                )
        });
        let viewport = kinu_ui::ScrollAreaViewport::new().px_6().py_3().child(
            div()
                .max_w(px(520.0))
                .mx_auto()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    Item::new("model-storage", "保存先")
                        .size(ItemSize::Xs)
                        .description(path)
                        .actions(div().children(folder.map(|path| {
                            div().flex().child(
                                Button::new("reveal-model-folder")
                                    .label("Finderで表示")
                                    .icon_start(Icon::new(IconName::FolderOpen, rems(1.0)))
                                    .size(ButtonSize::Sm)
                                    .variant(ButtonVariant::Ghost)
                                    .on_click(move |_, _, cx| cx.reveal_path(&path)),
                            )
                        }))),
                )
                .children(
                    self.error
                        .clone()
                        .map(|error| div().text_sm().text_color(theme.destructive).child(error)),
                )
                .child(
                    rows.enumerate()
                        .fold(ItemGroup::new("model-list"), |group, (index, row)| {
                            group.separator(("model-divider", index)).item(row)
                        }),
                ),
        );
        mozu_ui::ui::scroll_area::scroll_area(
            "models-scroll",
            mozu_ui::ui::scroll_area::scroll_fade(viewport, gpui::Axis::Vertical),
            kinu_ui::ScrollAreaAxes::Vertical,
            window,
            cx,
        )
        .flex_1()
        .min_h_0()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleting_a_model_leaves_other_revisions_and_respects_download_lock() {
        let cache = tempfile::tempdir().unwrap();
        let model = Model::Silero;
        let (directory, revision) = model.cache_layout();
        let parent = cache.path().join(directory);
        let target = parent.join(revision);
        let other = parent.join("older-revision");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir(&other).unwrap();
        let lock_path = parent.join("download.lock");
        let lock = fs::File::create(&lock_path).unwrap();
        lock.try_lock().unwrap();
        assert!(model.delete(cache.path()).is_err());
        assert!(target.is_dir());
        drop(lock);

        model.delete(cache.path()).unwrap();
        assert!(!target.exists());
        assert!(other.is_dir());
        assert!(lock_path.is_file());
    }
}
