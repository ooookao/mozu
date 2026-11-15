use crate::ui::utils::hover::{
    HoverAxis, HoverHighlight, HoverItem, HoverMotion, HoverSnapshot, HoverTarget,
};
use crate::ui::utils::motion::{MotionProfile, TimedValue};
use crate::ui::utils::presentation::{ComponentTheme, FontWeightStyle, ThemeScheme};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Corners, ElementId, Hsla, IntoElement, Pixels, Point, RenderOnce,
    SharedString, SpringConfig, SpringState, StyleTransition, Window, div, point, px,
};
use kinu_ui::ColorScheme;
use std::time::{Duration, Instant};

/// Reserves the semibold label width while animating only visible type and color.
pub fn choice_label(
    label: SharedString,
    selected: bool,
    active: bool,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let metrics = tokens.size.metrics(&tokens);
    let mut font = gpui::font(tokens.typography.family.clone());
    font.variations = tokens.typography.semibold.variations();
    let font_id = cx.text_system().resolve_font(&font);
    let baseline =
        cx.text_system()
            .baseline_offset(font_id, px(metrics.text_size), px(metrics.line_height));
    let cap = cx.text_system().cap_height(font_id, px(metrics.text_size));
    let colors = tokens.extras(if ColorScheme::current(window, cx) == ColorScheme::Dark {
        ThemeScheme::Dark
    } else {
        ThemeScheme::Light
    });
    let weight: FontWeightStyle = if selected {
        tokens.typography.semibold
    } else {
        tokens.typography.normal
    };
    let text = |label: SharedString| {
        div()
            .text_size(px(metrics.text_size))
            .line_height(px(metrics.line_height))
            .child(label)
    };
    div()
        .relative()
        .min_w_0()
        .flex_1()
        .mt(-(baseline - cap))
        .mb(-(px(metrics.line_height) - baseline))
        .child(
            text(label.clone())
                .font_variations(tokens.typography.semibold.variations())
                .invisible(),
        )
        .child(
            text(label)
                .id("visible-label")
                .absolute()
                .inset_0()
                .font_variations(weight.variations())
                .text_color(if selected || active {
                    gpui::Hsla::from(colors.foreground)
                } else {
                    gpui::Hsla::from(colors.muted_foreground)
                })
                .transition(StyleTransition::typography(tokens.motion.fast.entering())),
        )
        .into_any_element()
}

