use crate::domain::controller::TouchpadProcessor;
use crate::domain::gestures::{GestureDirection, GestureRecognizer};
use crate::domain::imu::ImuProcessor;
use crate::domain::models::{
    AppEvent, BluetoothCommand, CalibrationState, ConnectionStatus, ControllerData,
    MessageSeverity, ScannedDevice, StatusMessage, Tab,
};
use crate::domain::settings::SettingsService;
use crate::infrastructure::bluetooth::BluetoothService;
use crate::infrastructure::input_simulator::InputSimulator;
use crate::presentation::radial_menu::{ControlMode, RadialMenu};
use eframe::egui::{self, Pos2};
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

    // Radial Menu
    pub(crate) radial_menu: RadialMenu,
    pub(crate) current_control_mode: ControlMode,
    pub(crate) trigger_hold_start: Option<Instant>,
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
        let imu_processor = Some(ImuProcessor::new(settings.clone()));
        let last_connected_address = settings.lock().unwrap().get().last_connected_address;

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
            radial_menu: RadialMenu::new(),
            current_control_mode: ControlMode::default(),
            trigger_hold_start: None,
        }
    }

    fn process_controller_data(&mut self, mut data: ControllerData) {
        let (enable_tp, enable_btns, enable_gestures) = {
            let s = self.settings.lock().unwrap();
            let settings = s.get();
            (
                settings.enable_touchpad,
                settings.enable_buttons,
                settings.enable_gestures,
            )
        };

        // Skip normal touchpad/gesture processing when radial menu is active
        let menu_active = self.radial_menu.is_visible;
        // let input_disabled = self.current_control_mode == ControlMode::Disabled; // Disabled mode removed

        // Process touchpad data for normalization (needed for menu selection too)
        if let Some(processor) = &mut self.touchpad_processor {
            processor.process(&mut data);
        }

        // Handle input based on current control mode
        if !menu_active {
            match self.current_control_mode {
                ControlMode::Mouse => {
                    // --- AIR MOUSE MODE ---
                    // 1. IMU Cursor
                    if let Some(imu) = &mut self.imu_processor {
                        if let Some((dx, dy)) = imu.calculate_airmouse_delta(&data) {
                            let _ = self.input_simulator.move_mouse(dx, dy);
                        }
                    }

                    // 2. Touchpad Scroll (Vertical & Horizontal)
                    if enable_tp && data.touchpad_touched {
                        if let Some(processor) = &mut self.touchpad_processor {
                            // Use raw movement for scroll to avoid acceleration weirdness
                            let dx = data.touchpad_x as f64
                                - processor
                                    .last_processed_pos
                                    .unwrap_or((data.touchpad_x as f64, data.touchpad_y as f64))
                                    .0;
                            let dy = data.touchpad_y as f64
                                - processor
                                    .last_processed_pos
                                    .unwrap_or((data.touchpad_x as f64, data.touchpad_y as f64))
                                    .1;

                            // Scroll Threshold
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
                    }
                }
                ControlMode::Touchpad => {
                    // --- LAPTOP TRACKPAD MODE ---
                    // 1. Touchpad Cursor
                    if enable_tp && data.touchpad_touched {
                        if let Some(processor) = &mut self.touchpad_processor {
                            if let Some((dx, dy)) = processor.calculate_mouse_delta(&data) {
                                let _ = self.input_simulator.move_mouse(dx, dy);
                            }
                        }
                    }
                }
                ControlMode::Presentation | ControlMode::Settings => {
                    // No cursor movement in these modes
                }
            }
        }

        if enable_gestures && !menu_active {
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
                        GestureDirection::Left | GestureDirection::Right => {
                            let _ = self
                                .input_simulator
                                .key_press(windows::Win32::UI::Input::KeyboardAndMouse::VK_LMENU);
                        }
                        _ => {}
                    }
                }
            }
        }

        let now = Instant::now();
        let debounce_duration = Duration::from_millis(50);
        let menu_hold_threshold = Duration::from_millis(300);

        if enable_btns {
            // --- BUTTON MAPPING BASED ON MODE ---

            // Trigger Button
            if data.trigger_button != self.last_trigger_state {
                if self
                    .trigger_debounce
                    .map_or(true, |last| now.duration_since(last) > debounce_duration)
                {
                    self.last_trigger_state = data.trigger_button;
                    self.trigger_debounce = Some(now);

                    if data.trigger_button {
                        // Trigger Pressed
                        match self.current_control_mode {
                            ControlMode::Mouse | ControlMode::Touchpad => {
                                let _ = self.input_simulator.mouse_left_down();
                            }
                            ControlMode::Presentation => {
                                // Next Slide (Right Arrow)
                                let _ = self.input_simulator.key_press(
                                    windows::Win32::UI::Input::KeyboardAndMouse::VK_RIGHT,
                                );
                            }
                            _ => {}
                        }
                    } else {
                        // Trigger Released
                        match self.current_control_mode {
                            ControlMode::Mouse | ControlMode::Touchpad => {
                                let _ = self.input_simulator.mouse_left_up();
                            }
                            ControlMode::Presentation => {
                                // Key press already handled on down, no release needed for simple key.
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Touchpad Button (Center Click)
            if data.touchpad_button != self.last_touchpad_button_state {
                if self
                    .touchpad_btn_debounce
                    .map_or(true, |last| now.duration_since(last) > debounce_duration)
                {
                    self.last_touchpad_button_state = data.touchpad_button;
                    self.touchpad_btn_debounce = Some(now);
                    if data.touchpad_button {
                        // Touchpad Button Pressed
                        match self.current_control_mode {
                            ControlMode::Mouse | ControlMode::Touchpad => {
                                let _ = self.input_simulator.mouse_right_down();
                            }
                            ControlMode::Presentation => {
                                // Previous Slide (Left Arrow)
                                let _ = self.input_simulator.key_press(
                                    windows::Win32::UI::Input::KeyboardAndMouse::VK_LEFT,
                                );
                            }
                            _ => {}
                        }
                    } else {
                        // Touchpad Button Released
                        match self.current_control_mode {
                            ControlMode::Mouse | ControlMode::Touchpad => {
                                let _ = self.input_simulator.mouse_right_up();
                            }
                            ControlMode::Presentation => {
                                // Key press already handled on down.
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Back Button (Radial Menu Activator on Long Press, otherwise Escape)
            if data.back_button {
                if self.trigger_hold_start.is_none() {
                    // Reusing trigger_hold_start for back button hold
                    self.trigger_hold_start = Some(now);
                } else if let Some(start_time) = self.trigger_hold_start {
                    if now.duration_since(start_time) >= menu_hold_threshold
                        && !self.radial_menu.is_visible
                    {
                        // Show radial menu at current cursor position
                        if let Ok((x, y)) = self.input_simulator.get_cursor_pos() {
                            self.radial_menu.show(Pos2::new(x as f32, y as f32));
                        }
                    }

                    // Update menu selection based on touchpad
                    if self.radial_menu.is_visible && data.touchpad_touched {
                        self.radial_menu
                            .update_selection(data.processed_touchpad_x, data.processed_touchpad_y);
                    }
                }
            } else {
                // Back Button Released
                if let Some(start_time) = self.trigger_hold_start {
                    let hold_duration = now.duration_since(start_time);

                    if self.radial_menu.is_visible {
                        // Was showing radial menu - handle selection
                        if let Some(selected_mode) = self.radial_menu.hide() {
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
                    } else if hold_duration < menu_hold_threshold {
                        // Quick tap - normal back/escape behavior
                        if self
                            .back_btn_debounce
                            .map_or(true, |last| now.duration_since(last) > debounce_duration)
                        {
                            self.back_btn_debounce = Some(now);
                            match self.current_control_mode {
                                ControlMode::Mouse | ControlMode::Touchpad => {
                                    // Right Click
                                    let _ = self.input_simulator.mouse_right_click();
                                }
                                ControlMode::Presentation => {
                                    // Prev Slide
                                    let _ = self.input_simulator.key_press(
                                        windows::Win32::UI::Input::KeyboardAndMouse::VK_LEFT,
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    self.trigger_hold_start = None;
                }
            }

            // Volume Up Button
            if data.volume_up_button {
                if self
                    .volume_up_debounce
                    .map_or(true, |last| now.duration_since(last) > debounce_duration)
                {
                    self.volume_up_debounce = Some(now);
                    match self.current_control_mode {
                        ControlMode::Mouse => {
                            // Volume Up
                            let _ = self.input_simulator.key_press(
                                windows::Win32::UI::Input::KeyboardAndMouse::VK_VOLUME_UP,
                            );
                        }
                        ControlMode::Touchpad => {
                            // Scroll Up
                            let _ = self.input_simulator.mouse_wheel(1);
                        }
                        ControlMode::Presentation => {
                            // Volume Up
                            let _ = self.input_simulator.key_press(
                                windows::Win32::UI::Input::KeyboardAndMouse::VK_VOLUME_UP,
                            );
                        }
                        _ => {}
                    }
                }
            }

            // Volume Down Button
            if data.volume_down_button {
                if self
                    .volume_down_debounce
                    .map_or(true, |last| now.duration_since(last) > debounce_duration)
                {
                    self.volume_down_debounce = Some(now);
                    match self.current_control_mode {
                        ControlMode::Mouse => {
                            // Volume Down
                            let _ = self.input_simulator.key_press(
                                windows::Win32::UI::Input::KeyboardAndMouse::VK_VOLUME_DOWN,
                            );
                        }
                        ControlMode::Touchpad => {
                            // Scroll Down
                            let _ = self.input_simulator.mouse_wheel(-1);
                        }
                        ControlMode::Presentation => {
                            // Volume Down
                            let _ = self.input_simulator.key_press(
                                windows::Win32::UI::Input::KeyboardAndMouse::VK_VOLUME_DOWN,
                            );
                        }
                        _ => {}
                    }
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

        // Render radial menu overlay (on top of everything)
        self.radial_menu.render(ctx);
    }
}
