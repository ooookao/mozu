use crate::theme::Theme;
use crate::ui::icon::{Icon, IconName};
use crate::ui::utils::hover::{
    HoverAxis, HoverHighlight, HoverItem, HoverMotion, HoverOverlayTarget as OverlayTarget,
    HoverTarget, nearest_enabled,
};
use crate::ui::utils::presentation::{
    ComponentTheme, ControlSize, SurfaceLevel, TextRole, ThemeScheme,
};
use crate::ui::utils::selection::choice_label;
use gpui::prelude::*;
use gpui::{
    AnimationExt, AnyElement, App, Bounds, ElementId, IntoElement, Pixels, Point, StyleRefinement,
    Styled, Window, div, point, px,
};
use kinu_ui::{
    ColorScheme, PopupBackdrop, PopupExit, PopupPhase, PopupPresence, PopupVisibility, Sidebar,
    SidebarAppearance, SidebarMenu, SidebarMenuListPresentation, SidebarMenuNavigation,
    SidebarMenuPresentation,
};
pub use kinu_ui::{
    SidebarActions, SidebarChange, SidebarChangeReason, SidebarCollapse, SidebarFacts,
    SidebarMenuDisplay, SidebarMenuItem, SidebarMenuSuffixSpacing, SidebarMobileFocus,
    SidebarMotion, SidebarPeek, SidebarShortcut, SidebarSide, SidebarState,
};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarVariant {
    Sidebar,
    Floating,
    Inset,
}

/// Editable sidebar surfaces and motion, resolved from the project's single theme.
#[derive(Clone)]
pub struct SidebarStyle {
    pub tokens: ComponentTheme,
    pub variant: SidebarVariant,
    /// None inherits the surrounding visual surface.
    pub surface: Option<SurfaceLevel>,
    pub focus_ring: bool,
    pub spotlight: Option<ElementId>,
    /// None leaves the rail tooltip under native pointer ownership.
    pub rail_tooltip: Option<bool>,
}
impl SidebarStyle {
    /// Reads presentation settings from the application's shared theme.
    pub fn current(variant: SidebarVariant, cx: &gpui::App) -> Self {
        Self::new(ComponentTheme::current(cx), variant)
    }
    pub fn new(tokens: ComponentTheme, variant: SidebarVariant) -> Self {
        Self {
            tokens,
            variant,
            surface: None,
            focus_ring: true,
            spotlight: None,
            rail_tooltip: None,
        }
    }
}
fn scheme(window: &Window, cx: &App) -> ThemeScheme {
    match ColorScheme::current(window, cx) {
        ColorScheme::Dark => ThemeScheme::Dark,
        ColorScheme::Light => ThemeScheme::Light,
    }
}

/// Configures native resizing, collapse, modal focus, and peek without duplicating input rules.
pub fn sidebar(
    id: impl Into<ElementId>,
    state: SidebarState,
    style: SidebarStyle,
    window: &Window,
    cx: &App,
) -> Sidebar {
    let id = id.into();
    let tokens = &style.tokens;
    let surface = match style.surface {
        Some(surface) => surface,
        None => SurfaceLevel::current(window),
    };
    let scheme = scheme(window, cx);
    let colors = tokens.extras(scheme);
    let metrics = TextRole::Body.metrics(tokens.size, tokens);
    let radii = tokens.shape.radii(tokens);
    let panel = StyleRefinement::default()
        .flex()
        .flex_col()
        .min_h_0()
        .p(px(0.))
        .gap(px(0.))
        .text_color(gpui::Hsla::from(colors.foreground))
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.normal.weight)
        .text_size(px(metrics.size))
        .line_height(px(metrics.line_height));
    let mut main = StyleRefinement::default()
        .min_w_0()
        .min_h_0()
        .flex_1()
        .bg(gpui::Hsla::from(colors.background));
    let mut root = StyleRefinement::default().relative().flex().size_full();
    if style.variant == SidebarVariant::Inset {
        root = root.bg(surface.color(scheme, tokens));
        main = main
            .m(px(8.))
            .rounded(px(radii.container))
            .bg(SurfaceLevel::Two.color(scheme, tokens))
            .shadow(SurfaceLevel::Two.shadows(scheme, tokens));
    }
    let peek_inset = if style.variant == SidebarVariant::Floating {
        px(8.)
    } else {
        px(0.)
    };
    let peek_width = state.width
        - if style.variant == SidebarVariant::Floating {
            px(16.)
        } else {
            px(8.)
        };
    let rail = StyleRefinement::default().bg(gpui::transparent_black());
    let appearance = SidebarAppearance {
        root,
        panel,
        main,
        rail,
        panel_left: if style.variant == SidebarVariant::Sidebar {
            StyleRefinement::default()
                .border_r_1()
                .border_color(gpui::Hsla::from(colors.border))
        } else {
            StyleRefinement::default()
        },
        panel_right: if style.variant == SidebarVariant::Sidebar {
            StyleRefinement::default()
                .border_l_1()
                .border_color(gpui::Hsla::from(colors.border))
        } else {
            StyleRefinement::default()
        },
        rail_hover: StyleRefinement::default(),
        rail_grip: StyleRefinement::default(),
        rail_grip_hover: StyleRefinement::default().bg(gpui::Hsla::from(colors.border)),
        rail_focus: StyleRefinement::default()
            .outline_width(px(1.))
            .outline_color(gpui::Hsla::from(colors.brand)),
        peek: StyleRefinement::default()
            .p(px(0.))
            .m(px(0.))
            .mx(peek_inset)
            .w(peek_width)
            .h_auto()
            .top(px(8.))
            .bottom(px(8.))
            .border_0()
            .rounded(px(radii.container))
            .bg(surface.elevated(1).color(scheme, tokens))
            .shadow(SurfaceLevel::Three.shadows(scheme, tokens)),
        backdrop: Some(PopupBackdrop {
            color: gpui::black().opacity(if scheme == ThemeScheme::Dark {
                0.8
            } else {
                0.4
            }),
            blur_radius: px(0.),
        }),
        motion: Some(crate::ui::utils::motion::popup_motion(
            tokens.motion.moderate,
            px(0.),
        )),
        desktop_motion: Some({
            let motion = crate::ui::utils::motion::popup_motion(tokens.motion.slow, px(0.));
            SidebarMotion {
                enter: motion.enter,
                exit: motion.exit,
            }
        }),
        transition: None,
    };
    let rail_id = id.clone();
    let rail_style = style.clone();
    let panel_tokens = style.tokens.clone();
    let panel_variant = style.variant;
    let mut sidebar = Sidebar::new(id, state)
        .appearance(appearance)
        .rail_presentation(move |facts, rail, window, cx| {
            let offset = if rail_style.variant == SidebarVariant::Floating {
                if facts.side == SidebarSide::Left {
                    px(-8.)
                } else {
                    px(8.)
                }
            } else {
                px(0.)
            };
            div()
                .size_full()
                .translate_x(offset)
                .child(sidebar_rail_tooltip(
                    rail_id,
                    facts,
                    rail,
                    &rail_style,
                    window,
                    cx,
                ))
                .into_any_element()
        })
        .panel_presentation(move |kind, _, content, _, _| match kind {
            kinu_ui::SidebarPanelKind::Docked => {
                let frame = div().flex().flex_col().size_full().min_h_0();
                match panel_variant {
                    SidebarVariant::Sidebar => frame
                        .bg(surface.color(scheme, &panel_tokens))
                        .child(content)
                        .into_any_element(),
                    SidebarVariant::Floating => frame
                        .p(px(8.))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .size_full()
                                .min_h_0()
                                .rounded(px(radii.container))
                                .bg(surface.elevated(1).color(scheme, &panel_tokens))
                                .shadow(SurfaceLevel::Three.shadows(scheme, &panel_tokens))
                                .child(content),
                        )
                        .into_any_element(),
                    SidebarVariant::Inset => frame
                        .py(px(8.))
                        .bg(surface.color(scheme, &panel_tokens))
                        .child(content)
                        .into_any_element(),
                }
            }
            kinu_ui::SidebarPanelKind::Mobile => div()
                .flex()
                .flex_col()
                .size_full()
                .min_h_0()
                .bg(surface.elevated(2).color(scheme, &panel_tokens))
                .shadow(SurfaceLevel::Three.shadows(scheme, &panel_tokens))
                .child(content)
                .into_any_element(),
            kinu_ui::SidebarPanelKind::Peek => div()
                .flex()
                .flex_col()
                .size_full()
                .min_h_0()
                .child(content)
                .into_any_element(),
        })
        .panel_context(if style.variant == SidebarVariant::Floating {
            surface.elevated(1)
        } else {
            surface
        })
        .mobile_context(surface.elevated(2))
        .peek_context(surface.elevated(1))
        .main_context(surface)
        .collapse(SidebarCollapse::Offcanvas)
        .mobile_focus(SidebarMobileFocus::Panel)
        .shortcut(SidebarShortcut::Bracket)
        .resizable(true);
    if style.variant == SidebarVariant::Inset {
        sidebar = sidebar.main_presentation(|facts| {
            if facts.collapsed {
                StyleRefinement::default()
            } else {
                match facts.side {
                    SidebarSide::Left => StyleRefinement::default().ml(px(0.)),
                    SidebarSide::Right => StyleRefinement::default().mr(px(0.)),
                }
            }
        });
    }
    sidebar
}

