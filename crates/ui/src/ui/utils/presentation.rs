use crate::theme::{
    ComponentSettings, ShadowRecipeCache, ShapeSetting, SizeSetting, Theme, ThemePalette,
};
use crate::ui::utils::motion::MotionProfiles;
use gpui::{BoxShadow, FontWeight, Hsla, px};

#[derive(Clone, Copy, PartialEq)]
pub enum ThemeScheme {
    Light,
    Dark,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ControlSize {
    Default,
    Compact,
}

#[derive(Clone, Copy, PartialEq)]
pub struct ControlMetrics {
    pub control_height: f32,
    pub gap: f32,
    pub horizontal_padding: f32,
    pub text_size: f32,
    pub line_height: f32,
}

impl ControlSize {
    pub fn metrics(self, tokens: &ComponentTheme) -> ControlMetrics {
        let body = TextRole::Body.metrics(self, tokens);
        let metrics = match self {
            Self::Default => tokens.settings.sizes.default,
            Self::Compact => tokens.settings.sizes.compact,
        };
        ControlMetrics {
            control_height: metrics.control_height,
            gap: metrics.gap,
            horizontal_padding: metrics.horizontal_padding,
            text_size: body.size,
            line_height: body.line_height,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct ComponentTheme {
    pub motion: MotionProfiles,
    pub size: ControlSize,
    pub shape: Shape,
    pub typography: Typography,
    pub settings: std::sync::Arc<ComponentSettings>,
    pub light_theme: Theme,
    pub dark_theme: Theme,
}

impl ComponentTheme {
    pub fn resolve(explicit: Option<Self>, cx: &gpui::App) -> Self {
        explicit.unwrap_or_else(|| Self::current(cx))
    }
    /// Resolves both schemes and motion from the application's single theme.
    pub fn current(cx: &gpui::App) -> Self {
        Self::from_palette(ThemePalette::current(cx))
    }
    pub fn from_palette(palette: &ThemePalette) -> Self {
        let settings = palette.components.clone().expect(
            "Component settings are missing; install a component to add them to theme.toml",
        );
        let motion = palette
            .motion
            .as_ref()
            .expect("Motion settings are missing from theme.toml");
        Self {
            motion: MotionProfiles::from_settings(motion),
            size: match settings.size {
                SizeSetting::Default => ControlSize::Default,
                SizeSetting::Compact => ControlSize::Compact,
            },
            shape: match settings.shape {
                ShapeSetting::Rounded => Shape::Rounded,
                ShapeSetting::Pill => Shape::Pill,
                ShapeSetting::Sharp => Shape::Sharp,
            },
            typography: Typography::from_palette(palette, &settings),
            settings,
            light_theme: palette.light.clone(),
            dark_theme: palette.dark.clone(),
        }
    }
    pub fn extras(&self, scheme: ThemeScheme) -> &Theme {
        match scheme {
            ThemeScheme::Light => &self.light_theme,
            ThemeScheme::Dark => &self.dark_theme,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Rounded,
    Pill,
    Sharp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeRadii {
    pub item: f32,
    pub focus: f32,
    pub container: f32,
    pub merged: f32,
}

impl Shape {
    pub fn radii(self, tokens: &ComponentTheme) -> ShapeRadii {
        let value = match self {
            Self::Rounded => tokens.settings.shapes.rounded,
            Self::Pill => tokens.settings.shapes.pill,
            Self::Sharp => tokens.settings.shapes.sharp,
        };
        ShapeRadii {
            item: value.item,
            focus: value.focus,
            container: value.container,
            merged: value.merged,
        }
    }

    /// Checkbox squares keep their own size-dependent geometry across standalone and grouped controls.
    pub fn checkbox_radius(self, size: ControlSize) -> f32 {
        match (self, size) {
            (Self::Sharp, _) => 0.,
            (_, ControlSize::Compact) => 4.,
            (_, ControlSize::Default) => 5.,
        }
    }

    pub fn popup(self) -> Self {
        match self {
            Self::Sharp => Self::Sharp,
            Self::Rounded | Self::Pill => Self::Rounded,
        }
    }
}

pub fn nested_radius(outer: f32, inset: f32, border: f32) -> f32 {
    (outer - inset - border).max(0.)
}

#[derive(Clone, Copy, PartialEq)]
pub struct FontWeightStyle {
    pub weight: FontWeight,
    pub optical_size: f32,
}

#[derive(Clone, PartialEq)]
pub struct Typography {
    pub family: gpui::SharedString,
    pub normal: FontWeightStyle,
    pub semibold: FontWeightStyle,
    pub bold: FontWeightStyle,
}

impl Typography {
    fn from_palette(palette: &ThemePalette, settings: &ComponentSettings) -> Self {
        let weights = settings.font_weights;
        Self {
            family: palette.light.font_sans.clone(),
            normal: FontWeightStyle {
                weight: FontWeight(weights.normal.weight),
                optical_size: weights.normal.optical_size,
            },
            semibold: FontWeightStyle {
                weight: FontWeight(weights.semibold.weight),
                optical_size: weights.semibold.optical_size,
            },
            bold: FontWeightStyle {
                weight: FontWeight(weights.bold.weight),
                optical_size: weights.bold.optical_size,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfaceLevel {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
}

impl SurfaceLevel {
    /// Reads the visual substrate; an unscoped component sits on the page surface.
    pub fn current(window: &gpui::Window) -> Self {
        window
            .element_context::<Self>()
            .copied()
            .unwrap_or(Self::One)
    }

    /// Moves up the bounded surface ladder without changing theme ownership.
    pub fn elevated(self, offset: u8) -> Self {
        match (self as u8).saturating_add(offset).min(7) {
            0 => Self::One,
            1 => Self::Two,
            2 => Self::Three,
            3 => Self::Four,
            4 => Self::Five,
            5 => Self::Six,
            6 => Self::Seven,
            7 => Self::Eight,
            _ => unreachable!("Surface level is clamped to the eight-step ladder"),
        }
    }

    /// Provides only a rendering level to descendants, including cached and deferred views.
    pub fn scope(self, child: impl gpui::IntoElement) -> impl gpui::IntoElement {
        gpui::ElementContextScope::new(self, child)
    }

    /// Reads an explicit surface color, or the shared page background when unscoped.
    pub fn inherited_color(
        window: &gpui::Window,
        scheme: ThemeScheme,
        tokens: &ComponentTheme,
    ) -> Hsla {
        match window.element_context::<Self>() {
            Some(surface) => surface.color(scheme, tokens),
            None => gpui::Hsla::from((tokens.extras(scheme)).background),
        }
    }

    pub fn color(self, scheme: ThemeScheme, tokens: &ComponentTheme) -> Hsla {
        let surfaces = tokens.extras(scheme).surfaces;
        match self {
            Self::One => surfaces.one,
            Self::Two => surfaces.two,
            Self::Three => surfaces.three,
            Self::Four => surfaces.four,
            Self::Five => surfaces.five,
            Self::Six => surfaces.six,
            Self::Seven => surfaces.seven,
            Self::Eight => surfaces.eight,
        }
        .into()
    }
    pub fn shadows(self, scheme: ThemeScheme, tokens: &ComponentTheme) -> Vec<BoxShadow> {
        let shadows = &tokens.extras(scheme).shadows;
        let recipe = match self {
            Self::One => &shadows.one,
            Self::Two => &shadows.two,
            Self::Three => &shadows.three,
            Self::Four => &shadows.four,
            Self::Five => &shadows.five,
            Self::Six => &shadows.six,
            Self::Seven => &shadows.seven,
            Self::Eight => &shadows.eight,
        };
        let mut cache = recipe
            .runtime
            .lock()
            .expect("Shadow recipe cache lock poisoned");
        if !cache
            .as_ref()
            .is_some_and(|cached| cached.source == recipe.parts)
        {
            let shadows = recipe
                .parts
                .iter()
                .map(|part| {
                    let shadow = BoxShadow::new(px(part.x), px(part.y), part.color.into())
                        .blur_radius(px(part.blur))
                        .spread_radius(px(part.spread));
                    if part.inset { shadow.inset() } else { shadow }
                })
                .collect();
            *cache = Some(ShadowRecipeCache {
                source: recipe.parts.clone(),
                shadows,
            });
        }
        cache
            .as_ref()
            .expect("Shadow recipe cache initialized above")
            .shadows
            .clone()
    }
}

impl FontWeightStyle {
    pub fn variations(self) -> gpui::FontVariations {
        gpui::FontVariations::new([
            (gpui::FontAxis::WEIGHT, self.weight.0),
            (gpui::FontAxis::OPTICAL_SIZE, self.optical_size),
        ])
        .expect("Flow font coordinates must be finite and use distinct axes")
    }
}

/// Registers the exact Inter variable font used by Flow before any Flow views are drawn.
pub fn install_fonts(cx: &mut gpui::App) -> gpui::Result<()> {
    cx.text_system()
        .add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!(
            "assets/InterVariable.ttf"
        ))])
}

#[derive(Clone, Copy, PartialEq)]
pub enum TextRole {
    Display,
    Title,
    Subtitle,
    Body,
    Caption,
    Micro,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypographyMetrics {
    pub size: f32,
    pub line_height: f32,
}

impl TextRole {
    pub fn metrics(self, size: ControlSize, tokens: &ComponentTheme) -> TypographyMetrics {
        let step = match size {
            ControlSize::Default => tokens.settings.type_scale.default,
            ControlSize::Compact => tokens.settings.type_scale.compact,
        };
        let metrics = match self {
            Self::Display => step.display,
            Self::Title => step.title,
            Self::Subtitle => step.subtitle,
            Self::Body => step.body,
            Self::Caption => step.caption,
            Self::Micro => step.micro,
        };
        TypographyMetrics {
            size: metrics.size,
            line_height: metrics.line_height,
        }
    }
}

/// Font measurements for trimming the inherited label's first and last line boxes.
pub struct LabelMetrics {
    pub line_height: gpui::Pixels,
    pub baseline: gpui::Pixels,
    pub cap_height: gpui::Pixels,
}

/// Shares the OpenType tabular-digit feature between measurement and rendering.
pub fn tabular_numbers() -> gpui::FontFeatures {
    static FEATURES: std::sync::OnceLock<gpui::FontFeatures> = std::sync::OnceLock::new();
    FEATURES
        .get_or_init(|| gpui::FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])))
        .clone()
}
/// Measures the actual shaped label, including font fallback and variable axes.
pub fn label_metrics(label: &str, window: &gpui::Window) -> LabelMetrics {
    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let line_height = style.line_height_in_pixels(window.rem_size());
    let system = window.text_system();
    let font = system.resolve_font(&style.font());
    let layout = system.layout_line(label, font_size, &[style.to_run(label.len())], None);
    LabelMetrics {
        line_height,
        baseline: (line_height - layout.ascent - layout.descent) / 2. + layout.ascent,
        cap_height: system.cap_height(font, font_size),
    }
}

/// Trims a single inherited line box to its cap height and alphabetic baseline.
#[derive(gpui::IntoElement)]
pub struct CapTrimmed {
    label: gpui::SharedString,
    content: gpui::AnyElement,
}

impl CapTrimmed {
    pub fn new(label: impl Into<gpui::SharedString>, content: impl gpui::IntoElement) -> Self {
        Self {
            label: label.into(),
            content: content.into_any_element(),
        }
    }
}

impl gpui::RenderOnce for CapTrimmed {
    fn render(self, window: &mut gpui::Window, _: &mut gpui::App) -> impl gpui::IntoElement {
        use gpui::{ParentElement, Styled};
        let metrics = label_metrics(&self.label, window);
        gpui::div()
            .min_w_0()
            .mt(-(metrics.baseline - metrics.cap_height))
            .mb(-(metrics.line_height - metrics.baseline))
            .child(self.content)
    }
}
/// Centers cap-to-baseline letterforms in a fixed-height control's inherited line box.
pub fn label_cap_center_offset(label: &str, window: &gpui::Window) -> gpui::Pixels {
    let metrics = label_metrics(label, window);
    cap_center_offset(metrics.line_height, metrics.cap_height, metrics.baseline)
}

/// Computes the translation from a line baseline to cap-height centering.
pub fn cap_center_offset(
    line_height: gpui::Pixels,
    cap_height: gpui::Pixels,
    baseline: gpui::Pixels,
) -> gpui::Pixels {
    (line_height + cap_height) / 2. - baseline
}
