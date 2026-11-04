//! Project row composition with hover actions and project metadata preview.

use std::rc::Rc;

use gpui::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use gpui_base::{Transition, transition};

use crate::theme::border::BORDER_DEFAULT;
use crate::theme::colors::G2VTheme;
use crate::theme::motion::{MotionTokens, cubic_bezier_with_overshoot};
use crate::theme::radius::RADIUS_MD;
use crate::theme::sizing::{SIZE_3, SIZE_4, SIZE_6, SIZE_7};
use crate::theme::spacing::SPACE_1;
use crate::theme::typography;
use crate::ui::{
    Button, ButtonContentAlignment, ButtonSize, ButtonVariant, DropdownMenu, DropdownMenuDensity,
    DropdownMenuItem, DropdownMenuItemVariant, Icon, IconName, PopoverAlign, TextStyleExt as _,
};

use super::{ClickHandler, SidebarHoverCard, SidebarProjectButton};

struct ProjectMenuRuntime {
    hovered: bool,
}

pub(crate) type MenuSelectHandler = std::rc::Rc<dyn Fn(&mut Window, &mut App)>;

const fn action_opacity_target(hovered: bool) -> f32 {
    if hovered { 1.0 } else { 0.0 }
}

#[derive(IntoElement)]
#[must_use]
pub struct SidebarProjectMenu {
    id: SharedString,
    name: SharedString,
    path: SharedString,
    chat_count: usize,
    expanded: bool,
    active: bool,
    pinned: bool,
    on_click: ClickHandler,
    on_toggle_pin: ClickHandler,
    on_archive_chats: MenuSelectHandler,
    on_remove: MenuSelectHandler,
    on_new_chat: ClickHandler,
    on_edit: ClickHandler,
    on_import_sessions: MenuSelectHandler,
}

pub struct SidebarProjectDetails {
    name: SharedString,
    path: SharedString,
    chat_count: usize,
}

pub struct SidebarProjectActions {
    click: ClickHandler,
    toggle_pin: ClickHandler,
    archive_chats: MenuSelectHandler,
    remove: MenuSelectHandler,
    new_chat: ClickHandler,
    edit: ClickHandler,
    import_sessions: MenuSelectHandler,
}

impl SidebarProjectDetails {
    pub fn new(
        name: impl Into<SharedString>,
        path: impl Into<SharedString>,
        chat_count: usize,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            chat_count,
        }
    }
}

impl SidebarProjectActions {
    pub fn new(
        on_click: ClickHandler,
        on_toggle_pin: ClickHandler,
        on_archive_chats: MenuSelectHandler,
        on_remove: MenuSelectHandler,
        on_new_chat: ClickHandler,
        on_edit: ClickHandler,
        on_import_sessions: MenuSelectHandler,
    ) -> Self {
        Self {
            click: on_click,
            toggle_pin: on_toggle_pin,
            archive_chats: on_archive_chats,
            remove: on_remove,
            new_chat: on_new_chat,
            edit: on_edit,
            import_sessions: on_import_sessions,
        }
    }
}

impl SidebarProjectMenu {
    pub fn new(
        id: impl Into<SharedString>,
        details: SidebarProjectDetails,
        actions: SidebarProjectActions,
    ) -> Self {
        Self {
            id: id.into(),
            name: details.name,
            path: details.path,
            chat_count: details.chat_count,
            expanded: false,
            active: false,
            pinned: false,
            on_click: actions.click,
            on_toggle_pin: actions.toggle_pin,
            on_archive_chats: actions.archive_chats,
            on_remove: actions.remove,
            on_new_chat: actions.new_chat,
            on_edit: actions.edit,
            on_import_sessions: actions.import_sessions,
        }
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }
}

