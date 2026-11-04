use g2v_ui::theme::colors::G2VTheme;
use gpui::{Hsla, Rgba, Window, WindowBackgroundAppearance, rgb};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSTitlebarSeparatorStyle, NSToolbar, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindowOrderingMode, NSWindowToolbarStyle, NSWorkspace,
};
use objc2_foundation::{NSArray, NSNumber, NSPoint, NSProcessInfo, NSString, ns_string};
use objc2_quartz_core::CALayer;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const BACKDROP_BLUR_RADIUS: f64 = 60.0;
const BASE_TINT_ALPHA: f32 = 0.80;

pub(crate) fn sidebar_tint(main: Hsla, theme: &G2VTheme) -> Hsla {
    let base: Rgba = main.alpha(1.0).into();
    let accent: Rgba = theme.foreground.alpha(0.04).into();
    let tint: Hsla = base.blend(accent).into();
    tint.alpha(main.a)
}

pub(crate) fn native_view(window: &Window) -> &NSView {
    let handle = HasWindowHandle::window_handle(window).expect("Mozu window handle");
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        unreachable!("Mozu requires an AppKit window");
    };
    // GPUI owns this NSView for the lifetime of the window.
    unsafe { handle.ns_view.cast::<NSView>().as_ref() }
}

pub(crate) struct WindowMaterial {
    backdrop: Retained<MozuBackdropView>,
    opaque: bool,
}

impl WindowMaterial {
    pub(crate) fn install(window: &Window) -> Self {
        let main_thread = MainThreadMarker::new().expect("install material on the main thread");
        let render_view = native_view(window);
        // On Tahoe, AppKit gives unified-toolbar windows a larger frame radius.
        if NSProcessInfo::processInfo()
            .operatingSystemVersion()
            .majorVersion
            == 26
            && let Some(native_window) = render_view.window()
        {
            let toolbar = NSToolbar::initWithIdentifier(
                NSToolbar::alloc(main_thread),
                ns_string!("MozuWindowToolbar"),
            );
            native_window.setToolbarStyle(NSWindowToolbarStyle::Unified);
            native_window.setToolbar(Some(&toolbar));
            native_window.setTitlebarSeparatorStyle(NSTitlebarSeparatorStyle::None);
        }
        let host = unsafe { render_view.superview() }.expect("GPUI window content view");
        let backdrop: Retained<MozuBackdropView> = unsafe {
            msg_send![MozuBackdropView::alloc(main_thread), initWithFrame: host.bounds()]
        };
        backdrop.setMaterial(NSVisualEffectMaterial::UnderWindowBackground);
        backdrop.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        backdrop.setState(NSVisualEffectState::Active);
        backdrop.setWantsLayer(true);
        backdrop.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        host.addSubview_positioned_relativeTo(
            &backdrop,
            NSWindowOrderingMode::Below,
            Some(render_view),
        );
        let mut material = Self {
            backdrop,
            opaque: true,
        };
        material.sync(window);
        material
    }

    pub(crate) fn sync(&mut self, window: &Window) {
        self.opaque = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency();
        self.backdrop.setHidden(self.opaque);
        window.set_background_appearance(if self.opaque {
            WindowBackgroundAppearance::Opaque
        } else {
            WindowBackgroundAppearance::Transparent
        });
    }

    pub(crate) fn tint(&self, theme: &G2VTheme) -> Hsla {
        if self.opaque {
            return theme.background_solid;
        }

        let backdrop = rgb(if theme.is_dark { 0xffffff } else { 0x000000 });
        let foreground = theme.foreground.into();
        let muted = theme.muted_foreground.into();
        (0..=20)
            .map(|step| BASE_TINT_ALPHA + (1.0 - BASE_TINT_ALPHA) * step as f32 / 20.0)
            .map(|alpha| theme.window_material.alpha(alpha))
            .find(|tint| {
                let background = backdrop.blend((*tint).into());
                contrast(foreground, background) >= 4.5 && contrast(muted, background) >= 3.0
            })
            .unwrap_or_else(|| theme.window_material.alpha(1.0))
    }
}

fn contrast(foreground: Rgba, background: Rgba) -> f32 {
    let foreground = luminance(background.blend(foreground));
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn luminance(color: Rgba) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

define_class!(
    #[unsafe(super(NSVisualEffectView))]
    #[thread_kind = MainThreadOnly]
    struct MozuBackdropView;

    impl MozuBackdropView {
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView {
            std::ptr::null_mut()
        }

        #[unsafe(method(updateLayer))]
        fn update_layer(&self) {
            let _: () = unsafe { msg_send![super(self), updateLayer] };
            if let Some(layer) = self.layer() {
                style_backdrop(&layer);
            }
        }
    }
);

fn style_backdrop(layer: &CALayer) {
    layer.setBackgroundColor(None);
    let class_name: Retained<NSString> = unsafe { msg_send![layer, className] };
    if class_name.isEqualToString(ns_string!("CAChameleonLayer")) {
        layer.setHidden(true);
        return;
    }

    // AppKit owns these filters and does not expose the blur radius as a public property.
    if let Some(filters) = layer.filters() {
        let mut retained = Vec::new();
        let mut changed = false;
        for filter in filters.iter() {
            let name: Retained<NSString> = unsafe { msg_send![&*filter, description] };
            if name.containsString(ns_string!("Saturat")) {
                changed = true;
                continue;
            }
            if name.containsString(ns_string!("Blur")) {
                let radius = NSNumber::numberWithDouble(BACKDROP_BLUR_RADIUS);
                let _: () = unsafe {
                    msg_send![&*filter, setValue: &*radius, forKey: ns_string!("inputRadius")]
                };
                changed = true;
            }
            retained.push(filter);
        }
        if changed {
            let filters = NSArray::from_retained_slice(&retained);
            unsafe { layer.setFilters(Some(&filters)) };
        }
    }

    if let Some(sublayers) = unsafe { layer.sublayers() } {
        for sublayer in sublayers.iter() {
            style_backdrop(&sublayer);
        }
    }
}
