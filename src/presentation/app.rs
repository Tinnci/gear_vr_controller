use crate::domain::controller::TouchpadProcessor;
use crate::domain::gestures::{GestureDirection, GestureRecognizer};
use crate::domain::imu::ImuProcessor;
use crate::domain::models::{
    AppEvent, BluetoothCommand, CalibrationState, ConnectionStatus, ControllerData,
    MessageSeverity, ScannedDevice, StatusMessage, Tab,
};
use crate::domain::settings::SettingsService;
use crate::infrastructure::input_simulator::InputSimulator;
use crate::presentation::radial_menu::{ControlMode, RadialMenu};
use eframe::egui::{self, Pos2};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_LEFT, VK_LMENU, VK_RIGHT, VK_VOLUME_DOWN, VK_VOLUME_UP,
};

const BUTTON_DEBOUNCE: Duration = Duration::from_millis(50);
const RADIAL_MENU_HOLD: Duration = Duration::from_millis(300);

fn debounce_ready(last: Option<Instant>, now: Instant, duration: Duration) -> bool {
    last.is_none_or(|last| now.duration_since(last) > duration)
}

#[derive(Default)]
struct InputFeatureFlags {
    touchpad: bool,
    buttons: bool,
    gestures: bool,
}

pub struct GearVRApp {
    // Services
    pub(crate) settings: Arc<Mutex<SettingsService>>,
    pub(crate) input_simulator: InputSimulator,
    pub(crate) touchpad_processor: Option<TouchpadProcessor>,
    pub(crate) gesture_recognizer: Option<GestureRecognizer>,
    pub(crate) imu_processor: Option<ImuProcessor>,

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

    // Radial Menu
    pub(crate) radial_menu: RadialMenu,
    pub(crate) current_control_mode: ControlMode,
    pub(crate) back_hold_start: Option<Instant>,
}

