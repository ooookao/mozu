use std::time::Duration;

use gpui::{
    App, Context, Global, KeyBinding, Menu, MenuItem, OsAction, PromptLevel, Window, WindowHandle,
    actions,
};
use gpui_base::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use semver::Version;
use serde::Deserialize;

use crate::{MozuApp, Page};

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const LATEST_RELEASE: &str = "https://api.github.com/repos/ooookao/mozu/releases/latest";

actions!(
    mozu,
    [
        About,
        CheckForUpdates,
        OpenSettings,
        Hide,
        HideOthers,
        ShowAll,
        Quit,
        Minimize,
        Zoom,
    ]
);

#[derive(Default)]
struct UpdateCheck(bool);

impl Global for UpdateCheck {}

pub(crate) fn install(console: WindowHandle<MozuApp>, cx: &mut App) {
    cx.set_global(UpdateCheck::default());
    cx.on_action(|_: &About, _| {
        let marker = MainThreadMarker::new().expect("menu actions run on the main thread");
        NSApplication::sharedApplication(marker).orderFrontStandardAboutPanel(None);
    });
    cx.on_action(move |_: &CheckForUpdates, cx| check_for_updates(console, cx));
    cx.on_action(move |_: &OpenSettings, cx| {
        on_console(console, cx, |view, window, cx| {
            window.activate_window();
            view.navigate(Page::Settings, cx);
        });
    });
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(move |_: &Minimize, cx| {
        on_console(console, cx, |_, window, _| window.minimize_window());
    });
    cx.on_action(move |_: &Zoom, cx| {
        on_console(console, cx, |_, window, _| window.zoom_window());
    });

    cx.bind_keys([
        KeyBinding::new("cmd-,", OpenSettings, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]);
    cx.set_menus([
        Menu::new("Mozu").items([
            MenuItem::action("Mozuについて", About),
            MenuItem::action("アップデートを確認…", CheckForUpdates),
            MenuItem::separator(),
            MenuItem::action("設定…", OpenSettings),
            MenuItem::separator(),
            MenuItem::action("Mozuを隠す", Hide),
            MenuItem::action("ほかを隠す", HideOthers),
            MenuItem::action("すべてを表示", ShowAll),
            MenuItem::separator(),
            MenuItem::action("Mozuを終了", Quit),
        ]),
        Menu::new("編集").items([
            MenuItem::os_action("取り消す", Undo, OsAction::Undo),
            MenuItem::os_action("やり直す", Redo, OsAction::Redo),
            MenuItem::separator(),
            MenuItem::os_action("カット", Cut, OsAction::Cut),
            MenuItem::os_action("コピー", Copy, OsAction::Copy),
            MenuItem::os_action("ペースト", Paste, OsAction::Paste),
            MenuItem::separator(),
            MenuItem::os_action("すべてを選択", SelectAll, OsAction::SelectAll),
        ]),
        Menu::new("ウインドウ").items([
            MenuItem::action("しまう", Minimize),
            MenuItem::action("拡大／縮小", Zoom),
        ]),
    ]);
}

fn on_console(
    console: WindowHandle<MozuApp>,
    cx: &mut App,
    action: impl FnOnce(&mut MozuApp, &mut Window, &mut Context<MozuApp>) + 'static,
) {
    // GPUI has the active window on its update stack while dispatching the menu action.
    cx.defer(move |cx| {
        if let Err(error) = console.update(cx, action) {
            eprintln!("Could not run action on the Mozu window: {error}");
        }
    });
}

fn check_for_updates(console: WindowHandle<MozuApp>, cx: &mut App) {
    if cx.global::<UpdateCheck>().0 {
        return;
    }
    cx.global_mut::<UpdateCheck>().0 = true;
    let check = cx
        .background_executor()
        .spawn(async { fetch_latest_release() });
    cx.spawn(async move |cx| {
        let result = check.await;
        let prompt = cx.update(|cx| {
            if let Err(error) = &result {
                eprintln!("Could not check for updates at {LATEST_RELEASE}: {error}");
            }
            let prompt = console.update(cx, |_, window, cx| match result {
                Ok(Some(update)) => {
                    let detail = format!(
                        "現在のバージョン：{CURRENT_VERSION}\n新しいバージョン：{}",
                        update.version
                    );
                    let response = window.prompt(
                        PromptLevel::Info,
                        "新しいバージョンがあります",
                        Some(&detail),
                        &["ダウンロード", "あとで"],
                        cx,
                    );
                    (response, Some(update.download_url))
                }
                Ok(None) => (
                    window.prompt(
                        PromptLevel::Info,
                        "Mozuは最新版です",
                        Some(&format!("バージョン {CURRENT_VERSION}")),
                        &["OK"],
                        cx,
                    ),
                    None,
                ),
                Err(_) => (
                    window.prompt(
                        PromptLevel::Warning,
                        "アップデートを確認できませんでした",
                        None,
                        &["OK"],
                        cx,
                    ),
                    None,
                ),
            });
            prompt.ok()
        });
        if let Some((response, url)) = prompt
            && response.await == Ok(0)
            && let Some(url) = url
        {
            cx.update(|cx| cx.open_url(&url));
        }
        cx.update(|cx| cx.global_mut::<UpdateCheck>().0 = false);
    })
    .detach();
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    state: String,
}

#[derive(Debug, PartialEq, Eq)]
struct AvailableUpdate {
    version: Version,
    download_url: String,
}

fn fetch_latest_release() -> Result<Option<AvailableUpdate>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .into();
    let mut response = agent
        .get(LATEST_RELEASE)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", format!("Mozu/{CURRENT_VERSION}"))
        .call()
        .map_err(|error| format!("GitHub release request failed: {error}"))?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Could not read GitHub release: {error}"))?;
    let release = serde_json::from_str(&body)
        .map_err(|error| format!("Invalid GitHub release response: {error}"))?;
    let current = Version::parse(CURRENT_VERSION)
        .map_err(|error| format!("Invalid installed version {CURRENT_VERSION}: {error}"))?;
    available_update(&current, release)
}

