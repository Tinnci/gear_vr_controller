use crate::domain::models::{
    CalibrationState, MessageSeverity, StatusMessage, TouchpadCalibration,
};
use crate::presentation::app::GearVRApp;
use crate::presentation::components::Components;
use eframe::egui;

pub fn render(app: &mut GearVRApp, ui: &mut egui::Ui) {
    Components::heading(ui, "Touchpad Calibration");
    ui.add_space(20.0);

    Components::brutalist_card(ui, "Manual Calibration Process", |ui| {
        ui.label("Move your finger slowly across the entire touchpad to map the boundaries.");
        ui.add_space(10.0);

        if !app.is_calibrating {
            if ui.button("▶ Start Mapping Process").clicked() {
                app.is_calibrating = true;
                app.calibration_data = CalibrationState {
                    min_x: u16::MAX,
                    max_x: 0,
                    min_y: u16::MAX,
                    max_y: 0,
                    ..Default::default()
                };
            }
        } else {
            ui.label(format!(
                "Data Points Collected: {}",
                app.calibration_data.samples.len()
            ));

            // Visual Progress Bar (Mock)
            let progress = (app.calibration_data.samples.len() as f32 / 100.0).min(1.0);
            ui.add(egui::ProgressBar::new(progress).text("Mapping Profile..."));

            ui.end_row();
            ui.label(format!(
                "Boundary: [{}, {}] x [{}, {}]",
                app.calibration_data.min_x,
                app.calibration_data.max_x,
                app.calibration_data.min_y,
                app.calibration_data.max_y
            ));

            ui.add_space(15.0);

            if ui.button("✅ Save & Apply Profile").clicked() {
                app.is_calibrating = false;

                let calibration = TouchpadCalibration {
                    min_x: app.calibration_data.min_x,
                    max_x: app.calibration_data.max_x,
                    min_y: app.calibration_data.min_y,
                    max_y: app.calibration_data.max_y,
                    center_x: (app.calibration_data.min_x + app.calibration_data.max_x) / 2,
                    center_y: (app.calibration_data.min_y + app.calibration_data.max_y) / 2,
                };

                if let Ok(mut settings) = app.settings.lock() {
                    let _ = settings.update_calibration(calibration);
                    app.status_message = Some(StatusMessage {
                        message: "Touchpad profile saved!".to_string(),
                        severity: MessageSeverity::Success,
                    });
                }
            }
        }
    });
}
