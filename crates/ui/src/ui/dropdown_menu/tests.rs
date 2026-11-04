use super::*;
use gpui::px;
use std::time::{Duration, Instant};

use super::{
    content::update_typeahead_query,
    keyboard::{step_active, typeahead_match},
    presentation::{density_icon_size, density_text_style},
    state::DropdownMenuState,
};

struct WidthHarness;
struct DetailWidthHarness;
struct MessageWidthHarness;

impl gpui::Render for MessageWidthHarness {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        let submenu = DropdownMenuSub::new("diagnostic", "Committed").message(
            "list Git Review commits failed request_id=a1e93d2e-7459-437a-b170-fdc8ff0647a6: list Git Review commits limit=100: Git Review committed source requires a default or upstream branch",
        );
        let mut menu =
            DropdownMenu::new("message-menu", Button::new("message-trigger").label("Open"))
                .controlled(true, Rc::new(|_, _, _| {}), Rc::new(|_| {}));
        menu.entries = submenu.entries;
        menu
    }
}

#[gpui::test]
fn diagnostic_message_wraps_instead_of_expanding_the_menu(cx: &mut gpui::TestAppContext) {
    init_test_ui(cx);
    let (_, window) = cx.add_window_view(|_, _| MessageWidthHarness);
    for _ in 0..3 {
        window.update(|window, cx| window.draw(cx).clear(cx));
    }
    let message = window.debug_bounds("dropdown-menu-message").unwrap();
    assert!(
        message.size.width <= px(320.0),
        "message width: {:?}",
        message.size.width
    );
    assert!(
        message.size.height > px(26.0),
        "diagnostic must wrap onto multiple lines"
    );
}

fn init_test_ui(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        gpui_base::init(cx);
        crate::ui::init(cx);
        crate::theme::typography::install(cx);
        crate::theme::install(gpui::WindowAppearance::Light, cx);
    });
}

#[test]
fn compact_density_uses_the_agreed_tab_menu_metrics() {
    let menu = DropdownMenu::new(
        "compact-contract-menu",
        Button::new("compact-contract-trigger").label("Open"),
    )
    .density(DropdownMenuDensity::Compact)
    .minimum_width(rems_from_design_pixels(160.0));

    assert_eq!(menu.density, DropdownMenuDensity::Compact);
    assert_eq!(menu.minimum_width.to_pixels(px(16.0)), px(160.0));
    assert_eq!(menu.minimum_width.to_pixels(px(18.0)), px(180.0));
    assert_eq!(density_text_style(menu.density).size, px(13.0));
    assert_eq!(
        density_icon_size(menu.density).to_pixels(px(16.0)),
        px(14.0)
    );
}

#[test]
fn positioning_defaults_follow_the_window_rem_size() {
    let root = DropdownMenu::new(
        "positioning-root",
        Button::new("positioning-trigger").label("Open"),
    );
    assert_eq!(DropdownMenuDensity::default(), DropdownMenuDensity::Compact);
    assert_eq!(root.density, DropdownMenuDensity::Compact);
    assert_eq!(root.side_offset.to_pixels(px(16.0)), px(4.0));
    assert_eq!(root.side_offset.to_pixels(px(18.0)), px(4.5));
    assert_eq!(root.align_offset.to_pixels(px(18.0)), px(0.0));
    assert_eq!(root.minimum_width.to_pixels(px(18.0)), px(144.0));

    let submenu = DropdownMenu::custom("positioning-submenu", |_, _, _| {
        gpui::div().into_any_element()
    });
    assert_eq!(submenu.density, DropdownMenuDensity::Compact);
    assert_eq!(submenu.side_offset.to_pixels(px(18.0)), px(0.0));
    assert_eq!(submenu.align_offset.to_pixels(px(18.0)), px(-3.375));
}

impl gpui::Render for WidthHarness {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        DropdownMenu::new(
            "width-contract-menu",
            Button::new("width-contract-trigger").label("Open"),
        )
        .item(DropdownMenuItem::new(
            "long-item",
            "This menu item determines the intrinsic popup width",
        ))
        .item(DropdownMenuItem::new("short-item", "Short"))
        .separator()
        .controlled(true, Rc::new(|_, _, _| {}), Rc::new(|_| {}))
    }
}

impl gpui::Render for DetailWidthHarness {
    fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
        DropdownMenu::new(
            "detail-width-menu",
            Button::new("detail-width-trigger").label("Open"),
        )
        .item(
            DropdownMenuItem::new("future-harness", "Hermes Agent")
                .icon(IconName::ProviderHermesAgent)
                .detail("Coming soon")
                .disabled(true),
        )
        .controlled(true, Rc::new(|_, _, _| {}), Rc::new(|_| {}))
    }
}

#[test]
fn keyboard_navigation_skips_non_interactive_entries_and_wraps() {
    let navigable = ["first".into(), "third".into(), "fourth".into()];
    assert_eq!(step_active(None, &navigable, 1), Some("first".into()));
    assert_eq!(step_active(None, &navigable, -1), Some("fourth".into()));
    assert_eq!(
        step_active(Some(&navigable[2]), &navigable, 1),
        Some("first".into())
    );
    assert_eq!(
        step_active(Some(&navigable[0]), &navigable, -1),
        Some("fourth".into())
    );
}

#[test]
fn custom_rows_are_presentational() {
    let custom = DropdownMenuEntry::CustomRow(Rc::new(|_| gpui::div().into_any_element()));

    assert!(!custom.navigable());
}

