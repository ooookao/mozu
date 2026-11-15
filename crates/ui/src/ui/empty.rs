use crate::theme::Theme;
use crate::ui::utils::presentation::{ComponentTheme, ControlSize, TextRole};
use gpui::{
    AnyElement, App, Div, IntoElement, ParentElement, Refineable, RenderOnce, StyleRefinement,
    Styled, Window, div, px,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyMediaVariant {
    Default,
    Icon,
}

struct Presentation {
    root: StyleRefinement,
    header: StyleRefinement,
    media: StyleRefinement,
    icon_media: StyleRefinement,
    icon_size: f32,
    title: StyleRefinement,
    description: StyleRefinement,
    content: StyleRefinement,
}
fn presentation(window: &Window, cx: &App) -> Presentation {
    let theme = Theme::current(window, cx);
    let tokens = ComponentTheme::current(cx);
    let compact = tokens.size == ControlSize::Compact;
    let title = TextRole::Subtitle.metrics(tokens.size, &tokens);
    let body = TextRole::Body.metrics(tokens.size, &tokens);
    Presentation {
        root: StyleRefinement::default()
            .flex()
            .w_full()
            .min_w(px(0.))
            .flex_1()
            .flex_col()
            .items_center()
            .justify_center()
            .text_center()
            .font_family(tokens.typography.family.clone())
            .font_variations(tokens.typography.normal.variations())
            .text_color(theme.foreground)
            .gap(px(if compact { 12. } else { 16. }))
            .p(px(if compact { 24. } else { 32. }))
            .rounded(px(tokens.shape.radii(&tokens).container)),
        header: StyleRefinement::default()
            .flex()
            .w_full()
            .max_w(px(384.))
            .min_w(px(0.))
            .flex_col()
            .items_center()
            .gap(px(if compact { 2. } else { 4. })),
        media: StyleRefinement::default()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .mb(px(if compact { 6. } else { 8. })),
        icon_media: StyleRefinement::default()
            .size(px(if compact { 32. } else { 40. }))
            .rounded(px(tokens.shape.radii(&tokens).item))
            .bg(theme.hover())
            .text_color(theme.muted_foreground),
        icon_size: if compact { 16. } else { 20. },
        title: StyleRefinement::default()
            .max_w_full()
            .min_w(px(0.))
            .text_size(px(title.size))
            .line_height(px(title.line_height))
            .font_variations(tokens.typography.semibold.variations()),
        description: StyleRefinement::default()
            .flex()
            .flex_col()
            .items_center()
            .w_full()
            .min_w(px(0.))
            .text_size(px(body.size))
            .line_height(px(body.line_height))
            .text_color(theme.muted_foreground),
        content: StyleRefinement::default()
            .flex()
            .w_full()
            .max_w(px(384.))
            .min_w(px(0.))
            .flex_col()
            .items_center()
            .gap(px(if compact { 6. } else { 8. }))
            .text_size(px(body.size))
            .line_height(px(body.line_height)),
    }
}

/// An editable empty-state layout. Actions and visibility belong to the application.
#[derive(IntoElement)]
pub struct Empty {
    style: StyleRefinement,
    header_style: StyleRefinement,
    content_style: StyleRefinement,
    media: Option<(EmptyMediaVariant, Box<dyn FnOnce(f32) -> AnyElement>)>,
    title: Option<AnyElement>,
    description: Option<AnyElement>,
    content: Option<AnyElement>,
    children: Vec<AnyElement>,
}
impl Empty {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            header_style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            media: None,
            title: None,
            description: None,
            content: None,
            children: Vec::new(),
        }
    }
    pub fn media<E: IntoElement + Styled + 'static>(
        mut self,
        variant: EmptyMediaVariant,
        mut element: E,
    ) -> Self {
        self.media = Some((
            variant,
            Box::new(move |size| {
                if variant == EmptyMediaVariant::Icon {
                    let caller = element.style().clone();
                    element
                        .style()
                        .refine(&StyleRefinement::default().size(px(size)));
                    element.style().refine(&caller);
                }
                element.into_any_element()
            }),
        ));
        self
    }
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }
    pub fn description(mut self, description: impl IntoElement) -> Self {
        self.description = Some(description.into_any_element());
        self
    }
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }
    pub fn header_style(mut self, style: StyleRefinement) -> Self {
        self.header_style = style;
        self
    }
    pub fn content_style(mut self, style: StyleRefinement) -> Self {
        self.content_style = style;
        self
    }
}
impl Default for Empty {
    fn default() -> Self {
        Self::new()
    }
}
impl Styled for Empty {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl ParentElement for Empty {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}
fn styled_part(style: &StyleRefinement) -> Div {
    let mut part = div();
    part.style().refine(style);
    part
}
impl RenderOnce for Empty {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let recipe = presentation(window, cx);
        let mut root = styled_part(&recipe.root);
        root.style().refine(&self.style);
        if self.media.is_some() || self.title.is_some() || self.description.is_some() {
            let mut header = styled_part(&recipe.header);
            header.style().refine(&self.header_style);
            if let Some((variant, render)) = self.media {
                let mut media = styled_part(&recipe.media);
                if variant == EmptyMediaVariant::Icon {
                    media.style().refine(&recipe.icon_media);
                }
                header = header.child(media.child(render(recipe.icon_size)));
            }
            if let Some(title) = self.title {
                header = header.child(styled_part(&recipe.title).child(title));
            }
            if let Some(description) = self.description {
                header = header.child(styled_part(&recipe.description).child(description));
            }
            root = root.child(header);
        }
        if let Some(content) = self.content {
            let mut slot = styled_part(&recipe.content);
            slot.style().refine(&self.content_style);
            root = root.child(slot.child(content));
        }
        root.children(self.children)
    }
}
