//! G2V's canonical native compound input primitive.

use gpui::{
    AnyElement, App, ElementId, Entity, FocusHandle, Focusable as _, InteractiveElement as _,
    IntoElement, ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, div, prelude::FluentBuilder as _,
};
use gpui_base::input::{InputState, TextareaState};

use crate::theme::border::BORDER_DEFAULT;
use crate::theme::colors::G2VTheme;
use crate::theme::radius::{RADIUS_2XL, RADIUS_MD};
use crate::theme::shadows;
use crate::theme::sizing::{SIZE_7, SIZE_8};
use crate::theme::typography;

use super::input::{input_background, input_border};
use super::{Input, InputSize, TextStyleExt as _, Textarea};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InputGroupAddonAlign {
    #[default]
    InlineStart,
    InlineEnd,
    BlockStart,
    BlockEnd,
}

#[derive(IntoElement)]
#[must_use]
pub struct InputGroupAddon {
    align: InputGroupAddonAlign,
    content: AnyElement,
    composer: bool,
    inline_padding: Option<(gpui::Rems, gpui::Rems)>,
}

impl InputGroupAddon {
    pub fn new(align: InputGroupAddonAlign, content: impl IntoElement) -> Self {
        Self {
            align,
            content: content.into_any_element(),
            composer: false,
            inline_padding: None,
        }
    }

    pub fn text(align: InputGroupAddonAlign, text: impl Into<SharedString>) -> Self {
        Self::new(align, text.into())
    }

    pub fn composer(mut self) -> Self {
        self.composer = true;
        self
    }

    pub fn inline_padding(mut self, start: gpui::Rems, end: gpui::Rems) -> Self {
        self.inline_padding = Some((start, end));
        self
    }

    fn is_block(&self) -> bool {
        matches!(
            self.align,
            InputGroupAddonAlign::BlockStart | InputGroupAddonAlign::BlockEnd
        )
    }
}

impl RenderOnce for InputGroupAddon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx);
        let typography = typography::tokens();
        let block = self.is_block();

        div()
            .flex()
            .items_center()
            .gap_1p5()
            .text_style(typography.sm)
            .text_color(theme.muted_foreground)
            .when(block && !self.composer, |addon| addon.w_full().px_2p5())
            .when(block && self.composer, |addon| addon.w_full().px_2())
            .when(!block, |addon| match self.inline_padding {
                Some((start, end)) => addon.pl(start).pr(end),
                None => addon.px_2(),
            })
            .when(self.align == InputGroupAddonAlign::BlockStart, |addon| {
                addon.pt_2()
            })
            .when(
                self.align == InputGroupAddonAlign::BlockEnd && !self.composer,
                gpui::Styled::pb_2,
            )
            .when(
                self.align == InputGroupAddonAlign::BlockEnd && self.composer,
                |addon| addon.pt(gpui::Pixels::ZERO).pb_2(),
            )
            .child(self.content)
    }
}

enum InputGroupControl {
    Input(Entity<InputState>),
    Textarea(Entity<TextareaState>),
}

impl InputGroupControl {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self {
            Self::Input(state) => state.focus_handle(cx),
            Self::Textarea(state) => state.focus_handle(cx),
        }
    }

    fn render(
        &self,
        variant: InputGroupVariant,
        size: InputGroupSize,
        disabled: bool,
        readonly: bool,
        invalid: bool,
        accessibility_label: Option<SharedString>,
    ) -> AnyElement {
        match self {
            Self::Input(state) => Input::grouped(state)
                .size(match size {
                    InputGroupSize::Default => InputSize::Default,
                    InputGroupSize::Small => InputSize::Small,
                })
                .disabled(disabled)
                .readonly(readonly)
                .invalid(invalid)
                .when_some(accessibility_label, |input, label| {
                    input.accessibility_label(label)
                })
                .into_any_element(),
            Self::Textarea(state) => if variant == InputGroupVariant::Composer {
                Textarea::composer(state)
            } else {
                Textarea::grouped(state)
            }
            .disabled(disabled)
            .readonly(readonly)
            .invalid(invalid)
            .when_some(accessibility_label, |textarea, label| {
                textarea.accessibility_label(label)
            })
            .into_any_element(),
        }
    }
}

