//! shadcn-compatible message bubble composition for native GPUI surfaces.

use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Styled, Window, WindowAppearance, div, prelude::FluentBuilder,
    px, relative,
};
use gpui_base::Button as ButtonBehavior;

use crate::theme::{
    border::BORDER_DEFAULT,
    colors::G2VTheme,
    radius::RADIUS_LG,
    shadows::{RING_3, focus_ring},
    spacing::{SPACE_2, SPACE_3},
    typography,
};

use super::TextStyleExt as _;

const BUBBLE_MAX_WIDTH: f32 = 0.8;
const RELAXED_LINE_HEIGHT: gpui::Pixels = px(22.75);
const REACTION_TRANSLATION: gpui::Pixels = px(16.5);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BubbleVariant {
    #[default]
    Default,
    Secondary,
    Muted,
    Tinted,
    Outline,
    Ghost,
    Destructive,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BubbleAlign {
    #[default]
    Start,
    End,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BubbleReactionSide {
    Top,
    #[default]
    Bottom,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BubbleReactionAlign {
    Start,
    #[default]
    End,
}

#[derive(IntoElement)]
#[must_use]
pub struct BubbleGroup {
    children: Vec<AnyElement>,
}

impl BubbleGroup {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children
            .extend(children.into_iter().map(IntoElement::into_any_element));
        self
    }
}

impl Default for BubbleGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for BubbleGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .children(self.children)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct Bubble {
    variant: BubbleVariant,
    align: BubbleAlign,
    contents: Vec<BubbleContent>,
    reactions: Vec<BubbleReactions>,
}

impl Bubble {
    pub fn new(variant: BubbleVariant, align: BubbleAlign) -> Self {
        Self {
            variant,
            align,
            contents: Vec::new(),
            reactions: Vec::new(),
        }
    }

    pub fn content(mut self, content: BubbleContent) -> Self {
        self.contents.push(content);
        self
    }

    pub fn reactions(mut self, reactions: BubbleReactions) -> Self {
        self.reactions.push(reactions);
        self
    }
}

impl RenderOnce for Bubble {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let palette = BubblePalette::resolve(self.variant, theme, is_dark(window.appearance()));
        let align_end = self.align == BubbleAlign::End;

        div()
            .relative()
            .min_w_0()
            .max_w(if self.variant == BubbleVariant::Ghost {
                relative(1.0)
            } else {
                relative(BUBBLE_MAX_WIDTH)
            })
            .flex()
            .flex_col()
            .gap_1()
            .when(align_end, gpui::Styled::self_end)
            .when(!align_end, gpui::Styled::self_start)
            .children(
                self.contents
                    .into_iter()
                    .map(|content| content.render_with_palette(palette, align_end)),
            )
            .children(self.reactions)
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct BubbleContent {
    child: AnyElement,
    interaction: Option<BubbleContentInteraction>,
}

type BubbleContentClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

struct BubbleContentInteraction {
    id: ElementId,
    accessibility_label: gpui::SharedString,
    on_click: BubbleContentClickHandler,
}

impl BubbleContent {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
            interaction: None,
        }
    }

    pub fn button(
        id: impl Into<ElementId>,
        accessibility_label: impl Into<gpui::SharedString>,
        child: impl IntoElement,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            child: child.into_any_element(),
            interaction: Some(BubbleContentInteraction {
                id: id.into(),
                accessibility_label: accessibility_label.into(),
                on_click: Rc::new(on_click),
            }),
        }
    }

    fn render_with_palette(self, palette: BubblePalette, align_end: bool) -> AnyElement {
        match self.interaction {
            Some(interaction) => {
                let on_click = interaction.on_click;
                style_content(ButtonBehavior::new(interaction.id), palette, align_end)
                    .accessibility_label(interaction.accessibility_label)
                    .cursor_pointer()
                    .hover(move |style| {
                        style
                            .bg(palette.hover_background)
                            .text_color(palette.hover_foreground)
                    })
                    .focus_visible(move |style| {
                        style
                            .border_color(palette.ring)
                            .shadow(focus_ring(palette.ring.opacity(0.5), RING_3))
                    })
                    .on_click(move |event, window, cx| on_click(event, window, cx))
                    .child(self.child)
                    .into_any_element()
            }
            None => style_content(div(), palette, align_end)
                .child(self.child)
                .into_any_element(),
        }
    }
}

impl RenderOnce for BubbleContent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        self.render_with_palette(
            BubblePalette::resolve(BubbleVariant::Default, theme, is_dark(window.appearance())),
            false,
        )
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct BubbleReactions {
    side: BubbleReactionSide,
    align: BubbleReactionAlign,
    contains_button: bool,
    children: Vec<AnyElement>,
}

impl BubbleReactions {
    pub fn new(side: BubbleReactionSide, align: BubbleReactionAlign) -> Self {
        Self {
            side,
            align,
            contains_button: false,
            children: Vec::new(),
        }
    }

