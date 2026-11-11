#[path = "../src/updater.rs"]
#[allow(dead_code)]
mod updater;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_foundation::{NSBundle, NSString, NSUserDefaults, ns_string};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn run(command: &mut Command) {
    assert!(
        command.status().unwrap().success(),
        "command failed: {command:?}"
    );
}

fn bundle_string(key: &NSString) -> String {
    NSBundle::mainBundle()
        .objectForInfoDictionaryKey(key)
        .expect("fixture metadata")
        .downcast::<NSString>()
        .expect("fixture string metadata")
        .to_string()
}

fn trace_path(identifier: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{identifier}.trace"))
}

fn run_fixture(setup: impl FnOnce(&mut gpui::App, PathBuf, String) + 'static) {
    let identifier = bundle_string(ns_string!("CFBundleIdentifier"));
    let trace = trace_path(&identifier);
    let build = bundle_string(ns_string!("CFBundleVersion"));
    gpui_platform::application().run(move |cx| {
        NSApplication::sharedApplication(MainThreadMarker::new().unwrap())
            .setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        updater::install(cx);
        setup(cx, trace, build);
    });
}

fn installation_fixture() {
    run_fixture(|cx, trace, build| {
        assert!(updater::controller(cx).is_ok());
        updater::native_test::append(&trace, &format!("started {build}"));
        let shutdown_trace = trace.clone();
        let shutdown_build = build.clone();
        cx.on_app_quit(move |_| {
            updater::native_test::append(&shutdown_trace, &format!("shutdown {shutdown_build}"));
            async {}
        })
        .detach();
        if build == "9" {
            cx.spawn(async move |cx| {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                cx.update(|cx| cx.quit());
            })
            .detach();
        }
    });
}

fn guard_fixture() {
    run_fixture(|cx, trace, _| updater::native_test::verify_termination(cx, trace));
}

fn cancellation_fixture() {
    run_fixture(|cx, trace, _| updater::native_test::verify_cancellation(cx, trace));
}

fn run_native_case(executable: &str, expected_trace: &str) {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join("MozuUpdaterNativeTest.app");
    let contents = app.join("Contents");
    std::fs::create_dir_all(contents.join("MacOS")).unwrap();
    std::fs::create_dir_all(contents.join("Frameworks")).unwrap();
    std::fs::copy(
        std::env::current_exe().unwrap(),
        contents.join("MacOS").join(executable),
    )
    .unwrap();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::copy(repo.join("Info.plist"), contents.join("Info.plist")).unwrap();
    run(Command::new("/usr/bin/ditto")
        .arg(repo.join("target/sparkle/Sparkle.framework"))
        .arg(contents.join("Frameworks/Sparkle.framework")));
    let identifier = format!("io.github.ooookao.mozu.{executable}.{}", std::process::id());
    for setting in [
        format!("Set CFBundleExecutable {executable}"),
        format!(
            "Add CFBundleShortVersionString string {}",
            env!("CARGO_PKG_VERSION")
        ),
        format!("Set CFBundleIdentifier {identifier}"),
        "Set SUEnableAutomaticChecks false".to_owned(),
        "Set SUAutomaticallyUpdate false".to_owned(),
        "Add LSUIElement bool true".to_owned(),
    ] {
        run(Command::new("/usr/libexec/PlistBuddy")
            .arg("-c")
            .arg(setting)
            .arg(contents.join("Info.plist")));
    }
    run(Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-"])
        .arg(&app));
    let trace = trace_path(&identifier);
    let log_path = root.path().join("native.log");
    let log = std::fs::File::create(&log_path).unwrap();
    let mut process = Command::new(contents.join("MacOS").join(executable))
        .env_remove("DYLD_FRAMEWORK_PATH")
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = process.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            process.kill().unwrap();
            process.wait().unwrap();
            panic!(
                "native termination timed out: {}; trace={:?}",
                std::fs::read_to_string(&log_path).unwrap(),
                std::fs::read_to_string(&trace)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(
        status.success(),
        "native test failed: {}",
        std::fs::read_to_string(&log_path).unwrap()
    );
    let observed = std::fs::read_to_string(&trace).unwrap();
    assert_eq!(observed, expected_trace);
    std::fs::remove_file(trace).unwrap();
    NSUserDefaults::standardUserDefaults()
        .removePersistentDomainForName(&NSString::from_str(&identifier));
    println!("PASS: native updater lifecycle: {executable}");
}

fn main() {
    match std::env::current_exe()
        .unwrap()
        .file_name()
        .and_then(|name| name.to_str())
    {
        Some("updater-native") => guard_fixture(),
        Some("updater-native-cancel") => cancellation_fixture(),
        Some("updater-install") => installation_fixture(),
        _ => {
            run_native_case(
                "updater-native",
                "started\ndeferred\nrecording finished\nsave finished\nshutdown\n",
            );
            run_native_case(
                "updater-native-cancel",
                "deferred\nstill running after cancellation\nshutdown\n",
            );
        }
    }
}
