use gpui::{
    App, Context, KeyBinding, Menu, MenuItem, OsAction, PromptLevel, Window, WindowHandle, actions,
};
use gpui_base::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

use crate::{MozuApp, Page};

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

pub(crate) fn install(console: WindowHandle<MozuApp>, cx: &mut App) {
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
    let updater = match crate::updater::controller(cx) {
        Ok(updater) => updater,
        Err(error) => {
            update_error(console, error, cx);
            return;
        }
    };
    cx.spawn(async move |cx| {
        if let Err(error) = updater.check_for_updates() {
            cx.update(|cx| update_error(console, error.to_string(), cx));
        }
    })
    .detach();
}

fn update_error(console: WindowHandle<MozuApp>, error: String, cx: &mut App) {
    eprintln!("Could not check for updates: {error}");
    on_console(console, cx, move |_, window, cx| {
        drop(window.prompt(
            PromptLevel::Warning,
            "アップデートを確認できませんでした",
            Some(&error),
            &["OK"],
            cx,
        ));
    });
}
