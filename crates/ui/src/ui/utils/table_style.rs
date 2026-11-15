use crate::ui::utils::hover::{HoverAxis, HoverHighlight, HoverItem, HoverMotion, HoverTarget};
use crate::ui::utils::motion::TimedValue;
use crate::ui::utils::presentation::{ComponentTheme, ControlSize, TextRole, ThemeScheme};
use crate::ui::utils::selection::{SelectionBackgrounds, SelectionRow};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, ElementId, IntoElement, Pixels, StyleRefinement, StyleTransition,
    Window, div, point, px, size,
};
use kinu_ui::{ColorScheme, TableAppearance, TableFacts, TablePresentation};
use std::cell::Cell;
use std::time::Duration;

pub(crate) fn scheme(window: &Window, cx: &App) -> ThemeScheme {
    match ColorScheme::current(window, cx) {
        ColorScheme::Dark => ThemeScheme::Dark,
        _ => ThemeScheme::Light,
    }
}

#[derive(Clone, Copy)]
struct HighlightFrame {
    bounds: Bounds<Pixels>,
    opacity: f32,
}
struct RuleFade {
    row: ElementId,
    alpha: TimedValue,
}

fn backgrounds(
    id: ElementId,
    facts: &TableFacts,
    tokens: ComponentTheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let colors = tokens.extras(scheme(window, cx));
    let Some(parent) = window.window_to_element(facts.bounds) else {
        return gpui::Empty.into_any_element();
    };
    let origin = parent.origin;
    let rows = facts
        .rows
        .iter()
        .map(|row| SelectionRow {
            bounds: row
                .bounds
                .and_then(|bounds| window.window_to_element(bounds))
                .map(|bounds| Bounds::new(bounds.origin - origin, bounds.size)),
            selected: row.selected,
        })
        .collect::<Vec<_>>();
    let target = facts
        .decorative_active_id
        .as_ref()
        .and_then(|id| facts.rows.iter().position(|row| &row.id == id))
        .and_then(|index| {
            rows[index].bounds.map(|bounds| HoverItem {
                id: facts.rows[index].id.clone(),
                bounds,
                enabled: true,
            })
        });
    let hover = window.use_keyed_state((id.clone(), "hover"), cx, |_, _| HoverMotion::new());
    let snapshot = hover.update(cx, |hover, _| {
        if let Some(item) = target {
            hover.update(
                item.bounds.center(),
                [&item],
                HoverAxis::Vertical,
                HoverTarget::Containing,
            );
        } else {
            hover.clear();
        }
        hover.snapshot()
    });
    let frame = window.use_keyed_state((id.clone(), "hover-frame"), cx, |_, _| {
        Cell::new(None::<HighlightFrame>)
    });
    let observe_frame = frame.clone();
    let fades = window.use_keyed_state((id.clone(), "rule-fades"), cx, |_, _| {
        Vec::<RuleFade>::new()
    });
    let mut rules = Vec::new();
    if let Some(bounds) = facts.header_bounds {
        rules.push((
            (id.clone(), "header-rule").into(),
            bounds.bottom(),
            facts.rows.first().is_some_and(|row| row.selected),
        ));
    }
    for (index, row) in facts.rows.iter().enumerate() {
        if let Some(bounds) = row.bounds {
            rules.push((
                row.id.clone(),
                bounds.bottom(),
                row.selected || facts.rows.get(index + 1).is_some_and(|next| next.selected),
            ));
        }
    }
    let table_bounds = facts.bounds;
    let duration = Duration::from_millis(tokens.motion.animations.color.duration_ms);
    let measured_rules = rules.clone();
    let lines = gpui::canvas(
        move |_, window, _| {
            measured_rules
                .iter()
                .map(|(_, y, _)| {
                    window.window_to_element(Bounds::new(
                        point(table_bounds.left(), *y - px(1.)),
                        size(table_bounds.size.width, px(1.)),
                    ))
                })
                .collect::<Vec<_>>()
        },
        {
            let __theme_colors_accent = colors.accent;
            move |_, local_rules, window, cx| {
                let now = cx.background_executor().now();
                let highlight = frame.read(cx).get();
                let mut moving = false;
                fades.update(cx, |fades, _| {
                    for ((row, y, selected), local_bounds) in rules.iter().zip(local_rules) {
                        let covered = highlight.is_some_and(|fill| {
                            fill.opacity >= 0.5
                                && *y >= fill.bounds.top() - px(1.5)
                                && *y <= fill.bounds.bottom() + px(1.5)
                        });
                        let hidden = *selected || covered;
                        let index = match fades.iter().position(|fade| &fade.row == row) {
                            Some(index) => index,
                            None => {
                                fades.push(RuleFade {
                                    row: row.clone(),
                                    alpha: TimedValue::new(if hidden { 0. } else { 1. }, now),
                                });
                                fades.len() - 1
                            }
                        };
                        let fade = &mut fades[index];
                        let alpha = if hidden {
                            fade.alpha = TimedValue::new(0., now);
                            0.
                        } else {
                            fade.alpha.retarget(1., now, duration, gpui::linear);
                            let (alpha, settled) = fade.alpha.sample(now, gpui::linear);
                            moving |= !settled;
                            alpha
                        };
                        if alpha > 0. {
                            if let Some(bounds) = local_bounds {
                                window.paint_quad(gpui::fill(
                                    bounds,
                                    gpui::Hsla::from(__theme_colors_accent).opacity(0.4 * alpha),
                                ));
                            }
                        }
                    }
                    fades.retain(|fade| rules.iter().any(|(id, _, _)| id == &fade.row));
                });
                if moving {
                    window.request_animation_frame();
                }
            }
        },
    )
    .absolute()
    .inset_0();
    let layers = div()
        .absolute()
        .inset_0()
        .child(SelectionBackgrounds::new(
            (id.clone(), "selection"),
            rows,
            px(tokens.shape.radii(&tokens).merged),
            colors.active(),
            tokens.motion.moderate,
            tokens.clone(),
        ))
        .child(lines)
        .child(
            HoverHighlight::new(
                (id, "hover-fill"),
                snapshot,
                colors.hover(),
                px(tokens.shape.radii(&tokens).item),
                tokens.motion.fast,
            )
            .initial_opacity(0.)
            .on_frame(move |bounds, opacity, _, cx| {
                observe_frame
                    .read(cx)
                    .set(Some(HighlightFrame { bounds, opacity }))
            }),
        )
        .into_any_element();
    let Some(body) = facts
        .body_bounds
        .and_then(|bounds| window.window_to_element(bounds))
    else {
        return gpui::Empty.into_any_element();
    };
    let clip_top = (body.top() - origin.y - px(1.)).max(px(0.));
    let clip_bottom = (body.bottom() - origin.y).min(parent.size.height);
    let clipped = div()
        .absolute()
        .left_0()
        .right_0()
        .top(clip_top)
        .h((clip_bottom - clip_top).max(px(0.)))
        .overflow_hidden()
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(-clip_top)
                .h(parent.size.height)
                .child(layers),
        )
        .into_any_element();
    div()
        .absolute()
        .inset_0()
        .w(parent.size.width)
        .h(parent.size.height)
        .child(clipped)
        .into_any_element()
}

