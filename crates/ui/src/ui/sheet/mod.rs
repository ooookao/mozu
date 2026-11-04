//! G2V's shadcn-compatible sheet composition.

mod primitive;

use std::rc::Rc;

use gpui::{
    AbsoluteLength, Animation, AnimationExt as _, AnyElement, App, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement as _, Pixels, Rems, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _, px,
};
use gpui_base::{DialogTrigger as SheetTriggerBehavior, TextStyleToken, Theme as BaseTheme};

use crate::theme::{
    border::BORDER_DEFAULT,
    colors::G2VTheme,
    motion::{MotionTokens, elevated_surface_opacity},
    rems_from_design_pixels, rems_from_pixels,
    spacing::{SPACE_0_5, SPACE_3},
    typography,
};

use super::{
    Button, ButtonSize, ButtonVariant, IconName, TextStyleExt as _,
    modal_backdrop::standard_modal_backdrop,
};
use primitive::{LayersBuilder, SheetLayers, SheetPhase};
pub use primitive::{SheetHost, SheetHostGlobal};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SheetSide {
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

type ContentBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;
const MAX_HORIZONTAL_EXTENT: Rems = rems_from_design_pixels(384.0);

#[derive(Clone, Copy)]
struct SheetDismissBehavior {
    show_close_button: bool,
    overlay_closable: bool,
}

#[derive(IntoElement)]
#[must_use]
pub struct Sheet {
    trigger: AnyElement,
    side: SheetSide,
    size: Option<AbsoluteLength>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    body: ContentBuilder,
    footer: Option<ContentBuilder>,
    show_close_button: bool,
    overlay_closable: bool,
}

impl Sheet {
    pub fn new(trigger: impl IntoElement, side: SheetSide) -> Self {
        Self {
            trigger: trigger.into_any_element(),
            side,
            size: None,
            title: None,
            description: None,
            body: Rc::new(|_, _| div().into_any_element()),
            footer: None,
            show_close_button: true,
            overlay_closable: true,
        }
    }

    pub fn size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
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

    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }
}

impl RenderOnce for Sheet {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let trigger = self.trigger;
        let layers = sheet_layers_builder(
            self.side,
            self.size,
            self.title,
            self.description,
            self.body,
            self.footer,
            SheetDismissBehavior {
                show_close_button: self.show_close_button,
                overlay_closable: self.overlay_closable,
            },
        );
        SheetTriggerBehavior::new(trigger).on_open(move |window, cx| {
            primitive::SheetHost::open(layers.clone(), window, cx);
        })
    }
}

fn sheet_layers_builder(
    side: SheetSide,
    panel_size: Option<AbsoluteLength>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    body: ContentBuilder,
    footer: Option<ContentBuilder>,
    dismiss_behavior: SheetDismissBehavior,
) -> LayersBuilder {
    Rc::new(move |phase, window, cx| {
        let theme = G2VTheme::global(cx);
        let base_theme = BaseTheme::global(cx);
        let motion = MotionTokens::global(cx);
        let type_tokens = typography::tokens();
        let viewport = window.viewport_size();
        let rem_size = window.rem_size();
        let extent = panel_size
            .as_ref()
            .map(|size| size.to_pixels(rem_size))
            .or_else(|| default_extent(side, viewport.width, rem_size));
        let header = sheet_header(
            title.clone(),
            description.clone(),
            type_tokens.md,
            type_tokens.sm,
            theme.foreground,
            theme.muted_foreground,
        );
        let surface = div()
            .id("sheet-surface")
            .debug_selector(|| "sheet-surface".into())
            .role(Role::Dialog)
            .when_some(title.clone(), gpui::StatefulInteractiveElement::aria_label)
            .absolute()
            .occlude()
            .flex()
            .flex_col()
            .gap_4()
            .bg(theme.popover)
            .text_style(type_tokens.sm)
            .text_color(theme.popover_foreground)
            .border_color(theme.border)
            .shadow(base_theme.tokens.shadow.lg)
            .child(header)
            .child(div().flex_1().min_h_0().px_4().child(body(window, cx)))
            .when_some(footer.as_ref(), |surface, footer| {
                surface.child(
                    div()
                        .mt_auto()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .child(footer(window, cx)),
                )
            })
            .when(dismiss_behavior.show_close_button, |surface| {
                surface.child(
                    div()
                        .absolute()
                        .top(rems_from_pixels(SPACE_3))
                        .right(rems_from_pixels(SPACE_3))
                        .child(
                            Button::new("sheet-close")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::IconSmall)
                                .accessibility_label("Close")
                                .on_click(|_, window, cx| SheetHost::request_close(window, cx))
                                .icon(IconName::X),
                        ),
                )
            })
            .map(|surface| place_surface(surface, side, extent, viewport))
            .map(|surface| animate_surface(surface, side, extent, viewport, phase, motion));
        let overlay = standard_modal_backdrop(viewport)
            .when(dismiss_behavior.overlay_closable, |overlay| {
                overlay.on_mouse_down(MouseButton::Left, |_, window, cx| {
                    SheetHost::request_close(window, cx);
                })
            })
            .map(|overlay| animate_overlay(overlay, phase, motion));

        SheetLayers { overlay, surface }
    })
}

fn sheet_header(
    title: Option<SharedString>,
    description: Option<SharedString>,
    title_style: TextStyleToken,
    description_style: TextStyleToken,
    foreground: gpui::Hsla,
    muted_foreground: gpui::Hsla,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(rems_from_pixels(SPACE_0_5))
        .p_4()
        .when_some(title, |header, title| {
            header.child(
                div()
                    .text_style(title_style)
                    .text_color(foreground)
                    .child(title),
            )
        })
        .when_some(description, |header, description| {
            header.child(
                div()
                    .text_style(description_style)
                    .text_color(muted_foreground)
                    .child(description),
            )
        })
}