#[derive(IntoElement)]
#[must_use]
pub struct InputGroup {
    id: ElementId,
    control: InputGroupControl,
    addons: Vec<InputGroupAddon>,
    disabled: bool,
    readonly: bool,
    invalid: bool,
    accessibility_label: Option<SharedString>,
    variant: InputGroupVariant,
    size: InputGroupSize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum InputGroupVariant {
    #[default]
    Default,
    Command,
    CommandIntegrated,
    Composer,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum InputGroupSize {
    #[default]
    Default,
    Small,
}

impl InputGroupSize {
    const fn height(self) -> gpui::Rems {
        match self {
            Self::Default => SIZE_8,
            Self::Small => SIZE_7,
        }
    }
}

impl InputGroup {
    pub fn input(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self::new(id.into(), InputGroupControl::Input(state.clone()))
    }

    pub fn textarea(id: impl Into<ElementId>, state: &Entity<TextareaState>) -> Self {
        Self::new(id.into(), InputGroupControl::Textarea(state.clone()))
    }

    pub(crate) fn command(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self {
            variant: InputGroupVariant::Command,
            ..Self::new(id.into(), InputGroupControl::Input(state.clone()))
        }
    }

    pub(crate) fn command_integrated(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self {
            variant: InputGroupVariant::CommandIntegrated,
            ..Self::new(id.into(), InputGroupControl::Input(state.clone()))
        }
    }

    pub fn composer(id: impl Into<ElementId>, state: &Entity<TextareaState>) -> Self {
        Self {
            variant: InputGroupVariant::Composer,
            ..Self::new(id.into(), InputGroupControl::Textarea(state.clone()))
        }
    }

    fn new(id: ElementId, control: InputGroupControl) -> Self {
        Self {
            id,
            control,
            addons: Vec::new(),
            disabled: false,
            readonly: false,
            invalid: false,
            accessibility_label: None,
            variant: InputGroupVariant::Default,
            size: InputGroupSize::Default,
        }
    }

    pub fn addon(mut self, addon: InputGroupAddon) -> Self {
        self.addons.push(addon);
        self
    }

    pub(crate) fn size(mut self, size: InputGroupSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }
}

impl RenderOnce for InputGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = G2VTheme::global(cx).clone();
        let is_dark = matches!(
            window.appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        );
        let focus_handle = self.control.focus_handle(cx);
        let focused = focus_handle.is_focused(window) && !self.disabled;
        let border = match self.variant {
            InputGroupVariant::Default => input_border(&theme, is_dark, focused, self.invalid),
            InputGroupVariant::Command if focused => theme.ring,
            InputGroupVariant::Command => theme.input.opacity(0.3),
            InputGroupVariant::CommandIntegrated => theme.popover.opacity(0.0),
            InputGroupVariant::Composer => theme.input,
        };
        let background = match self.variant {
            InputGroupVariant::Default => input_background(&theme, is_dark, self.disabled),
            InputGroupVariant::Command => theme.input.opacity(0.3),
            InputGroupVariant::CommandIntegrated => theme.popover,
            InputGroupVariant::Composer => theme.composer,
        };
        let has_block_addon = self.addons.iter().any(InputGroupAddon::is_block);
        let control = self.control.render(
            self.variant,
            self.size,
            self.disabled,
            self.readonly,
            self.invalid,
            self.accessibility_label,
        );
        let mut inline_start = Vec::new();
        let mut inline_end = Vec::new();
        let mut block_start = Vec::new();
        let mut block_end = Vec::new();

        for addon in self.addons {
            match addon.align {
                InputGroupAddonAlign::InlineStart => inline_start.push(addon),
                InputGroupAddonAlign::InlineEnd => inline_end.push(addon),
                InputGroupAddonAlign::BlockStart => block_start.push(addon),
                InputGroupAddonAlign::BlockEnd => block_end.push(addon),
            }
        }

        let focus_on_click = focus_handle.clone();
        div()
            .id(self.id)
            .relative()
            .w_full()
            .min_w_0()
            .flex()
            .when(has_block_addon, gpui::Styled::flex_col)
            .when(!has_block_addon, |group| {
                group.items_center().h(self.size.height())
            })
            .rounded(match self.variant {
                InputGroupVariant::Composer => RADIUS_2XL,
                InputGroupVariant::CommandIntegrated => gpui::Pixels::ZERO,
                InputGroupVariant::Default | InputGroupVariant::Command => RADIUS_MD,
            })
            .border(BORDER_DEFAULT)
            .border_color(border)
            .bg(background)
            .when(self.variant == InputGroupVariant::Composer, |group| {
                group
                    .items_end()
                    .gap_1p5()
                    .shadow(shadows::tokens(window.appearance()).sm)
            })
            .when(
                self.disabled && self.variant != InputGroupVariant::Composer,
                |group| group.opacity(0.5),
            )
            .on_click(move |_, window, cx| focus_on_click.focus(window, cx))
            .children(block_start)
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .children(inline_start)
                    .child(control)
                    .children(inline_end),
            )
            .children(block_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    #[test]
    fn dimensions_match_the_react_input_group_contract() {
        assert_eq!(SIZE_8.to_pixels(px(16.0)), px(32.0));
        assert_eq!(RADIUS_MD, px(10.56));
        assert_eq!(BORDER_DEFAULT, px(0.5));
    }

    #[test]
    fn block_alignment_is_explicit() {
        assert!(InputGroupAddon::text(InputGroupAddonAlign::BlockStart, "Label").is_block());
        assert!(!InputGroupAddon::text(InputGroupAddonAlign::InlineEnd, "Label").is_block());
    }
}
