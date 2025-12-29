use crate::bluetooth::BluetoothService;
use crate::controller::TouchpadProcessor;
use crate::input_simulator::InputSimulator;
use crate::models::{
    AppEvent, ConnectionStatus, ControllerData, MessageSeverity, StatusMessage, TouchpadCalibration,
};
use crate::settings::SettingsService;
use eframe::egui;
use log::error;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;

pub struct GearVRApp {
    // Services
    settings: Arc<Mutex<SettingsService>>,
    input_simulator: InputSimulator,
    touchpad_processor: Option<TouchpadProcessor>,

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
        let settings = Arc::new(Mutex::new(
            SettingsService::new().expect("Failed to load settings"),
        ));

        let (data_tx, data_rx) = mpsc::unbounded_channel();
        let (bt_cmd_tx, mut bt_cmd_rx) = mpsc::unbounded_channel();

        // Spawn bluetooth task on a dedicated thread (Windows COM objects are not Send)
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for Bluetooth");

            rt.block_on(async move {
                let mut bt_service = BluetoothService::new(data_tx);

                while let Some(cmd) = bt_cmd_rx.recv().await {
                    match cmd {
                        BluetoothCommand::Connect(address) => {
                            if let Err(e) = bt_service.connect(address).await {
                                error!("Connection failed: {}", e);
                            }
                        }
                        BluetoothCommand::Disconnect => {
                            bt_service.disconnect();
                        }
                    }
                }
            });
        });

        let touchpad_processor = Some(TouchpadProcessor::new(settings.clone()));

        Self {
            settings,
            input_simulator: InputSimulator::new(),
            touchpad_processor,
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

        // Handle button presses (edge detection)
        if data.trigger_button && !self.last_trigger_state {
            let _ = self.input_simulator.mouse_left_click();
        }
        self.last_trigger_state = data.trigger_button;

        if data.touchpad_button && !self.last_touchpad_button_state {
            let _ = self.input_simulator.mouse_left_click();
        }
        self.last_touchpad_button_state = data.touchpad_button;

        // Handle back button (ESC key)
        if data.back_button {
            let _ = self.input_simulator.key_press(VK_ESCAPE);
        }

        // Handle volume buttons (Scroll)
        if data.volume_up_button {
            let _ = self.input_simulator.mouse_wheel(1);
        }
        if data.volume_down_button {
            let _ = self.input_simulator.mouse_wheel(-1);
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
                        let _ = self.bluetooth_tx.send(BluetoothCommand::Connect(address));
                    } else {
                        self.status_message = Some(StatusMessage {
                            message: "Invalid Bluetooth address".to_string(),
                            severity: MessageSeverity::Error,
                        });
                    }
                }

                if ui.button("Disconnect").clicked() {
                    let _ = self.bluetooth_tx.send(BluetoothCommand::Disconnect);
                    self.connection_status = ConnectionStatus::Disconnected;
                }
            });
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

            ui.horizontal(|ui| {
                ui.label("Mouse Sensitivity:");
                ui.add(egui::Slider::new(
                    &mut settings_mut.mouse_sensitivity,
                    0.1..=10.0,
                ));
            });

            ui.checkbox(&mut settings_mut.enable_touchpad, "Enable Touchpad");
            ui.checkbox(&mut settings_mut.enable_buttons, "Enable Buttons");

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

        if let Some(data) = &self.latest_controller_data {
            ui.group(|ui| {
                ui.label("Raw Sensor Data");
                ui.separator();

                ui.label(format!(
                    "Accelerometer: ({:.2}, {:.2}, {:.2})",
                    data.accel_x, data.accel_y, data.accel_z
                ));
                ui.label(format!(
                    "Gyroscope: ({:.2}, {:.2}, {:.2})",
                    data.gyro_x, data.gyro_y, data.gyro_z
                ));
                ui.label(format!("Timestamp: {}", data.timestamp));
            });
        }
    }
}

impl eframe::App for GearVRApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                    }
                }
                AppEvent::LogMessage(msg) => self.status_message = Some(msg),
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
