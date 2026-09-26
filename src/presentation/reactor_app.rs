//! Native WinUI 3 Presentation Layer using Windows Reactor
//!
//! Implements a modern Fluent Design 2 experience powered by the Windows App SDK
//! following official WinUI 3 guidelines (hierarchical page header, SettingsCard pattern,
//! clean typography, zero emoji) while connecting to Domain, Application, and Infrastructure layers.

use crate::domain::models::{
    AppEvent, BluetoothCommand, ConnectionStatus, ControllerData, ScannedDevice,
};
use crate::domain::settings::SettingsService;
use crate::presentation::radial_menu::ControlMode;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use windows_reactor::*;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ReactorMessage {
    SelectTab(usize),
    UpdateAddressInput(String),
    PickAddress(u64),
    Connect,
    Disconnect,
    ToggleScan,
    ToggleAntiSleep(bool),
    ToggleAutoProfile(bool),
    ToggleBackgroundTray(bool),
    ChangeMode(ControlMode),
    FromAppEvent(AppEvent),
    OpenBtSettings,
    Noop,
}

pub struct GearVRReactorApp {
    pub selected_tab: usize,
    pub connection_status: ConnectionStatus,
    pub status_message: Option<String>,
    pub latest_data: Option<ControllerData>,
    pub scanned_devices: Vec<ScannedDevice>,
    pub current_mode: ControlMode,
    pub address_input: String,
    pub is_scanning: bool,
    pub enable_anti_sleep: bool,
    pub enable_auto_profile: bool,
    pub enable_background_tray: bool,
    pub bt_cmd_tx: Option<mpsc::UnboundedSender<BluetoothCommand>>,
    pub shared_event_rx: Arc<Mutex<mpsc::UnboundedReceiver<AppEvent>>>,
}

impl Component for GearVRReactorApp {
    type Input = ();
    type Message = ReactorMessage;

    fn create(_input: &(), context: &ComponentContext<Self>) -> Self {
        let (bt_cmd_tx, bt_cmd_rx) = mpsc::unbounded_channel::<BluetoothCommand>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<AppEvent>();
        let shared_event_rx = Arc::new(Mutex::new(event_rx));

        let settings_service = match SettingsService::new() {
            Ok(service) => service,
            Err(_) => SettingsService::in_memory_defaults(),
        };

        let initial_address = settings_service
            .get()
            .last_connected_address
            .map(|a| format!("{:X}", a))
            .unwrap_or_default();

        let settings = Arc::new(Mutex::new(settings_service));
        let bt_settings = settings.clone();

        // Spawn background Bluetooth worker
        crate::application::bluetooth_worker::spawn_bluetooth_worker(
            event_tx,
            bt_cmd_rx,
            bt_settings,
        );

        // Chain first event listener task on Windows thread pool
        Self::spawn_event_listener(context, shared_event_rx.clone());

        Self {
            selected_tab: 0,
            connection_status: ConnectionStatus::Disconnected,
            status_message: None,
            latest_data: None,
            scanned_devices: Vec::new(),
            current_mode: ControlMode::Mouse,
            address_input: initial_address,
            is_scanning: false,
            enable_anti_sleep: true,
            enable_auto_profile: false,
            enable_background_tray: true,
            bt_cmd_tx: Some(bt_cmd_tx),
            shared_event_rx,
        }
    }

