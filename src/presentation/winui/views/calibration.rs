//! Tuning, guided calibration and a desktop-isolated input preview.
use super::super::{
    app::{GearVRReactorApp, ReactorMessage},
    components::controls::{button, paragraph, section, slider, toggle},
    state::{BooleanPreference as Bool, NumericPreference as Number, Page, Subpage},
    text::Text,
};
use crate::domain::{
    calibration::{CalibrationKind, CalibrationStatus},
    models::OutputTarget,
};
use windows_reactor::*;

pub fn render_calibration_view(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
) -> View {
    match app.ui.tuning_page {
        Subpage::Calibration => calibration(app, context),
        Subpage::Test => preview(app, context),
        _ => StackPanel::new().spacing(20.0).children((
            paragraph(app.text(Text::InputHint)),
            input_preferences(app, context),
        )),
    }
}
fn input_preferences(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let input = &app.ui.draft.input;
    let advanced = Expander::new().slots([
        SlotView::new(
            ExpanderSlot::Header,
            paragraph(app.text(Text::AdvancedInput)),
        ),
        SlotView::new(
            ExpanderSlot::Content,
            StackPanel::new().spacing(14.0).children((
                toggle(
                    app,
                    context,
                    Text::Smoothing,
                    Bool::Smoothing,
                    input.smoothing,
                ),
                paragraph(app.text(Text::SmoothingHint)),
                slider(
                    app,
                    context,
                    Text::SmoothingSamples,
                    Number::SmoothingSamples,
                    input.smoothing_samples as f64,
                    (1.0, 64.0, 1.0),
                ),
                slider(
                    app,
                    context,
                    Text::DeadZone,
                    Number::DeadZone,
                    input.dead_zone,
                    (0.0, 0.9, 0.01),
                ),
                toggle(
                    app,
                    context,
                    Text::Acceleration,
                    Bool::Acceleration,
                    input.acceleration,
                ),
                slider(
                    app,
                    context,
                    Text::AccelerationPower,
                    Number::AccelerationPower,
                    input.acceleration_power,
                    (1.0, 3.0, 0.1),
                ),
                toggle(
                    app,
                    context,
                    Text::EdgeMotion,
                    Bool::EdgeMotion,
                    input.edge_motion,
                ),
                paragraph(app.text(Text::EdgeHint)),
            )),
        ),
    ]);
    StackPanel::new().spacing(14.0).children((
        slider(
            app,
            context,
            Text::AirSpeed,
            Number::AirSpeed,
            input.air_sensitivity,
            (0.1, 20.0, 0.1),
        ),
        slider(
            app,
            context,
            Text::TouchSpeed,
            Number::TouchSpeed,
            input.touch_sensitivity,
            (0.1, 20.0, 0.1),
        ),
        toggle(
            app,
            context,
            Text::NaturalScroll,
            Bool::NaturalScroll,
            input.natural_scroll,
        ),
        advanced,
        button(
            app,
            context,
            Text::RestoreInput,
            ReactorMessage::RestoreInput,
        ),
    ))
}
fn calibration(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let available = app.ui.connected() && !app.ui.calibration.is_collecting();
    let status = paragraph(
        app.ui
            .calibration_text()
            .map(|text| app.text(text))
            .unwrap_or_default(),
    );
    let controls: View = if !app.ui.connected() {
        StackPanel::new().spacing(8.0).children((
            paragraph(app.text(Text::ConnectFirst)),
            button(
                app,
                context,
                Text::GoControl,
                ReactorMessage::Navigate(Page::Control),
            ),
        ))
    } else {
        let ready = matches!(
            app.ui.calibration,
            CalibrationStatus::Collecting {
                kind: CalibrationKind::Touchpad,
                ready: true,
                ..
            }
        );
        let progress = match app.ui.calibration {
            CalibrationStatus::Collecting { progress, .. } => f64::from(progress) * 100.0,
            CalibrationStatus::Complete(_) => 100.0,
            _ => 0.0,
        };
        StackPanel::new().spacing(14.0).children((
            paragraph(app.text(Text::GyroHint)),
            button(app, context, Text::StartGyro, ReactorMessage::CalibrateGyro)
                .is_enabled(available),
            paragraph(app.text(Text::TouchCalHint)),
            button(
                app,
                context,
                Text::StartTouch,
                ReactorMessage::CalibrateTouch,
            )
            .is_enabled(available),
            ProgressBar::new()
                .value(progress)
                .automation_name(app.text(Text::Calibration)),
            status,
            button(
                app,
                context,
                Text::SaveCalibration,
                ReactorMessage::SaveCalibration,
            )
            .is_enabled(ready),
            button(
                app,
                context,
                Text::Cancel,
                ReactorMessage::CancelCalibration,
            )
            .is_enabled(app.ui.calibration.is_collecting()),
        ))
    };
    section(app, Text::Calibration, Text::TuneHint, controls)
}
fn preview(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let active = app.ui.output == OutputTarget::Preview;
    let target = if active {
        OutputTarget::Paused
    } else {
        OutputTarget::Preview
    };
    let sample = &app.ui.preview;
    let readings = if active {
        format!(
            "X: {:+}    Y: {:+}\n{}: {}    {}: {}",
            sample.x,
            sample.y,
            app.text(Text::Clicks),
            sample.clicks,
            app.text(Text::Scroll),
            sample.scroll
        )
    } else {
        app.text(Text::InputPaused).to_string()
    };
    section(
        app,
        Text::TestArea,
        Text::TestHint,
        StackPanel::new().spacing(12.0).children((
            paragraph(readings),
            button(
                app,
                context,
                if active {
                    Text::StopTest
                } else {
                    Text::StartTest
                },
                ReactorMessage::SetOutput(target),
            )
            .is_enabled(app.ui.can_output()),
        )),
    )
}
