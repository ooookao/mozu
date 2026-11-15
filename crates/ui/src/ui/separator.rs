use crate::theme::Theme;
use gpui::{
    App, ElementId, IntoElement, Orientation, Refineable, RenderOnce, StyleRefinement, Styled,
    Window, px,
};
use kinu_ui::{Separator as SeparatorPrimitive, SeparatorKind};

/// A themed separator whose presentation remains editable by its owner.
#[derive(IntoElement)]
pub struct Separator {
    id: ElementId,
    orientation: Orientation,
    kind: SeparatorKind,
    style: StyleRefinement,
}
impl Separator {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            orientation: Orientation::Horizontal,
            kind: SeparatorKind::Semantic,
            style: StyleRefinement::default(),
        }
    }
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }
    pub fn kind(mut self, kind: SeparatorKind) -> Self {
        self.kind = kind;
        self
    }
    fn into_primitive(self, window: &Window, cx: &App) -> SeparatorPrimitive {
        let mut primitive = SeparatorPrimitive::new(self.id, self.orientation, self.kind)
            .flex_shrink_0()
            .bg(Theme::current(window, cx).border);
        primitive = match self.orientation {
            Orientation::Horizontal => primitive.h(px(1.)).w_full(),
            Orientation::Vertical => primitive.w(px(1.)).self_stretch(),
        };
        primitive.style().refine(&self.style);
        primitive
    }
}
impl Styled for Separator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for Separator {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.into_primitive(window, cx).render(window, cx)
    }
}
