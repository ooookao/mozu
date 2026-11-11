use std::{cell::Cell, ptr::NonNull, rc::Rc};

use async_channel::Sender;
use block2::RcBlock;
use core_foundation::{
    base::TCFType,
    boolean::CFBoolean,
    dictionary::{CFDictionary, CFDictionaryRef},
    string::{CFString, CFStringRef},
};
use core_graphics::{
    event::{CGEvent, CGEventFlags, CGEventTapLocation},
    event_source::{CGEventSource, CGEventSourceStateID},
};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{
    NSEvent, NSEventMask, NSEventModifierFlags, NSEventType, NSPasteboard, NSPasteboardItem,
    NSPasteboardTypeString, NSPasteboardWriting,
};
use objc2_foundation::{NSArray, NSString, NSUserDefaults, ns_string};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn IsSecureEventInputEnabled() -> u8;
}

pub fn is_trusted() -> bool {
    unsafe { AXIsProcessTrusted() != 0 }
}

pub fn request_accessibility_permission() {
    // AppKit presents the request asynchronously. Activation rechecks the permission.
    let prompt = unsafe { CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt) };
    let options = CFDictionary::from_CFType_pairs(&[(prompt, CFBoolean::true_value())]);
    unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };
}

pub fn request_accessibility_permission_on_first_launch() {
    let defaults = NSUserDefaults::standardUserDefaults();
    let prompted = ns_string!("DidRequestDictationAccessibility");
    if defaults.boolForKey(prompted) {
        return;
    }
    defaults.setBool_forKey(true, prompted);
    if !is_trusted() {
        request_accessibility_permission();
    }
}

#[derive(Clone, Copy)]
pub enum Key {
    Down,
    Up,
    Other,
    Escape,
}

pub struct KeyMonitor {
    monitors: Vec<Retained<AnyObject>>,
}

impl KeyMonitor {
    pub fn install(sender: Sender<Key>) -> Option<Self> {
        let mask = NSEventMask::FlagsChanged | NSEventMask::KeyDown;
        let pressed = Rc::new(Cell::new(false));
        let handler = move |event: NonNull<NSEvent>| {
            // AppKit owns the event for the duration of this callback.
            let event = unsafe { event.as_ref() };
            let event_type = event.r#type();
            let key = if event_type == NSEventType::FlagsChanged && event.keyCode() == 63 {
                let down = event
                    .modifierFlags()
                    .contains(NSEventModifierFlags::Function);
                if pressed.replace(down) == down {
                    return;
                }
                if down { Key::Down } else { Key::Up }
            } else if event.r#type() == NSEventType::KeyDown && event.keyCode() == 53 {
                Key::Escape
            } else if pressed.get() {
                Key::Other
            } else {
                return;
            };
            let _ = sender.try_send(key);
        };
        let global_handler = handler.clone();
        let global = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
            mask,
            &RcBlock::new(global_handler),
        );
        let local = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                mask,
                &RcBlock::new(move |event: NonNull<NSEvent>| {
                    handler(event);
                    event.as_ptr()
                }),
            )
        };
        let monitor = Self {
            monitors: global.into_iter().chain(local).collect(),
        };
        (monitor.monitors.len() == 2).then_some(monitor)
    }
}

impl Drop for KeyMonitor {
    fn drop(&mut self) {
        for monitor in &self.monitors {
            unsafe { NSEvent::removeMonitor(monitor) };
        }
    }
}

trait PastePlatform {
    type Snapshot;
    type Shortcut;
    type Restore;

    fn check_access(&self) -> Result<(), &'static str>;
    fn prepare_shortcut(&self) -> Result<Self::Shortcut, &'static str>;
    fn snapshot(&self) -> Result<Self::Snapshot, &'static str>;
    fn clipboard_unchanged(&self, snapshot: &Self::Snapshot) -> bool;
    fn write_text(
        &self,
        text: &str,
        snapshot: Self::Snapshot,
    ) -> Result<Self::Restore, &'static str>;
    fn post_shortcut(&self, shortcut: Self::Shortcut);
}

