//! GPU-rendered text shimmer matching shadcn's shimmer utility contract.

use std::{
    cell::RefCell,
    f32::consts::PI,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    Animation, AnimationExt as _, App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId,
    Hsla, InspectorElementId, IntoElement, LayoutId, ParentElement as _, Pixels, RenderOnce,
    ShapedLine, SharedString, Size, Style, StyleRefinement, Styled, TextAlign, TextShimmer,
    TruncateFrom, Window, WindowAppearance, div, px,
};
use gpui_base::StyledExt as _;

const DEFAULT_DURATION: Duration = Duration::from_secs(2);
const DEFAULT_ANGLE_DEGREES: f32 = 20.0;
const DEFAULT_MAX_FPS: f32 = 60.0;
const DEFAULT_EXTRA_SPREAD: Pixels = px(40.0);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShimmerDirection {
    #[default]
    Forward,
    Reverse,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShimmerPlayback {
    #[default]
    Repeat,
    Once,
    FrameDriven {
        started_at: Instant,
    },
    Static,
}

#[derive(IntoElement)]
#[must_use]
pub struct ShimmerText {
    id: ElementId,
    text: SharedString,
    duration: Duration,
    direction: ShimmerDirection,
    playback: ShimmerPlayback,
    spread: Option<Pixels>,
    angle_radians: f32,
    highlight_color: Option<Hsla>,
    truncate: bool,
    style: StyleRefinement,
}

impl ShimmerText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        Self {
            id: id.into(),
            text: single_line_text(&text),
            duration: DEFAULT_DURATION,
            direction: ShimmerDirection::Forward,
            playback: ShimmerPlayback::Repeat,
            spread: None,
            angle_radians: DEFAULT_ANGLE_DEGREES.to_radians(),
            highlight_color: None,
            truncate: false,
            style: StyleRefinement::default(),
        }
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn direction(mut self, direction: ShimmerDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn playback(mut self, playback: ShimmerPlayback) -> Self {
        self.playback = playback;
        self
    }

    pub fn spread(mut self, spread: Pixels) -> Self {
        self.spread = Some(spread);
        self
    }

    pub fn angle_degrees(mut self, angle_degrees: f32) -> Self {
        self.angle_radians = angle_degrees * PI / 180.0;
        self
    }

    pub fn highlight_color(mut self, highlight_color: Hsla) -> Self {
        self.highlight_color = Some(highlight_color);
        self
    }

    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }
}

fn single_line_text(text: &str) -> SharedString {
    text.split_whitespace().collect::<Vec<_>>().join(" ").into()
}

impl Styled for ShimmerText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ShimmerText {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let frame_driven = match self.playback {
            ShimmerPlayback::FrameDriven { started_at } if !cx.reduce_motion() => {
                Some(FrameDrivenPhase {
                    started_at,
                    duration: self.duration,
                })
            }
            _ => None,
        };
        let glyphs = ShimmerGlyphs {
            text: self.text,
            phase: static_phase(self.direction),
            frame_driven,
            direction: self.direction,
            spread: self.spread,
            angle_radians: self.angle_radians,
            highlight_color: self.highlight_color,
            truncate: self.truncate,
        };
        let content = match self.playback {
            ShimmerPlayback::Repeat => glyphs
                .with_animation(
                    self.id.clone(),
                    Animation::new(self.duration)
                        .repeat_synced()
                        .with_max_fps(DEFAULT_MAX_FPS),
                    |mut glyphs, phase| {
                        glyphs.phase = directed_phase(phase, glyphs.direction);
                        glyphs
                    },
                )
                .into_any_element(),
            ShimmerPlayback::Once => glyphs
                .with_animation(
                    self.id.clone(),
                    Animation::new(self.duration).with_max_fps(DEFAULT_MAX_FPS),
                    |mut glyphs, phase| {
                        glyphs.phase = directed_phase(phase, glyphs.direction);
                        glyphs
                    },
                )
                .into_any_element(),
            ShimmerPlayback::FrameDriven { .. } | ShimmerPlayback::Static => {
                glyphs.into_any_element()
            }
        };

        div().refine_style(&self.style).child(content)
    }
}

