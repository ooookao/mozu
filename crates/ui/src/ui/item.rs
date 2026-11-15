use crate::theme::Theme;
use crate::ui::separator::Separator;
use crate::ui::utils::presentation::{ComponentTheme, ControlSize, TextRole};
use gpui::{
    AnyElement, App, ElementId, InteractiveElement, IntoElement, Link, ParentElement, Refineable,
    RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemVariant {
    Default,
    Outline,
    Muted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemSize {
    Default,
    Sm,
    Xs,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemMediaVariant {
    Default,
    Icon,
    Image,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemContentKind {
    TitleOnly,
    WithDescription,
}
pub(crate) struct Presentation {
    pub root: StyleRefinement,
    pub hover: StyleRefinement,
    pub focus: StyleRefinement,
    pub media: StyleRefinement,
    pub media_icon_size: f32,
    pub content: StyleRefinement,
    pub title: StyleRefinement,
    pub description: StyleRefinement,
    pub actions: StyleRefinement,
    pub header: StyleRefinement,
    pub footer: StyleRefinement,
    pub group_gap: f32,
}
pub(crate) fn presentation(
    variant: ItemVariant,
    size: ItemSize,
    media_variant: ItemMediaVariant,
    content_kind: ItemContentKind,
    window: &Window,
    cx: &App,
) -> Presentation {
    let theme = Theme::current(window, cx);
    let tokens = ComponentTheme::current(cx);
    let compact = tokens.size == ControlSize::Compact;
    let title = match size {
        ItemSize::Xs => TextRole::Body,
        ItemSize::Default | ItemSize::Sm => TextRole::Subtitle,
    }
    .metrics(tokens.size, &tokens);
    let description = match size {
        ItemSize::Xs => TextRole::Caption,
        ItemSize::Default | ItemSize::Sm => TextRole::Body,
    }
    .metrics(tokens.size, &tokens);
    let (inset, gap) = match (size, compact) {
        (ItemSize::Default, false) => (16., 12.),
        (ItemSize::Default, true) => (12., 10.),
        (ItemSize::Sm, false) => (12., 10.),
        (ItemSize::Sm, true) => (10., 8.),
        (ItemSize::Xs, false) => (8., 8.),
        (ItemSize::Xs, true) => (6., 6.),
    };
    let radius = tokens.shape.radii(&tokens).container;
    let root = StyleRefinement::default()
        .flex()
        .w_full()
        .min_w(px(0.))
        .flex_wrap()
        .items_center()
        .gap(px(gap))
        .p(px(inset))
        .rounded(px(radius))
        .border_1()
        .border_color(match variant {
            ItemVariant::Outline => theme.border.into(),
            _ => gpui::transparent_black(),
        })
        .bg(match variant {
            ItemVariant::Muted => theme.hover(),
            _ => gpui::transparent_black(),
        })
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.normal.weight)
        .font_variations(tokens.typography.normal.variations())
        .text_size(px(description.size))
        .line_height(px(description.line_height))
        .text_color(theme.foreground);
    let hover = StyleRefinement::default().bg(theme.hover());
    let focus =
        StyleRefinement::default().border_color(gpui::Hsla::from(theme.foreground).opacity(0.4));
    let icon_size = if compact { 16. } else { 20. };
    let mut media = StyleRefinement::default()
        .flex()
        .flex_none()
        .items_center()
        .justify_center();
    match media_variant {
        ItemMediaVariant::Default => {
            if content_kind == ItemContentKind::WithDescription {
                media = media.self_start().h(px(title.line_height));
            }
        }
        ItemMediaVariant::Icon => {
            media = media
                .size(px(if compact { 28. } else { 32. }))
                .rounded(px(tokens.shape.radii(&tokens).item))
                .bg(theme.hover());
        }
        ItemMediaVariant::Image => {
            let image_size = match size {
                ItemSize::Default => 40.,
                ItemSize::Sm => 32.,
                ItemSize::Xs => 24.,
            };
            media = media
                .size(px(image_size - if compact { 4. } else { 0. }))
                .overflow_hidden()
                .rounded(px(tokens.shape.radii(&tokens).item));
        }
    }
    let header = StyleRefinement::default()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap(px(gap));
    let mut header = header;
    header.flex_basis = Some(gpui::relative(1.).into());
    Presentation {
        root,
        hover,
        focus,
        media,
        media_icon_size: icon_size,
        content: StyleRefinement::default()
            .flex()
            .flex_1()
            .min_w(px(0.))
            .flex_col()
            .gap(px(4.)),
        title: StyleRefinement::default()
            .text_size(px(title.size))
            .line_height(px(title.line_height))
            .font_weight(tokens.typography.semibold.weight)
            .font_variations(tokens.typography.semibold.variations()),
        description: StyleRefinement::default()
            .line_clamp(2)
            .text_left()
            .text_color(theme.muted_foreground)
            .font_weight(tokens.typography.normal.weight)
            .font_variations(tokens.typography.normal.variations())
            .text_size(px(description.size))
            .line_height(px(description.line_height)),
        actions: StyleRefinement::default()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.)),
        footer: header.clone(),
        header,
        group_gap: gap,
    }
}

/// Editable content and actions, with native link behavior when a URL is supplied.
#[derive(IntoElement)]
pub struct Item {
    id: ElementId,
    title: SharedString,
    variant: ItemVariant,
    size: ItemSize,
    media: Option<(ItemMediaVariant, Box<dyn FnOnce(f32) -> AnyElement>)>,
    description: Option<AnyElement>,
    actions: Option<AnyElement>,
    header: Option<AnyElement>,
    footer: Option<AnyElement>,
    content: Option<AnyElement>,
    url: Option<SharedString>,
    style: StyleRefinement,
    content_style: StyleRefinement,
    title_role: Option<TextRole>,
    description_role: Option<TextRole>,
}
impl Item {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            variant: ItemVariant::Default,
            size: ItemSize::Default,
            media: None,
            description: None,
            actions: None,
            header: None,
            footer: None,
            content: None,
            url: None,
            style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            title_role: None,
            description_role: None,
        }
    }
    /// Overrides title typography while retaining the item size and spacing.
    pub fn title_role(mut self, role: TextRole) -> Self {
        self.title_role = Some(role);
        self
    }
    /// Overrides description typography independently of the item spacing.
    pub fn description_role(mut self, role: TextRole) -> Self {
        self.description_role = Some(role);
        self
    }
    pub fn variant(mut self, variant: ItemVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn size(mut self, size: ItemSize) -> Self {
        self.size = size;
        self
    }
    pub fn description(mut self, description: impl IntoElement) -> Self {
        self.description = Some(description.into_any_element());
        self
    }
    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }
    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }
    pub fn content_style(mut self, style: StyleRefinement) -> Self {
        self.content_style = style;
        self
    }
    pub fn link(mut self, url: impl Into<SharedString>) -> Self {
        self.url = Some(url.into());
        self
    }
    /// Uses the native image element's cover fit inside the preset image frame.
    pub fn image(self, source: impl Into<gpui::ImageSource>) -> Self {
        use gpui::StyledImage;
        self.media(
            ItemMediaVariant::Image,
            gpui::img(source).object_fit(gpui::ObjectFit::Cover),
        )
    }
    pub fn media<E: IntoElement + Styled + 'static>(
        mut self,
        variant: ItemMediaVariant,
        mut child: E,
    ) -> Self {
        self.media = Some((
            variant,
            Box::new(move |icon_size| {
                let caller = child.style().clone();
                match variant {
                    ItemMediaVariant::Default => {}
                    ItemMediaVariant::Icon => child
                        .style()
                        .refine(&StyleRefinement::default().size(px(icon_size))),
                    ItemMediaVariant::Image => child
                        .style()
                        .refine(&StyleRefinement::default().size_full()),
                }
                child.style().refine(&caller);
                child.into_any_element()
            }),
        ));
        self
    }
}
impl Styled for Item {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
fn part(style: &StyleRefinement) -> gpui::Div {
    let mut element = div();
    element.style().refine(style);
    element
}
impl RenderOnce for Item {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let media_kind = self
            .media
            .as_ref()
            .map_or(ItemMediaVariant::Default, |media| media.0);
        let mut p = presentation(
            self.variant,
            self.size,
            media_kind,
            if self.description.is_some() {
                ItemContentKind::WithDescription
            } else {
                ItemContentKind::TitleOnly
            },
            window,
            cx,
        );
        let tokens = ComponentTheme::current(cx);
        if let Some(role) = self.title_role {
            let metrics = role.metrics(tokens.size, &tokens);
            p.title = p
                .title
                .text_size(px(metrics.size))
                .line_height(px(metrics.line_height));
        }
        if let Some(role) = self.description_role {
            let metrics = role.metrics(tokens.size, &tokens);
            p.description = p
                .description
                .text_size(px(metrics.size))
                .line_height(px(metrics.line_height));
        }
        let mut contents = Vec::new();
        if let Some(header) = self.header {
            contents.push(part(&p.header).child(header).into_any_element());
        }
        if let Some((_, render)) = self.media {
            contents.push(
                part(&p.media)
                    .child(render(p.media_icon_size))
                    .into_any_element(),
            );
        }
        let mut body = part(&p.content);
        body.style().refine(&self.content_style);
        if let Some(custom) = self.content {
            body = body.child(custom);
        } else {
            body = body.child(part(&p.title).child(self.title.clone()));
            if let Some(description) = self.description {
                body = body.child(part(&p.description).child(description));
            }
        }
        contents.push(body.into_any_element());
        if let Some(actions) = self.actions {
            contents.push(part(&p.actions).child(actions).into_any_element());
        }
        if let Some(footer) = self.footer {
            contents.push(part(&p.footer).child(footer).into_any_element());
        }
        let mut style = p.root;
        style.refine(&self.style);
        if let Some(url) = self.url {
            let mut link = Link::new(self.id, url)
                .aria_label(self.title)
                .children(contents);
            link.style().refine(&style);
            let hover = p.hover;
            let focus = p.focus;
            link.hover(move |mut style| {
                style.refine(&hover);
                style
            })
            .focus_visible(move |mut style| {
                style.refine(&focus);
                style
            })
            .transition(gpui::StyleTransition::colors(
                ComponentTheme::current(cx)
                    .motion
                    .animations
                    .item
                    .animation(),
            ))
            .into_any_element()
        } else {
            let mut root = div().id(self.id).children(contents);
            root.style().refine(&style);
            root.into_any_element()
        }
    }
}

