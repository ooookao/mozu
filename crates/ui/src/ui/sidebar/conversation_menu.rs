//! Conversation row with hover actions and compact thread metadata preview.

use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{
    AnyElement, App, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, Styled as _, Window, div,
};

use crate::theme::colors::G2VTheme;
use crate::theme::sizing::{SIZE_4, SIZE_6, SIZE_7};
use crate::theme::spacing::SPACE_1;
use crate::theme::typography;
use crate::ui::{Button, ButtonSize, ButtonVariant, Icon, IconName, TextStyleExt as _};

use super::{ClickHandler, SidebarConversationMenuButton, SidebarHoverCard};

#[derive(IntoElement)]
#[must_use]
pub struct SidebarConversationMenu {
    id: SharedString,
    title: SharedString,
    project_name: SharedString,
    branch_label: SharedString,
    updated_at_ms: i64,
    active: bool,
    pinned: bool,
    on_click: ClickHandler,
    on_toggle_pin: ClickHandler,
    on_archive: ClickHandler,
    on_delete: Option<ClickHandler>,
}

pub struct SidebarConversationDetails {
    title: SharedString,
    project_name: SharedString,
    branch_label: SharedString,
    updated_at_ms: i64,
}

pub struct SidebarConversationActions {
    click: ClickHandler,
    toggle_pin: ClickHandler,
    archive: ClickHandler,
}

impl SidebarConversationDetails {
    pub fn new(
        title: impl Into<SharedString>,
        project_name: impl Into<SharedString>,
        branch_label: impl Into<SharedString>,
        updated_at_ms: i64,
    ) -> Self {
        Self {
            title: title.into(),
            project_name: project_name.into(),
            branch_label: branch_label.into(),
            updated_at_ms,
        }
    }
}

impl SidebarConversationActions {
    pub fn new(
        on_click: ClickHandler,
        on_toggle_pin: ClickHandler,
        on_archive: ClickHandler,
    ) -> Self {
        Self {
            click: on_click,
            toggle_pin: on_toggle_pin,
            archive: on_archive,
        }
    }
}

