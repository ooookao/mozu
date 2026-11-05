use std::{cell::Cell, ffi::c_void, ptr::NonNull, rc::Rc};

use async_channel::Sender;
use block2::RcBlock;
use core_foundation::{
    base::{CFType, CFTypeRef, TCFType},
    boolean::CFBoolean,
    dictionary::{CFDictionary, CFDictionaryRef},
    string::{CFString, CFStringRef},
};
use core_graphics::{
    event::{CGEvent, CGEventFlags},
    event_source::{CGEventSource, CGEventSourceStateID},
};
use objc2::{
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{
    NSEvent, NSEventMask, NSEventModifierFlags, NSEventType, NSPasteboard, NSPasteboardItem,
    NSPasteboardTypeString, NSPasteboardWriting, NSWorkspace,
};
use objc2_foundation::{NSArray, NSString, NSUserDefaults, ns_string};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXUIElementCreateSystemWide() -> CFTypeRef;
    fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
    fn AXUIElementCopyAttributeValue(
        element: CFTypeRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    fn AXUIElementGetPid(element: CFTypeRef, pid: *mut i32) -> i32;
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
    interaction_epoch: Rc<Cell<u64>>,
}

impl KeyMonitor {
    pub fn install(sender: Sender<Key>) -> Option<Self> {
        let mouse_down =
            NSEventMask::LeftMouseDown | NSEventMask::RightMouseDown | NSEventMask::OtherMouseDown;
        let mask = NSEventMask::FlagsChanged | NSEventMask::KeyDown | mouse_down;
        let pressed = Rc::new(Cell::new(false));
        let interaction_epoch = Rc::new(Cell::new(0u64));
        let event_epoch = interaction_epoch.clone();
        let handler = move |event: NonNull<NSEvent>| {
            // AppKit owns the event for the duration of this callback.
            let event = unsafe { event.as_ref() };
            let event_type = event.r#type();
            if event_type != NSEventType::FlagsChanged {
                event_epoch.set(event_epoch.get().wrapping_add(1));
                if event_type != NSEventType::KeyDown {
                    return;
                }
            }
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
            interaction_epoch,
        };
        (monitor.monitors.len() == 2).then_some(monitor)
    }

    pub fn capture_target(&self) -> Option<PasteTarget> {
        if !is_trusted() || unsafe { IsSecureEventInputEnabled() } != 0 {
            return None;
        }
        let frontmost = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        let pid = frontmost.processIdentifier();
        if pid <= 0 || pid as u32 == std::process::id() {
            return None;
        }
        let focus = if let Some(element) = focused_element().filter(|element| {
            let mut element_pid = 0;
            let status = unsafe { AXUIElementGetPid(element.as_CFTypeRef(), &mut element_pid) };
            status == 0 && element_pid == pid
        }) {
            PasteFocus::Element(element)
        } else if frontmost.bundleIdentifier().as_deref() == Some(ns_string!("com.google.Chrome")) {
            // Chrome can omit the focused web control from macOS accessibility.
            PasteFocus::ChromeWindow {
                window: focused_window(pid)?,
                epoch: self.interaction_epoch.clone(),
                captured_epoch: self.interaction_epoch.get(),
            }
        } else {
            return None;
        };
        Some(PasteTarget { focus, pid })
    }
}

impl Drop for KeyMonitor {
    fn drop(&mut self) {
        for monitor in &self.monitors {
            unsafe { NSEvent::removeMonitor(monitor) };
        }
    }
}

enum PasteFocus {
    Element(CFType),
    ChromeWindow {
        window: CFType,
        epoch: Rc<Cell<u64>>,
        captured_epoch: u64,
    },
}

pub struct PasteTarget {
    focus: PasteFocus,
    pid: i32,
}

impl PasteTarget {
    fn still_focused(&self) -> bool {
        match &self.focus {
            PasteFocus::Element(element) => focused_element().as_ref() == Some(element),
            PasteFocus::ChromeWindow {
                window,
                epoch,
                captured_epoch,
            } => {
                epoch.get() == *captured_epoch
                    && NSWorkspace::sharedWorkspace()
                        .frontmostApplication()
                        .is_some_and(|app| app.processIdentifier() == self.pid)
                    && focused_window(self.pid).as_ref() == Some(window)
            }
        }
    }

    pub fn paste(&self, text: &str) -> Result<ClipboardRestore, &'static str> {
        if !is_trusted() || unsafe { IsSecureEventInputEnabled() } != 0 {
            return Err("元の入力欄に貼り付けできませんでした");
        }
        if !self.still_focused() {
            return Err("入力先が変わったため、貼り付けを見送りました");
        }
        let source = CGEventSource::new(CGEventSourceStateID::Private)
            .map_err(|_| "貼り付けキーを作成できませんでした")?;
        let down = CGEvent::new_keyboard_event(source.clone(), 9, true)
            .map_err(|_| "貼り付けキーを作成できませんでした")?;
        let up = CGEvent::new_keyboard_event(source, 9, false)
            .map_err(|_| "貼り付けキーを作成できませんでした")?;
        let board = NSPasteboard::generalPasteboard();
        let before = board.changeCount();
        let items = snapshot(&board)?;
        if board.changeCount() != before || !self.still_focused() {
            return Err("入力先またはクリップボードが変わりました");
        }
        board.clearContents();
        if !board.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString }) {
            restore(&board, &items);
            return Err("クリップボードに書き込めませんでした");
        }
        let change_count = board.changeCount();
        for event in [down, up] {
            event.set_flags(CGEventFlags::CGEventFlagCommand);
            event.post_to_pid(self.pid);
        }
        Ok(ClipboardRestore {
            change_count,
            items,
        })
    }
}

fn focused_element() -> Option<CFType> {
    let system = unsafe { CFType::wrap_under_create_rule(AXUIElementCreateSystemWide()) };
    ax_attribute(&system, "AXFocusedUIElement")
}

fn focused_window(pid: i32) -> Option<CFType> {
    let application = unsafe { CFType::wrap_under_create_rule(AXUIElementCreateApplication(pid)) };
    ax_attribute(&application, "AXFocusedWindow")
}

fn ax_attribute(element: &CFType, name: &str) -> Option<CFType> {
    let attribute = CFString::new(name);
    let mut value: *const c_void = std::ptr::null();
    let result = unsafe {
        AXUIElementCopyAttributeValue(
            element.as_CFTypeRef(),
            attribute.as_concrete_TypeRef(),
            &mut value,
        )
    };
    (result == 0 && !value.is_null()).then(|| unsafe { CFType::wrap_under_create_rule(value) })
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
