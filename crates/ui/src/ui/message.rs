//! Message row composition matching G2V's React shadcn component.

use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder as _,
};
use gpui_base::StyledExt as _;

use crate::theme::{colors::G2VTheme, radius::RADIUS_FULL, sizing::SIZE_8, typography};

use super::TextStyleExt as _;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageAlign {
    #[default]
    Start,
    End,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageChromeSpacing {
    #[default]
    Padded,
    Flush,
}

#[derive(IntoElement)]
#[must_use]
pub struct MessageGroup {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MessageGroup {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl Default for MessageGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for MessageGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Message {
    id: ElementId,
    align: MessageAlign,
    accessibility_label: SharedString,
    style: StyleRefinement,
    avatar: Option<MessageAvatar>,
    hover_group: Option<SharedString>,
    content: MessageContent,
}

impl Message {
    pub fn new(
        id: impl Into<ElementId>,
        align: MessageAlign,
        accessibility_label: impl Into<SharedString>,
        content: MessageContent,
    ) -> Self {
        Self {
            id: id.into(),
            align,
            accessibility_label: accessibility_label.into(),
            style: StyleRefinement::default(),
            avatar: None,
            hover_group: None,
            content,
        }
    }

    pub fn avatar(mut self, avatar: MessageAvatar) -> Self {
        self.avatar = Some(avatar);
        self
    }

    pub fn hover_group(mut self, group: impl Into<SharedString>) -> Self {
        self.hover_group = Some(group.into());
        self
    }
}

impl Styled for Message {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Message {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let align_end = self.align == MessageAlign::End;
        let has_footer = self.content.footer.is_some();
        div()
            .id(self.id)
            .role(Role::Article)
            .aria_label(self.accessibility_label)
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .gap_2()
            .text_style(typography::tokens().sm)
            .when(align_end, gpui::Styled::flex_row_reverse)
            .refine_style(&self.style)
            .when_some(self.hover_group, gpui::InteractiveElement::group)
            .when_some(self.avatar, |message, avatar| {
                message.child(avatar.with_footer_offset(has_footer))
            })
            .child(self.content.render_for_alignment(self.align))
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct MessageAvatar {
    style: StyleRefinement,
    child: AnyElement,
    footer_offset: bool,
}

impl MessageAvatar {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            style: StyleRefinement::default(),
            child: child.into_any_element(),
            footer_offset: false,
        }
    }

    fn with_footer_offset(mut self, footer_offset: bool) -> Self {
        self.footer_offset = footer_offset;
        self
    }
}

impl Styled for MessageAvatar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageAvatar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .min_w(SIZE_8)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .self_end()
            .overflow_hidden()
            .rounded(RADIUS_FULL)
            .bg(G2VTheme::global(cx).muted)
            .when(self.footer_offset, |avatar| {
                avatar.relative().bottom(SIZE_8)
            })
            .refine_style(&self.style)
            .child(self.child)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct MessageContent {
    style: StyleRefinement,
    header: Option<MessageHeader>,
    body: Vec<AnyElement>,
    footer: Option<MessageFooter>,
}

impl MessageContent {
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            header: None,
            body: Vec::new(),
            footer: None,
        }
    }

    pub fn header(mut self, header: MessageHeader) -> Self {
        self.header = Some(header);
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.body.push(child.into_any_element());
        self
    }

    pub fn footer(mut self, footer: MessageFooter) -> Self {
        self.footer = Some(footer);
        self
    }

    fn render_for_alignment(self, align: MessageAlign) -> AnyElement {
        let align_end = align == MessageAlign::End;
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2p5()
            .refine_style(&self.style)
            .when_some(self.header, |content, header| {
                content.child(header.with_end_alignment(align_end))
            })
            .children(self.body)
            .when_some(self.footer, |content, footer| {
                content.child(footer.with_end_alignment(align_end))
            })
            .into_any_element()
    }
}

impl Default for MessageContent {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for MessageContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MessageContent {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.render_for_alignment(MessageAlign::Start)
    }
}

macro_rules! message_chrome {
    ($name:ident) => {
        #[derive(IntoElement)]
        #[must_use]
        pub struct $name {
            spacing: MessageChromeSpacing,
            style: StyleRefinement,
            child: AnyElement,
            align_end: bool,
        }

        impl $name {
            pub fn new(spacing: MessageChromeSpacing, child: impl IntoElement) -> Self {
                Self {
                    spacing,
                    style: StyleRefinement::default(),
                    child: child.into_any_element(),
                    align_end: false,
                }
            }

            fn with_end_alignment(mut self, align_end: bool) -> Self {
                self.align_end = align_end;
                self
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl RenderOnce for $name {
            fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
                div()
                    .max_w_full()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .when(self.spacing == MessageChromeSpacing::Padded, |chrome| {
                        chrome.px_3()
                    })
                    .text_style(typography::tokens().xs)
                    .text_color(G2VTheme::global(cx).muted_foreground)
                    .when(self.align_end, gpui::Styled::self_end)
                    .refine_style(&self.style)
                    .child(self.child)
            }
        }
    };
}

message_chrome!(MessageHeader);
message_chrome!(MessageFooter);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::spacing::{SPACE_2, SPACE_2_5, SPACE_3};
    use gpui::px;

    #[test]
    fn geometry_matches_the_react_message_contract() {
        assert_eq!(SPACE_2, px(8.0));
        assert_eq!(SPACE_2_5, px(10.0));
        assert_eq!(SPACE_3, px(12.0));
        assert_eq!(SIZE_8.to_pixels(px(16.0)), px(32.0));
    }

    #[test]
    fn footer_presence_controls_avatar_offset() {
        let without_footer = MessageContent::new().child("Body");
        let with_footer = MessageContent::new()
            .child("Body")
            .footer(MessageFooter::new(MessageChromeSpacing::Padded, "Footer"));

        assert!(without_footer.footer.is_none());
        assert!(with_footer.footer.is_some());
    }

    #[test]
    fn hover_group_is_owned_by_the_full_message_row() {
        let message = Message::new(
            "message",
            MessageAlign::Start,
            "Assistant",
            MessageContent::new().child("Body"),
        )
        .hover_group("message-hover");

        assert_eq!(message.hover_group.as_deref(), Some("message-hover"));
    }
}
