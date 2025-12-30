use crate::domain::controller::TouchpadProcessor;
use crate::domain::gestures::{GestureDirection, GestureRecognizer};
use crate::domain::models::{
    AppEvent, BluetoothCommand, CalibrationState, ConnectionStatus, ControllerData,
    MessageSeverity, ScannedDevice, StatusMessage, Tab,
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
    pub(crate) settings: Arc<Mutex<SettingsService>>,
    pub(crate) input_simulator: InputSimulator,
    pub(crate) touchpad_processor: Option<TouchpadProcessor>,
    pub(crate) gesture_recognizer: Option<GestureRecognizer>,

    // Bluetooth
    pub(crate) bluetooth_tx: mpsc::UnboundedSender<BluetoothCommand>,
    pub(crate) controller_data_rx: mpsc::UnboundedReceiver<AppEvent>,

    // State
    pub(crate) connection_status: ConnectionStatus,
    pub(crate) status_message: Option<StatusMessage>,
    pub(crate) latest_controller_data: Option<ControllerData>,

    // UI State
    pub(crate) selected_tab: Tab,
    pub(crate) bluetooth_address_input: String,

    // Calibration
    pub(crate) is_calibrating: bool,
    pub(crate) calibration_data: CalibrationState,

    // Button states (for edge detection)
    pub(crate) last_trigger_state: bool,
    pub(crate) last_touchpad_button_state: bool,
    pub(crate) last_back_button_state: bool,

    // Scanning
    pub(crate) is_scanning: bool,
    pub(crate) scanned_devices: Vec<ScannedDevice>,

    // Reconnection
    pub(crate) auto_reconnect: bool,
    pub(crate) last_connected_address: Option<u64>,
    pub(crate) reconnect_timer: Option<Instant>,

    // Debounce
    pub(crate) trigger_debounce: Option<Instant>,
    pub(crate) touchpad_btn_debounce: Option<Instant>,
    pub(crate) back_btn_debounce: Option<Instant>,
    pub(crate) volume_up_debounce: Option<Instant>,
    pub(crate) volume_down_debounce: Option<Instant>,

    // Admin Client for elevated tasks
    pub(crate) admin_client: crate::admin_client::AdminClient,

    // UI Options
    pub(crate) is_dark_mode: bool,

    // Logging guard
    pub(crate) _logging_guard: Option<crate::infrastructure::logging::LoggingGuard>,
}

impl GearVRApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply Neubrutalism Style (default Light)
        crate::presentation::theme::configure_neubrutalism(&cc.egui_ctx, false);

        let settings_service = SettingsService::new().expect("Failed to load settings");

        let logging_guard =
            crate::infrastructure::logging::init_logger(&settings_service.get().log_settings)
                .map_err(|e| eprintln!("Failed to initialize logging: {}", e))
                .ok();

        tracing::info!("Starting Gear VR Controller Application");

        let settings = Arc::new(Mutex::new(settings_service));
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        let (bt_cmd_tx, mut bt_cmd_rx) = mpsc::unbounded_channel();
        let bt_settings = settings.clone();

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
            calibration_data: CalibrationState::default(),
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
        if let Some(processor) = &mut self.touchpad_processor {
            processor.process(&mut data);
            if data.touchpad_touched {
                if let Some((dx, dy)) = processor.calculate_mouse_delta(&data) {
                    let _ = self.input_simulator.move_mouse(dx, dy);
                }
            }
        }

        if let Some(recognizer) = &mut self.gesture_recognizer {
            if let Some(direction) = recognizer.process(&data) {
                let msg = format!("Gesture Detected: {:?}", direction);
                tracing::info!("{}", msg);
                self.status_message = Some(StatusMessage {
                    message: msg.clone(),
                    severity: MessageSeverity::Info,
                });

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
                    }
                    _ => {}
                }
            }
        }

        let now = Instant::now();
        let debounce_duration = Duration::from_millis(50);

        if data.trigger_button != self.last_trigger_state {
            if self
                .trigger_debounce
                .map_or(true, |last| now.duration_since(last) > debounce_duration)
            {
                self.last_trigger_state = data.trigger_button;
                self.trigger_debounce = Some(now);
                if data.trigger_button {
                    let _ = self.input_simulator.mouse_left_down();
                } else {
                    let _ = self.input_simulator.mouse_left_up();
                }
            }
        }

        if data.touchpad_button != self.last_touchpad_button_state {
            if self
                .touchpad_btn_debounce
                .map_or(true, |last| now.duration_since(last) > debounce_duration)
            {
                self.last_touchpad_button_state = data.touchpad_button;
                self.touchpad_btn_debounce = Some(now);
                if data.touchpad_button {
                    let _ = self.input_simulator.mouse_right_down();
                } else {
                    let _ = self.input_simulator.mouse_right_up();
                }
            }
        }

        if data.volume_up_button {
            if self
                .volume_up_debounce
                .map_or(true, |last| now.duration_since(last) > debounce_duration)
            {
                let _ = self.input_simulator.mouse_wheel(1);
                self.volume_up_debounce = Some(now);
            }
        }

        if data.volume_down_button {
            if self
                .volume_down_debounce
                .map_or(true, |last| now.duration_since(last) > debounce_duration)
            {
                let _ = self.input_simulator.mouse_wheel(-1);
                self.volume_down_debounce = Some(now);
            }
        }

        if data.back_button != self.last_back_button_state {
            if self
                .back_btn_debounce
                .map_or(true, |last| now.duration_since(last) > debounce_duration)
            {
                self.last_back_button_state = data.back_button;
                self.back_btn_debounce = Some(now);
                if data.back_button {
                    let _ = self.input_simulator.key_press(VK_ESCAPE);
                }
            }
        }

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
}

