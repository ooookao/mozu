//! Visible message row composition and row-render instrumentation.

use gpui::{AnyElement, Entity, IntoElement as _, ListSizingBehavior, Styled as _, list};

use super::MessageScrollerModel;
#[cfg(debug_assertions)]
use super::{Instant, Ordering, ROW_BUILD_NANOS, ROW_RENDERS};

pub(super) fn render(model: &Entity<MessageScrollerModel>, cx: &gpui::App) -> AnyElement {
    let state = model.read(cx);
    let item_count = state.item_count();
    let (top_content_inset, bottom_content_inset) = state.content_insets();
    let list_state = state.list();
    let weak_model = model.downgrade();
    list(list_state, move |index, window, cx| {
        #[cfg(debug_assertions)]
        ROW_RENDERS.fetch_add(1, Ordering::Relaxed);
        #[cfg(debug_assertions)]
        let row_started_at = Instant::now();
        let model = weak_model
            .upgrade()
            .expect("message scroller model must outlive its list");
        let item = model.read(cx).item(index);
        let rendered = item.render(
            index == 0,
            index + 1 == item_count,
            top_content_inset,
            bottom_content_inset,
            window,
            cx,
        );
        #[cfg(debug_assertions)]
        ROW_BUILD_NANOS.fetch_add(
            u64::try_from(row_started_at.elapsed().as_nanos()).unwrap_or(u64::MAX),
            Ordering::Relaxed,
        );
        rendered
    })
    .size_full()
    .with_sizing_behavior(ListSizingBehavior::Auto)
    .into_any_element()
}
