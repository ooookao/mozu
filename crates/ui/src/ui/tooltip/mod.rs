//! G2V's native counterpart to the shared shadcn tooltip compound primitive.

mod provider;

use std::{rc::Rc, time::Duration};

use gpui::{
    AbsoluteLength, Animation, AnimationExt as _, AnyElement, AnyView, App, AppContext as _,
    Bounds, Context, Entity, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, Rems, Render, SharedString, Styled as _, Window, div, ease_in_out,
    prelude::FluentBuilder as _,
};
use gpui_base::{Align, Placement, Tooltip as TooltipPopup};

use crate::theme::motion::{
    FLOATING_SURFACE_TRANSITION_DURATION, FLOATING_SURFACE_TRANSITION_OFFSET,
    floating_surface_visibility,
};
use crate::theme::radius::RADIUS_MD;
use crate::theme::rems_from_design_pixels;
use crate::theme::sizing::{MaxWidth, SIZE_4};
use crate::theme::{border::BORDER_DEFAULT, colors::G2VTheme};
use provider::{AttachTooltip as _, TooltipProviderState};

const DEFAULT_SIDE_OFFSET: Rems = rems_from_design_pixels(4.0);
type TooltipElementBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
pub type TooltipHoverHandler = Rc<dyn Fn(bool, &mut Window, &mut App)>;

fn tooltip_colors(theme: &G2VTheme) -> (Hsla, Hsla) {
    (theme.popover, theme.popover_foreground)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TooltipSide {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl TooltipSide {
    const fn placement(self) -> Placement {
        match self {
            Self::Top => Placement::Top,
            Self::Bottom => Placement::Bottom,
            Self::Left => Placement::Left,
            Self::Right => Placement::Right,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TooltipAlign {
    Start,
    #[default]
    Center,
    End,
}

impl TooltipAlign {
    const fn positioner_align(self) -> Align {
        match self {
            Self::Start => Align::Start,
            Self::Center => Align::Center,
            Self::End => Align::End,
        }
    }
}

#[derive(Clone)]
pub struct TooltipContent {
    body: TooltipBody,
    side: TooltipSide,
    side_offset: AbsoluteLength,
    align: TooltipAlign,
    align_offset: AbsoluteLength,
}

#[derive(Clone)]
enum TooltipBody {
    Text(SharedString),
    Element(TooltipElementBuilder),
}

impl TooltipContent {
    #[must_use]
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            body: TooltipBody::Text(text.into()),
            side: TooltipSide::Top,
            side_offset: DEFAULT_SIDE_OFFSET.into(),
            align: TooltipAlign::Center,
            align_offset: Rems::ZERO.into(),
        }
    }

    #[must_use]
    pub fn element<E>(build: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        Self {
            body: TooltipBody::Element(Rc::new(move |window, cx| {
                build(window, cx).into_any_element()
            })),
            side: TooltipSide::Top,
            side_offset: DEFAULT_SIDE_OFFSET.into(),
            align: TooltipAlign::Center,
            align_offset: Rems::ZERO.into(),
        }
    }

    #[must_use]
    pub fn side(mut self, side: TooltipSide) -> Self {
        self.side = side;
        self
    }

    #[must_use]
    pub fn side_offset(mut self, offset: impl Into<AbsoluteLength>) -> Self {
        self.side_offset = offset.into();
        self
    }

    #[must_use]
    pub fn align(mut self, align: TooltipAlign) -> Self {
        self.align = align;
        self
    }

    #[must_use]
    pub fn align_offset(mut self, offset: impl Into<AbsoluteLength>) -> Self {
        self.align_offset = offset.into();
        self
    }

    pub(super) const fn resolved_side(&self) -> Placement {
        self.side.placement()
    }

    pub(super) const fn resolved_align(&self) -> Align {
        self.align.positioner_align()
    }

    pub(super) fn resolved_side_offset(&self, rem_size: Pixels) -> Pixels {
        self.side_offset.to_pixels(rem_size)
    }

    pub(super) fn shifted_bounds(
        &self,
        mut bounds: Bounds<Pixels>,
        rem_size: Pixels,
    ) -> Bounds<Pixels> {
        let align_offset = self.align_offset.to_pixels(rem_size);
        match self.side {
            TooltipSide::Top | TooltipSide::Bottom => bounds.origin.x += align_offset,
            TooltipSide::Left | TooltipSide::Right => bounds.origin.y += align_offset,
        }
        bounds
    }

    pub(super) fn render(
        &self,
        phase: TooltipPhase,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let (background, foreground, border) = {
            let theme = G2VTheme::global(cx);
            let (background, foreground) = tooltip_colors(theme);
            (background, foreground, theme.popover_border)
        };
        let side = self.side;
        let body = match &self.body {
            TooltipBody::Text(text) => div().child(text.clone()).into_any_element(),
            TooltipBody::Element(build) => build(window, cx),
        };
        TooltipPopup::new("tooltip-content")
            .max_w(MaxWidth::EXTRA_SMALL)
            .rounded(RADIUS_MD)
            .border(BORDER_DEFAULT)
            .border_color(border)
            .bg(background)
            .text_color(foreground)
            .text_xs()
            .line_height(SIZE_4)
            .px_3()
            .py_1p5()
            .child(body)
            .with_animation(
                match phase {
                    TooltipPhase::Open => "tooltip-enter",
                    TooltipPhase::Closed => "tooltip-exit",
                },
                Animation::new(FLOATING_SURFACE_TRANSITION_DURATION).with_easing(ease_in_out),
                move |tooltip, delta| {
                    let progress =
                        floating_surface_visibility(matches!(phase, TooltipPhase::Closed), delta);
                    let distance = FLOATING_SURFACE_TRANSITION_OFFSET * (1.0 - progress);
                    tooltip
                        .relative()
                        .opacity(progress)
                        .map(|tooltip| match side {
                            TooltipSide::Top => tooltip.top(distance),
                            TooltipSide::Bottom => tooltip.top(-distance),
                            TooltipSide::Left => tooltip.left(distance),
                            TooltipSide::Right => tooltip.left(-distance),
                        })
                },
            )
            .into_any_element()
    }
}

#[derive(Clone, Copy)]
pub(super) enum TooltipPhase {
    Open,
    Closed,
}

pub struct Tooltip {
    content: TooltipContent,
}

impl Tooltip {
    #[must_use]
    pub fn new(content: TooltipContent) -> Self {
        Self { content }
    }
}

pub struct TooltipProvider {
    child: AnyView,
    overlay: Entity<TooltipProviderState>,
}

impl TooltipProvider {
    #[must_use]
    pub fn new(child: AnyView, delay: Duration, window: &Window, cx: &mut App) -> Self {
        let overlay = cx.new(|_| TooltipProviderState::new(delay));
        provider::register(window, overlay.clone(), cx);
        Self { child, overlay }
    }
}

impl Render for TooltipProvider {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let overlay = self.overlay.clone();
        div()
            .size_full()
            .relative()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                overlay.update(cx, |overlay, cx| overlay.hide(window, cx));
            })
            .child(self.child.clone())
            .child(self.overlay.clone())
    }
}

