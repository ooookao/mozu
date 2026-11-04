//! G2V's canonical native separator primitive.

use gpui::{App, InteractiveElement as _, IntoElement, RenderOnce, Styled as _, Window, div, px};

use crate::theme::{border::BORDER_DEFAULT, colors::G2VTheme};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SeparatorOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(IntoElement, Default)]
#[must_use]
pub struct Separator {
    orientation: SeparatorOrientation,
    presentation: SeparatorPresentation,
}

#[derive(Default)]
enum SeparatorPresentation {
    #[default]
    Standard,
    ButtonGroup,
}

impl Separator {
    pub(crate) fn grouped(orientation: SeparatorOrientation) -> Self {
        Self {
            orientation,
            presentation: SeparatorPresentation::ButtonGroup,
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn orientation(mut self, orientation: SeparatorOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn horizontal() -> Self {
        Self::new()
    }

    pub fn vertical() -> Self {
        Self::new().orientation(SeparatorOrientation::Vertical)
    }
}

impl RenderOnce for Separator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        if matches!(self.presentation, SeparatorPresentation::ButtonGroup) {
            let separator = div()
                .debug_selector(|| "button-group-separator".into())
                .flex_none()
                .self_stretch()
                .bg(G2VTheme::global(cx).input);
            return match self.orientation {
                SeparatorOrientation::Horizontal => separator.h(BORDER_DEFAULT).mx(px(1.0)),
                SeparatorOrientation::Vertical => separator.w(BORDER_DEFAULT).my(px(1.0)),
            }
            .into_any_element();
        }
        let color = G2VTheme::global(cx).border;

        match self.orientation {
            SeparatorOrientation::Horizontal => div()
                .w_full()
                .h(BORDER_DEFAULT)
                .bg(color)
                .into_any_element(),
            SeparatorOrientation::Vertical => div()
                .w(BORDER_DEFAULT)
                .h_full()
                .bg(color)
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_is_the_default_orientation() {
        assert_eq!(
            Separator::new().orientation,
            SeparatorOrientation::Horizontal
        );
    }

    #[test]
    fn vertical_constructor_sets_vertical_orientation() {
        assert_eq!(
            Separator::vertical().orientation,
            SeparatorOrientation::Vertical
        );
    }
}
