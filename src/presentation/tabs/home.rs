use crate::domain::models::{BluetoothCommand, ConnectionStatus, MessageSeverity};
use crate::presentation::app::GearVRApp;
use crate::presentation::components::Components;
use eframe::egui;

pub fn render(app: &mut GearVRApp, ui: &mut egui::Ui) {
    Components::heading(ui, "Gear VR Controller");
    ui.add_space(20.0);

    ui_connection_panel(app, ui);
    ui.add_space(15.0);

    ui_status_panel(app, ui);
    ui.add_space(15.0);

    ui_controller_data_panel(app, ui);
}

fn ui_connection_panel(app: &mut GearVRApp, ui: &mut egui::Ui) {
    Components::brutalist_card(ui, "Connection Control", |ui| {
        // Status Banner (Adaptive)
        let (status_text, bg_color, text_color) = match app.connection_status {
            ConnectionStatus::Connected => (
                "CONNECTED",
                egui::Color32::from_rgb(0, 200, 0),
                egui::Color32::BLACK,
            ),
            ConnectionStatus::Connecting => (
                "CONNECTING...",
                egui::Color32::from_rgb(255, 200, 0),
                egui::Color32::BLACK,
            ),
            ConnectionStatus::Disconnected => (
                "DISCONNECTED",
                egui::Color32::from_gray(100),
                egui::Color32::WHITE,
            ),
            ConnectionStatus::Error => (
                "ERROR",
                egui::Color32::from_rgb(255, 50, 50),
                egui::Color32::WHITE,
            ),
        };

        Components::status_banner(ui, status_text, bg_color, text_color);

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            ui.label("Address:");
            ui.text_edit_singleline(&mut app.bluetooth_address_input);
        });

        ui.horizontal(|ui| {
            if app.connection_status == ConnectionStatus::Connected {
                if ui.button("Disconnect Instance").clicked() {
                    app.auto_reconnect = false;
                    let _ = app.bluetooth_tx.send(BluetoothCommand::Disconnect);
                }
            } else {
                if ui.button("Establish Connection").clicked() {
                    if let Ok(address) =
                        u64::from_str_radix(&app.bluetooth_address_input.replace(":", ""), 16)
                    {
                        app.connection_status = ConnectionStatus::Connecting;
                        app.auto_reconnect = true;
                        app.last_connected_address = Some(address);
                        let _ = app.bluetooth_tx.send(BluetoothCommand::Connect(address));
                    }
                }
            }

            if app.is_scanning {
                if ui.button("Stop Scan").clicked() {
                    app.is_scanning = false;
                    let _ = app.bluetooth_tx.send(BluetoothCommand::StopScan);
                }
                ui.spinner();
            } else {
                if ui.button("Scan for Gear VR").clicked() {
                    app.is_scanning = true;
                    app.scanned_devices.clear();
                    let _ = app.bluetooth_tx.send(BluetoothCommand::StartScan);
                }
            }
        });

        if !app.scanned_devices.is_empty() {
            ui.separator();
            ui.label("Nearby Controllers:");
            egui::ScrollArea::vertical()
                .id_salt("scan_results")
                .max_height(120.0)
                .show(ui, |ui| {
                    for device in &app.scanned_devices {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} ({} dBm)", device.name, device.signal_strength));
                            if ui.button("Pick").clicked() {
                                app.bluetooth_address_input = format!("{:X}", device.address);
                            }
                        });
                    }
                });
        }
    });
}

fn ui_status_panel(app: &mut GearVRApp, ui: &mut egui::Ui) {
    let current_msg = app.status_message.clone();
    if let Some(msg) = current_msg {
        Components::brutalist_card(ui, "System Status", |ui| {
            let color = match msg.severity {
                MessageSeverity::Info => egui::Color32::BLUE,
                MessageSeverity::Success => egui::Color32::from_rgb(0, 150, 0),
                MessageSeverity::Warning => egui::Color32::from_rgb(200, 150, 0),
                MessageSeverity::Error => egui::Color32::RED,
            };

            ui.label(egui::RichText::new(&msg.message).color(color).strong());

            // Admin Actions
            if msg.message.contains("幽灵设备") || msg.severity == MessageSeverity::Error {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Fix Bluetooth Service").clicked() {
                        let _ = app.admin_client.launch_worker();
                        std::thread::sleep(std::time::Duration::from_millis(800));
                        let _ = app.admin_client.restart_bluetooth_service();
                    }
                    if ui.button("Windows Settings").clicked() {
                        let _ = std::process::Command::new("explorer")
                            .arg("ms-settings:bluetooth")
                            .spawn();
                    }
                });
            }
        });
    }
}

fn ui_controller_data_panel(app: &mut GearVRApp, ui: &mut egui::Ui) {
    if let Some(data) = &app.latest_controller_data {
        Components::brutalist_card(ui, "Live Controller Data", |ui| {
            egui::Grid::new("data_grid")
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Touchpad:");
                    ui.label(format!("({:.0}, {:.0})", data.touchpad_x, data.touchpad_y));
                    ui.end_row();

                    ui.label("Buttons:");
                    ui.horizontal(|ui| {
                        if data.trigger_button {
                            ui.label(
                                egui::RichText::new(" TRIGGER ")
                                    .background_color(egui::Color32::from_rgb(0, 255, 100))
                                    .color(egui::Color32::BLACK),
                            );
                        }
                        if data.back_button {
                            ui.label(
                                egui::RichText::new(" BACK ")
                                    .background_color(egui::Color32::from_rgb(255, 200, 0))
                                    .color(egui::Color32::BLACK),
                            );
                        }
                    });
                    ui.end_row();

                    ui.label("Battery:");
                    ui.label(format!("{}%", 100)); // Placeholder for now
                    ui.end_row();
                });
        });
    }
}