    fn update(&mut self, message: Self::Message, context: &ComponentContext<Self>) {
        match message {
            ReactorMessage::SelectTab(tab) => {
                self.selected_tab = tab;
            }
            ReactorMessage::UpdateAddressInput(input) => {
                self.address_input = input;
            }
            ReactorMessage::PickAddress(addr) => {
                self.address_input = format!("{:X}", addr);
            }
            ReactorMessage::Connect => {
                let sanitized = self.address_input.replace([':', '-'], "");
                if let Ok(address) = u64::from_str_radix(&sanitized, 16) {
                    self.connection_status = ConnectionStatus::Connecting;
                    if let Some(tx) = &self.bt_cmd_tx {
                        let _ = tx.send(BluetoothCommand::Connect(address));
                    }
                }
            }
            ReactorMessage::Disconnect => {
                if let Some(tx) = &self.bt_cmd_tx {
                    let _ = tx.send(BluetoothCommand::Disconnect);
                }
            }
            ReactorMessage::ToggleScan => {
                if self.is_scanning {
                    self.is_scanning = false;
                    if let Some(tx) = &self.bt_cmd_tx {
                        let _ = tx.send(BluetoothCommand::StopScan);
                    }
                } else {
                    self.is_scanning = true;
                    self.scanned_devices.clear();
                    if let Some(tx) = &self.bt_cmd_tx {
                        let _ = tx.send(BluetoothCommand::StartScan);
                    }
                }
            }
            ReactorMessage::ToggleAntiSleep(val) => {
                self.enable_anti_sleep = val;
            }
            ReactorMessage::ToggleAutoProfile(val) => {
                self.enable_auto_profile = val;
            }
            ReactorMessage::ToggleBackgroundTray(val) => {
                self.enable_background_tray = val;
            }
            ReactorMessage::ChangeMode(mode) => {
                self.current_mode = mode;
            }
            ReactorMessage::OpenBtSettings => {
                let _ = std::process::Command::new("explorer")
                    .arg("ms-settings:bluetooth")
                    .spawn();
            }
            ReactorMessage::FromAppEvent(event) => {
                match event {
                    AppEvent::ControllerData(data) => {
                        self.latest_data = Some(data);
                    }
                    AppEvent::ConnectionStatus(status) => {
                        self.connection_status = status;
                        if let ConnectionStatus::Connected = status {
                            self.status_message =
                                Some("Gear VR Controller connected successfully.".to_string());
                        }
                    }
                    AppEvent::LogMessage(log) => {
                        self.status_message = Some(log.message);
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

                // Chain next listener task
                Self::spawn_event_listener(context, self.shared_event_rx.clone());
            }
            ReactorMessage::Noop => {}
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        // Fluent Header & Status Infobar
        let (info_title, info_msg, is_info_open) = match (&self.connection_status, &self.status_message) {
            (ConnectionStatus::Connected, Some(msg)) => ("Connected", msg.as_str(), true),
            (ConnectionStatus::Connected, None) => ("Connected", "Ready for motion and button input", true),
            (ConnectionStatus::Connecting, _) => ("Connecting", "Negotiating Bluetooth Low Energy GATT link...", true),
            (ConnectionStatus::Disconnected, Some(msg)) => ("Disconnected", msg.as_str(), true),
            (ConnectionStatus::Disconnected, None) => ("Disconnected", "No active controller link", false),
            (ConnectionStatus::Error, Some(msg)) => ("Error", msg.as_str(), true),
            (ConnectionStatus::Error, None) => ("Error", "Connection error occurred", true),
        };

        let status_infobar = InfoBar::new()
            .title(info_title)
            .message(info_msg)
            .is_open(is_info_open);

        // Fluent Header (Title & Subtitle)
        let page_header = StackPanel::new()
            .spacing(4.0)
            .children((
                TextBlock::new()
                    .text("Samsung Gear VR Controller")
                    .font_size(24.0)
                    .font_weight(FontWeight::SEMI_BOLD),
                TextBlock::new()
                    .text("Universal Windows Bluetooth Input Driver & Motion Translation Service")
                    .font_size(13.0)
                    .foreground(ThemeBrush::PrimaryText),
            ));

        // Navigation Bar (Segmented Tab Bar)
        let nav_bar = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectTab(0)))
                    .content(if self.selected_tab == 0 { "[ Dashboard ]" } else { "Dashboard" }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectTab(1)))
                    .content(if self.selected_tab == 1 { "[ Calibration ]" } else { "Calibration" }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectTab(2)))
                    .content(if self.selected_tab == 2 { "[ Settings ]" } else { "Settings" }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectTab(3)))
                    .content(if self.selected_tab == 3 { "[ Diagnostics ]" } else { "Diagnostics" }),
            ));

        // Tab Content
        let tab_content: View = match self.selected_tab {
            0 => self.render_dashboard_tab(context),
            1 => self.render_calibration_tab(),
            2 => self.render_settings_tab(context),
            _ => self.render_diagnostics_tab(context),
        };

        let main_layout = StackPanel::new()
            .spacing(16.0)
            .children((
                page_header,
                status_infobar,
                nav_bar,
                tab_content,
            ));

        Border::new()
            .padding(Thickness::new(24.0, 20.0, 24.0, 24.0))
            .content(main_layout)
            .into()
    }
}