#[derive(Clone, PartialEq, Eq)]
struct ActiveKey {
    level: ElementId,
    occurrence: usize,
}
struct ActiveTarget {
    key: ActiveKey,
    row: ElementId,
    bounds: Bounds<Pixels>,
}
struct ActiveOverlay {
    key: ActiveKey,
    serial: u64,
    target: OverlayTarget,
    presence: PopupPresence,
}
struct SidebarLayers {
    hover: HoverMotion,
    hover_target: OverlayTarget,
    focus_target: OverlayTarget,
    focus_presence: PopupPresence,
    focus_serial: u64,
    active: Vec<ActiveOverlay>,
    next_serial: u64,
}
fn local(bounds: Bounds<Pixels>, origin: Point<Pixels>) -> Bounds<Pixels> {
    Bounds::new(bounds.origin - origin, bounds.size)
}
fn active_targets(facts: &SidebarMenuListPresentation) -> Vec<ActiveTarget> {
    let mut occurrences = HashMap::<ElementId, usize>::new();
    facts
        .rows
        .iter()
        .filter(|row| row.selected && row.enabled)
        .filter_map(|row| {
            let occurrence = occurrences.entry(row.level_id.clone()).or_insert(0);
            let key = ActiveKey {
                level: row.level_id.clone(),
                occurrence: *occurrence,
            };
            *occurrence += 1;
            row.bounds.map(|bounds| ActiveTarget {
                key,
                row: row.id.clone(),
                bounds: local(bounds, facts.bounds.origin),
            })
        })
        .collect()
}
impl SidebarLayers {
    fn new() -> Self {
        Self {
            hover: HoverMotion::new(),
            hover_target: OverlayTarget::new(),
            focus_target: OverlayTarget::new(),
            focus_presence: PopupPresence::new(PopupVisibility::Hidden),
            focus_serial: 0,
            active: Vec::new(),
            next_serial: 0,
        }
    }
    fn update(
        &mut self,
        facts: &SidebarMenuListPresentation,
        exit: PopupExit,
        focus_exit: PopupExit,
        now: Instant,
    ) {
        self.active
            .retain(|entry| entry.presence.at(now).is_present());
        let hovered = facts
            .decorative_active_id
            .as_ref()
            .and_then(|id| facts.rows.iter().find(|row| &row.id == id && row.enabled))
            .and_then(|row| row.bounds.map(|bounds| (row.id.clone(), bounds)));
        if let Some((id, bounds)) = hovered {
            self.hover.update(
                bounds.center(),
                std::iter::once(HoverItem {
                    id,
                    bounds,
                    enabled: true,
                }),
                HoverAxis::Vertical,
                HoverTarget::Containing,
            );
        } else {
            self.hover.clear();
        }
        let hover = self.hover.snapshot();
        self.hover_target.retarget(
            hover.active.clone(),
            hover
                .active
                .as_ref()
                .and_then(|id| facts.rows.iter().find(|row| &row.id == id))
                .and_then(|row| row.bounds)
                .map(|bounds| local(bounds, facts.bounds.origin)),
            hover.session,
        );

        let focused = facts
            .focus_visible
            .then(|| facts.focused_id.clone())
            .flatten()
            .and_then(|id| facts.rows.iter().find(|row| row.id == id && row.enabled));
        let focus_rect = focused.and_then(|row| row.bounds).map(|bounds| {
            let bounds = local(bounds, facts.bounds.origin);
            Bounds::new(
                bounds.origin - point(px(2.), px(2.)),
                bounds.size + gpui::size(px(4.), px(4.)),
            )
        });
        let focus_id = focus_rect.and_then(|_| focused.map(|row| row.id.clone()));
        if focus_id.is_some() && !self.focus_presence.at(now).is_present() {
            self.focus_serial = self
                .focus_serial
                .checked_add(1)
                .expect("Sidebar focus overlay identity exhausted");
        }
        self.focus_presence = self.focus_presence.with_visibility(
            if focus_id.is_some() {
                PopupVisibility::Visible
            } else {
                PopupVisibility::Hidden
            },
            focus_exit,
            now,
        );
        self.focus_target.retarget(focus_id, focus_rect, 0);

        let targets = active_targets(facts);
        for target in &targets {
            if !self.active.iter().any(|entry| entry.key == target.key) {
                self.next_serial = self
                    .next_serial
                    .checked_add(1)
                    .expect("Sidebar active overlay identity exhausted");
                self.active.push(ActiveOverlay {
                    key: target.key.clone(),
                    serial: self.next_serial,
                    target: OverlayTarget::new(),
                    presence: PopupPresence::new(PopupVisibility::Visible),
                });
            }
        }
        for entry in &mut self.active {
            let target = targets.iter().find(|target| target.key == entry.key);
            entry.presence = entry.presence.with_visibility(
                if target.is_some() {
                    PopupVisibility::Visible
                } else {
                    PopupVisibility::Hidden
                },
                exit,
                now,
            );
            entry.target.retarget(
                target.map(|target| target.row.clone()),
                target.map(|target| target.bounds),
                0,
            );
        }
        self.active.retain(|entry| entry.presence.is_present());
    }
}
fn sidebar_background(
    id: &ElementId,
    facts: &SidebarMenuListPresentation,
    tokens: ComponentTheme,
    scheme: ThemeScheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let colors = tokens.extras(scheme);
    let radius = px(tokens.shape.radii(&tokens).item);
    let runtime = window.use_keyed_state((id.clone(), "flow-sidebar-layers"), cx, |_, _| {
        SidebarLayers::new()
    });
    let now = cx.background_executor().now();
    runtime.update(cx, |state, _| {
        state.update(
            facts,
            PopupExit::Timed(tokens.motion.moderate.exit),
            PopupExit::Timed(tokens.motion.fast.exit),
            now,
        )
    });
    let (selected, hover, pending) = {
        let state = runtime.read(cx);
        let selected = state
            .active
            .iter()
            .map(|entry| {
                HoverHighlight::new(
                    (id.clone(), format!("sidebar-active-{}", entry.serial)),
                    entry.target.snapshot.clone(),
                    colors.active(),
                    radius,
                    tokens.motion.moderate,
                )
                .geometry_playback(entry.target.playback())
                .initial_opacity(1.)
                .into_any_element()
            })
            .collect::<Vec<_>>();
        let hover = HoverHighlight::new(
            (id.clone(), "sidebar-hover"),
            state.hover_target.snapshot.clone(),
            colors.hover(),
            radius,
            tokens.motion.fast,
        )
        .geometry_playback(state.hover_target.playback())
        .initial_opacity(0.);
        (
            selected,
            hover,
            state
                .active
                .iter()
                .any(|entry| entry.presence.phase() == PopupPhase::Closing),
        )
    };
    if pending {
        window.request_animation_frame();
    }
    div()
        .id((id.clone(), "sidebar-backgrounds"))
        .relative()
        .size_full()
        .children(selected)
        .child(hover)
        .into_any_element()
}
fn sidebar_focus(
    id: &ElementId,
    tokens: ComponentTheme,
    scheme: ThemeScheme,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let colors = tokens.extras(scheme);
    let runtime = window.use_keyed_state((id.clone(), "flow-sidebar-layers"), cx, |_, _| {
        SidebarLayers::new()
    });
    let state = runtime.read(cx);
    if !state.focus_presence.is_present() {
        return gpui::Empty.into_any_element();
    }
    let pending = state.focus_presence.phase() == PopupPhase::Closing;
    let focus = HoverHighlight::new(
        (id.clone(), format!("sidebar-focus-{}", state.focus_serial)),
        state.focus_target.snapshot.clone(),
        gpui::transparent_black(),
        px(tokens.shape.radii(&tokens).focus),
        tokens.motion.fast,
    )
    .geometry_playback(state.focus_target.playback())
    .outline(px(1.), gpui::Hsla::from(colors.brand))
    .initial_opacity(1.)
    .into_any_element();
    if pending {
        window.request_animation_frame();
    }
    focus
}

