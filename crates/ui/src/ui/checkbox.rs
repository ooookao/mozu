use crate::ui::utils::presentation::{ComponentTheme, ControlSize, ThemeScheme};
use gpui::prelude::*;
use gpui::{
    Animation, AnimationExt, App, Bounds, ElementId, FocusHandle, Hsla, IntoElement, PathBuilder,
    Pixels, Point, Refineable, RenderOnce, StyleRefinement, StyleTransition, Styled, Window,
    canvas, div, point, px,
};
use kinu_ui::{CheckState, ColorScheme, FieldControl, FieldControlBinding, FieldLabelBehavior};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

fn check_vertices(progress: f32) -> Vec<Point<Pixels>> {
    let start = point(px(6.), px(12.));
    let elbow = point(px(10.), px(16.));
    let end = point(px(18.), px(8.));
    let first_length = 32_f32.sqrt();
    let total_length = first_length + 128_f32.sqrt();
    let distance = progress.clamp(0., 1.) * total_length;
    if distance <= first_length {
        vec![start, start + (elbow - start) * (distance / first_length)]
    } else {
        vec![
            start,
            elbow,
            elbow + (end - elbow) * ((distance - first_length) / (total_length - first_length)),
        ]
    }
}

fn mark(progress: f32, mixed: bool, color: Hsla, size: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds: Bounds<Pixels>, _, window, _| {
            if progress <= 0. {
                return;
            }
            let scale = size / 24.;
            let vertices = if mixed {
                vec![
                    point(px(6.), px(12.)),
                    point(px(6. + 12. * progress), px(12.)),
                ]
            } else {
                check_vertices(progress)
            };
            let mut path = PathBuilder::stroke(px(2. * scale)).with_style(gpui::PathStyle::Stroke(
                gpui::StrokeOptions::default()
                    .with_line_width(2. * scale)
                    .with_line_cap(gpui::LineCap::Round),
            ));
            for (index, vertex) in vertices.iter().enumerate() {
                let position = bounds.origin + *vertex * scale;
                if index == 0 {
                    path.move_to(position);
                } else {
                    path.line_to(position);
                }
            }
            window.paint_path(
                path.build().expect("Finite checkbox path must tessellate"),
                color,
            );
            let radius = px(scale);
            for vertex in vertices {
                let position = bounds.origin + vertex * scale;
                window.paint_quad(
                    gpui::fill(
                        Bounds::new(
                            position - point(radius, radius),
                            gpui::size(radius * 2., radius * 2.),
                        ),
                        color,
                    )
                    .corner_radii(radius),
                );
            }
        },
    )
    .size(px(size))
    .flex_none()
}

struct Presentation {
    value: CheckState,
    progress: Rc<Cell<f32>>,
    from: f32,
    revision: usize,
}
impl Presentation {
    fn new(value: CheckState) -> Self {
        let progress = if value == CheckState::Unchecked {
            0.
        } else {
            1.
        };
        Self {
            value,
            progress: Rc::new(Cell::new(progress)),
            from: progress,
            revision: 0,
        }
    }
    fn update(&mut self, value: CheckState) {
        if self.value != value {
            self.from = self.progress.get();
            self.value = value;
            self.revision = self
                .revision
                .checked_add(1)
                .expect("Checkbox presentation revision overflow");
        }
    }
}

