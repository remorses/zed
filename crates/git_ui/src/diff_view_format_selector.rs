use crate::diff_view::DiffView;
use editor::{DiffViewFormat, Editor};
use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Render, WeakEntity,
    Window,
};
use picker::{Picker, PickerDelegate};
use ui::{ListItem, prelude::*};
use workspace::{ModalView, Workspace};

pub fn register(workspace: &mut Workspace) {
    workspace.register_action(open);
}

pub fn open(
    workspace: &mut Workspace,
    _: &zed_actions::diff_view_format_selector::Toggle,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    let Some((item, _)) = workspace.active_item(cx) else {
        return;
    };
    let Some(diff_view) = item.act_as::<DiffView>(cx) else {
        return;
    };
    workspace.toggle_modal(window, cx, |window, cx| {
        DiffViewFormatSelector::new(diff_view, window, cx)
    });
}

pub struct DiffViewFormatSelector {
    diff_view: Entity<DiffView>,
    picker: Entity<Picker<DiffViewFormatSelectorDelegate>>,
}

impl DiffViewFormatSelector {
    fn new(diff_view: Entity<DiffView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let current = diff_view.read(cx).editor.read(cx).diff_view_format();
        let delegate = DiffViewFormatSelectorDelegate::new(
            cx.entity().downgrade(),
            diff_view.clone(),
            current,
        );
        let picker = cx.new(|cx| Picker::nonsearchable_uniform_list(delegate, window, cx));
        Self { diff_view, picker }
    }
}

impl Render for DiffViewFormatSelector {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().w(rems(20.)).child(self.picker.clone())
    }
}

impl Focusable for DiffViewFormatSelector {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl EventEmitter<DismissEvent> for DiffViewFormatSelector {}
impl ModalView for DiffViewFormatSelector {}

struct DiffViewFormatSelectorDelegate {
    selector: WeakEntity<DiffViewFormatSelector>,
    diff_view: Entity<DiffView>,
    formats: [DiffViewFormat; 3],
    selected_index: usize,
}

impl DiffViewFormatSelectorDelegate {
    fn new(
        selector: WeakEntity<DiffViewFormatSelector>,
        diff_view: Entity<DiffView>,
        current: DiffViewFormat,
    ) -> Self {
        let formats = [
            DiffViewFormat::Unified,
            DiffViewFormat::AdditionsOnly,
            DiffViewFormat::DeletionsOnly,
        ];
        let selected_index = formats.iter().position(|&f| f == current).unwrap_or(0);
        Self {
            selector,
            diff_view,
            formats,
            selected_index,
        }
    }
}

impl PickerDelegate for DiffViewFormatSelectorDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _window: &mut Window, _cx: &mut App) -> Arc<str> {
        "Select Diff View Format".into()
    }

    fn match_count(&self) -> usize {
        self.formats.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(
        &mut self,
        ix: usize,
        _window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) {
        self.selected_index = ix.min(self.formats.len() - 1);
        cx.notify();
    }

    fn confirm(&mut self, _secondary: bool, _window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let format = self.formats[self.selected_index];
        self.diff_view
            .update(cx, |view, cx| {
                view.editor
                    .update(cx, |editor, cx| editor.set_diff_view_format(format, cx));
            })
            .ok();
        self.selector
            .update(cx, |_this, cx| cx.emit(DismissEvent))
            .ok();
    }

    fn dismissed(&mut self, _window: &mut Window, cx: &mut Context<Picker<Self>>) {
        self.selector
            .update(cx, |_this, cx| cx.emit(DismissEvent))
            .ok();
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) -> Option<ListItem> {
        let label = match self.formats.get(ix)? {
            DiffViewFormat::Unified => "Unified",
            DiffViewFormat::AdditionsOnly => "Additions Only",
            DiffViewFormat::DeletionsOnly => "Deletions Only",
        };
        Some(
            ListItem::new(ix)
                .toggle_state(selected)
                .child(Label::new(label)),
        )
    }
}
