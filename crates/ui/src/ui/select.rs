use crate::ui::icon::{Icon, IconName};
use crate::ui::scroll_area::{paint_scroll_fade, scrollbar};
use crate::ui::utils::hover::{HoverAxis, HoverItem, HoverMotion, HoverTarget};
use crate::ui::utils::motion::{ColorStrokeTween, StrokeReveal, popup_motion};
use crate::ui::utils::presentation::{
    CapTrimmed, ComponentTheme, ControlSize, SurfaceLevel, TextRole, ThemeScheme,
};
use crate::ui::utils::selection::{
    SelectionListFacts, SelectionListRow, selection_background, selection_focus,
};
use gpui::{
    AnyElement, App, ElementId, IntoElement, StyleRefinement, StyleTransition, Styled, Window, px,
};
use kinu_ui::{
    ColorScheme, PopupAlign, PopupModality, PopupMotion, SelectAppearance, SelectGlyph,
    SelectIconPresentation, SelectIndicatorPresentation, SelectListLayers, SelectPartsPresentation,
    SelectPopup, SelectPositioner, SelectTriggerAppearance, Selection,
};
use std::time::Duration;

fn icon_element(
    name: IconName,
    id: &ElementId,
    tokens: ComponentTheme,
    color: gpui::Hsla,
    stroke: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let now = cx.background_executor().now();
    let runtime = window.use_keyed_state((id.clone(), "flow-select-icon"), cx, |_, _| {
        ColorStrokeTween::new(color, stroke, now, tokens.motion.animations.color.easing)
    });
    let (color, stroke, done) = runtime.update(cx, |state, _| {
        state.retarget(
            color,
            stroke,
            now,
            Duration::from_millis(tokens.motion.animations.color.duration_ms),
            tokens.motion.animations.color.easing,
        );
        state.sample(now)
    });
    if !done {
        window.request_animation_frame();
    }
    Icon::new(name)
        .size(icon_size(tokens.size))
        .stroke_width(stroke)
        .text_color(color)
        .into_any_element()
}

/// Supplies a fresh named asset; the native Select provides interaction and selection facts.
pub fn select_icon_with_tokens(
    name: IconName,
    tokens: ComponentTheme,
) -> impl Fn(&SelectIconPresentation, &mut Window, &mut App) -> AnyElement {
    {
        let tokens = tokens.clone();
        move |facts, window, cx| {
            let colors = tokens.extras(scheme(window, cx));
            let active = facts.decorative_active || facts.selected;
            icon_element(
                name,
                &facts.id,
                tokens.clone(),
                if active {
                    gpui::Hsla::from(colors.foreground)
                } else {
                    gpui::Hsla::from(colors.muted_foreground)
                },
                if active { 2. } else { 1.5 },
                window,
                cx,
            )
        }
    }
}

/// A trigger's leading icon responds to pointer hover independently of its selected value.
pub fn trigger_icon_with_tokens(
    name: IconName,
    tokens: ComponentTheme,
) -> impl Fn(&SelectIconPresentation, &mut Window, &mut App) -> AnyElement {
    {
        let tokens = tokens.clone();
        move |facts, window, cx| {
            let colors = tokens.extras(scheme(window, cx));
            icon_element(
                name,
                &facts.id,
                tokens.clone(),
                if facts.hovered {
                    gpui::Hsla::from(colors.foreground)
                } else {
                    gpui::Hsla::from(colors.muted_foreground)
                },
                if facts.hovered { 2. } else { 1.5 },
                window,
                cx,
            )
        }
    }
}

