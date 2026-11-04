//! Native Thinking Indicator with shared motion and accessibility contracts.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, ElementId, EntityId, Global, InteractiveElement as _, IntoElement,
    ParentElement, PathBuilder, RenderOnce, Role, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, canvas, div, ease_in_out, point, prelude::FluentBuilder as _, px,
};
use gpui_base::StyledExt as _;

use crate::theme::colors::G2VTheme;

use super::ShimmerText;

const WORDS: [&str; 4] = ["Thinking", "Moonwalking", "Planning", "Refining"];
const LONGEST_WORD: &str = "Moonwalking";
const WORD_DURATION: Duration = Duration::from_secs(4);
const WORD_ENTER_DURATION: Duration = Duration::from_millis(240);
const WORD_EXIT_DURATION: Duration = Duration::from_millis(160);
const MORPH_DURATION: Duration = Duration::from_secs(6);
const SHIMMER_DURATION: Duration = Duration::from_millis(1_500);
const CLOCK_TICK: Duration = Duration::from_nanos(16_666_667);
const CLOCK_LEASE: Duration = Duration::from_millis(300);

type PathPoints = [(f32, f32); 13];

const CIRCLE_A: PathPoints = [
    (12.0, 8.0),
    (14.21, 8.0),
    (16.0, 9.79),
    (16.0, 12.0),
    (16.0, 14.21),
    (14.21, 16.0),
    (12.0, 16.0),
    (9.79, 16.0),
    (8.0, 14.21),
    (8.0, 12.0),
    (8.0, 9.79),
    (9.79, 8.0),
    (12.0, 8.0),
];
const INFINITY: PathPoints = [
    (12.0, 12.0),
    (14.0, 8.5),
    (19.0, 8.5),
    (19.0, 12.0),
    (19.0, 15.5),
    (14.0, 15.5),
    (12.0, 12.0),
    (10.0, 8.5),
    (5.0, 8.5),
    (5.0, 12.0),
    (5.0, 15.5),
    (10.0, 15.5),
    (12.0, 12.0),
];
const CIRCLE_B: PathPoints = [
    (12.0, 16.0),
    (14.21, 16.0),
    (16.0, 14.21),
    (16.0, 12.0),
    (16.0, 9.79),
    (14.21, 8.0),
    (12.0, 8.0),
    (9.79, 8.0),
    (8.0, 9.79),
    (8.0, 12.0),
    (8.0, 14.21),
    (9.79, 16.0),
    (12.0, 16.0),
];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ThinkingIndicatorSize {
    Compact,
    #[default]
    Default,
}

#[derive(Default)]
struct ThinkingClock {
    epoch: Option<Instant>,
    leases: HashMap<EntityId, Instant>,
    running: bool,
}

impl Global for ThinkingClock {}

#[derive(IntoElement)]
#[must_use]
pub struct ThinkingIndicator {
    id: ElementId,
    show_icon: bool,
    size: ThinkingIndicatorSize,
    style: StyleRefinement,
}

impl ThinkingIndicator {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            show_icon: true,
            size: ThinkingIndicatorSize::Default,
            style: StyleRefinement::default(),
        }
    }

    pub fn show_icon(mut self, show_icon: bool) -> Self {
        self.show_icon = show_icon;
        self
    }

    pub fn size(mut self, size: ThinkingIndicatorSize) -> Self {
        self.size = size;
        self
    }
}

impl Styled for ThinkingIndicator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ThinkingIndicator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let icon_color = G2VTheme::global(cx).muted_foreground;
        let reduced_motion = cx.reduce_motion();
        let elapsed = animation_elapsed(window.current_view(), reduced_motion, cx);
        let icon_size = match self.size {
            ThinkingIndicatorSize::Compact => 18.0,
            ThinkingIndicatorSize::Default => 20.0,
        };
        let text_size = match self.size {
            ThinkingIndicatorSize::Compact => 12.0,
            ThinkingIndicatorSize::Default => 13.0,
        };
        let points = if reduced_motion {
            INFINITY
        } else {
            morph_points(elapsed)
        };
        let word_state = word_state(elapsed, reduced_motion);

        div()
            .id(self.id)
            .role(Role::Status)
            .aria_label("Thinking…")
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(8.0))
            .refine_style(&self.style)
            .when(self.show_icon, |indicator| {
                indicator.child(morphing_icon(points, icon_size, icon_color))
            })
            .child(
                div()
                    .relative()
                    .h(px(20.0))
                    .flex()
                    .items_center()
                    .text_size(px(text_size))
                    .text_color(icon_color)
                    .child(div().opacity(0.0).child(LONGEST_WORD))
                    .children(word_state.layers.into_iter().map(|layer| {
                        div()
                            .absolute()
                            .left_0()
                            .top(px(layer.offset_y * 20.0))
                            .opacity(layer.opacity)
                            .child(
                                ShimmerText::new(
                                    ("thinking-shimmer", word_index(layer.word)),
                                    layer.word,
                                )
                                .duration(SHIMMER_DURATION),
                            )
                    })),
            )
    }
}

