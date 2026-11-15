use crate::theme::{
    AnimationProfiles, AnimationSettings, EasingProfiles, EasingSettings, MotionSettings,
    MotionSpringCache, MotionTierSettings,
};
use gpui::{Animation, IntoElement, RenderOnce, SpringConfig, SpringState};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
pub struct MotionProfile {
    pub enter: Duration,
    pub exit: Duration,
    pub exit_easing: EasingSettings,
    pub bounce: f32,
    pub spring: SpringConfig,
}

impl MotionProfile {
    fn calibrated(
        enter: Duration,
        exit: Duration,
        bounce: f32,
        exit_easing: EasingSettings,
    ) -> Self {
        let damping_ratio = 1. - bounce;
        let normalized = SpringConfig::new(1., 2. * damping_ratio, 1.);
        let resting = SpringState {
            position: 0.,
            velocity: 0.,
        };
        // Normalize GPUI's settling criterion; Motion's resolver parity is a separate check.
        let frequency =
            normalized.settle_time(resting, 1., 0.001).as_secs_f32() / enter.as_secs_f32();
        Self {
            enter,
            exit,
            exit_easing,
            bounce,
            spring: SpringConfig::new(frequency * frequency, 2. * damping_ratio * frequency, 1.),
        }
    }

    pub fn entering(self) -> Animation {
        Animation::new(self.enter)
    }
    pub fn exiting(self) -> Animation {
        Animation::new(self.exit).with_easing(self.exit_easing.curve())
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct MotionProfiles {
    pub animations: AnimationProfiles,
    pub curves: EasingProfiles,
    pub fast: MotionProfile,
    pub moderate: MotionProfile,
    pub slow: MotionProfile,
}

impl MotionProfiles {
    pub fn from_settings(settings: &MotionSettings) -> Self {
        let mut cache = settings.runtime.lock().expect("Motion cache lock poisoned");
        if !cache.as_ref().is_some_and(|cache| {
            cache.fast_source == settings.fast
                && cache.moderate_source == settings.moderate
                && cache.slow_source == settings.slow
        }) {
            let resolve = |tier: MotionTierSettings| {
                MotionProfile::calibrated(
                    Duration::from_millis(tier.enter_ms),
                    Duration::from_millis(tier.exit_ms),
                    tier.bounce,
                    tier.exit_easing,
                )
                .spring
            };
            *cache = Some(MotionSpringCache {
                fast_source: settings.fast,
                moderate_source: settings.moderate,
                slow_source: settings.slow,
                fast: resolve(settings.fast),
                moderate: resolve(settings.moderate),
                slow: resolve(settings.slow),
            });
        }
        let cached = cache.as_ref().expect("Motion cache initialized above");
        let resolve = |tier: MotionTierSettings, spring| MotionProfile {
            enter: Duration::from_millis(tier.enter_ms),
            exit: Duration::from_millis(tier.exit_ms),
            exit_easing: tier.exit_easing,
            bounce: tier.bounce,
            spring,
        };
        Self {
            animations: settings.animations,
            curves: settings.curves,
            fast: resolve(settings.fast, cached.fast),
            moderate: resolve(settings.moderate, cached.moderate),
            slow: resolve(settings.slow, cached.slow),
        }
    }
}

/// An interruptible timed scalar used by source-owned Flow presentation.
#[derive(Clone, Copy)]
pub struct TimedValue {
    from: f32,
    target: f32,
    started: Instant,
    duration: Duration,
}

impl TimedValue {
    /// Starts at a settled value on the supplied monotonic clock.
    pub fn new(value: f32, now: Instant) -> Self {
        Self {
            from: value,
            target: value,
            started: now,
            duration: Duration::ZERO,
        }
    }
    /// Samples the current value and completion without advancing stored state.
    pub fn sample(self, now: Instant, easing: impl Fn(f32) -> f32) -> (f32, bool) {
        if self.duration.is_zero() {
            return (self.target, true);
        }
        let progress =
            (now.duration_since(self.started).as_secs_f32() / self.duration.as_secs_f32()).min(1.);
        (
            self.from + (self.target - self.from) * easing(progress),
            progress >= 1.,
        )
    }
    /// Starts a new segment from the currently sampled value when the target changes.
    pub fn retarget(
        &mut self,
        target: f32,
        now: Instant,
        duration: Duration,
        easing: impl Fn(f32) -> f32,
    ) {
        if self.target != target {
            self.from = self.sample(now, easing).0;
            self.target = target;
            self.started = now;
            self.duration = duration;
        }
    }
}

/// CSS color-mix(in oklab) with premultiplied-alpha Cartesian coordinates.
pub fn mix_oklab(left: gpui::Hsla, right: gpui::Hsla, right_weight: f64) -> gpui::Hsla {
    assert!(
        right_weight.is_finite() && (0.0..=1.0).contains(&right_weight),
        "OKLab mix weight must be finite and in 0..=1: {right_weight}"
    );
    if right_weight == 0. {
        return left;
    }
    if right_weight == 1. {
        return right;
    }
    let a = gpui::Oklcha::from(gpui::Rgba::from(left)).components();
    let b = gpui::Oklcha::from(gpui::Rgba::from(right)).components();
    let left_alpha = a[3] * (1. - right_weight);
    let right_alpha = b[3] * right_weight;
    let alpha = left_alpha + right_alpha;
    if alpha == 0. {
        return gpui::transparent_black();
    }
    let lightness = (a[0] * left_alpha + b[0] * right_alpha) / alpha;
    let x = (a[1] * a[2].to_radians().cos() * left_alpha
        + b[1] * b[2].to_radians().cos() * right_alpha)
        / alpha;
    let y = (a[1] * a[2].to_radians().sin() * left_alpha
        + b[1] * b[2].to_radians().sin() * right_alpha)
        / alpha;
    gpui::Oklcha::new(lightness, x.hypot(y), y.atan2(x).to_degrees(), alpha).into()
}

/// Reversible color and stroke interpolation shared by named interactive artwork.
pub struct ColorStrokeTween {
    from_color: gpui::Hsla,
    color: gpui::Hsla,
    from_stroke: f32,
    stroke: f32,
    phase: TimedValue,
    easing: EasingSettings,
}
impl ColorStrokeTween {
    pub fn new(color: gpui::Hsla, stroke: f32, now: Instant, easing: EasingSettings) -> Self {
        Self {
            from_color: color,
            color,
            from_stroke: stroke,
            stroke,
            phase: TimedValue::new(1., now),
            easing,
        }
    }
    pub fn retarget(
        &mut self,
        color: gpui::Hsla,
        stroke: f32,
        now: Instant,
        duration: Duration,
        easing: EasingSettings,
    ) {
        self.easing = easing;
        if self.color != color || self.stroke != stroke {
            let (current_color, current_stroke, _) = self.sample(now);
            self.from_color = current_color;
            self.from_stroke = current_stroke;
            self.color = color;
            self.stroke = stroke;
            self.phase = TimedValue::new(0., now);
            self.phase.retarget(1., now, duration, self.easing.curve());
        }
    }
    pub fn sample(&self, now: Instant) -> (gpui::Hsla, f32, bool) {
        let (amount, done) = self.phase.sample(now, self.easing.curve());
        (
            mix_oklab(self.from_color, self.color, f64::from(amount)),
            self.from_stroke + (self.stroke - self.from_stroke) * amount,
            done,
        )
    }
}

/// Animates caller-owned artwork without importing an asset or component registry.
#[derive(IntoElement)]
pub struct AnimatedArtwork {
    id: gpui::ElementId,
    color: gpui::Hsla,
    stroke: f32,
    duration: Duration,
    easing: EasingSettings,
    render: std::rc::Rc<dyn Fn(gpui::Hsla, f32) -> gpui::AnyElement>,
}
impl AnimatedArtwork {
    pub fn new(
        id: impl Into<gpui::ElementId>,
        color: gpui::Hsla,
        stroke: f32,
        duration: Duration,
        easing: EasingSettings,
        render: impl Fn(gpui::Hsla, f32) -> gpui::AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            color,
            stroke,
            duration,
            easing,
            render: std::rc::Rc::new(render),
        }
    }
}
impl RenderOnce for AnimatedArtwork {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let now = cx.background_executor().now();
        let state = window.use_keyed_state((self.id, "animated-artwork"), cx, |_, _| {
            ColorStrokeTween::new(self.color, self.stroke, now, self.easing)
        });
        let (color, stroke, done) = state.update(cx, |state, _| {
            state.retarget(self.color, self.stroke, now, self.duration, self.easing);
            state.sample(now)
        });
        if !done {
            let weak = state.downgrade();
            window.on_next_frame(move |_, cx| {
                if let Some(state) = weak.upgrade() {
                    cx.notify(state.entity_id());
                }
            });
        }
        (self.render)(color, stroke)
    }
}

