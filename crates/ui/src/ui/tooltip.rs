use crate::ui::utils::presentation::{ComponentTheme, TextRole, ThemeScheme, TypographyMetrics};
use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, Pixels, Refineable, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, div, px,
};
use kinu_ui::{ColorScheme, Tooltip as TooltipPrimitive, TooltipOpenChangeReason};
pub use kinu_ui::{PopupAlign, PopupSide, TooltipCursorAxis, TooltipGroup};
use std::time::Duration;

fn primitive(id: impl Into<ElementId>, open: bool, description: SharedString) -> TooltipPrimitive {
    TooltipPrimitive::new(id, open)
        .description(description)
        .side_offset(px(8.))
}

/// Returns the styled native tooltip so links and passive labels retain their own input semantics.
pub fn tooltip_with_tokens(
    id: impl Into<ElementId>,
    open: bool,
    description: impl Into<SharedString>,
    tokens: ComponentTheme,
    window: &mut Window,
    cx: &mut App,
) -> TooltipPrimitive {
    let description = description.into();
    let content = tooltip_caption_with_tokens(description.clone().into_any_element(), &tokens, cx);
    tooltip_with_content_with_tokens(id, open, description, content, tokens, window, cx)
}

/// Accepts caller-composed content while retaining native tooltip input and the shared surface.
pub fn tooltip_with_content_with_tokens(
    id: impl Into<ElementId>,
    open: bool,
    description: impl Into<SharedString>,
    content: impl IntoElement,
    tokens: ComponentTheme,
    window: &mut Window,
    cx: &mut App,
) -> TooltipPrimitive {
    primitive(id, open, description.into())
        .open_delay(Duration::from_millis(
            tokens.settings.timings.tooltip_delay_ms,
        ))
        .motion(motion(tokens.clone()))
        .content(surface(
            content.into_any_element(),
            tokens,
            StyleRefinement::default(),
            window,
            cx,
        ))
}

