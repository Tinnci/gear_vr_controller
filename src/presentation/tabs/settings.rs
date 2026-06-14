use crate::domain::settings::Settings;
use crate::presentation::app::GearVRApp;
use crate::presentation::components::Components;
use eframe::egui;

pub fn render(app: &mut GearVRApp, ui: &mut egui::Ui) {
    Components::heading(ui, "Global Settings");
    ui.add_space(20.0);

    if let Ok(mut settings) = app.settings.lock() {
        let settings_mut = settings.get_mut();

        render_input_settings(settings_mut, ui);
        ui.add_space(10.0);
        render_bluetooth_settings(settings_mut, ui);
        ui.add_space(10.0);
        render_logging_settings(settings_mut, ui);
    }
}

fn render_input_settings(settings: &mut Settings, ui: &mut egui::Ui) {
    Components::brutalist_card(ui, "Input Engine", |ui| {
        ui.horizontal(|ui| {
            ui.label("Global Sensitivity:");
            ui.add(egui::Slider::new(
                &mut settings.mouse_sensitivity,
                0.1..=10.0,
            ));
        });
        ui.checkbox(&mut settings.enable_touchpad, "Enable Trackpad Input");
        ui.checkbox(&mut settings.enable_buttons, "Enable Button Mapping");
        ui.checkbox(&mut settings.enable_gestures, "Enable Gesture Commands");

        ui.separator();
        Components::sub_heading(ui, "Precision Processing");

        ui.horizontal(|ui| {
            ui.label("Dead Zone:");
            ui.add(egui::Slider::new(&mut settings.dead_zone, 0.0..=0.5));
        });

        ui.checkbox(&mut settings.enable_smoothing, "Motion Smoothing");
        if settings.enable_smoothing {
            ui.indent("smoothing_indent", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Sample Window:");
                    ui.add(egui::Slider::new(&mut settings.smoothing_factor, 1..=20));
                });
            });
        }

        ui.checkbox(&mut settings.enable_acceleration, "Pointer Acceleration");
        if settings.enable_acceleration {
            ui.indent("accel_indent", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Power Curve:");
                    ui.add(egui::Slider::new(
                        &mut settings.acceleration_power,
                        1.0..=5.0,
                    ));
                });
            });
        }
    });
}

fn render_bluetooth_settings(settings: &mut Settings, ui: &mut egui::Ui) {
    Components::brutalist_card(ui, "Bluetooth Protocol", |ui| {
        ui.checkbox(
            &mut settings.debug_show_all_devices,
            "Verbose Device Scanning (Debug mode)",
        );

        ui.collapsing("Override Service UUIDs", |ui| {
            ui.label(
                egui::RichText::new("Warning: Altering these may break device discovery.")
                    .color(egui::Color32::from_rgb(255, 200, 0)),
            );

            egui::Grid::new("ble_uuids")
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Service:");
                    ui.text_edit_singleline(&mut settings.ble_service_uuid);
                    ui.end_row();
                    ui.label("Data:");
                    ui.text_edit_singleline(&mut settings.ble_data_char_uuid);
                    ui.end_row();
                });
        });
    });
}

fn render_logging_settings(settings: &mut Settings, ui: &mut egui::Ui) {
    Components::brutalist_card(ui, "Logging & Debug", |ui| {
        ui.horizontal(|ui| {
            ui.label("Verbosity Level:");
            egui::ComboBox::from_id_salt("log_level")
                .selected_text(&settings.log_settings.level)
                .show_ui(ui, |ui| {
                    for level in &["trace", "debug", "info", "warn", "error"] {
                        ui.selectable_value(
                            &mut settings.log_settings.level,
                            level.to_string(),
                            *level,
                        );
                    }
                });
        });

        ui.checkbox(
            &mut settings.log_settings.console_logging_enabled,
            "Standard Console Logs",
        );
        ui.checkbox(
            &mut settings.log_settings.file_logging_enabled,
            "Persistent File Logs",
        );

        if settings.log_settings.file_logging_enabled {
            ui.indent("file_logs", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Save Path:");
                    ui.text_edit_singleline(&mut settings.log_settings.log_dir);
                });
                ui.horizontal(|ui| {
                    ui.label("Rotation:");
                    egui::ComboBox::from_id_salt("log_rot")
                        .selected_text(&settings.log_settings.rotation)
                        .show_ui(ui, |ui| {
                            for rot in &["daily", "hourly", "never"] {
                                ui.selectable_value(
                                    &mut settings.log_settings.rotation,
                                    rot.to_string(),
                                    *rot,
                                );
                            }
                        });
                });
            });
            ui.label(
                egui::RichText::new("Restart required for log changes.")
                    .italics()
                    .size(12.0),
            );
        }
    });
}