/// Measured local rectangles; selection and input remain owned by the primitive.
#[derive(Clone, Copy)]
pub struct SelectionRow {
    pub bounds: Option<Bounds<Pixels>>,
    pub selected: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Run {
    start: usize,
    end: usize,
    id: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Converge,
    Commit,
    SplitIn,
    Diverge,
}

struct Boundary {
    survivor: u64,
    other: u64,
    gap: usize,
    phase: Phase,
    started: Instant,
    frame_started: Instant,
}

#[derive(Clone)]
pub struct SelectionBlock {
    pub id: u64,
    pub bounds: Bounds<Pixels>,
    pub radii: Corners<Pixels>,
    pub instant: bool,
    pub exit_instant: bool,
    pub corner_delay: Duration,
    pub opacity: f32,
    pub enter_from: Option<(Bounds<Pixels>, Corners<Pixels>)>,
}

/// Stable contiguous runs and the transient halves of a one-row merge or split.
pub struct SelectionBlocks {
    runs: Vec<Run>,
    boundaries: Vec<Boundary>,
    last_id: u64,
}

fn bridge(outer: Run, runs: &[Run]) -> Option<(Run, Run, usize)> {
    let mut inside = runs
        .iter()
        .copied()
        .filter(|r| r.start >= outer.start && r.end <= outer.end);
    let upper = inside.next()?;
    let lower = inside.next()?;
    if inside.next().is_none() && lower.start == upper.end + 2 {
        Some((upper, lower, upper.end + 1))
    } else {
        None
    }
}

fn corners(top: Pixels, bottom: Pixels) -> Corners<Pixels> {
    Corners {
        top_left: top,
        top_right: top,
        bottom_right: bottom,
        bottom_left: bottom,
    }
}

impl SelectionBlocks {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            boundaries: Vec::new(),
            last_id: 0,
        }
    }

    fn snap(
        &mut self,
        rows: &[SelectionRow],
        radius: Pixels,
        now: Instant,
        tokens: &ComponentTheme,
    ) -> Vec<SelectionBlock> {
        self.update(rows, radius, now, tokens);
        self.boundaries.clear();
        self.update(rows, radius, now, tokens).0
    }

    /// Returns target blocks and whether another frame is needed for boundary choreography.
    pub fn update(
        &mut self,
        rows: &[SelectionRow],
        radius: Pixels,
        now: Instant,
        tokens: &ComponentTheme,
    ) -> (Vec<SelectionBlock>, bool) {
        let mut runs: Vec<Run> = Vec::new();
        for (index, _) in rows.iter().enumerate().filter(|(_, row)| row.selected) {
            if let Some(last) = runs.last_mut().filter(|last| last.end + 1 == index) {
                last.end = index;
            } else {
                runs.push(Run {
                    start: index,
                    end: index,
                    id: 0,
                });
            }
        }
        for index in 0..runs.len() {
            let run = runs[index];
            let prior = self.runs.iter().find(|old| {
                old.start <= run.end
                    && old.end >= run.start
                    && !runs[..index].iter().any(|used| used.id == old.id)
            });
            runs[index].id = match prior {
                Some(old) => old.id,
                None => {
                    self.last_id = self
                        .last_id
                        .checked_add(1)
                        .expect("selection block id overflow");
                    self.last_id
                }
            };
        }
        if runs != self.runs {
            self.boundaries.retain(|b| match b.phase {
                Phase::Converge | Phase::Commit => runs
                    .iter()
                    .any(|r| r.id == b.survivor && b.gap > r.start && b.gap < r.end),
                Phase::SplitIn | Phase::Diverge => {
                    runs.iter()
                        .any(|r| r.id == b.survivor && r.end + 1 == b.gap)
                        && runs.iter().any(|r| r.id == b.other && r.start == b.gap + 1)
                }
            });
            for run in &runs {
                if let Some((upper, lower, gap)) = bridge(*run, &self.runs) {
                    self.boundaries.push(Boundary {
                        survivor: run.id,
                        other: if run.id == upper.id {
                            lower.id
                        } else {
                            upper.id
                        },
                        gap,
                        phase: Phase::Converge,
                        started: now,
                        frame_started: now,
                    });
                }
            }
            for old in &self.runs {
                if let Some((upper, lower, gap)) = bridge(*old, &runs) {
                    self.boundaries.push(Boundary {
                        survivor: upper.id,
                        other: lower.id,
                        gap,
                        phase: Phase::SplitIn,
                        started: now,
                        frame_started: now,
                    });
                }
            }
            self.runs = runs;
        }
        self.boundaries.retain_mut(|b| {
            match b.phase {
                Phase::Commit if now > b.frame_started => return false,
                Phase::SplitIn if now > b.frame_started => b.phase = Phase::Diverge,
                Phase::Converge
                    if now.duration_since(b.started)
                        >= Duration::from_millis(tokens.settings.timings.group_converge_ms) =>
                {
                    b.phase = Phase::Commit;
                    b.frame_started = now;
                }
                _ => {}
            }
            !matches!(b.phase, Phase::Diverge)
                || now.duration_since(b.started)
                    < Duration::from_millis(tokens.settings.timings.group_diverge_ms)
        });
        let mut blocks = self
            .runs
            .iter()
            .filter_map(|run| {
                let first = rows.get(run.start)?.bounds?;
                let last = rows.get(run.end)?.bounds?;
                Some(SelectionBlock {
                    id: run.id,
                    bounds: Bounds::new(
                        point(first.left().min(last.left()), first.top()),
                        gpui::size(
                            first.size.width.max(last.size.width),
                            last.bottom() - first.top(),
                        ),
                    ),
                    radii: corners(radius, radius),
                    instant: false,
                    exit_instant: false,
                    corner_delay: Duration::ZERO,
                    opacity: 1.,
                    enter_from: None,
                })
            })
            .collect::<Vec<_>>();
        for boundary in &self.boundaries {
            let Some(gap) = rows.get(boundary.gap).and_then(|row| row.bounds) else {
                continue;
            };
            let Some(index) = blocks
                .iter()
                .position(|block| block.id == boundary.survivor)
            else {
                continue;
            };
            let seam = gap.top() + gap.size.height / 2.;
            if matches!(boundary.phase, Phase::Converge | Phase::Commit) {
                let survivor = &mut blocks[index];
                let lower_bounds = Bounds::new(
                    point(survivor.bounds.left(), seam),
                    gpui::size(survivor.bounds.size.width, survivor.bounds.bottom() - seam),
                );
                let commit = boundary.phase == Phase::Commit;
                let timing = tokens.settings.timings;
                let delay = Duration::from_secs_f32(
                    (timing.corner_delay_min_ms as f32
                        + f32::from(seam) / f32::from(gap.size.height).max(1.)
                            * timing.corner_delay_distance_ms)
                        .clamp(
                            timing.corner_delay_min_ms as f32,
                            timing.corner_delay_max_ms as f32,
                        )
                        / 1000.,
                );
                survivor.instant = commit;
                if !commit {
                    survivor.bounds.size.height = seam - survivor.bounds.top();
                    survivor.radii = corners(radius, px(0.));
                    survivor.corner_delay = delay;
                }
                blocks.push(SelectionBlock {
                    id: boundary.other,
                    bounds: lower_bounds,
                    radii: corners(px(0.), radius),
                    instant: commit,
                    exit_instant: true,
                    corner_delay: if commit { Duration::ZERO } else { delay },
                    opacity: if commit { 0. } else { 1. },
                    enter_from: Some((lower_bounds, corners(radius, radius))),
                });
            } else if boundary.phase == Phase::SplitIn {
                let Some(lower_index) = blocks.iter().position(|block| block.id == boundary.other)
                else {
                    continue;
                };
                let lower_bottom = blocks[lower_index].bounds.bottom();
                blocks[index].bounds.size.height = seam - blocks[index].bounds.top();
                blocks[index].radii = corners(radius, px(0.));
                blocks[index].instant = true;
                let lower = &mut blocks[lower_index];
                lower.bounds.origin.y = seam;
                lower.bounds.size.height = lower_bottom - seam;
                lower.radii = corners(px(0.), radius);
                lower.instant = true;
                lower.enter_from = Some((lower.bounds, lower.radii.clone()));
            }
        }
        (blocks, !self.boundaries.is_empty())
    }
}

