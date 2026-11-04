//! Single-line text that fades only when its shaped width exceeds its container.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement as _, Pixels,
    RenderOnce, SharedString, Styled as _, Window, div, font, linear_color_stop, linear_gradient,
    px,
};
use gpui_base::{ElementExt as _, TextStyleToken};

use crate::theme::typography;
use crate::ui::{TextStyleExt as _, TextStyleTokenExt as _};

const FADE_WIDTH: Pixels = px(24.0);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct FadeTruncateMeasurement {
    available_width: Pixels,
}

fn is_overflowing(content_width: Pixels, available_width: Pixels) -> bool {
    available_width > px(0.0) && content_width > available_width
}

#[derive(IntoElement)]
#[must_use]
pub struct FadeTruncate {
    id: ElementId,
    text: SharedString,
    fade_color: Option<Hsla>,
    fade_width: Pixels,
    font_family: SharedString,
    text_style: TextStyleToken,
}

impl FadeTruncate {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>, fade_color: Hsla) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            fade_color: Some(fade_color),
            fade_width: FADE_WIDTH,
            font_family: "Inter".into(),
            text_style: typography::TEXT_13,
        }
    }

    pub fn fade_width(mut self, fade_width: Pixels) -> Self {
        self.fade_width = fade_width;
        self
    }

    pub fn font_family(mut self, font_family: impl Into<SharedString>) -> Self {
        self.font_family = font_family.into();
        self
    }

    pub fn text_style(mut self, text_style: TextStyleToken) -> Self {
        self.text_style = text_style;
        self
    }

    pub fn on_transparent_surface(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            fade_color: None,
            fade_width: FADE_WIDTH,
            font_family: "Inter".into(),
            text_style: typography::TEXT_13,
        }
    }
}

impl RenderOnce for FadeTruncate {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let measurement = window.use_keyed_state((self.id.clone(), "measurement"), cx, |_, _| {
            FadeTruncateMeasurement::default()
        });
        let available_width = measurement.read(cx).available_width;
        let mut text_font = font(self.font_family);
        text_font.weight = self.text_style.weight;
        let text_style = window.text_style();
        let mut run = text_style.to_run(self.text.len());
        run.font = text_font;
        let shaped = window.text_system().layout_line(
            self.text.as_ref(),
            self.text_style.font_size_pixels(window.rem_size()),
            std::slice::from_ref(&run),
            None,
        );
        let content_width = shaped.width;
        let overflowing = is_overflowing(content_width, available_width);
        let measured = measurement.clone();
        let fade_color = self.fade_color;
        let fade_width = self.fade_width;
        let text = if overflowing && fade_color.is_none() {
            let runs = self
                .text
                .char_indices()
                .map(|(index, character)| {
                    let mut glyph_run = run.clone();
                    glyph_run.len = character.len_utf8();
                    let opacity = ((available_width - shaped.x_for_index(index)) / fade_width)
                        .clamp(0.0, 1.0);
                    glyph_run.color.a *= opacity;
                    glyph_run
                })
                .collect();
            gpui::StyledText::new(self.text)
                .with_runs(runs)
                .into_any_element()
        } else {
            self.text.into_any_element()
        };

        div()
            .id(self.id)
            .relative()
            .min_w_0()
            .w_full()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_style(self.text_style)
            .on_prepaint(move |bounds, _, cx| {
                if measured.read(cx).available_width != bounds.size.width {
                    measured.update(cx, |measurement, cx| {
                        measurement.available_width = bounds.size.width;
                        cx.notify();
                    });
                }
            })
            .child(text)
            .when_some(fade_color.filter(|_| overflowing), |text, fade_color| {
                text.child(
                    div()
                        .absolute()
                        .top_0()
                        .right_0()
                        .bottom_0()
                        .w(fade_width)
                        .bg(linear_gradient(
                            90.0,
                            linear_color_stop(fade_color.opacity(0.0), 0.0),
                            linear_color_stop(fade_color, 1.0),
                        )),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_is_visible_only_when_content_exceeds_available_width() {
        assert!(!is_overflowing(px(80.0), px(0.0)));
        assert!(!is_overflowing(px(80.0), px(80.0)));
        assert!(!is_overflowing(px(79.0), px(80.0)));
        assert!(is_overflowing(px(81.0), px(80.0)));
    }
}
