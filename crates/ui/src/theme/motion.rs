//! Shared motion durations for native transitions.

use std::time::Duration;

use gpui::{App, Global, Pixels, px};

pub const FLOATING_SURFACE_TRANSITION_DURATION: Duration = Duration::from_millis(100);
pub const FLOATING_SURFACE_TRANSITION_OFFSET: Pixels = px(8.0);

#[must_use]
pub const fn floating_surface_visibility(closing: bool, delta: f32) -> f32 {
    if closing { 1.0 - delta } else { delta }
}

#[derive(Clone, Copy)]
pub struct MotionTokens {
    pub fast: Duration,
    pub default: Duration,
    pub slow: Duration,
}

impl Global for MotionTokens {}

/// Resolves lifecycle waits that GPUI's visual animation primitives cannot
/// suppress automatically.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MotionPolicy {
    reduced: bool,
}

impl MotionPolicy {
    /// Reads the current application accessibility preference.
    pub fn global(cx: &App) -> Self {
        Self {
            reduced: cx.reduce_motion(),
        }
    }

    /// Returns the authored duration, or zero when reduced motion is active.
    #[must_use]
    pub const fn duration(self, duration: Duration) -> Duration {
        if self.reduced {
            Duration::ZERO
        } else {
            duration
        }
    }
}

impl MotionTokens {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            fast: Duration::from_millis(150),
            default: Duration::from_millis(200),
            slow: Duration::from_millis(300),
        }
    }

    pub fn global(cx: &App) -> Self {
        *cx.global::<Self>()
    }
}

/// Keeps an elevated, shadow-bearing surface opaque while its position and
/// elevation animate on a transparent native overlay.
#[must_use]
pub const fn elevated_surface_opacity(_progress: f32) -> f32 {
    1.0
}

pub fn install(cx: &mut App) {
    let tokens = MotionTokens::standard();
    debug_assert!(tokens.fast < tokens.default);
    debug_assert!(tokens.default < tokens.slow);
    cx.set_global(tokens);
}

pub fn cubic_bezier_with_overshoot(x1: f32, y1: f32, x2: f32, y2: f32) -> impl Fn(f32) -> f32 {
    let (cx, cy) = (3.0 * x1, 3.0 * y1);
    let (bx, by) = (3.0 * (x2 - x1) - cx, 3.0 * (y2 - y1) - cy);
    let (ax, ay) = (1.0 - cx - bx, 1.0 - cy - by);
    let sample_x = move |time: f32| ((ax * time + bx) * time + cx) * time;
    let sample_y = move |time: f32| ((ay * time + by) * time + cy) * time;
    let slope_x = move |time: f32| (3.0 * ax * time + 2.0 * bx) * time + cx;

    move |progress: f32| {
        let progress = progress.clamp(0.0, 1.0);
        let mut time = progress;
        for _ in 0..8 {
            let error = sample_x(time) - progress;
            if error.abs() < 1e-6 {
                return sample_y(time);
            }
            let slope = slope_x(time);
            if slope.abs() < 1e-6 {
                break;
            }
            time = (time - error / slope).clamp(0.0, 1.0);
        }

        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..32 {
            let coordinate_x = sample_x(time);
            if (coordinate_x - progress).abs() < 1e-6 {
                break;
            }
            if coordinate_x < progress {
                low = time;
            } else {
                high = time;
            }
            time = f32::midpoint(low, high);
        }
        sample_y(time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_scale_is_ordered() {
        let tokens = MotionTokens::standard();

        assert!(tokens.fast < tokens.default);
        assert!(tokens.default < tokens.slow);
    }

    #[test]
    fn cubic_bezier_preserves_css_overshoot() {
        let easing = cubic_bezier_with_overshoot(0.34, 1.15, 0.5, 1.0);

        assert!(easing(0.7) > 1.0);
        assert!((easing(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((easing(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn reduced_motion_removes_lifecycle_waits_without_changing_standard_tokens() {
        let duration = Duration::from_millis(200);

        assert_eq!(MotionPolicy { reduced: false }.duration(duration), duration);
        assert_eq!(
            MotionPolicy { reduced: true }.duration(duration),
            Duration::ZERO
        );
        assert_eq!(MotionTokens::standard().default, duration);
    }

    #[test]
    fn elevated_surfaces_stay_opaque_throughout_motion() {
        assert_eq!(elevated_surface_opacity(0.0), 1.0);
        assert_eq!(elevated_surface_opacity(0.5), 1.0);
        assert_eq!(elevated_surface_opacity(1.0), 1.0);
    }
}
