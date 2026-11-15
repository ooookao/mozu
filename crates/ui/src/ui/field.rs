use crate::ui::checkbox::Checkbox;
use crate::ui::icon::{Icon, IconName};
use crate::ui::utils::presentation::{ComponentTheme, ControlSize, TextRole, ThemeScheme};
use gpui::{
    App, Div, ElementId, InteractiveElement, ParentElement, Refineable, SharedString, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, px,
};
use kinu_ui::{
    ColorScheme, Field, FieldControl, FieldDescription, FieldError, FieldErrorMatch, FieldLabel,
    FieldParts, Fieldset, FieldsetLegend,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldLayout {
    Vertical,
    LeadingControl,
    TrailingControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegendVariant {
    Legend,
    Label,
}

fn scheme(window: &Window, cx: &App) -> ThemeScheme {
    if ColorScheme::current(window, cx) == ColorScheme::Dark {
        ThemeScheme::Dark
    } else {
        ThemeScheme::Light
    }
}

fn typography(tokens: ComponentTheme, role: TextRole) -> StyleRefinement {
    let metrics = role.metrics(tokens.size, &tokens);
    StyleRefinement::default()
        .font_family(tokens.typography.family.clone())
        .font_weight(tokens.typography.normal.weight)
        .font_variations(tokens.typography.normal.variations())
        .text_size(px(metrics.size))
        .line_height(px(metrics.line_height))
}

fn arrange(parts: FieldParts, layout: FieldLayout, gap: f32) -> Div {
    match layout {
        FieldLayout::Vertical => div()
            .flex()
            .flex_col()
            .w_full()
            .gap(px(4.))
            .children(parts.label)
            .child(parts.control)
            .children(parts.description)
            .children(parts.errors),
        FieldLayout::LeadingControl | FieldLayout::TrailingControl => {
            let has_description = parts.description.is_some();
            let text = div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(px(4.))
                .children(parts.label)
                .children(parts.description)
                .children(parts.errors);
            let mut row = div().flex().w_full().gap(px(gap));
            row = if has_description {
                row.items_start()
            } else {
                row.items_center()
            };
            match layout {
                FieldLayout::LeadingControl => row
                    .child(
                        div()
                            .flex_none()
                            .mt(px(if has_description { 2. } else { 0. }))
                            .child(parts.control),
                    )
                    .child(text),
                FieldLayout::TrailingControl => row.child(text).child(parts.control),
                FieldLayout::Vertical => unreachable!(),
            }
        }
    }
}

/// Arranges a native field's bound parts; validation, labeling, and activation remain native.
pub fn field_with_tokens<C: FieldControl>(
    id: impl Into<ElementId>,
    control: C,
    layout: FieldLayout,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> Field<C> {
    let colors = tokens.extras(scheme(window, cx));
    let gap = tokens.size.metrics(&tokens).gap;
    let mut result =
        Field::new(id, control).layout(move |parts, _, _, _| arrange(parts, layout, gap));
    result.style().refine(
        &typography(tokens.clone(), TextRole::Body)
            .w_full()
            .text_color(gpui::Hsla::from(colors.foreground)),
    );
    result
}

pub fn field_label_with_tokens(
    text: impl Into<SharedString>,
    layout: FieldLayout,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> FieldLabel {
    let colors = tokens.extras(scheme(window, cx));
    let mut label = FieldLabel::new(text).state_style({
        let __theme_colors_destructive = colors.destructive;
        let __theme_colors_foreground = colors.foreground;
        let __theme_colors_muted_foreground = colors.muted_foreground;
        move |state| {
            StyleRefinement::default().text_color(if state.invalid() {
                gpui::Hsla::from(__theme_colors_destructive)
            } else if layout == FieldLayout::Vertical {
                gpui::Hsla::from(__theme_colors_muted_foreground)
            } else {
                gpui::Hsla::from(__theme_colors_foreground)
            })
        }
    });
    label
        .style()
        .refine(&typography(tokens.clone(), TextRole::Body));
    label
}

pub fn field_description_with_tokens(
    text: impl Into<SharedString>,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> FieldDescription {
    let mut description = FieldDescription::new(text);
    description
        .style()
        .refine(
            &typography(tokens.clone(), TextRole::Caption).text_color(gpui::Hsla::from(
                (tokens.extras(scheme(window, cx))).muted_foreground,
            )),
        );
    description
}

fn unique_messages(messages: &[SharedString]) -> Vec<SharedString> {
    let mut unique = Vec::new();
    for message in messages {
        if !message.is_empty() && !unique.contains(message) {
            unique.push(message.clone());
        }
    }
    unique
}

fn error_content(messages: &[SharedString], tokens: ComponentTheme, color: gpui::Hsla) -> Div {
    let unique = unique_messages(messages);
    let icon_size = match tokens.size {
        ControlSize::Default => 14.,
        ControlSize::Compact => 12.,
    };
    div()
        .flex()
        .items_start()
        .gap(px(tokens.size.metrics(&tokens).gap))
        .text_color(color)
        .child(
            Icon::new(IconName::CircleAlert)
                .size(icon_size)
                .stroke_width(1.75)
                .mt(px(1.))
                .flex_none(),
        )
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(4.))
                .children(unique.into_iter().map(|text| div().child(text.clone()))),
        )
}

pub fn field_error_with_tokens(
    matcher: FieldErrorMatch,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> FieldError {
    let color = gpui::Hsla::from((tokens.extras(scheme(window, cx))).destructive);
    let mut error = FieldError::new(matcher)
        .role(gpui::Role::Alert)
        .render_messages({
            let tokens = tokens.clone();
            move |messages, _, _| error_content(messages, tokens.clone(), color)
        });
    error
        .style()
        .refine(&typography(tokens.clone(), TextRole::Caption).text_color(color));
    error
}

/// Presents messages supplied directly by a caller, without running validation.
pub fn field_errors_with_tokens(
    id: impl Into<ElementId>,
    messages: &[SharedString],
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> Option<Stateful<Div>> {
    let messages = unique_messages(messages);
    if messages.is_empty() {
        return None;
    }
    let text: SharedString = messages
        .iter()
        .map(SharedString::as_ref)
        .collect::<Vec<_>>()
        .join("\n")
        .into();
    let mut result = error_content(
        &messages,
        tokens.clone(),
        gpui::Hsla::from((tokens.extras(scheme(window, cx))).destructive),
    )
    .id(id)
    .role(gpui::Role::Alert)
    .aria_value(text);
    result
        .style()
        .refine(&typography(tokens.clone(), TextRole::Caption));
    Some(result)
}

pub fn field_group_with_tokens(tokens: ComponentTheme) -> Div {
    let mut group = div().flex().flex_col().w_full().gap(px(12.));
    group
        .style()
        .refine(&typography(tokens.clone(), TextRole::Body));
    group
}

pub fn fieldset_with_tokens(id: impl Into<ElementId>, tokens: ComponentTheme) -> Fieldset {
    let mut result = Fieldset::new(id).flex().flex_col().min_w_0().gap(px(12.));
    result
        .style()
        .refine(&typography(tokens.clone(), TextRole::Body));
    result
}

pub fn fieldset_legend_with_tokens(
    text: impl Into<SharedString>,
    variant: LegendVariant,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> FieldsetLegend {
    let colors = tokens.extras(scheme(window, cx));
    let role = if variant == LegendVariant::Legend {
        TextRole::Subtitle
    } else {
        TextRole::Body
    };
    let mut legend = FieldsetLegend::new(text).mb(px(4.));
    legend
        .style()
        .refine(&typography(tokens.clone(), role).text_color(
            if variant == LegendVariant::Legend {
                gpui::Hsla::from(colors.foreground)
            } else {
                gpui::Hsla::from(colors.muted_foreground)
            },
        ));
    if variant == LegendVariant::Legend {
        legend = legend
            .font_weight(tokens.typography.semibold.weight)
            .font_variations(tokens.typography.semibold.variations());
    }
    legend
}

pub fn field_separator_with_tokens(
    text: Option<SharedString>,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> Div {
    let colors = tokens.extras(scheme(window, cx));
    let mut separator = div()
        .relative()
        .flex()
        .items_center()
        .h(px(20.))
        .w_full()
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top(px(10.))
                .h(px(1.))
                .bg(gpui::Hsla::from(colors.border)),
        );
    if let Some(text) = text {
        let mut label = div()
            .relative()
            .mx_auto()
            .px(px(8.))
            .bg(gpui::Hsla::from(colors.background))
            .text_color(gpui::Hsla::from(colors.muted_foreground))
            .child(text);
        label
            .style()
            .refine(&typography(tokens.clone(), TextRole::Caption));
        separator = separator.child(label);
    }
    separator
}

/// A labeled choice card delegates its larger activation area and focus target to Field.
pub fn choice_card_with_tokens(
    id: impl Into<ElementId>,
    control: Checkbox,
    tokens: ComponentTheme,
    window: &Window,
    cx: &App,
) -> Field<Checkbox> {
    let colors = tokens.extras(scheme(window, cx));
    let checked = control.field_filled();
    let gap = tokens.size.metrics(&tokens).gap;
    field_with_tokens(
        id,
        control.focus_visible_style(StyleRefinement::default().shadow(Vec::new())),
        FieldLayout::LeadingControl,
        tokens.clone(),
        window,
        cx,
    )
    .present_control(|control, state, presentation| {
        control.group_hovered(presentation.hovered && !state.disabled && !state.invalid())
    })
    .layout({
        let tokens = tokens.clone();
        {
            let __theme_colors_border = colors.border;
            let __theme_colors_brand = colors.brand;
            let __theme_colors_foreground = colors.foreground;
            let __theme_colors_hover = colors.hover();
            move |parts, _, _, _| {
                let focused = parts.presentation.focus_visible;
                let hovered = parts.presentation.hovered;
                let card = parts
                    .label_container(div())
                    .flex()
                    .w_full()
                    .p(px(tokens.size.metrics(&tokens).horizontal_padding))
                    .rounded(px(tokens.shape.radii(&tokens).container))
                    .border_1()
                    .border_color(if checked {
                        gpui::Hsla::from(__theme_colors_foreground).opacity(0.4)
                    } else {
                        gpui::Hsla::from(__theme_colors_border)
                    })
                    .bg(if hovered {
                        __theme_colors_hover
                    } else {
                        gpui::transparent_black()
                    })
                    .transition(gpui::StyleTransition::colors(gpui::Animation::new(
                        std::time::Duration::from_millis(
                            tokens.motion.animations.color.duration_ms,
                        ),
                    )))
                    .shadow(if focused {
                        vec![
                            gpui::BoxShadow::new(
                                px(0.),
                                px(0.),
                                gpui::Hsla::from(__theme_colors_brand),
                            )
                            .spread_radius(px(1.)),
                        ]
                    } else {
                        Vec::new()
                    });
                card.child(arrange(parts, FieldLayout::LeadingControl, gap))
            }
        }
    })
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn field<C: FieldControl>(
    id: impl Into<ElementId>,
    control: C,
    layout: FieldLayout,
    window: &Window,
    cx: &App,
) -> Field<C> {
    field_with_tokens(id, control, layout, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn field_label(
    text: impl Into<SharedString>,
    layout: FieldLayout,
    window: &Window,
    cx: &App,
) -> FieldLabel {
    field_label_with_tokens(text, layout, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn field_description(
    text: impl Into<SharedString>,
    window: &Window,
    cx: &App,
) -> FieldDescription {
    field_description_with_tokens(text, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn field_error(matcher: FieldErrorMatch, window: &Window, cx: &App) -> FieldError {
    field_error_with_tokens(matcher, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn field_errors(
    id: impl Into<ElementId>,
    messages: &[SharedString],
    window: &Window,
    cx: &App,
) -> Option<Stateful<Div>> {
    field_errors_with_tokens(id, messages, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn field_group(cx: &App) -> Div {
    field_group_with_tokens(ComponentTheme::current(cx))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn fieldset(id: impl Into<ElementId>, cx: &App) -> Fieldset {
    fieldset_with_tokens(id, ComponentTheme::current(cx))
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn fieldset_legend(
    text: impl Into<SharedString>,
    variant: LegendVariant,
    window: &Window,
    cx: &App,
) -> FieldsetLegend {
    fieldset_legend_with_tokens(text, variant, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn field_separator(text: Option<SharedString>, window: &Window, cx: &App) -> Div {
    field_separator_with_tokens(text, ComponentTheme::current(cx), window, cx)
}

/// Uses the application theme without requiring an explicit presentation configuration.
pub fn choice_card(
    id: impl Into<ElementId>,
    control: Checkbox,
    window: &Window,
    cx: &App,
) -> Field<Checkbox> {
    choice_card_with_tokens(id, control, ComponentTheme::current(cx), window, cx)
}
