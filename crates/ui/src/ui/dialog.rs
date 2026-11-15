use crate::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::ui::icon::{Icon, IconName};
use crate::ui::utils::presentation::{
    ComponentTheme, ControlSize, SurfaceLevel, TextRole, ThemeScheme,
};
use gpui::{
    App, Div, ElementId, ParentElement, SharedString, Styled, Window, div, prelude::FluentBuilder,
    px,
};
use kinu_ui::{ColorScheme, Dialog, PopupActions, PopupBackdrop, PopupFade, PopupMotion};

#[derive(Clone, Copy, PartialEq)]
pub enum DialogSize {
    Small,
    Large,
    ExtraLarge,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DialogPosition {
    Center,
    Top,
}

/// Editable presentation values; modal state and interactions remain native.
#[derive(Clone)]
pub struct DialogStyle {
    pub tokens: ComponentTheme,
    pub size: DialogSize,
    pub surface: SurfaceLevel,
    pub position: DialogPosition,
    pub show_close_button: bool,
    /// Applied after the shared panel styling, for edge-to-edge compositions.
    pub panel: gpui::StyleRefinement,
}
impl DialogStyle {
    /// Reads presentation settings from the application's shared theme.
    pub fn current(cx: &gpui::App) -> Self {
        Self::new(ComponentTheme::current(cx))
    }
    pub fn new(tokens: ComponentTheme) -> Self {
        Self {
            tokens: tokens.clone(),
            size: DialogSize::Small,
            surface: SurfaceLevel::Five,
            position: DialogPosition::Center,
            show_close_button: true,
            panel: gpui::StyleRefinement::default(),
        }
    }
}

fn maximum_width(size: DialogSize, step: ControlSize) -> f32 {
    match (size, step) {
        (DialogSize::Small, ControlSize::Default) => 400.,
        (DialogSize::Small, ControlSize::Compact) => 360.,
        (DialogSize::Large, ControlSize::Default) => 540.,
        (DialogSize::Large, ControlSize::Compact) => 480.,
        (DialogSize::ExtraLarge, ControlSize::Default) => 880.,
        (DialogSize::ExtraLarge, ControlSize::Compact) => 800.,
    }
}

/// Resolves the shared dialog width ladder inside the viewport's side gutters.
pub fn dialog_width(
    size: DialogSize,
    step: ControlSize,
    viewport_width: gpui::Pixels,
) -> gpui::Pixels {
    px(maximum_width(size, step)).min((viewport_width - px(32.)).max(px(0.)))
}

fn scheme(window: &Window, cx: &App) -> ThemeScheme {
    if ColorScheme::current(window, cx) == ColorScheme::Dark {
        ThemeScheme::Dark
    } else {
        ThemeScheme::Light
    }
}

pub(crate) fn panel(style: DialogStyle, window: &Window, cx: &App) -> Div {
    let tokens = style.tokens.clone();
    let scheme = scheme(window, cx);
    let colors = tokens.extras(scheme);
    let metrics = TextRole::Body.metrics(tokens.size, &tokens);
    div()
        .relative()
        .flex()
        .flex_col()
        .w(dialog_width(
            style.size,
            tokens.size,
            window.viewport_size().width,
        ))
        .p(px(24.))
        .rounded(px(tokens.shape.radii(&tokens).container))
        .bg(style.surface.color(scheme, &tokens))
        .shadow(style.surface.shadows(scheme, &tokens))
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.normal.weight)
        .font_variations(tokens.typography.normal.variations())
        .text_size(px(metrics.size))
        .line_height(px(metrics.line_height))
        .text_color(gpui::Hsla::from(colors.foreground))
}

pub(crate) fn motion(tokens: ComponentTheme) -> PopupMotion {
    let profile = tokens.motion.slow;
    PopupMotion {
        enter: profile.entering().with_easing(move |progress| {
            if progress >= 1. {
                return 1.;
            }
            profile
                .spring
                .step(
                    gpui::SpringState {
                        position: 0.,
                        velocity: 0.,
                    },
                    1.,
                    progress * profile.enter.as_secs_f32(),
                )
                .position
        }),
        exit: profile
            .exiting()
            .with_easing(tokens.motion.curves.in_out.curve()),
        initial_opacity: 0.,
        initial_scale: 0.97,
        enter_offset: px(0.),
        exit_offset: px(0.),
        origin: kinu_ui::PopupTransformOrigin::Aligned,
    }
}

