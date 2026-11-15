use crate::ui::scroll_area::scrollbar;
use crate::ui::utils::presentation::ComponentTheme;
use crate::ui::utils::table_style::{scheme, table_parts};
use gpui::{App, ElementId, Window};
use kinu_ui::{DataTable, TableRow};

/// A virtualized native table with the same editable presentation as a regular table.
pub fn data_table_with_tokens(
    id: impl Into<ElementId>,
    row_count: usize,
    row_factory: impl Fn(usize, &mut Window, &mut App) -> TableRow + 'static,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> DataTable {
    let id = id.into();
    let bar = scrollbar(gpui::Axis::Vertical, tokens.clone(), scheme(window, cx));
    let (appearance, presentation) = table_parts(id.clone(), tokens, window, cx);
    DataTable::new(id, row_count, row_factory)
        .scrollbar(bar)
        .appearance(appearance)
        .presentation(presentation)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn data_table(
    id: impl Into<ElementId>,
    row_count: usize,
    row_factory: impl Fn(usize, &mut Window, &mut App) -> TableRow + 'static,
    window: &Window,
    cx: &App,
) -> DataTable {
    data_table_with_tokens(
        id,
        row_count,
        row_factory,
        ComponentTheme::current(cx),
        window,
        cx,
    )
}
