use crate::theme::EasingSettings;
use crate::ui::utils::motion::{TimedValue, mix_oklab};
use crate::ui::utils::presentation::{ComponentTheme, ThemeScheme};
use gpui::{App, ElementId, Entity, StyleRefinement, Styled, Window, px};
use kinu_ui::{ColorScheme, TextInputAppearance, TextInputState};
use std::cell::RefCell;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct InputSurface {
    pub(crate) background: gpui::Hsla,
    pub(crate) border: gpui::Hsla,
}
pub(crate) struct InputMotion {
    easing: EasingSettings,
    from: InputSurface,
    target: InputSurface,
    phase: TimedValue,
}
impl InputMotion {
    fn new(surface: InputSurface, now: Instant, easing: EasingSettings) -> Self {
        Self {
            easing,
            from: surface,
            target: surface,
            phase: TimedValue::new(1., now),
        }
    }
    fn sample(&self, now: Instant) -> (InputSurface, bool) {
        let (weight, done) = self.phase.sample(now, self.easing.curve());
        (
            InputSurface {
                background: mix_oklab(
                    self.from.background,
                    self.target.background,
                    f64::from(weight),
                ),
                border: mix_oklab(self.from.border, self.target.border, f64::from(weight)),
            },
            done,
        )
    }
    fn update(
        &mut self,
        target: InputSurface,
        now: Instant,
        reduced: bool,
        duration: Duration,
        easing: EasingSettings,
    ) -> (InputSurface, bool) {
        self.easing = easing;
        if self.target != target {
            self.from = self.sample(now).0;
            self.target = target;
            self.phase = TimedValue::new(0., now);
            self.phase.retarget(1., now, duration, self.easing.curve());
        }
        if reduced {
            (target, true)
        } else {
            self.sample(now)
        }
    }
}
pub(crate) fn presentation(
    tokens: ComponentTheme,
    state: TextInputState,
    scheme: ThemeScheme,
    surface: InputSurface,
) -> TextInputAppearance {
    let colors = tokens.extras(scheme);
    let metrics = tokens.size.metrics(&tokens);
    TextInputAppearance {
        style: StyleRefinement::default()
            .w_full()
            .min_w(px(0.))
            .h(px(metrics.control_height))
            .px(px(metrics.horizontal_padding))
            .border_1()
            .border_color(surface.border)
            .bg(surface.background)
            .rounded(px(tokens.shape.radii(&tokens).item))
            .font_family(tokens.typography.family.clone())
            .font_weight(tokens.typography.normal.weight)
            .font_variations(tokens.typography.normal.variations())
            .text_size(px(metrics.text_size))
            .line_height(px(metrics.line_height))
            .text_color(gpui::Hsla::from(colors.foreground))
            .opacity(if state.disabled { 0.5 } else { 1. }),
        placeholder_color: Some(gpui::Hsla::from(colors.muted_foreground)),
        resize: None,
        transition: None,
    }
}
pub(crate) type InputMotionState = Entity<RefCell<Option<InputMotion>>>;

pub(crate) fn motion_state(id: ElementId, window: &mut Window, cx: &mut App) -> InputMotionState {
    window.use_keyed_state((id, "flow-input-colors"), cx, |_, _| {
        RefCell::new(None::<InputMotion>)
    })
}

pub(crate) fn animated_appearance(
    tokens: ComponentTheme,
    motion: InputMotionState,
    now: Instant,
) -> impl Fn(TextInputState, &Window, &App) -> TextInputAppearance {
    {
        let tokens = tokens.clone();
        move |state, window, cx| {
            let scheme = if ColorScheme::current(window, cx) == ColorScheme::Dark {
                ThemeScheme::Dark
            } else {
                ThemeScheme::Light
            };
            let target = surface(&tokens, state, scheme);
            let surface = {
                let mut animation = motion.read(cx).borrow_mut();
                let animation = animation.get_or_insert_with(|| {
                    InputMotion::new(target, now, tokens.motion.animations.color.easing)
                });
                animation
                    .update(
                        target,
                        now,
                        cx.reduce_motion(),
                        Duration::from_millis(tokens.motion.animations.color.duration_ms),
                        tokens.motion.animations.color.easing,
                    )
                    .0
            };
            schedule_motion(&motion, now, window, cx);
            presentation(tokens.clone(), state, scheme, surface)
        }
    }
}

fn schedule_motion(motion: &InputMotionState, now: Instant, window: &Window, cx: &App) {
    let running = !cx.reduce_motion()
        && motion
            .read(cx)
            .borrow()
            .as_ref()
            .is_some_and(|animation| !animation.sample(now).1);
    if running {
        let motion = motion.downgrade();
        window.on_next_frame(move |_, cx| {
            if let Some(motion) = motion.upgrade() {
                cx.notify(motion.entity_id());
            }
        });
    }
}

/// Resolves the common input colors without owning input or animation state.
pub(crate) fn surface(
    tokens: &ComponentTheme,
    state: TextInputState,
    scheme: ThemeScheme,
) -> InputSurface {
    let colors = tokens.extras(scheme);
    InputSurface {
        background: if state.hovered && !state.focused && !state.disabled {
            colors.hover()
        } else {
            gpui::transparent_black()
        },
        border: if state.focused {
            gpui::Hsla::from(colors.foreground).opacity(0.4)
        } else if state.invalid {
            gpui::Hsla::from(colors.destructive).opacity(0.5)
        } else {
            gpui::Hsla::from(colors.border)
        },
    }
}
