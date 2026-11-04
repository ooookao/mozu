//! Base UI-compatible behavior for the Popover primitive.

use std::rc::Rc;

use gpui::{
    App, Bounds, Context, DismissEvent, EventEmitter, FocusHandle, Focusable, IntoElement, Pixels,
    Render, Subscription, Task, Window, div,
};
use gpui_base::{DeferredPopover, GlobalState, actions::Cancel};

use super::{PopoverSide, motion::MOTION_DURATION};
use crate::theme::motion::MotionPolicy;

pub(super) type OpenChangeHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PopoverPhase {
    Hidden,
    Opening,
    Open,
    Closing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReconciliationState {
    Initial,
    Reconciled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ControlMode {
    Uncontrolled,
    Controlled,
}

pub struct PopoverState {
    pub(super) focus_handle: FocusHandle,
    pub(super) trigger_focus_handle: FocusHandle,
    initial_focus_handle: Option<FocusHandle>,
    previous_focus_handle: Option<FocusHandle>,
    pub(super) phase: PopoverPhase,
    reconciliation: ReconciliationState,
    control_mode: ControlMode,
    generation: usize,
    on_open_change: Option<OpenChangeHandler>,
    dismiss_subscription: Option<Subscription>,
    close_task: Option<Task<()>>,
    deferred_context: Option<DeferredPopover>,
    pub(super) trigger_bounds: Bounds<Pixels>,
    pub(super) trigger_captured: bool,
    pub(super) resolved_side: PopoverSide,
    pub(super) needs_initial_focus: bool,
}

impl PopoverState {
    pub(super) fn new(preferred_side: PopoverSide, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            trigger_focus_handle: cx.focus_handle(),
            initial_focus_handle: None,
            previous_focus_handle: None,
            phase: PopoverPhase::Hidden,
            reconciliation: ReconciliationState::Initial,
            control_mode: ControlMode::Uncontrolled,
            generation: 0,
            on_open_change: None,
            dismiss_subscription: None,
            close_task: None,
            deferred_context: None,
            trigger_bounds: Bounds::default(),
            trigger_captured: false,
            resolved_side: preferred_side,
            needs_initial_focus: false,
        }
    }

    #[must_use]
    pub fn is_open(&self) -> bool {
        matches!(self.phase, PopoverPhase::Opening | PopoverPhase::Open)
    }

    pub(super) fn is_mounted(&self) -> bool {
        self.phase != PopoverPhase::Hidden
    }

    pub(super) fn generation(&self) -> usize {
        self.generation
    }

    pub(super) fn reconcile(
        &mut self,
        controlled_open: Option<bool>,
        default_open: bool,
        on_open_change: Option<OpenChangeHandler>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.control_mode = if controlled_open.is_some() {
            ControlMode::Controlled
        } else {
            ControlMode::Uncontrolled
        };
        self.on_open_change = on_open_change;
        let desired_open = desired_open_for_reconciliation(
            self.reconciliation == ReconciliationState::Reconciled,
            controlled_open,
            default_open,
        );
        self.reconciliation = ReconciliationState::Reconciled;
        if let Some(open) = desired_open {
            self.apply_open(open, window, cx);
        }
    }

    pub(super) fn set_initial_focus(&mut self, focus: Option<FocusHandle>) {
        self.initial_focus_handle = focus;
    }

    pub fn dismiss(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.request_open(false, window, cx);
    }

    pub(super) fn handle_trigger_press(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_mounted() {
            self.previous_focus_handle = Some(self.trigger_focus_handle.clone());
        }
        self.request_open(trigger_press_should_open(self.is_mounted()), window, cx);
    }

    fn request_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(callback) = self.on_open_change.as_ref() {
            callback(open, window, cx);
        }
        if self.control_mode == ControlMode::Uncontrolled {
            self.apply_open(open, window, cx);
        }
    }

    pub(super) fn apply_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        if open == self.is_open() {
            return;
        }
        self.generation += 1;
        self.close_task = None;
        let generation = self.generation;
        let motion_duration = MotionPolicy::global(cx).duration(MOTION_DURATION);
        if open {
            if self.previous_focus_handle.is_none() {
                self.previous_focus_handle = window.focused(cx);
            }
            self.phase = PopoverPhase::Opening;
            self.deferred_context = Some(GlobalState::register_deferred_popover(cx));
            self.needs_initial_focus = true;
            let entity = cx.entity();
            let focus_entity = entity.clone();
            window.on_next_frame(move |window, _cx| {
                let focus_entity = focus_entity.clone();
                window.on_next_frame(move |window, cx| {
                    focus_entity.update(cx, |state, cx| {
                        state.focus_initial(window, cx);
                    });
                });
            });
            let subscription_entity = entity.clone();
            self.dismiss_subscription = Some(window.subscribe(
                &subscription_entity,
                cx,
                move |_, _: &DismissEvent, window, cx| {
                    entity.update(cx, |state, cx| state.dismiss(window, cx));
                },
            ));
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(motion_duration).await;
                let _ = this.update_in(cx, |state, _, cx| {
                    if state.generation == generation && state.phase == PopoverPhase::Opening {
                        state.phase = PopoverPhase::Open;
                        cx.notify();
                    }
                });
            })
            .detach();
        } else {
            self.phase = PopoverPhase::Closing;
            self.needs_initial_focus = false;
            self.dismiss_subscription = None;
            if let Some(previous) = self.previous_focus_handle.take() {
                previous.focus(window, cx);
            }
            self.close_task = Some(cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(motion_duration).await;
                let _ = this.update_in(cx, |state, _, cx| {
                    if state.generation == generation && state.phase == PopoverPhase::Closing {
                        state.phase = PopoverPhase::Hidden;
                        state.deferred_context = None;
                        state.close_task = None;
                        cx.notify();
                    }
                });
            }));
        }
        cx.notify();
    }

    pub(super) fn on_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss(window, cx);
    }

    pub(super) fn focus_initial(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.needs_initial_focus && self.is_open() {
            self.needs_initial_focus = false;
            self.initial_focus_handle
                .as_ref()
                .unwrap_or(&self.focus_handle)
                .focus(window, cx);
        }
    }
}

impl Focusable for PopoverState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl EventEmitter<DismissEvent> for PopoverState {}

const fn trigger_press_should_open(popover_is_mounted: bool) -> bool {
    !popover_is_mounted
}

const fn desired_open_for_reconciliation(
    initialized: bool,
    controlled_open: Option<bool>,
    default_open: bool,
) -> Option<bool> {
    match controlled_open {
        Some(open) => Some(open),
        None if !initialized => Some(default_open),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_press_does_not_reopen_a_closing_popover() {
        assert!(trigger_press_should_open(false));
        assert!(!trigger_press_should_open(true));
    }

    #[test]
    fn reconciliation_applies_default_once_and_controlled_state_always() {
        assert_eq!(
            desired_open_for_reconciliation(false, None, true),
            Some(true)
        );
        assert_eq!(desired_open_for_reconciliation(true, None, true), None);
        assert_eq!(
            desired_open_for_reconciliation(true, Some(false), true),
            Some(false)
        );
    }
}
