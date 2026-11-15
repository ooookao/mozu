use crate::ui::button::Button;
use crate::ui::icon::{Icon, IconName};
use crate::ui::scroll_area::{paint_scroll_fade, scrollbar};
use crate::ui::utils::hover::{
    HoverAxis, HoverHighlight, HoverItem, HoverMotion, HoverSnapshot, HoverTarget,
};
use crate::ui::utils::motion::{ColorStrokeTween, StrokeReveal, popup_motion};
use crate::ui::utils::presentation::{
    ComponentTheme, ControlSize, SurfaceLevel, TextRole, ThemeScheme,
};
use crate::ui::utils::selection::{SelectionBackgrounds, SelectionRow};
use gpui::{
    AnyElement, App, Bounds, ElementId, IntoElement, ParentElement, StyleRefinement,
    StyleTransition, Styled, Window, div, px,
};
use kinu_ui::{
    ColorScheme, Menu, MenuAppearance, MenuIconPresentation, MenuIndicatorPlacement,
    MenuListLayers, MenuListPresentation, MenuPartsPresentation, MenuRowAppearance, MenuRowKind,
    MenuSubmenu, MenuSurfaceKind, PopupAlign, PopupModality, PopupSide,
};
use std::time::Duration;

#[derive(Clone)]
pub struct DropdownStyle {
    pub tokens: ComponentTheme,
    pub surface: SurfaceLevel,
}
impl DropdownStyle {
    /// Reads presentation settings from the application's shared theme.
    pub fn current(cx: &gpui::App) -> Self {
        Self::new(ComponentTheme::current(cx))
    }
    pub fn new(tokens: ComponentTheme) -> Self {
        Self {
            tokens: tokens.clone(),
            surface: SurfaceLevel::Three,
        }
    }
}
pub(crate) fn scheme(window: &Window, cx: &App) -> ThemeScheme {
    if ColorScheme::current(window, cx) == ColorScheme::Dark {
        ThemeScheme::Dark
    } else {
        ThemeScheme::Light
    }
}
fn checked(kind: MenuRowKind) -> bool {
    matches!(
        kind,
        MenuRowKind::Checkbox { checked: true } | MenuRowKind::Radio { checked: true }
    )
}
fn glyph_size(size: ControlSize) -> f32 {
    match size {
        ControlSize::Default => 16.,
        ControlSize::Compact => 14.,
    }
}
fn animated_icon(
    name: IconName,
    facts: &MenuIconPresentation,
    tokens: ComponentTheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    window.with_element_namespace(facts.owner_id.clone(), |window| {
        let colors = tokens.extras(scheme(window, cx));
        let active = facts.decorative_active || checked(facts.kind);
        let color = if active {
            gpui::Hsla::from(colors.foreground)
        } else {
            gpui::Hsla::from(colors.muted_foreground)
        };
        let stroke = if active { 2. } else { 1.5 };
        let now = cx.background_executor().now();
        let runtime =
            window.use_keyed_state((facts.id.clone(), "flow-dropdown-icon"), cx, |_, _| {
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
            .size(glyph_size(tokens.size))
            .stroke_width(stroke)
            .text_color(color)
            .into_any_element()
    })
}
/// Supplies a fresh named asset while the native menu owns its row's interaction facts.
pub fn dropdown_icon_with_tokens(
    name: IconName,
    tokens: ComponentTheme,
) -> impl Fn(&MenuIconPresentation, &mut Window, &mut App) -> AnyElement {
    {
        let tokens = tokens.clone();
        move |facts, window, cx| animated_icon(name, facts, tokens.clone(), window, cx)
    }
}
fn indicator(
    facts: &MenuIconPresentation,
    tokens: ComponentTheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    window.with_element_namespace(facts.owner_id.clone(), |window| {
        if matches!(facts.kind, MenuRowKind::Submenu { .. }) {
            let mut glyph = facts.clone();
            glyph.id = (facts.id.clone(), "chevron").into();
            return animated_icon(IconName::ChevronRight, &glyph, tokens.clone(), window, cx);
        }
        let now = cx.background_executor().now();
        let runtime =
            window.use_keyed_state((facts.id.clone(), "flow-dropdown-check"), cx, |_, _| {
                StrokeReveal::new(
                    facts.open_session,
                    facts.initially_checked,
                    now,
                    tokens.motion.curves,
                )
            });
        let (progress, done) = runtime.update(cx, |state, _| {
            state.update_animations(
                facts.open_session,
                checked(facts.kind),
                facts.initially_checked,
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
            .size(glyph_size(tokens.size))
            .stroke_width(2.)
            .stroke_direction(gpui::SvgStrokeDirection::Reverse)
            .stroke_progress(progress)
            .text_color(gpui::Hsla::from(
                (tokens.extras(scheme(window, cx))).foreground,
            ))
            .into_any_element()
    })
}
pub(crate) fn appearance(style: DropdownStyle, scheme: ThemeScheme) -> MenuAppearance {
    let tokens = style.tokens.clone();
    let colors = tokens.extras(scheme);
    let metrics = tokens.size.metrics(&tokens);
    let shape = tokens.shape.popup().radii(&tokens);
    let row = {
        let tokens = tokens.clone();
        {
            let __theme_colors_foreground = colors.foreground;
            let __theme_colors_muted_foreground = colors.muted_foreground;
            move |facts: kinu_ui::MenuRowState| {
                let selected = checked(facts.kind);
                let typography = if selected {
                    tokens.typography.semibold
                } else {
                    tokens.typography.normal
                };
                let row = StyleRefinement::default()
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
                    .cursor_pointer()
                    .opacity(if facts.disabled { 0.5 } else { 1. });
                let label = StyleRefinement::default()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .py(px(4.))
                    .my(px(-4.))
                    .font_weight(typography.weight)
                    .font_variations(typography.variations())
                    .text_color(if selected {
                        gpui::Hsla::from(__theme_colors_foreground)
                    } else {
                        gpui::Hsla::from(__theme_colors_muted_foreground)
                    });
                MenuRowAppearance {
                    row,
                    label,
                    icon: StyleRefinement::default()
                        .flex_none()
                        .size(px(glyph_size(tokens.size))),
                    shortcut: StyleRefinement::default()
                        .ml_auto()
                        .text_color(gpui::Hsla::from(__theme_colors_muted_foreground)),
                    indicator: StyleRefinement::default()
                        .flex_none()
                        .size(px(glyph_size(tokens.size))),
                    indicator_placement: MenuIndicatorPlacement::Trailing,
                }
            }
        }
    };
    let caption = TextRole::Caption.metrics(tokens.size, &tokens);
    MenuAppearance::new(
        {
            let tokens = tokens.clone();
            let style = style.clone();
            {
                let __theme_colors_foreground = colors.foreground;
                move |kind| {
                    StyleRefinement::default()
                        .relative()
                        .flex()
                        .flex_col()
                        .overflow_hidden()
                        .w(px(if kind == MenuSurfaceKind::Submenu {
                            224.
                        } else {
                            288.
                        }))
                        .max_w_full()
                        .max_h(px(480.))
                        .min_h_0()
                        .rounded(px(shape.container))
                        .bg(style.surface.color(scheme, &tokens))
                        .shadow(SurfaceLevel::Three.shadows(scheme, &tokens))
                        .font_family(tokens.typography.family.clone())
                        .font_weight(tokens.typography.normal.weight)
                        .font_variations(tokens.typography.normal.variations())
                        .text_size(px(metrics.text_size))
                        .line_height(px(metrics.line_height))
                        .text_color(gpui::Hsla::from(__theme_colors_foreground))
                }
            }
        },
        row,
    )
    .group_style(StyleRefinement::default().flex().flex_col().flex_none())
    .group_label_style({
        let __theme_colors_muted_foreground = colors.muted_foreground;
        move |_| {
            StyleRefinement::default()
                .px(px(8.))
                .py(px(6.))
                .text_size(px(caption.size))
                .line_height(px(caption.line_height))
                .text_color(gpui::Hsla::from(__theme_colors_muted_foreground))
        }
    })
    .separator_style(
        StyleRefinement::default()
            .my(px(4.))
            .mx(px(-4.))
            .h(px(1.))
            .flex_none()
            .bg(gpui::Hsla::from(colors.border).opacity(0.6)),
    )
}
fn project(facts: &MenuListPresentation) -> Option<ElementId> {
    if !facts.open {
        return None;
    }
    if facts.keyboard_origin {
        return facts
            .keyboard_focused
            .as_ref()
            .filter(|id| facts.rows.iter().any(|row| &row.id == *id && row.enabled))
            .cloned();
    }
    let Some(pointer) = facts.pointer_position else {
        return facts
            .focused
            .as_ref()
            .filter(|id| facts.rows.iter().any(|row| &row.id == *id && row.enabled))
            .cloned();
    };
    let mut hover = HoverMotion::new();
    hover.update(
        pointer,
        facts.rows.iter().filter_map(|row| {
            Some(HoverItem {
                id: row.id.clone(),
                bounds: row.bounds?,
                enabled: row.enabled,
            })
        }),
        HoverAxis::Vertical,
        HoverTarget::Nearest,
    );
    hover.snapshot().active
}
struct Layers {
    session: u64,
    hover: HoverMotion,
    selected: HoverSnapshot,
}
impl Layers {
    fn new() -> Self {
        Self {
            session: 0,
            hover: HoverMotion::new(),
            selected: HoverSnapshot {
                active: None,
                bounds: None,
                session: 0,
            },
        }
    }
}
/// Keeps native separators and group-label gaps out of merged checked backgrounds.
fn selected_rows(facts: &MenuListPresentation) -> Vec<SelectionRow> {
    let mut rows = Vec::with_capacity(facts.rows.len());
    let mut previous_bottom = None;
    for row in facts.rows.iter() {
        if let Some(bounds) = row.bounds {
            if previous_bottom.is_some_and(|bottom| bounds.top() > bottom) {
                rows.push(SelectionRow {
                    bounds: None,
                    selected: false,
                });
            }
            previous_bottom = Some(bounds.bottom());
        }
        rows.push(SelectionRow {
            bounds: row.bounds.map(|bounds| {
                Bounds::new(bounds.origin - facts.viewport_bounds.origin, bounds.size)
            }),
            selected: checked(row.kind),
        });
    }
    rows
}

pub(crate) fn layers(id: ElementId, tokens: ComponentTheme, scheme: ThemeScheme) -> MenuListLayers {
    let colors = tokens.extras(scheme);
    MenuListLayers::new().behind({
        let tokens = tokens.clone();
        {
            let __theme_colors_active = colors.active();
            let __theme_colors_hover = colors.hover();
            move |facts, window, cx| {
                window.with_element_namespace(facts.owner_id.clone(), |window| {
                    let runtime =
                        window.use_keyed_state((id.clone(), "flow-dropdown-layers"), cx, |_, _| {
                            Layers::new()
                        });
                    let (hover, selected) = runtime.update(cx, |state, _| {
                        if state.session != facts.open_session {
                            *state = Layers::new();
                            state.session = facts.open_session;
                        }
                        let active = if facts.open {
                            facts.decorative_active_id.clone()
                        } else {
                            state.hover.snapshot().active
                        };
                        let target = facts
                            .rows
                            .iter()
                            .find(|row| Some(&row.id) == active.as_ref());
                        if let Some((row, bounds)) =
                            target.and_then(|row| row.bounds.map(|bounds| (row, bounds)))
                        {
                            state.hover.update(
                                bounds.center(),
                                std::iter::once(HoverItem {
                                    id: row.id.clone(),
                                    bounds,
                                    enabled: true,
                                }),
                                HoverAxis::Vertical,
                                HoverTarget::Containing,
                            );
                        } else {
                            state.hover.clear();
                        }
                        let selected = facts.rows.iter().find(|row| checked(row.kind));
                        state.selected.active = selected.map(|row| row.id.clone());
                        if let Some(bounds) = selected.and_then(|row| row.bounds) {
                            state.selected.bounds = Some(bounds);
                        }
                        state.selected.session = facts.open_session as usize;
                        let mut hover = state.hover.snapshot();
                        let mut selected = state.selected.clone();
                        for snapshot in [&mut hover, &mut selected] {
                            snapshot.bounds = snapshot.bounds.map(|bounds| {
                                Bounds::new(
                                    bounds.origin - facts.viewport_bounds.origin,
                                    bounds.size,
                                )
                            });
                        }
                        (hover, selected)
                    });
                    let mut hover_highlight = HoverHighlight::new(
                        "hover",
                        hover,
                        __theme_colors_hover,
                        px(tokens.shape.popup().radii(&tokens).item),
                        tokens.motion.fast,
                    )
                    .initial_opacity(0.);
                    if !facts.keyboard_origin
                        && facts.pointer_position.is_some()
                        && let Some(origin) = selected.bounds
                    {
                        hover_highlight = hover_highlight.initial_bounds(origin);
                    }
                    div()
                        .relative()
                        .size_full()
                        .child(SelectionBackgrounds::new(
                            (
                                id.clone(),
                                gpui::SharedString::from(format!("checked-{}", facts.open_session)),
                            ),
                            selected_rows(facts),
                            px(tokens.shape.popup().radii(&tokens).item),
                            __theme_colors_active,
                            tokens.motion.moderate,
                            tokens.clone(),
                        ))
                        .child(hover_highlight)
                        .into_any_element()
                })
            }
        }
    })
}
/// Styles the native menu without duplicating command activation, navigation or submenu ownership.
pub fn dropdown_menu<T: Clone + 'static>(
    id: impl Into<ElementId>,
    open: bool,
    trigger: Button,
    style: DropdownStyle,
    window: &mut Window,
    cx: &mut App,
) -> Menu<T> {
    let trigger = trigger
        .has_popup(Some(gpui::accesskit::HasPopup::Menu))
        .expanded(open)
        .into_primitive(window, cx);
    dropdown_menu_with_trigger(id, open, trigger, style.clone(), window, cx)
}

/// Applies the same recipe to an application-styled native trigger.
pub fn dropdown_menu_with_trigger<T: Clone + 'static>(
    id: impl Into<ElementId>,
    open: bool,
    trigger: kinu_ui::Button,
    style: DropdownStyle,
    window: &mut Window,
    cx: &mut App,
) -> Menu<T> {
    let id = id.into();
    present_dropdown_menu(
        Menu::new(id.clone(), open)
            .focus_first_item()
            .trigger(trigger)
            .content(div()),
        id,
        style,
        window,
        cx,
    )
}

/// Styles an already prepared native menu without replacing its choices or input handlers.
pub fn present_dropdown_menu<T: Clone + 'static>(
    menu: Menu<T>,
    id: impl Into<ElementId>,
    style: DropdownStyle,
    window: &Window,
    cx: &App,
) -> Menu<T> {
    let id = id.into();
    let tokens = style.tokens.clone();
    let scheme = scheme(window, cx);
    let parts = parts(tokens.clone(), scheme);
    menu.appearance(appearance(style.clone(), scheme))
        .parts_presentation(parts)
        .list_layers(layers(id, tokens.clone(), scheme))
        .viewport_presentation(viewport(tokens.clone(), scheme))
        .motion(popup_motion(tokens.motion.fast, px(4.)))
        .motion_axes(kinu_ui::PopupMotionAxes::Vertical)
        .min_anchor_width(px(0.))
        .modality(PopupModality::NonModal)
        .side(PopupSide::Bottom)
        .align(PopupAlign::Start)
        .side_offset(px(6.))
        .align_offset(px(0.))
}
/// Places a native submenu beside its registered trigger row.
pub fn dropdown_submenu<T>(
    id: impl Into<ElementId>,
    label: impl Into<gpui::SharedString>,
) -> MenuSubmenu<T> {
    MenuSubmenu::new(id, label, div())
        .side_offset(px(6.))
        .align_offset(px(-4.))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn dropdown_icon(
    name: IconName,
    cx: &App,
) -> impl Fn(&MenuIconPresentation, &mut Window, &mut App) -> AnyElement + use<> {
    dropdown_icon_with_tokens(name, ComponentTheme::current(cx))
}

pub(crate) fn parts(tokens: ComponentTheme, scheme: ThemeScheme) -> MenuPartsPresentation {
    let colors = tokens.extras(scheme);
    MenuPartsPresentation::new()
        .list_style(
            StyleRefinement::default()
                .relative()
                .flex()
                .flex_col()
                .min_h_0()
                .p(px(4.)),
        )
        .decorative_active_row(project)
        .label_transition(
            StyleTransition::new(tokens.motion.animations.color.animation()).properties([
                gpui::StyleTransitionProperty::TextColor,
                gpui::StyleTransitionProperty::FontWeight,
                gpui::StyleTransitionProperty::FontVariations,
            ]),
        )
        .row({
            let foreground = gpui::Hsla::from(colors.foreground);
            let muted = gpui::Hsla::from(colors.muted_foreground);
            move |facts, mut parts| {
                // A command-specific ink override must survive the shared hover treatment.
                let inherited = parts.label.text.color;
                if (facts.decorative_active || checked(facts.kind))
                    && (inherited.is_none() || inherited == Some(muted))
                {
                    parts.label = parts.label.text_color(foreground);
                }
                parts
            }
        })
        .indicator({
            let tokens = tokens.clone();
            move |facts, window, cx| indicator(facts, tokens.clone(), window, cx)
        })
}

pub(crate) fn viewport(
    tokens: ComponentTheme,
    scheme: ThemeScheme,
) -> impl Fn(kinu_ui::ScrollViewportPresentation, &Window, &App) -> kinu_ui::ScrollViewportPresentation
{
    move |viewport, _, _| {
        viewport
            .scrollbar(scrollbar(gpui::Axis::Vertical, tokens.clone(), scheme))
            .mask(move |bounds, state, window| {
                paint_scroll_fade(bounds, state, gpui::Axis::Vertical, px(32.), window)
            })
    }
}
