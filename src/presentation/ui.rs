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

    // Admin Client for elevated tasks
    admin_client: crate::admin_client::AdminClient,

    // UI Options
    is_dark_mode: bool,

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
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply Neubrutalism Style (default Light)
        configure_neubrutalism(&cc.egui_ctx, false);

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
                                let _ = tx_clone.send(AppEvent::LogMessage(StatusMessage {
                                    message: format!("Connection failed: {}", e),
                                    severity: MessageSeverity::Error,
                                }));
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

        let last_connected_address = settings.lock().unwrap().get().last_connected_address;

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
            last_connected_address,
            reconnect_timer: None,

            trigger_debounce: None,
            touchpad_btn_debounce: None,
            back_btn_debounce: None,
            volume_up_debounce: None,
            volume_down_debounce: None,

            admin_client: crate::admin_client::AdminClient::new(),
            is_dark_mode: false,
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
        ui.vertical_centered(|ui| {
            ui.heading("Gear VR Controller");
        });
        ui.add_space(20.0);

        Self::ui_connection_panel(self, ui);
        ui.add_space(15.0);

        Self::ui_status_panel(self, ui);
        ui.add_space(15.0);

        Self::ui_controller_data_panel(self, ui);
    }

    fn brutalist_card<R>(
        ui: &mut egui::Ui,
        title: &str,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
        let bg = ui.style().visuals.widgets.noninteractive.bg_fill;

        egui::Frame::none()
            .inner_margin(egui::Margin::same(15.0))
            .stroke(stroke)
            .fill(bg)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(title).strong().size(18.0));
                    ui.add_space(8.0);
                    add_contents(ui)
                })
                .inner
            })
            .inner
    }

    fn ui_connection_panel(&mut self, ui: &mut egui::Ui) {
        Self::brutalist_card(ui, "Connection Control", |ui| {
            // Status Banner (Adaptive)
            let (status_text, bg_color, text_color) = match self.connection_status {
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

            ui.add_sized(
                [ui.available_width(), 35.0],
                egui::Label::new(
                    egui::RichText::new(status_text)
                        .color(text_color)
                        .background_color(bg_color)
                        .size(16.0)
                        .strong(),
                )
                .wrap_mode(egui::TextWrapMode::Extend),
            );

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                ui.label("Address:");
                ui.text_edit_singleline(&mut self.bluetooth_address_input);
            });

            ui.horizontal(|ui| {
                if self.connection_status == ConnectionStatus::Connected {
                    if ui.button("Disconnect Instance").clicked() {
                        self.auto_reconnect = false;
                        let _ = self.bluetooth_tx.send(BluetoothCommand::Disconnect);
                    }
                } else {
                    if ui.button("🚀 Establish Connection").clicked() {
                        if let Ok(address) =
                            u64::from_str_radix(&self.bluetooth_address_input.replace(":", ""), 16)
                        {
                            self.connection_status = ConnectionStatus::Connecting;
                            self.auto_reconnect = true;
                            self.last_connected_address = Some(address);
                            let _ = self.bluetooth_tx.send(BluetoothCommand::Connect(address));
                        }
                    }
                }

                if self.is_scanning {
                    if ui.button("Stop Scan").clicked() {
                        self.is_scanning = false;
                        let _ = self.bluetooth_tx.send(BluetoothCommand::StopScan);
                    }
                    ui.spinner();
                } else {
                    if ui.button("🔍 Scan for Gear VR").clicked() {
                        self.is_scanning = true;
                        self.scanned_devices.clear();
                        let _ = self.bluetooth_tx.send(BluetoothCommand::StartScan);
                    }
                }
            });

            if !self.scanned_devices.is_empty() {
                ui.separator();
                ui.label("Nearby Controllers:");
                egui::ScrollArea::vertical()
                    .id_salt("scan_results")
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for device in &self.scanned_devices {
                            ui.horizontal(|ui| {
                                ui.label(format!(
                                    "{} ({} dBm)",
                                    device.name, device.signal_strength
                                ));
                                if ui.button("Pick").clicked() {
                                    self.bluetooth_address_input = format!("{:X}", device.address);
                                }
                            });
                        }
                    });
            }
        });
    }

    fn ui_status_panel(&mut self, ui: &mut egui::Ui) {
        let current_msg = self.status_message.clone();
        if let Some(msg) = current_msg {
            Self::brutalist_card(ui, "System Status", |ui| {
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
                        if ui.button("🛠️ Fix Bluetooth Service").clicked() {
                            let _ = self.admin_client.launch_worker();
                            std::thread::sleep(std::time::Duration::from_millis(800));
                            let _ = self.admin_client.restart_bluetooth_service();
                        }
                        if ui.button("⚙ Windows Settings").clicked() {
                            let _ = std::process::Command::new("explorer")
                                .arg("ms-settings:bluetooth")
                                .spawn();
                        }
                    });
                }
            });
        }
    }

    fn ui_controller_data_panel(&mut self, ui: &mut egui::Ui) {
        if let Some(data) = &self.latest_controller_data {
            Self::brutalist_card(ui, "Live Controller Data", |ui| {
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
                                        .background_color(egui::Color32::from_rgb(0, 255, 100)),
                                );
                            }
                            if data.back_button {
                                ui.label(
                                    egui::RichText::new(" BACK ")
                                        .background_color(egui::Color32::from_rgb(255, 200, 0)),
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

    fn render_calibration_tab(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Touchpad Calibration");
        });
        ui.add_space(20.0);

        Self::brutalist_card(ui, "Manual Calibration Process", |ui| {
            ui.label("Move your finger slowly across the entire touchpad to map the boundaries.");
            ui.add_space(10.0);

            if !self.is_calibrating {
                if ui.button("▶ Start Mapping Process").clicked() {
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
                    "Data Points Collected: {}",
                    self.calibration_data.samples.len()
                ));

                // Visual Progress Bar (Mock)
                let progress = (self.calibration_data.samples.len() as f32 / 100.0).min(1.0);
                ui.add(egui::ProgressBar::new(progress).text("Mapping Profile..."));

                ui.end_row();
                ui.label(format!(
                    "Boundary: [{}, {}] x [{}, {}]",
                    self.calibration_data.min_x,
                    self.calibration_data.max_x,
                    self.calibration_data.min_y,
                    self.calibration_data.max_y
                ));

                ui.add_space(15.0);

                if ui.button("✅ Save & Apply Profile").clicked() {
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
                        let _ = settings.update_calibration(calibration);
                        self.status_message = Some(StatusMessage {
                            message: "Touchpad profile saved!".to_string(),
                            severity: MessageSeverity::Success,
                        });
                    }
                }
            }
        });
    }

    fn render_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Global Settings");
        });
        ui.add_space(20.0);

        if let Ok(mut settings) = self.settings.lock() {
            let settings_mut = settings.get_mut();

            Self::brutalist_card(ui, "Input Engine", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Global Sensitivity:");
                    ui.add(egui::Slider::new(
                        &mut settings_mut.mouse_sensitivity,
                        0.1..=10.0,
                    ));
                });
                ui.checkbox(&mut settings_mut.enable_touchpad, "Enable Trackpad Input");
                ui.checkbox(&mut settings_mut.enable_buttons, "Enable Button Mapping");

                ui.separator();
                ui.label(egui::RichText::new("Precision Processing").strong());

                ui.horizontal(|ui| {
                    ui.label("Dead Zone:");
                    ui.add(egui::Slider::new(&mut settings_mut.dead_zone, 0.0..=0.5));
                });

                ui.checkbox(&mut settings_mut.enable_smoothing, "Motion Smoothing");
                if settings_mut.enable_smoothing {
                    ui.indent("smoothing_indent", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Sample Window:");
                            ui.add(egui::Slider::new(
                                &mut settings_mut.smoothing_factor,
                                1..=20,
                            ));
                        });
                    });
                }

                ui.checkbox(
                    &mut settings_mut.enable_acceleration,
                    "Pointer Acceleration",
                );
                if settings_mut.enable_acceleration {
                    ui.indent("accel_indent", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Power Curve:");
                            ui.add(egui::Slider::new(
                                &mut settings_mut.acceleration_power,
                                1.0..=5.0,
                            ));
                        });
                    });
                }
            });

            ui.add_space(10.0);

            Self::brutalist_card(ui, "Bluetooth Protocol", |ui| {
                ui.checkbox(
                    &mut settings_mut.debug_show_all_devices,
                    "Verbose Device Scanning (Debug mode)",
                );

                ui.collapsing("Override Service UUIDs", |ui| {
                    ui.label(
                        egui::RichText::new(
                            "⚠️ Warning: Altering these may break device discovery.",
                        )
                        .color(egui::Color32::from_rgb(255, 200, 0)),
                    );

                    egui::Grid::new("ble_uuids")
                        .spacing([10.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Service:");
                            ui.text_edit_singleline(&mut settings_mut.ble_service_uuid);
                            ui.end_row();
                            ui.label("Data:");
                            ui.text_edit_singleline(&mut settings_mut.ble_data_char_uuid);
                            ui.end_row();
                        });
                });
            });

            ui.add_space(10.0);

            Self::brutalist_card(ui, "Logging & Debug", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Verbosity Level:");
                    egui::ComboBox::from_id_salt("log_level")
                        .selected_text(&settings_mut.log_settings.level)
                        .show_ui(ui, |ui| {
                            for level in &["trace", "debug", "info", "warn", "error"] {
                                ui.selectable_value(
                                    &mut settings_mut.log_settings.level,
                                    level.to_string(),
                                    *level,
                                );
                            }
                        });
                });

                ui.checkbox(
                    &mut settings_mut.log_settings.console_logging_enabled,
                    "Standard Console Logs",
                );
                ui.checkbox(
                    &mut settings_mut.log_settings.file_logging_enabled,
                    "Persistent File Logs",
                );

                if settings_mut.log_settings.file_logging_enabled {
                    ui.indent("file_logs", |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Save Path:");
                            ui.text_edit_singleline(&mut settings_mut.log_settings.log_dir);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Rotation:");
                            egui::ComboBox::from_id_salt("log_rot")
                                .selected_text(&settings_mut.log_settings.rotation)
                                .show_ui(ui, |ui| {
                                    for rot in &["daily", "hourly", "never"] {
                                        ui.selectable_value(
                                            &mut settings_mut.log_settings.rotation,
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
    }

    fn render_debug_tab(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Debug & Internal State");
        });
        ui.add_space(20.0);

        Self::brutalist_card(ui, "Bluetooth Engine Status", |ui| {
            ui.horizontal(|ui| {
                ui.label("State:");
                let (text, color) = match self.connection_status {
                    ConnectionStatus::Connected => {
                        ("STREAMING", egui::Color32::from_rgb(0, 255, 100))
                    }
                    ConnectionStatus::Disconnected => ("IDLE", egui::Color32::from_gray(150)),
                    _ => ("TRANSITIONING", egui::Color32::from_rgb(255, 200, 0)),
                };
                ui.label(egui::RichText::new(text).color(color).strong());
            });

            if let Some(addr) = self.last_connected_address {
                ui.label(format!("Endpoint: {:#X}", addr));
            }
        });

        ui.add_space(10.0);

        if let Some(data) = &self.latest_controller_data {
            Self::brutalist_card(ui, "Raw Telemetry", |ui| {
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

        Self::brutalist_card(ui, "Input Injection Test", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Trigger Left-Click").clicked() {
                    let _ = self.input_simulator.mouse_left_click();
                }
                if ui.button("Trigger Right-Click").clicked() {
                    let _ = self.input_simulator.mouse_right_click();
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if self.is_dark_mode {
                            "☀ Light"
                        } else {
                            "🌙 Dark"
                        })
                        .clicked()
                    {
                        self.is_dark_mode = !self.is_dark_mode;
                        configure_neubrutalism(ctx, self.is_dark_mode);
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Adaptive Layout Wrapper
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(800.0); // Readability limit for wide monitors
                    ui.add_space(20.0); // Top padding

                    match self.selected_tab {
                        Tab::Home => self.render_home_tab(ui),
                        Tab::Calibration => self.render_calibration_tab(ui),
                        Tab::Settings => self.render_settings_tab(ui),
                        Tab::Debug => self.render_debug_tab(ui),
                    }

                    ui.add_space(50.0); // Bottom padding
                });
            });
        });
    }
}