/// Paste into the input selected when the shortcut is dispatched.
pub fn paste_text(text: &str) -> Result<ClipboardRestore, &'static str> {
    paste_with(&NativePaste, text)
}

fn paste_with<P: PastePlatform>(platform: &P, text: &str) -> Result<P::Restore, &'static str> {
    platform.check_access()?;
    let shortcut = platform.prepare_shortcut()?;
    let snapshot = platform.snapshot()?;
    platform.check_access()?;
    if !platform.clipboard_unchanged(&snapshot) {
        return Err("クリップボードが変わったため、貼り付けを見送りました");
    }
    let restore = platform.write_text(text, snapshot)?;
    platform.post_shortcut(shortcut);
    Ok(restore)
}

struct NativePaste;

struct ClipboardSnapshot {
    change_count: isize,
    items: Vec<ClipboardItem>,
}

impl PastePlatform for NativePaste {
    type Snapshot = ClipboardSnapshot;
    type Shortcut = [CGEvent; 2];
    type Restore = ClipboardRestore;

    fn check_access(&self) -> Result<(), &'static str> {
        if !is_trusted() {
            return Err("設定でアクセシビリティへのアクセスを許可してください");
        }
        if unsafe { IsSecureEventInputEnabled() } != 0 {
            return Err("セキュア入力が有効なため、貼り付けできませんでした");
        }
        Ok(())
    }

    fn prepare_shortcut(&self) -> Result<Self::Shortcut, &'static str> {
        let source = CGEventSource::new(CGEventSourceStateID::Private)
            .map_err(|_| "貼り付けキーを作成できませんでした")?;
        let down = CGEvent::new_keyboard_event(source.clone(), 9, true)
            .map_err(|_| "貼り付けキーを作成できませんでした")?;
        let up = CGEvent::new_keyboard_event(source, 9, false)
            .map_err(|_| "貼り付けキーを作成できませんでした")?;
        for event in [&down, &up] {
            event.set_flags(CGEventFlags::CGEventFlagCommand);
        }
        Ok([down, up])
    }

    fn snapshot(&self) -> Result<Self::Snapshot, &'static str> {
        let board = NSPasteboard::generalPasteboard();
        let change_count = board.changeCount();
        Ok(ClipboardSnapshot {
            change_count,
            items: snapshot(&board)?,
        })
    }

    fn clipboard_unchanged(&self, snapshot: &Self::Snapshot) -> bool {
        NSPasteboard::generalPasteboard().changeCount() == snapshot.change_count
    }

    fn write_text(
        &self,
        text: &str,
        snapshot: Self::Snapshot,
    ) -> Result<Self::Restore, &'static str> {
        let board = NSPasteboard::generalPasteboard();
        board.clearContents();
        if !board.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString }) {
            restore(&board, &snapshot.items);
            return Err("クリップボードに書き込めませんでした");
        }
        Ok(ClipboardRestore {
            change_count: board.changeCount(),
            items: snapshot.items,
        })
    }

    fn post_shortcut(&self, shortcut: Self::Shortcut) {
        for event in shortcut {
            event.post(CGEventTapLocation::HID);
        }
    }
}

type ClipboardItem = Retained<ProtocolObject<dyn NSPasteboardWriting>>;

fn snapshot(board: &NSPasteboard) -> Result<Vec<ClipboardItem>, &'static str> {
    let Some(items) = board.pasteboardItems() else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|item| {
            let copy = NSPasteboardItem::new();
            for kind in item.types().iter() {
                let data = item
                    .dataForType(&kind)
                    .ok_or("クリップボードの内容を保護できませんでした")?;
                if !copy.setData_forType(&data, &kind) {
                    return Err("クリップボードの内容を保護できませんでした");
                }
            }
            Ok(ProtocolObject::from_retained(copy))
        })
        .collect()
}

fn restore(board: &NSPasteboard, items: &[ClipboardItem]) {
    board.clearContents();
    if !items.is_empty() && !board.writeObjects(&NSArray::from_retained_slice(items)) {
        eprintln!("Could not restore the clipboard");
    }
}

pub struct ClipboardRestore {
    change_count: isize,
    items: Vec<ClipboardItem>,
}

