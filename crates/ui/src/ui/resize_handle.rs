use std::rc::Rc;

use gpui::{
    App, DragMoveEvent, Empty, IntoElement, MouseButton, Point, Render, RenderOnce, SharedString,
    Window, div, linear_color_stop, linear_gradient, prelude::*, px,
};

use crate::theme::border::BORDER_DEFAULT;

pub type ResizeStartHandler = Rc<dyn Fn(&mut Window, &mut App)>;
pub type ResizeMoveHandler = Rc<dyn Fn(Point<gpui::Pixels>, &mut Window, &mut App)>;
pub type ResizeEndHandler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(Clone)]
struct VerticalResizeDrag {
    owner: SharedString,
}

impl VerticalResizeDrag {
    fn belongs_to(&self, owner: &SharedString) -> bool {
        &self.owner == owner
    }
}

struct ResizePreview;

impl Render for ResizePreview {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        Empty
    }
}

/// Shared left-edge drag handle for right-aligned resizable surfaces.
#[derive(IntoElement)]
pub struct VerticalResizeHandle {
    id: SharedString,
    ring: gpui::Hsla,
    on_start: ResizeStartHandler,
    on_move: ResizeMoveHandler,
    on_end: ResizeEndHandler,
}

impl VerticalResizeHandle {
    pub fn new(
        id: impl Into<SharedString>,
        ring: gpui::Hsla,
        on_start: ResizeStartHandler,
        on_move: ResizeMoveHandler,
        on_end: ResizeEndHandler,
    ) -> Self {
        Self {
            id: id.into(),
            ring,
            on_start,
            on_move,
            on_end,
        }
    }
}

impl RenderOnce for VerticalResizeHandle {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let group = self.id.clone();
        let drag_owner = self.id.clone();
        let move_owner = self.id.clone();
        let on_start = self.on_start;
        let on_move = self.on_move;
        let on_end = self.on_end;
        let on_end_out = on_end.clone();
        let ring = self.ring;

        div()
            .id(self.id)
            .group(group.clone())
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(-4.0))
            .w(px(8.0))
            .occlude()
            .cursor_col_resize()
            .on_drag(
                VerticalResizeDrag { owner: drag_owner },
                move |_, _, window, cx| {
                    on_start(window, cx);
                    cx.new(|_| ResizePreview)
                },
            )
            .on_drag_move(
                move |event: &DragMoveEvent<VerticalResizeDrag>, window, cx| {
                    if !event.drag(cx).belongs_to(&move_owner) {
                        return;
                    }
                    on_move(event.event.position, window, cx);
                },
            )
            .on_mouse_up(MouseButton::Left, move |_, window, cx| {
                on_end(window, cx);
            })
            .on_mouse_up_out(MouseButton::Left, move |_, window, cx| {
                on_end_out(window, cx);
            })
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(3.5))
                    .w(BORDER_DEFAULT)
                    .flex()
                    .flex_col()
                    .opacity(0.0)
                    .group_hover(group, |line| line.opacity(1.0))
                    .child(div().flex_1().bg(linear_gradient(
                        0.0,
                        linear_color_stop(ring, 0.0),
                        linear_color_stop(ring.opacity(0.0), 1.0),
                    )))
                    .child(div().flex_1().bg(linear_gradient(
                        0.0,
                        linear_color_stop(ring.opacity(0.0), 0.0),
                        linear_color_stop(ring, 1.0),
                    ))),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        Context, IntoElement, MouseButton, ParentElement as _, Render, Styled as _, Window, div,
        point, px,
    };

    use super::{VerticalResizeDrag, VerticalResizeHandle};

    struct ResizeHarness {
        started: Rc<Cell<bool>>,
    }

    impl Render for ResizeHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let started = self.started.clone();
            div().relative().size_full().child(
                div()
                    .absolute()
                    .left(px(100.0))
                    .top_0()
                    .bottom_0()
                    .w(px(0.0))
                    .child(
                        gpui::deferred(VerticalResizeHandle::new(
                            "test-resize",
                            gpui::transparent_black(),
                            Rc::new(move |_, _| started.set(true)),
                            Rc::new(|_, _, _| {}),
                            Rc::new(|_, _| {}),
                        ))
                        .with_priority(1),
                    ),
            )
        }
    }

    #[test]
    fn nested_resize_handles_only_accept_their_own_drag() {
        let file_tree = gpui::SharedString::from("file-tree");
        let right_panel = gpui::SharedString::from("right-panel");
        let drag = VerticalResizeDrag {
            owner: file_tree.clone(),
        };

        assert!(drag.belongs_to(&file_tree));
        assert!(!drag.belongs_to(&right_panel));
    }

    #[gpui::test]
    fn deferred_handle_starts_drag_from_the_outer_half(cx: &mut gpui::TestAppContext) {
        let started = Rc::new(Cell::new(false));
        let (_, window) = cx.add_window_view({
            let started = started.clone();
            move |_, _| ResizeHarness { started }
        });
        window.update(|window, cx| window.draw(cx).clear(cx));
        window.simulate_mouse_move(point(px(97.0), px(20.0)), None, gpui::Modifiers::none());
        window.simulate_mouse_down(
            point(px(97.0), px(20.0)),
            MouseButton::Left,
            gpui::Modifiers::none(),
        );
        window.simulate_mouse_move(
            point(px(80.0), px(20.0)),
            Some(MouseButton::Left),
            gpui::Modifiers::none(),
        );
        assert!(started.get());
    }
}