/// Editable checkbox presentation; input, value proposals and Field semantics remain native.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    primitive: kinu_ui::Checkbox,
    value: CheckState,
    tokens: Option<ComponentTheme>,
    group_hovered: bool,
    invalid: bool,
    style: StyleRefinement,
    focus_visible_style: Option<StyleRefinement>,
}
impl Checkbox {
    pub fn new(id: impl Into<ElementId>, value: CheckState) -> Self {
        let id = id.into();
        Self {
            primitive: kinu_ui::Checkbox::new(id.clone(), value),
            id,
            value,
            tokens: None,
            group_hovered: false,
            invalid: false,
            style: StyleRefinement::default(),
            focus_visible_style: None,
        }
    }
    pub fn tokens(mut self, tokens: ComponentTheme) -> Self {
        self.tokens = Some(tokens.clone());
        self
    }
    /// Projects the enclosing label row's hover appearance without changing activation.
    pub fn group_hovered(mut self, hovered: bool) -> Self {
        self.group_hovered = hovered;
        self
    }
    /// Replaces only keyboard-focus presentation, for a surrounding labeled card.
    pub fn focus_visible_style(mut self, style: StyleRefinement) -> Self {
        self.focus_visible_style = Some(style.clone());
        self
    }
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self.primitive = self.primitive.invalid(invalid);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.primitive = self.primitive.disabled(disabled);
        self
    }
    pub fn track_focus(mut self, focus: &FocusHandle) -> Self {
        self.primitive = self.primitive.track_focus(focus);
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<gpui::SharedString>) -> Self {
        self.primitive = self.primitive.accessibility_label(label);
        self
    }
    pub fn on_change(
        mut self,
        change: impl Fn(&CheckState, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_change(change);
        self
    }
}
impl Styled for Checkbox {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl FieldControl for Checkbox {
    type Value = CheckState;
    fn field_value(&self) -> CheckState {
        self.primitive.field_value()
    }
    fn field_filled(&self) -> bool {
        self.primitive.field_filled()
    }
    fn field_disabled(&self) -> bool {
        self.primitive.field_disabled()
    }
    fn field_label_behavior(&self) -> FieldLabelBehavior {
        self.primitive.field_label_behavior()
    }
    fn with_field(mut self, binding: FieldControlBinding) -> Self {
        self.primitive = self.primitive.with_field(binding);
        self
    }
}
impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ComponentTheme::resolve(self.tokens.clone(), cx);

        let compact = tokens.size == ControlSize::Compact;
        let colors = tokens.extras(if ColorScheme::current(window, cx) == ColorScheme::Dark {
            ThemeScheme::Dark
        } else {
            ThemeScheme::Light
        });
        let checked = self.value != CheckState::Unchecked;
        let disabled = self.primitive.effective_disabled(window);
        let border = if self.group_hovered && !disabled && !self.invalid {
            gpui::Hsla::from(colors.foreground).opacity(0.4)
        } else if checked {
            gpui::transparent_black()
        } else if self.invalid {
            gpui::Hsla::from(colors.destructive).opacity(0.5)
        } else {
            gpui::Hsla::from(colors.border)
        };
        let state = window.use_keyed_state((self.id, "flow-checkbox-presentation"), cx, |_, _| {
            Presentation::new(self.value)
        });
        let (from, progress, revision) = state.update(cx, |state, _| {
            state.update(self.value);
            (state.from, state.progress.clone(), state.revision)
        });
        let target = if checked { 1. } else { 0. };
        let mixed = self.value == CheckState::Mixed;
        let tick = div().flex().items_center().justify_center().with_animation(
            ("check-drawing", revision),
            (if checked {
                tokens.motion.animations.check_draw
            } else {
                tokens.motion.animations.check_retract
            })
            .animation(),
            {
                let __theme_colors_foreground = colors.foreground;
                move |tick, delta| {
                    let fraction = from + (target - from) * delta;
                    progress.set(fraction);
                    tick.child(mark(
                        fraction,
                        mixed,
                        gpui::Hsla::from(__theme_colors_foreground),
                        if compact { 16. } else { 18. },
                    ))
                }
            },
        );
        let mut primitive = self
            .primitive
            .size(px(if compact { 14. } else { 16. }))
            .relative()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .border(px(1.5))
            .border_color(border)
            .rounded(px(tokens.shape.checkbox_radius(tokens.size)))
            .opacity(if disabled { 0.5 } else { 1. })
            .composite_opacity()
            .hover({
                let __theme_colors_foreground = colors.foreground;
                move |style| {
                    style.border_color(gpui::Hsla::from(__theme_colors_foreground).opacity(0.4))
                }
            })
            .focus_visible({
                let __theme_colors_brand = colors.brand;
                move |style| {
                    self.focus_visible_style.unwrap_or_else(|| {
                        style.shadow(vec![
                            gpui::BoxShadow::new(
                                px(0.),
                                px(0.),
                                gpui::Hsla::from(__theme_colors_brand),
                            )
                            .spread_radius(px(1.)),
                        ])
                    })
                }
            })
            .transition(StyleTransition::colors(Animation::new(
                Duration::from_millis(tokens.motion.animations.check_draw.duration_ms),
            )))
            .child(tick);
        primitive.style().refine(&self.style);
        primitive
    }
}
