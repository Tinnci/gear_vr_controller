//! Native WinUI 3 Presentation Layer using Windows Reactor
//!
//! Implements a modern Fluent Design 2 experience powered by the Windows App SDK
//! following official WinUI 3 guidelines (NavigationView, SettingsCard pattern,
//! 4-language i18n auto-detection, clean typography, zero emoji).

use crate::domain::i18n::{Language, I18nStrings};
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
    SelectLanguage(Language),
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
    NavSelectionChanged(Option<String>),
    Noop,
}

pub struct GearVRReactorApp {
    pub selected_tab: usize,
    pub language: Language,
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
    pub settings_service: Arc<Mutex<SettingsService>>,
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

        let initial_lang = settings_service.get().language;
        let anti_sleep = settings_service.get().enable_presentation_anti_sleep;
        let auto_profile = settings_service.get().enable_auto_profile_switching;
        let tray = settings_service.get().minimize_to_tray;

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
            language: initial_lang,
            connection_status: ConnectionStatus::Disconnected,
            status_message: None,
            latest_data: None,
            scanned_devices: Vec::new(),
            current_mode: ControlMode::Mouse,
            address_input: initial_address,
            is_scanning: false,
            enable_anti_sleep: anti_sleep,
            enable_auto_profile: auto_profile,
            enable_background_tray: tray,
            settings_service: settings,
            bt_cmd_tx: Some(bt_cmd_tx),
            shared_event_rx,
        }
    }

    fn update(&mut self, message: Self::Message, context: &ComponentContext<Self>) {
        match message {
            ReactorMessage::SelectTab(tab) => {
                self.selected_tab = tab;
            }
            ReactorMessage::SelectLanguage(lang) => {
                self.language = lang;
                if let Ok(mut svc) = self.settings_service.lock() {
                    svc.get_mut().language = lang;
                    let _ = svc.save();
                }
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
                if let Ok(mut svc) = self.settings_service.lock() {
                    svc.get_mut().enable_presentation_anti_sleep = val;
                    let _ = svc.save();
                }
            }
            ReactorMessage::ToggleAutoProfile(val) => {
                self.enable_auto_profile = val;
                if let Ok(mut svc) = self.settings_service.lock() {
                    svc.get_mut().enable_auto_profile_switching = val;
                    let _ = svc.save();
                }
            }
            ReactorMessage::ToggleBackgroundTray(val) => {
                self.enable_background_tray = val;
                if let Ok(mut svc) = self.settings_service.lock() {
                    svc.get_mut().minimize_to_tray = val;
                    let _ = svc.save();
                }
            }
            ReactorMessage::ChangeMode(mode) => {
                self.current_mode = mode;
            }
            ReactorMessage::OpenBtSettings => {
                let _ = std::process::Command::new("explorer")
                    .arg("ms-settings:bluetooth")
                    .spawn();
            }
            ReactorMessage::NavSelectionChanged(Some(tag)) => {
                if let Ok(idx) = tag.parse::<usize>() {
                    self.selected_tab = idx;
                }
            }
            ReactorMessage::NavSelectionChanged(None) => {}
            ReactorMessage::FromAppEvent(event) => {
                match event {
                    AppEvent::ControllerData(data) => {
                        self.latest_data = Some(data);
                    }
                    AppEvent::ConnectionStatus(status) => {
                        self.connection_status = status;
                        if let ConnectionStatus::Connected = status {
                            let s = self.language.strings();
                            self.status_message = Some(s.status_ready.to_string());
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
        let s = self.language.strings();

        // Fluent Header & Status Infobar
        let (info_title, info_msg, is_info_open) = match (&self.connection_status, &self.status_message) {
            (ConnectionStatus::Connected, Some(msg)) => (s.status_connected, msg.as_str(), true),
            (ConnectionStatus::Connected, None) => (s.status_connected, s.status_ready, true),
            (ConnectionStatus::Connecting, _) => (s.status_connecting, s.status_negotiating, true),
            (ConnectionStatus::Disconnected, Some(msg)) => (s.status_disconnected, msg.as_str(), true),
            (ConnectionStatus::Disconnected, None) => (s.status_disconnected, s.status_no_link, false),
            (ConnectionStatus::Error, Some(msg)) => (s.status_error, msg.as_str(), true),
            (ConnectionStatus::Error, None) => (s.status_error, s.status_error, true),
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
                    .text(s.app_title)
                    .font_size(24.0)
                    .font_weight(FontWeight::SEMI_BOLD),
                TextBlock::new()
                    .text(s.app_subtitle)
                    .font_size(13.0)
                    .foreground(ThemeBrush::PrimaryText),
            ));

        // Fluent NavigationView Items (Windows 11 Navigation Architecture)
        let nav_items = [
            KeyedView::new(
                "0",
                NavigationViewItem::new()
                    .tag("0")
                    .is_selected(self.selected_tab == 0)
                    .slot(
                        NavigationViewItemSlot::Content,
                        TextBlock::new().text(s.nav_dashboard),
                    ),
            ),
            KeyedView::new(
                "1",
                NavigationViewItem::new()
                    .tag("1")
                    .is_selected(self.selected_tab == 1)
                    .slot(
                        NavigationViewItemSlot::Content,
                        TextBlock::new().text(s.nav_calibration),
                    ),
            ),
            KeyedView::new(
                "2",
                NavigationViewItem::new()
                    .tag("2")
                    .is_selected(self.selected_tab == 2)
                    .slot(
                        NavigationViewItemSlot::Content,
                        TextBlock::new().text(s.nav_settings),
                    ),
            ),
            KeyedView::new(
                "3",
                NavigationViewItem::new()
                    .tag("3")
                    .is_selected(self.selected_tab == 3)
                    .slot(
                        NavigationViewItemSlot::Content,
                        TextBlock::new().text(s.nav_diagnostics),
                    ),
            ),
        ];

        // Tab Content
        let tab_content: View = match self.selected_tab {
            0 => self.render_dashboard_tab(context, s),
            1 => self.render_calibration_tab(s),
            2 => self.render_settings_tab(context, s),
            _ => self.render_diagnostics_tab(context, s),
        };

        let content_area = Border::new()
            .padding(Thickness::new(24.0, 16.0, 24.0, 24.0))
            .content(
                StackPanel::new()
                    .spacing(16.0)
                    .children((
                        status_infobar,
                        tab_content,
                    )),
            );

        NavigationView::new()
            .pane_title(s.nav_pane_title)
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .is_settings_visible(false)
            .on_selected_tag_changed(context.callback(ReactorMessage::NavSelectionChanged))
            .slots([
                SlotView::collection(NavigationViewSlot::MenuItems, nav_items),
                SlotView::new(NavigationViewSlot::Header, page_header),
                SlotView::new(NavigationViewSlot::Content, content_area),
            ])
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

    fn render_dashboard_tab(&self, context: &mut ViewContext<Self>, s: &I18nStrings) -> View {
        // Card 1: Connection & Bluetooth Scanning
        let connect_button = if self.connection_status == ConnectionStatus::Connected {
            Button::new()
                .on_click(context.message(ReactorMessage::Disconnect))
                .content(s.disconnect_button)
        } else {
            Button::new()
                .on_click(context.message(ReactorMessage::Connect))
                .content(s.connect_button)
        };

        let scan_button = Button::new()
            .on_click(context.message(ReactorMessage::ToggleScan))
            .content(if self.is_scanning { s.stop_scan_button } else { s.scan_button });

        let scan_ring = if self.is_scanning {
            ProgressRing::new().is_active(true)
        } else {
            ProgressRing::new().is_active(false)
        };

        let address_box = TextBox::new()
            .text(&self.address_input)
            .placeholder_text(s.address_placeholder)
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
            s.conn_card_title,
            s.conn_card_desc,
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
                        s.active_prefix.replace("{}", s.mode_air_mouse)
                    } else {
                        s.mode_air_mouse.to_string()
                    }),
                Button::new()
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Touchpad)))
                    .content(if self.current_mode == ControlMode::Touchpad {
                        s.active_prefix.replace("{}", s.mode_trackpad)
                    } else {
                        s.mode_trackpad.to_string()
                    }),
                Button::new()
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Presentation)))
                    .content(if self.current_mode == ControlMode::Presentation {
                        s.active_prefix.replace("{}", s.mode_presenter)
                    } else {
                        s.mode_presenter.to_string()
                    }),
            ));

        let mode_card = Self::render_card(
            s.mode_card_title,
            s.mode_card_desc,
            mode_buttons.into(),
        );

        // Card 3: Real-Time Input Telemetry Monitor
        let (tp_text, btn_text, sample_text) = if let Some(data) = &self.latest_data {
            (
                format!(
                    "Touchpad: Normalized (X: {:+.3}, Y: {:+.3}) | Raw: ({}, {})",
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
                format!("Timestamp: {} ms | Status: Normal", data.timestamp),
            )
        } else {
            (
                s.telemetry_awaiting.to_string(),
                s.buttons_idle.to_string(),
                s.timestamp_no_tx.to_string(),
            )
        };

        let telemetry_card = Self::render_card(
            s.telemetry_card_title,
            s.telemetry_card_desc,
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

    fn render_calibration_tab(&self, s: &I18nStrings) -> View {
        let touch_card = Self::render_card(
            s.touch_cal_title,
            s.touch_cal_desc,
            StackPanel::new()
                .spacing(8.0)
                .children((
                    ProgressBar::new().value(100.0),
                    TextBlock::new()
                        .text(s.touch_cal_status)
                        .font_size(12.0),
                ))
                .into(),
        );

        let imu_card = Self::render_card(
            s.imu_cal_title,
            s.imu_cal_desc,
            StackPanel::new()
                .spacing(8.0)
                .children((
                    TextBlock::new()
                        .text(s.imu_cal_status)
                        .font_size(13.0),
                    TextBlock::new()
                        .text(s.imu_cal_filter)
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

    fn render_settings_tab(&self, context: &mut ViewContext<Self>, s: &I18nStrings) -> View {
        // Language Selection Card (4 Languages + Auto)
        let resolved_lang = self.language.resolve();
        let lang_buttons = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::Auto)))
                    .content(if self.language == Language::Auto {
                        format!("[ Active: Auto ({}) ]", resolved_lang.display_name())
                    } else {
                        "Auto".to_string()
                    }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::SimplifiedChinese)))
                    .content(if self.language == Language::SimplifiedChinese {
                        "[ Active: 简体中文 ]"
                    } else {
                        "简体中文"
                    }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::English)))
                    .content(if self.language == Language::English {
                        "[ Active: English ]"
                    } else {
                        "English"
                    }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::Japanese)))
                    .content(if self.language == Language::Japanese {
                        "[ Active: 日本語 ]"
                    } else {
                        "日本語"
                    }),
                Button::new()
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::Korean)))
                    .content(if self.language == Language::Korean {
                        "[ Active: 한국어 ]"
                    } else {
                        "한국어"
                    }),
            ));

        let language_card = Self::render_card(
            s.language_card_title,
            s.language_card_desc,
            lang_buttons.into(),
        );

        let anti_sleep_row = Self::render_toggle_card(
            s.anti_sleep_title,
            s.anti_sleep_desc,
            ToggleSwitch::new()
                .is_on(self.enable_anti_sleep)
                .on_toggled(context.callback(ReactorMessage::ToggleAntiSleep)),
        );

        let auto_profile_row = Self::render_toggle_card(
            s.auto_profile_title,
            s.auto_profile_desc,
            ToggleSwitch::new()
                .is_on(self.enable_auto_profile)
                .on_toggled(context.callback(ReactorMessage::ToggleAutoProfile)),
        );

        let tray_row = Self::render_toggle_card(
            s.tray_title,
            s.tray_desc,
            ToggleSwitch::new()
                .is_on(self.enable_background_tray)
                .on_toggled(context.callback(ReactorMessage::ToggleBackgroundTray)),
        );

        StackPanel::new()
            .spacing(12.0)
            .children((
                language_card,
                anti_sleep_row,
                auto_profile_row,
                tray_row,
            ))
            .into()
    }

    fn render_diagnostics_tab(&self, context: &mut ViewContext<Self>, s: &I18nStrings) -> View {
        let (accel, gyro, mag) = if let Some(d) = &self.latest_data {
            (
                format!("Accelerometer (g):     X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.accel_x, d.accel_y, d.accel_z),
                format!("Gyroscope (rad/s):       X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.gyro_x, d.gyro_y, d.gyro_z),
                format!("Magnetometer (uT):       X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.mag_x, d.mag_y, d.mag_z),
            )
        } else {
            (
                "Accelerometer (g):     Awaiting transmission...".to_string(),
                "Gyroscope (rad/s):       Awaiting transmission...".to_string(),
                "Magnetometer (uT):       Awaiting transmission...".to_string(),
            )
        };

        let imu_card = Self::render_card(
            s.imu_diag_title,
            s.imu_diag_desc,
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
            s.bt_recovery_title,
            s.bt_recovery_desc,
            StackPanel::new()
                .spacing(8.0)
                .children((
                    TextBlock::new()
                        .text(s.bt_troubleshoot_hint)
                        .font_size(12.0),
                    Button::new()
                        .on_click(context.message(ReactorMessage::OpenBtSettings))
                        .content(s.open_bt_settings),
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