fn animation_elapsed(view: EntityId, reduced_motion: bool, cx: &mut App) -> Duration {
    if reduced_motion {
        return Duration::ZERO;
    }
    let now = Instant::now();
    let (epoch, start_clock) = {
        let clock = cx.default_global::<ThinkingClock>();
        let epoch = *clock.epoch.get_or_insert(now);
        clock.leases.insert(view, now + CLOCK_LEASE);
        let start_clock = !clock.running;
        clock.running = true;
        (epoch, start_clock)
    };
    if start_clock {
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(CLOCK_TICK).await;
                let parked = cx.update(|cx| {
                    let clock = cx.default_global::<ThinkingClock>();
                    let now = Instant::now();
                    clock.leases.retain(|_, until| *until > now);
                    if clock.leases.is_empty() {
                        clock.running = false;
                        return true;
                    }
                    for view in clock.leases.keys().copied().collect::<Vec<_>>() {
                        cx.notify(view);
                    }
                    false
                });
                if parked {
                    break;
                }
            }
        })
        .detach();
    }
    now.duration_since(epoch)
}

fn morph_points(elapsed: Duration) -> PathPoints {
    let keyframe_ms = MORPH_DURATION.as_millis() / 4;
    let cycle_ms = elapsed.as_millis() % MORPH_DURATION.as_millis();
    let segment =
        usize::try_from(cycle_ms / keyframe_ms).expect("morph segment is always in the range 0..4");
    let progress_ms = u16::try_from(cycle_ms % keyframe_ms)
        .expect("morph keyframe duration fits into u16 milliseconds");
    let keyframe_ms =
        u16::try_from(keyframe_ms).expect("morph keyframe duration fits into u16 milliseconds");
    let progress = ease_in_out(f32::from(progress_ms) / f32::from(keyframe_ms));
    let keyframes = [CIRCLE_A, INFINITY, CIRCLE_B, INFINITY, CIRCLE_A];
    interpolate_points(&keyframes[segment], &keyframes[segment + 1], progress)
}

fn interpolate_points(from: &PathPoints, to: &PathPoints, progress: f32) -> PathPoints {
    std::array::from_fn(|index| {
        let (from_x, from_y) = from[index];
        let (to_x, to_y) = to[index];
        (
            from_x + (to_x - from_x) * progress,
            from_y + (to_y - from_y) * progress,
        )
    })
}

fn morphing_icon(points: PathPoints, size: f32, color: gpui::Hsla) -> AnyElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let scale = size / 24.0;
            let map = |(x, y): (f32, f32)| {
                point(bounds.left() + px(x * scale), bounds.top() + px(y * scale))
            };
            let mut path = PathBuilder::stroke(px(1.5));
            path.move_to(map(points[0]));
            for segment in 0..4 {
                let index = 1 + segment * 3;
                path.cubic_bezier_to(
                    map(points[index + 2]),
                    map(points[index]),
                    map(points[index + 1]),
                );
            }
            if let Ok(path) = path.build() {
                window.paint_path(path, color);
            }
        },
    )
    .size(px(size))
    .flex_none()
    .into_any_element()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WordLayer {
    word: &'static str,
    offset_y: f32,
    opacity: f32,
}

#[derive(Debug, PartialEq)]
struct WordState {
    layers: Vec<WordLayer>,
}