struct ScalarSpring {
    state: SpringState,
    target: f32,
    updated: Instant,
    resume: Instant,
}
impl ScalarSpring {
    fn new(value: f32, now: Instant) -> Self {
        Self {
            state: SpringState {
                position: value,
                velocity: 0.,
            },
            target: value,
            updated: now,
            resume: now,
        }
    }
    fn sample(&mut self, now: Instant, config: SpringConfig) -> f32 {
        if now > self.resume {
            self.state = config.step(
                self.state,
                self.target,
                now.duration_since(self.updated.max(self.resume))
                    .as_secs_f32(),
            );
        }
        self.updated = now;
        self.state.position
    }
    fn target(
        &mut self,
        value: f32,
        instant: bool,
        delay: Duration,
        now: Instant,
        config: SpringConfig,
    ) {
        self.sample(now, config);
        if instant {
            *self = Self::new(value, now);
        } else if value != self.target {
            self.target = value;
            self.resume = now + delay;
        }
    }
    fn settled(&self, config: SpringConfig) -> bool {
        config.is_settled(self.state, self.target, 0.001)
    }
}

struct AnimatedBlock {
    id: u64,
    x: ScalarSpring,
    y: ScalarSpring,
    width: ScalarSpring,
    height: ScalarSpring,
    top_left: ScalarSpring,
    top_right: ScalarSpring,
    bottom_right: ScalarSpring,
    bottom_left: ScalarSpring,
    opacity: TimedValue,
    opacity_target: f32,
    exit_instant: bool,
    removed: bool,
}
impl AnimatedBlock {
    fn snap_geometry(&mut self, now: Instant) {
        for spring in [
            &mut self.x,
            &mut self.y,
            &mut self.width,
            &mut self.height,
            &mut self.top_left,
            &mut self.top_right,
            &mut self.bottom_right,
            &mut self.bottom_left,
        ] {
            *spring = ScalarSpring::new(spring.target, now);
        }
    }
    fn new(block: &SelectionBlock, now: Instant) -> Self {
        let (bounds, radii) = block
            .enter_from
            .clone()
            .unwrap_or((block.bounds, block.radii.clone()));
        Self {
            id: block.id,
            x: ScalarSpring::new(f32::from(bounds.origin.x), now),
            y: ScalarSpring::new(f32::from(bounds.origin.y), now),
            width: ScalarSpring::new(f32::from(bounds.size.width), now),
            height: ScalarSpring::new(f32::from(bounds.size.height), now),
            top_left: ScalarSpring::new(f32::from(radii.top_left), now),
            top_right: ScalarSpring::new(f32::from(radii.top_right), now),
            bottom_right: ScalarSpring::new(f32::from(radii.bottom_right), now),
            bottom_left: ScalarSpring::new(f32::from(radii.bottom_left), now),
            opacity: TimedValue::new(block.opacity, now),
            opacity_target: block.opacity,
            exit_instant: block.exit_instant,
            removed: false,
        }
    }
    fn target(
        &mut self,
        block: &SelectionBlock,
        now: Instant,
        motion: MotionProfile,
        opacity_duration: Duration,
    ) {
        self.removed = false;
        self.exit_instant = block.exit_instant;
        for (spring, target) in [
            (&mut self.x, block.bounds.origin.x),
            (&mut self.y, block.bounds.origin.y),
            (&mut self.width, block.bounds.size.width),
            (&mut self.height, block.bounds.size.height),
        ] {
            spring.target(
                f32::from(target),
                block.instant,
                Duration::ZERO,
                now,
                motion.spring,
            );
        }
        for (spring, target) in [
            (&mut self.top_left, block.radii.top_left),
            (&mut self.top_right, block.radii.top_right),
            (&mut self.bottom_right, block.radii.bottom_right),
            (&mut self.bottom_left, block.radii.bottom_left),
        ] {
            spring.target(
                f32::from(target),
                block.instant,
                block.corner_delay,
                now,
                motion.spring,
            );
        }
        if block.opacity != self.opacity_target {
            self.opacity.retarget(
                block.opacity,
                now,
                if block.instant {
                    Duration::ZERO
                } else {
                    opacity_duration
                },
                gpui::linear,
            );
            self.opacity_target = block.opacity;
        }
    }
}

