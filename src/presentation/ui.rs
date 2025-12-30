use crate::domain::controller::TouchpadProcessor;
use crate::domain::gestures::{GestureDirection, GestureRecognizer};
use crate::domain::models::{
    AppEvent, ConnectionStatus, ControllerData, MessageSeverity, ScannedDevice, StatusMessage,
    TouchpadCalibration,
};
use crate::domain::settings::SettingsService;
use crate::infrastructure::bluetooth::BluetoothService;
use crate::infrastructure::input_simulator::InputSimulator;
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::error;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;

pub struct GearVRApp {
    // Services
    settings: Arc<Mutex<SettingsService>>,
    input_simulator: InputSimulator,
    touchpad_processor: Option<TouchpadProcessor>,
    gesture_recognizer: Option<GestureRecognizer>,

    // Bluetooth
    bluetooth_tx: mpsc::UnboundedSender<BluetoothCommand>,
    controller_data_rx: mpsc::UnboundedReceiver<AppEvent>,

    // State
    connection_status: ConnectionStatus,
    status_message: Option<StatusMessage>,
    latest_controller_data: Option<ControllerData>,

    // UI State
    selected_tab: Tab,
    bluetooth_address_input: String,

    // Calibration
    is_calibrating: bool,
    calibration_data: CalibrationData,

    // Button states (for edge detection)
    last_trigger_state: bool,
    last_touchpad_button_state: bool,
    last_back_button_state: bool,

    // Scanning
    is_scanning: bool,
    scanned_devices: Vec<ScannedDevice>,

    // Reconnection
    auto_reconnect: bool,
    last_connected_address: Option<u64>,
    reconnect_timer: Option<Instant>,

    // Debounce
    trigger_debounce: Option<Instant>,
    touchpad_btn_debounce: Option<Instant>,
    back_btn_debounce: Option<Instant>,
    volume_up_debounce: Option<Instant>,
    volume_down_debounce: Option<Instant>,

    // Logging guard
    _logging_guard: Option<crate::infrastructure::logging::LoggingGuard>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Home,
    Calibration,
    Settings,
    Debug,
}

enum BluetoothCommand {
    Connect(u64),
    Disconnect,
    StartScan,
    StopScan,
}

#[derive(Default)]
struct CalibrationData {
    min_x: u16,
    max_x: u16,
    min_y: u16,
    max_y: u16,
    samples: Vec<(u16, u16)>,
}