impl ClipboardRestore {
    pub fn restore(self) {
        let board = NSPasteboard::generalPasteboard();
        if board.changeCount() == self.change_count {
            restore(&board, &self.items);
        }
    }
}

#[cfg(test)]
mod paste_tests {
    use super::*;
    use std::cell::RefCell;

    struct Desktop {
        window: Cell<u32>,
        clipboard: RefCell<String>,
        sent: RefCell<Vec<(u32, String)>>,
        permission: Result<(), &'static str>,
        change_count: Cell<isize>,
        switch_during_snapshot: Option<u32>,
        external_clipboard: Option<String>,
    }

    impl Desktop {
        fn new(
            permission: Result<(), &'static str>,
            switch_during_snapshot: Option<u32>,
            external_clipboard: Option<String>,
        ) -> Self {
            Self {
                window: Cell::new(1),
                clipboard: RefCell::new("previous clipboard".into()),
                sent: RefCell::new(Vec::new()),
                permission,
                change_count: Cell::new(1),
                switch_during_snapshot,
                external_clipboard,
            }
        }
    }

    impl PastePlatform for Desktop {
        type Snapshot = (isize, String);
        type Shortcut = ();
        type Restore = String;

        fn check_access(&self) -> Result<(), &'static str> {
            self.permission
        }
        fn prepare_shortcut(&self) -> Result<(), &'static str> {
            Ok(())
        }
        fn snapshot(&self) -> Result<Self::Snapshot, &'static str> {
            let snapshot = (self.change_count.get(), self.clipboard.borrow().clone());
            if let Some(window) = self.switch_during_snapshot {
                self.window.set(window);
            }
            if let Some(text) = &self.external_clipboard {
                *self.clipboard.borrow_mut() = text.clone();
                self.change_count.set(self.change_count.get() + 1);
            }
            Ok(snapshot)
        }
        fn clipboard_unchanged(&self, snapshot: &Self::Snapshot) -> bool {
            self.change_count.get() == snapshot.0
        }
        fn write_text(&self, text: &str, snapshot: Self::Snapshot) -> Result<String, &'static str> {
            *self.clipboard.borrow_mut() = text.into();
            self.change_count.set(self.change_count.get() + 1);
            Ok(snapshot.1)
        }
        fn post_shortcut(&self, _: ()) {
            self.sent
                .borrow_mut()
                .push((self.window.get(), self.clipboard.borrow().clone()));
        }
    }

    #[test]
    fn completion_pastes_into_the_current_window_after_recording_elsewhere() {
        let desktop = Desktop::new(Ok(()), None, None);
        assert_eq!(desktop.window.get(), 1);
        desktop.window.set(2);
        let previous = paste_with(&desktop, "test transcript").unwrap();
        assert_eq!(*desktop.sent.borrow(), vec![(2, "test transcript".into())]);
        assert_eq!(previous, "previous clipboard");
    }

    #[test]
    fn window_switch_during_preparation_uses_the_window_at_dispatch() {
        let desktop = Desktop::new(Ok(()), Some(2), None);
        paste_with(&desktop, "test transcript").unwrap();
        assert_eq!(*desktop.sent.borrow(), vec![(2, "test transcript".into())]);
    }

    #[test]
    fn denied_access_preserves_clipboard_and_sends_no_keys() {
        for error in ["permission revoked", "secure input"] {
            let desktop = Desktop::new(Err(error), None, None);
            assert_eq!(paste_with(&desktop, "test transcript"), Err(error));
            assert!(desktop.sent.borrow().is_empty());
            assert_eq!(*desktop.clipboard.borrow(), "previous clipboard");
        }
    }

    #[test]
    fn concurrent_clipboard_update_is_neither_overwritten_nor_pasted() {
        let desktop = Desktop::new(Ok(()), None, Some("new clipboard".into()));
        assert_eq!(
            paste_with(&desktop, "test transcript"),
            Err("クリップボードが変わったため、貼り付けを見送りました")
        );
        assert!(desktop.sent.borrow().is_empty());
        assert_eq!(*desktop.clipboard.borrow(), "new clipboard");
    }
}
