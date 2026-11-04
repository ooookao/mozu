//! Type-safe embedded SVG icons used by native UI primitives.

use std::borrow::Cow;

use gpui::{
    AssetSource, Hsla, IntoElement, Radians, Rems, RenderOnce, Result, SharedString, Styled as _,
    Transformation, img, radians, rgb, svg,
};

use crate::theme::colors::G2VTheme;

macro_rules! icon_assets {
    ($(($variant:ident, $path:literal)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum IconName {
            $($variant),+
        }

        impl IconName {
            pub const fn path(self) -> &'static str {
                match self {
                    $(Self::$variant => concat!("icons/", $path, ".svg")),+
                }
            }
        }

        pub struct Assets;

        impl AssetSource for Assets {
            fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
                let embedded = match path {
                    $(concat!("icons/", $path, ".svg") => Some(Cow::Borrowed(
                        include_bytes!(concat!("../../assets/icons/", $path, ".svg")).as_slice(),
                    )),)+
                    _ => None,
                };

                Ok(embedded)
            }

            fn list(&self, path: &str) -> Result<Vec<SharedString>> {
                let icons = [$(concat!("icons/", $path, ".svg")),+];
                let paths = icons
                    .into_iter()
                    .filter(|icon| icon.starts_with(path))
                    .map(SharedString::from)
                    .collect::<Vec<_>>();
                Ok(paths)
            }
        }
    };
}

icon_assets![
    (ArrowLeft, "arrow-left"),
    (ArrowRight, "arrow-right"),
    (ArrowUp, "arrow-up"),
    (ArrowDown, "arrow-down"),
    (ArrowTurnForward, "arrow-turn-forward"),
    (ArrowUpRight, "arrow-up-right"),
    (ArrowRotateClockwise, "arrow-rotate-clockwise"),
    (Archive, "archive"),
    (CircleAlert, "circle-alert"),
    (CircleInfo, "circle-info"),
    (CircleUser, "circle-user"),
    (Bug, "bug"),
    (Branch, "branch"),
    (Check, "check"),
    (Changes, "changes"),
    (Commit, "commit"),
    (CloudUpload, "cloud-upload"),
    (PullRequest, "pull-request"),
    (CircleX, "circle-x"),
    (ChevronDown, "chevron-down"),
    (ChevronLeft, "chevron-left"),
    (ChevronRight, "chevron-right"),
    (ChevronUp, "chevron-up"),
    (Components, "components"),
    (Control, "control"),
    (Setting, "setting"),
    (Copy, "copy"),
    (Console, "console"),
    (Cue, "cue"),
    (CodeAnalyze, "code-analyze"),
    (Dots, "dots"),
    (DotsVertical, "dots-vertical"),
    (FolderClosed, "folder-closed"),
    (FolderOpen, "folder-open"),
    (Folder1, "folder-1"),
    (Folders, "folders"),
    (Files, "files"),
    (Globe, "globe"),
    (Hammer, "hammer"),
    (HandSafe, "hand-safe"),
    (Hold, "hold"),
    (ProviderOpenAi, "openai"),
    (ProviderClaude, "claude"),
    (ProviderGrok, "grok"),
    (ProviderPi, "pi"),
    (ProviderHermesAgent, "hermesagent"),
    (ProviderGemini, "gemini"),
    (ProviderGithub, "github"),
    (ProviderHuggingFace, "hugging-face"),
    (ProviderGitlab, "gitlab"),
    (ProviderAzureDevops, "azure-devops"),
    (LayoutAlignBottom, "layout-align-bottom"),
    (LayoutAlignLeft, "layout-align-left"),
    (LayoutAlignRight, "layout-align-right"),
    (Loading, "loading"),
    (Moon, "moon"),
    (Macbook, "macbook"),
    (MinusSmall, "minus-small"),
    (Microphone, "microphone"),
    (Paperclip, "paperclip"),
    (PencilEdit, "pencil-edit"),
    (Edit, "edit"),
    (Plan, "plan"),
    (PlusMedium, "plus-medium"),
    (PlusComments, "plus-comments"),
    (ProductMark, "g2v-b"),
    (Pin, "pin"),
    (Search, "search"),
    (ShieldOff, "shield-off"),
    (Scripts, "scripts"),
    (Stop, "stop"),
    (Sun, "sun"),
    (Trash, "trash"),
    (X, "x"),
];

#[derive(IntoElement)]
#[must_use]
pub struct Icon {
    name: IconName,
    size: Rems,
    color: Option<Hsla>,
    rotation: Radians,
}

#[derive(IntoElement)]
#[must_use]
pub struct BrandIcon {
    name: BrandIconName,
    size: Rems,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrandIconName {
    OpenAi,
    Claude,
    Grok,
    Github,
    HuggingFace,
    Gitlab,
    AzureDevops,
}

impl BrandIconName {
    const fn icon_name(self) -> IconName {
        match self {
            Self::OpenAi => IconName::ProviderOpenAi,
            Self::Claude => IconName::ProviderClaude,
            Self::Grok => IconName::ProviderGrok,
            Self::Github => IconName::ProviderGithub,
            Self::HuggingFace => IconName::ProviderHuggingFace,
            Self::Gitlab => IconName::ProviderGitlab,
            Self::AzureDevops => IconName::ProviderAzureDevops,
        }
    }
}

impl BrandIcon {
    pub fn new(name: BrandIconName, size: Rems) -> Self {
        Self { name, size }
    }
}

impl RenderOnce for BrandIcon {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let color = match self.name {
            BrandIconName::Gitlab | BrandIconName::AzureDevops | BrandIconName::HuggingFace => {
                return img(self.name.icon_name().path())
                    .size(self.size)
                    .flex_none()
                    .into_any_element();
            }
            BrandIconName::OpenAi | BrandIconName::Grok | BrandIconName::Github => {
                G2VTheme::global(cx).foreground
            }
            BrandIconName::Claude => rgb(0x00d9_7757).into(),
        };
        svg()
            .path(self.name.icon_name().path())
            .size(self.size)
            .text_color(color)
            .flex_none()
            .into_any_element()
    }
}

impl Icon {
    pub fn new(name: IconName, size: Rems) -> Self {
        Self {
            name,
            size,
            color: None,
            rotation: radians(0.0),
        }
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn rotate(mut self, rotation: Radians) -> Self {
        self.rotation = rotation;
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let color = self
            .color
            .unwrap_or_else(|| G2VTheme::global(cx).foreground);
        svg()
            .path(self.name.path())
            .size(self.size)
            .text_color(color)
            .flex_none()
            .with_transformation(Transformation::rotate(self.rotation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_source_serves_every_declared_icon() {
        let assets = Assets;
        let paths = assets.list("icons/").expect("list embedded icons");

        assert!(!paths.is_empty());
        for icon in [
            IconName::ArrowUp,
            IconName::CircleUser,
            IconName::LayoutAlignLeft,
            IconName::ProductMark,
            IconName::Search,
            IconName::Setting,
            IconName::Check,
            IconName::X,
            IconName::ProviderGithub,
            IconName::ProviderGitlab,
            IconName::ProviderAzureDevops,
        ] {
            assert!(
                assets
                    .load(icon.path())
                    .expect("load required icon")
                    .is_some()
            );
        }
    }
}
