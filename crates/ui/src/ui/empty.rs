//! Shared empty-state composition matching G2V's React UI.

use gpui::{
    AnyElement, App, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _,
    Window, div, prelude::FluentBuilder as _, rems,
};

use crate::theme::{
    colors::G2VTheme,
    radius::{RADIUS_LG, RADIUS_XL},
    sizing::SIZE_8,
    typography,
};

use super::TextStyleExt as _;

const EMPTY_MAX_WIDTH: gpui::Rems = rems(24.0);
const EMPTY_ICON_SIZE: gpui::Rems = SIZE_8;
const RELAXED_LINE_HEIGHT: gpui::Rems = rems(1.421_875);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EmptyMediaVariant {
    #[default]
    Default,
    Icon,
}

#[derive(IntoElement)]
#[must_use]
pub struct Empty {
    title: SharedString,
    description: Option<SharedString>,
    media: Option<(EmptyMediaVariant, AnyElement)>,
    content: Option<AnyElement>,
}

impl Empty {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            description: None,
            media: None,
            content: None,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn media(mut self, variant: EmptyMediaVariant, media: impl IntoElement) -> Self {
        self.media = Some((variant, media.into_any_element()));
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }
}

impl RenderOnce for Empty {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let header = div()
            .w_full()
            .max_w(EMPTY_MAX_WIDTH)
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .when_some(self.media, |header, (variant, media)| {
                header.child(
                    div()
                        .mb_2()
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(variant == EmptyMediaVariant::Icon, |media_container| {
                            media_container
                                .size(EMPTY_ICON_SIZE)
                                .rounded(RADIUS_LG)
                                .bg(theme.muted)
                                .text_color(theme.foreground)
                        })
                        .child(media),
                )
            })
            .child(div().text_style(typography::tokens().sm).child(self.title))
            .when_some(self.description, |header, description| {
                header.child(
                    div()
                        .text_style(typography::tokens().sm)
                        .line_height(RELAXED_LINE_HEIGHT)
                        .text_color(theme.muted_foreground)
                        .child(description),
                )
            });

        div()
            .w_full()
            .min_w_0()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .rounded(RADIUS_XL)
            .p_6()
            .text_center()
            .child(header)
            .when_some(self.content, |empty, content| {
                empty.child(
                    div()
                        .w_full()
                        .min_w_0()
                        .max_w(EMPTY_MAX_WIDTH)
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2p5()
                        .text_style(typography::tokens().sm)
                        .child(content),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::spacing::{SPACE_2_5, SPACE_4, SPACE_6};
    use gpui::px;

    #[test]
    fn dimensions_follow_the_window_rem_size() {
        assert_eq!(SPACE_4, px(16.0));
        assert_eq!(SPACE_6, px(24.0));
        assert_eq!(SPACE_2_5, px(10.0));
        assert_eq!(EMPTY_MAX_WIDTH.to_pixels(px(16.0)), px(384.0));
        assert_eq!(EMPTY_ICON_SIZE.to_pixels(px(16.0)), px(32.0));
        assert_eq!(RELAXED_LINE_HEIGHT.to_pixels(px(16.0)), px(22.75));

        assert_eq!(EMPTY_MAX_WIDTH.to_pixels(px(18.0)), px(432.0));
        assert_eq!(EMPTY_ICON_SIZE.to_pixels(px(18.0)), px(36.0));
        assert_eq!(RELAXED_LINE_HEIGHT.to_pixels(px(18.0)), px(25.59375));
    }
}
