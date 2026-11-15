use gpui::{Background, Fill, Hsla, Oklcha, Rgba, Window};
use kinu_ui::ColorScheme;
use std::borrow::Cow;

pub use kinu_ui::IconSize;

/// One editable OKLCH value, converted only when a drawing API needs a color.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(transparent)]
pub struct ThemeColor(pub Oklcha);
impl ThemeColor {
    pub fn opacity(self, opacity: f32) -> Hsla {
        Hsla::from(self).opacity(opacity)
    }
}
impl From<Oklcha> for ThemeColor {
    fn from(value: Oklcha) -> Self {
        Self(value)
    }
}
impl From<ThemeColor> for Hsla {
    fn from(value: ThemeColor) -> Self {
        value.0.into()
    }
}
impl From<ThemeColor> for Rgba {
    fn from(value: ThemeColor) -> Self {
        value.0.into()
    }
}
impl From<ThemeColor> for Fill {
    fn from(value: ThemeColor) -> Self {
        Hsla::from(value).into()
    }
}
impl From<ThemeColor> for Background {
    fn from(value: ThemeColor) -> Self {
        Hsla::from(value).into()
    }
}

/// Independent syntax colors shared by editable documents and read-only code.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SyntaxPaletteSettings {
    pub foreground: ThemeColor,
    pub comment: ThemeColor,
    pub string: ThemeColor,
    pub number: ThemeColor,
    pub constant: ThemeColor,
    pub variable: ThemeColor,
    pub keyword: ThemeColor,
    pub function: ThemeColor,
    pub r#type: ThemeColor,
    pub punctuation: ThemeColor,
    pub tag: ThemeColor,
    pub regex: ThemeColor,
    pub escape: ThemeColor,
    pub attribute: ThemeColor,
}