impl GearVRApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings_service = SettingsService::new().expect("Failed to load settings");

        // Initialize logging
        let logging_guard =
            crate::infrastructure::logging::init_logger(&settings_service.get().log_settings)
                .map_err(|e| eprintln!("Failed to initialize logging: {}", e))
                .ok();

        tracing::info!("Starting Gear VR Controller Application");

        let settings = Arc::new(Mutex::new(settings_service));

        let (data_tx, data_rx) = mpsc::unbounded_channel();
        let (bt_cmd_tx, mut bt_cmd_rx) = mpsc::unbounded_channel();

        // Create a clone for the Bluetooth thread
        let bt_settings = settings.clone();

        // Spawn bluetooth task on a dedicated thread (Windows COM objects are not Send)
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for Bluetooth");

            rt.block_on(async move {
                let tx_clone = data_tx.clone();
                let mut bt_service = BluetoothService::new(data_tx, bt_settings);

                while let Some(cmd) = bt_cmd_rx.recv().await {
                    match cmd {
                        BluetoothCommand::Connect(address) => {
                            if let Err(e) = bt_service.connect(address).await {
                                error!("Connection failed: {}", e);
                                let _ = tx_clone.send(AppEvent::ConnectionStatus(
                                    ConnectionStatus::Disconnected,
                                ));
                            }
                        }
                        BluetoothCommand::Disconnect => {
                            bt_service.disconnect();
                        }
                        BluetoothCommand::StartScan => {
                            if let Err(e) = bt_service.start_scan() {
                                error!("Failed to start scan: {}", e);
                            }
                        }
                        BluetoothCommand::StopScan => {
                            if let Err(e) = bt_service.stop_scan() {
                                error!("Failed to stop scan: {}", e);
                            }
                        }
                    }
                }
            });
        });

        let touchpad_processor = Some(TouchpadProcessor::new(settings.clone()));
        let gesture_recognizer = Some(GestureRecognizer::new(settings.clone()));

        Self {
            settings,
            input_simulator: InputSimulator::new(),
            touchpad_processor,
            gesture_recognizer,
            bluetooth_tx: bt_cmd_tx,
            controller_data_rx: data_rx,
            connection_status: ConnectionStatus::Disconnected,
            status_message: None,
            latest_controller_data: None,
            selected_tab: Tab::Home,
            bluetooth_address_input: String::new(),
            is_calibrating: false,
            calibration_data: CalibrationData::default(),
            last_trigger_state: false,

            last_touchpad_button_state: false,
            last_back_button_state: false,
            is_scanning: false,
            scanned_devices: Vec::new(),
            auto_reconnect: false,
            last_connected_address: None,
            reconnect_timer: None,
            trigger_debounce: None,
            touchpad_btn_debounce: None,
            back_btn_debounce: None,
            volume_up_debounce: None,
            volume_down_debounce: None,
            _logging_guard: logging_guard,
        }
    }

    fn process_controller_data(&mut self, mut data: ControllerData) {
        // Process touchpad coordinates
        if let Some(processor) = &mut self.touchpad_processor {
            processor.process(&mut data);

            // Handle mouse movement
            if data.touchpad_touched {
                if let Some((dx, dy)) = processor.calculate_mouse_delta(&data) {
                    let _ = self.input_simulator.move_mouse(dx, dy);
                }
            }
        }

        // Process gestures
        if let Some(recognizer) = &mut self.gesture_recognizer {
            if let Some(direction) = recognizer.process(&data) {
                let msg = format!("Gesture Detected: {:?}", direction);
                tracing::info!("{}", msg);
                self.status_message = Some(StatusMessage {
                    message: msg.clone(),
                    severity: MessageSeverity::Info,
                });

                // TODO: Map actions
                match direction {
                    GestureDirection::Up => {
                        let _ = self.input_simulator.mouse_wheel(1);
                    }
                    GestureDirection::Down => {
                        let _ = self.input_simulator.mouse_wheel(-1);
                    }
                    GestureDirection::Left => {
                        let _ = self
                            .input_simulator
                            .key_press(windows::Win32::UI::Input::KeyboardAndMouse::VK_LMENU);
                    } // Alt (Placeholder)
                    GestureDirection::Right => {}
                    _ => {}
                }
            }
        }

        // Handle button presses with debouncing
        let now = Instant::now();
        let debounce_duration = Duration::from_millis(50);

        // Helper macro or closure for debounce logic
        // But since we need to mutate specific fields, maybe verbose is safer

        // TRIGGER (Left Mouse)
        if data.trigger_button != self.last_trigger_state {
            let can_switch = match self.trigger_debounce {
                Some(last) => now.duration_since(last) > debounce_duration,
                None => true,
            };

            if can_switch {
                self.last_trigger_state = data.trigger_button;
                self.trigger_debounce = Some(now);

                if data.trigger_button {
                    let _ = self.input_simulator.mouse_left_down();
                } else {
                    let _ = self.input_simulator.mouse_left_up();
                }
            }
        }

        // TOUCHPAD BUTTON (Right Mouse)
        if data.touchpad_button != self.last_touchpad_button_state {
            let can_switch = match self.touchpad_btn_debounce {
                Some(last) => now.duration_since(last) > debounce_duration,
                None => true,
            };

            if can_switch {
                self.last_touchpad_button_state = data.touchpad_button;
                self.touchpad_btn_debounce = Some(now);

                if data.touchpad_button {
                    let _ = self.input_simulator.mouse_right_down();
                } else {
                    let _ = self.input_simulator.mouse_right_up();
                }
            }
        }

        // BACK BUTTON (Esc)
        // For keys, we usually just want single press (Down+Up) or simulate hold?
        // Let's do hold simulation for consistency
        // Wait, input_simulator has key_down/key_up
        // But back button is often used as a click.
        // Let's implement simple press (down...up) or debounced single shot?
        // C# impl used SimulateKeyPress (Down+Up) on edge.
        // Let's stick to C# behavior for now but with debounce.
        // Actually, if we hold Back, we might want repeat? No.

        // Use a simple state tracker for back button if not already existing?
        // We don't have last_back_btn_state. Let's look at struct.
        // Struct has last_trigger_state, last_touchpad_button_state.
        // We need to add last_back_button_state etc. to struct if we want properly debounce generic buttons.
        // For now, let's just use the simpler edge detection if we don't track state, BUT we can't debounce without state tracking.
        // The data.back_button IS the current state.
        // So we need to add fields to struct.

        // VOLUME (Scroll)
        // Usually scroll is discrete events. Holding volume should scroll continuously?
        // C# impl: if held, repeat.
        // Rust code: "if data.volume_up { wheel(1) }" -> This executes EVERY frame if held.
        // This makes scroll VERY fast (60Hz scroll).
        // C# used a debounce timer (50ms) to limit the rate.
        // effectively 20 scrolls per second.

        // Let's fix Volume repetition rate first.
        if data.volume_up_button {
            let can_fire = match self.volume_up_debounce {
                Some(last) => now.duration_since(last) > debounce_duration,
                None => true,
            };
            if can_fire {
                let _ = self.input_simulator.mouse_wheel(1);
                self.volume_up_debounce = Some(now);
            }
        }

        if data.volume_down_button {
            let can_fire = match self.volume_down_debounce {
                Some(last) => now.duration_since(last) > debounce_duration,
                None => true,
            };
            if can_fire {
                let _ = self.input_simulator.mouse_wheel(-1);
                self.volume_down_debounce = Some(now);
            }
        }

        // BACK BUTTON (Esc)
        if data.back_button != self.last_back_button_state {
            let can_switch = match self.back_btn_debounce {
                Some(last) => now.duration_since(last) > debounce_duration,
                None => true,
            };

            if can_switch {
                self.last_back_button_state = data.back_button;
                self.back_btn_debounce = Some(now);

                if data.back_button {
                    // Rising edge
                    let _ = self.input_simulator.key_press(VK_ESCAPE);
                }
            }
        }

        // Update calibration if active
        if self.is_calibrating && data.touchpad_touched {
            self.calibration_data
                .samples
                .push((data.touchpad_x, data.touchpad_y));
            self.calibration_data.min_x = self.calibration_data.min_x.min(data.touchpad_x);
            self.calibration_data.max_x = self.calibration_data.max_x.max(data.touchpad_x);
            self.calibration_data.min_y = self.calibration_data.min_y.min(data.touchpad_y);
            self.calibration_data.max_y = self.calibration_data.max_y.max(data.touchpad_y);
        }

        self.latest_controller_data = Some(data);
    }

    fn render_home_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Gear VR Controller");
        ui.separator();

        // Connection section
        ui.group(|ui| {
            ui.label("Connection");

            ui.horizontal(|ui| {
                ui.label("Status:");
                let (text, color) = match self.connection_status {
                    ConnectionStatus::Connected => ("Connected", egui::Color32::GREEN),
                    ConnectionStatus::Connecting => ("Connecting...", egui::Color32::YELLOW),
                    ConnectionStatus::Disconnected => ("Disconnected", egui::Color32::GRAY),
                    ConnectionStatus::Error => ("Error", egui::Color32::RED),
                };
                ui.colored_label(color, text);
            });

            ui.horizontal(|ui| {
                ui.label("Bluetooth Address:");
                ui.text_edit_singleline(&mut self.bluetooth_address_input);
            });

            ui.horizontal(|ui| {
                if ui.button("Connect").clicked() {
                    if let Ok(address) =
                        u64::from_str_radix(&self.bluetooth_address_input.replace(":", ""), 16)
                    {
                        self.connection_status = ConnectionStatus::Connecting;
                        self.auto_reconnect = true;
                        self.last_connected_address = Some(address);
                        self.reconnect_timer = None;
                        let _ = self.bluetooth_tx.send(BluetoothCommand::Connect(address));
                    } else {
                        self.status_message = Some(StatusMessage {
                            message: "Invalid Bluetooth address".to_string(),
                            severity: MessageSeverity::Error,
                        });
                    }
                }

                if ui.button("Disconnect").clicked() {
                    self.auto_reconnect = false;
                    self.last_connected_address = None;
                    self.reconnect_timer = None;
                    let _ = self.bluetooth_tx.send(BluetoothCommand::Disconnect);
                    self.connection_status = ConnectionStatus::Disconnected;
                }
            });

            ui.horizontal(|ui| {
                if self.is_scanning {
                    if ui.button("Stop Scan").clicked() {
                        self.is_scanning = false;
                        let _ = self.bluetooth_tx.send(BluetoothCommand::StopScan);
                    }
                    ui.spinner();
                } else {
                    if ui.button("Scan for Devices").clicked() {
                        self.is_scanning = true;
                        self.scanned_devices.clear();
                        let _ = self.bluetooth_tx.send(BluetoothCommand::StartScan);
                    }
                }
            });

            if !self.scanned_devices.is_empty() {
                ui.separator();
                ui.label("Discovered Devices:");
                egui::ScrollArea::vertical()
                    .id_salt("scan_results")
                    .show(ui, |ui| {
                        for device in &self.scanned_devices {
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "{} ({} dBm)",
                                    device.name, device.signal_strength
                                ));
                                if ui.button("Connect").clicked() {
                                    self.bluetooth_address_input = format!("{:X}", device.address);
                                    self.connection_status = ConnectionStatus::Connecting;
                                    self.is_scanning = false;
                                    self.auto_reconnect = true;
                                    self.last_connected_address = Some(device.address);
                                    self.reconnect_timer = None;
                                    let _ = self.bluetooth_tx.send(BluetoothCommand::StopScan);
                                    let _ = self
                                        .bluetooth_tx
                                        .send(BluetoothCommand::Connect(device.address));
                                }
                            });
                        }
                    });
            }
        });

        ui.add_space(10.0);

        // Status message
        if let Some(msg) = &self.status_message {
            let color = match msg.severity {
                MessageSeverity::Info => egui::Color32::LIGHT_BLUE,
                MessageSeverity::Success => egui::Color32::GREEN,
                MessageSeverity::Warning => egui::Color32::YELLOW,
                MessageSeverity::Error => egui::Color32::RED,
            };
            ui.colored_label(color, &msg.message);
        }

        ui.add_space(10.0);

        // Controller data display
        if let Some(data) = &self.latest_controller_data {
            ui.group(|ui| {
                ui.label("Controller Data");
                ui.separator();

                ui.label(format!(
                    "Touchpad: ({}, {})",
                    data.touchpad_x, data.touchpad_y
                ));
                ui.label(format!(
                    "Processed: ({:.2}, {:.2})",
                    data.processed_touchpad_x, data.processed_touchpad_y
                ));
                ui.label(format!("Touched: {}", data.touchpad_touched));
                ui.label(format!("Trigger: {}", data.trigger_button));
                ui.label(format!("Back: {}", data.back_button));
                ui.label(format!("Home: {}", data.home_button));
            });
        }
    }

    fn render_calibration_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Touchpad Calibration");
        ui.separator();

        ui.label("Move your finger around the entire touchpad surface to calibrate.");
        ui.add_space(10.0);

        if !self.is_calibrating {
            if ui.button("Start Calibration").clicked() {
                self.is_calibrating = true;
                self.calibration_data = CalibrationData {
                    min_x: u16::MAX,
                    max_x: 0,
                    min_y: u16::MAX,
                    max_y: 0,
                    ..Default::default()
                };
            }
        } else {
            ui.label(format!(
                "Samples collected: {}",
                self.calibration_data.samples.len()
            ));
            ui.label(format!(
                "X Range: {} - {}",
                self.calibration_data.min_x, self.calibration_data.max_x
            ));
            ui.label(format!(
                "Y Range: {} - {}",
                self.calibration_data.min_y, self.calibration_data.max_y
            ));

            ui.add_space(10.0);

            if ui.button("Finish Calibration").clicked() {
                self.is_calibrating = false;

                let calibration = TouchpadCalibration {
                    min_x: self.calibration_data.min_x,
                    max_x: self.calibration_data.max_x,
                    min_y: self.calibration_data.min_y,
                    max_y: self.calibration_data.max_y,
                    center_x: (self.calibration_data.min_x + self.calibration_data.max_x) / 2,
                    center_y: (self.calibration_data.min_y + self.calibration_data.max_y) / 2,
                };

                if let Ok(mut settings) = self.settings.lock() {
                    tracing::info!("Calibration saved: {:?}", calibration);
                    let _ = settings.update_calibration(calibration);
                    self.status_message = Some(StatusMessage {
                        message: "Calibration saved!".to_string(),
                        severity: MessageSeverity::Success,
                    });
                }
            }
        }
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();

        if let Ok(mut settings) = self.settings.lock() {
            let settings_mut = settings.get_mut();

            ui.heading("Input Configuration");
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Mouse Sensitivity:");
                    ui.add(egui::Slider::new(
                        &mut settings_mut.mouse_sensitivity,
                        0.1..=10.0,
                    ));
                });
                ui.checkbox(&mut settings_mut.enable_touchpad, "Enable Touchpad");
                ui.checkbox(&mut settings_mut.enable_buttons, "Enable Buttons");

                ui.separator();
                ui.label(egui::RichText::new("Advanced Input Processing").strong());

                // Dead Zone
                ui.horizontal(|ui| {
                    ui.label("Dead Zone:");
                    ui.add(
                        egui::Slider::new(&mut settings_mut.dead_zone, 0.0..=0.5)
                            .text("Normalized Radius"),
                    )
                    .on_hover_text("Ignore small touches near the center to prevent drift.");
                });

                // Smoothing
                ui.checkbox(
                    &mut settings_mut.enable_smoothing,
                    "Enable Motion Smoothing",
                )
                .on_hover_text("Average multiple samples to reduce jitter.");
                if settings_mut.enable_smoothing {
                    ui.indent("smoothing_indent", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Smoothing Samples:");
                            ui.add(egui::Slider::new(
                                &mut settings_mut.smoothing_factor,
                                1..=20,
                            ));
                        });
                    });
                }

                // Acceleration
                ui.checkbox(
                    &mut settings_mut.enable_acceleration,
                    "Enable Mouse Acceleration",
                )
                .on_hover_text("Move cursor faster with faster swipes.");
                if settings_mut.enable_acceleration {
                    ui.indent("accel_indent", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Acceleration Power:");
                            ui.add(egui::Slider::new(
                                &mut settings_mut.acceleration_power,
                                1.0..=5.0,
                            ));
                        });
                    });
                }
            });

            ui.add_space(10.0);

            ui.collapsing("Advanced Bluetooth Configuration", |ui| {
                ui.label(
                    egui::RichText::new("Warning: Changing these values may prevent connection.")
                        .color(egui::Color32::YELLOW),
                );

                ui.horizontal(|ui| {
                    ui.label("Service UUID:");
                    ui.text_edit_singleline(&mut settings_mut.ble_service_uuid);
                });
                ui.horizontal(|ui| {
                    ui.label("Data Characteristic:");
                    ui.text_edit_singleline(&mut settings_mut.ble_data_char_uuid);
                });
                ui.horizontal(|ui| {
                    ui.label("Command Characteristic:");
                    ui.text_edit_singleline(&mut settings_mut.ble_command_char_uuid);
                });
            });

            ui.add_space(10.0);
            ui.separator();
            ui.heading("Logging");

            ui.horizontal(|ui| {
                ui.label("Log Level:");
                egui::ComboBox::from_id_salt("log_level")
                    .selected_text(&settings_mut.log_settings.level)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut settings_mut.log_settings.level,
                            "trace".to_string(),
                            "Trace",
                        );
                        ui.selectable_value(
                            &mut settings_mut.log_settings.level,
                            "debug".to_string(),
                            "Debug",
                        );
                        ui.selectable_value(
                            &mut settings_mut.log_settings.level,
                            "info".to_string(),
                            "Info",
                        );
                        ui.selectable_value(
                            &mut settings_mut.log_settings.level,
                            "warn".to_string(),
                            "Warn",
                        );
                        ui.selectable_value(
                            &mut settings_mut.log_settings.level,
                            "error".to_string(),
                            "Error",
                        );
                    });
            });
            ui.checkbox(
                &mut settings_mut.log_settings.console_logging_enabled,
                "Enable Console Logging",
            );
            ui.checkbox(
                &mut settings_mut.log_settings.file_logging_enabled,
                "Enable File Logging",
            );

            if settings_mut.log_settings.file_logging_enabled {
                ui.horizontal(|ui| {
                    ui.label("Log Directory:");
                    ui.text_edit_singleline(&mut settings_mut.log_settings.log_dir);
                });
                ui.horizontal(|ui| {
                    ui.label("File Name Prefix:");
                    ui.text_edit_singleline(&mut settings_mut.log_settings.file_name_prefix);
                });
                ui.horizontal(|ui| {
                    ui.label("Rotation Strategy:");
                    egui::ComboBox::from_id_salt("log_rotation")
                        .selected_text(&settings_mut.log_settings.rotation)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings_mut.log_settings.rotation,
                                "daily".to_string(),
                                "Daily",
                            );
                            ui.selectable_value(
                                &mut settings_mut.log_settings.rotation,
                                "hourly".to_string(),
                                "Hourly",
                            );
                            ui.selectable_value(
                                &mut settings_mut.log_settings.rotation,
                                "minutely".to_string(),
                                "Minutely",
                            );
                            ui.selectable_value(
                                &mut settings_mut.log_settings.rotation,
                                "never".to_string(),
                                "Never",
                            );
                        });
                });
                ui.label(
                    egui::RichText::new(
                        "To apply logging changes, please restart the application.",
                    )
                    .color(egui::Color32::YELLOW),
                );
            }

            ui.collapsing("Advanced Logging Formatting", |ui| {
                ui.checkbox(
                    &mut settings_mut.log_settings.show_file_line,
                    "Show File & Line",
                );
                ui.checkbox(
                    &mut settings_mut.log_settings.show_thread_ids,
                    "Show Thread IDs",
                );
                ui.checkbox(
                    &mut settings_mut.log_settings.show_target,
                    "Show Target (Module)",
                );
                ui.checkbox(
                    &mut settings_mut.log_settings.ansi_colors,
                    "ANSI Colors (Console)",
                );
            });

            ui.separator();
            ui.heading("Input Polish");

            ui.horizontal(|ui| {
                ui.label("Dead Zone:");
                // Range 0.0 to 1.0 normalized? Or 0.0 to 20.0 "percent"?
                // In controller.rs we use settings.dead_zone / 100.0.
                // So if user selects 1.0 here, it means 0.01 normalized threshold.
                ui.add(egui::Slider::new(&mut settings_mut.dead_zone, 0.0..=10.0).text("%"));
            });

            ui.checkbox(&mut settings_mut.enable_smoothing, "Enable Smoothing");
            if settings_mut.enable_smoothing {
                ui.horizontal(|ui| {
                    ui.label("Smoothing Factor:");
                    ui.add(egui::Slider::new(
                        &mut settings_mut.smoothing_factor,
                        2..=10,
                    ));
                });
            }

            ui.checkbox(&mut settings_mut.enable_acceleration, "Enable Acceleration");
            if settings_mut.enable_acceleration {
                ui.horizontal(|ui| {
                    ui.label("Acceleration Power:");
                    ui.add(egui::Slider::new(
                        &mut settings_mut.acceleration_power,
                        1.0..=3.0,
                    ));
                });
            }
            ui.separator();

            if ui.button("Save Settings").clicked() {
                if let Err(e) = settings.save() {
                    error!("Failed to save settings: {}", e);
                } else {
                    self.status_message = Some(StatusMessage {
                        message: "Settings saved!".to_string(),
                        severity: MessageSeverity::Success,
                    });
                }
            }
        }
    }

    fn render_debug_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Debug Information");
        ui.separator();

        // Connection Info
        ui.group(|ui| {
            ui.label(egui::RichText::new("Connection Status").strong());
            ui.horizontal(|ui| {
                ui.label("Status:");
                let status_text = format!("{:?}", self.connection_status);
                let color = match self.connection_status {
                    ConnectionStatus::Connected => egui::Color32::GREEN,
                    ConnectionStatus::Disconnected => egui::Color32::RED,
                    _ => egui::Color32::YELLOW,
                };
                ui.colored_label(color, status_text);
            });

            if let Some(addr) = self.last_connected_address {
                ui.horizontal(|ui| {
                    ui.label("Last Address:");
                    ui.monospace(format!("{:#X}", addr));
                });
            }
        });

        ui.add_space(5.0);

        // Scanner Info
        ui.group(|ui| {
            ui.label(egui::RichText::new("Scanner State").strong());
            if self.is_scanning {
                ui.colored_label(egui::Color32::GREEN, "Scanning Active");
            } else {
                ui.label("Scanner Idle");
            }

            if !self.scanned_devices.is_empty() {
                ui.separator();
                ui.label("Found Devices:");
                egui::ScrollArea::vertical()
                    .max_height(100.0)
                    .show(ui, |ui| {
                        for device in &self.scanned_devices {
                            ui.horizontal(|ui| {
                                ui.label(&device.name);
                                ui.monospace(format!("[{:#X}]", device.address));
                                ui.label(format!("RSSI: {} dBm", device.signal_strength));
                            });
                        }
                    });
            } else if self.is_scanning {
                ui.label("No devices found yet...");
            }
        });

        ui.add_space(5.0);

        if let Some(data) = &self.latest_controller_data {
            ui.group(|ui| {
                ui.label(egui::RichText::new("Raw Sensor Data").strong());
                ui.separator();

                ui.label(format!(
                    "Accelerometer: ({:.2}, {:.2}, {:.2})",
                    data.accel_x, data.accel_y, data.accel_z
                ));
                ui.label(format!(
                    "Gyroscope: ({:.2}, {:.2}, {:.2})",
                    data.gyro_x, data.gyro_y, data.gyro_z
                ));
                ui.label(format!(
                    "Touchpad: ({}, {})",
                    data.touchpad_x, data.touchpad_y
                ));
                ui.label(format!(
                    "Buttons: T:{}, H:{}, B:{}",
                    data.trigger_button, data.home_button, data.back_button
                ));
                ui.label(format!("Timestamp: {}", data.timestamp));
            });
        }

        ui.add_space(5.0);

        ui.group(|ui| {
            ui.label(egui::RichText::new("Input Simulator Test").strong());
            ui.horizontal(|ui| {
                if ui.button("Left Click").clicked() {
                    let _ = self.input_simulator.mouse_left_click();
                }
                if ui.button("Right Click").clicked() {
                    let _ = self.input_simulator.mouse_right_click();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Move to (500, 500)").clicked() {
                    let _ = self.input_simulator.set_cursor_pos(500, 500);
                }
                if let Ok((x, y)) = self.input_simulator.get_cursor_pos() {
                    ui.label(format!("Current Pos: ({}, {})", x, y));
                }
            });
        });
    }
}