fn default_extent(side: SheetSide, viewport_width: Pixels, rem_size: Pixels) -> Option<Pixels> {
    match side {
        SheetSide::Left | SheetSide::Right => {
            Some((viewport_width * 0.75).min(MAX_HORIZONTAL_EXTENT.to_pixels(rem_size)))
        }
        SheetSide::Top | SheetSide::Bottom => None,
    }
}

fn place_surface(
    surface: gpui::Stateful<gpui::Div>,
    side: SheetSide,
    extent: Option<Pixels>,
    viewport: gpui::Size<Pixels>,
) -> gpui::Stateful<gpui::Div> {
    match side {
        SheetSide::Top => surface
            .top_0()
            .left_0()
            .right_0()
            .when_some(extent, gpui::Styled::h)
            .border_b(BORDER_DEFAULT),
        SheetSide::Right => surface
            .top_0()
            .left(viewport.width - extent.expect("horizontal sheets always have an extent"))
            .h(viewport.height)
            .when_some(extent, gpui::Styled::w)
            .border_l(BORDER_DEFAULT),
        SheetSide::Bottom => surface
            .bottom_0()
            .left_0()
            .right_0()
            .when_some(extent, gpui::Styled::h)
            .border_t(BORDER_DEFAULT),
        SheetSide::Left => surface
            .top_0()
            .left_0()
            .h(viewport.height)
            .when_some(extent, gpui::Styled::w)
            .border_r(BORDER_DEFAULT),
    }
}

fn surface_frame(
    surface: gpui::Stateful<gpui::Div>,
    side: SheetSide,
    extent: Option<Pixels>,
    viewport: gpui::Size<Pixels>,
    delta: f32,
) -> gpui::Stateful<gpui::Div> {
    let offset = px(40.0) * (1.0 - delta);
    match side {
        SheetSide::Top => surface
            .top(-offset)
            .opacity(elevated_surface_opacity(delta)),
        SheetSide::Right => surface
            .left(
                viewport.width - extent.expect("horizontal sheets always have an extent") + offset,
            )
            .opacity(elevated_surface_opacity(delta)),
        SheetSide::Bottom => surface
            .bottom(-offset)
            .opacity(elevated_surface_opacity(delta)),
        SheetSide::Left => surface
            .left(-offset)
            .opacity(elevated_surface_opacity(delta)),
    }
}

fn animate_surface(
    surface: gpui::Stateful<gpui::Div>,
    side: SheetSide,
    extent: Option<Pixels>,
    viewport: gpui::Size<Pixels>,
    phase: SheetPhase,
    motion: MotionTokens,
) -> AnyElement {
    match phase {
        SheetPhase::Opening => surface
            .with_animation(
                "sheet-enter",
                Animation::new(motion.default).with_easing(gpui::ease_in_out),
                move |surface, delta| surface_frame(surface, side, extent, viewport, delta),
            )
            .into_any_element(),
        SheetPhase::Closing => surface
            .with_animation(
                "sheet-exit",
                Animation::new(motion.default).with_easing(gpui::ease_in_out),
                move |surface, delta| surface_frame(surface, side, extent, viewport, 1.0 - delta),
            )
            .into_any_element(),
        SheetPhase::Open | SheetPhase::Hidden => surface.into_any_element(),
    }
}

fn animate_overlay(overlay: gpui::Div, phase: SheetPhase, motion: MotionTokens) -> AnyElement {
    match phase {
        SheetPhase::Opening => overlay
            .with_animation(
                "sheet-overlay-enter",
                Animation::new(motion.fast).with_easing(gpui::ease_in_out),
                gpui::Styled::opacity,
            )
            .into_any_element(),
        SheetPhase::Closing => overlay
            .with_animation(
                "sheet-overlay-exit",
                Animation::new(motion.fast).with_easing(gpui::ease_in_out),
                |overlay, delta| overlay.opacity(1.0 - delta),
            )
            .into_any_element(),
        SheetPhase::Open | SheetPhase::Hidden => overlay.into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_sheet_width_is_responsive_and_capped() {
        assert_eq!(
            default_extent(SheetSide::Right, px(320.0), px(16.0)),
            Some(px(240.0))
        );
        assert_eq!(
            default_extent(SheetSide::Left, px(1024.0), px(16.0)),
            Some(px(384.0))
        );
        assert_eq!(
            default_extent(SheetSide::Left, px(1024.0), px(18.0)),
            Some(px(432.0))
        );
    }

    #[test]
    fn vertical_sheet_uses_content_height() {
        assert_eq!(default_extent(SheetSide::Top, px(1024.0), px(16.0)), None);
        assert_eq!(
            default_extent(SheetSide::Bottom, px(1024.0), px(16.0)),
            None
        );
    }

    #[test]
    fn explicit_extent_accepts_a_rem_length() {
        let sheet = Sheet::new(div(), SheetSide::Right).size(rems_from_design_pixels(400.0));
        assert_eq!(
            sheet
                .size
                .expect("explicit sheet extent")
                .to_pixels(px(18.0)),
            px(450.0)
        );
    }

    #[test]
    fn sheets_are_overlay_closable_by_default() {
        let sheet = Sheet::new(div(), SheetSide::Right);
        assert!(sheet.overlay_closable);
    }
}
