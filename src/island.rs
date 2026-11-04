use std::{
    sync::{Arc, mpsc::Sender},
    time::{Duration, Instant},
};

use g2v_ui::theme::{colors::G2VTheme, motion::cubic_bezier_with_overshoot};
use gpui::{
    App, AppContext as _, Bounds, Context, Div, Hsla, IntoElement, MouseButton, Render, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div, point,
    prelude::*, px, size,
};
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSEvent, NSPanel, NSScreen, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
    NSWindowOrderingMode, NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, ns_string};
use objc2_quartz_core::CATransaction;
use realfft::{RealFftPlanner, RealToComplex, num_complex::Complex32};

use crate::{Input, MozuApp, Phase, SAMPLE_RATE, Update, material::native_view};

const MORPH_DURATION: Duration = Duration::from_millis(400);
const CONTENT_DELAY: Duration = Duration::from_millis(100);
const MARGIN: f64 = 12.0;
const MAX_RADIUS: f64 = 24.0;
const WAVE_BARS: usize = 36;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Anchor {
    TopLeft = 1,
    Top,
    TopRight,
    Left,
    Right,
    BottomLeft,
    #[default]
    Bottom,
    BottomRight,
}

impl Anchor {
    const SNAP_POINTS: [Self; 8] = [
        Self::TopLeft,
        Self::Top,
        Self::TopRight,
        Self::Left,
        Self::Right,
        Self::BottomLeft,
        Self::Bottom,
        Self::BottomRight,
    ];

    pub(crate) fn from_saved(value: isize) -> Self {
        Self::SNAP_POINTS
            .into_iter()
            .find(|anchor| *anchor as isize == value)
            .unwrap_or_default()
    }

    fn top(self) -> bool {
        matches!(self, Self::TopLeft | Self::Top | Self::TopRight)
    }

    fn frame(self, visible: NSRect, dimensions: NSSize) -> NSRect {
        let x = match self {
            Self::TopLeft | Self::Left | Self::BottomLeft => visible.origin.x + MARGIN,
            Self::TopRight | Self::Right | Self::BottomRight => {
                visible.origin.x + visible.size.width - MARGIN - dimensions.width
            }
            _ => visible.origin.x + (visible.size.width - dimensions.width) / 2.0,
        };
        let y = if self.top() {
            visible.origin.y + visible.size.height - MARGIN - dimensions.height
        } else if matches!(self, Self::Left | Self::Right) {
            visible.origin.y + (visible.size.height - dimensions.height) / 2.0
        } else {
            visible.origin.y + MARGIN
        };
        NSRect::new(NSPoint::new(x.round(), y.round()), dimensions)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scene {
    Tucked,
    Recording,
    Stopping,
}

impl Scene {
    fn size(self) -> NSSize {
        let (width, height) = match self {
            Self::Tucked => (40.0, 10.0),
            Self::Recording | Self::Stopping => (120.0, 32.0),
        };
        NSSize::new(width, height)
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Session {
    Idle,
    Recording,
    Stopping,
}

impl Session {
    fn read(demo: &MozuApp) -> Self {
        if !demo
            .run
            .as_ref()
            .is_some_and(|run| run.input == Input::Dictation)
        {
            return Self::Idle;
        }
        match demo.screen.phase {
            Phase::Preparing | Phase::Listening => Self::Recording,
            Phase::Stopping => Self::Stopping,
            _ => Self::Idle,
        }
    }
}

struct Morph {
    from: NSRect,
    to: NSRect,
    started: Instant,
}

pub(crate) struct Island {
    console: WindowHandle<MozuApp>,
    panel: Retained<NSWindow>,
    effect: Retained<NSVisualEffectView>,
    scene: Scene,
    session: Session,
    anchor: Anchor,
    visible: NSRect,
    shown: bool,
    opaque: bool,
    animation: Option<Morph>,
    drag_origin: Option<NSPoint>,
    dragging: bool,
    ghosts: Vec<WindowHandle<Ghosts>>,
    wave: [f32; WAVE_BARS],
    wave_tick: Instant,
}

pub(crate) fn open(console: WindowHandle<MozuApp>, cx: &mut App) -> gpui::Result<()> {
    let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(40.0), px(10.0)));
    cx.open_window(panel_options(bounds), |window, cx| {
        cx.new(|cx| Island::new(console, window, cx))
    })?;
    Ok(())
}

fn panel_options(bounds: Bounds<gpui::Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        focus: false,
        show: false,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        inactive_frame_interval: None,
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    }
}

fn configure_panel(window: &Window) -> Retained<NSWindow> {
    let panel = native_view(window).window().expect("GPUI island panel");
    panel.setStyleMask(NSWindowStyleMask::NonactivatingPanel);
    panel.setHidesOnDeactivate(false);
    let native_panel = panel
        .downcast_ref::<NSPanel>()
        .expect("nonactivating GPUI panel");
    native_panel.setBecomesKeyOnlyIfNeeded(true);
    panel
}

impl Island {
    fn new(console: WindowHandle<MozuApp>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let panel = configure_panel(window);
        panel.setHasShadow(true);
        panel.setTitle(ns_string!("音声入力"));
        let marker = MainThreadMarker::new().expect("island runs on the main thread");
        let view = native_view(window);
        let parent = unsafe { view.superview() }.expect("island content view");
        let effect =
            NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(marker), parent.bounds());
        effect.setMaterial(NSVisualEffectMaterial::HUDWindow);
        effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        effect.setState(NSVisualEffectState::Active);
        effect.setWantsLayer(true);
        effect.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        parent.addSubview_positioned_relativeTo(&effect, NSWindowOrderingMode::Below, Some(view));