#[derive(Clone)]
struct ShimmerGlyphs {
    text: SharedString,
    phase: f32,
    frame_driven: Option<FrameDrivenPhase>,
    direction: ShimmerDirection,
    spread: Option<Pixels>,
    angle_radians: f32,
    highlight_color: Option<Hsla>,
    truncate: bool,
}

#[derive(Clone, Copy)]
struct FrameDrivenPhase {
    started_at: Instant,
    duration: Duration,
}

impl IntoElement for ShimmerGlyphs {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

#[derive(Clone, Default)]
struct ShimmerLayout(Rc<RefCell<Option<ShimmerLayoutState>>>);

#[derive(Clone)]
struct ShimmerLayoutState {
    line: ShapedLine,
    line_height: Pixels,
    spread: Pixels,
    base_color: Hsla,
    highlight_color: Hsla,
}

impl Element for ShimmerGlyphs {
    type RequestLayoutState = ShimmerLayout;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line_height = window.pixel_snap(
            text_style
                .line_height
                .to_pixels(font_size.into(), window.rem_size()),
        );
        let text = self.text.clone();
        let truncate = self.truncate;
        let spread = self.spread;
        let base_color = text_style.color;
        let highlight_color = self
            .highlight_color
            .unwrap_or_else(|| default_highlight_color(base_color, window.appearance()));
        let state = ShimmerLayout::default();
        let measured_state = state.clone();
        let layout_id = window.request_measured_layout(
            Style::default(),
            move |known, available, window, cx| {
                let truncate_width = truncate
                    .then(|| {
                        known.width.or(match available.width {
                            AvailableSpace::Definite(width) => Some(width),
                            AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
                        })
                    })
                    .flatten();
                let initial_run = text_style.to_run(text.len());
                let (text, runs) = if let Some(width) = truncate_width {
                    let mut wrapper = cx.text_system().line_wrapper(text_style.font(), font_size);
                    let (text, runs) = wrapper.truncate_line(
                        text.clone(),
                        width,
                        "…",
                        std::slice::from_ref(&initial_run),
                        TruncateFrom::End,
                    );
                    (text, runs.into_owned())
                } else {
                    (text.clone(), vec![initial_run])
                };
                let line = window
                    .text_system()
                    .shape_line(text, font_size, &runs, None);
                let resolved_spread = spread.unwrap_or_else(|| {
                    let zeroes: SharedString = "000".into();
                    let zeroes_run = text_style.to_run(zeroes.len());
                    window
                        .text_system()
                        .shape_line(zeroes, font_size, std::slice::from_ref(&zeroes_run), None)
                        .width()
                        + DEFAULT_EXTRA_SPREAD
                });
                let size = Size::new(line.width().ceil(), line_height);
                measured_state.0.borrow_mut().replace(ShimmerLayoutState {
                    line,
                    line_height,
                    spread: resolved_spread,
                    base_color,
                    highlight_color,
                });
                size
            },
        );
        (layout_id, state)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        (): &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = layout.0.borrow();
        let state = state
            .as_ref()
            .expect("ShimmerText must be measured before it is painted");
        let text_align = window.text_style().text_align;
        let text_origin = gpui::point(
            aligned_x(bounds, state.line.width(), text_align),
            bounds.origin.y,
        );
        let text_bounds = Bounds::new(
            text_origin,
            Size::new(state.line.width(), state.line_height),
        );
        let phase = self.frame_driven.map_or(self.phase, |frame_driven| {
            repeating_phase(
                frame_driven.started_at.elapsed(),
                frame_driven.duration,
                self.direction,
            )
        });
        state
            .line
            .paint_shimmer(
                bounds.origin,
                state.line_height,
                text_align,
                Some(bounds.size.width),
                TextShimmer {
                    bounds: text_bounds,
                    color: state.base_color,
                    highlight_color: state.highlight_color,
                    phase,
                    spread: state.spread,
                    angle: self.angle_radians,
                },
                window,
                cx,
            )
            .expect("ShimmerText glyph painting must succeed");
    }
}