struct SelectionAnimation {
    model: SelectionBlocks,
    blocks: Vec<AnimatedBlock>,
}

#[derive(IntoElement)]
pub struct SelectionBackgrounds {
    id: ElementId,
    rows: Vec<SelectionRow>,
    radius: Pixels,
    color: Hsla,
    motion: MotionProfile,
    tokens: ComponentTheme,
}
impl SelectionBackgrounds {
    pub fn new(
        id: impl Into<ElementId>,
        rows: Vec<SelectionRow>,
        radius: Pixels,
        color: Hsla,
        motion: MotionProfile,
        tokens: ComponentTheme,
    ) -> Self {
        Self {
            id: id.into(),
            tokens,
            rows,
            radius,
            color,
            motion,
        }
    }
}
impl RenderOnce for SelectionBackgrounds {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let now = cx.background_executor().now();
        let reduced = cx.reduce_motion();
        let state = window.use_keyed_state((self.id.clone(), "selection-blocks"), cx, |_, _| {
            SelectionAnimation {
                model: SelectionBlocks::new(),
                blocks: Vec::new(),
            }
        });
        let (elements, pending) = state.update(cx, |state, _| {
            let (targets, mut pending) = if reduced {
                (
                    state.model.snap(&self.rows, self.radius, now, &self.tokens),
                    false,
                )
            } else {
                state
                    .model
                    .update(&self.rows, self.radius, now, &self.tokens)
            };
            for target in &targets {
                if let Some(block) = state.blocks.iter_mut().find(|block| block.id == target.id) {
                    block.target(
                        target,
                        now,
                        self.motion,
                        Duration::from_millis(self.tokens.motion.animations.color.duration_ms),
                    );
                } else {
                    let mut block = AnimatedBlock::new(target, now);
                    block.target(
                        target,
                        now,
                        self.motion,
                        Duration::from_millis(self.tokens.motion.animations.color.duration_ms),
                    );
                    state.blocks.push(block);
                }
            }
            if reduced {
                for block in &mut state.blocks {
                    block.snap_geometry(now);
                }
            }
            state.blocks.retain_mut(|block| {
                if targets.iter().any(|target| target.id == block.id) {
                    return true;
                }
                if block.exit_instant {
                    return false;
                }
                if !block.removed {
                    block.removed = true;
                    block.opacity_target = 0.;
                    block
                        .opacity
                        .retarget(0., now, self.motion.exit, gpui::linear);
                }
                !block.opacity.sample(now, gpui::linear).1
            });
            let elements = state
                .blocks
                .iter_mut()
                .map(|block| {
                    let x = block.x.sample(now, self.motion.spring);
                    let y = block.y.sample(now, self.motion.spring);
                    let width = block.width.sample(now, self.motion.spring);
                    let height = block.height.sample(now, self.motion.spring);
                    let tl = block.top_left.sample(now, self.motion.spring);
                    let tr = block.top_right.sample(now, self.motion.spring);
                    let br = block.bottom_right.sample(now, self.motion.spring);
                    let bl = block.bottom_left.sample(now, self.motion.spring);
                    let (opacity, faded) = block.opacity.sample(now, gpui::linear);
                    pending |= !faded
                        || [
                            &block.x,
                            &block.y,
                            &block.width,
                            &block.height,
                            &block.top_left,
                            &block.top_right,
                            &block.bottom_right,
                            &block.bottom_left,
                        ]
                        .iter()
                        .any(|spring| !spring.settled(self.motion.spring));
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(y))
                        .w(px(width))
                        .h(px(height))
                        .bg(self.color)
                        .opacity(opacity)
                        .rounded_tl(px(tl))
                        .rounded_tr(px(tr))
                        .rounded_br(px(br))
                        .rounded_bl(px(bl))
                        .into_any_element()
                })
                .collect::<Vec<_>>();
            (elements, pending)
        });
        if pending {
            window.request_animation_frame();
        }
        div().id(self.id).absolute().inset_0().children(elements)
    }
}

