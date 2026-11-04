//! Application-owned UI primitives built on unstyled GPUI behavior.

mod animated_height;
mod breadcrumb;
mod bubble;
mod button;
mod button_group;
mod card;
mod checkbox;
mod code_editor;
mod collapsible;
mod command;
mod dialog;
mod dropdown_menu;
mod empty;
mod fade_truncate;
mod hover_card;
mod icon;
mod input;
mod input_group;
mod item;
mod kbd;
mod marker;
mod message;
mod message_scroller;
mod modal_backdrop;
mod popover;
mod questionnaire;
mod radio_group;
mod resize_handle;
mod responsive;
mod scroll_area;
mod searchable_popover;
mod select;
mod separator;
mod sheet;
mod shimmer_text;
mod sidebar;
mod skeleton;
mod spinner;
mod style_ext;
mod switch;
mod textarea;
mod thinking_indicator;
mod toggle_group;
mod tooltip;

/// Routes floating application UI to the platform overlay surface above
/// embedded native child views. Platform backends own the concrete surface.
pub(crate) fn floating_surface(
    element: impl gpui::IntoElement,
    priority: usize,
) -> impl gpui::IntoElement {
    gpui::deferred(element)
        .with_priority(priority)
        .on_native_overlay()
}

pub use animated_height::AnimatedHeight;
pub use breadcrumb::{Breadcrumb, BreadcrumbDensity, BreadcrumbItem};
pub use bubble::{
    Bubble, BubbleAlign, BubbleContent, BubbleGroup, BubbleReactionAlign, BubbleReactionSide,
    BubbleReactions, BubbleVariant,
};
pub use button::{Button, ButtonClickHandler, ButtonContentAlignment, ButtonSize, ButtonVariant};
pub use button_group::{
    ButtonGroup, ButtonGroupOrientation, ButtonGroupSeparator, ButtonGroupText,
};
pub use card::{
    Card, CardAction, CardContent, CardDescription, CardFooter, CardHeader, CardSize, CardTitle,
};
pub use checkbox::Checkbox;
pub use code_editor::{
    CodeEditor, SourceLineHighlights, code_editor_state, highlight_source_lines,
};
pub use collapsible::CollapsibleContent;
pub use command::{
    Command, CommandDensity, CommandDialog, CommandGroup, CommandItem, CommandSearchStyle,
    CommandState,
};
pub use dialog::Dialog;
pub use dialog::{DialogHost, DialogHostGlobal};
pub use dialog::{
    close_dialog, open_action_dialog, open_destructive_confirmation, open_dialog,
    open_dialog_with_dismiss, open_guarded_action_dialog,
};
pub use dropdown_menu::{
    DropdownMenu, DropdownMenuCheckboxItem, DropdownMenuDensity, DropdownMenuItem,
    DropdownMenuItemVariant, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuSub,
};
pub use empty::{Empty, EmptyMediaVariant};
pub use fade_truncate::FadeTruncate;
pub use gpui_base::input::{InputEvent, InputState, TextareaState};
pub use hover_card::{HoverCard, HoverCardAlign, HoverCardSide};
pub use icon::{Assets, BrandIcon, BrandIconName, Icon, IconName};
pub use input::{Input, InputSize};
pub use input_group::{InputGroup, InputGroupAddon, InputGroupAddonAlign};
pub use item::{
    Item, ItemActions, ItemContent, ItemDescription, ItemGroup, ItemGroupVariant, ItemMedia,
    ItemMediaVariant, ItemSeparator, ItemSize, ItemTitle, ItemVariant,
};
pub use kbd::{Kbd, KbdGroup};
pub use marker::{Marker, MarkerContent, MarkerIcon, MarkerVariant};
pub use message::{
    Message, MessageAlign, MessageAvatar, MessageChromeSpacing, MessageContent, MessageFooter,
    MessageGroup, MessageHeader,
};
pub use message_scroller::{
    MessageScroller, MessageScrollerAlign, MessageScrollerConfig, MessageScrollerDefaultPosition,
    MessageScrollerDirection, MessageScrollerItem, MessageScrollerModel, MessageScrollerScrollbar,
    MessageScrollerScrollbarPlacement, MessageScrollerSnapshot, MessageScrollerState,
};
#[cfg(debug_assertions)]
pub use message_scroller::{MessageScrollerTraceSnapshot, message_scroller_trace_snapshot};
pub use popover::{
    Popover, PopoverAlign, PopoverDescription, PopoverHeader, PopoverSide, PopoverState,
    PopoverTitle,
};
pub use questionnaire::{
    Questionnaire, QuestionnaireAnswer, QuestionnaireChoiceDefinition,
    QuestionnaireInputDefinition, QuestionnaireItemDefinition, QuestionnaireItemStatus,
    QuestionnaireState,
};
pub use radio_group::{RadioGroup, RadioGroupItem};
pub use resize_handle::{
    ResizeEndHandler, ResizeMoveHandler, ResizeStartHandler, VerticalResizeHandle,
};
pub use responsive::{COMPACT_BREAKPOINT, is_compact};
pub use scroll_area::{ScrollArea, ScrollAreaOrientation, scrollbar_track_width};
pub use searchable_popover::{SearchablePopover, SearchablePopoverItem, SearchablePopoverState};
pub use select::{Select, SelectItem};
pub use separator::{Separator, SeparatorOrientation};
pub use sheet::{Sheet, SheetSide};
pub use sheet::{SheetHost, SheetHostGlobal};
pub use shimmer_text::{ShimmerDirection, ShimmerPlayback, ShimmerText};
pub use sidebar::{
    SIDEBAR_TRANSITION_DURATION, SIDEBAR_WIDTH, SIDEBAR_WIDTH_ICON, SIDEBAR_WIDTH_MOBILE, Sidebar,
    SidebarBorder, SidebarCollapsible, SidebarCollapsibleContent, SidebarContent,
    SidebarConversationActions, SidebarConversationDetails, SidebarConversationMenu,
    SidebarConversationMenuButton, SidebarFooter, SidebarGroup, SidebarGroupAction,
    SidebarGroupContent, SidebarGroupLabel, SidebarHeader, SidebarHoverCard, SidebarInput,
    SidebarInset, SidebarMenu, SidebarMenuAction, SidebarMenuBadge, SidebarMenuButton,
    SidebarMenuItem, SidebarMenuSkeleton, SidebarMenuSub, SidebarMenuSubItem,
    SidebarProjectActions, SidebarProjectButton, SidebarProjectDetails, SidebarProjectMenu,
    SidebarProjectMenuButton, SidebarProvider, SidebarRail, SidebarSeparator, SidebarSide,
    SidebarState, SidebarTrigger, SidebarVariant, ToggleSidebar,
};
pub use skeleton::Skeleton;
pub use spinner::Spinner;
pub use style_ext::{TextStyleExt, TextStyleTokenExt};
pub use switch::{Switch, SwitchSize};
pub use textarea::Textarea;
pub use thinking_indicator::{ThinkingIndicator, ThinkingIndicatorSize};
pub use toggle_group::{ToggleGroup, ToggleGroupItem};
pub use tooltip::{
    Tooltip, TooltipAlign, TooltipContent, TooltipHoverHandler, TooltipProvider, TooltipSide,
    TooltipTrigger,
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct LayoutMeasurementKey {
    width: gpui::Pixels,
    rem_size: gpui::Pixels,
}

#[derive(Debug, Default)]
pub struct ListLayoutMeasurementState {
    key: Option<LayoutMeasurementKey>,
}

impl ListLayoutMeasurementState {
    pub fn sync(
        &mut self,
        list: &gpui::ListState,
        item_count: usize,
        width: gpui::Pixels,
        rem_size: gpui::Pixels,
    ) -> bool {
        let next_key = LayoutMeasurementKey { width, rem_size };
        let should_remeasure = self.key.is_some() && self.key != Some(next_key) && item_count > 0;
        self.key = Some(next_key);
        if should_remeasure {
            list.remeasure_items(0..item_count);
        }
        should_remeasure
    }
}

pub fn init(cx: &mut gpui::App) {
    popover::init(cx);
    sidebar::init(cx);
}

pub fn sync_base_theme(cx: &mut gpui::App) {
    scroll_area::sync_base_theme(cx);
}

#[cfg(test)]
mod tests {
    use gpui::{ListAlignment, ListState, px};

    use super::ListLayoutMeasurementState;

    #[test]
    fn layout_measurement_changes_after_width_or_rem_changes() {
        let list = ListState::new(1, ListAlignment::Top, px(0.0));
        let mut state = ListLayoutMeasurementState::default();

        assert!(!state.sync(&list, 1, px(600.0), px(16.0)));
        assert!(!state.sync(&list, 1, px(600.0), px(16.0)));
        assert!(state.sync(&list, 1, px(620.0), px(16.0)));
        assert!(!state.sync(&list, 1, px(620.0), px(16.0)));
        assert!(state.sync(&list, 1, px(620.0), px(18.0)));
    }
}