fn nearest(facts: &SidebarMenuListPresentation) -> Option<ElementId> {
    if facts.focus_visible {
        return facts.focused_id.clone();
    }
    let pointer = facts.pointer_position?;
    let rows = facts
        .rows
        .iter()
        .filter_map(|row| {
            row.bounds.map(|bounds| HoverItem {
                id: row.id.clone(),
                bounds,
                enabled: row.enabled,
            })
        })
        .collect::<Vec<_>>();
    nearest_enabled(pointer, &rows, HoverAxis::Vertical)
}

/// Styles a native navigation tree with shared hover, selection, and focus layers.
pub fn sidebar_menu(
    id: impl Into<ElementId>,
    style: SidebarStyle,
    window: &Window,
    cx: &App,
) -> SidebarMenu {
    let id = id.into();
    let colors = style.tokens.extras(scheme(window, cx));
    let behind_id = id.clone();
    let above_id = id.clone();
    let label_tokens = style.tokens.clone();
    let row_tokens = style.tokens.clone();
    let behind_tokens = style.tokens.clone();
    let branch_tokens = style.tokens.clone();
    let profile = branch_tokens.motion.moderate;
    let easing = profile.exit_easing;
    let spotlight = style.spotlight.clone();
    SidebarMenu::new(id)
        .navigation(SidebarMenuNavigation::Roving)
        .branch_transition(kinu_ui::PanelTransition::Spring(profile.spring))
        .branch_exit_transition(kinu_ui::PanelTransition::CubicBezier(
            profile.exit,
            [easing.x1, easing.y1, easing.x2, easing.y2],
        ))
        .branch_fade(kinu_ui::PopupFade {
            enter: profile.entering(),
            exit: profile.exiting(),
            initial_opacity: 0.,
        })
        .branch_style(
            StyleRefinement::default()
                .ml(px(15.))
                .border_l_1()
                .border_color(gpui::Hsla::from(colors.border))
                .pl(px(8.)),
        )
        .flex()
        .flex_col()
        .gap(px(2.))
        .min_w_0()
        .w_full()
        .label_presentation(move |facts, window, cx| {
            let metrics = TextRole::Body.metrics(label_tokens.size, &label_tokens);
            let mut label = div()
                .flex()
                .items_center()
                .flex_1()
                .min_w_0()
                .h(window.pixel_snap(px(metrics.line_height)))
                .truncate()
                .child(choice_label(
                    facts.label.clone(),
                    facts.selected,
                    facts.decorative_active,
                    label_tokens.clone(),
                    window,
                    cx,
                ));
            if facts.has_children {
                let open = facts.expanded == Some(true);
                label = label.child(
                    div()
                        .id((facts.id.clone(), "branch-chevron-opacity"))
                        .flex_none()
                        .size(px(24.))
                        .mr(px(-2.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(if !open || facts.hovered || facts.focus_within {
                            1.
                        } else {
                            0.
                        })
                        .transition(gpui::StyleTransition::opacity(
                            gpui::Animation::new(std::time::Duration::from_millis(80))
                                .with_easing(gpui::cubic_bezier(0.25, 0.1, 0.25, 1.)),
                        ))
                        .child(
                            Icon::new(IconName::ChevronRight)
                                .size(if label_tokens.size == ControlSize::Default {
                                    16.
                                } else {
                                    14.
                                })
                                .stroke_width(1.5)
                                .with_spring(
                                    "branch-chevron",
                                    gpui::SpringAnimation::new(label_tokens.motion.fast.spring).to(
                                        if open {
                                            std::f32::consts::FRAC_PI_2
                                        } else {
                                            0.
                                        },
                                    ),
                                    |icon, angle| icon.rotate(gpui::radians(angle)),
                                ),
                        ),
                );
            }
            label.into_any_element()
        })
        .row_presentation({
            let __theme_colors_foreground = colors.foreground;
            let __theme_colors_muted_foreground = colors.muted_foreground;
            move |facts, button, _, _| {
                let metrics = TextRole::Body.metrics(row_tokens.size, &row_tokens);
                let height = match (facts.depth, row_tokens.size) {
                    (0, ControlSize::Default) => 32.,
                    (0, ControlSize::Compact) => 28.,
                    (_, ControlSize::Default) => 28.,
                    (_, ControlSize::Compact) => 24.,
                };
                let radius = row_tokens.shape.radii(&row_tokens).item;
                button
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .w_full()
                    .min_w_0()
                    .h(px(height))
                    .pl(px(8.))
                    .pr(facts.suffix_inset.unwrap_or(px(8.)))
                    .rounded(px(radius))
                    .font_family(row_tokens.typography.family.clone())
                    .font_weight(if facts.selected {
                        row_tokens.typography.semibold.weight
                    } else {
                        row_tokens.typography.normal.weight
                    })
                    .text_size(px(metrics.size))
                    .line_height(px(metrics.line_height))
                    .text_color(if facts.selected {
                        gpui::Hsla::from(__theme_colors_foreground)
                    } else {
                        gpui::Hsla::from(__theme_colors_muted_foreground)
                    })
            }
        })
        .row_layout(|_, primary, suffix| {
            let mut strip = div().relative().w_full().child(primary);
            if let Some(suffix) = suffix {
                strip = strip.child(
                    div()
                        .absolute()
                        .right_0()
                        .top_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .child(suffix),
                );
            }
            strip.into_any_element()
        })
        .presentation({
            let above_tokens = style.tokens;
            let presentation = SidebarMenuPresentation::new()
                .decorative_active_row(move |facts| {
                    if facts.focus_visible {
                        return facts.focused_id.clone();
                    }
                    if facts
                        .pointer_position
                        .is_some_and(|pointer| facts.bounds.contains(&pointer))
                    {
                        return nearest(facts);
                    }
                    spotlight.clone()
                })
                .behind(move |facts, window, cx| {
                    sidebar_background(
                        &behind_id,
                        facts,
                        behind_tokens.clone(),
                        scheme(window, cx),
                        window,
                        cx,
                    )
                });
            if style.focus_ring {
                presentation.above(move |_, window, cx| {
                    sidebar_focus(
                        &above_id,
                        above_tokens.clone(),
                        scheme(window, cx),
                        window,
                        cx,
                    )
                })
            } else {
                presentation
            }
        })
}

/// Styles the native text input on the sidebar row's density and surface ladder.
/// Editing, selection, IME, focus, and accessibility remain owned by the native input.
pub fn sidebar_input_with_tokens(
    id: impl Into<gpui::ElementId>,
    value: impl Into<gpui::SharedString>,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> kinu_ui::Input {
    let colors = tokens.extras(scheme(window, cx));
    let theme = Theme::current(window, cx);
    let metrics = TextRole::Body.metrics(tokens.size, &tokens);
    let height = match tokens.size {
        ControlSize::Default => 32.,
        ControlSize::Compact => 28.,
    };
    kinu_ui::Input::new(id, value).appearance({
        let __theme_colors_border = colors.border;
        let __theme_colors_brand = colors.brand;
        let __theme_colors_foreground = colors.foreground;
        let __theme_colors_muted_foreground = colors.muted_foreground;
        move |state, _, _| {
            let ring = if state.focus_visible {
                gpui::Hsla::from(__theme_colors_brand)
            } else if state.focused || state.hovered {
                gpui::Hsla::from(__theme_colors_border)
            } else {
                gpui::transparent_black()
            };
            let background = if state.focused {
                gpui::Hsla::from(theme.card)
            } else if state.hovered {
                gpui::Hsla::from(theme.muted).opacity(0.5)
            } else {
                gpui::transparent_black()
            };
            kinu_ui::TextInputAppearance {
                style: StyleRefinement::default()
                    .w_full()
                    .min_w_0()
                    .h(px(height))
                    .px(px(12.))
                    .border_0()
                    .bg(background)
                    .shadow(vec![
                        gpui::BoxShadow::new(px(0.), px(0.), ring).spread_radius(px(1.)),
                    ])
                    .rounded(px(tokens.shape.radii(&tokens).item))
                    .font_family(tokens.typography.family.clone())
                    .font_weight(tokens.typography.normal.weight)
                    .font_variations(tokens.typography.normal.variations())
                    .text_size(px(metrics.size))
                    .line_height(px(metrics.line_height))
                    .text_color(gpui::Hsla::from(__theme_colors_foreground)),
                placeholder_color: Some(gpui::Hsla::from(__theme_colors_muted_foreground)),
                resize: None,
                transition: Some(
                    gpui::StyleTransition::new(tokens.motion.animations.color.animation())
                        .properties([
                            gpui::StyleTransitionProperty::Background,
                            gpui::StyleTransitionProperty::BoxShadow,
                        ]),
                ),
            }
        }
    })
}

/// Letter mark shared by a workspace trigger and its menu rows.
pub fn sidebar_workspace_tile_with_tokens(
    letter: impl Into<gpui::SharedString>,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    let colors = tokens.extras(scheme(window, cx));
    let radius = match tokens.shape {
        crate::ui::utils::presentation::Shape::Pill => 10.,
        crate::ui::utils::presentation::Shape::Sharp => 1.,
        crate::ui::utils::presentation::Shape::Rounded => 6.,
    };
    div()
        .flex()
        .items_center()
        .justify_center()
        .size(px(20.))
        .flex_none()
        .rounded(px(radius))
        .bg(gpui::Hsla::from(colors.foreground))
        .text_color(gpui::Hsla::from(colors.background))
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.semibold.weight)
        .text_size(px(10.))
        .line_height(px(12.))
        .child(letter.into())
}

/// A twenty-pixel workspace mark centred on the current menu icon slot.
pub fn sidebar_workspace_menu_tile_with_tokens(
    letter: impl Into<gpui::SharedString>,
    background: gpui::Hsla,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    let slot = if tokens.size == ControlSize::Default {
        16.
    } else {
        14.
    };
    let offset = px((slot - 20.) / 2.);
    div().relative().size(px(slot)).child(
        sidebar_workspace_tile_with_tokens(letter, tokens, window, cx)
            .absolute()
            .left(offset)
            .top(offset)
            .bg(background),
    )
}

/// Caller-owned visual elements for an identity row. Native menu ownership is preserved.
pub struct SidebarIdentitySlots {
    pub leading: AnyElement,
    pub label: AnyElement,
    pub trailing: AnyElement,
}

/// Builds the default identity geometry while leaving each visual element editable.
pub fn sidebar_identity_slots(
    label: impl Into<gpui::SharedString>,
    leading: impl IntoElement,
    trailing: IconName,
    style: &SidebarStyle,
    window: &Window,
    cx: &App,
) -> SidebarIdentitySlots {
    let colors = style.tokens.extras(scheme(window, cx));
    SidebarIdentitySlots {
        leading: div()
            .flex_none()
            .ml(px(-2.))
            .mr(px(-2.))
            .size(px(20.))
            .child(leading)
            .into_any_element(),
        label: div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .whitespace_nowrap()
            .child(label.into())
            .into_any_element(),
        trailing: div()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .size(px(24.))
            .mr(px(-2.))
            .text_color(gpui::Hsla::from(colors.muted_foreground))
            .child(
                Icon::new(trailing)
                    .size(if style.tokens.size == ControlSize::Default {
                        16.
                    } else {
                        14.
                    })
                    .stroke_width(1.5),
            )
            .into_any_element(),
    }
}
fn sidebar_identity_control(
    id: ElementId,
    style: &SidebarStyle,
    window: &Window,
    cx: &App,
) -> kinu_ui::Button {
    let tokens = &style.tokens;
    let colors = tokens.extras(scheme(window, cx));
    let metrics = TextRole::Body.metrics(tokens.size, tokens);
    kinu_ui::Button::new(id)
        .flex()
        .items_center()
        .w_full()
        .h(px(if tokens.size == ControlSize::Default {
            32.
        } else {
            28.
        }))
        .gap(px(8.))
        .px(px(8.))
        .min_w_0()
        .rounded(px(tokens.shape.radii(tokens).item))
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.normal.weight)
        .text_size(px(metrics.size))
        .line_height(px(metrics.line_height))
        .text_color(gpui::Hsla::from(colors.foreground))
        .hover({
            let __theme_colors_hover = colors.hover();
            move |s| s.bg(__theme_colors_hover)
        })
}
fn sidebar_identity_trigger(
    id: ElementId,
    slots: SidebarIdentitySlots,
    style: &SidebarStyle,
    window: &Window,
    cx: &App,
) -> kinu_ui::Button {
    sidebar_identity_control(id, style, window, cx)
        .child(slots.leading)
        .child(slots.label)
        .child(slots.trailing)
}

/// A controlled workspace switcher; callers supply the menu's data and callbacks.
pub fn sidebar_workspace_header<T: Clone + 'static>(
    id: impl Into<ElementId>,
    open: bool,
    name: impl Into<gpui::SharedString>,
    tile: impl IntoElement,
    style: SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> kinu_ui::Menu<T> {
    let slots = sidebar_identity_slots(name, tile, IconName::ChevronDown, &style, window, cx);
    sidebar_workspace_header_with_slots(id, open, slots, style, window, cx)
}
fn workspace_trigger_layout(
    trigger: kinu_ui::Button,
    slots: SidebarIdentitySlots,
    overlay: Option<AnyElement>,
    height: Pixels,
) -> AnyElement {
    gpui::container_query(move |size, _, _| {
        let trigger = trigger.child(slots.leading).child(slots.label);
        let trigger = if size.width < px(112.) {
            trigger
        } else {
            trigger.child(slots.trailing)
        };
        let mut row = div().relative().w_full().h(height).child(trigger);
        if let Some(overlay) = overlay {
            row = row.child(overlay);
        }
        row
    })
    .w_full()
    .h(height)
    .min_w_0()
    .into_any_element()
}

// The browser recipe declares anchor+10px and a -4px inline margin. Its
// shrink-to-fit positioner measures that margin box first; max-width:100%
// then caps the popup to anchor+6px (or its 240px minimum). Native alignment
// is paint placement, not a margin-box contribution, so use that resolved
// width offset explicitly while retaining the source's -4px alignment.
fn sidebar_identity_popup<T: Clone + 'static>(menu: kinu_ui::Menu<T>) -> kinu_ui::Menu<T> {
    menu.content(div().w_auto().min_w(px(240.)))
        .match_anchor_width_with_offset(px(10. - 4.))
        .align_offset(px(-4.))
}