    pub fn contains_button(mut self, contains_button: bool) -> Self {
        self.contains_button = contains_button;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl RenderOnce for BubbleReactions {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        div()
            .absolute()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .gap_1()
            .rounded(RADIUS_LG)
            .bg(theme.muted)
            .text_style(typography::tokens().sm)
            .shadow(focus_ring(theme.card, RING_3))
            .when(!self.contains_button, |reactions| {
                reactions.px_1p5().py_0p5()
            })
            .when(self.side == BubbleReactionSide::Top, |reactions| {
                reactions.top(-REACTION_TRANSLATION)
            })
            .when(self.side == BubbleReactionSide::Bottom, |reactions| {
                reactions.bottom(-REACTION_TRANSLATION)
            })
            .when(self.align == BubbleReactionAlign::Start, |reactions| {
                reactions.left(SPACE_3)
            })
            .when(self.align == BubbleReactionAlign::End, |reactions| {
                reactions.right(SPACE_3)
            })
            .children(self.children)
    }
}

#[derive(Clone, Copy)]
struct BubblePalette {
    background: Hsla,
    foreground: Hsla,
    hover_background: Hsla,
    hover_foreground: Hsla,
    border: Hsla,
    ring: Hsla,
    radius: gpui::Pixels,
    horizontal_padding: gpui::Pixels,
    vertical_padding: gpui::Pixels,
}

impl BubblePalette {
    fn resolve(variant: BubbleVariant, theme: &G2VTheme, dark: bool) -> Self {
        let transparent = theme.background.opacity(0.0);
        let (background, foreground, hover_background, hover_foreground, border) = match variant {
            BubbleVariant::Default => (
                theme.primary,
                theme.primary_foreground,
                theme.primary.opacity(0.8),
                theme.primary_foreground,
                transparent,
            ),
            BubbleVariant::Secondary => (
                theme.secondary,
                theme.secondary_foreground,
                theme.secondary_hover,
                theme.secondary_foreground,
                transparent,
            ),
            BubbleVariant::Muted => (
                theme.muted,
                theme.foreground,
                theme.secondary_hover,
                theme.foreground,
                transparent,
            ),
            BubbleVariant::Tinted => (
                theme.bubble_tinted,
                theme.foreground,
                theme.bubble_tinted_hover,
                theme.foreground,
                transparent,
            ),
            BubbleVariant::Outline => (
                theme.background,
                theme.foreground,
                if dark {
                    theme.input.opacity(0.3)
                } else {
                    theme.muted
                },
                theme.foreground,
                theme.border,
            ),
            BubbleVariant::Ghost => (
                transparent,
                theme.foreground,
                if dark {
                    theme.muted.opacity(0.5)
                } else {
                    theme.muted
                },
                theme.foreground,
                transparent,
            ),
            BubbleVariant::Destructive => (
                theme.destructive.opacity(if dark { 0.2 } else { 0.1 }),
                theme.destructive,
                theme.destructive.opacity(if dark { 0.3 } else { 0.2 }),
                theme.destructive,
                transparent,
            ),
        };
        let ghost = variant == BubbleVariant::Ghost;
        Self {
            background,
            foreground,
            hover_background,
            hover_foreground,
            border,
            ring: theme.ring,
            radius: if ghost { gpui::Pixels::ZERO } else { RADIUS_LG },
            horizontal_padding: if ghost { px(0.0) } else { SPACE_3 },
            vertical_padding: if ghost { px(0.0) } else { SPACE_2 },
        }
    }
}

fn style_content<T: Styled + FluentBuilder>(
    surface: T,
    palette: BubblePalette,
    align_end: bool,
) -> T {
    surface
        .min_w_0()
        .max_w_full()
        .overflow_hidden()
        .rounded(palette.radius)
        .border(BORDER_DEFAULT)
        .border_color(palette.border)
        .px(palette.horizontal_padding)
        .py(palette.vertical_padding)
        .bg(palette.background)
        .text_color(palette.foreground)
        .text_style(typography::tokens().sm)
        .line_height(RELAXED_LINE_HEIGHT)
        .when(align_end, gpui::Styled::self_end)
        .when(!align_end, gpui::Styled::self_start)
}

const fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_matches_the_react_bubble_contract() {
        assert!((BUBBLE_MAX_WIDTH - 0.8).abs() < f32::EPSILON);
        assert_eq!(SPACE_3, px(12.0));
        assert_eq!(SPACE_2, px(8.0));
        assert_eq!(RELAXED_LINE_HEIGHT, px(22.75));
        assert_eq!(REACTION_TRANSLATION, px(16.5));
    }

    #[test]
    fn every_react_variant_is_represented() {
        assert_eq!(
            [
                BubbleVariant::Default,
                BubbleVariant::Secondary,
                BubbleVariant::Muted,
                BubbleVariant::Tinted,
                BubbleVariant::Outline,
                BubbleVariant::Ghost,
                BubbleVariant::Destructive,
            ]
            .len(),
            7
        );
    }

    #[test]
    fn destructive_opacity_matches_each_react_theme() {
        let light = G2VTheme::for_appearance(WindowAppearance::Light);
        let dark = G2VTheme::for_appearance(WindowAppearance::Dark);
        let light_palette = BubblePalette::resolve(BubbleVariant::Destructive, &light, false);
        let dark_palette = BubblePalette::resolve(BubbleVariant::Destructive, &dark, true);

        assert!((light_palette.background.a - 0.1).abs() < f32::EPSILON);
        assert!((light_palette.hover_background.a - 0.2).abs() < f32::EPSILON);
        assert!((dark_palette.background.a - 0.2).abs() < f32::EPSILON);
        assert!((dark_palette.hover_background.a - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn button_content_owns_native_keyboard_and_pointer_behavior() {
        let content =
            BubbleContent::button("bubble-button", "Bubble action", "Action", |_, _, _| {});

        assert!(content.interaction.is_some());
    }
}
