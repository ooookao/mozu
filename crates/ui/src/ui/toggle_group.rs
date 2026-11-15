use crate::ui::toggle::{Toggle, ToggleSize, ToggleVariant};
use gpui::{
    AnyElement, App, ElementId, IntoElement, Orientation, ParentElement, Refineable, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, px,
};
use kinu_ui::{Selection, ToggleGroup as GroupPrimitive, ToggleGroupItem as ItemPrimitive};
use std::rc::Rc;

/// An editable toggle option whose activation and value belong to its group.
pub struct ToggleGroupItem<T> {
    id: ElementId,
    value: T,
    label: Option<SharedString>,
    disabled: bool,
    invalid: bool,
    icon_only: bool,
    icon_start: Option<AnyElement>,
    icon_end: Option<AnyElement>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}
impl<T> ToggleGroupItem<T> {
    pub fn new(id: impl Into<ElementId>, value: T) -> Self {
        Self {
            id: id.into(),
            value,
            label: None,
            disabled: false,
            invalid: false,
            icon_only: false,
            icon_start: None,
            icon_end: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
    /// Uses the common square icon-button geometry without guessing child types.
    pub fn icon_only(mut self, icon_only: bool) -> Self {
        self.icon_only = icon_only;
        self
    }
    pub fn icon_start(mut self, icon: impl IntoElement) -> Self {
        self.icon_start = Some(icon.into_any_element());
        self
    }
    pub fn icon_end(mut self, icon: impl IntoElement) -> Self {
        self.icon_end = Some(icon.into_any_element());
        self
    }
}
impl<T> Styled for ToggleGroupItem<T> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl<T> ParentElement for ToggleGroupItem<T> {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(children);
    }
}
type Change<T> = Rc<dyn Fn(&Selection<T>, &mut Window, &mut App)>;

/// Application-controlled grouped toggles sharing the installed toggle presentation.
#[derive(IntoElement)]
pub struct ToggleGroup<T: Clone + Eq + 'static> {
    id: ElementId,
    selection: Selection<T>,
    variant: ToggleVariant,
    size: Option<ToggleSize>,
    spacing: f32,
    orientation: Orientation,
    disabled: bool,
    label: Option<SharedString>,
    items: Vec<ToggleGroupItem<T>>,
    style: StyleRefinement,
    change: Option<Change<T>>,
}
impl<T: Clone + Eq + 'static> ToggleGroup<T> {
    pub fn new(id: impl Into<ElementId>, selection: Selection<T>) -> Self {
        Self {
            id: id.into(),
            selection,
            variant: ToggleVariant::Default,
            size: None,
            spacing: 2.,
            orientation: Orientation::Horizontal,
            disabled: false,
            label: None,
            items: Vec::new(),
            style: StyleRefinement::default(),
            change: None,
        }
    }
    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = Some(size);
        self
    }
    /// Uses quarter-rem spacing units, matching the installed spacing scale.
    pub fn spacing(mut self, spacing: f32) -> Self {
        assert!(
            spacing.is_finite() && spacing >= 0.,
            "toggle group spacing must be finite and nonnegative"
        );
        self.spacing = spacing;
        self
    }
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
    pub fn item(mut self, item: ToggleGroupItem<T>) -> Self {
        self.items.push(item);
        self
    }
    pub fn on_change(
        mut self,
        change: impl Fn(&Selection<T>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.change = Some(Rc::new(change));
        self
    }
}
impl<T: Clone + Eq + 'static> Styled for ToggleGroup<T> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl<T: Clone + Eq + 'static> RenderOnce for ToggleGroup<T> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut layout = StyleRefinement::default()
            .flex()
            .w_auto()
            .gap(px(self.spacing * 4.));
        layout = match self.orientation {
            Orientation::Horizontal => layout.flex_row().items_center(),
            Orientation::Vertical => layout.flex_col().items_stretch(),
        };
        let mut root = GroupPrimitive::new(self.id, self.selection.clone())
            .orientation(self.orientation)
            .disabled(self.disabled);
        root.style().refine(&layout);
        root.style().refine(&self.style);
        if let Some(label) = self.label {
            root = root.accessibility_label(label);
        }
        for item in self.items {
            let mut toggle = Toggle::new(item.id, self.selection.contains(&item.value))
                .variant(self.variant)
                .disabled(self.disabled || item.disabled)
                .invalid(item.invalid)
                .icon_only(item.icon_only);
            if let Some(size) = self.size {
                toggle = toggle.size(size);
            }
            if let Some(label) = item.label {
                toggle = toggle.accessibility_label(label);
            }
            if let Some(icon) = item.icon_start {
                toggle = toggle.icon_start(icon);
            }
            toggle = toggle.children(item.children);
            if let Some(icon) = item.icon_end {
                toggle = toggle.icon_end(icon);
            }
            toggle.style().refine(&item.style);
            root = root.item(ItemPrimitive::from_toggle(
                item.value,
                toggle.into_primitive(window, cx),
            ));
        }
        if let Some(change) = self.change {
            root = root.on_change(move |selection, window, cx| change(selection, window, cx));
        }
        root
    }
}
