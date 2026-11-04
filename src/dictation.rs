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
    NSPasteboardTypeString, NSPasteboardWriting,
};
use objc2_foundation::{NSArray, NSString, NSUserDefaults, ns_string};

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn AXIsProcessTrusted() -> u8;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    static kAXTrustedCheckOptionPrompt: CFStringRef;
    fn AXUIElementCreateSystemWide() -> CFTypeRef;
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

pub struct KeyMonitor(Vec<Retained<AnyObject>>);

impl KeyMonitor {
    pub fn install(sender: Sender<Key>) -> Option<Self> {
        let mask = NSEventMask::FlagsChanged | NSEventMask::KeyDown;
        let pressed = Rc::new(Cell::new(false));
        let handler = move |event: NonNull<NSEvent>| {
            // AppKit owns the event for the duration of this callback.
            let event = unsafe { event.as_ref() };
            let key = if event.r#type() == NSEventType::FlagsChanged && event.keyCode() == 63 {
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
        let monitor = Self(global.into_iter().chain(local).collect());
        (monitor.0.len() == 2).then_some(monitor)
    }
}

impl Drop for KeyMonitor {
    fn drop(&mut self) {
        for monitor in &self.0 {
            unsafe { NSEvent::removeMonitor(monitor) };
        }
    }
}

pub struct PasteTarget {
    element: CFType,
    pid: i32,
}

impl PasteTarget {
    pub fn capture() -> Option<Self> {
        if !is_trusted() || unsafe { IsSecureEventInputEnabled() } != 0 {
            return None;
        }
        let element = focused_element()?;
        let mut pid = 0;
        if unsafe { AXUIElementGetPid(element.as_CFTypeRef(), &mut pid) } != 0
            || pid <= 0
            || pid as u32 == std::process::id()
        {
            return None;
        }
        Some(Self { element, pid })
    }

    pub fn paste(&self, text: &str) -> Result<ClipboardRestore, &'static str> {
        if !is_trusted() || unsafe { IsSecureEventInputEnabled() } != 0 {
            return Err("元の入力欄に貼り付けできませんでした");
        }
        if focused_element().as_ref() != Some(&self.element) {
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
        if board.changeCount() != before || focused_element().as_ref() != Some(&self.element) {
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
    // Both AX calls return retained Core Foundation values on success.
    let system = unsafe { CFType::wrap_under_create_rule(AXUIElementCreateSystemWide()) };
    let attribute = CFString::new("AXFocusedUIElement");
    let mut value: *const c_void = std::ptr::null();
    let result = unsafe {
        AXUIElementCopyAttributeValue(
            system.as_CFTypeRef(),
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
