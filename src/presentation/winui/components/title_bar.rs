//! A short title leaves space for Windows caption buttons at narrow widths.
use super::super::{
    app::{GearVRReactorApp, ReactorMessage},
    text::Text,
};
use windows_reactor::*;
pub fn render_title_bar(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
) -> View {
    TitleBar::new()
        .grid_row(0)
        .title(app.text(Text::AppName))
        .preferred_height(WindowTitleBarHeight::Standard)
        .is_back_button_visible(false)
        .is_pane_toggle_button_visible(true)
        .on_pane_toggle_requested(context.message(ReactorMessage::TogglePane))
        .into()
}