enum GroupEntry {
    Item(Item),
    Separator(ElementId),
}
/// A semantic list with size-aware spacing derived from explicitly added items.
#[derive(IntoElement)]
pub struct ItemGroup {
    id: ElementId,
    entries: Vec<GroupEntry>,
    style: StyleRefinement,
}
impl ItemGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            entries: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
    pub fn item(mut self, item: Item) -> Self {
        self.entries.push(GroupEntry::Item(item));
        self
    }
    pub fn separator(mut self, id: impl Into<ElementId>) -> Self {
        self.entries.push(GroupEntry::Separator(id.into()));
        self
    }
}
impl Styled for ItemGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for ItemGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let size = if self
            .entries
            .iter()
            .any(|entry| matches!(entry,GroupEntry::Item(item) if item.size==ItemSize::Xs))
        {
            ItemSize::Xs
        } else if self
            .entries
            .iter()
            .any(|entry| matches!(entry,GroupEntry::Item(item) if item.size==ItemSize::Sm))
        {
            ItemSize::Sm
        } else {
            ItemSize::Default
        };
        let p = presentation(
            ItemVariant::Default,
            size,
            ItemMediaVariant::Default,
            ItemContentKind::TitleOnly,
            window,
            cx,
        );
        let mut group = div()
            .id(self.id)
            .role(gpui::Role::List)
            .flex()
            .w_full()
            .flex_col()
            .gap(px(p.group_gap));
        group.style().refine(&self.style);
        for (index, entry) in self.entries.into_iter().enumerate() {
            group = match entry {
                GroupEntry::Item(item) => group.child(
                    div()
                        .id(("item", index))
                        .role(gpui::Role::ListItem)
                        .w_full()
                        .child(item),
                ),
                GroupEntry::Separator(id) => group.child(Separator::new(id).my(px(8.))),
            };
        }
        group
    }
}
