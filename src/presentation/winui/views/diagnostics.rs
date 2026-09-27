//! Recovery is progressive. Sensor vectors and native errors live in details.
use super::super::{
    app::{GearVRReactorApp, ReactorMessage},
    components::controls::{button, paragraph, section},
    text::Text,
};
use windows_reactor::*;

pub fn render_diagnostics_view(
    app: &GearVRReactorApp,
    context: &mut ViewContext<GearVRReactorApp>,
) -> View {
    let details = Expander::new().slots([
        SlotView::new(ExpanderSlot::Header, paragraph(app.text(Text::Details))),
        SlotView::new(
            ExpanderSlot::Content,
            StackPanel::new().spacing(14.0).children((
                paragraph(if app.ui.diagnostic_details.is_empty() {
                    app.text(Text::NoDetails)
                } else {
                    &app.ui.diagnostic_details
                }),
                sensors(app),
                button(app, context, Text::OpenLogs, ReactorMessage::OpenLogs),
            )),
        ),
    ]);
    StackPanel::new().spacing(20.0).children((
        paragraph(app.text(Text::HelpHint)),
        section(
            app,
            Text::Connect,
            Text::PairingHelp,
            StackPanel::new().spacing(8.0).children((
                button(
                    app,
                    context,
                    Text::BluetoothSettings,
                    ReactorMessage::OpenBluetooth,
                ),
                button(
                    app,
                    context,
                    Text::GoControl,
                    ReactorMessage::Navigate(super::super::state::Page::Control),
                ),
            )),
        ),
        recovery(app, context),
        details,
        paragraph(app.text(Text::ExportHint)),
        button(
            app,
            context,
            Text::Export,
            ReactorMessage::ExportDiagnostics,
        ),
    ))
}
fn recovery(app: &GearVRReactorApp, context: &ViewContext<GearVRReactorApp>) -> View {
    let controls = if app.ui.recovery_confirm {
        StackPanel::new().spacing(8.0).children((
            button(
                app,
                context,
                Text::ConfirmRecovery,
                ReactorMessage::RecoverBluetooth,
            )
            .is_enabled(!app.ui.recovery_running),
            button(app, context, Text::Cancel, ReactorMessage::CancelRecovery),
        ))
    } else {
        StackPanel::new().spacing(8.0).children((
            button(
                app,
                context,
                Text::Recovery,
                ReactorMessage::ConfirmRecovery,
            )
            .is_enabled(!app.ui.recovery_running),
            paragraph(if app.ui.recovery_running {
                app.text(Text::RecoveryRunning)
            } else {
                ""
            }),
        ))
    };
    section(app, Text::Recovery, Text::RecoveryImpact, controls)
}
fn sensors(app: &GearVRReactorApp) -> View {
    let Some(data) = app.ui.latest.as_ref() else {
        return paragraph(app.text(Text::NoData));
    };
    StackPanel::new().spacing(8.0).children([
        paragraph(format!(
            "{} (g): X {:+.4} · Y {:+.4} · Z {:+.4}",
            app.text(Text::Accel),
            data.accel_x,
            data.accel_y,
            data.accel_z
        )),
        paragraph(format!(
            "{} (rad/s): X {:+.4} · Y {:+.4} · Z {:+.4}",
            app.text(Text::Gyro),
            data.gyro_x,
            data.gyro_y,
            data.gyro_z
        )),
        paragraph(format!(
            "{} (µT): X {:+.4} · Y {:+.4} · Z {:+.4}",
            app.text(Text::Magnet),
            data.mag_x,
            data.mag_y,
            data.mag_z
        )),
    ])
}
