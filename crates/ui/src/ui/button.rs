//! G2V's canonical native button primitive.

use std::rc::Rc;

use gpui::{
    AnyElement, App, BoxShadow, ClickEvent, ElementId, Entity, FocusHandle, Hsla,
    InteractiveElement as _, IntoElement, ParentElement, Rems, RenderOnce, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, WindowAppearance, div,
    prelude::FluentBuilder as _, px,
};
use gpui_base::{
    Button as ButtonBehavior, TextStyleToken, Transition as ValueTransition, TypographyTokens,
    transition,
};

use crate::theme::border::BORDER_DEFAULT;
use crate::theme::colors::G2VTheme;
use crate::theme::motion::MotionTokens;
use crate::theme::radius::{RADIUS_FULL, RADIUS_LG, RADIUS_MD, RADIUS_XL};
use crate::theme::shadows::{RING_3, focus_ring};
use crate::theme::sizing::{
    SIZE_3, SIZE_4, SIZE_5, SIZE_6, SIZE_7, SIZE_8, SIZE_9, SIZE_12, SIZE_16, SIZE_26,
};
use crate::theme::spacing::{SPACE_1, SPACE_1_5, SPACE_2, SPACE_2_5, SPACE_3, SPACE_4};
use crate::theme::typography;
use crate::theme::{rems_from_design_pixels, rems_from_pixels};
use crate::ui::{Icon, IconName, TextStyleExt as _, TooltipHoverHandler, TooltipTrigger as _};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Secondary,
    Ghost,
    GhostSecondary,
    SurfaceOption,
    GhostSubtle,
    GhostSecondarySubtle,
    GhostText,
    PopoverItem,
    Destructive,
    Link,
    Card,
    PopoverCard,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonSize {
    #[default]
    Default,
    ExtraSmall,
    Small,
    Large,
    Icon,
    IconTiny,
    IconExtraSmall,
    IconSmall,
    IconLarge,
    PopoverItem,
    Card,
    CardSmall,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonContentAlignment {
    Start,
    #[default]
    Center,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonShape {
    #[default]
    Default,
    RoundedFull,
    JoinedStart,
    JoinedEnd,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ButtonMetrics {
    height: Rems,
    width: Option<Rems>,
    horizontal_padding: Rems,
    gap: Rems,
    radius: Rems,
}

const EXTRA_SMALL_RADIUS: Rems = rems_from_design_pixels(10.0);

impl ButtonSize {
    fn metrics(self) -> ButtonMetrics {
        match self {
            Self::ExtraSmall => metrics(
                SIZE_6,
                None,
                rems_from_pixels(SPACE_2),
                rems_from_pixels(SPACE_1),
                EXTRA_SMALL_RADIUS,
            ),
            Self::Small => metrics(
                SIZE_7,
                None,
                rems_from_pixels(SPACE_2_5),
                rems_from_pixels(SPACE_1),
                rems_from_pixels(RADIUS_MD),
            ),
            Self::Default => metrics(
                SIZE_8,
                None,
                rems_from_pixels(SPACE_2_5),
                rems_from_pixels(SPACE_1_5),
                rems_from_pixels(RADIUS_LG),
            ),
            Self::Large => metrics(
                SIZE_9,
                None,
                rems_from_pixels(SPACE_2_5),
                rems_from_pixels(SPACE_1_5),
                rems_from_pixels(RADIUS_LG),
            ),
            Self::IconTiny => metrics(
                SIZE_4,
                Some(SIZE_4),
                Rems::ZERO,
                Rems::ZERO,
                EXTRA_SMALL_RADIUS,
            ),
            Self::IconExtraSmall => metrics(
                SIZE_6,
                Some(SIZE_6),
                Rems::ZERO,
                Rems::ZERO,
                EXTRA_SMALL_RADIUS,
            ),
            Self::IconSmall => metrics(
                SIZE_7,
                Some(SIZE_7),
                Rems::ZERO,
                Rems::ZERO,
                rems_from_pixels(RADIUS_MD),
            ),
            Self::Icon => metrics(
                SIZE_8,
                Some(SIZE_8),
                Rems::ZERO,
                Rems::ZERO,
                rems_from_pixels(RADIUS_LG),
            ),
            Self::IconLarge => metrics(
                SIZE_9,
                Some(SIZE_9),
                Rems::ZERO,
                Rems::ZERO,
                rems_from_pixels(RADIUS_LG),
            ),
            Self::PopoverItem => metrics(
                SIZE_12,
                None,
                rems_from_pixels(SPACE_2),
                rems_from_pixels(SPACE_2),
                rems_from_pixels(RADIUS_LG),
            ),
            Self::Card => metrics(
                SIZE_26,
                None,
                rems_from_pixels(SPACE_4),
                rems_from_pixels(SPACE_1_5),
                rems_from_pixels(RADIUS_LG),
            ),
            Self::CardSmall => metrics(
                SIZE_16,
                None,
                rems_from_pixels(SPACE_2),
                rems_from_pixels(SPACE_3),
                rems_from_pixels(RADIUS_XL),
            ),
        }
    }

    fn text_style(self, typography: &TypographyTokens) -> TextStyleToken {
        match self {
            Self::ExtraSmall | Self::IconTiny | Self::IconExtraSmall => typography.xs,
            Self::Small => TextStyleToken {
                size: px(12.8),
                ..typography.sm
            },
            Self::IconSmall
            | Self::Default
            | Self::Large
            | Self::Icon
            | Self::IconLarge
            | Self::PopoverItem
            | Self::Card
            | Self::CardSmall => typography.sm,
        }
    }

    const fn icon_size(self) -> Rems {
        match self {
            Self::ExtraSmall | Self::IconTiny | Self::IconExtraSmall => SIZE_3,
            Self::Small
            | Self::Default
            | Self::Large
            | Self::Icon
            | Self::IconSmall
            | Self::IconLarge
            | Self::PopoverItem
            | Self::Card
            | Self::CardSmall => SIZE_4,
        }
    }

    fn horizontal_padding(self, has_icon: bool) -> (Rems, Rems) {
        let padding = self.metrics().horizontal_padding;
        if !has_icon {
            return (padding, padding);
        }
        match self {
            Self::ExtraSmall => (rems_from_pixels(SPACE_1_5), rems_from_pixels(SPACE_2)),
            Self::Small => (rems_from_pixels(SPACE_1_5), rems_from_pixels(SPACE_2_5)),
            Self::Default | Self::Large | Self::PopoverItem => {
                (rems_from_pixels(SPACE_2), rems_from_pixels(SPACE_2_5))
            }
            Self::Icon
            | Self::IconTiny
            | Self::IconExtraSmall
            | Self::IconSmall
            | Self::IconLarge => (Rems::ZERO, Rems::ZERO),
            Self::Card => (rems_from_pixels(SPACE_4), rems_from_pixels(SPACE_4)),
            Self::CardSmall => (rems_from_pixels(SPACE_2), rems_from_pixels(SPACE_2)),
        }
    }
}

const fn metrics(
    height: Rems,
    width: Option<Rems>,
    horizontal_padding: Rems,
    gap: Rems,
    radius: Rems,
) -> ButtonMetrics {
    ButtonMetrics {
        height,
        width,
        horizontal_padding,
        gap,
        radius,
    }
}

#[derive(Clone, Copy)]
struct VariantPalette {
    background: Hsla,
    foreground: Hsla,
    border: Hsla,
    hover_background: Hsla,
    hover_foreground: Hsla,
    expanded_background: Option<Hsla>,
}

impl VariantPalette {
    const fn new(
        background: Hsla,
        foreground: Hsla,
        border: Hsla,
        hover_background: Hsla,
        hover_foreground: Hsla,
        expanded_background: Option<Hsla>,
    ) -> Self {
        Self {
            background,
            foreground,
            border,
            hover_background,
            hover_foreground,
            expanded_background,
        }
    }
}

impl ButtonVariant {
    #[expect(
        clippy::too_many_lines,
        reason = "the exhaustive semantic palette keeps every button variant auditable in one table"
    )]
    fn palette(self, theme: &G2VTheme, dark: bool) -> VariantPalette {
        let transparent = theme.background.opacity(0.0);
        match self {
            Self::Default => VariantPalette::new(
                theme.primary,
                theme.primary_foreground,
                transparent,
                theme.primary.opacity(0.8),
                theme.primary_foreground,
                None,
            ),
            Self::Outline => VariantPalette {
                background: if dark {
                    theme.input.opacity(0.3)
                } else {
                    theme.background
                },
                foreground: theme.foreground,
                border: if dark { theme.input } else { theme.border },
                hover_background: if dark {
                    theme.input.opacity(0.5)
                } else {
                    theme.muted
                },
                hover_foreground: theme.foreground,
                expanded_background: Some(theme.muted),
            },
            Self::Secondary => VariantPalette::new(
                theme.secondary,
                theme.secondary_foreground,
                transparent,
                theme.secondary_hover,
                theme.secondary_foreground,
                Some(theme.secondary),
            ),
            Self::Ghost => VariantPalette {
                background: transparent,
                foreground: theme.foreground,
                border: transparent,
                hover_background: theme.foreground.opacity(0.05),
                hover_foreground: theme.foreground,
                expanded_background: Some(theme.foreground.opacity(0.1)),
            },
            Self::GhostSecondary => VariantPalette {
                background: transparent,
                foreground: theme.muted_foreground,
                border: transparent,
                hover_background: theme.foreground.opacity(0.05),
                hover_foreground: theme.foreground,
                expanded_background: Some(theme.foreground.opacity(0.1)),
            },
            Self::SurfaceOption => VariantPalette {
                background: transparent,
                foreground: theme.foreground,
                border: transparent,
                hover_background: theme.control_hover_fill(),
                hover_foreground: theme.foreground,
                expanded_background: Some(theme.control_selected_fill()),
            },
            Self::GhostSubtle => VariantPalette {
                background: transparent,
                foreground: theme.foreground,
                border: transparent,
                hover_background: theme.foreground.opacity(0.03),
                hover_foreground: theme.foreground,
                expanded_background: Some(theme.foreground.opacity(0.03)),
            },
            Self::GhostSecondarySubtle => VariantPalette {
                background: transparent,
                foreground: theme.muted_foreground,
                border: transparent,
                hover_background: theme.foreground.opacity(0.02),
                hover_foreground: theme.foreground,
                expanded_background: Some(theme.foreground.opacity(0.02)),
            },
            Self::GhostText => VariantPalette {
                background: transparent,
                foreground: theme.muted_foreground,
                border: transparent,
                hover_background: transparent,
                hover_foreground: theme.foreground,
                expanded_background: Some(transparent),
            },
            Self::PopoverItem => VariantPalette {
                background: transparent,
                foreground: theme.foreground,
                border: transparent,
                hover_background: theme.popover_hover,
                hover_foreground: theme.foreground,
                expanded_background: Some(theme.popover_selected),
            },
            Self::Destructive => VariantPalette {
                background: theme.destructive.opacity(if dark { 0.2 } else { 0.1 }),
                foreground: theme.destructive,
                border: transparent,
                hover_background: theme.destructive.opacity(if dark { 0.3 } else { 0.2 }),
                hover_foreground: theme.destructive,
                expanded_background: None,
            },
            Self::Link => VariantPalette::new(
                transparent,
                theme.primary,
                transparent,
                transparent,
                theme.primary,
                None,
            ),
            Self::Card => VariantPalette {
                background: theme.card,
                foreground: theme.card_foreground,
                border: theme.surface_border,
                hover_background: theme.card_hover,
                hover_foreground: theme.accent_foreground,
                expanded_background: Some(theme.card_hover),
            },
            Self::PopoverCard => VariantPalette {
                background: theme.popover,
                foreground: theme.popover_foreground,
                border: theme.popover_border,
                hover_background: theme.popover_hover,
                hover_foreground: theme.popover_foreground,
                expanded_background: Some(theme.popover_hover),
            },
        }
    }
}

pub type ButtonClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

struct ButtonRuntime {
    focus: gpui::FocusHandle,
    hovered: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct ButtonFlags(u8);

impl ButtonFlags {
    const FULL_WIDTH: u8 = 1 << 0;
    const DISABLED: u8 = 1 << 1;
    const SELECTED: u8 = 1 << 2;
    const INVALID: u8 = 1 << 3;
    const HAS_POPUP: u8 = 1 << 4;
    const LARGE_ICON: u8 = 1 << 5;

    fn set(&mut self, flag: u8, enabled: bool) {
        if enabled {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }

    const fn contains(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    accessibility_label: Option<SharedString>,
    icon: Option<IconName>,
    hover_leading_action: Option<(ElementId, IconName, ButtonClickHandler)>,
    icon_size_override: Option<Rems>,
    children: Vec<AnyElement>,
    trailing_children: Vec<AnyElement>,
    variant: ButtonVariant,
    size: ButtonSize,
    content_alignment: ButtonContentAlignment,
    shape: ButtonShape,
    group_edges: Option<super::button_group::GroupEdges>,
    flags: ButtonFlags,
    tooltip: Option<SharedString>,
    debug_selector: Option<SharedString>,
    on_click: Option<ButtonClickHandler>,
    focus_handle: Option<FocusHandle>,
}

impl Button {
    pub(crate) fn grouped(mut self, edges: super::button_group::GroupEdges) -> Self {
        self.group_edges = Some(edges);
        self
    }

    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            accessibility_label: None,
            icon: None,
            hover_leading_action: None,
            icon_size_override: None,
            children: Vec::new(),
            trailing_children: Vec::new(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            content_alignment: ButtonContentAlignment::Center,
            shape: ButtonShape::Default,
            group_edges: None,
            flags: ButtonFlags::default(),
            tooltip: None,
            debug_selector: None,
            on_click: None,
            focus_handle: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        self.accessibility_label = Some(label.clone());
        self.label = Some(label);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn trailing_child(mut self, child: impl IntoElement) -> Self {
        self.trailing_children.push(child.into_any_element());
        self
    }

    pub fn icon(mut self, name: IconName) -> Self {
        self.icon = Some(name);
        self
    }

    pub fn hover_leading_action(
        mut self,
        id: impl Into<ElementId>,
        icon: IconName,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.hover_leading_action = Some((id.into(), icon, Rc::new(handler)));
        self
    }

    pub fn icon_size(mut self, size: Rems) -> Self {
        self.icon_size_override = Some(size);
        self
    }

    pub fn large_icon(mut self) -> Self {
        self.flags.set(ButtonFlags::LARGE_ICON, true);
        self
    }

    pub fn rounded_full(mut self) -> Self {
        self.shape = ButtonShape::RoundedFull;
        self
    }

    pub fn joined_start(mut self) -> Self {
        self.shape = ButtonShape::JoinedStart;
        self
    }

    pub fn joined_end(mut self) -> Self {
        self.shape = ButtonShape::JoinedEnd;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn content_alignment(mut self, alignment: ButtonContentAlignment) -> Self {
        self.content_alignment = alignment;
        self
    }

    pub fn full_width(mut self) -> Self {
        self.flags.set(ButtonFlags::FULL_WIDTH, true);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.flags.set(ButtonFlags::DISABLED, disabled);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.flags.set(ButtonFlags::SELECTED, selected);
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.flags.set(ButtonFlags::INVALID, invalid);
        self
    }

    pub fn has_popup(mut self, has_popup: bool) -> Self {
        self.flags.set(ButtonFlags::HAS_POPUP, has_popup);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    #[cfg(test)]
    pub(crate) fn debug_selector(mut self, selector: impl Into<SharedString>) -> Self {
        self.debug_selector = Some(selector.into());
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn track_focus(mut self, focus_handle: &FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle.clone());
        self
    }

    pub fn on_click_shared(mut self, handler: ButtonClickHandler) -> Self {
        self.on_click = Some(handler);
        self
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

struct ButtonPresentation {
    theme: G2VTheme,
    palette: VariantPalette,
    metrics: ButtonMetrics,
    padding: (Rems, Rems),
    text_style: TextStyleToken,
    flags: ButtonFlags,
    border_color: Hsla,
    focus_ring: Vec<BoxShadow>,
}

impl ButtonPresentation {
    fn resolve(button: &Button, window: &Window, cx: &App) -> Self {
        let theme = G2VTheme::global(cx).clone();
        let dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let palette = button.variant.palette(&theme, dark);
        let invalid = button.flags.contains(ButtonFlags::INVALID);
        let (border_color, ring_color) = state_colors(&theme, palette, dark, invalid);

        Self {
            theme,
            palette,
            metrics: button.size.metrics(),
            padding: button.size.horizontal_padding(button.icon.is_some()),
            text_style: button.size.text_style(&typography::tokens()),
            flags: button.flags,
            border_color,
            focus_ring: focus_ring(ring_color, RING_3),
        }
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let runtime = self.runtime(window, cx);
        let focus = runtime.read(cx).focus.clone();
        let grouped = self.group_edges.is_some();
        let button = self.render_behavior_with_runtime(runtime, window, cx);
        if grouped {
            super::button_group::paint_group_control(button, &focus, window)
        } else {
            button.into_any_element()
        }
    }
}

impl Button {
    fn runtime(&self, window: &mut Window, cx: &mut App) -> Entity<ButtonRuntime> {
        let provided_focus_handle = self.focus_handle.clone();
        window.use_keyed_state(self.id.clone(), cx, |_, cx| ButtonRuntime {
            focus: provided_focus_handle.unwrap_or_else(|| cx.focus_handle()),
            hovered: false,
        })
    }

    #[cfg(test)]
    fn render_behavior(self, window: &mut Window, cx: &mut App) -> ButtonBehavior {
        let runtime = self.runtime(window, cx);
        self.render_behavior_with_runtime(runtime, window, cx)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the declarative GPUI button composition stays in visual and interaction order"
    )]
    fn render_behavior_with_runtime(
        self,
        runtime: Entity<ButtonRuntime>,
        window: &mut Window,
        cx: &mut App,
    ) -> ButtonBehavior {
        let presentation = ButtonPresentation::resolve(&self, window, cx);
        let palette = presentation.palette;
        let metrics = presentation.metrics;
        let (left_padding, right_padding) = presentation.padding;
        let disabled = presentation.flags.contains(ButtonFlags::DISABLED);
        let selected = presentation.flags.contains(ButtonFlags::SELECTED);
        let invalid = presentation.flags.contains(ButtonFlags::INVALID);
        let has_popup = presentation.flags.contains(ButtonFlags::HAS_POPUP);
        let focus_handle = runtime.read(cx).focus.clone();
        let hovered = runtime.read(cx).hovered && !disabled;
        let focus_ring = presentation.focus_ring;
        let (background, foreground) =
            animated_colors(&self.id, palette, selected, hovered, window, cx);
        let border_color = if self.variant == ButtonVariant::Card && hovered {
            presentation.theme.foreground.opacity(0.2)
        } else {
            presentation.border_color
        };
        let is_link = self.variant == ButtonVariant::Link;
        let button = ButtonBehavior::new(self.id.clone())
            .track_focus(&focus_handle)
            .when(
                matches!(
                    self.size,
                    ButtonSize::PopoverItem | ButtonSize::Card | ButtonSize::CardSmall
                ),
                |button| {
                    let vertical_padding = match self.size {
                        ButtonSize::PopoverItem => SPACE_1,
                        ButtonSize::Card => SPACE_4,
                        ButtonSize::CardSmall => SPACE_2,
                        _ => unreachable!("only minimum-height button sizes reach this branch"),
                    };
                    button.min_w_0().py(rems_from_pixels(vertical_padding))
                },
            )
            .when(
                matches!(
                    self.size,
                    ButtonSize::PopoverItem | ButtonSize::Card | ButtonSize::CardSmall
                ),
                |button| button.min_h(metrics.height),
            )
            .when(
                !matches!(
                    self.size,
                    ButtonSize::PopoverItem | ButtonSize::Card | ButtonSize::CardSmall
                ),
                |button| button.h(metrics.height),
            )
            .pl(left_padding)
            .pr(right_padding)
            .flex()
            .flex_shrink_0()
            .items_center()
            .map(|button| match self.content_alignment {
                ButtonContentAlignment::Start => button.justify_start(),
                ButtonContentAlignment::Center => button.justify_center(),
            })
            .gap(metrics.gap)
            .text_style(presentation.text_style)
            .map(|button| match self.shape {
                ButtonShape::Default => button.rounded(metrics.radius),
                ButtonShape::RoundedFull => button.rounded(rems_from_pixels(RADIUS_FULL)),
                ButtonShape::JoinedStart => button
                    .rounded_l(rems_from_pixels(RADIUS_FULL))
                    .rounded_r(gpui::rems(0.0)),
                ButtonShape::JoinedEnd => button
                    .rounded_l(gpui::rems(0.0))
                    .rounded_r(rems_from_pixels(RADIUS_FULL)),
            })
            .border(BORDER_DEFAULT)
            .when_some(self.group_edges, |button, edges| edges.apply(button))
            .border_color(border_color)
            .bg(background)
            .when(self.variant == ButtonVariant::Card, |button| {
                button.shadow(crate::theme::shadows::tokens(window.appearance()).sm)
            })
            .text_color(foreground)
            .cursor_pointer()
            .disabled(disabled)
            .selected(selected)
            .styles(|styles| styles.disabled(|style| style.opacity(0.5).cursor_default()))
            .when(has_popup, |button| button.aria_expanded(selected))
            .when(invalid, |button| button.shadow(focus_ring.clone()))
            .when(!disabled, |button| {
                button
                    .hover(|style| {
                        if is_link {
                            style.text_decoration_1()
                        } else {
                            style
                        }
                    })
                    .active(|style| {
                        if has_popup {
                            style
                        } else {
                            style.relative().top(px(1.0))
                        }
                    })
                    .focus_visible(move |style| {
                        style
                            .border_color(if invalid {
                                border_color
                            } else {
                                presentation.theme.ring
                            })
                            .shadow(focus_ring)
                    })
            })
            .when_some(metrics.width, gpui::Styled::w)
            .when(
                presentation.flags.contains(ButtonFlags::FULL_WIDTH),
                gpui::Styled::w_full,
            )
            .when_some(self.debug_selector.clone(), |button, selector| {
                button.debug_selector(move || selector.to_string())
            });

        self.render_content(button, foreground, focus_handle, runtime, hovered)
    }
}

impl Button {
    fn render_content(
        self,
        button: ButtonBehavior,
        icon_color: Hsla,
        focus_handle: gpui::FocusHandle,
        runtime: Entity<ButtonRuntime>,
        hovered: bool,
    ) -> ButtonBehavior {
        let icon_size = self.icon_size_override.unwrap_or_else(|| {
            if self.flags.contains(ButtonFlags::LARGE_ICON) {
                SIZE_5
            } else {
                self.size.icon_size()
            }
        });
        let hover_leading_action = self.hover_leading_action;
        let button = button
            .when_some(self.accessibility_label, |button, label| {
                button.accessibility_label(label)
            })
            .when_some(self.on_click, |button, handler| {
                button.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when_some(self.icon, |button, name| {
                button.child(match hover_leading_action {
                    Some((id, hover_icon, handler)) if hovered => div()
                        .id(id)
                        .flex()
                        .size(icon_size)
                        .items_center()
                        .justify_center()
                        .rounded(RADIUS_FULL)
                        .child(Icon::new(hover_icon, icon_size).color(icon_color))
                        .on_click(move |event, window, cx| {
                            cx.stop_propagation();
                            handler(event, window, cx);
                        })
                        .into_any_element(),
                    _ => Icon::new(name, icon_size)
                        .color(icon_color)
                        .into_any_element(),
                })
            })
            .children(self.children)
            .when_some(self.label, |button, label| {
                button.child(div().whitespace_nowrap().child(label))
            })
            .children(self.trailing_children);

        let on_hover = button_hover_handler(runtime);
        match (self.tooltip, self.flags.contains(ButtonFlags::DISABLED)) {
            (Some(tooltip), false) => {
                button.with_tooltip_focus_and_hover(tooltip, focus_handle, on_hover)
            }
            (Some(tooltip), true) => button.with_tooltip_focus(tooltip, focus_handle),
            (None, false) => button.on_hover(move |hovered, window, cx| {
                on_hover(*hovered, window, cx);
            }),
            (None, true) => button,
        }
    }
}

fn button_hover_handler(runtime: Entity<ButtonRuntime>) -> TooltipHoverHandler {
    Rc::new(move |hovered, _, cx| {
        runtime.update(cx, |runtime, cx| {
            if runtime.hovered != hovered {
                runtime.hovered = hovered;
                cx.notify();
            }
        });
    })
}

fn animated_colors(
    id: &ElementId,
    palette: VariantPalette,
    selected: bool,
    hovered: bool,
    window: &mut Window,
    cx: &mut App,
) -> (Hsla, Hsla) {
    let (target_background, target_foreground) = target_colors(palette, selected, hovered);
    let policy = ValueTransition::new(MotionTokens::global(cx).fast);
    let background = transition(
        (id.clone(), "background"),
        target_background,
        policy.clone(),
        window,
        cx,
    );
    let foreground = transition(
        (id.clone(), "foreground"),
        target_foreground,
        policy,
        window,
        cx,
    );
    (background, foreground)
}

fn target_colors(palette: VariantPalette, selected: bool, hovered: bool) -> (Hsla, Hsla) {
    let expanded_background = palette.expanded_background.unwrap_or(palette.background);
    let target_background = if selected {
        expanded_background
    } else if hovered {
        palette.hover_background
    } else {
        palette.background
    };
    let target_foreground = if selected || hovered {
        palette.hover_foreground
    } else {
        palette.foreground
    };
    (target_background, target_foreground)
}

fn state_colors(
    theme: &G2VTheme,
    palette: VariantPalette,
    dark: bool,
    invalid: bool,
) -> (Hsla, Hsla) {
    if invalid {
        (
            theme.destructive.opacity(if dark { 0.5 } else { 1.0 }),
            theme.destructive.opacity(if dark { 0.4 } else { 0.2 }),
        )
    } else {
        (palette.border, theme.ring.opacity(0.5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Context, Element as _, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, Role,
        TestAppContext, VisualTestContext, WindowAppearance, accesskit, canvas,
    };
    use std::{
        cell::Cell,
        rc::Rc,
        sync::{Arc, Mutex},
    };

    fn install_test_globals(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            typography::install(cx);
            crate::theme::install(cx.window_appearance(), cx);
        });
    }

    struct TooltipButtonHarness;

    impl Render for TooltipButtonHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Button::new("tooltip-button-regression")
                .label("Tooltip")
                .tooltip("Tooltip content")
        }
    }

    #[gpui::test]
    fn tooltip_button_registers_one_hover_handler(cx: &mut TestAppContext) {
        install_test_globals(cx);
        let (_, cx) = cx.add_window_view(|_, _| TooltipButtonHarness);
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }

    struct InteractionHarness {
        disabled: bool,
        button_clicks: Rc<Cell<usize>>,
        parent_clicks: Rc<Cell<usize>>,
        keyboard_clicks: Rc<Cell<usize>>,
    }

    type InteractionHarnessResult<'a> = (
        &'a mut VisualTestContext,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
        Rc<Cell<usize>>,
    );

    impl Render for InteractionHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let button_clicks = self.button_clicks.clone();
            let parent_clicks = self.parent_clicks.clone();
            let keyboard_clicks = self.keyboard_clicks.clone();

            div()
                .id("button-parent")
                .tab_group()
                .size(px(100.0))
                .on_click(move |_, _, _| parent_clicks.set(parent_clicks.get() + 1))
                .child(
                    Button::new("interaction-button")
                        .label("Save")
                        .debug_selector("interaction-button")
                        .disabled(self.disabled)
                        .on_click(move |event, _, _| {
                            button_clicks.set(button_clicks.get() + 1);
                            if matches!(event, ClickEvent::Keyboard(_)) {
                                keyboard_clicks.set(keyboard_clicks.get() + 1);
                            }
                        }),
                )
        }
    }

    fn interaction_harness(
        cx: &mut TestAppContext,
        disabled: bool,
    ) -> InteractionHarnessResult<'_> {
        install_test_globals(cx);
        let button_clicks = Rc::new(Cell::new(0));
        let parent_clicks = Rc::new(Cell::new(0));
        let keyboard_clicks = Rc::new(Cell::new(0));
        let (_, window) = cx.add_window_view({
            let button_clicks = button_clicks.clone();
            let parent_clicks = parent_clicks.clone();
            let keyboard_clicks = keyboard_clicks.clone();
            move |_, _| InteractionHarness {
                disabled,
                button_clicks,
                parent_clicks,
                keyboard_clicks,
            }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        (window, button_clicks, parent_clicks, keyboard_clicks)
    }

    fn simulate_keyboard_click(window: &mut VisualTestContext, key: &str) {
        let keystroke = Keystroke::parse(key).unwrap();
        window.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        window.simulate_event(KeyUpEvent { keystroke });
    }

    #[gpui::test]
    fn pointer_enter_and_space_activate_the_facade_once(cx: &mut TestAppContext) {
        let (window, button_clicks, _, keyboard_clicks) = interaction_harness(cx, false);
        let button = window.debug_bounds("interaction-button").unwrap();

        window.simulate_click(button.center(), Modifiers::default());
        assert_eq!(button_clicks.get(), 1);
        assert_eq!(keyboard_clicks.get(), 0);

        simulate_keyboard_click(window, "enter");
        simulate_keyboard_click(window, "space");
        assert_eq!(button_clicks.get(), 3);
        assert_eq!(keyboard_clicks.get(), 2);
    }

    #[gpui::test]
    fn disabled_facade_blocks_pointer_keyboard_and_parent_activation(cx: &mut TestAppContext) {
        let (window, button_clicks, parent_clicks, keyboard_clicks) = interaction_harness(cx, true);
        let button = window.debug_bounds("interaction-button").unwrap();

        window.simulate_click(button.center(), Modifiers::default());
        window.update(gpui::Window::focus_next);
        simulate_keyboard_click(window, "enter");
        simulate_keyboard_click(window, "space");

        assert_eq!(button_clicks.get(), 0);
        assert_eq!(parent_clicks.get(), 0);
        assert_eq!(keyboard_clicks.get(), 0);
    }

    #[gpui::test]
    fn facade_exposes_button_role_label_and_disabled_action_surface(cx: &mut TestAppContext) {
        type Captured = Arc<Mutex<Option<(accesskit::Node, accesskit::Node)>>>;

        struct AccessibilityHarness {
            captured: Captured,
        }

        impl Render for AccessibilityHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let captured = self.captured.clone();
                canvas(
                    move |_, window, cx| {
                        let mut accessibility_node = |button: Button| {
                            let mut node = accesskit::Node::new(Role::Button);
                            button
                                .render_behavior(window, cx)
                                .render(window, cx)
                                .into_element()
                                .write_a11y_info(&mut node);
                            node
                        };
                        let enabled = accessibility_node(
                            Button::new("enabled-accessibility-button")
                                .label("Save")
                                .on_click(|_, _, _| {}),
                        );
                        let disabled = accessibility_node(
                            Button::new("disabled-accessibility-button")
                                .label("Save")
                                .disabled(true)
                                .on_click(|_, _, _| {}),
                        );
                        *captured.lock().unwrap() = Some((enabled, disabled));
                    },
                    |_, (), _, _| {},
                )
            }
        }

        install_test_globals(cx);
        let captured: Captured = Arc::new(Mutex::new(None));
        let result = captured.clone();
        let (_, window) = cx.add_window_view(move |_, _| AccessibilityHarness { captured });
        window.update(|window, cx| window.draw(cx).clear(cx));

        let (enabled, disabled) = result.lock().unwrap().take().unwrap();
        assert_eq!(enabled.role(), Role::Button);
        assert_eq!(enabled.label(), Some("Save"));
        assert!(enabled.supports_action(accesskit::Action::Click));
        assert_eq!(disabled.role(), Role::Button);
        assert_eq!(disabled.label(), Some("Save"));
        assert!(!disabled.supports_action(accesskit::Action::Click));
    }

    #[test]
    fn sizes_match_the_react_button_contract() {
        let rem_size = px(16.0);
        assert_eq!(
            ButtonSize::ExtraSmall.metrics().height.to_pixels(rem_size),
            px(24.0)
        );
        assert_eq!(
            ButtonSize::Small.metrics().height.to_pixels(rem_size),
            px(28.0)
        );
        assert_eq!(
            ButtonSize::Default.metrics().height.to_pixels(rem_size),
            px(32.0)
        );
        assert_eq!(
            ButtonSize::Large.metrics().height.to_pixels(rem_size),
            px(36.0)
        );
        assert_eq!(
            ButtonSize::PopoverItem.metrics().height.to_pixels(rem_size),
            px(48.0)
        );
        assert_eq!(
            ButtonSize::Icon
                .metrics()
                .width
                .map(|width| width.to_pixels(rem_size)),
            Some(px(32.0))
        );
        assert_eq!(
            ButtonSize::IconTiny
                .metrics()
                .width
                .map(|width| width.to_pixels(rem_size)),
            Some(px(16.0))
        );
        assert_eq!(
            ButtonSize::IconExtraSmall
                .metrics()
                .width
                .map(|width| width.to_pixels(rem_size)),
            Some(px(24.0))
        );
        assert_eq!(
            ButtonSize::IconSmall
                .metrics()
                .width
                .map(|width| width.to_pixels(rem_size)),
            Some(px(28.0))
        );
        assert_eq!(
            ButtonSize::IconLarge
                .metrics()
                .width
                .map(|width| width.to_pixels(rem_size)),
            Some(px(36.0))
        );
        assert_eq!(
            ButtonSize::Default.metrics().height.to_pixels(px(18.0)),
            px(36.0)
        );
        assert_eq!(
            ButtonSize::ExtraSmall.metrics().radius.to_pixels(px(16.0)),
            px(10.0)
        );
        assert_eq!(
            ButtonSize::ExtraSmall.metrics().radius.to_pixels(px(18.0)),
            px(11.25)
        );
        assert_eq!(
            ButtonSize::Small.metrics().radius.to_pixels(px(16.0)),
            RADIUS_MD
        );
        assert_eq!(
            ButtonSize::Default.metrics().radius.to_pixels(px(16.0)),
            RADIUS_LG
        );
    }

    #[test]
    fn typography_matches_the_react_button_contract() {
        let typography = typography::tokens();

        assert_eq!(ButtonSize::Default.text_style(&typography).size, px(14.0));
        assert_eq!(
            ButtonSize::Default.text_style(&typography).line_height,
            px(20.0)
        );
        assert_eq!(
            ButtonSize::Default.text_style(&typography).weight,
            typography::FONT_WEIGHT.normal
        );
        assert_eq!(
            ButtonSize::ExtraSmall.text_style(&typography).size,
            px(12.0)
        );
        assert_eq!(
            ButtonSize::ExtraSmall.text_style(&typography).line_height,
            px(16.0)
        );
        assert_eq!(ButtonSize::Small.text_style(&typography).size, px(12.8));
        assert_eq!(
            ButtonSize::Small.text_style(&typography).line_height,
            px(20.0)
        );
        assert_eq!(
            ButtonSize::Small.text_style(&typography).weight,
            typography::FONT_WEIGHT.normal
        );
    }

    #[test]
    fn icon_sizes_and_leading_padding_match_the_react_contract() {
        let rem_size = px(16.0);
        let resolve_padding =
            |(left, right): (Rems, Rems)| (left.to_pixels(rem_size), right.to_pixels(rem_size));
        assert_eq!(
            ButtonSize::ExtraSmall.icon_size().to_pixels(rem_size),
            px(12.0)
        );
        assert_eq!(ButtonSize::Small.icon_size().to_pixels(rem_size), px(16.0));
        assert_eq!(
            ButtonSize::Default.icon_size().to_pixels(rem_size),
            px(16.0)
        );
        assert_eq!(
            ButtonSize::IconSmall.icon_size().to_pixels(rem_size),
            px(16.0)
        );
        assert_eq!(
            resolve_padding(ButtonSize::ExtraSmall.horizontal_padding(true)),
            (px(6.0), px(8.0))
        );
        assert_eq!(
            resolve_padding(ButtonSize::Small.horizontal_padding(true)),
            (px(6.0), px(10.0))
        );
        assert_eq!(
            resolve_padding(ButtonSize::Default.horizontal_padding(true)),
            (px(8.0), px(10.0))
        );
        assert_eq!(
            resolve_padding(ButtonSize::Default.horizontal_padding(false)),
            (px(10.0), px(10.0))
        );
        assert_eq!(
            ButtonSize::Default.icon_size().to_pixels(px(18.0)),
            px(18.0)
        );
    }

    #[test]
    fn composer_send_button_can_override_icon_size_and_radius() {
        let button = Button::new("composer-send")
            .size(ButtonSize::IconSmall)
            .icon(IconName::ArrowUp)
            .large_icon()
            .rounded_full();

        assert!(button.flags.contains(ButtonFlags::LARGE_ICON));
        assert_eq!(button.shape, ButtonShape::RoundedFull);
    }

    #[test]
    fn explicit_icon_size_overrides_the_size_variant() {
        let button = Button::new("browser-site-information")
            .size(ButtonSize::IconExtraSmall)
            .icon(IconName::Control)
            .icon_size(SIZE_4);

        assert_eq!(button.icon_size_override, Some(SIZE_4));
    }

    #[test]
    fn variants_resolve_to_their_semantic_color_roles() {
        let theme = G2VTheme::for_appearance(WindowAppearance::Light);

        assert_eq!(
            ButtonVariant::Default.palette(&theme, false).background,
            theme.primary
        );
        assert_eq!(
            ButtonVariant::Outline.palette(&theme, false).border,
            theme.border
        );
        assert_eq!(
            ButtonVariant::Secondary.palette(&theme, false).background,
            theme.secondary
        );
        assert_eq!(
            ButtonVariant::GhostSecondary
                .palette(&theme, false)
                .foreground,
            theme.muted_foreground
        );
        assert_eq!(
            ButtonVariant::Destructive.palette(&theme, false).foreground,
            theme.destructive
        );
        assert_eq!(
            ButtonVariant::Link.palette(&theme, false).foreground,
            theme.primary
        );
    }

    #[test]
    fn dark_variants_match_the_react_opacity_contract() {
        let theme = G2VTheme::for_appearance(WindowAppearance::Dark);
        let outline = ButtonVariant::Outline.palette(&theme, true);
        let ghost = ButtonVariant::Ghost.palette(&theme, true);
        let destructive = ButtonVariant::Destructive.palette(&theme, true);

        assert_eq!(outline.background, theme.input.opacity(0.3));
        assert_eq!(outline.hover_background, theme.input.opacity(0.5));
        assert_eq!(ghost.hover_background, theme.foreground.opacity(0.05));
        assert_eq!(
            ghost.expanded_background,
            Some(theme.foreground.opacity(0.1))
        );
        assert_eq!(destructive.background, theme.destructive.opacity(0.2));
        assert_eq!(destructive.hover_background, theme.destructive.opacity(0.3));
    }

    #[test]
    fn secondary_hover_keeps_ninety_five_percent_secondary_color() {
        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            let theme = G2VTheme::for_appearance(appearance);
            let palette =
                ButtonVariant::Secondary.palette(&theme, appearance == WindowAppearance::Dark);

            assert_eq!(palette.hover_background, theme.secondary_hover);
        }
    }

    #[test]
    fn subtle_ghost_targets_remain_translucent_in_both_appearances() {
        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            let theme = G2VTheme::for_appearance(appearance);
            for (variant, opacity) in [
                (ButtonVariant::GhostSubtle, 0.03),
                (ButtonVariant::GhostSecondarySubtle, 0.02),
            ] {
                let palette = variant.palette(&theme, appearance == WindowAppearance::Dark);
                let interaction_background = theme.foreground.opacity(opacity);

                assert_eq!(
                    target_colors(palette, false, false),
                    (palette.background, palette.foreground)
                );
                assert_eq!(
                    target_colors(palette, false, true),
                    (interaction_background, palette.hover_foreground)
                );
                assert_eq!(
                    target_colors(palette, true, false),
                    (interaction_background, palette.hover_foreground)
                );
            }
        }
    }

    #[test]
    fn dark_secondary_subtle_hover_is_visible_on_the_dark_surface() {
        let theme = G2VTheme::for_appearance(WindowAppearance::Dark);
        let palette = ButtonVariant::GhostSecondarySubtle.palette(&theme, true);

        assert_eq!(palette.hover_background, theme.foreground.opacity(0.02));
        assert_eq!(
            palette.expanded_background,
            Some(theme.foreground.opacity(0.02))
        );
    }

    #[test]
    fn ghost_text_only_strengthens_foreground_on_hover() {
        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            let theme = G2VTheme::for_appearance(appearance);
            let palette =
                ButtonVariant::GhostText.palette(&theme, appearance == WindowAppearance::Dark);
            let transparent = theme.background.opacity(0.0);

            assert_eq!(
                target_colors(palette, false, false),
                (transparent, theme.muted_foreground)
            );
            assert_eq!(
                target_colors(palette, false, true),
                (transparent, theme.foreground)
            );
            assert_eq!(
                target_colors(palette, true, false),
                (transparent, theme.foreground)
            );
        }
    }

    #[test]
    fn popover_item_uses_the_searchable_popover_interaction_colors() {
        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            let theme = G2VTheme::for_appearance(appearance);
            let palette =
                ButtonVariant::PopoverItem.palette(&theme, appearance == WindowAppearance::Dark);

            assert_eq!(palette.hover_background, theme.popover_hover);
            assert_eq!(palette.expanded_background, Some(theme.popover_selected));
        }
    }

    #[test]
    fn invalid_colors_match_light_and_dark_react_contracts() {
        let light = G2VTheme::for_appearance(WindowAppearance::Light);
        let dark = G2VTheme::for_appearance(WindowAppearance::Dark);

        assert_eq!(
            state_colors(
                &light,
                ButtonVariant::Outline.palette(&light, false),
                false,
                true
            ),
            (light.destructive, light.destructive.opacity(0.2))
        );
        assert_eq!(
            state_colors(
                &dark,
                ButtonVariant::Outline.palette(&dark, true),
                true,
                true
            ),
            (dark.destructive.opacity(0.5), dark.destructive.opacity(0.4))
        );
    }
}
