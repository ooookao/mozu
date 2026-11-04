//! Keyboard-key presentation primitives.

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder as _,
};
use gpui_base::StyledExt as _;

use crate::theme::colors::G2VTheme;
use crate::theme::radius::RADIUS_SM;
use crate::theme::sizing::{SIZE_3, SIZE_5};
use crate::theme::typography;

use super::{Icon, IconName, TextStyleExt as _};

const HEIGHT: gpui::Rems = SIZE_5;
const MIN_WIDTH: gpui::Rems = SIZE_5;
const ICON_SIZE: gpui::Rems = SIZE_3;

#[derive(IntoElement, Default)]
#[must_use]
pub struct Kbd {
    label: Option<SharedString>,
    icon: Option<IconName>,
    children: Vec<AnyElement>,
    in_tooltip: bool,
    style: StyleRefinement,
}

impl Kbd {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn in_tooltip(mut self) -> Self {
        self.in_tooltip = true;
        self
    }
}

impl ParentElement for Kbd {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Kbd {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Kbd {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let is_dark = matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        let (background, foreground) = kbd_colors(theme, is_dark, self.in_tooltip);
        let typography = typography::tokens();

        div()
            .flex()
            .flex_shrink_0()
            .h(HEIGHT)
            .w_auto()
            .min_w(MIN_WIDTH)
            .items_center()
            .justify_center()
            .gap_1()
            .rounded(RADIUS_SM)
            .bg(background)
            .px_1()
            .text_style(typography.xs)
            .text_color(foreground)
            .refine_style(&self.style)
            .when_some(self.icon, |element, icon| {
                element.child(Icon::new(icon, ICON_SIZE).color(foreground))
            })
            .children(self.children)
            .when_some(self.label, gpui::ParentElement::child)
    }
}

fn kbd_colors(theme: &G2VTheme, is_dark: bool, in_tooltip: bool) -> (gpui::Hsla, gpui::Hsla) {
    if in_tooltip {
        (
            theme
                .popover_foreground
                .opacity(if is_dark { 0.1 } else { 0.08 }),
            theme.popover_foreground,
        )
    } else {
        (theme.muted, theme.muted_foreground)
    }
}

#[derive(IntoElement, Default)]
#[must_use]
pub struct KbdGroup {
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl KbdGroup {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for KbdGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for KbdGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for KbdGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_1()
            .refine_style(&self.style)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::WindowAppearance;
    use gpui::px;

    #[test]
    fn dimensions_match_the_react_kbd_contract() {
        assert_eq!(HEIGHT.to_pixels(px(16.0)), px(20.0));
        assert_eq!(MIN_WIDTH.to_pixels(px(16.0)), px(20.0));
        assert_eq!(ICON_SIZE.to_pixels(px(16.0)), px(12.0));
        assert_eq!(RADIUS_SM, px(7.92));
    }

    #[test]
    fn tooltip_colors_match_light_and_dark_ancestor_variants() {
        let light = G2VTheme::for_appearance(WindowAppearance::Light);
        let dark = G2VTheme::for_appearance(WindowAppearance::Dark);

        assert_eq!(
            kbd_colors(&light, false, true),
            (
                light.popover_foreground.opacity(0.08),
                light.popover_foreground
            )
        );
        assert_eq!(
            kbd_colors(&dark, true, true),
            (
                dark.popover_foreground.opacity(0.1),
                dark.popover_foreground
            )
        );
    }
}
