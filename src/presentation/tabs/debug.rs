use crate::domain::models::{ConnectionStatus, ControllerData};
use crate::presentation::app::GearVRApp;
use crate::presentation::components::Components;
use eframe::egui;

pub fn render(app: &mut GearVRApp, ui: &mut egui::Ui) {
    Components::heading(ui, "Debug & Internal State");
    ui.add_space(20.0);
    let latest_data = app.latest_controller_data.clone();

    render_bluetooth_status(app, ui);
    ui.add_space(10.0);

    if let Some(data) = &latest_data {
        render_raw_telemetry(data, ui);
    }

    ui.add_space(10.0);
    render_imu_diagnostics(app, latest_data.as_ref(), ui);

    ui.add_space(10.0);
    render_input_tests(app, ui);
}

fn render_bluetooth_status(app: &GearVRApp, ui: &mut egui::Ui) {
    Components::brutalist_card(ui, "Bluetooth Engine Status", |ui| {
        ui.horizontal(|ui| {
            ui.label("State:");
            let (text, color) = match app.connection_status {
                ConnectionStatus::Connected => ("STREAMING", egui::Color32::from_rgb(0, 255, 100)),
                ConnectionStatus::Disconnected => ("IDLE", egui::Color32::from_gray(150)),
                _ => ("TRANSITIONING", egui::Color32::from_rgb(255, 200, 0)),
            };
            ui.label(egui::RichText::new(text).color(color).strong());
        });

        if let Some(addr) = app.last_connected_address {
            ui.label(format!("Endpoint: {:#X}", addr));
        }
    });
}

fn render_raw_telemetry(data: &ControllerData, ui: &mut egui::Ui) {
    Components::brutalist_card(ui, "Raw Telemetry", |ui| {
        egui::Grid::new("debug_grid")
            .spacing([20.0, 5.0])
            .show(ui, |ui| {
                ui.label("Accel:");
                ui.label(format!(
                    "{:.2}, {:.2}, {:.2}",
                    data.accel_x, data.accel_y, data.accel_z
                ));
                ui.end_row();
                ui.label("Gyro:");
                ui.label(format!(
                    "{:.2}, {:.2}, {:.2}",
                    data.gyro_x, data.gyro_y, data.gyro_z
                ));
                ui.end_row();
                ui.label("Mag:");
                ui.label(format!(
                    "{:.2}, {:.2}, {:.2}",
                    data.mag_x, data.mag_y, data.mag_z
                ));
                ui.end_row();
                ui.label("Packets:");
                ui.label(format!("{}", data.timestamp));
                ui.end_row();
                ui.label("Temperature:");
                ui.label(
                    data.temperature
                        .map_or_else(|| "n/a".to_string(), |value| value.to_string()),
                );
                ui.end_row();
                ui.label("Home Button:");
                ui.label(if data.home_button {
                    "pressed"
                } else {
                    "released"
                });
                ui.end_row();
                #[cfg(debug_assertions)]
                {
                    ui.label("Raw Packet:");
                    ui.label(data.raw_bytes.as_ref().map_or_else(
                        || "n/a".to_string(),
                        |bytes| format!("{} bytes", bytes.len()),
                    ));
                    ui.end_row();
                }
            });
    });
}

fn render_imu_diagnostics(
    app: &mut GearVRApp,
    latest_data: Option<&ControllerData>,
    ui: &mut egui::Ui,
) {
    if let Some(imu) = &mut app.imu_processor {
        Components::brutalist_card(ui, "IMU Diagnostics", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Start Gyro Calibration").clicked() {
                    imu.start_calibration();
                }
                if ui.button("Reset IMU Filter").clicked() {
                    imu.reset_orientation();
                }
            });

            if imu.is_calibrating() {
                ui.add(
                    egui::ProgressBar::new(imu.calibration_progress().min(1.0))
                        .text("Calibrating gyro..."),
                );
            }

            if let Some(data) = latest_data {
                egui::Grid::new("imu_debug_grid")
                    .spacing([20.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("Tilt Scroll:");
                        ui.label(
                            imu.calculate_tilt_scroll(data)
                                .map_or_else(|| "neutral".to_string(), |value| value.to_string()),
                        );
                        ui.end_row();

                        ui.label("Shake:");
                        ui.label(if imu.detect_shake(data) { "yes" } else { "no" });
                        ui.end_row();
                    });
            }
        });
    }
}

fn render_input_tests(app: &mut GearVRApp, ui: &mut egui::Ui) {
    Components::brutalist_card(ui, "Input Injection Test", |ui| {
        ui.horizontal(|ui| {
            if ui.button("Trigger Left-Click").clicked() {
                let _ = app.input_simulator.mouse_left_click();
            }
            if ui.button("Trigger Right-Click").clicked() {
                let _ = app.input_simulator.mouse_right_click();
            }
        });
    });
}
