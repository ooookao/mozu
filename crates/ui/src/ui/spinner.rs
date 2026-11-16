use super::icon::{Icon, IconName};
use crate::theme::MotionSettings;
use gpui::prelude::*;
use gpui::{
    AnimationExt, App, IntoElement, Refineable, RenderOnce, SharedString, StyleRefinement, Styled,
    Window, px,
};

#[derive(IntoElement)]
pub struct Spinner {
    id: gpui::ElementId,
    label: SharedString,
    style: StyleRefinement,
}
impl Spinner {
    pub fn new(id: impl Into<gpui::ElementId>) -> Self {
        Self {
            id: id.into(),
            label: "Loading".into(),
            style: StyleRefinement::default(),
        }
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
}
impl Styled for Spinner {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for Spinner {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut root = gpui::div()
            .id(self.id)
            .flex()
            .flex_none()
            .size(px(16.))
            .role(gpui::Role::Status)
            .aria_label(self.label)
            .child(
                Icon::new(IconName::LoaderCircle)
                    .size_full()
                    .with_animation(
                        "spin",
                        MotionSettings::current(cx)
                            .animations
                            .spinner
                            .animation()
                            .repeat(),
                        |icon, delta| {
                            // The SVG rotates about its center while keeping its layout footprint.
                            icon.rotate(gpui::radians(delta * std::f32::consts::TAU))
                        },
                    ),
            );
        root.style().refine(&self.style);
        root
    }
}