fn available_update(
    current: &Version,
    release: Release,
) -> Result<Option<AvailableUpdate>, String> {
    let tag = release.tag_name;
    let version = Version::parse(tag.strip_prefix('v').unwrap_or(&tag))
        .map_err(|error| format!("Invalid release tag {tag:?}: {error}"))?;
    if !version.pre.is_empty() || !version.cmp_precedence(current).is_gt() {
        return Ok(None);
    }

    let asset = format!("Mozu-{version}-macos-arm64.dmg");
    if !release
        .assets
        .iter()
        .any(|file| file.name == asset && file.state == "uploaded")
    {
        return Err(format!("GitHub release {tag} has no uploaded {asset}"));
    }
    Ok(Some(AvailableUpdate {
        version,
        download_url: format!(
            "{}/releases/download/{tag}/{asset}",
            env!("CARGO_PKG_REPOSITORY")
        ),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_release_versions_by_semver_and_ignores_prereleases() {
        let current = Version::new(0, 9, 0);
        for tag in ["v0.9.0", "v0.8.9", "v0.9.0+build.2", "v0.10.0-beta.1"] {
            let release = Release {
                tag_name: tag.into(),
                assets: Vec::new(),
            };
            assert_eq!(available_update(&current, release), Ok(None));
        }
        let release = serde_json::from_str(
            r#"{"tag_name":"v0.10.0","assets":[
            {"name":"Mozu-0.10.0-macos-arm64.dmg","state":"uploaded"}
        ]}"#,
        )
        .unwrap();
        let update = available_update(&current, release).unwrap().unwrap();
        assert_eq!(update.version, Version::new(0, 10, 0));
        assert_eq!(
            update.download_url,
            "https://github.com/ooookao/mozu/releases/download/v0.10.0/Mozu-0.10.0-macos-arm64.dmg"
        );
    }

    #[test]
    fn refuses_newer_release_without_an_uploaded_arm64_disk_image() {
        let release = serde_json::from_str(
            r#"{"tag_name":"v0.2.0","assets":[
            {"name":"Mozu-0.2.0-macos-arm64.zip","state":"uploaded"},
            {"name":"Mozu-0.2.0-macos-arm64.dmg","state":"open"}
        ]}"#,
        )
        .unwrap();
        assert!(available_update(&Version::new(0, 1, 0), release).is_err());
    }
}
