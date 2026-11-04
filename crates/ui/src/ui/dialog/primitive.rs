//! Window-root modal lifecycle and Base UI-compatible behavior.

use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Context, Entity, FocusHandle, Global,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Pixels, Rems, Render,
    Styled as _, Task, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_base::Dialog as DialogBehavior;

use crate::{
    theme::{
        border::BORDER_DEFAULT,
        motion::{MotionPolicy, elevated_surface_opacity},
        rems_from_design_pixels, rems_from_pixels, shadows,
        sizing::MaxWidth,
        spacing::{SPACE_1, SPACE_4},
        typography,
    },
    ui::{
        TextStyleExt as _, floating_surface, modal_backdrop::dialog_modal_backdrop,
        responsive::Breakpoint,
    },
};

const TRANSITION_DURATION: Duration = Duration::from_millis(100);
const VIEWPORT_INSET: Rems = rems_from_design_pixels(16.0);

pub(super) type ContentBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
pub(super) type DismissHandler = Rc<dyn Fn(&mut App)>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum DialogSurface {
    #[default]
    Default,
    Command,
    Actions,
}

#[derive(Clone)]
struct DialogContent {
    builder: ContentBuilder,
    surface: DialogSurface,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DialogPhase {
    Hidden,
    Opening,
    Open,
    Closing,
}

pub struct DialogHostGlobal(pub Entity<DialogHost>);

impl Global for DialogHostGlobal {}

pub struct DialogHost {
    focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
    phase: DialogPhase,
    content: Option<DialogContent>,
    generation: usize,
    transition_task: Option<Task<()>>,
    on_dismiss: Option<DismissHandler>,
    dismiss_guard: Option<Rc<dyn Fn(&App) -> bool>>,
}

impl DialogHost {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            previous_focus: None,
            phase: DialogPhase::Hidden,
            content: None,
            generation: 0,
            transition_task: None,
            on_dismiss: None,
            dismiss_guard: None,
        }
    }

    pub(crate) fn open(content: ContentBuilder, window: &mut Window, cx: &mut App) {
        Self::open_surface(content, DialogSurface::Default, None, window, cx);
    }

    pub(super) fn open_guarded_actions(
        content: ContentBuilder,
        guard: Rc<dyn Fn(&App) -> bool>,
        window: &mut Window,
        cx: &mut App,
    ) {
        Self::open_actions(content, window, cx);
        let host = cx.global::<DialogHostGlobal>().0.clone();
        host.update(cx, |host, _| host.dismiss_guard = Some(guard));
    }

    pub(super) fn open_with_dismiss(
        content: ContentBuilder,
        on_dismiss: DismissHandler,
        window: &mut Window,
        cx: &mut App,
    ) {
        Self::open_surface(
            content,
            DialogSurface::Default,
            Some(on_dismiss),
            window,
            cx,
        );
    }

    pub(super) fn open_command(content: ContentBuilder, window: &mut Window, cx: &mut App) {
        Self::open_surface(content, DialogSurface::Command, None, window, cx);
    }

    pub(super) fn open_actions(content: ContentBuilder, window: &mut Window, cx: &mut App) {
        Self::open_surface(content, DialogSurface::Actions, None, window, cx);
    }

    fn open_surface(
        content: ContentBuilder,
        surface: DialogSurface,
        on_dismiss: Option<DismissHandler>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let host = cx.global::<DialogHostGlobal>().0.clone();
        host.update(cx, |host, cx| {
            if let Some(previous) = host.on_dismiss.take() {
                cx.defer(move |cx| previous(cx));
            }
            host.on_dismiss = on_dismiss;
            host.dismiss_guard = None;
            let transition_duration = MotionPolicy::global(cx).duration(TRANSITION_DURATION);
            host.generation += 1;
            let generation = host.generation;
            host.transition_task = None;
            if host.phase == DialogPhase::Hidden {
                host.previous_focus = window.focused(cx);
            }
            host.content = Some(DialogContent {
                builder: content,
                surface,
            });
            host.phase = DialogPhase::Opening;
            host.focus.focus(window, cx);
            host.transition_task = Some(cx.spawn_in(window, async move |host, cx| {
                cx.background_executor().timer(transition_duration).await;
                let _ = host.update_in(cx, |host, _, cx| {
                    if host.generation == generation && host.phase == DialogPhase::Opening {
                        host.phase = DialogPhase::Open;
                        host.transition_task = None;
                        cx.notify();
                    }
                });
            }));
            cx.notify();
        });
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dismiss_guard.as_ref().is_some_and(|guard| !guard(cx)) {
            return;
        }
        if !matches!(self.phase, DialogPhase::Opening | DialogPhase::Open) {
            return;
        }
        let transition_duration = MotionPolicy::global(cx).duration(TRANSITION_DURATION);
        self.generation += 1;
        let generation = self.generation;
        self.transition_task = None;
        self.phase = DialogPhase::Closing;
        if let Some(on_dismiss) = self.on_dismiss.take() {
            cx.defer(move |cx| on_dismiss(cx));
        }
        self.transition_task = Some(cx.spawn_in(window, async move |host, cx| {
            cx.background_executor().timer(transition_duration).await;
            let _ = host.update_in(cx, |host, window, cx| {
                if host.generation == generation && host.phase == DialogPhase::Closing {
                    host.phase = DialogPhase::Hidden;
                    host.content = None;
                    host.transition_task = None;
                    if let Some(previous_focus) = host.previous_focus.take() {
                        previous_focus.focus(window, cx);
                    }
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    pub(crate) fn dismiss_global(window: &mut Window, cx: &mut App) {
        let host = cx.global::<DialogHostGlobal>().0.clone();
        host.update(cx, |host, cx| host.dismiss(window, cx));
    }
}

impl Render for DialogHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.phase == DialogPhase::Hidden {
            return div().into_any_element();
        }

        let theme = crate::theme::colors::G2VTheme::global(cx);
        let popover = theme.popover;
        let popover_foreground = theme.popover_foreground;
        let outline = theme.popover_border;
        let elevation = shadows::tokens(window.appearance()).lg;
        let viewport = window.viewport_size();
        let type_tokens = typography::tokens();
        let phase = self.phase;
        let host = cx.entity();
        let dialog_content = self
            .content
            .as_ref()
            .expect("mounted dialog always owns a content builder");
        let content = (dialog_content.builder)(window, cx);
        let surface = dialog_content.surface;
        let width = dialog_width(viewport.width, window.rem_size(), surface);

        let backdrop_host = host.clone();
        let backdrop = animate_backdrop(
            dialog_modal_backdrop(viewport).on_mouse_down(
                MouseButton::Left,
                move |_, window, cx| {
                    backdrop_host.update(cx, |host, cx| host.dismiss(window, cx));
                },
            ),
            phase,
        );
        let popup = div()
            .absolute()
            .inset_0()
            .flex()
            .justify_center()
            .when(
                matches!(surface, DialogSurface::Default | DialogSurface::Actions),
                gpui::Styled::items_center,
            )
            .when(surface == DialogSurface::Command, |popup| {
                popup.items_start().pt(viewport.height / 3.0)
            })
            .child(animate_popup(
                div()
                    .id("dialog-content")
                    .debug_selector(|| "dialog-content".into())
                    .relative()
                    .occlude()
                    .w(width)
                    .flex()
                    .flex_col()
                    .when(surface == DialogSurface::Default, |popup| {
                        popup
                            .gap(rems_from_pixels(SPACE_4))
                            .p(rems_from_pixels(SPACE_4 - BORDER_DEFAULT))
                            .rounded(crate::theme::radius::RADIUS_XL)
                    })
                    .when(surface == DialogSurface::Command, |popup| {
                        popup.rounded(crate::theme::radius::RADIUS_LG)
                    })
                    .when(surface == DialogSurface::Actions, |popup| {
                        popup
                            .p(SPACE_1 - BORDER_DEFAULT)
                            .rounded(crate::theme::radius::RADIUS_XL)
                    })
                    .bg(popover)
                    .text_style(type_tokens.sm)
                    .text_color(popover_foreground)
                    .border(BORDER_DEFAULT)
                    .border_color(outline)
                    .shadow(elevation)
                    .child(content),
                phase,
            ));

        let dialog = DialogBehavior::new(cx)
            .focus_handle(self.focus.clone())
            .close_on_escape(true)
            .close_on_backdrop_press(false)
            .backdrop(backdrop)
            .popup(popup)
            .on_cancel(|_, _, _| true)
            .request_close(move |_, window, cx| {
                host.update(cx, |host, cx| host.dismiss(window, cx));
            })
            .into_any_element();

        floating_surface(dialog, gpui_base::POPUP_PRIORITY).into_any_element()
    }
}

fn dialog_width(viewport_width: Pixels, rem_size: Pixels, surface: DialogSurface) -> Pixels {
    let available_width =
        (viewport_width - VIEWPORT_INSET.to_pixels(rem_size) * 2.0).max(Pixels::ZERO);
    let maximum_width = rems_from_pixels(match surface {
        DialogSurface::Actions => MaxWidth::MEDIUM,
        DialogSurface::Default | DialogSurface::Command => MaxWidth::SMALL,
    })
    .to_pixels(rem_size);
    if viewport_width < Breakpoint::SMALL {
        available_width
    } else {
        available_width.min(maximum_width)
    }
}

fn animate_backdrop(backdrop: gpui::Div, phase: DialogPhase) -> AnyElement {
    match phase {
        DialogPhase::Opening => backdrop
            .with_animation(
                "dialog-backdrop-enter",
                Animation::new(TRANSITION_DURATION),
                gpui::Styled::opacity,
            )
            .into_any_element(),
        DialogPhase::Closing => backdrop
            .with_animation(
                "dialog-backdrop-exit",
                Animation::new(TRANSITION_DURATION),
                |backdrop, delta| backdrop.opacity(1.0 - delta),
            )
            .into_any_element(),
        DialogPhase::Open | DialogPhase::Hidden => backdrop.into_any_element(),
    }
}

fn animate_popup(popup: gpui::Stateful<gpui::Div>, phase: DialogPhase) -> AnyElement {
    match phase {
        DialogPhase::Opening => popup
            .with_animation(
                "dialog-popup-enter",
                Animation::new(TRANSITION_DURATION),
                |popup, delta| {
                    popup
                        .opacity(elevated_surface_opacity(delta))
                        .top(px(2.0 * (1.0 - delta)))
                },
            )
            .into_any_element(),
        DialogPhase::Closing => popup
            .with_animation(
                "dialog-popup-exit",
                Animation::new(TRANSITION_DURATION),
                |popup, delta| {
                    popup
                        .opacity(elevated_surface_opacity(1.0 - delta))
                        .top(px(2.0 * delta))
                },
            )
            .into_any_element(),
        DialogPhase::Open | DialogPhase::Hidden => popup.into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AppContext as _;

    struct DialogHostHarness(Entity<DialogHost>);

    impl Render for DialogHostHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            self.0.clone()
        }
    }

    #[gpui::test]
    fn reduced_motion_completes_the_dialog_lifecycle_without_waiting(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(gpui::WindowAppearance::Light, cx);
            cx.set_reduce_motion(true);
        });
        let host = cx.update(|cx| cx.new(DialogHost::new));
        cx.update(|cx| cx.set_global(DialogHostGlobal(host.clone())));
        let (_, window) = cx.add_window_view({
            let host = host.clone();
            move |_, _| DialogHostHarness(host)
        });

        window.update(|window, cx| {
            DialogHost::open(Rc::new(|_, _| div().into_any_element()), window, cx);
        });
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| host.read(cx).phase),
            DialogPhase::Open
        );

        window.update(DialogHost::dismiss_global);
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| host.read(cx).phase),
            DialogPhase::Hidden
        );

        window.update(|window, cx| {
            DialogHost::open_actions(
                Rc::new(|_, _| {
                    div()
                        .debug_selector(|| "dialog-action-inner".into())
                        .w_full()
                        .h(gpui::px(32.0))
                        .into_any_element()
                }),
                window,
                cx,
            );
        });
        window.run_until_parked();
        window.update(|window, cx| window.draw(cx).clear(cx));
        let outer = window.debug_bounds("dialog-content").unwrap();
        let inner = window.debug_bounds("dialog-action-inner").unwrap();
        assert_eq!(inner.origin.x - outer.origin.x, gpui::px(4.0));
        assert_eq!(inner.origin.y - outer.origin.y, gpui::px(4.0));
        assert_eq!(outer.bottom() - inner.bottom(), gpui::px(4.0));
        window.update(DialogHost::dismiss_global);
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| host.read(cx).phase),
            DialogPhase::Hidden
        );
    }

    #[test]
    fn transition_duration_matches_the_react_dialog() {
        assert_eq!(TRANSITION_DURATION, Duration::from_millis(100));
    }

    #[gpui::test]
    fn guarded_dialog_rejects_dismissal_until_operation_finishes(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(gpui::WindowAppearance::Light, cx);
            cx.set_reduce_motion(true);
        });
        let host = cx.update(|cx| cx.new(DialogHost::new));
        cx.update(|cx| cx.set_global(DialogHostGlobal(host.clone())));
        let (_, window) = cx.add_window_view({
            let host = host.clone();
            move |_, _| DialogHostHarness(host)
        });
        let finished = Rc::new(std::cell::Cell::new(false));
        let guard = finished.clone();
        window.update(|window, cx| {
            DialogHost::open_guarded_actions(
                Rc::new(|_, _| div().into_any_element()),
                Rc::new(move |_| guard.get()),
                window,
                cx,
            )
        });
        window.run_until_parked();
        window.update(DialogHost::dismiss_global);
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| host.read(cx).phase),
            DialogPhase::Open
        );
        finished.set(true);
        window.update(DialogHost::dismiss_global);
        window.run_until_parked();
        assert_eq!(
            window.update(|_, cx| host.read(cx).phase),
            DialogPhase::Hidden
        );
    }

    #[gpui::test]
    fn dismissal_callback_is_once_and_can_update_the_host(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            gpui_base::init(cx);
            crate::ui::init(cx);
            crate::theme::typography::install(cx);
            crate::theme::install(gpui::WindowAppearance::Light, cx);
            cx.set_reduce_motion(true);
        });
        let host = cx.update(|cx| cx.new(DialogHost::new));
        cx.update(|cx| cx.set_global(DialogHostGlobal(host.clone())));
        let (_, window) = cx.add_window_view({
            let host = host.clone();
            move |_, _| DialogHostHarness(host)
        });
        let count = Rc::new(std::cell::Cell::new(0));
        let observed = count.clone();
        window.update(|window, cx| {
            DialogHost::open_with_dismiss(
                Rc::new(|_, _| div().into_any_element()),
                Rc::new(move |cx| {
                    observed.set(observed.get() + 1);
                    let host = cx.global::<DialogHostGlobal>().0.clone();
                    host.update(cx, |_, _| {});
                }),
                window,
                cx,
            );
            DialogHost::dismiss_global(window, cx);
            DialogHost::dismiss_global(window, cx);
        });
        window.run_until_parked();
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn dialog_width_matches_shadcn_responsive_constraints() {
        assert_eq!(
            dialog_width(px(320.0), px(16.0), DialogSurface::Default),
            px(288.0)
        );
        assert_eq!(
            dialog_width(px(320.0), px(18.0), DialogSurface::Default),
            px(284.0)
        );
        assert_eq!(
            dialog_width(Breakpoint::SMALL, px(16.0), DialogSurface::Default),
            MaxWidth::SMALL
        );
        assert_eq!(
            dialog_width(Breakpoint::SMALL, px(16.0), DialogSurface::Actions),
            MaxWidth::MEDIUM
        );
        assert_eq!(
            dialog_width(px(1280.0), px(18.0), DialogSurface::Default),
            px(432.0)
        );
    }
}