fn indicator(
    facts: &SelectIconPresentation,
    tokens: ComponentTheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let selection = window
        .element_context::<SelectIndicatorPresentation>()
        .expect("Select indicator requires native selection facts")
        .clone();
    let now = cx.background_executor().now();
    let runtime = window.use_keyed_state((facts.id.clone(), "flow-select-tick"), cx, |_, _| {
        StrokeReveal::new(
            selection.open_session,
            selection.initially_selected,
            now,
            tokens.motion.curves,
        )
    });
    let (progress, done) = runtime.update(cx, |state, _| {
        state.update_animations(
            selection.open_session,
            selection.selected,
            selection.initially_selected,
            now,
            tokens.motion.animations.check_draw,
            tokens.motion.animations.check_retract,
        );
        state.sample(now)
    });
    if !done {
        window.request_animation_frame();
    }
    Icon::new(IconName::CheckStroke)
        .size(icon_size(tokens.size))
        .stroke_width(2.)
        .stroke_direction(gpui::SvgStrokeDirection::Reverse)
        .stroke_progress(progress)
        .text_color(gpui::Hsla::from(
            (tokens.extras(scheme(window, cx))).foreground,
        ))
        .into_any_element()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectVariant {
    Bordered,
    Borderless,
}

/// Editable control and popup values; selected values and open state remain application-owned.
#[derive(Clone)]
pub struct SelectStyle {
    pub tokens: ComponentTheme,
    pub variant: SelectVariant,
    pub surface: SurfaceLevel,
}
impl SelectStyle {
    /// Reads presentation settings from the application's shared theme.
    pub fn current(cx: &gpui::App) -> Self {
        Self::new(ComponentTheme::current(cx))
    }
    pub fn new(tokens: ComponentTheme) -> Self {
        Self {
            tokens: tokens.clone(),
            variant: SelectVariant::Bordered,
            surface: SurfaceLevel::Three,
        }
    }
}

/// An accessible native error line with the Select trigger's inset.
pub fn select_error_with_tokens(
    message: impl Into<gpui::SharedString>,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> kinu_ui::FieldError {
    let caption = TextRole::Caption.metrics(tokens.size, &tokens);
    kinu_ui::FieldError::new(kinu_ui::FieldErrorMatch::Always)
        .message(message)
        .pl(px(12.))
        .text_size(px(caption.size))
        .line_height(px(caption.line_height))
        .text_color(gpui::Hsla::from(
            (tokens.extras(scheme(window, cx))).destructive,
        ))
}

fn scheme(window: &Window, cx: &App) -> ThemeScheme {
    if ColorScheme::current(window, cx) == ColorScheme::Dark {
        ThemeScheme::Dark
    } else {
        ThemeScheme::Light
    }
}
fn icon_size(size: ControlSize) -> f32 {
    match size {
        ControlSize::Default => 16.,
        ControlSize::Compact => 14.,
    }
}
fn motion(tokens: ComponentTheme) -> PopupMotion {
    popup_motion(tokens.motion.fast, px(4.))
}
fn appearance(style: SelectStyle, scheme: ThemeScheme) -> SelectAppearance {
    let tokens = style.tokens.clone();
    let colors = tokens.extras(scheme);
    let metrics = tokens.size.metrics(&tokens);
    let shape = tokens.shape.popup().radii(&tokens);
    let size = icon_size(tokens.size);
    let minimum = if tokens.size == ControlSize::Compact {
        128.
    } else {
        160.
    };
    let trigger = {
        let tokens = tokens.clone();
        let style = style.clone();
        {
            let __theme_colors_border = colors.border;
            let __theme_colors_brand = colors.brand;
            let __theme_colors_destructive = colors.destructive;
            let __theme_colors_foreground = colors.foreground;
            let __theme_colors_hover = colors.hover();
            let __theme_colors_muted_foreground = colors.muted_foreground;
            move |state: kinu_ui::SelectTriggerState| {
                let border = if state.invalid {
                    gpui::Hsla::from(__theme_colors_destructive).opacity(0.5)
                } else if style.variant == SelectVariant::Bordered {
                    gpui::Hsla::from(__theme_colors_border)
                } else {
                    gpui::transparent_black()
                };
                SelectTriggerAppearance {
                    base: StyleRefinement::default()
                        .flex()
                        .items_center()
                        .justify_between()
                        .h(px(metrics.control_height))
                        .min_w(px(minimum))
                        .px(px(metrics.horizontal_padding))
                        .gap(px(metrics.gap))
                        .rounded(px(tokens.shape.radii(&tokens).item))
                        .border_1()
                        .border_color(border)
                        .bg(gpui::transparent_black())
                        .font_family(tokens.typography.family.clone())
                        .font_weight(tokens.typography.normal.weight)
                        .font_variations(tokens.typography.normal.variations())
                        .text_size(px(metrics.text_size))
                        .line_height(px(metrics.line_height))
                        .text_color(if state.placeholder {
                            gpui::Hsla::from(__theme_colors_muted_foreground)
                        } else {
                            gpui::Hsla::from(__theme_colors_foreground)
                        }),
                    hover: StyleRefinement::default().bg(__theme_colors_hover),
                    focus: StyleRefinement::default().shadow(vec![
                        gpui::BoxShadow::new(
                            px(0.),
                            px(0.),
                            gpui::Hsla::from(__theme_colors_brand),
                        )
                        .spread_radius(px(1.)),
                    ]),
                    disabled: StyleRefinement::default().opacity(0.5),
                    transition: Some(
                        StyleTransition::new(tokens.motion.animations.color.animation())
                            .properties([
                                gpui::StyleTransitionProperty::Background,
                                gpui::StyleTransitionProperty::TextColor,
                                gpui::StyleTransitionProperty::CornerRadii,
                                gpui::StyleTransitionProperty::BoxShadow,
                                gpui::StyleTransitionProperty::Opacity,
                            ])
                            .reduced_motion(gpui::StyleTransitionReducedMotion::OpacityOnly),
                    ),
                }
            }
        }
    };
    let mut p = SelectAppearance::new(trigger, move |part| {
        Icon::new(match part {
            SelectGlyph::Trigger | SelectGlyph::ScrollDown => IconName::ChevronDown,
            SelectGlyph::ScrollUp => IconName::ChevronUp,
            SelectGlyph::Indicator => IconName::CheckStroke,
        })
        .size(size)
        .into_any_element()
    });
    p.value = StyleRefinement::default()
        .min_w_0()
        .flex_1()
        .truncate()
        .py(px(4.))
        .my(px(-4.));
    p.icon = StyleRefinement::default()
        .flex_none()
        .size(px(size))
        .text_color(gpui::Hsla::from(colors.muted_foreground));
    p.popup = StyleRefinement::default()
        .flex()
        .flex_col()
        .min_h_0()
        .max_h(px(300.))
        .rounded(px(shape.container))
        .bg(style.surface.color(scheme, &tokens))
        .shadow(SurfaceLevel::Three.shadows(scheme, &tokens))
        .overflow_hidden()
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.normal.weight)
        .font_variations(tokens.typography.normal.variations())
        .text_color(gpui::Hsla::from(colors.foreground))
        .text_size(px(metrics.text_size))
        .line_height(px(metrics.line_height));
    p.list = StyleRefinement::default()
        .relative()
        .flex()
        .flex_col()
        .p(px(4.));
    p.scroll_padding = gpui::Edges::all(px(4.));
    p.group = StyleRefinement::default().flex().flex_col().flex_none();
    let caption = TextRole::Caption.metrics(tokens.size, &tokens);
    p.group_label = StyleRefinement::default()
        .px(px(8.))
        .py(px(6.))
        .flex_none()
        .text_size(px(caption.size))
        .line_height(px(caption.line_height))
        .text_color(gpui::Hsla::from(colors.muted_foreground));
    p.item = StyleRefinement::default()
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .h(px(metrics.control_height))
        .gap(px(metrics.gap))
        .px(px(if tokens.size == ControlSize::Compact {
            6.
        } else {
            8.
        }))
        .rounded(px(shape.item))
        .text_color(gpui::Hsla::from(colors.muted_foreground));
    p.item_text = StyleRefinement::default()
        .min_w_0()
        .flex_1()
        .truncate()
        .py(px(4.))
        .my(px(-4.));
    p.item_highlighted = StyleRefinement::default();
    p.item_disabled = StyleRefinement::default().opacity(0.5);
    p.indicator = StyleRefinement::default()
        .flex_none()
        .size(px(size))
        .text_color(gpui::Hsla::from(colors.foreground));
    p.separator = StyleRefinement::default()
        .h(px(1.))
        .flex_none()
        .my(px(4.))
        .mx(px(-4.))
        .bg(gpui::Hsla::from(colors.border).opacity(0.6));
    p.scroll_arrow = StyleRefinement::default().h(px(0.)).overflow_hidden();
    p
}

fn parts(style: SelectStyle, scheme: ThemeScheme) -> SelectPartsPresentation {
    let tokens = style.tokens.clone();
    let colors = tokens.extras(scheme);
    let size = icon_size(tokens.size);
    SelectPartsPresentation::new()
        .value_label(|label| CapTrimmed::new(label.clone(), label).into_any_element())
        .item_selected(StyleRefinement::default().text_color(gpui::Hsla::from(colors.foreground)))
        .decorative_active_row(decorative_active)
        .item_decorative_active(
            StyleRefinement::default().text_color(gpui::Hsla::from(colors.foreground)),
        )
        .item_transition(StyleTransition::colors(
            tokens.motion.animations.color.animation(),
        ))
        .indicator_keep_mounted(true)
        .trigger_leading_icon_style(StyleRefinement::default().flex_none().size(px(size)))
        .item_icon_style(StyleRefinement::default().flex_none().size(px(size)))
        .glyph_with_state({
            let tokens = tokens.clone();
            {
                let __theme_colors_foreground = colors.foreground;
                let __theme_colors_muted_foreground = colors.muted_foreground;
                move |part, facts, window, cx| match part {
                    SelectGlyph::Indicator => indicator(facts, tokens.clone(), window, cx),
                    _ => icon_element(
                        match part {
                            SelectGlyph::Trigger | SelectGlyph::ScrollDown => IconName::ChevronDown,
                            SelectGlyph::ScrollUp => IconName::ChevronUp,
                            SelectGlyph::Indicator => unreachable!(),
                        },
                        &facts.id,
                        tokens.clone(),
                        if facts.hovered {
                            gpui::Hsla::from(__theme_colors_foreground)
                        } else {
                            gpui::Hsla::from(__theme_colors_muted_foreground)
                        },
                        2.,
                        window,
                        cx,
                    ),
                }
            }
        })
}

fn decorative_active(facts: &kinu_ui::SelectListPresentation) -> Option<ElementId> {
    if !facts.open {
        return None;
    }
    if facts.keyboard_origin {
        return facts
            .keyboard_focused
            .as_ref()
            .and_then(|id| {
                facts
                    .rows
                    .iter()
                    .find(|row| &row.id == id && row.enabled && row.bounds.is_some())
            })
            .map(|row| row.id.clone());
    }
    let pointer = facts.pointer_position?;
    if !facts.viewport_bounds.contains(&pointer) {
        return None;
    }
    let mut projection = HoverMotion::new();
    projection.update(
        pointer,
        facts.rows.iter().filter_map(|row| {
            row.bounds.map(|bounds| HoverItem {
                id: row.id.clone(),
                bounds,
                enabled: row.enabled,
            })
        }),
        HoverAxis::Vertical,
        HoverTarget::Nearest,
    );
    projection.snapshot().active
}

fn selection_facts(facts: &kinu_ui::SelectListPresentation) -> SelectionListFacts {
    SelectionListFacts {
        rows: facts
            .rows
            .iter()
            .map(|row| SelectionListRow {
                id: row.id.clone(),
                bounds: row.bounds,
                enabled: row.enabled,
                selected: row.selected,
            })
            .collect(),
        decorative_active_id: facts.decorative_active_id.clone(),
        keyboard_focused: facts.keyboard_focused.clone(),
        keyboard_origin: facts.keyboard_origin,
        viewport_bounds: facts.viewport_bounds,
        open: facts.open,
        open_session: facts.open_session,
    }
}
fn layers(id: ElementId, tokens: ComponentTheme, scheme: ThemeScheme) -> SelectListLayers {
    let behind = id.clone();
    SelectListLayers::new()
        .behind({
            let tokens = tokens.clone();
            move |facts, window, cx| {
                selection_background(
                    &behind,
                    &selection_facts(facts),
                    tokens.clone(),
                    scheme,
                    window,
                    cx,
                )
            }
        })
        .above({
            let tokens = tokens.clone();
            move |facts, window, cx| {
                selection_focus(
                    &id,
                    &selection_facts(facts),
                    tokens.clone(),
                    scheme,
                    window,
                    cx,
                )
            }
        })
}

/// Applies component presentation to the native controlled Select.
pub fn select<T: Clone + PartialEq + 'static>(
    id: impl Into<ElementId>,
    selected: Selection<T>,
    open: bool,
    style: SelectStyle,
    window: &Window,
    cx: &App,
) -> kinu_ui::Select<T> {
    let id = id.into();
    kinu_ui::Select::new(id.clone(), selected, open)
        .flex()
        .flex_none()
        .appearance(appearance(style.clone(), scheme(window, cx)))
        .parts_presentation(parts(style.clone(), scheme(window, cx)))
        .list_layers(layers(id.clone(), style.tokens.clone(), scheme(window, cx)))
        .viewport_presentation({
            let style = style.clone();
            move |viewport, window, cx| {
                viewport
                    .scrollbar(scrollbar(
                        gpui::Axis::Vertical,
                        style.tokens.clone(),
                        scheme(window, cx),
                    ))
                    .mask(|bounds, state, window| {
                        paint_scroll_fade(bounds, state, gpui::Axis::Vertical, px(32.), window)
                    })
            }
        })
        .popup(SelectPopup::new((id, "popup")).min_anchor_width(px(0.)))
        .positioner(
            SelectPositioner::new()
                .align(PopupAlign::Start)
                .align_item_with_trigger(false)
                .side_offset(px(6.)),
        )
        .modality(PopupModality::NonModal)
        .selection_acknowledgement(Duration::from_millis(
            style.tokens.settings.timings.selection_acknowledgement_ms,
        ))
        .motion(motion(style.tokens.clone()))
        .motion_axes(kinu_ui::PopupMotionAxes::Vertical)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn select_icon(
    name: IconName,
    cx: &App,
) -> impl Fn(&SelectIconPresentation, &mut Window, &mut App) -> AnyElement + use<> {
    select_icon_with_tokens(name, ComponentTheme::current(cx))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn trigger_icon(
    name: IconName,
    cx: &App,
) -> impl Fn(&SelectIconPresentation, &mut Window, &mut App) -> AnyElement + use<> {
    trigger_icon_with_tokens(name, ComponentTheme::current(cx))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn select_error(
    message: impl Into<gpui::SharedString>,
    window: &Window,
    cx: &App,
) -> kinu_ui::FieldError {
    select_error_with_tokens(message, ComponentTheme::current(cx), window, cx)
}