/// Returns the native modal, retaining its controlled state and full interaction API.
/// Supply its accessible name with `title`, and its description with `description`.
pub fn dialog(
    id: impl Into<ElementId>,
    open: bool,
    content: Div,
    style: DialogStyle,
    window: &mut Window,
    cx: &mut App,
) -> Dialog {
    let id = id.into();
    let actions = PopupActions::new();
    let mut panel = panel(style.clone(), window, cx);
    gpui::Refineable::refine(panel.style(), &style.panel);
    panel = panel.child(content);
    if style.show_close_button {
        let close = actions.clone();
        panel = panel.child(
            Button::new((id.clone(), "close"))
                .tokens(style.tokens.clone())
                .variant(ButtonVariant::Ghost)
                .size(ButtonSize::IconSm)
                .accessibility_label("Close")
                .absolute()
                .right(px(12.))
                .top(px(12.))
                .child(Icon::new(IconName::X).size(14.).stroke_width(1.5))
                .on_click(move |_, window, cx| {
                    close
                        .close(window, cx)
                        .expect("mounted dialog close action");
                }),
        );
    }
    let mut root = dialog_shell_with_tokens(id, open, style.tokens.clone(), window, cx)
        .content(panel)
        .actions(actions);
    if style.position == DialogPosition::Top {
        root = root.viewport_anchor(kinu_ui::PopupViewportAnchor {
            viewport: gpui::point(0.5, 0.12),
            surface: gpui::point(0.5, 0.),
        });
    }
    root
}

/// Applies the shared modal transition and backdrop to a native dialog.
/// The caller supplies the prepared content and placement.
pub fn dialog_shell_with_tokens(
    id: impl Into<ElementId>,
    open: bool,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> Dialog {
    let motion = motion(tokens);
    Dialog::new(id, open)
        .backdrop(backdrop(window, cx))
        .backdrop_motion(backdrop_fade(&motion))
        .motion(motion)
}

pub fn dialog_header_with_tokens(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> Div {
    let description: SharedString = description.into();
    let colors = tokens.extras(scheme(window, cx));
    let title_metrics = TextRole::Title.metrics(tokens.size, &tokens);
    let body_metrics = TextRole::Body.metrics(tokens.size, &tokens);
    div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .mb(px(16.))
        .child(
            div()
                .text_size(px(title_metrics.size))
                .line_height(px(title_metrics.line_height))
                .font_weight(tokens.typography.semibold.weight)
                .font_variations(tokens.typography.semibold.variations())
                .child(title.into()),
        )
        .when(!description.is_empty(), |header| {
            header.child(
                div()
                    .text_size(px(body_metrics.size))
                    .line_height(px(body_metrics.line_height))
                    .text_color(gpui::Hsla::from(colors.muted_foreground))
                    .child(description),
            )
        })
}

pub fn dialog_footer() -> Div {
    div().flex().justify_end().gap(px(8.)).mt(px(24.))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn dialog_shell(id: impl Into<ElementId>, open: bool, window: &Window, cx: &App) -> Dialog {
    dialog_shell_with_tokens(id, open, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn dialog_header(
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    window: &Window,
    cx: &App,
) -> Div {
    dialog_header_with_tokens(title, description, ComponentTheme::current(cx), window, cx)
}

/// Resolves the common modal scrim independently of modal interaction rules.
pub(crate) fn backdrop(window: &Window, cx: &App) -> PopupBackdrop {
    PopupBackdrop {
        color: gpui::black().opacity(match scheme(window, cx) {
            ThemeScheme::Light => 0.4,
            ThemeScheme::Dark => 0.8,
        }),
        blur_radius: px(0.),
    }
}

/// Shares the modal backdrop choreography with alert surfaces.
pub(crate) fn backdrop_fade(motion: &PopupMotion) -> PopupFade {
    PopupFade {
        enter: motion.enter.clone(),
        exit: motion.exit.clone(),
        initial_opacity: 0.,
    }
}