/// Semantic color, radius and font tokens owned by the application.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Theme {
    pub syntax: SyntaxPaletteSettings,
    pub checker_a: ThemeColor,
    pub checker_b: ThemeColor,
    pub overlay: ThemeColor,
    #[serde(deserialize_with = "deserialize_opacity")]
    pub tint_opacity: f32,
    #[serde(deserialize_with = "deserialize_opacity")]
    pub tint_hover_opacity: f32,
    #[serde(deserialize_with = "deserialize_opacity")]
    pub active_opacity: f32,
    pub destructive_light: ThemeColor,
    pub destructive_fill: ThemeColor,
    pub destructive_foreground: ThemeColor,
    pub radio_border: ThemeColor,
    pub selected: ThemeColor,
    #[serde(deserialize_with = "deserialize_opacity")]
    pub hover_opacity: f32,
    pub thumb: ThemeColor,
    pub info: ThemeColor,
    pub success: ThemeColor,
    pub warning: ThemeColor,
    pub surfaces: SurfaceColorsSettings,
    pub shadows: std::sync::Arc<SurfaceShadowsSettings>,
    pub background: ThemeColor,
    pub foreground: ThemeColor,
    pub card: ThemeColor,
    pub card_foreground: ThemeColor,
    pub popover: ThemeColor,
    pub popover_foreground: ThemeColor,
    pub primary: ThemeColor,
    pub primary_foreground: ThemeColor,
    pub secondary: ThemeColor,
    pub secondary_foreground: ThemeColor,
    pub muted: ThemeColor,
    pub muted_foreground: ThemeColor,
    pub brand: ThemeColor,
    pub accent: ThemeColor,
    pub accent_foreground: ThemeColor,
    pub destructive: ThemeColor,
    pub border: ThemeColor,
    pub input: ThemeColor,
    pub ring: ThemeColor,
    pub chart_1: ThemeColor,
    pub chart_2: ThemeColor,
    pub chart_3: ThemeColor,
    pub chart_4: ThemeColor,
    pub chart_5: ThemeColor,
    pub sidebar: ThemeColor,
    pub sidebar_foreground: ThemeColor,
    pub sidebar_primary: ThemeColor,
    pub sidebar_primary_foreground: ThemeColor,
    pub sidebar_accent: ThemeColor,
    pub sidebar_accent_foreground: ThemeColor,
    pub sidebar_border: ThemeColor,
    pub sidebar_ring: ThemeColor,
    #[serde(deserialize_with = "deserialize_radius")]
    pub radius: f32,
    #[serde(deserialize_with = "deserialize_font")]
    pub font_sans: gpui::SharedString,
    #[serde(deserialize_with = "deserialize_font")]
    pub font_mono: gpui::SharedString,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ThemePalette {
    pub light: Theme,
    pub dark: Theme,
    pub motion: Option<std::sync::Arc<MotionSettings>>,
    pub components: Option<std::sync::Arc<ComponentSettings>>,
}
impl ThemePalette {
    pub fn with_font_sans(self, font: &'static str) -> Self {
        let font: gpui::SharedString = font.into();
        Self {
            light: Theme {
                font_sans: font.clone(),
                ..self.light
            },
            dark: Theme {
                font_sans: font.clone(),
                ..self.dark
            },
            motion: self.motion,
            components: self.components,
        }
    }
    /// Reads the application-owned palette installed before opening windows.
    pub fn current(cx: &gpui::App) -> &Self {
        cx.try_global::<Self>()
            .expect("No theme installed; call theme::install(cx) once at application startup")
    }
    /// Replaces the application palette and refreshes cached views in every window.
    pub fn apply(self, cx: &mut gpui::App) {
        cx.set_global(self);
        cx.refresh_windows();
    }
}
impl Theme {
    /// Resolves Tree-sitter capture names and semantic token kinds from one palette.
    /// More specific names inherit their nearest authored category.
    pub fn syntax_style(&self, name: &str) -> Option<gpui::HighlightStyle> {
        let mut category = name;
        loop {
            let color = match category {
                "foreground" | "embedded" | "predictive" | "primary" => {
                    Some(self.syntax.foreground)
                }
                "comment" | "hint" => Some(self.syntax.comment),
                "string" | "text.code.span" | "text.literal" | "link_uri" => {
                    Some(self.syntax.string)
                }
                "number" | "boolean" => Some(self.syntax.number),
                "constant" | "label" | "variable.special" => Some(self.syntax.constant),
                "variable" | "property" | "parameter" | "namespace" => Some(self.syntax.variable),
                "keyword" | "preproc" | "tag.doctype" => Some(self.syntax.keyword),
                "function" | "method" | "constructor" | "link_text" => Some(self.syntax.function),
                "type" | "enum" | "variant" | "class" | "interface" | "struct"
                | "typeParameter" => Some(self.syntax.r#type),
                "punctuation" | "operator" => Some(self.syntax.punctuation),
                "tag" | "title" | "punctuation.special" => Some(self.syntax.tag),
                "string.regex" | "regexp" => Some(self.syntax.regex),
                "string.escape" => Some(self.syntax.escape),
                "attribute" | "decorator" => Some(self.syntax.attribute),
                "emphasis" => {
                    return Some(gpui::HighlightStyle {
                        color: Some(self.syntax.keyword.into()),
                        font_style: Some(gpui::FontStyle::Italic),
                        ..Default::default()
                    });
                }
                "emphasis.strong" => {
                    return Some(gpui::HighlightStyle {
                        color: Some(self.syntax.constant.into()),
                        font_weight: Some(gpui::FontWeight::BOLD),
                        ..Default::default()
                    });
                }
                _ => None,
            };
            if let Some(color) = color {
                return Some(gpui::HighlightStyle {
                    color: Some(color.into()),
                    ..Default::default()
                });
            }
            category = category.rsplit_once('.')?.0;
        }
    }

    /// Supplies native Editor's palette without coupling code display to an editor.
    pub fn syntax_highlights(&self) -> Vec<(String, gpui::HighlightStyle)> {
        [
            "foreground",
            "embedded",
            "predictive",
            "primary",
            "comment",
            "hint",
            "string",
            "text.code.span",
            "text.literal",
            "link_uri",
            "number",
            "boolean",
            "constant",
            "label",
            "variable.special",
            "variable",
            "property",
            "parameter",
            "namespace",
            "keyword",
            "preproc",
            "tag.doctype",
            "function",
            "method",
            "constructor",
            "link_text",
            "type",
            "enum",
            "variant",
            "class",
            "interface",
            "struct",
            "typeParameter",
            "punctuation",
            "operator",
            "tag",
            "title",
            "punctuation.special",
            "string.regex",
            "regexp",
            "string.escape",
            "attribute",
            "decorator",
            "emphasis",
            "emphasis.strong",
        ]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                self.syntax_style(name).expect("Declared syntax category"),
            )
        })
        .collect()
    }

    pub fn hover(&self) -> Hsla {
        self.overlay.opacity(self.hover_opacity)
    }

    pub fn active(&self) -> Hsla {
        self.overlay.opacity(self.active_opacity)
    }

    pub fn tint(&self) -> Hsla {
        self.overlay.opacity(self.tint_opacity)
    }

    pub fn tint_hover(&self) -> Hsla {
        self.overlay.opacity(self.tint_hover_opacity)
    }

    pub fn current(window: &Window, cx: &gpui::App) -> Self {
        let palette = ThemePalette::current(cx);
        match ColorScheme::current(window, cx) {
            ColorScheme::Light => palette.light.clone(),
            ColorScheme::Dark => palette.dark.clone(),
        }
    }
    pub fn radius_sm(&self) -> f32 {
        self.radius * 0.6
    }
    pub fn radius_md(&self) -> f32 {
        self.radius * 0.8
    }
    pub fn radius_lg(&self) -> f32 {
        self.radius
    }
    pub fn radius_xl(&self) -> f32 {
        self.radius * 1.4
    }
    pub fn radius_2xl(&self) -> f32 {
        self.radius * 1.8
    }
    pub fn radius_3xl(&self) -> f32 {
        self.radius * 2.2
    }
    pub fn radius_4xl(&self) -> f32 {
        self.radius * 2.6
    }
    pub fn secondary_hover(&self) -> Hsla {
        self.secondary.0.mix(self.foreground.0, 0.05).into()
    }
}
#[derive(Clone, Copy, Debug)]
enum ThemeOrigin {
    Header,
    Palette,
}
#[derive(Clone, Debug)]
struct ThemeLocation {
    origin: ThemeOrigin,
    span: std::ops::Range<usize>,
    token: String,
    value: String,
}
#[derive(Debug)]
struct ThemeReadError {
    message: String,
    location: Option<ThemeLocation>,
}
impl std::fmt::Display for ThemeReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ThemeReadError {}
impl serde::de::Error for ThemeReadError {
    fn custom<T: std::fmt::Display>(message: T) -> Self {
        Self {
            message: message.to_string(),
            location: None,
        }
    }
}
struct ThemeField {
    origin: ThemeOrigin,
    name: String,
    key_span: std::ops::Range<usize>,
    value: toml::Spanned<toml::Value>,
}
/// Source locations exist only during decoding; the installed palette contains typed fields.
struct ThemeFields(Vec<ThemeField>);
impl<'de> serde::Deserialize<'de> for ThemeFields {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldsVisitor;
        impl<'de> serde::de::Visitor<'de> for FieldsVisitor {
            type Value = ThemeFields;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a table of typed theme tokens")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Self::Value, M::Error> {
                let mut fields = Vec::new();
                while let Some(key) = map.next_key::<toml::Spanned<String>>()? {
                    let span = key.span();
                    fields.push(ThemeField {
                        origin: ThemeOrigin::Palette,
                        name: key.into_inner(),
                        key_span: span,
                        value: map.next_value()?,
                    });
                }
                Ok(ThemeFields(fields))
            }
        }
        deserializer.deserialize_map(FieldsVisitor)
    }
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ThemeFile {
    motion: Option<MotionSettings>,
    components: Option<toml::Spanned<ComponentSettings>>,
    radius: toml::Spanned<String>,
    font_sans: toml::Spanned<String>,
    font_mono: toml::Spanned<String>,
    light: toml::Spanned<ThemeFields>,
    dark: toml::Spanned<ThemeFields>,
}
struct ThemeFieldsAccess {
    fields: std::vec::IntoIter<ThemeField>,
    current: Option<ThemeField>,
}
impl<'de> serde::de::MapAccess<'de> for ThemeFieldsAccess {
    type Error = ThemeReadError;
    fn next_key_seed<S: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Self::Error> {
        let Some(field) = self.fields.next() else {
            return Ok(None);
        };
        let location = ThemeLocation {
            origin: field.origin,
            span: field.key_span.clone(),
            token: field.name.clone(),
            value: field.value.get_ref().to_string(),
        };
        let result = seed
            .deserialize(serde::de::value::StringDeserializer::<ThemeReadError>::new(
                field.name.clone(),
            ))
            .map(Some)
            .map_err(|error| ThemeReadError {
                message: format!("{}; add this token as a field of theme.rs Theme and provide its value in theme.toml", error.message),
                location: Some(location),
            });
        self.current = Some(field);
        result
    }
    fn next_value_seed<S: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        let field = self.current.take().ok_or_else(|| ThemeReadError {
            message: "Theme decoder requested a value before its key".into(),
            location: None,
        })?;
        let location = ThemeLocation {
            origin: field.origin,
            span: field.value.span(),
            token: field.name,
            value: field.value.get_ref().to_string(),
        };
        seed.deserialize(field.value.into_inner())
            .map_err(|error| ThemeReadError {
                message: error.to_string(),
                location: Some(location),
            })
    }
}
fn theme_error(
    source: &str,
    filename: &str,
    table: &str,
    message: &str,
    location: &ThemeLocation,
) -> ThemeReadError {
    let prefix = &source[..location.span.start];
    let table = match location.origin {
        ThemeOrigin::Header => "root",
        ThemeOrigin::Palette => table,
    };
    let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .expect("A source prefix has one line")
        .chars()
        .count()
        + 1;
    let source_line = source
        .split('\n')
        .nth(line - 1)
        .expect("Source location is in the input");
    ThemeReadError {
        message: format!(
            "{filename}:{line}:{column}: table={table}, token={}, value={}: {message}; source={source_line:?}",
            location.token, location.value
        ),
        location: None,
    }
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct ThemeBorderRecipe {
    from: ThemeBorderSource,
    #[serde(deserialize_with = "deserialize_opacity")]
    opacity: f32,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ThemeBorderSource {
    Foreground,
}

/// Resolves an explicitly authored border recipe once, before installing the palette.
fn resolve_theme_border(
    fields: toml::Spanned<ThemeFields>,
    source: &str,
    filename: &str,
    table: &str,
) -> gpui::Result<toml::Spanned<ThemeFields>> {
    let span = fields.span();
    let mut resolved = fields.into_inner();
    let Some(index) = resolved.0.iter().position(|field| field.name == "border") else {
        return Ok(toml::Spanned::new(span, resolved));
    };
    let border = &resolved.0[index];
    if !border.value.get_ref().is_table() {
        return Ok(toml::Spanned::new(span, resolved));
    }
    let location = ThemeLocation {
        origin: border.origin,
        span: border.value.span(),
        token: border.name.clone(),
        value: border.value.get_ref().to_string(),
    };
    let recipe: ThemeBorderRecipe =
        border
            .value
            .get_ref()
            .clone()
            .try_into()
            .map_err(|error: toml::de::Error| {
                theme_error(source, filename, table, error.message(), &location)
            })?;
    let name = match recipe.from {
        ThemeBorderSource::Foreground => "foreground",
    };
    let base = resolved
        .0
        .iter()
        .find(|field| field.name == name)
        .ok_or_else(|| {
            theme_error(
                source,
                filename,
                table,
                "Border recipe requires foreground",
                &location,
            )
        })?;
    let color: ThemeColor =
        base.value
            .get_ref()
            .clone()
            .try_into()
            .map_err(|error: toml::de::Error| {
                theme_error(
                    source,
                    filename,
                    table,
                    error.message(),
                    &ThemeLocation {
                        origin: base.origin,
                        span: base.value.span(),
                        token: base.name.clone(),
                        value: base.value.get_ref().to_string(),
                    },
                )
            })?;
    let [lightness, chroma, hue, alpha] = color.0.opacity(f64::from(recipe.opacity)).components();
    resolved.0[index].value = toml::Spanned::new(
        location.span,
        toml::Value::String(format!("oklch({lightness} {chroma} {hue} / {alpha})")),
    );
    Ok(toml::Spanned::new(span, resolved))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeSurfaceReference {
    from: ThemeSurfaceSource,
}
#[derive(serde::Deserialize)]
enum ThemeSurfaceSource {
    #[serde(rename = "surfaces.one")]
    One,
    #[serde(rename = "surfaces.three")]
    Three,
}

/// Resolves explicit background and card aliases without converting their authored color values.
fn resolve_theme_surface_references(
    fields: toml::Spanned<ThemeFields>,
    source: &str,
    filename: &str,
    table: &str,
) -> gpui::Result<toml::Spanned<ThemeFields>> {
    let span = fields.span();
    let mut resolved = fields.into_inner();
    for token in ["background", "card"] {
        let Some(index) = resolved.0.iter().position(|field| field.name == token) else {
            continue;
        };
        let field = &resolved.0[index];
        if !field.value.get_ref().is_table() {
            continue;
        }
        let location = ThemeLocation {
            origin: field.origin,
            span: field.value.span(),
            token: field.name.clone(),
            value: field.value.get_ref().to_string(),
        };
        let reference: ThemeSurfaceReference =
            field
                .value
                .get_ref()
                .clone()
                .try_into()
                .map_err(|error: toml::de::Error| {
                    theme_error(source, filename, table, error.message(), &location)
                })?;
        let name = match reference.from {
            ThemeSurfaceSource::One => "one",
            ThemeSurfaceSource::Three => "three",
        };
        let value = resolved
            .0
            .iter()
            .find(|field| field.name == "surfaces")
            .and_then(|field| field.value.get_ref().get(name))
            .ok_or_else(|| {
                theme_error(
                    source,
                    filename,
                    table,
                    &format!("Surface reference requires surfaces.{name} in the same scheme"),
                    &location,
                )
            })?
            .clone();
        resolved.0[index].value = toml::Spanned::new(location.span, value);
    }
    Ok(toml::Spanned::new(span, resolved))
}

fn decode_theme(
    fields: toml::Spanned<ThemeFields>,
    common: [(&str, &toml::Spanned<String>); 3],
    source: &str,
    filename: &str,
    table: &str,
) -> gpui::Result<Theme> {
    let fields = resolve_theme_surface_references(fields, source, filename, table)?;
    let fields = resolve_theme_border(fields, source, filename, table)?;
    let table_span = fields.span();
    let mut fields = fields.into_inner().0;
    for (name, value) in common {
        fields.push(ThemeField {
            origin: ThemeOrigin::Header,
            name: name.into(),
            key_span: value.span(),
            value: toml::Spanned::new(value.span(), toml::Value::String(value.get_ref().clone())),
        });
    }
    let access = ThemeFieldsAccess {
        fields: fields.into_iter(),
        current: None,
    };
    Ok(
        <Theme as serde::Deserialize>::deserialize(serde::de::value::MapAccessDeserializer::new(
            access,
        ))
        .map_err(|error| {
            let location = match error.location {
                Some(location) => location,
                None => ThemeLocation {
                    origin: ThemeOrigin::Palette,
                    span: table_span,
                    token: error.message.clone(),
                    value: "<missing>".into(),
                },
            };
            theme_error(source, filename, table, &error.message, &location)
        })?,
    )
}
/// Parses one complete, application-owned theme without defaults or a token dictionary.
pub fn load(source: &str, filename: &str) -> gpui::Result<ThemePalette> {
    let document: ThemeFile = toml::from_str(source).map_err(|error: toml::de::Error| {
        let span = match error.span() {
            Some(span) => span,
            None => 0..0,
        };
        let value = &source[span.clone()];
        theme_error(
            source,
            filename,
            "root",
            error.message(),
            &ThemeLocation {
                origin: ThemeOrigin::Header,
                span,
                token: error.message().into(),
                value: value.into(),
            },
        )
    })?;
    let common = [
        ("radius", &document.radius),
        ("font-sans", &document.font_sans),
        ("font-mono", &document.font_mono),
    ];
    Ok(ThemePalette {
        motion: document.motion.map(std::sync::Arc::new),
        components: document
            .components
            .map(|settings| validate_component_settings(settings, source, filename))
            .transpose()?
            .map(std::sync::Arc::new),
        light: decode_theme(document.light, common, source, filename, "light")?,
        dark: decode_theme(document.dark, common, source, filename, "dark")?,
    })
}
fn deserialize_radius<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    let input = <String as serde::Deserialize>::deserialize(deserializer)?;
    let normalized = input.trim().to_ascii_lowercase();
    let (number, scale) = if let Some(number) = normalized.strip_suffix("rem") {
        (number, 16.)
    } else if let Some(number) = normalized.strip_suffix("px") {
        (number, 1.)
    } else {
        return Err(serde::de::Error::custom(format!(
            "Invalid radius {input:?}; expected a nonnegative rem or px length"
        )));
    };
    let number = number.parse::<f64>().map_err(|error| {
        serde::de::Error::custom(format!(
            "Invalid radius {input:?}: {error}; expected a nonnegative rem or px length"
        ))
    })?;
    let pixels = (number * scale) as f32;
    if !number.is_finite() || number < 0. || !pixels.is_finite() {
        return Err(serde::de::Error::custom(format!(
            "Invalid radius {input:?}; length must be finite and nonnegative"
        )));
    }
    Ok(pixels)
}
fn deserialize_font<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<gpui::SharedString, D::Error> {
    let font = <String as serde::Deserialize>::deserialize(deserializer)?;
    if font.trim().is_empty() {
        return Err(serde::de::Error::custom(
            "Font name must be a nonempty string",
        ));
    }
    Ok(font.into())
}
impl gpui::Global for ThemePalette {}
/// Registers the embedded, application-owned palette and its bundled font faces once at startup.
pub fn install(cx: &mut gpui::App) -> gpui::Result<()> {
    let palette = load(include_str!("theme.toml"), "theme.toml")?;
    cx.text_system().add_fonts(font_bytes())?;
    palette.apply(cx);
    Ok(())
}

/// Register these faces once before opening the application's windows.
pub fn font_bytes() -> Vec<Cow<'static, [u8]>> {
    vec![
        Cow::Borrowed(include_bytes!("fonts/Sans-400.ttf")),
        Cow::Borrowed(include_bytes!("fonts/Sans-500.ttf")),
        Cow::Borrowed(include_bytes!("fonts/Sans-600.ttf")),
        Cow::Borrowed(include_bytes!("fonts/Sans-700.ttf")),
        Cow::Borrowed(include_bytes!("fonts/Mono-400.ttf")),
        Cow::Borrowed(include_bytes!("fonts/Mono-500.ttf")),
        Cow::Borrowed(include_bytes!("fonts/Mono-600.ttf")),
        Cow::Borrowed(include_bytes!("fonts/Mono-700.ttf")),
    ]
}

/// Shared motion settings read from the application's theme file.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MotionTierSettings {
    pub exit_easing: EasingSettings,
    #[serde(deserialize_with = "deserialize_positive_millis")]
    pub enter_ms: u64,
    pub exit_ms: u64,
    #[serde(deserialize_with = "deserialize_bounce")]
    pub bounce: f32,
}
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MotionSettings {
    pub animations: AnimationProfiles,
    pub curves: EasingProfiles,
    pub fast: MotionTierSettings,
    pub moderate: MotionTierSettings,
    pub slow: MotionTierSettings,
    #[serde(skip)]
    pub runtime: std::sync::Mutex<Option<MotionSpringCache>>,
}
#[derive(Clone, Copy, Debug)]
pub struct MotionSpringCache {
    pub fast_source: MotionTierSettings,
    pub moderate_source: MotionTierSettings,
    pub slow_source: MotionTierSettings,
    pub fast: gpui::SpringConfig,
    pub moderate: gpui::SpringConfig,
    pub slow: gpui::SpringConfig,
}
impl Clone for MotionSettings {
    fn clone(&self) -> Self {
        Self {
            animations: self.animations,
            curves: self.curves,
            fast: self.fast,
            moderate: self.moderate,
            slow: self.slow,
            runtime: std::sync::Mutex::new(None),
        }
    }
}
impl PartialEq for MotionSettings {
    fn eq(&self, other: &Self) -> bool {
        self.animations == other.animations
            && self.curves == other.curves
            && self.fast == other.fast
            && self.moderate == other.moderate
            && self.slow == other.slow
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShapeSetting {
    Rounded,
    Pill,
    Sharp,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SizeSetting {
    Default,
    Compact,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RadiusSettings {
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub item: f32,
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub focus: f32,
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub container: f32,
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub merged: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ShapeSettings {
    pub rounded: RadiusSettings,
    pub pill: RadiusSettings,
    pub sharp: RadiusSettings,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ControlMetricsSettings {
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub control_height: f32,
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub gap: f32,
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub horizontal_padding: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SizeSettings {
    pub default: ControlMetricsSettings,
    pub compact: ControlMetricsSettings,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TextMetricsSettings {
    #[serde(deserialize_with = "deserialize_positive_number")]
    pub size: f32,
    #[serde(deserialize_with = "deserialize_positive_number")]
    pub line_height: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TypeScaleStepSettings {
    pub display: TextMetricsSettings,
    pub title: TextMetricsSettings,
    pub subtitle: TextMetricsSettings,
    pub body: TextMetricsSettings,
    pub caption: TextMetricsSettings,
    pub micro: TextMetricsSettings,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TypeScaleSettings {
    pub default: TypeScaleStepSettings,
    pub compact: TypeScaleStepSettings,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FontWeightSettings {
    #[serde(deserialize_with = "deserialize_positive_number")]
    pub weight: f32,
    #[serde(deserialize_with = "deserialize_positive_number")]
    pub optical_size: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FontWeightsSettings {
    pub normal: FontWeightSettings,
    pub semibold: FontWeightSettings,
    pub bold: FontWeightSettings,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SurfaceColorsSettings {
    pub one: ThemeColor,
    pub two: ThemeColor,
    pub three: ThemeColor,
    pub four: ThemeColor,
    pub five: ThemeColor,
    pub six: ThemeColor,
    pub seven: ThemeColor,
    pub eight: ThemeColor,
}
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ThemeShadowSettings {
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub x: f32,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub y: f32,
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub blur: f32,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub spread: f32,
    pub color: ThemeColor,
    pub inset: bool,
}
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SurfaceShadowsSettings {
    pub one: ShadowRecipeSettings,
    pub two: ShadowRecipeSettings,
    pub three: ShadowRecipeSettings,
    pub four: ShadowRecipeSettings,
    pub five: ShadowRecipeSettings,
    pub six: ShadowRecipeSettings,
    pub seven: ShadowRecipeSettings,
    pub eight: ShadowRecipeSettings,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct NamedColorsSettings {
    pub gray: ThemeColor,
    pub red: ThemeColor,
    pub orange: ThemeColor,
    pub amber: ThemeColor,
    pub yellow: ThemeColor,
    pub lime: ThemeColor,
    pub green: ThemeColor,
    pub emerald: ThemeColor,
    pub teal: ThemeColor,
    pub cyan: ThemeColor,
    pub blue: ThemeColor,
    pub indigo: ThemeColor,
    pub violet: ThemeColor,
    pub purple: ThemeColor,
    pub fuchsia: ThemeColor,
    pub pink: ThemeColor,
    pub rose: ThemeColor,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TransitionSettings {
    pub slider_preview_ms: u64,
    pub tooltip_delay_ms: u64,
    pub scroll_fade_hold_ms: u64,
    pub group_converge_ms: u64,
    pub group_diverge_ms: u64,
    pub selection_acknowledgement_ms: u64,
    pub date_picker_selection_ack_ms: u64,
    pub corner_delay_min_ms: u64,
    pub corner_delay_max_ms: u64,
    #[serde(deserialize_with = "deserialize_nonnegative_number")]
    pub corner_delay_distance_ms: f32,
}
/// Additional presentation settings; shared semantic colors remain in light/dark.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ComponentSettings {
    pub size: SizeSetting,
    pub shape: ShapeSetting,
    pub shapes: ShapeSettings,
    pub sizes: SizeSettings,
    pub type_scale: TypeScaleSettings,
    pub font_weights: FontWeightsSettings,
    pub palette: NamedColorsSettings,
    pub timings: TransitionSettings,
}
fn deserialize_finite_number<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f32, D::Error> {
    let value = <f32 as serde::Deserialize>::deserialize(deserializer)?;
    if !value.is_finite() {
        return Err(serde::de::Error::custom("Expected a finite number"));
    }
    Ok(value)
}
fn deserialize_nonnegative_number<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f32, D::Error> {
    let value = deserialize_finite_number(deserializer)?;
    if value < 0. {
        return Err(serde::de::Error::custom("Expected a nonnegative number"));
    }
    Ok(value)
}
fn deserialize_positive_number<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f32, D::Error> {
    let value = deserialize_finite_number(deserializer)?;
    if value <= 0. {
        return Err(serde::de::Error::custom("Expected a positive number"));
    }
    Ok(value)
}
fn deserialize_positive_millis<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<u64, D::Error> {
    let value = <u64 as serde::Deserialize>::deserialize(deserializer)?;
    if value == 0 {
        return Err(serde::de::Error::custom("Duration must be positive"));
    }
    Ok(value)
}
fn deserialize_bounce<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    let value = deserialize_finite_number(deserializer)?;
    if !(0. ..1.).contains(&value) {
        return Err(serde::de::Error::custom(
            "Bounce must be at least zero and less than one",
        ));
    }
    Ok(value)
}

/// Cubic-bezier coordinates shared by source-owned animations.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EasingSettings {
    #[serde(deserialize_with = "deserialize_unit_number")]
    pub x1: f32,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub y1: f32,
    #[serde(deserialize_with = "deserialize_unit_number")]
    pub x2: f32,
    #[serde(deserialize_with = "deserialize_finite_number")]
    pub y2: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AnimationSettings {
    pub duration_ms: u64,
    pub easing: EasingSettings,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AnimationProfiles {
    pub color: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub backdrop: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub control: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub control_slow: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub item: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub popup: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub tooltip: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub accordion: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub sheet: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub drawer: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub drawer_backdrop: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub toast: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub toast_height: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub toast_content: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub skeleton: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub spinner: AnimationSettings,
    pub accordion_enter: AnimationSettings,
    pub accordion_exit: AnimationSettings,
    pub tab_label: AnimationSettings,
    pub radio_dot_exit: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub loading_spinner: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub loading_spinner_reduced: AnimationSettings,
    pub press_enter: AnimationSettings,
    pub press_exit: AnimationSettings,
    pub check_draw: AnimationSettings,
    pub check_retract: AnimationSettings,
    pub arrow_rotation: AnimationSettings,
    pub label_weight: AnimationSettings,
    pub slider_label: AnimationSettings,
    pub scrollbar_enter: AnimationSettings,
    #[serde(deserialize_with = "deserialize_positive_animation")]
    pub scrollbar_exit: AnimationSettings,
}
fn deserialize_positive_animation<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<AnimationSettings, D::Error> {
    let profile = <AnimationSettings as serde::Deserialize>::deserialize(deserializer)?;
    if profile.duration_ms == 0 {
        return Err(serde::de::Error::custom("Duration must be positive"));
    }
    Ok(profile)
}

impl MotionSettings {
    pub fn current(cx: &gpui::App) -> &Self {
        ThemePalette::current(cx).motion.as_deref().expect(
            "Motion settings are missing; reinstall the component to add them to theme.toml",
        )
    }
}
fn deserialize_opacity<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
    let value = deserialize_finite_number(deserializer)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(serde::de::Error::custom("Opacity must be between 0 and 1"));
    }
    Ok(value)
}

fn deserialize_unit_number<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f32, D::Error> {
    let value = deserialize_finite_number(deserializer)?;
    if !(0. ..=1.).contains(&value) {
        return Err(serde::de::Error::custom(
            "Bezier x coordinate must be between zero and one",
        ));
    }
    Ok(value)
}

impl AnimationSettings {
    pub fn curve(self) -> impl Fn(f32) -> f32 + 'static {
        gpui::cubic_bezier(
            self.easing.x1,
            self.easing.y1,
            self.easing.x2,
            self.easing.y2,
        )
    }
    pub fn animation(self) -> gpui::Animation {
        gpui::Animation::new(std::time::Duration::from_millis(self.duration_ms))
            .with_easing(self.curve())
    }
}

fn validate_component_settings(
    settings: toml::Spanned<ComponentSettings>,
    source: &str,
    filename: &str,
) -> gpui::Result<ComponentSettings> {
    let span = settings.span();
    let settings = settings.into_inner();
    let timing = settings.timings;
    for (valid, key, value, message) in [(
        timing.corner_delay_max_ms >= timing.corner_delay_min_ms,
        "corner-delay-max-ms",
        timing.corner_delay_max_ms,
        "Maximum corner delay must be at least corner-delay-min-ms",
    )] {
        if !valid {
            let mut offset = span.start;
            for line in source[span.clone()].split_inclusive('\n') {
                if line
                    .split_once('=')
                    .is_some_and(|(name, _)| name.trim().trim_matches('"') == key)
                {
                    break;
                }
                offset += line.len();
            }
            let location = ThemeLocation {
                origin: ThemeOrigin::Palette,
                span: offset..offset,
                token: key.into(),
                value: value.to_string(),
            };
            return Err(
                theme_error(source, filename, "components.timings", message, &location).into(),
            );
        }
    }
    Ok(settings)
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EasingProfiles {
    pub surface: EasingSettings,
    pub enter: EasingSettings,
    pub exit: EasingSettings,
    pub in_out: EasingSettings,
}
impl EasingSettings {
    pub fn curve(self) -> impl Fn(f32) -> f32 + 'static {
        gpui::cubic_bezier(self.x1, self.y1, self.x2, self.y2)
    }
}

/// One editable shadow recipe with a cache owned by this immutable theme value.
#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
pub struct ShadowRecipeSettings {
    pub parts: Vec<ThemeShadowSettings>,
    #[serde(skip)]
    pub runtime: std::sync::Mutex<Option<ShadowRecipeCache>>,
}
#[derive(Clone, Debug)]
pub struct ShadowRecipeCache {
    pub source: Vec<ThemeShadowSettings>,
    pub shadows: Vec<gpui::BoxShadow>,
}
impl Clone for ShadowRecipeSettings {
    fn clone(&self) -> Self {
        Self {
            parts: self.parts.clone(),
            runtime: std::sync::Mutex::new(None),
        }
    }
}
impl PartialEq for ShadowRecipeSettings {
    fn eq(&self, other: &Self) -> bool {
        self.parts == other.parts
    }
}