/// Applies the workspace menu recipe to caller-owned visual slots.
pub fn sidebar_workspace_header_with_slots<T: Clone + 'static>(
    id: impl Into<ElementId>,
    open: bool,
    slots: SidebarIdentitySlots,
    style: SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> kinu_ui::Menu<T> {
    let id = id.into();
    let height = px(if style.tokens.size == ControlSize::Default {
        32.
    } else {
        28.
    });
    let trigger = sidebar_identity_control((id.clone(), "trigger").into(), &style, window, cx)
        .accessibility_label("Switch workspace")
        .font_weight(style.tokens.typography.semibold.weight);
    sidebar_identity_popup(
        crate::ui::dropdown_menu::dropdown_menu_with_trigger(
            id,
            open,
            trigger,
            crate::ui::dropdown_menu::DropdownStyle::new(style.tokens),
            window,
            cx,
        )
        .trigger_presentation(move |trigger, _, _| {
            workspace_trigger_layout(trigger, slots, None, height)
        }),
    )
    .side_offset(px(4.))
}

/// A controlled user menu with an independently supplied twenty-pixel avatar.
pub fn sidebar_user_footer<T: Clone + 'static>(
    id: impl Into<ElementId>,
    open: bool,
    name: impl Into<gpui::SharedString>,
    avatar: impl IntoElement,
    style: SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> kinu_ui::Menu<T> {
    let border = if scheme(window, cx) == ThemeScheme::Dark {
        gpui::white().opacity(0.1)
    } else {
        gpui::black().opacity(0.1)
    };
    let avatar = div().relative().size(px(20.)).child(avatar).child(
        div()
            .absolute()
            .inset_0()
            .rounded_full()
            .border_1()
            .border_color(border)
            .pointer_events(false),
    );
    let slots = sidebar_identity_slots(name, avatar, IconName::ChevronsUpDown, &style, window, cx);
    sidebar_user_footer_with_slots(id, open, slots, style, window, cx)
}
/// Applies the upward user-menu recipe to caller-owned visual slots.
pub fn sidebar_user_footer_with_slots<T: Clone + 'static>(
    id: impl Into<ElementId>,
    open: bool,
    slots: SidebarIdentitySlots,
    style: SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> kinu_ui::Menu<T> {
    let id = id.into();
    let trigger =
        sidebar_identity_trigger((id.clone(), "trigger").into(), slots, &style, window, cx)
            .accessibility_label("Open user menu");
    sidebar_identity_popup(crate::ui::dropdown_menu::dropdown_menu_with_trigger(
        id,
        open,
        trigger,
        crate::ui::dropdown_menu::DropdownStyle::new(style.tokens),
        window,
        cx,
    ))
    .side(kinu_ui::PopupSide::Top)
    .side_offset(px(6.))
}