impl eframe::App for GearVRApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                        if let Some(addr) = self.last_connected_address {
                            if let Ok(mut settings) = self.settings.lock() {
                                let _ = settings.add_known_address(addr);
                            }
                        }
                    } else if let ConnectionStatus::Disconnected = status {
                        if self.auto_reconnect {
                            self.reconnect_timer =
                                Some(Instant::now() + Duration::from_millis(2000));

                            // Optimization: Only set "Reconnecting" message if there is no current Error message
                            // This prevents hiding critical diagnostic buttons that help fix the root cause.
                            let should_update_msg = self
                                .status_message
                                .as_ref()
                                .map_or(true, |m| m.severity != MessageSeverity::Error);

                            if should_update_msg {
                                self.status_message = Some(StatusMessage {
                                    message: "Disconnected. Reconnecting in 2s...".to_string(),
                                    severity: MessageSeverity::Warning,
                                });
                            }
                        }
                    }
                }
                AppEvent::LogMessage(msg) => {
                    // Optimization: If a critical error occurs, stop auto-reconnecting
                    // to give the user time to use diagnostic tools.
                    if msg.severity == MessageSeverity::Error {
                        self.auto_reconnect = false;
                        self.reconnect_timer = None;
                    }
                    self.status_message = Some(msg);
                }
                AppEvent::DeviceFound(device) => {
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

        ctx.request_repaint();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.selectable_value(&mut self.selected_tab, Tab::Home, "Home");
                ui.selectable_value(&mut self.selected_tab, Tab::Calibration, "Calibration");
                ui.selectable_value(&mut self.selected_tab, Tab::Settings, "Settings");
                ui.selectable_value(&mut self.selected_tab, Tab::Debug, "Debug");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let switch_icon = if self.is_dark_mode {
                        "☀ Light"
                    } else {
                        "🌙 Dark"
                    };
                    if ui.button(switch_icon).clicked() {
                        self.is_dark_mode = !self.is_dark_mode;
                        crate::presentation::theme::configure_neubrutalism(ctx, self.is_dark_mode);
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.set_max_width(800.0);
                    ui.add_space(20.0);

                    use crate::presentation::tabs;
                    match self.selected_tab {
                        Tab::Home => tabs::home::render(self, ui),
                        Tab::Calibration => tabs::calibration::render(self, ui),
                        Tab::Settings => tabs::settings::render(self, ui),
                        Tab::Debug => tabs::debug::render(self, ui),
                    }

                    ui.add_space(50.0);
                });
            });
        });
    }
}
