use std::{cell::Cell, collections::HashMap, rc::Rc, time::Duration};

use gpui::{
    App, Bounds, Context, Entity, FocusHandle, Global, IntoElement, MouseButton,
    ParentElement as _, Pixels, Render, Window, WindowId, div,
};
use gpui_base::{ElementExt, Positioner};

use super::{TooltipContent, TooltipHoverHandler, TooltipPhase};
use crate::ui::floating_surface;

#[derive(Clone)]
pub(super) struct TooltipRequest {
    trigger_bounds: Bounds<Pixels>,
    content: TooltipContent,
}

pub(super) struct TooltipProviderState {
    request: Option<TooltipRequest>,
    exiting: Option<TooltipRequest>,
    delay: Duration,
    generation: u64,
    show_task: Option<gpui::Task<()>>,
    hide_task: Option<gpui::Task<()>>,
}

impl TooltipProviderState {
    pub(super) fn new(delay: Duration) -> Self {
        Self {
            request: None,
            exiting: None,
            delay,
            generation: 0,
            show_task: None,
            hide_task: None,
        }
    }

    fn show(&mut self, request: TooltipRequest, window: &mut Window, cx: &mut Context<Self>) {
        self.generation += 1;
        let generation = self.generation;
        self.hide_task = None;
        self.exiting = None;
        if self.delay.is_zero() || self.request.is_some() {
            self.request = Some(request);
            self.show_task = None;
            cx.notify();
            return;
        }
        let delay = self.delay;
        self.show_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.generation == generation {
                    this.request = Some(request);
                    this.show_task = None;
                    cx.notify();
                }
            });
        }));
    }

    fn begin_hide(&mut self) -> Option<u64> {
        self.generation += 1;
        let generation = self.generation;
        self.show_task = None;
        let request = self.request.take()?;
        self.exiting = Some(request);
        Some(generation)
    }

    pub(super) fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(generation) = self.begin_hide() else {
            return;
        };
        cx.notify();
        self.hide_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(100))
                .await;
            let _ = this.update_in(cx, |this, _, cx| {
                if this.generation == generation {
                    this.exiting = None;
                    this.hide_task = None;
                    cx.notify();
                }
            });
        }));
    }
}

impl Render for TooltipProviderState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (request, phase) = match (&self.request, &self.exiting) {
            (Some(request), _) => (Some(request.clone()), TooltipPhase::Open),
            (None, Some(request)) => (Some(request.clone()), TooltipPhase::Closed),
            (None, None) => (None, TooltipPhase::Closed),
        };
        let Some(request) = request else {
            return div().into_any_element();
        };
        let rem_size = window.rem_size();
        floating_surface(
            Positioner::side(
                request
                    .content
                    .shifted_bounds(request.trigger_bounds, rem_size),
            )
            .placement(request.content.resolved_side())
            .align(request.content.resolved_align())
            .offset(request.content.resolved_side_offset(rem_size))
            .child(request.content.render(phase, window, cx)),
            200,
        )
        .into_any_element()
    }
}

#[derive(Default)]
struct TooltipRegistry(HashMap<WindowId, Entity<TooltipProviderState>>);
impl Global for TooltipRegistry {}

fn should_show_for_focus(is_focused: bool, last_input_was_keyboard: bool) -> bool {
    is_focused && last_input_was_keyboard
}

pub(super) fn register(window: &Window, provider: Entity<TooltipProviderState>, cx: &mut App) {
    if !cx.has_global::<TooltipRegistry>() {
        cx.set_global(TooltipRegistry::default());
    }
    cx.global_mut::<TooltipRegistry>()
        .0
        .insert(window.window_handle().window_id(), provider);
}

fn provider(window: &Window, cx: &App) -> Option<Entity<TooltipProviderState>> {
    cx.try_global::<TooltipRegistry>()?
        .0
        .get(&window.window_handle().window_id())
        .cloned()
}

pub(super) trait AttachTooltip:
    gpui::StatefulInteractiveElement + ElementExt + Sized
{
    fn attach_tooltip(self, content: TooltipContent) -> Self {
        self.attach_tooltip_with_focus(content, None)
    }

    fn attach_tooltip_with_focus(
        self,
        content: TooltipContent,
        focus_handle: Option<FocusHandle>,
    ) -> Self {
        self.attach_tooltip_with_focus_and_hover(content, focus_handle, None)
    }

    fn attach_tooltip_with_focus_and_hover(
        self,
        content: TooltipContent,
        focus_handle: Option<FocusHandle>,
        on_hover: Option<TooltipHoverHandler>,
    ) -> Self {
        let bounds = Rc::new(Cell::new(Bounds::default()));
        let hover_state = Rc::new(Cell::new(false));
        let focus_state = Rc::new(Cell::new(false));
        let bounds_writer = bounds.clone();
        let focus_content = content.clone();
        let focus_hover_state = hover_state.clone();
        let prepaint_focus_state = focus_state.clone();
        self.on_prepaint(move |value, window, cx| {
            bounds_writer.set(value);
            let is_focused = focus_handle
                .as_ref()
                .is_some_and(|handle| handle.is_focused(window));
            let has_visible_focus =
                should_show_for_focus(is_focused, window.last_input_was_keyboard());
            if prepaint_focus_state.replace(has_visible_focus) == has_visible_focus {
                return;
            }
            if let Some(provider) = provider(window, cx) {
                provider.update(cx, |provider, cx| {
                    if has_visible_focus {
                        provider.show(
                            TooltipRequest {
                                trigger_bounds: value,
                                content: focus_content.clone(),
                            },
                            window,
                            cx,
                        );
                    } else if !focus_hover_state.get() {
                        provider.hide(window, cx);
                    }
                });
            }
        })
        .on_hover({
            let bounds = bounds.clone();
            let hover_state = hover_state.clone();
            let focus_state = focus_state.clone();
            move |hovered, window, cx| {
                hover_state.set(*hovered);
                if let Some(on_hover) = &on_hover {
                    on_hover(*hovered, window, cx);
                }
                if let Some(provider) = provider(window, cx) {
                    provider.update(cx, |provider, cx| {
                        if *hovered {
                            provider.show(
                                TooltipRequest {
                                    trigger_bounds: bounds.get(),
                                    content: content.clone(),
                                },
                                window,
                                cx,
                            );
                        } else if !focus_state.get() {
                            provider.hide(window, cx);
                        }
                    });
                }
            }
        })
        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
            if let Some(provider) = provider(window, cx) {
                provider.update(cx, |provider, cx| provider.hide(window, cx));
            }
        })
    }
}

impl<T> AttachTooltip for T where T: gpui::StatefulInteractiveElement + ElementExt {}

#[cfg(test)]
mod tests {
    use gpui::Bounds;

    use super::{TooltipContent, TooltipProviderState, TooltipRequest, should_show_for_focus};

    #[test]
    fn pointer_focus_does_not_open_tooltip() {
        assert!(!should_show_for_focus(true, false));
    }

    #[test]
    fn keyboard_focus_opens_tooltip() {
        assert!(should_show_for_focus(true, true));
    }

    #[test]
    fn dismissing_moves_the_active_request_into_the_exit_phase() {
        let mut provider = TooltipProviderState::new(std::time::Duration::ZERO);
        provider.request = Some(TooltipRequest {
            trigger_bounds: Bounds::default(),
            content: TooltipContent::new("Copy code"),
        });

        assert_eq!(provider.begin_hide(), Some(1));
        assert!(provider.request.is_none());
        assert!(provider.exiting.is_some());
    }
}
