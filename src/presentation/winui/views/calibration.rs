//! Calibration Tab View: Dynamic Touchpad Normalization and Gyroscope Zero-Drift Compensation

use crate::domain::i18n::I18nStrings;
use crate::domain::models::ConnectionStatus;
use crate::presentation::winui::app::GearVRReactorApp;
use crate::presentation::winui::components::cards::render_card;
use crate::presentation::winui::tokens::FluentTokens;
use windows_reactor::*;

pub fn render_calibration_view(app: &GearVRReactorApp, s: &I18nStrings) -> View {
    // Dynamic Touchpad Calibration: compute radial displacement [0.0, 100.0]
    let (touch_progress, touch_status_text) = if let Some(d) = &app.latest_data {
        let mag = (d.processed_touchpad_x.powi(2) + d.processed_touchpad_y.powi(2)).sqrt();
        let val = (mag.min(1.0) * 100.0) as f64;
        (
            val,
            format!(
                "{} (X: {:+.2}, Y: {:+.2})",
                s.touch_cal_status, d.processed_touchpad_x, d.processed_touchpad_y
            ),
        )
    } else {
        (100.0, s.touch_cal_status.to_string())
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
            ))
            .into(),
    );

    // Gyroscope Calibration: indeterminate running bar indicates real-time drift cancellation
    let imu_progress_bar = if app.connection_status == ConnectionStatus::Connected {
        ProgressBar::new().is_indeterminate(true)
    } else {
        ProgressBar::new().is_indeterminate(false).value(0.0)
    };

    let imu_card = render_card(
        s.imu_cal_title,
        s.imu_cal_desc,
        StackPanel::new()
            .spacing(FluentTokens::SPACING_MD)
            .children((
                imu_progress_bar,
                TextBlock::new()
                    .text(s.imu_cal_status)
                    .font_size(FluentTokens::FONT_BODY),
                TextBlock::new()
                    .text(s.imu_cal_filter)
                    .font_size(FluentTokens::FONT_CAPTION),
            ))
            .into(),
    );

    StackPanel::new()
        .spacing(FluentTokens::SPACING_XL)
        .children((
            touch_card,
            imu_card,
        ))
        .into()
}
