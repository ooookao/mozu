use crate::ui::utils::motion::{MotionProfile, TimedValue};
use gpui::prelude::*;
use gpui::{
    AnimationExt, App, Bounds, ElementId, Hsla, IntoElement, Pixels, Point, RenderOnce,
    SpringAnimation, SpringPlayback, Window, div,
};

struct HoverFade {
    target: f32,
    phase: TimedValue,
}
impl HoverFade {
    fn mounted(visible: bool, initial: Option<f32>, now: std::time::Instant) -> Self {
        let opacity = if visible { initial.unwrap_or(1.) } else { 0. };
        Self {
            target: opacity,
            phase: TimedValue::new(opacity, now),
        }
    }
    fn retarget(&mut self, opacity: f32, motion: MotionProfile, now: std::time::Instant) {
        let duration = if opacity > self.target {
            motion.enter
        } else {
            motion.exit
        };
        self.phase.retarget(opacity, now, duration, gpui::linear);
        self.target = opacity;
    }
    fn sample(&self, now: std::time::Instant) -> (f32, bool) {
        self.phase.sample(now, gpui::linear)
    }
}

pub struct HoverItem {
    pub id: ElementId,
    pub bounds: Bounds<Pixels>,
    pub enabled: bool,
}

#[derive(Clone, Copy)]
pub enum HoverAxis {
    Horizontal,
    Vertical,
    Both,
}

/// Projects the nearest enabled measured item without changing input or selection.
pub fn nearest_enabled(
    pointer: Point<Pixels>,
    items: &[HoverItem],
    axis: HoverAxis,
) -> Option<ElementId> {
    items
        .iter()
        .filter(|item| item.enabled)
        .min_by(|left, right| {
            distance(pointer, left.bounds.center(), axis).total_cmp(&distance(
                pointer,
                right.bounds.center(),
                axis,
            ))
        })
        .map(|item| item.id.clone())
}

#[derive(Clone, Copy)]
pub enum HoverTarget {
    Containing,
    Nearest,
}

#[derive(Clone, PartialEq)]
pub struct HoverSnapshot {
    pub active: Option<ElementId>,
    pub bounds: Option<Bounds<Pixels>>,
    pub session: usize,
}

/// Owns visual hover targets, independently of selection and keyboard focus.
pub struct HoverMotion {
    snapshot: HoverSnapshot,
    target: Option<ElementId>,
}

impl HoverMotion {
    pub fn new() -> Self {
        Self {
            target: None,
            snapshot: HoverSnapshot {
                active: None,
                bounds: None,
                session: 0,
            },
        }
    }

    /// Items supply already measured bounds in the pointer's coordinate space.
    pub fn update<I>(
        &mut self,
        pointer: Point<Pixels>,
        items: I,
        axis: HoverAxis,
        target: HoverTarget,
    ) -> bool
    where
        I: IntoIterator,
        I::Item: std::borrow::Borrow<HoverItem>,
    {
        use std::borrow::Borrow;

        let mut candidate: Option<I::Item> = None;
        for entry in items {
            let item: &HoverItem = entry.borrow();
            if !item.enabled {
                continue;
            }
            if item.bounds.contains(&pointer) {
                candidate = Some(entry);
                break;
            }
            if matches!(target, HoverTarget::Nearest)
                && candidate.as_ref().is_none_or(|current| {
                    let current: &HoverItem = current.borrow();
                    distance(pointer, item.bounds.center(), axis).total_cmp(&distance(
                        pointer,
                        current.bounds.center(),
                        axis,
                    )) == std::cmp::Ordering::Less
                })
            {
                candidate = Some(entry);
            }
        }
        let item: Option<&HoverItem> = candidate.as_ref().map(|entry| entry.borrow());
        let Some(item) = item else {
            return self.clear();
        };
        let previous = self.snapshot.clone();
        if self.snapshot.active.is_none() {
            self.snapshot.session = self
                .snapshot
                .session
                .checked_add(1)
                .expect("Hover session overflow");
        }
        self.target = Some(item.id.clone());
        self.snapshot.active = Some(item.id.clone());
        self.snapshot.bounds = Some(item.bounds);
        self.snapshot != previous
    }

    pub fn clear(&mut self) -> bool {
        self.snapshot.active.take().is_some()
    }

    /// Retains the last target during its exit so callers can resolve current local geometry.
    pub fn target(&self) -> Option<&ElementId> {
        self.target.as_ref()
    }

    pub fn snapshot(&self) -> HoverSnapshot {
        self.snapshot.clone()
    }
}

fn distance(pointer: Point<Pixels>, center: Point<Pixels>, axis: HoverAxis) -> f32 {
    let x = f32::from(pointer.x - center.x);
    let y = f32::from(pointer.y - center.y);
    match axis {
        HoverAxis::Horizontal => x.abs(),
        HoverAxis::Vertical => y.abs(),
        HoverAxis::Both => x.hypot(y),
    }
}

