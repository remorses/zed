use editor::Editor;
use gpui::{
    Element, EventEmitter, IntoElement, ParentElement, Render, StyledText, Subscription,
    ViewContext,
};
use itertools::Itertools;
use std::cmp;
use theme::ActiveTheme;
use ui::{prelude::*, ButtonLike, ButtonStyle, Label, Tooltip};
use workspace::{
    item::{BreadcrumbText, ItemEvent, ItemHandle},
    ToolbarItemEvent, ToolbarItemLocation, ToolbarItemView,
};

pub struct Breadcrumbs {
    pane_focused: bool,
    active_item: Option<Box<dyn ItemHandle>>,
    subscription: Option<Subscription>,
}

impl Default for Breadcrumbs {
    fn default() -> Self {
        Self::new()
    }
}

impl Breadcrumbs {
    pub fn new() -> Self {
        Self {
            pane_focused: false,
            active_item: Default::default(),
            subscription: Default::default(),
        }
    }
}

impl EventEmitter<ToolbarItemEvent> for Breadcrumbs {}

impl Render for Breadcrumbs {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        const MAX_SEGMENTS: usize = 12;
        let element = h_flex().text_ui(cx);
        let Some(active_item) = self.active_item.as_ref() else {
            return element;
        };
        let Some(mut segments) = active_item.breadcrumbs(cx.theme(), cx) else {
            return element;
        };

        let prefix_end_ix = cmp::min(segments.len(), MAX_SEGMENTS / 2);
        let suffix_start_ix = cmp::max(
            prefix_end_ix,
            segments.len().saturating_sub(MAX_SEGMENTS / 2),
        );
        if suffix_start_ix > prefix_end_ix {
            segments.splice(
                prefix_end_ix..suffix_start_ix,
                Some(BreadcrumbText {
                    text: "⋯".into(),
                    highlights: None,
                    font: None,
                }),
            );
        }

        let highlighted_segments = segments.into_iter().map(|segment| {
            let mut text_style = cx.text_style();
            if let Some(font) = segment.font {
                text_style.font_family = font.family;
                text_style.font_features = font.features;
                text_style.font_style = font.style;
                text_style.font_weight = font.weight;
            }
            text_style.color = Color::Muted.color(cx);

            StyledText::new(segment.text.replace('\n', "␤"))
                .with_highlights(&text_style, segment.highlights.unwrap_or_default())
                .into_any()
        });
        let breadcrumbs = Itertools::intersperse_with(highlighted_segments, || {
            Label::new("›").color(Color::Placeholder).into_any_element()
        });

        let breadcrumbs_stack = h_flex().gap_1().children(breadcrumbs);
        match active_item
            .downcast::<Editor>()
            .map(|editor| editor.downgrade())
        {
            Some(editor) => element.child(
                ButtonLike::new("toggle outline view")
                    .child(breadcrumbs_stack)
                    .style(ButtonStyle::Transparent)
                    .on_click(move |_, cx| {
                        if let Some(editor) = editor.upgrade() {
                            outline::toggle(editor, &editor::actions::ToggleOutline, cx)
                        }
                    })
                    .tooltip(|cx| {
                        Tooltip::for_action(
                            "Show symbol outline",
                            &editor::actions::ToggleOutline,
                            cx,
                        )
                    }),
            ),
            None => element
                // Match the height of the `ButtonLike` in the other arm.
                .h(rems_from_px(22.))
                .child(breadcrumbs_stack),
        }
    }
}

impl ToolbarItemView for Breadcrumbs {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        cx: &mut ViewContext<Self>,
    ) -> ToolbarItemLocation {
        cx.notify();
        self.active_item = None;

        let Some(item) = active_pane_item else {
            return ToolbarItemLocation::Hidden;
        };

        let this = cx.view().downgrade();
        self.subscription = Some(item.subscribe_to_item_events(
            cx,
            Box::new(move |event, cx| {
                if let ItemEvent::UpdateBreadcrumbs = event {
                    this.update(cx, |this, cx| {
                        cx.notify();
                        if let Some(active_item) = this.active_item.as_ref() {
                            cx.emit(ToolbarItemEvent::ChangeLocation(
                                active_item.breadcrumb_location(cx),
                            ))
                        }
                    })
                    .ok();
                }
            }),
        ));
        self.active_item = Some(item.boxed_clone());
        item.breadcrumb_location(cx)
    }

    fn pane_focus_update(&mut self, pane_focused: bool, _: &mut ViewContext<Self>) {
        self.pane_focused = pane_focused;
    }
}
