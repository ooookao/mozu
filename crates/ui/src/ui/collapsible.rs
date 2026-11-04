use gpui::{
    AnyElement, App, IntoElement, ParentElement, Pixels, RenderOnce, SharedString, Styled as _,
    Window, div,
};
use gpui_base::{ElementExt as _, Transition, transition};

use crate::theme::motion::{MotionTokens, cubic_bezier_with_overshoot};

#[derive(Clone, Copy, Debug, PartialEq)]
enum CollapsibleContentLayout {
    Hidden,
    Measure,
    Natural,
    Transition(Pixels),
}

fn collapsible_content_layout(
    open: bool,
    progress: f32,
    measured_height: Pixels,
) -> CollapsibleContentLayout {
    if open && progress >= 1.0 {
        return CollapsibleContentLayout::Natural;
    }
    if open && measured_height == Pixels::ZERO {
        return CollapsibleContentLayout::Measure;
    }
    if !open && progress <= 0.0 {
        return CollapsibleContentLayout::Hidden;
    }
    CollapsibleContentLayout::Transition(measured_height * progress)
}

#[derive(Default)]
struct CollapsibleContentState {
    measured_height: Pixels,
}

#[derive(IntoElement)]
#[must_use]
pub struct CollapsibleContent {
    id: SharedString,
    open: bool,
    children: Vec<AnyElement>,
}

impl CollapsibleContent {
    pub fn new(id: impl Into<SharedString>, open: bool) -> Self {
        Self {
            id: id.into(),
            open,
            children: Vec::new(),
        }
    }
}

impl ParentElement for CollapsibleContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for CollapsibleContent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| {
            CollapsibleContentState::default()
        });
        let measured_height = state.read(cx).measured_height;
        let progress = transition(
            (self.id.clone(), "content-progress"),
            if self.open { 1.0 } else { 0.0 },
            Transition::new(MotionTokens::global(cx).fast)
                .ease(cubic_bezier_with_overshoot(0.0, 0.0, 0.2, 1.0)),
            window,
            cx,
        );
        let layout = collapsible_content_layout(self.open, progress, measured_height);
        let measured_state = state.clone();
        let content = div()
            .w_full()
            .children(self.children)
            .on_prepaint(move |bounds, _, cx| {
                if matches!(
                    layout,
                    CollapsibleContentLayout::Measure | CollapsibleContentLayout::Natural
                ) && bounds.size.height > Pixels::ZERO
                {
                    measured_state.update(cx, |state, cx| {
                        if state.measured_height != bounds.size.height {
                            state.measured_height = bounds.size.height;
                            cx.notify();
                        }
                    });
                }
            });
        match layout {
            CollapsibleContentLayout::Hidden => div().into_any_element(),
            CollapsibleContentLayout::Measure => div()
                .relative()
                .h_0()
                .w_full()
                .overflow_hidden()
                .child(content.absolute().left_0().right_0().top_0().opacity(0.0))
                .into_any_element(),
            CollapsibleContentLayout::Natural => content.into_any_element(),
            CollapsibleContentLayout::Transition(height) => content
                .h(height)
                .overflow_hidden()
                .opacity(progress)
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CollapsibleContentLayout, collapsible_content_layout};

    #[test]
    fn opening_content_is_measured_before_height_transition() {
        assert_eq!(
            collapsible_content_layout(true, 0.0, gpui::Pixels::ZERO),
            CollapsibleContentLayout::Measure,
        );
        assert_eq!(
            collapsible_content_layout(true, 0.5, gpui::px(80.0)),
            CollapsibleContentLayout::Transition(gpui::px(40.0)),
        );
        assert_eq!(
            collapsible_content_layout(true, 1.0, gpui::px(80.0)),
            CollapsibleContentLayout::Natural,
        );
    }

    #[test]
    fn already_open_content_needs_no_zero_height_measurement_frame() {
        assert_eq!(
            collapsible_content_layout(true, 1.0, gpui::Pixels::ZERO),
            CollapsibleContentLayout::Natural,
        );
    }

    #[test]
    fn closing_content_stays_mounted_until_its_exit_finishes() {
        assert_eq!(
            collapsible_content_layout(false, 0.5, gpui::px(80.0)),
            CollapsibleContentLayout::Transition(gpui::px(40.0)),
        );
        assert_eq!(
            collapsible_content_layout(false, 0.0, gpui::px(80.0)),
            CollapsibleContentLayout::Hidden,
        );
    }
}