/// Visual and accessibility state of an assistant thread row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidebarThreadStatus {
    Active,
    Unread,
    Idle,
}

/// Styles a 24px sidebar action; activation and popup composition remain caller-owned.
pub fn sidebar_action_button_with_tokens(
    id: impl Into<ElementId>,
    label: impl Into<gpui::SharedString>,
    icon: IconName,
    tokens: ComponentTheme,
) -> crate::ui::button::Button {
    let size = match tokens.size {
        ControlSize::Default => crate::ui::button::ButtonSize::Icon,
        ControlSize::Compact => crate::ui::button::ButtonSize::IconCompact,
    };
    crate::ui::button::Button::new(id)
        .accessibility_label(label)
        .variant(crate::ui::button::ButtonVariant::Ghost)
        .size(size)
        .tokens(tokens)
        .flex_none()
        .w(px(24.))
        .h(px(24.))
        .p(px(0.))
        .leading_icon(move |slot| {
            Icon::new(icon)
                .size(f32::from(slot.size))
                .stroke_width(slot.stroke_width)
                .into_any_element()
        })
}

/// Returns the badge's reserved trailing slot from native row state.
pub fn sidebar_menu_badge_with_tokens(
    value: impl Into<gpui::SharedString>,
    tokens: ComponentTheme,
) -> impl FnOnce(&kinu_ui::SidebarMenuRowFacts, &mut Window, &mut App) -> AnyElement {
    let value = value.into();
    move |facts, window, cx| {
        let colors = tokens.extras(scheme(window, cx));
        let metrics = TextRole::Micro.metrics(tokens.size, &tokens);
        let lit = facts.selected;
        let weight = if lit {
            tokens.typography.semibold
        } else {
            tokens.typography.normal
        };
        div()
            .id((facts.id.clone(), "sidebar-badge"))
            .flex_none()
            .h(px(20.))
            .min_w(px(20.))
            .mr(px(8.))
            .px(px(4.))
            .flex()
            .items_center()
            .justify_center()
            .pointer_events(false)
            .font_family(tokens.typography.family.clone())
            .font_weight(weight.weight)
            .font_variations(weight.variations())
            .font_features(crate::ui::utils::presentation::tabular_numbers())
            .text_size(px(metrics.size))
            .line_height(px(metrics.line_height))
            .text_color(if lit {
                gpui::Hsla::from(colors.foreground)
            } else {
                gpui::Hsla::from(colors.muted_foreground)
            })
            .transition(gpui::StyleTransition::typography(
                gpui::Animation::new(std::time::Duration::from_millis(80))
                    .with_easing(gpui::cubic_bezier(0.25, 0.1, 0.25, 1.)),
            ))
            .child(value)
            .into_any_element()
    }
}

struct SidebarGroupFocus {
    trigger: gpui::FocusHandle,
    scope: gpui::FocusHandle,
    _subscriptions: Vec<gpui::Subscription>,
}
/// Explicit focus registration and artwork for a collapsible section header.
pub struct SidebarGroupChevron {
    pub focus: gpui::FocusHandle,
    pub scope: gpui::FocusHandle,
    pub element: AnyElement,
}
/// Builds the section's size-aware rotation/reveal; the native disclosure owns activation.
pub fn sidebar_group_chevron_with_tokens(
    id: impl Into<ElementId>,
    open: bool,
    hover_group: gpui::SharedString,
    tokens: ComponentTheme,
    window: &mut Window,
    cx: &mut App,
) -> SidebarGroupChevron {
    let id = id.into();
    let state = window.use_keyed_state((id.clone(), "group-chevron-focus"), cx, |window, cx| {
        let trigger = cx.focus_handle();
        let scope = cx.focus_handle().tab_stop(false);
        let subscriptions = vec![
            cx.on_focus_in(&scope, window, |_, _, cx| cx.notify()),
            cx.on_focus_out(&scope, window, |_, _, _, cx| cx.notify()),
        ];
        SidebarGroupFocus {
            trigger,
            scope,
            _subscriptions: subscriptions,
        }
    });
    let state = state.read(cx);
    let focus = state.trigger.clone();
    let scope = state.scope.clone();
    let revealed = !open || scope.contains_focused(window, cx);
    let element = div()
        .id((id, "group-chevron"))
        .ml_auto()
        .h(px(24.))
        .w(px(if revealed { 24. } else { 0. }))
        .opacity(if revealed { 1. } else { 0. })
        .overflow_hidden()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .group_hover(hover_group, |style| style.w(px(24.)).opacity(1.))
        .child(
            Icon::new(IconName::ChevronRight)
                .size(if tokens.size == ControlSize::Default {
                    16.
                } else {
                    14.
                })
                .stroke_width(1.5)
                .with_spring(
                    "section-chevron-rotation",
                    gpui::SpringAnimation::new(tokens.motion.fast.spring).to(if open {
                        std::f32::consts::FRAC_PI_2
                    } else {
                        0.
                    }),
                    |icon, angle| icon.rotate(gpui::radians(angle)),
                ),
        );
    SidebarGroupChevron {
        focus,
        scope,
        element: element.into_any_element(),
    }
}

