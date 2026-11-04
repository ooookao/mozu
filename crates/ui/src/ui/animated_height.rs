use gpui::{
    AnimationExt as _, AnyElement, ElementId, IntoElement, ParentElement, Pixels, Rems, RenderOnce,
    SpringAnimation, SpringConfig, Styled as _, Window, div, px,
};

const HEIGHT_SPRING: SpringConfig = SpringConfig::new(320.0, 32.0, 1.0);
const HEIGHT_EPSILON: f32 = 0.25;

/// Clips content to a height that moves smoothly between bounded targets.
#[derive(IntoElement)]
#[must_use]
pub struct AnimatedHeight {
    id: ElementId,
    target: Rems,
    minimum: Rems,
    maximum: Rems,
    children: Vec<AnyElement>,
}

impl AnimatedHeight {
    pub fn new(id: impl Into<ElementId>, target: Rems, minimum: Rems, maximum: Rems) -> Self {
        Self {
            id: id.into(),
            target,
            minimum,
            maximum,
            children: Vec::new(),
        }
    }
}

impl ParentElement for AnimatedHeight {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for AnimatedHeight {
    fn render(self, window: &mut Window, _: &mut gpui::App) -> impl IntoElement {
        let rem_size = window.rem_size();
        let minimum = self.minimum.to_pixels(rem_size);
        let maximum = self.maximum.to_pixels(rem_size);
        let target = bounded_height(self.target.to_pixels(rem_size), minimum, maximum);
        div()
            .w_full()
            .overflow_hidden()
            .with_spring(
                self.id,
                SpringAnimation::new(HEIGHT_SPRING)
                    .to(target)
                    .with_epsilon(HEIGHT_EPSILON),
                move |surface, height| surface.h(bounded_height(height, minimum, maximum)),
            )
            .children(self.children)
    }
}

fn bounded_height(target: Pixels, minimum: Pixels, maximum: Pixels) -> Pixels {
    px(f32::from(target).clamp(f32::from(minimum), f32::from(maximum)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_height_is_bounded_without_changing_values_inside_the_range() {
        assert_eq!(bounded_height(px(20.0), px(30.0), px(60.0)), px(30.0));
        assert_eq!(bounded_height(px(45.0), px(30.0), px(60.0)), px(45.0));
        assert_eq!(bounded_height(px(80.0), px(30.0), px(60.0)), px(60.0));
    }
}