        cx.spawn_in(window, async move |entity, cx| {
            let mut delay = Duration::from_millis(16);
            loop {
                cx.background_executor().timer(delay).await;
                match entity.update_in(cx, |island, window, cx| island.tick(window, cx)) {
                    Ok(Some(next)) => delay = next,
                    _ => break,
                }
            }
        })
        .detach();

        Self {
            console,
            panel,
            effect,
            scene: Scene::Tucked,
            session: Session::Idle,
            anchor: Anchor::Bottom,
            visible: NSRect::ZERO,
            shown: false,
            opaque: false,
            animation: None,
            drag_origin: None,
            dragging: false,
            ghosts: Vec::new(),
            wave: [0.0; WAVE_BARS],
            wave_tick: Instant::now(),
        }
    }

    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Duration> {
        let Ok(demo) = self.console.read(cx) else {
            self.clear_ghosts(cx);
            window.remove_window();
            return None;
        };
        let prefs = demo.preferences;
        let session = Session::read(demo);
        let spectrum = demo.screen.spectrum;
        if !prefs.show_island {
            if self.shown {
                self.clear_ghosts(cx);
                self.panel.orderOut(None);
                self.shown = false;
                self.scene = Scene::Tucked;
                self.animation = None;
                self.drag_origin = None;
                self.dragging = false;
            }
            self.session = Session::Idle;
            return Some(Duration::from_millis(200));
        }

        let previous_frame = self.panel.frame();
        let cursor = NSEvent::mouseLocation();
        if (!self.shown || (self.scene == Scene::Tucked && !self.dragging))
            && let Some(screen) = screen_under_pointer(cursor)
        {
            let visible = screen.visibleFrame();
            if visible != self.visible || !self.shown {
                self.visible = visible;
                self.anchor = prefs.anchor;
                self.apply_frame(self.anchor.frame(visible, self.scene.size()));
                self.animation = None;
            }
        }
        let opaque = NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency();
        if opaque != self.opaque {
            self.opaque = opaque;
            self.effect.setHidden(opaque);
            cx.notify();
        }
        if !self.shown {
            self.panel.orderFrontRegardless();
            self.shown = true;
        }

        if prefs.anchor != self.anchor && !self.dragging {
            self.anchor = prefs.anchor;
            self.layout(cx);
        }
        let previous_session = self.session;
        self.session = session;
        match (previous_session, session) {
            (Session::Idle, Session::Recording) => {
                self.wave.fill(0.0);
                self.transition(Scene::Recording, cx);
            }
            (_, Session::Stopping) => self.transition(Scene::Stopping, cx),
            (Session::Recording | Session::Stopping, Session::Idle) => {
                self.transition(Scene::Tucked, cx);
            }
            _ => {}
        }

        if let Some(origin) = self.drag_origin {
            self.track_drag(origin, cursor, cx);
        }

        if let Some(animation) = &self.animation {
            let elapsed = animation.started.elapsed();
            if elapsed >= MORPH_DURATION {
                self.apply_frame(animation.to);
                self.animation = None;
            } else {
                let ease = cubic_bezier_with_overshoot(0.22, 1.0, 0.36, 1.0);
                let delta = f64::from(ease(elapsed.as_secs_f32() / MORPH_DURATION.as_secs_f32()));
                self.apply_frame(interpolate(animation.from, animation.to, delta));
            }
            cx.notify();
        }
        if session == Session::Recording && self.wave_tick.elapsed() >= Duration::from_millis(30) {
            for (bar, next) in self.wave.iter_mut().zip(spectrum.0) {
                *bar += (next - *bar) * 0.3;
            }
            self.wave_tick = Instant::now();
            cx.notify();
        }

        // AppKit's resize callback cannot reenter GPUI while this window is borrowed.
        if self.panel.frame() != previous_frame || self.dragging {
            window.bounds_changed(cx);
        }

        Some(if self.animation.is_some() || self.drag_origin.is_some() {
            Duration::from_millis(16)
        } else if self.scene == Scene::Tucked {
            Duration::from_millis(100)
        } else {
            Duration::from_millis(30)
        })
    }

