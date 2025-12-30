use crate::domain::models::ConnectionStatus;
use crate::presentation::app::GearVRApp;
use crate::presentation::components::Components;
use eframe::egui;

pub fn render(app: &mut GearVRApp, ui: &mut egui::Ui) {
    Components::heading(ui, "Debug & Internal State");
    ui.add_space(20.0);

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

    ui.add_space(10.0);

    if let Some(data) = &app.latest_controller_data {
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
                    ui.label("Packets:");
                    ui.label(format!("{}", data.timestamp));
                    ui.end_row();
                });
        });
    }

    ui.add_space(10.0);

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
