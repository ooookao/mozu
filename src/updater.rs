use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use async_channel::Sender;
use gpui::{App, Global};
use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSApplicationTerminateReply};
use sparkle_updater::{
    MainThreadMarker, RelaunchContinuation, SparkleUpdater, UpdateEvent, UpdaterConfig,
};

enum Signal {
    WorkFinished,
    RelaunchRequested,
    CycleEnded,
}

struct ActivityState {
    active: AtomicUsize,
    restart_requested: AtomicBool,
    retry_termination: AtomicBool,
    signals: Sender<Signal>,
}

impl ActivityState {
    fn hold(self: &Arc<Self>) -> Activity {
        self.active.fetch_add(1, Ordering::AcqRel);
        Activity {
            state: self.clone(),
        }
    }

    fn notify(&self, signal: Signal) {
        if self.signals.try_send(signal).is_err() {
            eprintln!("Could not notify the updater: application event loop is closed");
        }
    }
}

/// Keeps installation deferred until a recording, model job, or final save completes.
pub(crate) struct Activity {
    state: Arc<ActivityState>,
}

impl Drop for Activity {
    fn drop(&mut self) {
        let previous = self.state.active.fetch_sub(1, Ordering::AcqRel);
        assert!(previous > 0, "unbalanced updater activity");
        self.state.notify(Signal::WorkFinished);
    }
}

#[derive(Default)]
struct InstallQueue<T> {
    requests: Vec<T>,
}

impl<T> InstallQueue<T> {
    fn take_ready(&mut self, active: usize) -> Vec<T> {
        if active == 0 {
            std::mem::take(&mut self.requests)
        } else {
            Vec::new()
        }
    }
}

struct Updater {
    native: Result<Option<Rc<SparkleUpdater>>, String>,
    activity: Arc<ActivityState>,
    _delegate: Retained<MozuUpdaterDelegate>,
}

impl Global for Updater {}

pub(crate) fn install(cx: &mut App) {
    let (signals, receiver) = async_channel::unbounded();
    let activity = Arc::new(ActivityState {
        active: AtomicUsize::new(0),
        restart_requested: AtomicBool::new(false),
        retry_termination: AtomicBool::new(false),
        signals,
    });
    let delegate = MozuUpdaterDelegate::install(activity.clone());
    let pending = Rc::new(RefCell::new(InstallQueue {
        requests: Vec::<RelaunchContinuation>::new(),
    }));
    let relaunch_activity = activity.clone();
    let relaunch_queue = pending.clone();
    let abort_activity = activity.clone();
    let native = SparkleUpdater::new(
        MainThreadMarker::new().expect("updater starts on the application thread"),
        UpdaterConfig {
            event_callback: Some(Rc::new(move |event| observe_event(&abort_activity, event))),
            relaunch_handler: Some(Rc::new(move |_, continuation| {
                relaunch_activity
                    .restart_requested
                    .store(true, Ordering::Release);
                relaunch_queue.borrow_mut().requests.push(continuation);
                relaunch_activity.notify(Signal::RelaunchRequested);
            })),
            gentle_reminders: None,
        },
    )
    .map(|native| native.map(Rc::new))
    .map_err(|error| error.to_string());
    if let Err(error) = &native {
        eprintln!("Could not start the updater: {error}");
    }
    cx.set_global(Updater {
        native,
        activity: activity.clone(),
        _delegate: delegate,
    });
    cx.spawn(async move |_| {
        while let Ok(signal) = receiver.recv().await {
            if matches!(signal, Signal::CycleEnded) {
                pending.borrow_mut().requests.clear();
                activity.restart_requested.store(false, Ordering::Release);
                activity.retry_termination.store(false, Ordering::Release);
                continue;
            }
            let ready = pending
                .borrow_mut()
                .take_ready(activity.active.load(Ordering::Acquire));
            if !ready.is_empty() {
                activity.retry_termination.store(false, Ordering::Release);
                for continuation in ready {
                    continuation.resume(
                        MainThreadMarker::new().expect("resume updates on the application thread"),
                    );
                }
            } else if activity.active.load(Ordering::Acquire) == 0
                && activity.retry_termination.swap(false, Ordering::AcqRel)
            {
                NSApplication::sharedApplication(
                    MainThreadMarker::new().expect("updater event thread"),
                )
                .terminate(None);
            }
        }
    })
    .detach();
}

fn observe_event(activity: &ActivityState, event: UpdateEvent) {
    if matches!(
        event,
        UpdateEvent::DidFinishUpdateCycle(_) | UpdateEvent::UserDidCancelDownload
    ) {
        activity.notify(Signal::CycleEnded);
    }
    if matches!(
        event,
        UpdateEvent::WillInstallUpdate(_) | UpdateEvent::WillRelaunchApplication
    ) {
        activity.restart_requested.store(true, Ordering::Release);
    }
    if let UpdateEvent::DidAbortWithError(error) = event
        && error.no_update.is_none()
    {
        eprintln!(
            "Update failed: domain={}, code={}, message={}, reason={:?}, underlying={:?}",
            error.domain, error.code, error.message, error.failure_reason, error.underlying
        );
        activity.notify(Signal::CycleEnded);
    }
}

