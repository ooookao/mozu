use crate::ui::utils::presentation::{ComponentTheme, ControlSize, TextRole, ThemeScheme};
use gpui::{
    AnyElement, App, ElementContextScope, IntoElement, ParentElement, Refineable, RenderOnce,
    StyleRefinement, Styled, Window, div, px,
};
use kinu_ui::ColorScheme;

/// The surrounding surface changes keyboard-hint contrast without changing its meaning.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KbdSurface {
    #[default]
    Default,
    Tooltip,
    InputGroup,
}
impl KbdSurface {
    pub fn scope(self, child: impl IntoElement) -> impl IntoElement {
        ElementContextScope::new(self, child)
    }
}
pub(crate) struct Presentation {
    pub(crate) key: StyleRefinement,
    pub(crate) group: StyleRefinement,
}
pub(crate) fn presentation(surface: KbdSurface, window: &Window, cx: &App) -> Presentation {
    presentation_with_tokens(surface, &ComponentTheme::current(cx), window, cx)
}

pub(crate) fn presentation_with_tokens(
    surface: KbdSurface,
    tokens: &ComponentTheme,
    window: &Window,
    cx: &App,
) -> Presentation {
    let colors = tokens.extras(if ColorScheme::current(window, cx) == ColorScheme::Dark {
        ThemeScheme::Dark
    } else {
        ThemeScheme::Light
    });
    let metrics = TextRole::Micro.metrics(tokens.size, tokens);
    let height = if tokens.size == ControlSize::Compact {
        16.
    } else {
        20.
    };
    let mut key = StyleRefinement::default()
        .flex()
        .items_center()
        .justify_center()
        .w_auto()
        .h(px(height))
        .min_w(px(height))
        .gap(px(4.))
        .px(px(4.))
        .rounded(px(5.))
        .font_family(tokens.typography.family.clone())
        .text_size(px(metrics.size))
        .line_height(px(metrics.line_height))
        .bg(colors.hover())
        .text_color(gpui::Hsla::from(colors.muted_foreground));
    match surface {
        KbdSurface::Default | KbdSurface::InputGroup => {}
        KbdSurface::Tooltip => {
            key = key
                .h(px(16.))
                .min_w(px(16.))
                .rounded(px(4.))
                .border_1()
                .bg(gpui::transparent_black())
                .border_color(gpui::Hsla::from(colors.background).opacity(0.3))
                .text_color(gpui::Hsla::from(colors.background));
        }
    }
    let group = StyleRefinement::default().flex().items_center().gap(px(4.));
    Presentation { key, group }
}

/// A static keyboard key or shortcut hint, without registering a key binding.
#[derive(IntoElement, Default)]
pub struct Kbd {
    surface: Option<KbdSurface>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}
impl Kbd {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn surface(mut self, surface: KbdSurface) -> Self {
        self.surface = Some(surface);
        self
    }
    /// Applies the default 12px icon size only when the icon has no explicit size.
    pub fn icon(mut self, mut icon: impl IntoElement + Styled) -> Self {
        if icon.style().size.width.is_none() && icon.style().size.height.is_none() {
            icon = icon.size(px(12.));
        }
        self.children.push(icon.into_any_element());
        self
    }
}
impl Styled for Kbd {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for Kbd {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}
impl RenderOnce for Kbd {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let surface = self
            .surface
            .or_else(|| window.element_context::<KbdSurface>().copied())
            .unwrap_or(KbdSurface::Default);
        let p = presentation(surface, window, cx);
        let mut key = div().children(self.children);
        key.style().refine(&p.key);
        key.style().refine(&self.style);
        key
    }
}

/// A static inline row of key hints and optional separators.
#[derive(IntoElement, Default)]
pub struct KbdGroup {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}
impl KbdGroup {
    pub fn new() -> Self {
        Self::default()
    }
}
impl Styled for KbdGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for KbdGroup {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}
impl RenderOnce for KbdGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut group = div().children(self.children);
        group
            .style()
            .refine(&presentation(KbdSurface::Default, window, cx).group);
        group.style().refine(&self.style);
        group
    }
}