/// Supplies row artwork from native interaction facts, using the shared color/stroke tween.
pub fn sidebar_menu_icon(
    name: IconName,
    style: SidebarStyle,
) -> impl FnOnce(&kinu_ui::SidebarMenuRowFacts, &mut Window, &mut App) -> AnyElement {
    move |facts, window, cx| {
        let tokens = &style.tokens;
        let colors = tokens.extras(scheme(window, cx));
        let active = facts.selected || facts.decorative_active;
        let color = if active {
            gpui::Hsla::from(colors.foreground)
        } else {
            gpui::Hsla::from(colors.muted_foreground)
        };
        let stroke = if active { 2. } else { 1.5 };
        let now = cx.background_executor().now();
        let runtime = window.use_keyed_state(
            (facts.id.clone(), "flow-sidebar-leading-icon"),
            cx,
            |_, _| {
                crate::ui::utils::motion::ColorStrokeTween::new(
                    color,
                    stroke,
                    now,
                    tokens.motion.animations.color.easing,
                )
            },
        );
        let (color, stroke, done) = runtime.update(cx, |state, _| {
            state.retarget(
                color,
                stroke,
                now,
                std::time::Duration::from_millis(tokens.motion.animations.color.duration_ms),
                tokens.motion.animations.color.easing,
            );
            state.sample(now)
        });
        if !done {
            window.request_animation_frame();
        }
        Icon::new(name)
            .size(if tokens.size == ControlSize::Default {
                16.
            } else {
                14.
            })
            .stroke_width(stroke)
            .text_color(color)
            .into_any_element()
    }
}

/// A status mark aligned with ordinary sidebar row icons.
pub fn sidebar_thread_dot(
    status: SidebarThreadStatus,
    style: SidebarStyle,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    let colors = style.tokens.extras(scheme(window, cx));
    let color = if status == SidebarThreadStatus::Active {
        gpui::Hsla::from(colors.foreground).opacity(0.6)
    } else {
        gpui::Hsla::from(colors.muted_foreground).opacity(0.5)
    };
    sidebar_thread_dot_presentation(status, color, style)
}

/// Uses the native row's actual selected and decorative state without owning navigation.
pub fn sidebar_thread_dot_with_state(
    status: SidebarThreadStatus,
    facts: &kinu_ui::SidebarMenuRowFacts,
    style: SidebarStyle,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    let colors = style.tokens.extras(scheme(window, cx));
    let color =
        if status == SidebarThreadStatus::Active || facts.selected || facts.decorative_active {
            gpui::Hsla::from(colors.foreground).opacity(0.6)
        } else {
            gpui::Hsla::from(colors.muted_foreground).opacity(0.5)
        };
    sidebar_thread_dot_presentation(status, color, style)
}
fn sidebar_thread_dot_presentation(
    status: SidebarThreadStatus,
    color: gpui::Hsla,
    style: SidebarStyle,
) -> gpui::Div {
    let dot = div()
        .id("thread-status-dot")
        .size(px(8.))
        .rounded_full()
        .transition(gpui::StyleTransition::colors(
            gpui::Animation::new(std::time::Duration::from_millis(80))
                .with_easing(gpui::cubic_bezier(0.25, 0.1, 0.25, 1.)),
        ));
    let dot = match status {
        SidebarThreadStatus::Active | SidebarThreadStatus::Unread => dot.bg(color),
        SidebarThreadStatus::Idle => dot.border_1().border_color(color),
    };
    let slot = px(if style.tokens.size == ControlSize::Default {
        16.
    } else {
        14.
    });
    div()
        .size(slot)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(dot)
}

struct SearchChrome {
    hovered: bool,
    focus: gpui::FocusHandle,
    _subscriptions: Vec<gpui::Subscription>,
}

/// Places search decorations around a prepared native input without replacing editing.
pub fn sidebar_search_field(
    id: impl Into<ElementId>,
    input: kinu_ui::Input,
    shortcut: Option<gpui::SharedString>,
    style: SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let id = id.into();
    let state = window.use_keyed_state((id.clone(), "search-chrome"), cx, |window, cx| {
        let focus = cx.focus_handle();
        let subscriptions = vec![
            cx.on_focus_in(&focus, window, |_, _, cx| cx.notify()),
            cx.on_focus_out(&focus, window, |_, _, _, cx| cx.notify()),
        ];
        SearchChrome {
            hovered: false,
            focus,
            _subscriptions: subscriptions,
        }
    });
    let focus = state.read(cx).focus.clone();
    let revealed = state.read(cx).hovered || focus.contains_focused(window, cx);
    let colors = style.tokens.extras(scheme(window, cx));
    let metrics = TextRole::Micro.metrics(ControlSize::Default, &style.tokens);
    let hover = state.clone();
    let mut row = div()
        .id(id.clone())
        .relative()
        .w_full()
        .focus_scope(&focus)
        .on_hover(move |value, _, cx| {
            hover.update(cx, |state, cx| {
                if state.hovered != *value {
                    state.hovered = *value;
                    cx.notify();
                }
            })
        })
        .child(input.track_focus(&focus).pl(px(32.)).pr(px(48.)))
        .child(
            div()
                .absolute()
                .left(px(8.))
                .top_0()
                .bottom_0()
                .flex()
                .items_center()
                .pointer_events(false)
                .text_color(gpui::Hsla::from(colors.muted_foreground))
                .child(
                    Icon::new(IconName::Search)
                        .size(if style.tokens.size == ControlSize::Default {
                            16.
                        } else {
                            14.
                        })
                        .stroke_width(1.5),
                ),
        );
    if let Some(shortcut) = shortcut {
        row = row.child(
            div()
                .id((id, "shortcut"))
                .absolute()
                .right(px(8.))
                .top_0()
                .bottom_0()
                .flex()
                .items_center()
                .pointer_events(false)
                .font_family(style.tokens.typography.family.clone())
                .text_size(px(metrics.size))
                .line_height(px(metrics.line_height))
                .text_color(gpui::Hsla::from(colors.muted_foreground))
                .opacity(if revealed { 1. } else { 0. })
                .transition(gpui::StyleTransition::opacity(
                    style.tokens.motion.fast.entering(),
                ))
                .child(shortcut),
        );
    }
    row.into_any_element()
}

struct InsetTopbarPresentation {
    peeking: bool,
    visible: bool,
    timer: Option<gpui::Task<()>>,
}

/// The main-region toggle, preserving native sidebar activation and tooltip ownership.
pub fn sidebar_inset_topbar(
    id: impl Into<ElementId>,
    facts: &SidebarFacts,
    actions: SidebarActions,
    style: SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Div {
    let id = id.into();
    let peeking = facts.peeking;
    let state = window.use_keyed_state((id.clone(), "inset-topbar"), cx, |_, _| {
        InsetTopbarPresentation {
            peeking,
            visible: !peeking,
            timer: None,
        }
    });
    state.update(cx, |state, cx| {
        if state.peeking != peeking {
            state.peeking = peeking;
            state.timer = None;
            state.timer = Some(cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(200))
                    .await;
                let _ = this.update(cx, |state, cx| {
                    state.visible = !state.peeking;
                    state.timer = None;
                    cx.notify();
                });
            }));
        }
    });
    let visible = state.read(cx).visible;
    let tokens = style.tokens.clone();
    let size = if tokens.size == ControlSize::Default {
        crate::ui::button::ButtonSize::Icon
    } else {
        crate::ui::button::ButtonSize::IconCompact
    };
    let icon_size = if tokens.size == ControlSize::Default {
        16.
    } else {
        14.
    };
    let icon = Icon::new(IconName::Panel)
        .size(icon_size)
        .stroke_width(1.5)
        .rotate(gpui::radians(if facts.side == SidebarSide::Right {
            std::f32::consts::PI
        } else {
            0.
        }));
    let activate = actions.clone();
    let trigger = crate::ui::button::Button::new((id.clone(), "trigger"))
        .accessibility_label("Toggle Sidebar")
        .variant(crate::ui::button::ButtonVariant::Ghost)
        .size(size)
        .tokens(tokens.clone())
        .icon_start(icon)
        .on_click(move |_, window, cx| activate.toggle(window, cx))
        .into_primitive(window, cx)
        .opacity(if visible { 1. } else { 0. })
        .transition(gpui::StyleTransition::opacity(
            gpui::Animation::new(std::time::Duration::from_millis(160))
                .with_easing(gpui::cubic_bezier(0.25, 0.1, 0.25, 1.)),
        ));
    let trigger = actions.decorate_trigger(trigger);
    let tooltip = sidebar_toggle_tooltip(id, trigger, facts, &style, window, cx);
    div()
        .flex()
        .flex_none()
        .items_center()
        .h(px(48.))
        .gap(px(8.))
        .px(px(6.))
        .child(tooltip)
}