struct DelegateIvars {
    original: Option<Retained<ProtocolObject<dyn NSApplicationDelegate>>>,
    activity: Arc<ActivityState>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = DelegateIvars]
    struct MozuUpdaterDelegate;

    unsafe impl NSObjectProtocol for MozuUpdaterDelegate {}

    unsafe impl NSApplicationDelegate for MozuUpdaterDelegate {
        #[unsafe(method(applicationShouldTerminate:))]
        fn application_should_terminate(&self, app: &NSApplication) -> NSApplicationTerminateReply {
            let original = self.ivars().original.as_ref().map(|delegate| {
                let supported: bool = unsafe { msg_send![&**delegate, respondsToSelector: sel!(applicationShouldTerminate:)] };
                if supported {
                    unsafe { msg_send![&**delegate, applicationShouldTerminate: app] }
                } else {
                    NSApplicationTerminateReply::TerminateNow
                }
            }).unwrap_or(NSApplicationTerminateReply::TerminateNow);
            let reply = termination_reply(original, &self.ivars().activity);
            if original == NSApplicationTerminateReply::TerminateNow
                && reply == NSApplicationTerminateReply::TerminateCancel
            {
                self.ivars().activity.retry_termination.store(true, Ordering::Release);
            }
            reply
        }
    }

    impl MozuUpdaterDelegate {
        #[unsafe(method(respondsToSelector:))]
        fn responds_to_selector(&self, selector: Sel) -> bool {
            let supported: bool = unsafe { msg_send![super(self), respondsToSelector: selector] };
            supported || self.ivars().original.as_ref().is_some_and(|delegate| unsafe {
                msg_send![&**delegate, respondsToSelector: selector]
            })
        }

        #[unsafe(method(forwardingTargetForSelector:))]
        fn forwarding_target(&self, _selector: Sel) -> *mut AnyObject {
            self.ivars().original.as_ref().map_or(std::ptr::null_mut(), |delegate| {
                Retained::as_ptr(delegate).cast_mut().cast()
            })
        }
    }
);

impl MozuUpdaterDelegate {
    fn install(activity: Arc<ActivityState>) -> Retained<Self> {
        let marker = MainThreadMarker::new().expect("application delegate runs on the main thread");
        let app = NSApplication::sharedApplication(marker);
        let this = Self::alloc(marker).set_ivars(DelegateIvars {
            original: app.delegate(),
            activity,
        });
        let delegate: Retained<Self> = unsafe { msg_send![super(this), init] };
        app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        delegate
    }
}

/// Cancel an installer's quit while work is active; retry once work finishes.
/// AppKit's TerminateLater modal loop cannot drive GPUI's main dispatch queue.
fn termination_reply(
    original: NSApplicationTerminateReply,
    activity: &ActivityState,
) -> NSApplicationTerminateReply {
    if original == NSApplicationTerminateReply::TerminateNow
        && activity.restart_requested.load(Ordering::Acquire)
        && activity.active.load(Ordering::Acquire) > 0
    {
        NSApplicationTerminateReply::TerminateCancel
    } else {
        original
    }
}

pub(crate) fn protect(cx: &App) -> Activity {
    cx.global::<Updater>().activity.hold()
}

pub(crate) fn restart_requested(cx: &App) -> bool {
    cx.global::<Updater>()
        .activity
        .restart_requested
        .load(Ordering::Acquire)
}