impl eframe::App for GearVRApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle auto-reconnect timer
        if let Some(time) = self.reconnect_timer {
            if Instant::now() >= time {
                self.reconnect_timer = None;
                if let Some(address) = self.last_connected_address {
                    self.connection_status = ConnectionStatus::Connecting;
                    let _ = self.bluetooth_tx.send(BluetoothCommand::Connect(address));
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }

        // Process incoming controller data
        while let Ok(event) = self.controller_data_rx.try_recv() {
            match event {
                AppEvent::ControllerData(data) => self.process_controller_data(data),
                AppEvent::ConnectionStatus(status) => {
                    self.connection_status = status;
                    if let ConnectionStatus::Connected = status {
                        self.status_message = Some(StatusMessage {
                            message: "Connected to Gear VR Controller".to_string(),
                            severity: MessageSeverity::Success,
                        });
                        self.reconnect_timer = None;

                        // Save known address
                        if let Some(addr) = self.last_connected_address {
                            if let Ok(mut settings) = self.settings.lock() {
                                let _ = settings.add_known_address(addr);
                            }
                        }
                    } else if let ConnectionStatus::Disconnected = status {
                        if self.auto_reconnect {
                            self.reconnect_timer =
                                Some(Instant::now() + Duration::from_millis(2000));
                            self.status_message = Some(StatusMessage {
                                message: "Disconnected. Reconnecting in 2s...".to_string(),
                                severity: MessageSeverity::Warning,
                            });
                        }
                    }
                }
                AppEvent::LogMessage(msg) => self.status_message = Some(msg),
                AppEvent::DeviceFound(device) => {
                    // Update existing or add new
                    if let Some(existing) = self
                        .scanned_devices
                        .iter_mut()
                        .find(|d| d.address == device.address)
                    {
                        existing.signal_strength = device.signal_strength;
                    } else {
                        self.scanned_devices.push(device);
                    }
                }
            }
        }

        // Request continuous repaint
        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::Home, "Home");
                ui.selectable_value(&mut self.selected_tab, Tab::Calibration, "Calibration");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "Settings");
                ui.selectable_value(&mut self.selected_tab, Tab::Debug, "Debug");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.selected_tab {
            Tab::Home => self.render_home_tab(ui),
            Tab::Calibration => self.render_calibration_tab(ui),
            Tab::Settings => self.render_settings_tab(ui),
            Tab::Debug => self.render_debug_tab(ui),
        });
    }
}