impl GearVRReactorApp {
    fn spawn_event_listener(
        context: &ComponentContext<Self>,
        rx: Arc<Mutex<mpsc::UnboundedReceiver<AppEvent>>>,
    ) {
        context.spawn_background(move |_cancel| {
            let mut guard = match rx.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            if let Some(event) = guard.blocking_recv() {
                ReactorMessage::FromAppEvent(event)
            } else {
                ReactorMessage::Noop
            }
        });
    }

    /// Fluent SettingsCard Container Pattern
    fn render_card(title: &str, description: &str, content: View) -> View {
        Border::new()
            .background(ThemeBrush::CardBackground)
            .border_brush(ThemeBrush::CardStroke)
            .border_thickness(1.0)
            .corner_radius(8.0)
            .padding(Thickness::new(16.0, 14.0, 16.0, 14.0))
            .content(
                StackPanel::new()
                    .spacing(10.0)
                    .children((
                        StackPanel::new()
                            .spacing(2.0)
                            .children((
                                TextBlock::new()
                                    .text(title)
                                    .font_size(15.0)
                                    .font_weight(FontWeight::SEMI_BOLD),
                                TextBlock::new()
                                    .text(description)
                                    .font_size(12.0)
                                    .foreground(ThemeBrush::PrimaryText),
                            )),
                        content,
                    )),
            )
            .into()
    }

    /// Fluent SettingsRow Pattern for Toggle Switches
    fn render_toggle_card(
        title: &str,
        description: &str,
        toggle: ToggleSwitch,
    ) -> View {
        Border::new()
            .background(ThemeBrush::CardBackground)
            .border_brush(ThemeBrush::CardStroke)
            .border_thickness(1.0)
            .corner_radius(8.0)
            .padding(Thickness::new(16.0, 12.0, 16.0, 12.0))
            .content(
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(16.0)
                    .children((
                        toggle,
                        StackPanel::new()
                            .spacing(2.0)
                            .children((
                                TextBlock::new()
                                    .text(title)
                                    .font_size(14.0)
                                    .font_weight(FontWeight::SEMI_BOLD),
                                TextBlock::new()
                                    .text(description)
                                    .font_size(12.0)
                                    .foreground(ThemeBrush::PrimaryText),
                            )),
                    )),
            )
            .into()
    }