    fn transition(&mut self, scene: Scene, cx: &mut Context<Self>) {
        if scene != self.scene {
            self.scene = scene;
            self.layout(cx);
            cx.notify();
        }
    }

    fn layout(&mut self, cx: &App) {
        let to = self.anchor.frame(self.visible, self.scene.size());
        if cx.reduce_motion() {
            self.apply_frame(to);
            self.animation = None;
        } else {
            self.animation = Some(Morph {
                from: self.panel.frame(),
                to,
                started: Instant::now(),
            });
        }
    }

    fn apply_frame(&self, frame: NSRect) {
        self.panel.setFrame_display(frame, true);
        if let Some(layer) = self.effect.layer() {
            CATransaction::begin();
            CATransaction::setDisableActions(true);
            layer.setMasksToBounds(true);
            layer.setCornerRadius(radius(frame.size));
            CATransaction::commit();
        }
    }

    fn track_drag(&mut self, origin: NSPoint, cursor: NSPoint, cx: &mut Context<Self>) {
        if NSEvent::pressedMouseButtons() & 1 == 0 {
            self.drag_origin = None;
            if self.dragging {
                self.dragging = false;
                self.clear_ghosts(cx);
                if let Some(screen) = screen_under_pointer(cursor) {
                    self.visible = screen.visibleFrame();
                    self.anchor = nearest_anchor(self.visible, cursor);
                    let anchor = self.anchor;
                    let _ = self.console.update(cx, |demo, _, cx| {
                        demo.preferences.anchor = anchor;
                        demo.preferences.save();
                        cx.notify();
                    });
                }
                self.layout(cx);
                cx.notify();
            }
            return;
        }
        if !self.dragging && distance(origin, cursor) >= 4.0 {
            self.dragging = true;
            self.animation = None;
            self.ghosts = open_ghosts(cx);
        }
        if self.dragging {
            let dimensions = self.panel.frame().size;
            self.apply_frame(NSRect::new(
                NSPoint::new(
                    cursor.x - dimensions.width / 2.0,
                    cursor.y - dimensions.height / 2.0,
                ),
                dimensions,
            ));
            let screen = screen_under_pointer(cursor).map(|screen| screen.visibleFrame());
            for ghost in &self.ghosts {
                let _ = ghost.update(cx, |ghost, _, cx| {
                    let next = (Some(ghost.visible) == screen)
                        .then(|| nearest_anchor(ghost.visible, cursor));
                    if next != ghost.highlighted {
                        ghost.highlighted = next;
                        cx.notify();
                    }
                });
            }
        }
    }