pub(crate) fn table_parts(
    id: ElementId,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> (TableAppearance, TablePresentation) {
    let colors = tokens.extras(scheme(window, cx));
    let metrics = TextRole::Body.metrics(tokens.size, &tokens);
    let compact = tokens.size == ControlSize::Compact;
    let py = if compact { 5. } else { 8. };
    let px_value = if compact { 10. } else { 12. };
    let body = StyleRefinement::default()
        .font_family(tokens.typography.family.clone())
        .text_size(px(metrics.size))
        .line_height(px(metrics.line_height))
        .font_weight(tokens.typography.normal.weight)
        .font_variations(tokens.typography.normal.variations());
    let cell = StyleRefinement::default()
        .px(px(px_value))
        .py(px(py))
        .border_b_1()
        .border_color(gpui::transparent_black());
    let behind = id.clone();
    let row_tokens = tokens.clone();
    (
        TableAppearance {
            root: body
                .clone()
                .w_full()
                .relative()
                .text_color(gpui::Hsla::from(colors.foreground)),
            header: StyleRefinement::default()
                .font_weight(tokens.typography.semibold.weight)
                .font_variations(tokens.typography.semibold.variations()),
            body: StyleRefinement::default(),
            row: StyleRefinement::default(),
            row_transition: Some(StyleTransition::colors(
                tokens
                    .motion
                    .animations
                    .color
                    .animation()
                    .with_easing(gpui::linear),
            )),
            header_cell: cell.clone(),
            cell,
            empty: StyleRefinement::default()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .py(px(16.))
                .px(px(12.)),
            row_height: px(metrics.line_height + py * 2. + 1.),
        },
        TablePresentation::new()
            .behind(move |facts, window, cx| {
                backgrounds(behind.clone(), facts, tokens.clone(), window, cx)
            })
            .row(move |facts, window, cx| {
                let colors = row_tokens.extras(scheme(window, cx));
                StyleRefinement::default().text_color(if facts.decorative_active {
                    gpui::Hsla::from(colors.foreground)
                } else {
                    gpui::Hsla::from(colors.muted_foreground)
                })
            }),
    )
}