impl SidebarConversationMenu {
    pub fn new(
        id: impl Into<SharedString>,
        details: SidebarConversationDetails,
        actions: SidebarConversationActions,
    ) -> Self {
        Self {
            id: id.into(),
            title: details.title,
            project_name: details.project_name,
            branch_label: details.branch_label,
            updated_at_ms: details.updated_at_ms,
            active: false,
            pinned: false,
            on_click: actions.click,
            on_toggle_pin: actions.toggle_pin,
            on_archive: actions.archive,
            on_delete: None,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    pub fn on_delete(mut self, on_delete: ClickHandler) -> Self {
        self.on_delete = Some(on_delete);
        self
    }
}

impl RenderOnce for SidebarConversationMenu {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let group = SharedString::from(format!("sidebar-conversation-menu-{}", self.id));
        let hover_id = format!("sidebar-conversation-hover-{}", self.id);
        let delete_id = SharedString::from(format!("sidebar-conversation-delete-{}", self.id));
        let row = div()
            .relative()
            .w_full()
            .group(group.clone())
            .child(
                SidebarConversationMenuButton::new(
                    format!("sidebar-conversation-button-{}", self.id),
                    self.title.clone(),
                )
                .active(self.active)
                .hover_group(group.clone())
                .on_click(self.on_click),
            )
            .child(
                div()
                    .absolute()
                    .right(SPACE_1)
                    .top(gpui::px(2.0))
                    .h(SIZE_6)
                    .flex()
                    .items_center()
                    .gap_0p5()
                    .opacity(0.0)
                    .group_hover(group, |style| style.opacity(1.0))
                    .child(conversation_action(
                        format!("sidebar-conversation-pin-{}", self.id),
                        if self.pinned {
                            "Unpin chat"
                        } else {
                            "Pin chat"
                        },
                        IconName::Pin,
                        self.on_toggle_pin,
                    ))
                    .child(conversation_action(
                        format!("sidebar-conversation-archive-{}", self.id),
                        "Archive chat",
                        IconName::Archive,
                        self.on_archive,
                    )),
            );

        let title = self.title;
        let project_name = self.project_name;
        let branch_label = self.branch_label;
        let updated_at_ms = self.updated_at_ms;
        let delete_action = self.on_delete.map(|on_delete| (delete_id, on_delete));
        SidebarHoverCard::new(hover_id)
            .menu_surface()
            .trigger(row)
            .content(move |cx| {
                conversation_hover_content(
                    title.clone(),
                    project_name.clone(),
                    branch_label.clone(),
                    relative_time_label(updated_at_ms, now_ms()),
                    delete_action.clone(),
                    cx,
                )
            })
    }
}

fn conversation_action(
    id: impl Into<gpui::ElementId>,
    label: &'static str,
    icon: IconName,
    on_click: ClickHandler,
) -> Button {
    Button::new(id)
        .variant(ButtonVariant::GhostText)
        .size(ButtonSize::IconExtraSmall)
        .icon(icon)
        .icon_size(SIZE_4)
        .accessibility_label(label)
        .tooltip(label)
        .on_click_shared(on_click)
}

fn conversation_hover_content(
    title: SharedString,
    project_name: SharedString,
    branch_label: SharedString,
    time_label: SharedString,
    delete_action: Option<(SharedString, ClickHandler)>,
    cx: &App,
) -> AnyElement {
    let theme = G2VTheme::global(cx);
    let content = div()
        .w_full()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_start()
                .gap_2()
                .px_1p5()
                .py_1()
                .text_style(typography::tokens().sm)
                .child(div().min_w_0().flex_1().child(title))
                .child(
                    div()
                        .flex_none()
                        .text_style(typography::tokens().xs)
                        .text_color(theme.muted_foreground)
                        .child(time_label),
                ),
        )
        .child(metadata_row(
            IconName::FolderClosed,
            project_name,
            theme.muted_foreground,
        ))
        .child(metadata_row(
            IconName::Branch,
            branch_label,
            theme.muted_foreground,
        ));
    if let Some((delete_id, on_delete)) = delete_action {
        content
            .child(
                Button::new(delete_id)
                    .variant(ButtonVariant::Destructive)
                    .size(ButtonSize::Small)
                    .child("Delete")
                    .on_click_shared(on_delete),
            )
            .into_any_element()
    } else {
        content.into_any_element()
    }
}

fn metadata_row(icon: IconName, label: SharedString, icon_color: gpui::Hsla) -> gpui::Div {
    div()
        .h(SIZE_7)
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap_2()
        .px_1p5()
        .text_style(typography::tokens().xs)
        .child(Icon::new(icon, SIZE_4).color(icon_color))
        .child(div().min_w_0().truncate().child(label))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis())
                .expect("current Unix timestamp in milliseconds must fit in i64")
        })
}

fn relative_time_label(updated_at_ms: i64, current_time_ms: i64) -> SharedString {
    let elapsed_seconds = current_time_ms.saturating_sub(updated_at_ms) / 1_000;
    if elapsed_seconds < 60 {
        return "now".into();
    }
    let (value, suffix) = if elapsed_seconds < 3_600 {
        (elapsed_seconds / 60, "m")
    } else if elapsed_seconds < 86_400 {
        (elapsed_seconds / 3_600, "h")
    } else if elapsed_seconds < 2_592_000 {
        (elapsed_seconds / 86_400, "d")
    } else if elapsed_seconds < 31_536_000 {
        (elapsed_seconds / 2_592_000, "mo")
    } else {
        (elapsed_seconds / 31_536_000, "y")
    };
    format!("{value}{suffix}").into()
}

#[cfg(test)]
mod tests {
    use super::relative_time_label;

    #[test]
    fn relative_time_uses_compact_sidebar_units() {
        assert_eq!(relative_time_label(0, 1_000).as_ref(), "now");
        assert_eq!(relative_time_label(0, 59_000).as_ref(), "now");
        assert_eq!(relative_time_label(0, 120_000).as_ref(), "2m");
        assert_eq!(relative_time_label(0, 5_184_000_000).as_ref(), "2mo");
    }
}