fn word_state(elapsed: Duration, reduced_motion: bool) -> WordState {
    if reduced_motion {
        return WordState {
            layers: vec![WordLayer {
                word: WORDS[0],
                offset_y: 0.0,
                opacity: 1.0,
            }],
        };
    }
    let duration_ms = WORD_DURATION.as_millis();
    let elapsed_ms = elapsed.as_millis();
    let word_count = u128::try_from(WORDS.len()).expect("word count fits into u128");
    let index = usize::try_from((elapsed_ms / duration_ms) % word_count)
        .expect("word index is always within the vocabulary");
    let local_ms =
        u16::try_from(elapsed_ms % duration_ms).expect("word duration fits into u16 milliseconds");
    let enter_ms = u16::try_from(WORD_ENTER_DURATION.as_millis())
        .expect("word enter duration fits into u16 milliseconds");
    let exit_ms = u16::try_from(WORD_EXIT_DURATION.as_millis())
        .expect("word exit duration fits into u16 milliseconds");
    if local_ms >= enter_ms {
        return WordState {
            layers: vec![WordLayer {
                word: WORDS[index],
                offset_y: 0.0,
                opacity: 1.0,
            }],
        };
    }
    let enter = ease_in_out(f32::from(local_ms) / f32::from(enter_ms));
    let exit = ease_in_out((f32::from(local_ms) / f32::from(exit_ms)).clamp(0.0, 1.0));
    let previous = (index + WORDS.len() - 1) % WORDS.len();
    WordState {
        layers: vec![
            WordLayer {
                word: WORDS[previous],
                offset_y: -0.8 * exit,
                opacity: 1.0 - exit,
            },
            WordLayer {
                word: WORDS[index],
                offset_y: 0.8 * (1.0 - enter),
                opacity: enter,
            },
        ],
    }
}

fn word_index(word: &'static str) -> usize {
    WORDS
        .iter()
        .position(|candidate| *candidate == word)
        .expect("thinking indicator layers only contain known words")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use gpui::{Context, Element as _, Render, Role, TestAppContext, WindowAppearance, accesskit};

    struct ThinkingIndicatorHarness;

    impl Render for ThinkingIndicatorHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            ThinkingIndicator::new("thinking-indicator-lifecycle")
        }
    }

    #[gpui::test]
    fn indicator_starts_its_clock_inside_the_gpui_render_lifecycle(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (_, window) = cx.add_window_view(|_, _| ThinkingIndicatorHarness);
        window.update(|window, cx| window.draw(cx).clear(cx));
    }

    #[gpui::test]
    fn indicator_exposes_a_stable_status_announcement(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<(Option<Role>, accesskit::Node)>>>;

        struct AccessibilityHarness {
            captured: Captured,
        }

        impl Render for AccessibilityHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.captured.clone();
                canvas(
                    move |_, window, cx| {
                        let element = ThinkingIndicator::new("thinking-indicator-accessibility")
                            .render(window, cx)
                            .into_element();
                        let role = element.a11y_role();
                        let mut node = accesskit::Node::new(Role::GenericContainer);
                        element.write_a11y_info(&mut node);
                        *captured.lock().unwrap() = Some((role, node));
                    },
                    |_, (), _, _| {},
                )
            }
        }

        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, window) = cx.add_window_view(move |_, _| AccessibilityHarness { captured });
        window.update(|window, cx| window.draw(cx).clear(cx));

        let (role, node) = result.lock().unwrap().take().unwrap();
        assert_eq!(role, Some(Role::Status));
        assert_eq!(node.label(), Some("Thinking…"));
    }

    #[test]
    fn dimensions_and_durations_match_the_shared_motion_contract() {
        assert_eq!(WORD_DURATION, Duration::from_secs(4));
        assert_eq!(WORD_ENTER_DURATION, Duration::from_millis(240));
        assert_eq!(WORD_EXIT_DURATION, Duration::from_millis(160));
        assert_eq!(MORPH_DURATION, Duration::from_secs(6));
        assert_eq!(SHIMMER_DURATION, Duration::from_millis(1_500));
        assert_eq!(CLOCK_TICK, Duration::from_nanos(16_666_667));
    }

    #[test]
    fn morph_hits_every_motion_keyframe() {
        assert_eq!(morph_points(Duration::ZERO), CIRCLE_A);
        assert_eq!(morph_points(Duration::from_millis(1_500)), INFINITY);
        assert_eq!(morph_points(Duration::from_secs(3)), CIRCLE_B);
        assert_eq!(morph_points(Duration::from_millis(4_500)), INFINITY);
    }

    #[test]
    fn reduced_motion_is_static_and_semantically_stable() {
        let state = word_state(Duration::from_secs(20), true);
        assert_eq!(state.layers.len(), 1);
        assert_eq!(state.layers[0].word, "Thinking");
        assert!((state.layers[0].opacity - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn words_rotate_every_four_seconds() {
        assert_eq!(
            word_state(Duration::from_millis(300), false).layers[0].word,
            "Thinking"
        );
        assert_eq!(
            word_state(Duration::from_millis(4_300), false).layers[0].word,
            "Moonwalking"
        );
        assert_eq!(
            word_state(Duration::from_millis(8_300), false).layers[0].word,
            "Planning"
        );
    }
}