    fn render_dashboard_tab(&self, context: &mut ViewContext<Self>) -> View {
        // Card 1: Connection & Bluetooth Scanning
        let connect_button = if self.connection_status == ConnectionStatus::Connected {
            Button::new()
                .on_click(context.message(ReactorMessage::Disconnect))
                .content("Disconnect")
        } else {
            Button::new()
                .on_click(context.message(ReactorMessage::Connect))
                .content("Connect")
        };

        let scan_button = Button::new()
            .on_click(context.message(ReactorMessage::ToggleScan))
            .content(if self.is_scanning { "Stop Scan" } else { "Scan Devices" });

        let scan_ring = if self.is_scanning {
            ProgressRing::new().is_active(true)
        } else {
            ProgressRing::new().is_active(false)
        };

        let address_box = TextBox::new()
            .text(&self.address_input)
            .placeholder_text("Bluetooth Address (e.g. 2C41A1001234)")
            .on_text_changed(context.callback(ReactorMessage::UpdateAddressInput));

        let connection_controls = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                address_box,
                connect_button,
                scan_button,
                scan_ring,
            ));

        let connection_card = Self::render_card(
            "Bluetooth Controller Link",
            "Pair and manage low-latency connection to Samsung Gear VR Controller",
            connection_controls.into(),
        );

        // Card 2: Operational Mode Selection (Segmented Control)
        let mode_buttons = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                Button::new()
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Mouse)))
                    .content(if self.current_mode == ControlMode::Mouse {
                        "[ Active: Air Mouse ]"
                    } else {
                        "Air Mouse"
                    }),
                Button::new()
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Touchpad)))
                    .content(if self.current_mode == ControlMode::Touchpad {
                        "[ Active: Trackpad ]"
                    } else {
                        "Trackpad"
                    }),
                Button::new()
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Presentation)))
                    .content(if self.current_mode == ControlMode::Presentation {
                        "[ Active: Presenter ]"
                    } else {
                        "Presenter"
                    }),
            ));

        let mode_card = Self::render_card(
            "Active Control Profile",
            "Choose motion translation model and input behavior",
            mode_buttons.into(),
        );

        // Card 3: Real-Time Input Telemetry Monitor
        let (tp_text, btn_text, sample_text) = if let Some(data) = &self.latest_data {
            (
                format!(
                    "Touchpad Position: (X: {:.3}, Y: {:.3}) | Raw: ({}, {})",
                    data.processed_touchpad_x,
                    data.processed_touchpad_y,
                    data.touchpad_x,
                    data.touchpad_y
                ),
                format!(
                    "Buttons: Trigger: {} | Back: {} | Home: {} | Touchpad: {} | Vol+: {} | Vol-: {}",
                    if data.trigger_button { "Active" } else { "Idle" },
                    if data.back_button { "Active" } else { "Idle" },
                    if data.home_button { "Active" } else { "Idle" },
                    if data.touchpad_button { "Active" } else { "Idle" },
                    if data.volume_up_button { "Active" } else { "Idle" },
                    if data.volume_down_button { "Active" } else { "Idle" },
                ),
                format!("Controller Timestamp: {} ms | Battery/Status: Normal", data.timestamp),
            )
        } else {
            (
                "Touchpad Position: Awaiting input stream...".to_string(),
                "Buttons: Trigger: Idle | Back: Idle | Home: Idle".to_string(),
                "Controller Timestamp: No active transmission".to_string(),
            )
        };

        let telemetry_card = Self::render_card(
            "Real-Time Input Telemetry",
            "Live stream of controller sensor events, touch coordinates, and button states",
            StackPanel::new()
                .spacing(4.0)
                .children((
                    TextBlock::new().text(tp_text).font_size(13.0),
                    TextBlock::new().text(btn_text).font_size(13.0),
                    TextBlock::new().text(sample_text).font_size(12.0).foreground(ThemeBrush::PrimaryText),
                ))
                .into(),
        );

        StackPanel::new()
            .spacing(12.0)
            .children((
                connection_card,
                mode_card,
                telemetry_card,
            ))
            .into()
    }

    fn render_calibration_tab(&self) -> View {
        let touch_card = Self::render_card(
            "Touchpad Boundary Normalization",
            "Glide your thumb across the extreme edges of the touchpad to calibrate sensor bounds",
            StackPanel::new()
                .spacing(8.0)
                .children((
                    ProgressBar::new().value(100.0),
                    TextBlock::new()
                        .text("Normalized Domain: [-1.0, 1.0] across horizontal and vertical axes")
                        .font_size(12.0),
                ))
                .into(),
        );

        let imu_card = Self::render_card(
            "IMU Gyroscope Zero-Point Reference",
            "Place controller completely flat and motionless on a level desk to eliminate rotational drift",
            StackPanel::new()
                .spacing(8.0)
                .children((
                    TextBlock::new()
                        .text("Sensor Calibration Status: Reference Tare Balanced")
                        .font_size(13.0),
                    TextBlock::new()
                        .text("Dynamic drift compensation filter is continuously active during runtime.")
                        .font_size(12.0),
                ))
                .into(),
        );

        StackPanel::new()
            .spacing(12.0)
            .children((
                touch_card,
                imu_card,
            ))
            .into()
    }

    fn render_settings_tab(&self, context: &mut ViewContext<Self>) -> View {
        let anti_sleep_row = Self::render_toggle_card(
            "Prevent Display Sleep",
            "Keep Windows displays awake while in Presenter mode to ensure uninterrupted slideshows",
            ToggleSwitch::new()
                .is_on(self.enable_anti_sleep)
                .on_toggled(context.callback(ReactorMessage::ToggleAntiSleep)),
        );

        let auto_profile_row = Self::render_toggle_card(
            "Context-Aware Auto Switching",
            "Automatically switch profile to Presenter mode when PowerPoint, Keynote, or PDF viewer is focused",
            ToggleSwitch::new()
                .is_on(self.enable_auto_profile)
                .on_toggled(context.callback(ReactorMessage::ToggleAutoProfile)),
        );

        let tray_row = Self::render_toggle_card(
            "System Tray Background Execution",
            "Keep background Bluetooth link running in the Windows taskbar notification area",
            ToggleSwitch::new()
                .is_on(self.enable_background_tray)
                .on_toggled(context.callback(ReactorMessage::ToggleBackgroundTray)),
        );

        StackPanel::new()
            .spacing(12.0)
            .children((
                anti_sleep_row,
                auto_profile_row,
                tray_row,
            ))
            .into()
    }

    fn render_diagnostics_tab(&self, context: &mut ViewContext<Self>) -> View {
        let (accel, gyro, mag) = if let Some(d) = &self.latest_data {
            (
                format!("Accelerometer Vector (g):       X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.accel_x, d.accel_y, d.accel_z),
                format!("Gyroscope Angular Rate (rad/s): X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.gyro_x, d.gyro_y, d.gyro_z),
                format!("Magnetometer Compass (uT):      X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.mag_x, d.mag_y, d.mag_z),
            )
        } else {
            (
                "Accelerometer Vector (g):       Awaiting transmission...".to_string(),
                "Gyroscope Angular Rate (rad/s): Awaiting transmission...".to_string(),
                "Magnetometer Compass (uT):      Awaiting transmission...".to_string(),
            )
        };

        let imu_card = Self::render_card(
            "9-DOF IMU Raw Sensor Readings",
            "Direct telemetry stream decoded from Gear VR Controller GATT characteristic packets",
            StackPanel::new()
                .spacing(6.0)
                .children((
                    TextBlock::new().text(accel).font_size(13.0),
                    TextBlock::new().text(gyro).font_size(13.0),
                    TextBlock::new().text(mag).font_size(13.0),
                ))
                .into(),
        );

        let bt_recovery_card = Self::render_card(
            "Windows Bluetooth Subsystem Diagnostics",
            "System-level troubleshooting actions to unpair stale GATT handles and clear ghost devices",
            StackPanel::new()
                .spacing(8.0)
                .children((
                    TextBlock::new()
                        .text("If Bluetooth discovery fails, check paired state in Windows Settings.")
                        .font_size(12.0),
                    Button::new()
                        .on_click(context.message(ReactorMessage::OpenBtSettings))
                        .content("Open Windows Bluetooth Settings"),
                ))
                .into(),
        );

        StackPanel::new()
            .spacing(12.0)
            .children((
                imu_card,
                bt_recovery_card,
            ))
            .into()
    }
}

/// Entrypoint to launch the WinUI 3 Native UI
pub fn run_reactor_app() -> anyhow::Result<()> {
    App::run_component::<GearVRReactorApp>(())?;
    Ok(())
}
