//! A bounded page keeps navigation and save actions outside the scroll region.
use super::super::{
    app::{GearVRReactorApp, ReactorMessage},
    state::Page,
};
use windows_reactor::*;

pub fn render_page_layout(
    app: &GearVRReactorApp,
    context: &ViewContext<GearVRReactorApp>,
    content: View,
) -> View {
    let notice = InfoBar::new()
        .title(app.text(app.ui.page.title()))
        .message(app.ui.notice.map(|text| app.text(text)).unwrap_or_default())
        .severity(match app.ui.severity {
            crate::domain::models::MessageSeverity::Error => InfoBarSeverity::Error,
            crate::domain::models::MessageSeverity::Warning => InfoBarSeverity::Warning,
            crate::domain::models::MessageSeverity::Success => InfoBarSeverity::Success,
            crate::domain::models::MessageSeverity::Info => InfoBarSeverity::Informational,
        })
        .is_open(app.ui.notice.is_some())
        .is_closable(true)
        .on_closed(context.message(ReactorMessage::DismissNotice));
    let heading = TextBlock::new()
        .text(app.text(app.ui.page.title()))
        .font_size(26.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .automation_heading_level(AutomationHeadingLevel::Level1);
    let secondary: View = if app.ui.subpages().is_empty() {
        StackPanel::new().into()
    } else {
        SelectorBar::new()
            .on_selected_text_changed(context.callback(ReactorMessage::Subpage))
            .slots([SlotView::collection(
                SelectorBarSlot::Items,
                app.ui.subpages().iter().map(|page| {
                    KeyedView::new(
                        format!("{page:?}"),
                        SelectorBarItem::new()
                            .text(app.text(page.title()))
                            .is_selected(app.ui.active_subpage() == Some(*page)),
                    )
                }),
            )])
    };
    let footer: View = if matches!(app.ui.page, Page::Tuning | Page::Settings) || app.ui.dirty() {
        super::controls::save_bar(app, context)
    } else {
        StackPanel::new().into()
    };
    let body_key = format!("{:?}/{:?}", app.ui.page, app.ui.active_subpage());
    Border::new()
        .padding(Thickness::new(24.0, 20.0, 24.0, 20.0))
        .content(
            Grid::new()
                .max_width(880.0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .rows([
                    GridLength::Auto,
                    GridLength::Auto,
                    GridLength::STAR,
                    GridLength::Auto,
                ])
                .row_spacing(16.0)
                .children((
                    StackPanel::new()
                        .grid_row(0)
                        .spacing(12.0)
                        .children((heading, notice)),
                    Border::new().grid_row(1).content(secondary),
                    // Changing routes resets that page's scroll position, not application state.
                    Border::new().grid_row(2).content(
                        Grid::new().keyed_children([KeyedView::new(
                            body_key,
                            ScrollViewer::new()
                                .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                                .content(
                                    Border::new()
                                        .vertical_alignment(VerticalAlignment::Top)
                                        .padding(Thickness::new(0.0, 0.0, 8.0, 12.0))
                                        .content(content),
                                ),
                        )]),
                    ),
                    Border::new().grid_row(3).content(footer),
                )),
        )
}