#[test]
fn submenu_preserves_its_trailing_detail() {
    let menu = DropdownMenu::new(
        "settings-menu",
        Button::new("settings-trigger").label("Settings"),
    )
    .submenu(DropdownMenuSub::new("model", "Model").detail("5.6 Sol"));

    let DropdownMenuEntry::Submenu { detail, .. } = &menu.entries[0] else {
        panic!("settings entry must remain a submenu");
    };
    assert_eq!(detail.as_ref().map(SharedString::as_ref), Some("5.6 Sol"));
}

#[test]
fn item_preserves_its_trailing_detail() {
    let item = DropdownMenuItem::new("future-option", "Future option").detail("Coming soon");

    assert_eq!(
        item.detail.as_ref().map(SharedString::as_ref),
        Some("Coming soon")
    );
}

#[test]
fn typeahead_starts_after_the_active_item_and_wraps() {
    let entries = vec![
        DropdownMenuEntry::Item(DropdownMenuItem::new("archive", "Archive")),
        DropdownMenuEntry::Item(DropdownMenuItem::new("duplicate", "Duplicate")),
        DropdownMenuEntry::Item(DropdownMenuItem::new("delete", "Delete")),
    ];
    let navigable = ["archive".into(), "duplicate".into(), "delete".into()];
    assert_eq!(
        typeahead_match(&entries, &navigable, Some(&navigable[1]), "d"),
        Some("delete".into())
    );
    assert_eq!(
        typeahead_match(&entries, &navigable, Some(&navigable[2]), "a"),
        Some("archive".into())
    );
}

#[test]
fn typeahead_accumulates_within_timeout_and_resets_after_timeout() {
    let started = Instant::now();
    let mut state = DropdownMenuState::default();

    update_typeahead_query(&mut state, "a", started);
    update_typeahead_query(&mut state, "r", started + Duration::from_millis(499));
    assert_eq!(state.typeahead, "ar");

    update_typeahead_query(&mut state, "d", started + Duration::from_secs(1));
    assert_eq!(state.typeahead, "d");
}

#[test]
fn keyboard_navigation_cancels_pending_and_open_submenus() {
    let mut state = DropdownMenuState {
        open_submenu: Some("open".into()),
        pending_submenu: Some("pending".into()),
        hover_generation: 4,
        ..DropdownMenuState::default()
    };

    state.cancel_submenu_interaction();

    assert_eq!(state.open_submenu, None);
    assert_eq!(state.pending_submenu, None);
    assert_eq!(state.hover_generation, 5);
}

#[test]
fn item_identity_survives_reordering() {
    let first = DropdownMenuEntry::Item(DropdownMenuItem::new("archive", "Archive"));
    let second = DropdownMenuEntry::Item(DropdownMenuItem::new("delete", "Delete"));
    let reordered = [second, first];

    assert_eq!(reordered[0].id().map(SharedString::as_ref), Some("delete"));
    assert_eq!(reordered[1].id().map(SharedString::as_ref), Some("archive"));
}

#[test]
fn retained_active_identity_resolves_the_same_item_after_reordering() {
    let active_id = SharedString::from("delete");
    let state = DropdownMenuState {
        active: Some(active_id.clone()),
        ..DropdownMenuState::default()
    };
    let reordered = [
        DropdownMenuEntry::Item(DropdownMenuItem::new("delete", "Delete")),
        DropdownMenuEntry::Item(DropdownMenuItem::new("archive", "Archive")),
    ];

    let resolved = reordered
        .iter()
        .find(|entry| entry.id() == state.active.as_ref());

    assert_eq!(resolved.and_then(DropdownMenuEntry::id), Some(&active_id));
}

#[gpui::test]
fn popup_expands_to_its_longest_item_and_rows_fill_the_popup(cx: &mut gpui::TestAppContext) {
    init_test_ui(cx);
    let (_, window) = cx.add_window_view(|_, _| WidthHarness);

    for _ in 0..3 {
        window.update(|window, cx| window.draw(cx).clear(cx));
    }

    let content = window.debug_bounds("dropdown-menu-content").unwrap();
    let modal_layer = window.debug_bounds("popover-modal-layer").unwrap();
    let viewport = window.update(|window, _| window.viewport_size());
    let long_row = window.debug_bounds("dropdown-menu-row-long-item").unwrap();
    let short_row = window.debug_bounds("dropdown-menu-row-short-item").unwrap();
    let separator = window.debug_bounds("dropdown-menu-separator").unwrap();
    assert_eq!(separator.origin.x, content.origin.x);
    assert_eq!(separator.right(), content.right());

    assert!(content.size.width > px(128.0));
    assert_eq!(long_row.size.width, content.size.width);
    assert_eq!(short_row.size.width, content.size.width);
    assert_eq!(long_row.size.height, px(26.0));
    assert_eq!(modal_layer.size, viewport);
}

#[gpui::test]
fn item_label_and_detail_reserve_visual_separation(cx: &mut gpui::TestAppContext) {
    init_test_ui(cx);
    let (_, window) = cx.add_window_view(|_, _| DetailWidthHarness);

    for _ in 0..3 {
        window.update(|window, cx| window.draw(cx).clear(cx));
    }

    let label = window.debug_bounds("dropdown-menu-row-label").unwrap();
    let detail = window.debug_bounds("dropdown-menu-row-detail").unwrap();

    assert!(detail.origin.x >= label.right() + px(8.0));
}