// Neubrutalism Style Configuration
fn configure_neubrutalism(ctx: &egui::Context, is_dark: bool) {
    let mut style = (*ctx.style()).clone();

    // Define Palette
    let (bg_color, fg_color, stroke_color, accent_yellow, accent_green, accent_cyan) = if is_dark {
        (
            egui::Color32::from_rgb(25, 25, 25),  // Dark BG
            egui::Color32::WHITE,                 // White Text
            egui::Color32::WHITE,                 // White Borders
            egui::Color32::from_rgb(255, 200, 0), // Yellow
            egui::Color32::from_rgb(0, 255, 127), // Green
            egui::Color32::from_rgb(0, 255, 255), // Cyan
        )
    } else {
        (
            egui::Color32::from_rgb(245, 245, 245), // Light BG
            egui::Color32::BLACK,                   // Black Text
            egui::Color32::BLACK,                   // Black Borders
            egui::Color32::from_rgb(255, 220, 0),
            egui::Color32::from_rgb(0, 255, 100),
            egui::Color32::from_rgb(0, 200, 255),
        )
    };

    // Typography
    style
        .text_styles
        .iter_mut()
        .for_each(|(text_style, font_id)| {
            font_id.size = match text_style {
                egui::TextStyle::Heading => 28.0,
                egui::TextStyle::Body => 15.0,
                egui::TextStyle::Button => 15.0,
                _ => font_id.size,
            };
        });

    // Spacing
    style.spacing.item_spacing = egui::vec2(12.0, 12.0);
    style.spacing.button_padding = egui::vec2(16.0, 10.0);

    // Visuals
    // Non-interactive (Labels / Frames)
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(2.0, stroke_color);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, fg_color);
    style.visuals.widgets.noninteractive.bg_fill = bg_color;

    // Inactive (Buttons normal)
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(2.0, stroke_color);
    style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
    style.visuals.widgets.inactive.bg_fill = if is_dark {
        egui::Color32::from_gray(30)
    } else {
        egui::Color32::WHITE
    };
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, fg_color);

    // Hovered
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(2.5, stroke_color);
    style.visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
    style.visuals.widgets.hovered.bg_fill = accent_yellow;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK); // High contrast black on yellow
    style.visuals.widgets.hovered.expansion = 2.0;

    // Active
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(3.0, stroke_color);
    style.visuals.widgets.active.rounding = egui::Rounding::ZERO;
    style.visuals.widgets.active.bg_fill = accent_green;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::BLACK);

    // Selection
    style.visuals.selection.stroke = egui::Stroke::new(1.0, stroke_color);
    style.visuals.selection.bg_fill = accent_cyan;

    // Window
    style.visuals.window_rounding = egui::Rounding::ZERO;
    style.visuals.window_stroke = egui::Stroke::new(2.0, stroke_color);
    style.visuals.window_shadow = egui::Shadow {
        offset: egui::vec2(8.0, 8.0),
        blur: 0.0,
        spread: 0.0,
        color: stroke_color,
    };
    style.visuals.window_fill = bg_color;

    style.visuals.panel_fill = bg_color;
    style.visuals.override_text_color = Some(fg_color);

    ctx.set_style(style);
}
