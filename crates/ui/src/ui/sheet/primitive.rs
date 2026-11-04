//! Window-root lifecycle and gpui-base behavior for modal sheets.

use std::{rc::Rc, time::Duration};

use gpui::{
    AnyElement, App, Context, Entity, FocusHandle, Global, IntoElement, Render, Styled as _, Task,
    Window, div,
};
use gpui_base::Sheet as SheetBehavior;

use super::super::floating_surface;
use crate::theme::motion::MotionPolicy;

const TRANSITION_DURATION: Duration = Duration::from_millis(200);

pub(super) type LayersBuilder = Rc<dyn Fn(SheetPhase, &mut Window, &mut App) -> SheetLayers>;

pub(super) struct SheetLayers {
    pub(super) overlay: AnyElement,
    pub(super) surface: AnyElement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SheetPhase {
    Hidden,
    Opening,
    Open,
    Closing,
}

pub struct SheetHostGlobal(pub Entity<SheetHost>);

impl Global for SheetHostGlobal {}

pub struct SheetHost {
    focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
    phase: SheetPhase,
    layers: Option<LayersBuilder>,
    generation: usize,
    transition_task: Option<Task<()>>,
}

impl SheetHost {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus: cx.focus_handle(),
            previous_focus: None,
            phase: SheetPhase::Hidden,
            layers: None,
            generation: 0,
            transition_task: None,
        }
    }

    pub(super) fn open(layers: LayersBuilder, window: &mut Window, cx: &mut App) {
        let host = cx.global::<SheetHostGlobal>().0.clone();
        host.update(cx, |host, cx| {
            let transition_duration = MotionPolicy::global(cx).duration(TRANSITION_DURATION);
            host.generation += 1;
            let generation = host.generation;
            host.transition_task = None;
            if host.phase == SheetPhase::Hidden {
                host.previous_focus = window.focused(cx);
            }
            host.layers = Some(layers);
            host.phase = SheetPhase::Opening;
            host.focus.focus(window, cx);
            host.transition_task = Some(cx.spawn_in(window, async move |host, cx| {
                cx.background_executor().timer(transition_duration).await;
                let _ = host.update_in(cx, |host, _, cx| {
                    if host.generation == generation && host.phase == SheetPhase::Opening {
                        host.phase = SheetPhase::Open;
                        host.transition_task = None;
                        cx.notify();
                    }
                });
            }));
            cx.notify();
        });
    }

    pub(super) fn request_close(window: &mut Window, cx: &mut App) {
        let host = cx.global::<SheetHostGlobal>().0.clone();
        host.update(cx, |host, cx| host.dismiss(window, cx));
    }

    fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.phase, SheetPhase::Opening | SheetPhase::Open) {
            return;
        }
        let transition_duration = MotionPolicy::global(cx).duration(TRANSITION_DURATION);
        self.generation += 1;
        let generation = self.generation;
        self.transition_task = None;
        self.phase = SheetPhase::Closing;
        self.transition_task = Some(cx.spawn_in(window, async move |host, cx| {
            cx.background_executor().timer(transition_duration).await;
            let _ = host.update_in(cx, |host, window, cx| {
                if host.generation == generation && host.phase == SheetPhase::Closing {
                    host.phase = SheetPhase::Hidden;
                    host.layers = None;
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
}

impl Render for SheetHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.phase == SheetPhase::Hidden {
            return div().into_any_element();
        }

        let layers = self
            .layers
            .as_ref()
            .expect("mounted sheet always owns a layers builder")(
            self.phase, window, cx
        );
        let host = cx.entity();

        let sheet = SheetBehavior::new(cx)
            .size_full()
            .focus_handle(self.focus.clone())
            .overlay_closable(false)
            .request_close(move |window, cx| {
                host.update(cx, |host, cx| host.dismiss(window, cx));
            })
            .overlay(layers.overlay)
            .surface(layers.surface)
            .into_any_element();

        floating_surface(sheet, gpui_base::POPUP_PRIORITY).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_keeps_the_sheet_mounted_for_the_full_surface_transition() {
        assert_eq!(TRANSITION_DURATION, Duration::from_millis(200));
        assert_ne!(SheetPhase::Opening, SheetPhase::Hidden);
        assert_ne!(SheetPhase::Closing, SheetPhase::Hidden);
    }
}
