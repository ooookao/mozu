//! Semantic menu buttons used by the application sidebar.

use std::f32::consts::FRAC_PI_2;

use gpui::{
    App, FontWeight, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Rems,
    RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, radians, relative,
};
use gpui_base::{Button as ButtonBehavior, TextStyleToken, Transition, transition};

use crate::theme::colors::G2VTheme;
use crate::theme::motion::{MotionTokens, cubic_bezier_with_overshoot};
use crate::theme::radius::RADIUS_MD;
use crate::theme::shadows::{RING_2, focus_ring};
use crate::theme::sizing::{SIZE_7, SIZE_8};
use crate::theme::spacing::SPACE_2;
use crate::theme::typography;
use crate::ui::{
    Icon, IconName, TextStyleExt as _, Tooltip, TooltipContent, TooltipSide, TooltipTrigger as _,
};

use super::ClickHandler;

struct SidebarButtonRuntime {
    focus: gpui::FocusHandle,
}

const fn trailing_icon_rotation_target(expanded: bool) -> f32 {
    if expanded { FRAC_PI_2 } else { 0.0 }
}

struct SidebarButton {
    id: gpui::ElementId,
    label: SharedString,
    icon: Option<(IconName, Rems)>,
    trailing_icon: Option<(IconName, Rems)>,
    trailing_icon_expanded: Option<bool>,
    status: SidebarButtonStatus,
    hover_group: Option<SharedString>,
    presentation: SidebarButtonPresentation,
    text_style: TextStyleToken,
    right_padding: Pixels,
    on_click: Option<ClickHandler>,
}

struct SidebarButtonStatus {
    active: bool,
    collapsed: bool,
    disabled: bool,
}

struct SidebarButtonPresentation {
    muted: bool,
    paints_interaction_background: bool,
}

impl SidebarButton {
    #[expect(
        clippy::too_many_lines,
        reason = "the declarative sidebar button composition stays in visual and interaction order"
    )]
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let tooltip = self.status.collapsed.then(|| self.label.clone());
        let background = if self.status.active {
            theme.sidebar_accent
        } else {
            theme.background.opacity(0.0)
        };
        let foreground = if self.status.active {
            theme.sidebar_accent_foreground
        } else if self.presentation.muted {
            theme.sidebar_foreground.opacity(0.7)
        } else {
            theme.sidebar_foreground
        };
        let accent_background = theme.sidebar_accent;
        let hover_background = if self.status.active {
            accent_background
        } else {
            accent_background.alpha(if theme.is_dark { 0.06 } else { 0.04 })
        };
        let accent_foreground = theme.sidebar_accent_foreground;
        let focus = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| SidebarButtonRuntime {
                focus: cx.focus_handle(),
            })
            .read(cx)
            .focus
            .clone();
        let focus_ring = focus_ring(theme.sidebar_ring, RING_2);
        let trailing_icon_rotation = self.trailing_icon_expanded.map(|expanded| {
            transition(
                (self.id.clone(), "trailing-icon-rotation"),
                trailing_icon_rotation_target(expanded),
                Transition::new(MotionTokens::global(cx).fast)
                    .ease(cubic_bezier_with_overshoot(0.0, 0.0, 0.2, 1.0)),
                window,
                cx,
            )
        });
        let title_group = self.hover_group.clone();
        let right_padding = self.right_padding;
        let button = ButtonBehavior::new(self.id)
            .track_focus(&focus)
            .h(if self.status.collapsed {
                SIZE_8
            } else {
                SIZE_7
            })
            .when(self.status.collapsed, |button| button.w(SIZE_8))
            .when(!self.status.collapsed, gpui::Styled::w_full)
            .flex()
            .items_center()
            .justify_start()
            .gap_2()
            .pl_2()
            .pr(if title_group.is_some() {
                SPACE_2
            } else {
                right_padding
            })
            .overflow_hidden()
            .rounded(RADIUS_MD)
            .text_style(self.text_style)
            .when(self.status.active, |button| {
                button.font_weight(FontWeight::SEMIBOLD)
            })
            .line_height(relative(1.2))
            .bg(background)
            .text_color(foreground)
            .cursor_pointer()
            .disabled(self.status.disabled)
            .accessibility_label(self.label.clone())
            .selected(self.status.active)
            .styles(|styles| styles.disabled(|style| style.opacity(0.5).cursor_default()))
            .when(
                !self.status.disabled && self.presentation.paints_interaction_background,
                |button| {
                    button
                        .hover(move |style| {
                            style.bg(hover_background).text_color(accent_foreground)
                        })
                        .active(move |style| {
                            style.bg(accent_background).text_color(accent_foreground)
                        })
                },
            )
            .when(!self.status.disabled, |button| {
                button.focus_visible(move |style| style.shadow(focus_ring))
            })
            .when_some(self.hover_group, |button, group| {
                button.group_hover(group, move |style| {
                    style
                        .bg(hover_background)
                        .text_color(accent_foreground)
                        .pr(right_padding)
                })
            })
            .when_some(self.on_click, |button, handler| {
                button.on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when_some(self.icon, |button, (name, size)| {
                button.child(Icon::new(name, size).color(foreground))
            })
            .when(!self.status.collapsed, |button| match title_group {
                Some(_) => button.child(div().min_w_0().flex_1().truncate().child(self.label)),
                None => button.child(div().min_w_0().truncate().child(self.label)),
            })
            .when_some(
                self.trailing_icon.filter(|_| !self.status.collapsed),
                |button, (name, size)| {
                    button.child(
                        Icon::new(name, size)
                            .color(foreground)
                            .rotate(radians(trailing_icon_rotation.unwrap_or(0.0))),
                    )
                },
            );

        match tooltip {
            Some(tooltip) => button.with_tooltip_content_and_focus(
                Tooltip::new(TooltipContent::new(tooltip).side(TooltipSide::Right)),
                focus,
            ),
            None => button,
        }
    }
}