/// An open-session-owned stroke reveal: initially selected artwork starts complete.
pub struct StrokeReveal {
    session: u64,
    selected: bool,
    phase: TimedValue,
    easings: EasingProfiles,
}
fn stroke_easing(selected: bool, settings: EasingProfiles) -> impl Fn(f32) -> f32 {
    (if selected {
        settings.enter
    } else {
        settings.exit
    })
    .curve()
}

impl StrokeReveal {
    pub fn new(
        session: u64,
        initially_selected: bool,
        now: Instant,
        easings: EasingProfiles,
    ) -> Self {
        Self {
            session,
            selected: initially_selected,
            phase: TimedValue::new(if initially_selected { 1. } else { 0. }, now),
            easings,
        }
    }
    pub fn update(
        &mut self,
        session: u64,
        selected: bool,
        initially_selected: bool,
        now: Instant,
        draw: Duration,
        retract: Duration,
        easings: EasingProfiles,
    ) {
        self.easings = easings;
        if self.session != session {
            *self = Self::new(session, initially_selected, now, easings);
        }
        if self.selected != selected {
            self.phase = TimedValue::new(self.sample(now).0, now);
            self.phase.retarget(
                if selected { 1. } else { 0. },
                now,
                if selected { draw } else { retract },
                stroke_easing(selected, self.easings),
            );
            self.selected = selected;
        }
    }
    /// Retargets the reveal with each phase's complete editable animation profile.
    pub fn update_animations(
        &mut self,
        session: u64,
        selected: bool,
        initially_selected: bool,
        now: Instant,
        draw: AnimationSettings,
        retract: AnimationSettings,
    ) {
        let easings = EasingProfiles {
            enter: draw.easing,
            exit: retract.easing,
            ..self.easings
        };
        self.update(
            session,
            selected,
            initially_selected,
            now,
            Duration::from_millis(draw.duration_ms),
            Duration::from_millis(retract.duration_ms),
            easings,
        );
    }
    pub fn sample(&self, now: Instant) -> (f32, bool) {
        self.phase
            .sample(now, stroke_easing(self.selected, self.easings))
    }
}

/// Popup choreography shared by source-owned dropdown and select recipes.
pub fn popup_motion(profile: MotionProfile, offset: gpui::Pixels) -> kinu_ui::PopupMotion {
    kinu_ui::PopupMotion {
        enter: profile.entering().with_easing(move |progress| {
            if progress >= 1. {
                return 1.;
            }
            profile
                .spring
                .step(
                    SpringState {
                        position: 0.,
                        velocity: 0.,
                    },
                    1.,
                    progress * profile.enter.as_secs_f32(),
                )
                .position
        }),
        exit: profile.exiting(),
        initial_opacity: 0.,
        initial_scale: 0.96,
        enter_offset: offset,
        exit_offset: offset,
        origin: kinu_ui::PopupTransformOrigin::Aligned,
    }
}