impl GearVRApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply Neubrutalism Style (default Light)
        crate::presentation::theme::configure_neubrutalism(&cc.egui_ctx, false);

        let settings_service = match SettingsService::new() {
            Ok(service) => service,
            Err(e) => {
                eprintln!("Failed to load settings; using in-memory defaults: {}", e);
                SettingsService::in_memory_defaults()
            }
        };

        let logging_guard =
            crate::infrastructure::logging::init_logger(&settings_service.get().log_settings)
                .map_err(|e| eprintln!("Failed to initialize logging: {}", e))
                .ok();

        tracing::info!("Starting Gear VR Controller Application");

        let settings = Arc::new(Mutex::new(settings_service));
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        let (bt_cmd_tx, bt_cmd_rx) = mpsc::unbounded_channel();
        let bt_settings = settings.clone();

        crate::application::bluetooth_worker::spawn_bluetooth_worker(
            data_tx,
            bt_cmd_rx,
            bt_settings,
        );

        let touchpad_processor = Some(TouchpadProcessor::new(settings.clone()));
        let gesture_recognizer = Some(GestureRecognizer::new(settings.clone()));
        let imu_processor = Some(ImuProcessor::new(settings.clone()));
        let last_connected_address = match settings.lock() {
            Ok(settings) => settings.get().last_connected_address,
            Err(_) => {
                tracing::warn!("Settings lock poisoned during startup");
                None
            }
        };

        Self {
            settings,
            input_simulator: InputSimulator::new(),
            touchpad_processor,
            gesture_recognizer,
            imu_processor,
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
            radial_menu: RadialMenu::new(),
            current_control_mode: ControlMode::default(),
            back_hold_start: None,
        }
    }

    fn process_controller_data(&mut self, mut data: ControllerData) {
        let flags = self.input_feature_flags();
        let menu_active = self.radial_menu.is_visible;

        self.normalize_touchpad(&mut data);

        if !menu_active {
            self.process_motion_input(&data, flags.touchpad);
            if flags.gestures {
                self.process_gesture_input(&data);
            }
        }

        if flags.buttons {
            self.process_button_input(&data, Instant::now());
        }

        self.capture_calibration_sample(&data);
        self.latest_controller_data = Some(data);
    }

    fn input_feature_flags(&self) -> InputFeatureFlags {
        match self.settings.lock() {
            Ok(settings) => {
                let settings = settings.get();
                InputFeatureFlags {
                    touchpad: settings.enable_touchpad,
                    buttons: settings.enable_buttons,
                    gestures: settings.enable_gestures,
                }
            }
            Err(_) => {
                tracing::warn!("Settings lock poisoned; disabling controller input for this frame");
                InputFeatureFlags::default()
            }
        }
    }

    fn normalize_touchpad(&mut self, data: &mut ControllerData) {
        if let Some(processor) = &mut self.touchpad_processor {
            processor.process(data);
        }
    }

    fn process_motion_input(&mut self, data: &ControllerData, touchpad_enabled: bool) {
        match self.current_control_mode {
            ControlMode::Mouse => self.process_mouse_mode_input(data, touchpad_enabled),
            ControlMode::Touchpad => self.process_touchpad_mode_input(data, touchpad_enabled),
            ControlMode::Presentation | ControlMode::Settings => {}
        }
    }

    fn process_mouse_mode_input(&mut self, data: &ControllerData, touchpad_enabled: bool) {
        if let Some(imu) = &mut self.imu_processor {
            if let Some((dx, dy)) = imu.calculate_airmouse_delta(data) {
                let _ = self.input_simulator.move_mouse(dx, dy);
            }
        }

        if touchpad_enabled && data.touchpad_touched {
            self.process_touchpad_scroll(data);
        }
    }

    fn process_touchpad_scroll(&mut self, data: &ControllerData) {
        let Some(processor) = &mut self.touchpad_processor else {
            return;
        };

        let fallback = (data.touchpad_x as f64, data.touchpad_y as f64);
        let (last_x, last_y) = processor.last_processed_pos.unwrap_or(fallback);
        let dx = data.touchpad_x as f64 - last_x;
        let dy = data.touchpad_y as f64 - last_y;
        let threshold = 0.05;

        if dy.abs() > threshold {
            let scroll = if dy > 0.0 { -1 } else { 1 };
            let _ = self.input_simulator.mouse_wheel(scroll);
        }
        if dx.abs() > threshold {
            let scroll = if dx > 0.0 { 1 } else { -1 };
            let _ = self.input_simulator.mouse_h_wheel(scroll);
        }
    }

    fn process_touchpad_mode_input(&mut self, data: &ControllerData, touchpad_enabled: bool) {
        if !touchpad_enabled || !data.touchpad_touched {
            return;
        }

        if let Some(processor) = &mut self.touchpad_processor {
            if let Some((dx, dy)) = processor.calculate_mouse_delta(data) {
                let _ = self.input_simulator.move_mouse(dx, dy);
            }
        }
    }

    fn process_gesture_input(&mut self, data: &ControllerData) {
        let Some(recognizer) = &mut self.gesture_recognizer else {
            return;
        };
        let Some(direction) = recognizer.process(data) else {
            return;
        };

        let msg = format!("Gesture Detected: {:?}", direction);
        tracing::info!("{}", msg);
        self.status_message = Some(StatusMessage {
            message: msg,
            severity: MessageSeverity::Info,
        });

        match direction {
            GestureDirection::Up => {
                let _ = self.input_simulator.mouse_wheel(1);
            }
            GestureDirection::Down => {
                let _ = self.input_simulator.mouse_wheel(-1);
            }
            GestureDirection::Left | GestureDirection::Right => self.press_key(VK_LMENU),
            GestureDirection::None => {}
        }
    }

    fn process_button_input(&mut self, data: &ControllerData, now: Instant) {
        self.handle_trigger_button(data, now);
        self.handle_touchpad_button(data, now);
        self.handle_back_button(data, now);
        self.handle_volume_buttons(data, now);
    }

    fn handle_trigger_button(&mut self, data: &ControllerData, now: Instant) {
        if data.trigger_button == self.last_trigger_state
            || !debounce_ready(self.trigger_debounce, now, BUTTON_DEBOUNCE)
        {
            return;
        }

        self.last_trigger_state = data.trigger_button;
        self.trigger_debounce = Some(now);

        match (self.current_control_mode, data.trigger_button) {
            (ControlMode::Mouse | ControlMode::Touchpad, true) => {
                let _ = self.input_simulator.mouse_left_down();
            }
            (ControlMode::Mouse | ControlMode::Touchpad, false) => {
                let _ = self.input_simulator.mouse_left_up();
            }
            (ControlMode::Presentation, true) => self.press_key(VK_RIGHT),
            _ => {}
        }
    }

    fn handle_touchpad_button(&mut self, data: &ControllerData, now: Instant) {
        if data.touchpad_button == self.last_touchpad_button_state
            || !debounce_ready(self.touchpad_btn_debounce, now, BUTTON_DEBOUNCE)
        {
            return;
        }

        self.last_touchpad_button_state = data.touchpad_button;
        self.touchpad_btn_debounce = Some(now);

        match (self.current_control_mode, data.touchpad_button) {
            (ControlMode::Mouse | ControlMode::Touchpad, true) => {
                let _ = self.input_simulator.mouse_right_down();
            }
            (ControlMode::Mouse | ControlMode::Touchpad, false) => {
                let _ = self.input_simulator.mouse_right_up();
            }
            (ControlMode::Presentation, true) => self.press_key(VK_LEFT),
            _ => {}
        }
    }

    fn handle_back_button(&mut self, data: &ControllerData, now: Instant) {
        if data.back_button {
            self.handle_back_pressed(data, now);
        } else {
            self.handle_back_released(now);
        }
    }

    fn handle_back_pressed(&mut self, data: &ControllerData, now: Instant) {
        let Some(start_time) = self.back_hold_start else {
            self.back_hold_start = Some(now);
            return;
        };

        if now.duration_since(start_time) >= RADIAL_MENU_HOLD && !self.radial_menu.is_visible {
            if let Ok((x, y)) = self.input_simulator.get_cursor_pos() {
                self.radial_menu.show(Pos2::new(x as f32, y as f32));
            }
        }

        if self.radial_menu.is_visible && data.touchpad_touched {
            self.radial_menu
                .update_selection(data.processed_touchpad_x, data.processed_touchpad_y);
        }
    }

    fn handle_back_released(&mut self, now: Instant) {
        let Some(start_time) = self.back_hold_start else {
            return;
        };

        let hold_duration = now.duration_since(start_time);
        if self.radial_menu.is_visible {
            self.apply_radial_menu_selection();
        } else if hold_duration < RADIAL_MENU_HOLD {
            self.handle_quick_back_tap(now);
        }

        self.back_hold_start = None;
    }

    fn apply_radial_menu_selection(&mut self) {
        let Some(selected_mode) = self.radial_menu.hide() else {
            return;
        };

        if selected_mode == ControlMode::Settings {
            self.selected_tab = Tab::Settings;
        } else {
            self.current_control_mode = selected_mode;
        }

        self.status_message = Some(StatusMessage {
            message: format!(
                "Mode: {} - {}",
                selected_mode.name(),
                selected_mode.description()
            ),
            severity: MessageSeverity::Success,
        });
    }

    fn handle_quick_back_tap(&mut self, now: Instant) {
        if !debounce_ready(self.back_btn_debounce, now, BUTTON_DEBOUNCE) {
            return;
        }

        self.back_btn_debounce = Some(now);
        match self.current_control_mode {
            ControlMode::Mouse | ControlMode::Touchpad => {
                let _ = self.input_simulator.mouse_right_click();
            }
            ControlMode::Presentation => self.press_key(VK_LEFT),
            ControlMode::Settings => {}
        }
    }

    fn handle_volume_buttons(&mut self, data: &ControllerData, now: Instant) {
        if data.volume_up_button && debounce_ready(self.volume_up_debounce, now, BUTTON_DEBOUNCE) {
            self.volume_up_debounce = Some(now);
            match self.current_control_mode {
                ControlMode::Mouse | ControlMode::Presentation => self.press_key(VK_VOLUME_UP),
                ControlMode::Touchpad => {
                    let _ = self.input_simulator.mouse_wheel(1);
                }
                ControlMode::Settings => {}
            }
        }

        if data.volume_down_button
            && debounce_ready(self.volume_down_debounce, now, BUTTON_DEBOUNCE)
        {
            self.volume_down_debounce = Some(now);
            match self.current_control_mode {
                ControlMode::Mouse | ControlMode::Presentation => self.press_key(VK_VOLUME_DOWN),
                ControlMode::Touchpad => {
                    let _ = self.input_simulator.mouse_wheel(-1);
                }
                ControlMode::Settings => {}
            }
        }
    }

    fn capture_calibration_sample(&mut self, data: &ControllerData) {
        if !self.is_calibrating || !data.touchpad_touched {
            return;
        }

        self.calibration_data
            .samples
            .push((data.touchpad_x, data.touchpad_y));
        self.calibration_data.min_x = self.calibration_data.min_x.min(data.touchpad_x);
        self.calibration_data.max_x = self.calibration_data.max_x.max(data.touchpad_x);
        self.calibration_data.min_y = self.calibration_data.min_y.min(data.touchpad_y);
        self.calibration_data.max_y = self.calibration_data.max_y.max(data.touchpad_y);
    }

    fn press_key(&self, key: VIRTUAL_KEY) {
        let _ = self.input_simulator.key_press(key);
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
                                .is_none_or(|m| m.severity != MessageSeverity::Error);

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

        // Render radial menu overlay (on top of everything)
        self.radial_menu.render(ctx);
    }
}