struct SidebarRailTooltipState {
    open: bool,
}
fn sidebar_rail_tooltip(
    id: ElementId,
    facts: &SidebarFacts,
    rail: AnyElement,
    style: &SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let state = window.use_keyed_state((id.clone(), "rail-tooltip"), cx, |_, _| {
        SidebarRailTooltipState { open: false }
    });
    let open = !facts.resizing && style.rail_tooltip.unwrap_or(state.read(cx).open);
    let mut tooltip_tokens = style.tokens.clone();
    tooltip_tokens.size = ControlSize::Default;
    let shortcut = if facts.side == SidebarSide::Right {
        "]"
    } else {
        "["
    };
    let bold = gpui::HighlightStyle {
        font_weight: Some(style.tokens.typography.semibold.weight),
        ..Default::default()
    };
    let drag = gpui::StyledText::new("Drag to resize").with_highlights([(0..4, bold.clone())]);
    let click = gpui::StyledText::new("Click to collapse").with_highlights([(0..5, bold)]);
    let content = div()
        .flex()
        .flex_col()
        .items_start()
        .gap(px(8.))
        .child(crate::ui::tooltip::tooltip_caption_with_tokens(
            drag.into_any_element(),
            &tooltip_tokens,
            cx,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(crate::ui::tooltip::tooltip_caption_with_tokens(
                    click.into_any_element(),
                    &tooltip_tokens,
                    cx,
                ))
                .child(crate::ui::tooltip::tooltip_shortcut_with_tokens(
                    shortcut,
                    TextRole::Micro.metrics(ControlSize::Compact, &tooltip_tokens),
                    &tooltip_tokens,
                    window,
                    cx,
                )),
        );
    crate::ui::tooltip::tooltip_with_content_with_tokens(
        (id.clone(), "rail-tooltip-surface"),
        open,
        "Drag to resize. Click to collapse",
        content,
        tooltip_tokens,
        window,
        cx,
    )
    .passive_trigger(
        div()
            .id((id, "rail-tooltip-trigger"))
            .size_full()
            .child(rail),
    )
    .follow_cursor(kinu_ui::TooltipCursorAxis::Vertical)
    .side(if facts.side == SidebarSide::Left {
        kinu_ui::PopupSide::Right
    } else {
        kinu_ui::PopupSide::Left
    })
    .side_offset(px(8.))
    .disabled(facts.resizing)
    .on_open_change(move |open, _, _, cx| {
        state.update(cx, |state, cx| {
            if state.open != open {
                state.open = open;
                cx.notify();
            }
        })
    })
    .size_full()
    .into_any_element()
}

struct SidebarToggleTooltipState {
    open: bool,
}
fn sidebar_toggle_tooltip(
    id: ElementId,
    trigger: kinu_ui::Button,
    facts: &SidebarFacts,
    style: &SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> kinu_ui::Tooltip {
    let state = window.use_keyed_state((id.clone(), "toggle-tooltip"), cx, |_, _| {
        SidebarToggleTooltipState { open: false }
    });
    let open = state.read(cx).open;
    let collapsed = if facts.mobile {
        !facts.state.mobile_open
    } else {
        facts.collapsed
    };
    let description = if collapsed {
        "Expand sidebar"
    } else {
        "Collapse sidebar"
    };
    let shortcut = if facts.side == SidebarSide::Right {
        "]"
    } else {
        "["
    };
    let mut tokens = style.tokens.clone();
    tokens.size = ControlSize::Default;
    let content = div()
        .flex()
        .items_center()
        .gap(px(6.))
        .child(crate::ui::tooltip::tooltip_caption_with_tokens(
            description.into_any_element(),
            &tokens,
            cx,
        ))
        .child(crate::ui::tooltip::tooltip_shortcut_with_tokens(
            shortcut,
            TextRole::Micro.metrics(ControlSize::Compact, &tokens),
            &tokens,
            window,
            cx,
        ));
    crate::ui::tooltip::tooltip_with_content_with_tokens(
        (id, "tooltip"),
        open,
        description,
        content,
        tokens,
        window,
        cx,
    )
    .trigger(trigger)
    .side(kinu_ui::PopupSide::Bottom)
    .on_open_change(move |open, _, _, cx| {
        state.update(cx, |state, cx| {
            if state.open != open {
                state.open = open;
                cx.notify();
            }
        })
    })
}
fn sidebar_pin_overlay(
    id: ElementId,
    facts: &SidebarFacts,
    actions: SidebarActions,
    style: &SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let pin = kinu_ui::Button::new((id.clone(), "pin"))
        .accessibility_label("Toggle Sidebar")
        .tab_stop(facts.peeking)
        .pointer_interactive(facts.peeking)
        .flex()
        .items_center()
        .justify_center()
        .w(px(28.))
        .h(px(28.))
        .bg(gpui::transparent_black())
        .child(
            Icon::new(IconName::Panel)
                .size(16.)
                .stroke_width(1.5)
                .rotate(gpui::radians(if facts.side == SidebarSide::Right {
                    std::f32::consts::PI
                } else {
                    0.
                })),
        )
        .on_click(move |_, window, cx| actions.toggle(window, cx));
    let tip = sidebar_toggle_tooltip(id.clone(), pin, facts, style, window, cx);
    let tip = if facts.peeking {
        tip.into_any_element()
    } else {
        kinu_ui::InertContent::new(tip).into_any_element()
    };
    div()
        .id((id, "pin-overlay"))
        .absolute()
        .left(px(4.))
        .top_0()
        .bottom_0()
        .flex()
        .items_center()
        .pointer_events(facts.peeking)
        .opacity(if facts.peeking { 1. } else { 0. })
        .transition(gpui::StyleTransition::opacity(
            gpui::Animation::new(std::time::Duration::from_millis(80))
                .with_easing(gpui::cubic_bezier(0.25, 0.1, 0.25, 1.)),
        ))
        .child(tip)
        .into_any_element()
}

/// A noninteractive brand lockup whose independent pin control appears during peek.
pub fn sidebar_logo_header(
    id: impl Into<ElementId>,
    name: impl Into<gpui::SharedString>,
    tile: impl IntoElement,
    facts: &SidebarFacts,
    actions: SidebarActions,
    style: SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> gpui::Div {
    let id = id.into();
    let colors = style.tokens.extras(scheme(window, cx));
    let metrics = TextRole::Body.metrics(style.tokens.size, &style.tokens);
    let tile = div()
        .id((id.clone(), "tile"))
        .absolute()
        .left(px(6.))
        .top_0()
        .bottom_0()
        .flex()
        .items_center()
        .pointer_events(false)
        .opacity(if facts.peeking { 0. } else { 1. })
        .transition(gpui::StyleTransition::opacity(
            gpui::Animation::new(std::time::Duration::from_millis(80))
                .with_easing(gpui::cubic_bezier(0.25, 0.1, 0.25, 1.)),
        ))
        .child(tile);
    let pin = sidebar_pin_overlay(id, facts, actions, &style, window, cx);
    div()
        .relative()
        .flex()
        .items_center()
        .h(px(32.))
        .pl(px(32.))
        .pr(px(8.))
        .min_w_0()
        .font_family(style.tokens.typography.family.clone())
        .font_weight(style.tokens.typography.semibold.weight)
        .text_size(px(metrics.size))
        .line_height(px(metrics.line_height))
        .text_color(gpui::Hsla::from(colors.foreground))
        .child(div().min_w_0().truncate().child(name.into()))
        .child(tile)
        .child(pin)
}

/// A workspace switcher with independent tile and pin controls cross-fading in place.
pub fn sidebar_workspace_peek_header<T: Clone + 'static>(
    id: impl Into<ElementId>,
    open: bool,
    name: impl Into<gpui::SharedString>,
    tile: impl IntoElement,
    facts: &SidebarFacts,
    actions: SidebarActions,
    style: SidebarStyle,
    window: &mut Window,
    cx: &mut App,
) -> kinu_ui::Menu<T> {
    let id = id.into();
    let height = px(if style.tokens.size == ControlSize::Default {
        32.
    } else {
        28.
    });
    let tile = div()
        .id((id.clone(), "tile-fade"))
        .opacity(if facts.peeking { 0. } else { 1. })
        .pointer_events(false)
        .transition(gpui::StyleTransition::opacity(
            gpui::Animation::new(std::time::Duration::from_millis(80))
                .with_easing(gpui::cubic_bezier(0.25, 0.1, 0.25, 1.)),
        ))
        .child(tile);
    let slots = sidebar_identity_slots(name, tile, IconName::ChevronDown, &style, window, cx);
    let pin = sidebar_pin_overlay(id.clone(), facts, actions, &style, window, cx);
    let trigger = sidebar_identity_control((id.clone(), "trigger").into(), &style, window, cx)
        .accessibility_label("Switch workspace")
        .font_weight(style.tokens.typography.semibold.weight);
    sidebar_identity_popup(
        crate::ui::dropdown_menu::dropdown_menu_with_trigger(
            id,
            open,
            trigger,
            crate::ui::dropdown_menu::DropdownStyle::new(style.tokens),
            window,
            cx,
        )
        .trigger_presentation(move |trigger, _, _| {
            workspace_trigger_layout(trigger, slots, Some(pin), height)
        }),
    )
    .side_offset(px(4.))
}

#[derive(Clone, Copy)]
struct SidebarGroupOpacity {
    open: bool,
    value: gpui::SpringState,
    updated: Instant,
    closing: Option<(Instant, f32)>,
}
impl SidebarGroupOpacity {
    fn new(open: bool, now: Instant) -> Self {
        Self {
            open,
            value: gpui::SpringState {
                position: if open { 1. } else { 0. },
                velocity: 0.,
            },
            updated: now,
            closing: None,
        }
    }
    fn exit_value(
        from: f32,
        elapsed: std::time::Duration,
        profile: crate::ui::utils::motion::MotionProfile,
    ) -> f32 {
        let progress = (elapsed.as_secs_f32() / profile.exit.as_secs_f32()).min(1.);
        from * (1. - (profile.exit_easing.curve())(progress))
    }
    fn sample(&mut self, now: Instant, profile: crate::ui::utils::motion::MotionProfile) {
        if self.open {
            self.value = profile.spring.step(
                self.value,
                1.,
                now.saturating_duration_since(self.updated).as_secs_f32(),
            );
            if (self.value.position - 1.).abs() <= 0.001 && self.value.velocity.abs() <= 0.001 {
                self.value = gpui::SpringState {
                    position: 1.,
                    velocity: 0.,
                };
            }
        } else if let Some((started, from)) = self.closing {
            let elapsed = now.saturating_duration_since(started);
            self.value.position = Self::exit_value(from, elapsed, profile);
            if elapsed >= profile.exit {
                self.value = gpui::SpringState {
                    position: 0.,
                    velocity: 0.,
                };
                self.closing = None;
            }
        }
        self.updated = now;
    }
    fn stop_exit(&mut self, now: Instant, profile: crate::ui::utils::motion::MotionProfile) {
        let Some((started, from)) = self.closing else {
            self.sample(now, profile);
            return;
        };
        let elapsed = now.saturating_duration_since(started);
        if elapsed >= profile.exit {
            self.sample(now, profile);
            return;
        }
        // Motion 14.1.0 NativeAnimationExtended samples an interrupted opacity
        // animation twice, at most 10ms apart, independently of render cadence.
        let interval = std::time::Duration::from_millis(10);
        let sample_time = elapsed.max(interval);
        let delta = sample_time.saturating_sub(interval).min(interval);
        let current = Self::exit_value(from, sample_time, profile);
        let previous = Self::exit_value(from, sample_time - delta, profile);
        self.value = gpui::SpringState {
            position: current,
            velocity: if delta.is_zero() {
                0.
            } else {
                (current - previous) / delta.as_secs_f32()
            },
        };
        self.updated = now;
    }
    fn update(
        &mut self,
        open: bool,
        now: Instant,
        profile: crate::ui::utils::motion::MotionProfile,
    ) -> (f32, bool) {
        if self.open != open {
            if self.open {
                self.sample(now, profile);
            } else {
                self.stop_exit(now, profile);
            }
            self.open = open;
            self.closing = if open {
                None
            } else {
                Some((now, self.value.position))
            };
            self.updated = now;
        } else {
            self.sample(now, profile);
        }
        let moving = if open {
            self.value.position != 1. || self.value.velocity != 0.
        } else {
            self.closing.is_some()
        };
        (self.value.position.clamp(0., 1.), moving)
    }
}
fn sidebar_group_height_transition(
    open: bool,
    reduced: bool,
    profile: crate::ui::utils::motion::MotionProfile,
) -> kinu_ui::PanelTransition {
    if reduced {
        return kinu_ui::PanelTransition::Instant;
    }
    if open {
        kinu_ui::PanelTransition::Spring(profile.spring)
    } else {
        let curve = profile.exit_easing;
        kinu_ui::PanelTransition::CubicBezier(
            profile.exit,
            [curve.x1, curve.y1, curve.x2, curve.y2],
        )
    }
}

/// Styles disclosure motion while native ownership controls mounting, focus and input.
pub fn sidebar_group_panel_with_tokens(
    id: impl Into<ElementId>,
    open: bool,
    tokens: &ComponentTheme,
    window: &mut Window,
    cx: &mut App,
) -> kinu_ui::CollapsiblePanel {
    let now = cx.background_executor().now();
    let reduced = cx.reduce_motion();
    let opacity = window.use_keyed_state((id.into(), "sidebar-group-opacity"), cx, |_, _| {
        SidebarGroupOpacity::new(open, now)
    });
    let (alpha, moving) = opacity.update(cx, |state, _| {
        state.update(open, now, tokens.motion.moderate)
    });
    if moving && (open || !reduced) {
        window.request_animation_frame();
        cx.notify(window.current_view());
    }
    kinu_ui::CollapsiblePanel::new(kinu_ui::PanelRetention::Unmount)
        .transition(sidebar_group_height_transition(
            open,
            reduced,
            tokens.motion.moderate,
        ))
        .overflow_visible_when_open(true)
        .opacity(alpha)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn sidebar_input(
    id: impl Into<gpui::ElementId>,
    value: impl Into<gpui::SharedString>,
    window: &Window,
    cx: &App,
) -> kinu_ui::Input {
    sidebar_input_with_tokens(id, value, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn sidebar_workspace_tile(
    letter: impl Into<gpui::SharedString>,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    sidebar_workspace_tile_with_tokens(letter, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn sidebar_workspace_menu_tile(
    letter: impl Into<gpui::SharedString>,
    background: gpui::Hsla,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    sidebar_workspace_menu_tile_with_tokens(
        letter,
        background,
        ComponentTheme::current(cx),
        window,
        cx,
    )
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn sidebar_action_button(
    id: impl Into<ElementId>,
    label: impl Into<gpui::SharedString>,
    icon: IconName,
    cx: &App,
) -> crate::ui::button::Button {
    sidebar_action_button_with_tokens(id, label, icon, ComponentTheme::current(cx))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn sidebar_menu_badge<V: Into<gpui::SharedString>>(
    value: V,
    cx: &App,
) -> impl FnOnce(&kinu_ui::SidebarMenuRowFacts, &mut Window, &mut App) -> AnyElement + use<V> {
    sidebar_menu_badge_with_tokens(value, ComponentTheme::current(cx))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn sidebar_group_chevron(
    id: impl Into<ElementId>,
    open: bool,
    hover_group: gpui::SharedString,
    window: &mut Window,
    cx: &mut App,
) -> SidebarGroupChevron {
    sidebar_group_chevron_with_tokens(
        id,
        open,
        hover_group,
        ComponentTheme::current(cx),
        window,
        cx,
    )
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn sidebar_group_panel(
    id: impl Into<ElementId>,
    open: bool,
    window: &mut Window,
    cx: &mut App,
) -> kinu_ui::CollapsiblePanel {
    sidebar_group_panel_with_tokens(id, open, &ComponentTheme::current(cx), window, cx)
}
