//! Native WinUI 3 Presentation Layer using Windows Reactor
//!
//! Provides a modern Fluent Design 2 experience powered by the Windows App SDK
//! while connecting to the decoupled Domain, Application, and Infrastructure layers.

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
    ChangeMode(ControlMode),
    FromAppEvent(AppEvent),
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
                if let Ok(address) =
                    u64::from_str_radix(&self.address_input.replace(':', ""), 16)
                {
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
            ReactorMessage::ChangeMode(mode) => {
                self.current_mode = mode;
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
        let (info_title, info_msg, is_info_open) = match (&self.connection_status, &self.status_message) {
            (ConnectionStatus::Connected, Some(msg)) => ("Connected", msg.as_str(), true),
            (ConnectionStatus::Connected, None) => ("Connected", "Ready for input", true),
            (ConnectionStatus::Connecting, _) => ("Connecting", "Negotiating GATT connection...", true),
            (ConnectionStatus::Disconnected, Some(msg)) => ("Disconnected", msg.as_str(), true),
            (ConnectionStatus::Disconnected, None) => ("Disconnected", "No active controller link", false),
            (ConnectionStatus::Error, Some(msg)) => ("Error", msg.as_str(), true),
            (ConnectionStatus::Error, None) => ("Error", "Connection error occurred", true),
        };

        let status_infobar = InfoBar::new()
            .title(info_title)
            .message(info_msg)
            .is_open(is_info_open);

        // Header Navigation Bar
        let nav_bar = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectTab(0)))
                    .content(if self.selected_tab == 0 { "🏠 [Home]" } else { "🏠 Home" }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectTab(1)))
                    .content(if self.selected_tab == 1 { "🎯 [Calibration]" } else { "🎯 Calibration" }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectTab(2)))
                    .content(if self.selected_tab == 2 { "⚙️ [Settings]" } else { "⚙️ Settings" }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectTab(3)))
                    .content(if self.selected_tab == 3 { "🔍 [Debug]" } else { "🔍 Debug" }),
            ));

        // Tab Content
        let tab_content: View = match self.selected_tab {
            0 => self.render_home_tab(context),
            1 => self.render_calibration_tab(),
            2 => self.render_settings_tab(context),
            _ => self.render_debug_tab(),
        };

        StackPanel::new()
            .spacing(16.0)
            .children((
                TextBlock::new()
                    .text("Gear VR Controller - WinUI 3 Fluent Experience")
                    .font_size(24.0),
                status_infobar,
                nav_bar,
                tab_content,
            ))
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

    fn render_home_tab(&self, context: &mut ViewContext<Self>) -> View {
        let connect_button = if self.connection_status == ConnectionStatus::Connected {
            Button::new()
                .on_click(context.message(ReactorMessage::Disconnect))
                .content("Disconnect Controller")
        } else {
            Button::new()
                .on_click(context.message(ReactorMessage::Connect))
                .content("Connect Controller")
        };

        let scan_button = Button::new()
            .on_click(context.message(ReactorMessage::ToggleScan))
            .content(if self.is_scanning { "Stop Scanning" } else { "Scan for Gear VR" });

        let scan_indicator = if self.is_scanning {
            ProgressRing::new().is_active(true)
        } else {
            ProgressRing::new().is_active(false)
        };

        let mode_picker = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                TextBlock::new().text("Active Mode:"),
                Button::new()
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Mouse)))
                    .content(if self.current_mode == ControlMode::Mouse { "✈️ Air Mouse (Selected)" } else { "✈️ Air Mouse" }),
                Button::new()
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Touchpad)))
                    .content(if self.current_mode == ControlMode::Touchpad { "🖱️ Touchpad (Selected)" } else { "🖱️ Touchpad" }),
                Button::new()
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Presentation)))
                    .content(if self.current_mode == ControlMode::Presentation { "📽️ Presenter (Selected)" } else { "📽️ Presenter" }),
            ));

        let telemetry_info = if let Some(data) = &self.latest_data {
            format!(
                "Touchpad: ({:.1}, {:.1}) | Trigger: {} | Back: {}",
                data.touchpad_x,
                data.touchpad_y,
                if data.trigger_button { "Pressed" } else { "Released" },
                if data.back_button { "Pressed" } else { "Released" }
            )
        } else {
            "Telemetry: No active controller stream".to_string()
        };

        StackPanel::new()
            .spacing(12.0)
            .children((
                TextBlock::new().text("Connection & Control").font_size(18.0),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .children((
                        TextBlock::new().text(format!("Target Address: {}", self.address_input)),
                        connect_button,
                        scan_button,
                        scan_indicator,
                    )),
                mode_picker,
                TextBlock::new().text(telemetry_info),
            ))
            .into()
    }

    fn render_calibration_tab(&self) -> View {
        StackPanel::new()
            .spacing(12.0)
            .children((
                TextBlock::new().text("Touchpad Calibration").font_size(18.0),
                TextBlock::new().text("Slide your thumb across all edges of the touchpad to map boundaries."),
                ProgressBar::new().value(100.0),
                TextBlock::new().text("Touchpad Boundary: Normalized [-1.0, 1.0]"),
            ))
            .into()
    }

    fn render_settings_tab(&self, context: &mut ViewContext<Self>) -> View {
        StackPanel::new()
            .spacing(12.0)
            .children((
                TextBlock::new().text("Windows Integration Settings").font_size(18.0),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(12.0)
                    .children((
                        ToggleSwitch::new()
                            .is_on(self.enable_anti_sleep)
                            .on_toggled(context.callback(ReactorMessage::ToggleAntiSleep)),
                        TextBlock::new().text("Prevent Display Sleep during Presentation Mode"),
                    )),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(12.0)
                    .children((
                        ToggleSwitch::new()
                            .is_on(self.enable_auto_profile)
                            .on_toggled(context.callback(ReactorMessage::ToggleAutoProfile)),
                        TextBlock::new().text("Context-Aware Auto Profile Switching (PPT/Media detection)"),
                    )),
            ))
            .into()
    }

    fn render_debug_tab(&self) -> View {
        let (accel, gyro, mag) = if let Some(d) = &self.latest_data {
            (
                format!("Accel: ({:.2}, {:.2}, {:.2})", d.accel_x, d.accel_y, d.accel_z),
                format!("Gyro: ({:.2}, {:.2}, {:.2})", d.gyro_x, d.gyro_y, d.gyro_z),
                format!("Mag: ({:.2}, {:.2}, {:.2})", d.mag_x, d.mag_y, d.mag_z),
            )
        } else {
            (
                "Accel: n/a".to_string(),
                "Gyro: n/a".to_string(),
                "Mag: n/a".to_string(),
            )
        };

        StackPanel::new()
            .spacing(12.0)
            .children((
                TextBlock::new().text("Raw Telemetry & Diagnostics").font_size(18.0),
                TextBlock::new().text(accel),
                TextBlock::new().text(gyro),
                TextBlock::new().text(mag),
            ))
            .into()
    }
}

/// Entrypoint to launch the WinUI 3 Native UI
pub fn run_reactor_app() -> anyhow::Result<()> {
    App::run_component::<GearVRReactorApp>(())?;
    Ok(())
}
