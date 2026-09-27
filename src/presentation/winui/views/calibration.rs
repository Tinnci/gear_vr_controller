//! Calibration Tab View: Dynamic Touchpad Normalization and Gyroscope Zero-Drift Compensation

use crate::domain::i18n::I18nStrings;
use crate::domain::models::ConnectionStatus;
use crate::presentation::winui::app::{GearVRReactorApp, ReactorMessage};
use crate::presentation::winui::components::cards::render_card;
use crate::presentation::winui::tokens::FluentTokens;
use windows_reactor::*;

pub fn render_calibration_view(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
    s: &I18nStrings,
) -> View {
    // Dynamic Touchpad Calibration: compute radial displacement [0.0, 100.0]
    let (touch_progress, touch_status_text) = if let Some(d) = &app.latest_data {
        let mag = (d.processed_touchpad_x.powi(2) + d.processed_touchpad_y.powi(2)).sqrt();
        let val = mag.min(1.0) * 100.0;
        (
            val,
            format!(
                "{} (X: {:+.2}, Y: {:+.2})",
                s.touch_cal_status, d.processed_touchpad_x, d.processed_touchpad_y
            ),
        )
    } else {
        (0.0, s.touch_cal_status.to_string())
    };

    let touch_card = render_card(
        s.touch_cal_title,
        s.touch_cal_desc,
        StackPanel::new()
            .spacing(FluentTokens::SPACING_MD)
            .children((
                ProgressBar::new().value(touch_progress),
                TextBlock::new()
                    .text(touch_status_text)
                    .font_size(FluentTokens::FONT_CAPTION),
                Button::new()
                    .is_enabled(app.connection_status == ConnectionStatus::Connected)
                    .on_click(context.message(ReactorMessage::StartTouchCalibration))
                    .content(app.language.action_text("touch_start")),
                Button::new()
                    .is_enabled(app.connection_status == ConnectionStatus::Connected)
                    .on_click(context.message(ReactorMessage::FinishTouchCalibration))
                    .content(app.language.action_text("touch_save")),
            )),
    );

    let imu_progress_bar = ProgressBar::new().value(if app.imu_completed {
        100.0
    } else {
        app.imu_progress.unwrap_or(0.0) as f64 * 100.0
    });

    let imu_card = render_card(
        s.imu_cal_title,
        s.imu_cal_desc,
        StackPanel::new()
            .spacing(FluentTokens::SPACING_MD)
            .children((
                imu_progress_bar,
                Button::new()
                    .is_enabled(
                        app.connection_status == ConnectionStatus::Connected
                            && app.imu_progress.is_none(),
                    )
                    .on_click(context.message(ReactorMessage::CalibrateImu))
                    .content(app.language.action_text("imu_start")),
                TextBlock::new()
                    .text(if app.imu_completed {
                        app.language.action_text("imu_done")
                    } else {
                        s.imu_cal_status
                    })
                    .font_size(FluentTokens::FONT_BODY),
                TextBlock::new()
                    .text(s.imu_cal_filter)
                    .font_size(FluentTokens::FONT_CAPTION),
            )),
    );

    StackPanel::new()
        .spacing(FluentTokens::SPACING_XL)
        .children((touch_card, imu_card))
}