/// One non-interactive background per hover group; item activation stays with the primitive.
#[derive(IntoElement)]
pub struct HoverHighlight {
    id: ElementId,
    snapshot: HoverSnapshot,
    color: Hsla,
    radius: Pixels,
    motion: MotionProfile,
    outline: Option<(Pixels, Hsla)>,
    initial_bounds: Option<Bounds<Pixels>>,
    initial_opacity: Option<f32>,
    geometry_playback: SpringPlayback,
    surface: Option<Box<dyn FnOnce(gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div>>>,
    frame: Option<Box<dyn Fn(Bounds<Pixels>, f32, &mut Window, &mut App)>>,
}

impl HoverHighlight {
    pub fn new(
        id: impl Into<ElementId>,
        snapshot: HoverSnapshot,
        color: Hsla,
        radius: Pixels,
        motion: MotionProfile,
    ) -> Self {
        Self {
            id: id.into(),
            snapshot,
            color,
            radius,
            motion,
            outline: None,
            initial_bounds: None,
            initial_opacity: None,
            geometry_playback: SpringPlayback::Running,
            surface: None,
            frame: None,
        }
    }
    /// Composes a non-interactive surface inside the shared moving rectangle.
    pub fn surface(
        mut self,
        render: impl FnOnce(gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div> + 'static,
    ) -> Self {
        self.surface = Some(Box::new(render));
        self
    }

    /// Observes the current animated visual rectangle and opacity before painting.
    pub fn on_frame(
        mut self,
        observe: impl Fn(Bounds<Pixels>, f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.frame = Some(Box::new(observe));
        self
    }

    /// Starts a newly mounted highlight from an existing local rectangle.
    pub fn initial_bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.initial_bounds = Some(bounds);
        self
    }
    /// Sets opacity on the first visible mount; later reversals preserve the sampled value.
    /// Omitted, existing callers start opaque and retain their previous entrance behavior.
    pub fn initial_opacity(mut self, opacity: f32) -> Self {
        assert!(
            opacity.is_finite() && (0. ..=1.).contains(&opacity),
            "Hover highlight initial opacity must be finite and in 0..=1: {opacity}"
        );
        self.initial_opacity = Some(opacity);
        self
    }

    /// Controls rectangle springs independently from the shared opacity transition.
    /// Running glides between targets; Completed snaps to a reflowed target.
    pub fn geometry_playback(mut self, playback: SpringPlayback) -> Self {
        self.geometry_playback = playback;
        self
    }

    /// Draws an outlined moving rectangle using the same geometry and motion as the fill.
    pub fn outline(mut self, width: Pixels, color: Hsla) -> Self {
        self.outline = Some((width, color));
        self
    }
}

impl RenderOnce for HoverHighlight {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let visible = self.snapshot.active.is_some();
        let now = cx.background_executor().now();
        let fade = window.use_keyed_state((self.id.clone(), "flow-hover-opacity"), cx, |_, _| {
            HoverFade::mounted(visible, self.initial_opacity, now)
        });
        let (opacity, done) = fade.update(cx, |fade, _| {
            fade.retarget(if visible { 1. } else { 0. }, self.motion, now);
            fade.sample(now)
        });
        if !done {
            window.request_animation_frame();
        }
        div()
            .id(self.id)
            .absolute()
            .inset_0()
            .opacity(opacity)
            .when_some(self.snapshot.bounds, |layer, bounds| {
                layer.child(
                    div()
                        .id(("session", self.snapshot.session))
                        .absolute()
                        .top_0()
                        .left_0()
                        .bg(self.color)
                        .rounded(self.radius)
                        .when_some(self.outline, |element, (width, color)| {
                            element.border(width).border_color(color)
                        })
                        .when_some(self.surface, |element, render| render(element))
                        .when_some(self.frame, |element, observe| {
                            element.on_prepainted(move |bounds, window, cx| {
                                observe(window.element_to_window(bounds), opacity, window, cx);
                            })
                        })
                        .with_spring(
                            "x",
                            SpringAnimation::new(self.motion.spring)
                                .playback(self.geometry_playback)
                                .to(bounds.origin.x)
                                .from(self.initial_bounds.unwrap_or(bounds).origin.x),
                            |el, x| el.translate_x(x),
                        )
                        .with_spring(
                            "y",
                            SpringAnimation::new(self.motion.spring)
                                .playback(self.geometry_playback)
                                .to(bounds.origin.y)
                                .from(self.initial_bounds.unwrap_or(bounds).origin.y),
                            |el, y| el.map_element(|inner| inner.translate_y(y)),
                        )
                        .with_spring(
                            "width",
                            SpringAnimation::new(self.motion.spring)
                                .playback(self.geometry_playback)
                                .to(bounds.size.width)
                                .from(self.initial_bounds.unwrap_or(bounds).size.width),
                            |el, width| {
                                el.map_element(|inner| inner.map_element(|inner| inner.w(width)))
                            },
                        )
                        .with_spring(
                            "height",
                            SpringAnimation::new(self.motion.spring)
                                .playback(self.geometry_playback)
                                .to(bounds.size.height)
                                .from(self.initial_bounds.unwrap_or(bounds).size.height),
                            |el, height| {
                                el.map_element(|inner| {
                                    inner.map_element(|inner| {
                                        inner.map_element(|inner| inner.h(height))
                                    })
                                })
                            },
                        ),
                )
            })
    }
}

/// Retains identity when a row stays put, and distinguishes navigation from reflow.
#[derive(Clone)]
pub struct HoverOverlayTarget {
    row: Option<ElementId>,
    rect: Option<Bounds<Pixels>>,
    row_changed: bool,
    pub snapshot: HoverSnapshot,
}
impl HoverOverlayTarget {
    pub fn new() -> Self {
        Self {
            row: None,
            rect: None,
            row_changed: false,
            snapshot: HoverSnapshot {
                active: None,
                bounds: None,
                session: 0,
            },
        }
    }
    pub fn retarget(
        &mut self,
        row: Option<ElementId>,
        rect: Option<Bounds<Pixels>>,
        session: usize,
    ) {
        if self.row != row || self.rect != rect {
            self.row_changed = self.row != row;
            self.row = row.clone();
            self.rect = rect;
        }
        self.snapshot.active = row;
        if let Some(rect) = rect {
            self.snapshot.bounds = Some(rect);
        }
        self.snapshot.session = session;
    }
    pub fn playback(&self) -> SpringPlayback {
        if self.row_changed {
            SpringPlayback::Running
        } else {
            SpringPlayback::Completed
        }
    }
}