/// Measured visual facts shared by editable selection presentations.
#[derive(Clone)]
pub struct SelectionListRow {
    pub id: ElementId,
    pub bounds: Option<Bounds<Pixels>>,
    pub enabled: bool,
    pub selected: bool,
}
pub struct SelectionListFacts {
    pub rows: Vec<SelectionListRow>,
    pub decorative_active_id: Option<ElementId>,
    pub keyboard_focused: Option<ElementId>,
    pub keyboard_origin: bool,
    pub viewport_bounds: Bounds<Pixels>,
    pub open: bool,
    pub open_session: u64,
}
struct SelectionLayerState {
    session: u64,
    hover: HoverMotion,
    selected: HoverSnapshot,
    focus: HoverSnapshot,
}
impl SelectionLayerState {
    fn new() -> Self {
        Self {
            session: 0,
            hover: HoverMotion::new(),
            selected: HoverSnapshot {
                active: None,
                bounds: None,
                session: 0,
            },
            focus: HoverSnapshot {
                active: None,
                bounds: None,
                session: 0,
            },
        }
    }
}
fn local(snapshot: &HoverSnapshot, origin: Point<Pixels>) -> HoverSnapshot {
    let mut result = snapshot.clone();
    result.bounds = result
        .bounds
        .map(|bounds| Bounds::new(bounds.origin - origin, bounds.size));
    result
}

