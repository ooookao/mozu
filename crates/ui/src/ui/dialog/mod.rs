//! G2V's shadcn-compatible dialog composition.

mod primitive;

use std::rc::Rc;

use gpui::{
    AnyElement, App, IntoElement, ParentElement as _, RenderOnce, SharedString, Styled as _,
    Window, div, prelude::FluentBuilder as _,
};
use gpui_base::{DialogClose as DialogCloseBehavior, DialogTrigger as DialogTriggerBehavior};

use crate::theme::colors::G2VTheme;
use crate::theme::rems_from_pixels;
use crate::theme::spacing::{SPACE_2, SPACE_4};
use crate::theme::typography;

use super::{Button, ButtonSize, ButtonVariant, IconName, TextStyleExt as _};

use primitive::ContentBuilder;
pub use primitive::{DialogHost, DialogHostGlobal};

type ConfirmationHandler = Rc<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App)>;

pub fn open_dialog(
    content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    DialogHost::open(Rc::new(content), window, cx);
}

pub fn close_dialog(window: &mut Window, cx: &mut App) {
    DialogHost::dismiss_global(window, cx);
}

/// Reports every dismissal path once, after releasing the modal host's mutable borrow.
pub fn open_dialog_with_dismiss(
    content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    on_dismiss: impl Fn(&mut App) + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    DialogHost::open_with_dismiss(Rc::new(content), Rc::new(on_dismiss), window, cx);
}

/// Opens an action-list dialog with an inset surface and the standard modal lifecycle.
pub fn open_action_dialog(
    content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    DialogHost::open_actions(Rc::new(content), window, cx);
}

/// Uses the same dismissal guard for Escape, outside pointer, and explicit close requests.
pub fn open_guarded_action_dialog(
    content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    can_dismiss: impl Fn(&App) -> bool + 'static,
    window: &mut Window,
    cx: &mut App,
) {
    DialogHost::open_guarded_actions(Rc::new(content), Rc::new(can_dismiss), window, cx);
}

pub fn open_destructive_confirmation(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    confirm_label: impl Into<SharedString>,
    on_confirm: ConfirmationHandler,
    window: &mut Window,
    cx: &mut App,
) {
    let confirm_label = confirm_label.into();
    let footer = Rc::new(move |_: &mut Window, _: &mut App| {
        let on_confirm = on_confirm.clone();
        div()
            .flex()
            .gap_2()
            .child(
                Button::new("destructive-confirmation-cancel")
                    .variant(ButtonVariant::Outline)
                    .size(ButtonSize::Default)
                    .child("Cancel")
                    .on_click(|_, window, cx| close_dialog(window, cx)),
            )
            .child(
                Button::new("destructive-confirmation-submit")
                    .variant(ButtonVariant::Destructive)
                    .size(ButtonSize::Default)
                    .child(confirm_label.clone())
                    .on_click(move |event, window, cx| {
                        on_confirm(event, window, cx);
                        close_dialog(window, cx);
                    }),
            )
            .into_any_element()
    });
    let content = dialog_content_builder(
        title.into(),
        Some(description.into()),
        Rc::new(|_, _| div().into_any_element()),
        Some(footer),
        true,
    );
    DialogHost::open(content, window, cx);
}

pub(crate) fn command_dialog_trigger(
    trigger: impl IntoElement,
    content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
) -> impl IntoElement {
    let content = Rc::new(content);
    DialogTriggerBehavior::new(trigger).on_open(move |window, cx| {
        primitive::DialogHost::open_command(content.clone(), window, cx);
    })
}

#[derive(IntoElement)]
#[must_use]
pub struct Dialog {
    trigger: AnyElement,
    title: SharedString,
    description: Option<SharedString>,
    body: ContentBuilder,
    footer: Option<ContentBuilder>,
    show_close_button: bool,
}

impl Dialog {
    pub fn new(trigger: impl IntoElement, title: impl Into<SharedString>) -> Self {
        Self {
            trigger: trigger.into_any_element(),
            title: title.into(),
            description: None,
            body: Rc::new(|_, _| div().into_any_element()),
            footer: None,
            show_close_button: true,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn content(
        mut self,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.body = Rc::new(content);
        self
    }

    pub fn footer(
        mut self,
        footer: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.footer = Some(Rc::new(footer));
        self
    }

    pub fn show_close_button(mut self, show: bool) -> Self {
        self.show_close_button = show;
        self
    }
}

impl RenderOnce for Dialog {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let content = dialog_content_builder(
            self.title,
            self.description,
            self.body,
            self.footer,
            self.show_close_button,
        );
        DialogTriggerBehavior::new(self.trigger)
            .on_open(move |window, cx| primitive::DialogHost::open(content.clone(), window, cx))
    }
}

fn dialog_content_builder(
    title: SharedString,
    description: Option<SharedString>,
    body: ContentBuilder,
    footer: Option<ContentBuilder>,
    show_close_button: bool,
) -> ContentBuilder {
    Rc::new(move |window, cx| {
        let theme = G2VTheme::global(cx);
        let muted_foreground = theme.muted_foreground;
        let popover = theme.popover;
        let type_tokens = typography::tokens();
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_style(type_tokens.md)
                            .line_height(gpui::relative(1.0))
                            .child(title.clone()),
                    )
                    .when_some(description.clone(), |header, description| {
                        header.child(
                            div()
                                .text_style(type_tokens.sm)
                                .text_color(muted_foreground)
                                .child(description),
                        )
                    }),
            )
            .child(body(window, cx))
            .when_some(footer.as_ref(), |content, footer| {
                content.child(
                    div()
                        .mx(-rems_from_pixels(SPACE_4))
                        .mb(-rems_from_pixels(SPACE_4))
                        .flex()
                        .justify_end()
                        .gap_2()
                        .rounded_b(crate::theme::radius::RADIUS_XL)
                        .bg(popover)
                        .p_4()
                        .child(footer(window, cx)),
                )
            })
            .when(show_close_button, |content| {
                content.child(
                    div()
                        .absolute()
                        .top(-rems_from_pixels(SPACE_2))
                        .right(-rems_from_pixels(SPACE_2))
                        .child(
                            DialogCloseBehavior::new().child(
                                Button::new("dialog-close")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::IconSmall)
                                    .accessibility_label("Close")
                                    .icon(IconName::X),
                            ),
                        ),
                )
            })
            .into_any_element()
    })
}