/// An inverted descriptive surface. The primitive owns input, placement, and lifetime.
#[derive(IntoElement)]
pub struct Tooltip {
    primitive: TooltipPrimitive,
    trigger: kinu_ui::Button,
    description: SharedString,
    content: Option<AnyElement>,
    tokens: Option<ComponentTheme>,
    style: StyleRefinement,
}
impl Tooltip {
    pub fn new(
        id: impl Into<ElementId>,
        open: bool,
        trigger: kinu_ui::Button,
        description: impl Into<SharedString>,
    ) -> Self {
        let description = description.into();
        Self {
            primitive: primitive(id, open, description.clone()),
            trigger,
            description,
            content: None,
            tokens: None,
            style: StyleRefinement::default(),
        }
    }
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }
    pub fn tokens(mut self, tokens: ComponentTheme) -> Self {
        self.tokens = Some(tokens.clone());
        self
    }
    pub fn group(mut self, group: &TooltipGroup) -> Self {
        self.primitive = self.primitive.group(group);
        self
    }
    pub fn follow_cursor(mut self, axis: TooltipCursorAxis) -> Self {
        self.primitive = self.primitive.follow_cursor(axis);
        self
    }
    pub fn side(mut self, side: PopupSide) -> Self {
        self.primitive = self.primitive.side(side);
        self
    }
    pub fn align(mut self, align: PopupAlign) -> Self {
        self.primitive = self.primitive.align(align);
        self
    }
    pub fn side_offset(mut self, offset: Pixels) -> Self {
        self.primitive = self.primitive.side_offset(offset);
        self
    }
    pub fn align_offset(mut self, offset: Pixels) -> Self {
        self.primitive = self.primitive.align_offset(offset);
        self
    }
    pub fn open_delay(mut self, delay: Duration) -> Self {
        self.primitive = self.primitive.open_delay(delay);
        self
    }
    pub fn close_delay(mut self, delay: Duration) -> Self {
        self.primitive = self.primitive.close_delay(delay);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.primitive = self.primitive.disabled(disabled);
        self
    }
    pub fn on_open_change(
        mut self,
        change: impl Fn(bool, TooltipOpenChangeReason, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.primitive = self.primitive.on_open_change(change);
        self
    }
}
impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
fn motion(tokens: ComponentTheme) -> kinu_ui::PopupMotion {
    let profile = tokens.motion.fast;
    kinu_ui::PopupMotion {
        enter: profile.entering().with_easing(move |progress| {
            if progress >= 1. {
                return 1.;
            }
            profile
                .spring
                .step(
                    gpui::SpringState {
                        position: 0.,
                        velocity: 0.,
                    },
                    1.,
                    progress * profile.enter.as_secs_f32(),
                )
                .position
        }),
        exit: profile
            .exiting()
            .with_easing(tokens.motion.curves.in_out.curve()),
        initial_opacity: 0.,
        initial_scale: 1.,
        enter_offset: px(4.),
        exit_offset: px(4.),
        origin: kinu_ui::PopupTransformOrigin::Aligned,
    }
}
/// Trims one caption line to its cap height and alphabetic baseline.
pub fn tooltip_caption_with_tokens(
    child: AnyElement,
    tokens: &ComponentTheme,
    cx: &App,
) -> gpui::Div {
    let metrics = TextRole::Caption.metrics(tokens.size, tokens);
    let mut font = gpui::font(tokens.typography.family.clone());
    font.variations = tokens.typography.normal.variations();
    let font_id = cx.text_system().resolve_font(&font);
    let baseline =
        cx.text_system()
            .baseline_offset(font_id, px(metrics.size), px(metrics.line_height));
    let cap = cx.text_system().cap_height(font_id, px(metrics.size));
    div()
        .min_w(px(0.))
        .mt(-(baseline - cap))
        .mb(-(px(metrics.line_height) - baseline))
        .child(child)
}
fn surface(
    child: AnyElement,
    tokens: ComponentTheme,
    style: StyleRefinement,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    let colors = tokens.extras(if ColorScheme::current(window, cx) == ColorScheme::Dark {
        ThemeScheme::Dark
    } else {
        ThemeScheme::Light
    });
    let metrics = TextRole::Caption.metrics(tokens.size, &tokens);
    let mut content = div()
        .flex()
        .items_center()
        .px(px(8.))
        .py(px(8.))
        .rounded(px(tokens.shape.radii(&tokens).item))
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.normal.weight)
        .font_variations(tokens.typography.normal.variations())
        .text_size(px(metrics.size))
        .line_height(px(metrics.line_height))
        .bg(gpui::Hsla::from(colors.foreground))
        .text_color(gpui::Hsla::from(colors.background))
        .child(child);
    content.style().refine(&style);
    content
}
impl RenderOnce for Tooltip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = ComponentTheme::resolve(self.tokens.clone(), cx);
        let child = self
            .content
            .unwrap_or_else(|| self.description.into_any_element());
        let content = surface(
            tooltip_caption_with_tokens(child, &tokens, cx).into_any_element(),
            tokens.clone(),
            self.style,
            window,
            cx,
        );
        self.primitive
            .open_delay(Duration::from_millis(
                tokens.settings.timings.tooltip_delay_ms,
            ))
            .motion(motion(tokens.clone()))
            .trigger(self.trigger)
            .content(content)
    }
}

/// Styles a keyboard cap for an inverted tooltip surface.
pub fn tooltip_shortcut_with_tokens(
    child: impl IntoElement,
    metrics: TypographyMetrics,
    tokens: &ComponentTheme,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    let style = crate::ui::kbd::presentation_with_tokens(
        crate::ui::kbd::KbdSurface::Tooltip,
        tokens,
        window,
        cx,
    )
    .key;
    let mut key = div().my(px(-4.)).child(child);
    key.style().refine(&style);
    key.text_size(px(metrics.size))
        .line_height(px(metrics.line_height))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn tooltip(
    id: impl Into<ElementId>,
    open: bool,
    description: impl Into<SharedString>,
    window: &mut Window,
    cx: &mut App,
) -> TooltipPrimitive {
    tooltip_with_tokens(
        id,
        open,
        description,
        ComponentTheme::current(cx),
        window,
        cx,
    )
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn tooltip_with_content(
    id: impl Into<ElementId>,
    open: bool,
    description: impl Into<SharedString>,
    content: impl IntoElement,
    window: &mut Window,
    cx: &mut App,
) -> TooltipPrimitive {
    tooltip_with_content_with_tokens(
        id,
        open,
        description,
        content,
        ComponentTheme::current(cx),
        window,
        cx,
    )
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn tooltip_caption(child: AnyElement, cx: &App) -> gpui::Div {
    tooltip_caption_with_tokens(child, &ComponentTheme::current(cx), cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn tooltip_shortcut(
    child: impl IntoElement,
    metrics: TypographyMetrics,
    window: &Window,
    cx: &App,
) -> gpui::Div {
    tooltip_shortcut_with_tokens(child, metrics, &ComponentTheme::current(cx), window, cx)
}