macro_rules! semantic_sidebar_button {
    ($name:ident, $text_style:expr, $muted:expr, $interaction:expr, $right_padding:expr) => {
        #[derive(IntoElement)]
        #[must_use]
        pub struct $name {
            button: SidebarButton,
        }

        impl $name {
            pub fn new(id: impl Into<gpui::ElementId>, label: impl Into<SharedString>) -> Self {
                Self {
                    button: SidebarButton {
                        id: id.into(),
                        label: label.into(),
                        icon: None,
                        trailing_icon: None,
                        trailing_icon_expanded: None,
                        status: SidebarButtonStatus {
                            active: false,
                            collapsed: false,
                            disabled: false,
                        },
                        hover_group: None,
                        presentation: SidebarButtonPresentation {
                            muted: $muted,
                            paints_interaction_background: $interaction,
                        },
                        text_style: $text_style,
                        right_padding: $right_padding,
                        on_click: None,
                    },
                }
            }

            pub fn icon(mut self, name: IconName, size: Rems) -> Self {
                self.button.icon = Some((name, size));
                self
            }

            pub fn trailing_icon(mut self, name: IconName, size: Rems) -> Self {
                self.button.trailing_icon = Some((name, size));
                self
            }

            pub fn trailing_icon_expanded(mut self, expanded: bool) -> Self {
                self.button.trailing_icon_expanded = Some(expanded);
                self
            }

            pub fn active(mut self, active: bool) -> Self {
                self.button.status.active = active;
                self
            }

            pub fn collapsed(mut self, collapsed: bool) -> Self {
                self.button.status.collapsed = collapsed;
                self
            }

            pub fn disabled(mut self, disabled: bool) -> Self {
                self.button.status.disabled = disabled;
                self
            }

            pub fn muted(mut self, muted: bool) -> Self {
                self.button.presentation.muted = muted;
                self
            }

            pub fn on_click(mut self, handler: ClickHandler) -> Self {
                self.button.on_click = Some(handler);
                self
            }
        }

        impl RenderOnce for $name {
            fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
                self.button.render(window, cx)
            }
        }
    };
}

semantic_sidebar_button!(SidebarMenuButton, typography::TEXT_13, false, true, SPACE_2);
semantic_sidebar_button!(
    SidebarProjectButton,
    typography::TEXT_13,
    false,
    true,
    gpui::px(68.0)
);

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::{SidebarConversationMenuButton, trailing_icon_rotation_target};

    #[test]
    fn collapsible_chevron_rotates_from_right_to_down() {
        assert!(trailing_icon_rotation_target(false).abs() < f32::EPSILON);
        assert!((trailing_icon_rotation_target(true) - FRAC_PI_2).abs() < f32::EPSILON);
    }

    #[test]
    fn conversation_buttons_are_not_muted_unless_requested() {
        let conversation = SidebarConversationMenuButton::new("conversation", "Conversation");
        let show_more = SidebarConversationMenuButton::new("show-more", "Show more").muted(true);

        assert!(!conversation.button.presentation.muted);
        assert!(show_more.button.presentation.muted);
    }
}

impl SidebarProjectButton {
    pub(crate) fn hover_group(mut self, group: impl Into<SharedString>) -> Self {
        self.button.hover_group = Some(group.into());
        self
    }
}

impl SidebarConversationMenuButton {
    pub(crate) fn hover_group(mut self, group: impl Into<SharedString>) -> Self {
        self.button.hover_group = Some(group.into());
        self.button.right_padding = gpui::px(52.0);
        self
    }
}
semantic_sidebar_button!(
    SidebarProjectMenuButton,
    typography::tokens().xs,
    true,
    false,
    SPACE_2
);
semantic_sidebar_button!(
    SidebarConversationMenuButton,
    typography::tokens().xs,
    false,
    true,
    SPACE_2
);