    fn clear_ghosts(&mut self, cx: &mut App) {
        for ghost in self.ghosts.drain(..) {
            let _ = ghost.update(cx, |_, window, _| window.remove_window());
        }
    }
}

impl Render for Island {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let scene = self.scene;
        let radius = px(radius(self.panel.frame().size) as f32);
        let tint = theme
            .window_material
            .alpha(if self.opaque { 1.0 } else { 0.8 });
        let wave = Spectrum(self.wave).wave(WaveSize::Island, theme.muted_foreground);
        let opacity = self.animation.as_ref().map_or(1.0, |morph| {
            (morph
                .started
                .elapsed()
                .saturating_sub(CONTENT_DELAY)
                .as_secs_f32()
                / 0.12)
                .min(1.0)
        });

        div()
            .id("dictation-island")
            .size_full()
            .overflow_hidden()
            .rounded(radius)
            .border_1()
            .border_color(theme.border.opacity(0.45))
            .bg(tint)
            .text_color(theme.foreground)
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|island, _, _, _| {
                    island.drag_origin = Some(NSEvent::mouseLocation());
                }),
            )
            .on_mouse_move(cx.listener(|island, _, _, cx| {
                if let Some(origin) = island.drag_origin
                    && NSEvent::pressedMouseButtons() & 1 != 0
                {
                    island.track_drag(origin, NSEvent::mouseLocation(), cx);
                }
            }))
            .children((scene != Scene::Tucked).then(|| {
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .opacity(opacity)
                    .when(opacity <= 0.01, |content| content.invisible())
                    .children((scene == Scene::Recording).then_some(wave))
                    .children((scene == Scene::Stopping).then(|| {
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child("処理中…")
                    }))
            }))
    }
}

fn radius(dimensions: NSSize) -> f64 {
    MAX_RADIUS
        .min(dimensions.height / 2.0)
        .min(dimensions.width / 2.0)
}

fn screen_under_pointer(cursor: NSPoint) -> Option<Retained<NSScreen>> {
    let marker = MainThreadMarker::new().expect("island runs on the main thread");
    let screens = NSScreen::screens(marker);
    screens
        .iter()
        .find(|screen| contains(screen.frame(), cursor))
        .or_else(|| screens.iter().next())
}

fn contains(rect: NSRect, point: NSPoint) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.width
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.height
}

fn distance(a: NSPoint, b: NSPoint) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn nearest_anchor(visible: NSRect, cursor: NSPoint) -> Anchor {
    Anchor::SNAP_POINTS
        .into_iter()
        .min_by(|a, b| {
            let center = |anchor: Anchor| {
                let frame = anchor.frame(visible, Scene::Recording.size());
                NSPoint::new(
                    frame.origin.x + frame.size.width / 2.0,
                    frame.origin.y + frame.size.height / 2.0,
                )
            };
            distance(center(*a), cursor).total_cmp(&distance(center(*b), cursor))
        })
        .unwrap_or_default()
}

fn interpolate(from: NSRect, to: NSRect, delta: f64) -> NSRect {
    let mix = |from, to| from + (to - from) * delta;
    NSRect::new(
        NSPoint::new(
            mix(from.origin.x, to.origin.x),
            mix(from.origin.y, to.origin.y),
        ),
        NSSize::new(
            mix(from.size.width, to.size.width),
            mix(from.size.height, to.size.height),
        ),
    )
}