pub trait TooltipTrigger: gpui::StatefulInteractiveElement + gpui_base::ElementExt + Sized {
    #[must_use]
    fn with_tooltip(self, tooltip: impl Into<SharedString>) -> Self {
        self.attach_tooltip(TooltipContent::new(tooltip))
    }

    #[must_use]
    fn with_tooltip_content(self, tooltip: Tooltip) -> Self {
        self.attach_tooltip(tooltip.content)
    }

    #[must_use]
    fn with_tooltip_content_and_focus(self, tooltip: Tooltip, focus: gpui::FocusHandle) -> Self {
        self.attach_tooltip_with_focus(tooltip.content, Some(focus))
    }

    #[must_use]
    fn with_tooltip_focus(
        self,
        tooltip: impl Into<SharedString>,
        focus: gpui::FocusHandle,
    ) -> Self {
        self.attach_tooltip_with_focus(TooltipContent::new(tooltip), Some(focus))
    }

    #[must_use]
    fn with_tooltip_focus_and_hover(
        self,
        tooltip: impl Into<SharedString>,
        focus: gpui::FocusHandle,
        on_hover: TooltipHoverHandler,
    ) -> Self {
        self.attach_tooltip_with_focus_and_hover(
            TooltipContent::new(tooltip),
            Some(focus),
            Some(on_hover),
        )
    }
}

impl<T> TooltipTrigger for T where T: gpui::StatefulInteractiveElement + gpui_base::ElementExt {}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{WindowAppearance, point, px, size};

    #[test]
    fn content_defaults_match_the_react_contract() {
        let content = TooltipContent::new("Tooltip");
        assert_eq!(content.side, TooltipSide::Top);
        assert_eq!(content.side_offset.to_pixels(px(16.0)), px(4.0));
        assert_eq!(content.side_offset.to_pixels(px(18.0)), px(4.5));
        assert_eq!(content.align, TooltipAlign::Center);
        assert_eq!(content.align_offset.to_pixels(px(16.0)), px(0.0));
    }

    #[test]
    fn colors_match_the_popover_surface_in_both_themes() {
        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            let theme = G2VTheme::for_appearance(appearance);
            let (background, foreground) = tooltip_colors(&theme);
            assert_eq!(background, theme.popover);
            assert_eq!(foreground, theme.popover_foreground);
        }
    }

    #[test]
    fn align_offset_shifts_the_trigger_on_the_alignment_axis() {
        let bounds = Bounds::new(point(px(10.0), px(20.0)), size(px(40.0), px(20.0)));
        let vertical = TooltipContent::new("Tooltip")
            .side(TooltipSide::Top)
            .align_offset(rems_from_design_pixels(6.0))
            .shifted_bounds(bounds, px(18.0));
        let horizontal = TooltipContent::new("Tooltip")
            .side(TooltipSide::Left)
            .align_offset(rems_from_design_pixels(6.0))
            .shifted_bounds(bounds, px(18.0));
        assert_eq!(vertical.origin.x, px(16.75));
        assert_eq!(vertical.origin.y, px(20.0));
        assert_eq!(horizontal.origin.x, px(10.0));
        assert_eq!(horizontal.origin.y, px(26.75));
    }
}