pub(crate) fn controller(cx: &App) -> Result<Rc<SparkleUpdater>, String> {
    cx.global::<Updater>()
        .native
        .as_ref()
        .map_err(Clone::clone)?
        .as_ref()
        .cloned()
        .ok_or_else(|| "アプリケーションからMozuを起動してください".to_owned())
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod native_test {
    use super::*;
    use std::{
        io::Write,
        path::{Path, PathBuf},
        time::Duration,
    };

    pub(crate) fn append(trace: &Path, message: &str) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(trace)
            .unwrap();
        writeln!(file, "{message}").unwrap();
    }

    pub(crate) fn verify_termination(cx: &mut App, trace: PathBuf) {
        assert!(
            controller(cx).is_ok(),
            "Sparkle must initialize in the fixture bundle"
        );
        assert!(!restart_requested(cx));
        let recording = protect(cx);
        let save = protect(cx);
        let activity = cx.global::<Updater>().activity.clone();
        let shutdown_trace = trace.clone();
        cx.on_app_quit(move |_| {
            append(&shutdown_trace, "shutdown");
            async {}
        })
        .detach();
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            append(&trace, "started");
            observe_event(
                &activity,
                UpdateEvent::WillInstallUpdate(sparkle_updater::events::VersionInfo {
                    version: "9".into(),
                }),
            );
            std::thread::spawn(move || {
                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while !activity.retry_termination.load(Ordering::Acquire) {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "termination callback did not defer"
                    );
                    std::thread::sleep(Duration::from_millis(5));
                }
                append(&trace, "deferred");
                drop(recording);
                append(&trace, "recording finished");
                std::thread::sleep(Duration::from_millis(150));
                append(&trace, "save finished");
                drop(save);
            });
            NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).terminate(None);
        })
        .detach();
    }

    pub(crate) fn verify_cancellation(cx: &mut App, trace: PathBuf) {
        assert!(controller(cx).is_ok());
        let work = protect(cx);
        let activity = cx.global::<Updater>().activity.clone();
        let shutdown_trace = trace.clone();
        cx.on_app_quit(move |_| {
            append(&shutdown_trace, "shutdown");
            async {}
        })
        .detach();
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            observe_event(
                &activity,
                UpdateEvent::WillInstallUpdate(sparkle_updater::events::VersionInfo {
                    version: "9".into(),
                }),
            );
            NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).terminate(None);
            assert!(activity.retry_termination.load(Ordering::Acquire));
            append(&trace, "deferred");
            observe_event(
                &activity,
                UpdateEvent::DidFinishUpdateCycle(sparkle_updater::events::UpdateCycleInfo {
                    update_check: "user-initiated".into(),
                    error: None,
                }),
            );
            while activity.restart_requested.load(Ordering::Acquire) {
                cx.background_executor()
                    .timer(Duration::from_millis(5))
                    .await;
            }
            assert!(!activity.retry_termination.load(Ordering::Acquire));
            drop(work);
            cx.background_executor()
                .timer(Duration::from_millis(50))
                .await;
            append(&trace, "still running after cancellation");
            cx.update(|cx| cx.quit());
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn activity() -> (Arc<ActivityState>, async_channel::Receiver<Signal>) {
        let (signals, receiver) = async_channel::unbounded();
        (
            Arc::new(ActivityState {
                active: AtomicUsize::new(0),
                restart_requested: AtomicBool::new(false),
                retry_termination: AtomicBool::new(false),
                signals,
            }),
            receiver,
        )
    }

    #[test]
    fn installation_waits_for_recording_and_clipboard_restoration() {
        let (activity, _receiver) = activity();
        let recording = activity.hold();
        let clipboard = activity.hold();
        let mut queue = InstallQueue {
            requests: vec!["install"],
        };
        assert!(
            queue
                .take_ready(activity.active.load(Ordering::Acquire))
                .is_empty()
        );
        drop(recording);
        assert!(
            queue
                .take_ready(activity.active.load(Ordering::Acquire))
                .is_empty()
        );
        drop(clipboard);
        assert_eq!(
            queue.take_ready(activity.active.load(Ordering::Acquire)),
            vec!["install"]
        );
        assert!(queue.take_ready(0).is_empty());
    }

    #[test]
    fn final_database_write_on_another_thread_blocks_installation() {
        let (activity, receiver) = activity();
        let save = activity.hold();
        let mut queue = InstallQueue {
            requests: vec!["install"],
        };
        assert!(
            queue
                .take_ready(activity.active.load(Ordering::Acquire))
                .is_empty()
        );
        std::thread::spawn(move || drop(save)).join().unwrap();
        assert!(matches!(receiver.try_recv(), Ok(Signal::WorkFinished)));
        assert_eq!(
            queue.take_ready(activity.active.load(Ordering::Acquire)),
            vec!["install"]
        );
    }

    #[test]
    fn updater_quit_without_a_relaunch_callback_waits_for_active_work() {
        let (activity, _receiver) = activity();
        let recording = activity.hold();
        activity.restart_requested.store(true, Ordering::Release);
        assert_eq!(
            termination_reply(NSApplicationTerminateReply::TerminateNow, &activity),
            NSApplicationTerminateReply::TerminateCancel
        );
        drop(recording);
        assert_eq!(
            termination_reply(NSApplicationTerminateReply::TerminateNow, &activity),
            NSApplicationTerminateReply::TerminateNow
        );
    }

    #[test]
    fn host_quit_decisions_are_preserved() {
        let (activity, _receiver) = activity();
        let _recording = activity.hold();
        assert_eq!(
            termination_reply(NSApplicationTerminateReply::TerminateNow, &activity),
            NSApplicationTerminateReply::TerminateNow
        );
        activity.restart_requested.store(true, Ordering::Release);
        for reply in [
            NSApplicationTerminateReply::TerminateCancel,
            NSApplicationTerminateReply::TerminateLater,
        ] {
            assert_eq!(termination_reply(reply, &activity), reply);
        }
    }
}