fn aligned_x(bounds: Bounds<Pixels>, line_width: Pixels, text_align: TextAlign) -> Pixels {
    match text_align {
        TextAlign::Left => bounds.origin.x,
        TextAlign::Center => bounds.origin.x + (bounds.size.width - line_width) / 2.0,
        TextAlign::Right => bounds.origin.x + bounds.size.width - line_width,
    }
}

fn directed_phase(phase: f32, direction: ShimmerDirection) -> f32 {
    match direction {
        ShimmerDirection::Forward => phase,
        ShimmerDirection::Reverse => 1.0 - phase,
    }
}

fn static_phase(direction: ShimmerDirection) -> f32 {
    directed_phase(0.0, direction)
}

fn repeating_phase(elapsed: Duration, duration: Duration, direction: ShimmerDirection) -> f32 {
    if duration.is_zero() {
        return static_phase(direction);
    }
    let cycle_nanoseconds = elapsed.as_nanos() % duration.as_nanos();
    let cycle_seconds = u64::try_from(cycle_nanoseconds / 1_000_000_000)
        .expect("shimmer cycle seconds must fit in Duration");
    let subsec_nanoseconds = u32::try_from(cycle_nanoseconds % 1_000_000_000)
        .expect("shimmer cycle subsecond nanoseconds must fit in Duration");
    let elapsed = Duration::new(cycle_seconds, subsec_nanoseconds);
    directed_phase(elapsed.as_secs_f32() / duration.as_secs_f32(), direction)
}

fn default_highlight_color(base: Hsla, appearance: WindowAppearance) -> Hsla {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => Hsla {
            a: base.a * 0.2,
            ..base
        },
        WindowAppearance::Dark | WindowAppearance::VibrantDark => Hsla {
            l: (base.l + 0.4).clamp(0.8, 1.0),
            a: (base.a + 0.4).min(1.0),
            ..base
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Context, Render, TestAppContext, hsla};

    struct ShimmerHarness;

    impl Render for ShimmerHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            ShimmerText::new("shimmer-test", "cargo test\ncargo check")
        }
    }

    #[gpui::test]
    fn multiline_shimmer_paints_through_the_gpui_render_lifecycle(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(WindowAppearance::Light, cx);
        });
        let (_, window) = cx.add_window_view(|_, _| ShimmerHarness);
        window.update(|window, cx| window.draw(cx).clear(cx));
    }

    #[test]
    fn defaults_match_shadcn_shimmer() {
        assert_eq!(DEFAULT_DURATION, Duration::from_secs(2));
        assert!((DEFAULT_ANGLE_DEGREES - 20.0).abs() < f32::EPSILON);
        assert!((DEFAULT_MAX_FPS - 60.0).abs() < f32::EPSILON);
        assert_eq!(DEFAULT_EXTRA_SPREAD, px(40.0));
    }

    #[test]
    fn multiline_text_is_normalized_for_single_line_shaping() {
        assert_eq!(
            single_line_text("cargo test\r\ncargo check").as_ref(),
            "cargo test cargo check"
        );
    }

    #[test]
    fn frame_driven_phase_tracks_elapsed_time_without_scheduling_frames() {
        assert!(
            (repeating_phase(
                Duration::from_millis(500),
                Duration::from_secs(2),
                ShimmerDirection::Forward,
            ) - 0.25)
                .abs()
                < f32::EPSILON
        );
        assert!(
            (repeating_phase(
                Duration::from_millis(500),
                Duration::from_secs(2),
                ShimmerDirection::Reverse,
            ) - 0.75)
                .abs()
                < f32::EPSILON
        );
        assert!(
            repeating_phase(
                Duration::from_secs(1),
                Duration::ZERO,
                ShimmerDirection::Forward,
            )
            .abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn highlight_palette_matches_light_and_dark_contracts() {
        let base = hsla(0.1, 0.2, 0.3, 0.5);
        let light = default_highlight_color(base, WindowAppearance::Light);
        let dark = default_highlight_color(base, WindowAppearance::Dark);
        assert!((light.a - 0.1).abs() < f32::EPSILON);
        assert!((dark.l - 0.8).abs() < f32::EPSILON);
        assert!((dark.a - 0.9).abs() < f32::EPSILON);
    }
}