pub fn selection_background(
    id: &ElementId,
    facts: &SelectionListFacts,
    tokens: ComponentTheme,
    scheme: ThemeScheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let colors = tokens.extras(scheme);

    let runtime = window.use_keyed_state((id.clone(), "flow-select-layers"), cx, |_, _| {
        SelectionLayerState::new()
    });
    let (hover, selected) = runtime.update(cx, |state, _| {
        if state.session != facts.open_session {
            *state = SelectionLayerState::new();
            state.session = facts.open_session;
        }
        if !facts.open {
            state.hover.clear();
            state.selected.active = None;
        }
        let target = facts
            .decorative_active_id
            .as_ref()
            .and_then(|id| facts.rows.iter().find(|row| &row.id == id))
            .and_then(|row| row.bounds);
        if let Some(bounds) = target {
            state.hover.update(
                bounds.center(),
                std::iter::once(HoverItem {
                    id: facts
                        .decorative_active_id
                        .clone()
                        .expect("Measured decorative target has an ID"),
                    bounds,
                    enabled: true,
                }),
                HoverAxis::Vertical,
                HoverTarget::Containing,
            );
        } else {
            state.hover.clear();
        }
        let selected = facts.rows.iter().find(|row| row.selected);
        state.selected.active = selected.map(|row| row.id.clone());
        if let Some(bounds) = selected.and_then(|row| row.bounds) {
            state.selected.bounds = Some(bounds);
        }
        state.selected.session = facts.open_session as usize;
        state.focus.active = if facts.keyboard_origin {
            facts.keyboard_focused.clone()
        } else {
            None
        };
        if let Some(bounds) = state
            .focus
            .active
            .as_ref()
            .and_then(|id| facts.rows.iter().find(|row| &row.id == id))
            .and_then(|row| row.bounds)
        {
            state.focus.bounds = Some(Bounds::new(
                bounds.origin - gpui::point(px(2.), px(2.)),
                bounds.size + gpui::size(px(4.), px(4.)),
            ));
        }
        state.focus.session = facts.open_session as usize;
        (
            local(&state.hover.snapshot(), facts.viewport_bounds.origin),
            local(&state.selected, facts.viewport_bounds.origin),
        )
    });
    if !facts.open {
        return gpui::Empty.into_any_element();
    }
    let mut selected_motion = tokens.motion.moderate;
    selected_motion.enter = Duration::from_millis(tokens.motion.animations.color.duration_ms);
    div()
        .relative()
        .size_full()
        .child(
            HoverHighlight::new(
                "selection",
                selected,
                colors.active(),
                px(tokens.shape.popup().radii(&tokens).item),
                selected_motion,
            )
            .initial_opacity(1.),
        )
        .child(
            HoverHighlight::new(
                "hover",
                hover,
                colors.hover(),
                px(tokens.shape.popup().radii(&tokens).item),
                tokens.motion.fast,
            )
            .initial_opacity(0.),
        )
        .into_any_element()
}
pub fn selection_focus(
    id: &ElementId,
    facts: &SelectionListFacts,
    tokens: ComponentTheme,
    scheme: ThemeScheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let colors = tokens.extras(scheme);

    if !facts.open {
        return gpui::Empty.into_any_element();
    }
    let runtime = window.use_keyed_state((id.clone(), "flow-select-layers"), cx, |_, _| {
        SelectionLayerState::new()
    });
    let focus = local(&runtime.read(cx).focus, facts.viewport_bounds.origin);
    HoverHighlight::new(
        "focus",
        focus,
        gpui::transparent_black(),
        px(tokens.shape.popup().radii(&tokens).focus),
        tokens.motion.fast,
    )
    .outline(px(1.), gpui::Hsla::from(colors.brand))
    .initial_opacity(1.)
    .into_any_element()
}
