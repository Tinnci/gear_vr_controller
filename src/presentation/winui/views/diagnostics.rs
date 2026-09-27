//! Diagnostics Tab View: Raw sensor vector feed and Bluetooth recovery InfoBar

use crate::domain::i18n::I18nStrings;
use crate::presentation::winui::app::{GearVRReactorApp, ReactorMessage};
use crate::presentation::winui::components::cards::render_card;
use crate::presentation::winui::components::formatters::format_imu_diagnostics;
use crate::presentation::winui::tokens::FluentTokens;
use windows_reactor::*;

pub fn render_diagnostics_view(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
    s: &I18nStrings,
) -> View {
    let (accel, gyro, mag) = format_imu_diagnostics(app.latest_data.as_ref());

    let imu_card = render_card(
        s.imu_diag_title,
        s.imu_diag_desc,
        StackPanel::new()
            .spacing(FluentTokens::SPACING_SM)
            .children((
                TextBlock::new().text(accel).font_size(FluentTokens::FONT_BODY),
                TextBlock::new().text(gyro).font_size(FluentTokens::FONT_BODY),
                TextBlock::new().text(mag).font_size(FluentTokens::FONT_BODY),
            )),
    );

    let bt_infobar = InfoBar::new()
        .is_open(true)
        .is_closable(false)
        .severity(InfoBarSeverity::Warning)
        .title(s.bt_recovery_title)
        .message(s.bt_troubleshoot_hint);

    let bt_recovery_card = render_card(
        s.bt_recovery_title,
        s.bt_recovery_desc,
        StackPanel::new()
            .spacing(FluentTokens::SPACING_XL)
            .children((
                bt_infobar,
                Button::new()
                    .style(ButtonStyle::Default)
                    .on_click(context.message(ReactorMessage::OpenBtSettings))
                    .content(s.open_bt_settings),
            )),
    );

    StackPanel::new()
        .spacing(FluentTokens::SPACING_XL)
        .children((
            imu_card,
            bt_recovery_card,
        ))
}