struct Ghosts {
    visible: NSRect,
    highlighted: Option<Anchor>,
}

impl Render for Ghosts {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        div()
            .size_full()
            .relative()
            .children(Anchor::SNAP_POINTS.into_iter().map(|anchor| {
                let frame = anchor.frame(self.visible, Scene::Recording.size());
                let selected = self.highlighted == Some(anchor);
                div()
                    .absolute()
                    .left(px((frame.origin.x - self.visible.origin.x) as f32))
                    .bottom(px((frame.origin.y - self.visible.origin.y) as f32))
                    .w(px(frame.size.width as f32))
                    .h(px(frame.size.height as f32))
                    .rounded_full()
                    .border_1()
                    .border_color(if selected {
                        theme.blue_500
                    } else {
                        theme.foreground.opacity(0.35)
                    })
                    .bg(theme
                        .background_solid
                        .opacity(if selected { 0.7 } else { 0.4 }))
            }))
    }
}

fn open_ghosts(cx: &mut App) -> Vec<WindowHandle<Ghosts>> {
    let marker = MainThreadMarker::new().expect("island runs on the main thread");
    NSScreen::screens(marker)
        .iter()
        .filter_map(|screen| {
            let visible = screen.visibleFrame();
            let bounds = Bounds::new(
                point(px(0.0), px(0.0)),
                size(
                    px(visible.size.width as f32),
                    px(visible.size.height as f32),
                ),
            );
            match cx.open_window(panel_options(bounds), |window, cx| {
                let panel = configure_panel(window);
                panel.setIgnoresMouseEvents(true);
                panel.setHasShadow(false);
                panel.setLevel(panel.level() - 1);
                panel.setFrame_display(visible, true);
                panel.orderFrontRegardless();
                cx.new(|_| Ghosts {
                    visible,
                    highlighted: None,
                })
            }) {
                Ok(window) => Some(window),
                Err(error) => {
                    eprintln!("Could not display island snap targets: {error}");
                    None
                }
            }
        })
        .collect()
}

#[derive(Clone, Copy)]
pub(crate) struct Spectrum([f32; WAVE_BARS]);

pub(crate) enum WaveSize {
    Island,
    Badge,
}

impl Spectrum {
    pub(crate) fn wave(self, size: WaveSize, color: Hsla) -> Div {
        let (columns, width, height, gap) = match size {
            WaveSize::Island => (18, 3.6, 26.0, 2.4),
            WaveSize::Badge => (7, 2.0, 12.0, 1.5),
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .h(px(height))
            .gap(px(gap))
            .children((0..columns).map(|index| {
                let level = self.0[index * (WAVE_BARS - 1) / (columns - 1)];
                let edge = index.min(columns - 1 - index) as f32;
                let alpha = ((edge + 0.4) / (columns as f32 * 0.15)).min(1.0);
                div()
                    .w(px(width))
                    .h(px((level.powf(0.7) * height).max(width)))
                    .rounded_full()
                    .bg(color.opacity(alpha))
            }))
    }
}

impl Default for Spectrum {
    fn default() -> Self {
        Self([0.0; WAVE_BARS])
    }
}

pub(crate) struct AudioMeter {
    fft: Arc<dyn RealToComplex<f32>>,
    input: Vec<f32>,
    bins: Vec<Complex32>,
    scratch: Vec<Complex32>,
    pending: Vec<f32>,
    samples: usize,
    spectrum: Spectrum,
}

impl AudioMeter {
    pub(crate) fn new() -> Self {
        // Preserve the source island's ~85 ms analysis window after mono resampling.
        let length = (4096.0_f64 * f64::from(SAMPLE_RATE) / 48000.0).round() as usize;
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(length);
        Self {
            input: fft.make_input_vec(),
            bins: fft.make_output_vec(),
            scratch: fft.make_scratch_vec(),
            fft,
            pending: Vec::with_capacity(length * 2),
            samples: 0,
            spectrum: Spectrum::default(),
        }
    }

