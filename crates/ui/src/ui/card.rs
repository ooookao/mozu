//! G2V's shadcn-compatible intrinsic card composition.

use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled as _, Window, div};

use crate::theme::border::BORDER_DEFAULT;
use crate::theme::colors::G2VTheme;
use crate::theme::radius::RADIUS_XL;
use crate::theme::spacing::{SPACE_3, SPACE_4};
use crate::theme::typography;

use super::TextStyleExt as _;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CardSize {
    #[default]
    Default,
    Small,
}

fn spacing(size: CardSize) -> gpui::Pixels {
    match size {
        CardSize::Default => SPACE_4,
        CardSize::Small => SPACE_3,
    }
}

macro_rules! card_container {
    ($name:ident) => {
        #[derive(Default, IntoElement)]
        pub struct $name {
            children: Vec<AnyElement>,
        }

        impl $name {
            pub fn new() -> Self {
                Self::default()
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }
    };
}

card_container!(Card);
card_container!(CardHeader);
card_container!(CardTitle);
card_container!(CardDescription);
card_container!(CardAction);
card_container!(CardContent);
card_container!(CardFooter);

impl Card {
    #[must_use]
    pub fn size(mut self, size: CardSize) -> SizedCard {
        let children = std::mem::take(&mut self.children);
        SizedCard { size, children }
    }
}

#[derive(IntoElement)]
pub struct SizedCard {
    size: CardSize,
    children: Vec<AnyElement>,
}

impl ParentElement for SizedCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Card {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        SizedCard {
            size: CardSize::Default,
            children: self.children,
        }
        .render(window, cx)
    }
}

impl RenderOnce for SizedCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let space = spacing(self.size);
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(space)
            .overflow_hidden()
            .rounded(RADIUS_XL)
            .border(BORDER_DEFAULT)
            .border_color(theme.foreground.opacity(0.1))
            .bg(theme.card)
            .py(space)
            .text_style(typography::tokens().sm)
            .text_color(theme.card_foreground)
            .children(self.children)
    }
}

impl RenderOnce for CardHeader {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .w_full()
            .min_w_0()
            .flex()
            .items_start()
            .justify_between()
            .gap_1()
            .rounded_t(RADIUS_XL)
            .px_4()
            .children(self.children)
    }
}

impl RenderOnce for CardTitle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .min_w_0()
            .whitespace_normal()
            .text_style(typography::tokens().md)
            .children(self.children)
    }
}

impl RenderOnce for CardDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .min_w_0()
            .whitespace_normal()
            .text_style(typography::tokens().sm)
            .text_color(G2VTheme::global(cx).muted_foreground)
            .children(self.children)
    }
}

impl RenderOnce for CardAction {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().flex_none().children(self.children)
    }
}

impl RenderOnce for CardContent {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .px_4()
            .children(self.children)
    }
}

impl RenderOnce for CardFooter {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        div()
            .flex()
            .items_center()
            .rounded_b(RADIUS_XL)
            .border_t(BORDER_DEFAULT)
            .border_color(theme.border)
            .bg(theme.muted.opacity(0.5))
            .p_4()
            .children(self.children)
    }
}
