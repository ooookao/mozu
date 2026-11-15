pub use gpui::SvgStrokeDirection;
use gpui::{App, IntoElement, RenderOnce, StyleRefinement, Styled, Window, px};
use kinu_ui::IconSize;

#[derive(Clone, Copy)]
pub enum IconName {
    Encoder,
    Indicator,
    Shortcut,
    OpenFile,
    Model,
    RecordAudio,
    OpenAI,

    VoiceInput,
    Transcription,
    Dictionary,
    Microphone,
    Files,
    Plan,
    Components,
    Setting,

    WrapText,
    Shuffle,
    Pause,
    MousePointer2,
    Send,
    LifeBuoy,
    File,
    ShoppingCart,
    ShoppingBag,
    ChartLine,
    Map,
    PieChart,
    Frame,
    Settings2,
    BookOpen,
    Bot,
    TerminalSquare,
    Lightbulb,
    SlidersHorizontal,
    MoreVertical,
    CornerDownRight,
    Circle,
    Shield,
    Palette,
    Pipette,
    ColorScrubCursor,
    Moon,
    SquareLibrary,
    Brain,
    Play,
    Clock,
    Star,
    Users,

    Code,
    Image,
    HelpCircle,
    Bell,
    ZoomOut,
    ZoomIn,
    List,
    LayoutGrid,
    Trash,
    ClipboardPaste,
    Scissors,
    Copy,
    FolderPlus,
    Folder,
    FolderOpen,
    Home,
    ArrowUp,
    ArrowUpRight,
    CircleFadingArrowUp,
    GitBranch,
    GitFork,
    LoaderCircle,
    Layout,
    Activity,
    Panel,
    ArrowDown,
    ArrowRight,
    ArrowLeft,
    ChevronRight,
    MoreHorizontal,
    ChevronDown,
    ChevronUp,
    CheckStroke,
    CheckboxGroupCheck,
    X,
    Search,
    Rocket,
    MessageCircle,
    Calendar,
    Smile,
    Calculator,
    User,
    CreditCard,
    Settings,
    CircleCheck,
    Info,
    TriangleAlert,
    OctagonX,
    FileText,
    BookOpenCheck,
    Inbox,
    Mail,
    Sun,
    Monitor,
    Globe,
    Lock,
    Link,
    Pencil,
    BannerCircleFilled,
    BannerWarningFilled,
    BannerInfoMark,
    BannerSuccessMark,
    BannerWarningMark,
    BannerErrorMark,
    FolderCode,
    CircleAlert,
    Terminal,
    ChevronsUpDown,
    Plus,
    Minus,
    Bold,
    Italic,
    Underline,
    BadgeCheck,
    Bookmark,
    PlusThin,
    MinusThin,
    BoldFilled,
    ItalicFilled,
    UnderlineFilled,
    TriangleRight,
    TriangleDown,
    Check,
    MinusThick,
    HeartFilled,
    HeartOutline,
    MinusNonScaling,
    CaretUpDown,
    TriangleUp,
    AlignLeft,
    AlignCenter,
    AlignRight,
}
impl IconName {
    /// Renders the named asset with its original colors, without a monochrome mask.
    pub fn image(self) -> gpui::Img {
        gpui::img(std::sync::Arc::new(gpui::Image::from_bytes(
            gpui::ImageFormat::Svg,
            self.bytes().to_vec(),
        )))
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Encoder => include_bytes!("../../assets/icons/encoder.svg"),
            Self::Indicator => include_bytes!("../../assets/icons/indicator.svg"),
            Self::Shortcut => include_bytes!("../../assets/icons/shortcut.svg"),
            Self::OpenFile => include_bytes!("../../assets/icons/open-file.svg"),
            Self::Model => include_bytes!("../../assets/icons/model.svg"),
            Self::RecordAudio => include_bytes!("../../assets/icons/record-audio.svg"),
            Self::OpenAI => include_bytes!("../../assets/icons/openai.svg"),
            Self::VoiceInput => include_bytes!("../../assets/icons/voice-input.svg"),
            Self::Transcription => include_bytes!("../../assets/icons/transcription.svg"),
            Self::Dictionary => include_bytes!("../../assets/icons/dictionary.svg"),
            Self::Microphone => include_bytes!("../../assets/icons/microphone.svg"),
            Self::Files => include_bytes!("../../assets/icons/files.svg"),
            Self::Plan => include_bytes!("../../assets/icons/plan.svg"),
            Self::Components => include_bytes!("../../assets/icons/components.svg"),
            Self::Setting => include_bytes!("../../assets/icons/setting.svg"),

            Self::CornerDownRight => include_bytes!("assets/icons/corner-down-right.svg"),
            Self::MoreVertical => include_bytes!("assets/icons/ellipsis-vertical.svg"),
            Self::SlidersHorizontal => include_bytes!("assets/icons/sliders-horizontal.svg"),
            Self::Lightbulb => include_bytes!("assets/icons/lightbulb.svg"),
            Self::TerminalSquare => include_bytes!("assets/icons/square-terminal.svg"),
            Self::Bot => include_bytes!("assets/icons/bot.svg"),
            Self::BookOpen => include_bytes!("assets/icons/book-open.svg"),
            Self::Settings2 => include_bytes!("assets/icons/settings-2.svg"),
            Self::Frame => include_bytes!("assets/icons/frame.svg"),
            Self::PieChart => include_bytes!("assets/icons/chart-pie.svg"),
            Self::Map => include_bytes!("assets/icons/map.svg"),
            Self::ChartLine => include_bytes!("assets/icons/chart-line.svg"),
            Self::ShoppingBag => include_bytes!("assets/icons/shopping-bag.svg"),
            Self::ShoppingCart => include_bytes!("assets/icons/shopping-cart.svg"),
            Self::File => include_bytes!("assets/icons/file.svg"),
            Self::LifeBuoy => include_bytes!("assets/icons/life-buoy.svg"),
            Self::Send => include_bytes!("assets/icons/send.svg"),
            Self::MousePointer2 => include_bytes!("assets/icons/mouse-pointer-2.svg"),
            Self::Pause => include_bytes!("assets/icons/pause.svg"),
            Self::Circle => include_bytes!("assets/icons/circle.svg"),
            Self::Lock => include_bytes!("assets/icons/lock.svg"),
            Self::Link => include_bytes!("assets/icons/link.svg"),
            Self::Pencil => include_bytes!("assets/icons/pencil.svg"),
            Self::BannerCircleFilled => include_bytes!("assets/icons/banner-circle-filled.svg"),
            Self::BannerWarningFilled => include_bytes!("assets/icons/banner-warning-filled.svg"),
            Self::BannerInfoMark => include_bytes!("assets/icons/banner-info-mark.svg"),
            Self::BannerSuccessMark => include_bytes!("assets/icons/banner-success-mark.svg"),
            Self::BannerWarningMark => include_bytes!("assets/icons/banner-warning-mark.svg"),
            Self::BannerErrorMark => include_bytes!("assets/icons/banner-error-mark.svg"),
            Self::Shield => include_bytes!("assets/icons/shield.svg"),
            Self::Palette => include_bytes!("assets/icons/palette.svg"),
            Self::Pipette => include_bytes!("assets/icons/pipette.svg"),
            Self::ColorScrubCursor => include_bytes!("assets/icons/color-scrub-cursor.svg"),
            Self::Moon => include_bytes!("assets/icons/moon.svg"),
            Self::Brain => include_bytes!("assets/icons/brain.svg"),
            Self::Play => include_bytes!("assets/icons/play.svg"),
            Self::SquareLibrary => include_bytes!("assets/icons/square-library.svg"),
            Self::Clock => include_bytes!("assets/icons/clock.svg"),
            Self::Star => include_bytes!("assets/icons/star.svg"),
            Self::Users => include_bytes!("assets/icons/users.svg"),

            Self::Code => include_bytes!("assets/icons/code.svg"),
            Self::Image => include_bytes!("assets/icons/image.svg"),
            Self::HelpCircle => include_bytes!("assets/icons/circle-question-mark.svg"),
            Self::Bell => include_bytes!("assets/icons/bell.svg"),
            Self::ZoomOut => include_bytes!("assets/icons/zoom-out.svg"),
            Self::ZoomIn => include_bytes!("assets/icons/zoom-in.svg"),
            Self::List => include_bytes!("assets/icons/list.svg"),
            Self::LayoutGrid => include_bytes!("assets/icons/layout-grid.svg"),
            Self::Trash => include_bytes!("assets/icons/trash.svg"),
            Self::ClipboardPaste => include_bytes!("assets/icons/clipboard-paste.svg"),
            Self::Scissors => include_bytes!("assets/icons/scissors.svg"),
            Self::Copy => include_bytes!("assets/icons/copy.svg"),
            Self::FolderPlus => include_bytes!("assets/icons/folder-plus.svg"),
            Self::Folder => include_bytes!("assets/icons/folder.svg"),
            Self::FolderOpen => include_bytes!("assets/icons/folder-open.svg"),
            Self::Home => include_bytes!("assets/icons/house.svg"),
            Self::ArrowUp => include_bytes!("assets/icons/arrow-up.svg"),
            Self::ArrowUpRight => include_bytes!("assets/icons/arrow-up-right.svg"),
            Self::CircleFadingArrowUp => include_bytes!("assets/icons/circle-fading-arrow-up.svg"),
            Self::GitBranch => include_bytes!("assets/icons/git-branch.svg"),
            Self::Search => include_bytes!("assets/icons/search.svg"),
            Self::Rocket => include_bytes!("assets/icons/rocket.svg"),
            Self::MessageCircle => include_bytes!("assets/icons/message-circle.svg"),
            Self::Calendar => include_bytes!("assets/icons/calendar.svg"),
            Self::Smile => include_bytes!("assets/icons/smile.svg"),
            Self::Calculator => include_bytes!("assets/icons/calculator.svg"),
            Self::User => include_bytes!("assets/icons/user.svg"),
            Self::CreditCard => include_bytes!("assets/icons/credit-card.svg"),
            Self::Settings => include_bytes!("assets/icons/settings.svg"),
            Self::CircleCheck => include_bytes!("assets/icons/circle-check.svg"),
            Self::Info => include_bytes!("assets/icons/info.svg"),
            Self::TriangleAlert => include_bytes!("assets/icons/triangle-alert.svg"),
            Self::OctagonX => include_bytes!("assets/icons/octagon-x.svg"),
            Self::FileText => include_bytes!("assets/icons/file-text.svg"),
            Self::BookOpenCheck => include_bytes!("assets/icons/book-open-check.svg"),
            Self::Inbox => include_bytes!("assets/icons/inbox.svg"),
            Self::Mail => include_bytes!("assets/icons/mail.svg"),
            Self::Sun => include_bytes!("assets/icons/sun.svg"),
            Self::Monitor => include_bytes!("assets/icons/monitor.svg"),
            Self::Globe => include_bytes!("assets/icons/globe.svg"),
            Self::GitFork => include_bytes!("assets/icons/git-fork.svg"),
            Self::LoaderCircle => include_bytes!("assets/icons/loader-circle.svg"),
            Self::Layout => include_bytes!("assets/icons/layout.svg"),
            Self::Activity => include_bytes!("assets/icons/activity.svg"),
            Self::WrapText => include_bytes!("assets/icons/wrap-text.svg"),
            Self::Shuffle => include_bytes!("assets/icons/shuffle.svg"),
            Self::Panel => include_bytes!("assets/icons/panel.svg"),
            Self::ArrowDown => include_bytes!("assets/icons/arrow-down.svg"),
            Self::ArrowRight => include_bytes!("assets/icons/arrow-right.svg"),
            Self::ArrowLeft => include_bytes!("assets/icons/arrow-left.svg"),
            Self::ChevronRight => include_bytes!("assets/icons/chevron-right.svg"),
            Self::MoreHorizontal => include_bytes!("assets/icons/more-horizontal.svg"),
            Self::ChevronDown => include_bytes!("assets/icons/chevron-down.svg"),
            Self::ChevronUp => include_bytes!("assets/icons/chevron-up.svg"),
            Self::CheckStroke => include_bytes!("assets/icons/check-stroke.svg"),
            Self::CheckboxGroupCheck => include_bytes!("assets/icons/checkbox-group-check.svg"),
            Self::X => include_bytes!("assets/icons/x.svg"),
            Self::FolderCode => include_bytes!("assets/icons/folder-code.svg"),
            Self::CircleAlert => include_bytes!("assets/icons/circle-alert.svg"),
            Self::Terminal => include_bytes!("assets/icons/terminal.svg"),
            Self::ChevronsUpDown => include_bytes!("assets/icons/chevrons-up-down.svg"),
            Self::Plus => include_bytes!("assets/icons/plus.svg"),
            Self::Minus => include_bytes!("assets/icons/minus.svg"),
            Self::Bold => include_bytes!("assets/icons/bold.svg"),
            Self::Italic => include_bytes!("assets/icons/italic.svg"),
            Self::Underline => include_bytes!("assets/icons/underline.svg"),
            Self::BadgeCheck => include_bytes!("assets/icons/badge-check.svg"),
            Self::Bookmark => include_bytes!("assets/icons/bookmark.svg"),
            Self::PlusThin => include_bytes!("assets/icons/plus-thin.svg"),
            Self::MinusThin => include_bytes!("assets/icons/minus-thin.svg"),
            Self::BoldFilled => include_bytes!("assets/icons/bold-filled.svg"),
            Self::ItalicFilled => include_bytes!("assets/icons/italic-filled.svg"),
            Self::UnderlineFilled => include_bytes!("assets/icons/underline-filled.svg"),
            Self::TriangleRight => include_bytes!("assets/icons/triangle-right.svg"),
            Self::TriangleDown => include_bytes!("assets/icons/triangle-down.svg"),
            Self::Check => include_bytes!("assets/icons/check.svg"),
            Self::MinusThick => include_bytes!("assets/icons/minus-thick.svg"),
            Self::HeartFilled => include_bytes!("assets/icons/heart-filled.svg"),
            Self::HeartOutline => include_bytes!("assets/icons/heart-outline.svg"),
            Self::MinusNonScaling => include_bytes!("assets/icons/minus-non-scaling.svg"),
            Self::CaretUpDown => include_bytes!("assets/icons/caret-up-down.svg"),
            Self::TriangleUp => include_bytes!("assets/icons/triangle-up.svg"),
            Self::AlignLeft => include_bytes!("assets/icons/align-left.svg"),
            Self::AlignCenter => include_bytes!("assets/icons/align-center.svg"),
            Self::AlignRight => include_bytes!("assets/icons/align-right.svg"),
        }
    }
}
#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    style: StyleRefinement,
    rotation: gpui::Radians,
    stroke_width: Option<f32>,
    stroke_progress: Option<f32>,
    stroke_direction: SvgStrokeDirection,
}
impl Icon {
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            style: StyleRefinement::default(),
            rotation: gpui::radians(0.),
            stroke_width: None,
            stroke_progress: None,
            stroke_direction: SvgStrokeDirection::Forward,
        }
    }
    pub fn rotate(mut self, rotation: gpui::Radians) -> Self {
        self.rotation = rotation;
        self
    }
    pub fn stroke_width(mut self, width: f32) -> Self {
        assert!(
            width.is_finite() && width >= 0.,
            "Icon stroke width must be finite and nonnegative: {width}"
        );
        self.stroke_width = Some(width);
        self
    }
    pub fn stroke_progress(mut self, progress: f32) -> Self {
        assert!(
            progress.is_finite() && (0. ..=1.).contains(&progress),
            "Icon stroke progress must be finite and between zero and one: {progress}"
        );
        self.stroke_progress = Some(progress);
        self
    }
    pub fn stroke_direction(mut self, direction: SvgStrokeDirection) -> Self {
        self.stroke_direction = direction;
        self
    }
    pub fn size(mut self, size: f32) -> Self {
        self.style = self.style.size(px(size));
        self
    }
}
impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        use gpui::Refineable;
        let size = window.element_context::<IconSize>().map_or(16., |s| s.0);
        let mut icon = gpui::svg()
            .data(self.name.bytes())
            .with_transformation(gpui::Transformation::rotate(self.rotation))
            .size(px(size))
            .flex_none();
        if let Some(width) = self.stroke_width {
            icon = icon.stroke_width(width);
        }
        if let Some(progress) = self.stroke_progress {
            icon = icon
                .stroke_progress(progress)
                .stroke_direction(self.stroke_direction);
        }
        icon.style().refine(&self.style);
        icon
    }
}