    pub(crate) fn send(
        &mut self,
        samples: &[f32],
        updates: &Sender<Update>,
    ) -> mozu_audio::Result<bool> {
        if samples.is_empty() {
            return Ok(true);
        }
        self.samples += samples.len();
        self.pending.extend_from_slice(samples);
        let length = self.input.len();
        let mut chunks = self.pending.chunks_exact(length);
        for chunk in &mut chunks {
            self.input.copy_from_slice(chunk);
            self.fft
                .process_with_scratch(&mut self.input, &mut self.bins, &mut self.scratch)
                .map_err(|error| {
                    mozu_audio::Error::Invalid(format!("Analyze microphone spectrum: {error}"))
                })?;
            self.spectrum = Spectrum(std::array::from_fn(|index| {
                let t = index as f32 / (WAVE_BARS - 1) as f32;
                let bin = 1.0 + 198.0 * (3.5 * t).exp_m1() / 3.5_f32.exp_m1();
                let low = bin.floor() as usize;
                let fraction = bin.fract();
                let magnitude =
                    self.bins[low].norm() * (1.0 - fraction) + self.bins[low + 1].norm() * fraction;
                ((magnitude * (4096.0 / length as f32) * 4.0).min(100.0) / 60.0).clamp(0.0, 1.0)
            }));
        }
        let used = self.pending.len() - chunks.remainder().len();
        self.pending.drain(..used);
        Ok(updates
            .send(Update::Position {
                seconds: self.samples as f64 / f64::from(SAMPLE_RATE),
                spectrum: self.spectrum,
            })
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_morph_keeps_the_selected_screen_edge_fixed() {
        let display = NSRect::new(NSPoint::new(-1440.0, 40.0), NSSize::new(1440.0, 860.0));
        for anchor in Anchor::SNAP_POINTS {
            let idle = anchor.frame(display, Scene::Tucked.size());
            let expanded = anchor.frame(display, Scene::Recording.size());
            if anchor.top() {
                assert_eq!(
                    idle.origin.y + idle.size.height,
                    expanded.origin.y + expanded.size.height
                );
            } else if matches!(
                anchor,
                Anchor::Bottom | Anchor::BottomLeft | Anchor::BottomRight
            ) {
                assert_eq!(idle.origin.y, expanded.origin.y);
            }
            let center = NSPoint::new(
                idle.origin.x + idle.size.width / 2.0,
                idle.origin.y + idle.size.height / 2.0,
            );
            assert_eq!(nearest_anchor(display, center), anchor);
            assert_eq!(Anchor::from_saved(anchor as isize), anchor);
        }
    }

    #[test]
    fn waveform_updates_from_split_capture_without_waiting_for_recognition() {
        let mut meter = AudioMeter::new();
        let (sender, receiver) = std::sync::mpsc::channel();
        let samples = (0..meter.input.len())
            .map(|sample| {
                (2.0 * std::f32::consts::PI * 180.0 * sample as f32 / SAMPLE_RATE as f32).sin()
                    * 0.1
            })
            .collect::<Vec<_>>();
        let middle = samples.len() / 2;
        meter.send(&samples[..middle], &sender).unwrap();
        meter.send(&samples[middle..], &sender).unwrap();
        let Update::Position {
            spectrum: initial, ..
        } = receiver.recv().unwrap()
        else {
            panic!("Expected audio position");
        };
        let Update::Position {
            seconds, spectrum, ..
        } = receiver.recv().unwrap()
        else {
            panic!("Expected audio position");
        };
        assert_eq!(initial.0, [0.0; WAVE_BARS]);
        assert!(spectrum.0.iter().any(|&level| level > 0.05));
        assert_eq!(seconds, samples.len() as f64 / f64::from(SAMPLE_RATE));
    }
}