impl RenderOnce for SidebarProjectMenu {
    #[expect(
        clippy::too_many_lines,
        reason = "the declarative project row keeps hover, actions, and popup composition together"
    )]
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let group = SharedString::from(format!("sidebar-project-menu-{}", self.id));
        let hover_id = format!("sidebar-project-hover-{}", self.id);
        let row_id = SharedString::from(format!("sidebar-project-row-{}", self.id));
        let runtime: Entity<ProjectMenuRuntime> =
            window.use_keyed_state(row_id.clone(), cx, |_, _| ProjectMenuRuntime {
                hovered: false,
            });
        let action_opacity = transition(
            (self.id.clone(), "actions-opacity"),
            action_opacity_target(runtime.read(cx).hovered),
            Transition::new(MotionTokens::global(cx).fast)
                .ease(cubic_bezier_with_overshoot(0.0, 0.0, 0.2, 1.0)),
            window,
            cx,
        );
        let row = div()
            .id(row_id)
            .relative()
            .w_full()
            .group(group.clone())
            .on_hover({
                let runtime = runtime.clone();
                move |hovered, _, cx| {
                    runtime.update(cx, |runtime, cx| {
                        if runtime.hovered != *hovered {
                            runtime.hovered = *hovered;
                            cx.notify();
                        }
                    });
                }
            })
            .child(
                SidebarProjectButton::new(
                    format!("sidebar-project-button-{}", self.id),
                    self.name.clone(),
                )
                .icon(
                    if self.expanded {
                        IconName::FolderOpen
                    } else {
                        IconName::FolderClosed
                    },
                    SIZE_4,
                )
                .active(self.active)
                .hover_group(group.clone())
                .on_click(self.on_click),
            )
            .child(
                div()
                    .absolute()
                    .right(SPACE_1)
                    .top(px(2.0))
                    .h(SIZE_6)
                    .flex()
                    .items_center()
                    .gap_0p5()
                    .opacity(action_opacity)
                    .child(
                        DropdownMenu::new(
                            format!("sidebar-project-actions-menu-{}", self.id),
                            project_action(
                                format!("sidebar-project-menu-action-{}", self.id),
                                "Project actions",
                                IconName::DotsVertical,
                                Rc::new(|_, _, _| {}),
                            ),
                        )
                        .density(DropdownMenuDensity::Compact)
                        .align(PopoverAlign::End)
                        .item(
                            DropdownMenuItem::new("import-cli-sessions", "Import CLI sessions…")
                                .on_select({
                                    let handler = self.on_import_sessions;
                                    move |window, cx| handler(window, cx)
                                }),
                        )
                        .separator()
                        .item(
                            DropdownMenuItem::new("archive-all-chats", "Archive all chats")
                                .icon(IconName::Archive)
                                .on_select({
                                    let handler = self.on_archive_chats;
                                    move |window, cx| handler(window, cx)
                                }),
                        )
                        .separator()
                        .item(
                            DropdownMenuItem::new("remove-project", "Remove project")
                                .icon(IconName::X)
                                .variant(DropdownMenuItemVariant::Destructive)
                                .on_select({
                                    let handler = self.on_remove;
                                    move |window, cx| handler(window, cx)
                                }),
                        ),
                    )
                    .child(project_action(
                        format!("sidebar-project-new-chat-action-{}", self.id),
                        "New chat",
                        IconName::PencilEdit,
                        self.on_new_chat,
                    )),
            );

        let content = ProjectHoverContent {
            id: self.id,
            name: self.name,
            path: self.path,
            chat_count: self.chat_count,
            pinned: self.pinned,
            on_toggle_pin: self.on_toggle_pin,
            on_edit: self.on_edit,
        };
        SidebarHoverCard::new(hover_id)
            .menu_surface()
            .trigger(row)
            .content(move |cx| project_hover_content(&content, cx))
    }
}

fn project_action(
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

struct ProjectHoverContent {
    id: SharedString,
    name: SharedString,
    path: SharedString,
    chat_count: usize,
    pinned: bool,
    on_toggle_pin: ClickHandler,
    on_edit: ClickHandler,
}

fn project_hover_content(content: &ProjectHoverContent, cx: &App) -> AnyElement {
    let theme = G2VTheme::global(cx);
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(
            hover_row()
                .gap_2p5()
                .child(hover_icon(IconName::FolderOpen, theme.muted_foreground))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .child(content.name.clone()),
                )
                .child(
                    Button::new(format!("sidebar-project-hover-pin-{}", content.id))
                        .variant(ButtonVariant::GhostText)
                        .size(ButtonSize::IconExtraSmall)
                        .icon(if content.pinned {
                            IconName::Check
                        } else {
                            IconName::PlusMedium
                        })
                        .icon_size(SIZE_3)
                        .accessibility_label(if content.pinned {
                            "Unpin project"
                        } else {
                            "Pin project"
                        })
                        .tooltip(if content.pinned {
                            "Unpin project"
                        } else {
                            "Pin project"
                        })
                        .on_click_shared(content.on_toggle_pin.clone()),
                ),
        )
        .child(
            hover_row()
                .child(hover_icon(IconName::PlusComments, theme.muted_foreground))
                .child(format!(
                    "{} {}",
                    content.chat_count,
                    if content.chat_count == 1 {
                        "chat"
                    } else {
                        "chats"
                    }
                )),
        )
        .child(hover_separator(theme.border))
        .child(
            hover_row()
                .child(hover_icon(IconName::FolderClosed, theme.muted_foreground))
                .child(div().min_w_0().truncate().child(content.path.clone())),
        )
        .child(hover_separator(theme.border).mb_0p5())
        .child(
            Button::new(format!("sidebar-project-hover-edit-{}", content.id))
                .variant(ButtonVariant::PopoverItem)
                .size(ButtonSize::ExtraSmall)
                .content_alignment(ButtonContentAlignment::Start)
                .full_width()
                .icon(IconName::PencilEdit)
                .icon_size(SIZE_4)
                .label("Edit project")
                .on_click_shared(content.on_edit.clone()),
        )
        .into_any_element()
}

fn hover_row() -> gpui::Div {
    div()
        .h(SIZE_7)
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap_2()
        .px_1p5()
        .rounded(RADIUS_MD)
        .text_style(typography::tokens().xs)
}

fn hover_icon(icon: IconName, color: gpui::Hsla) -> Icon {
    Icon::new(icon, SIZE_4).color(color)
}

fn hover_separator(color: gpui::Hsla) -> gpui::Div {
    div().mx_0p5().h(BORDER_DEFAULT).bg(color)
}

#[cfg(test)]
mod tests {
    use super::action_opacity_target;

    #[test]
    fn project_actions_fade_between_hidden_and_visible_targets() {
        assert!(action_opacity_target(false).abs() < f32::EPSILON);
        assert!((action_opacity_target(true) - 1.0).abs() < f32::EPSILON);
    }
}
