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
    TogglePane,
    PaneOpenChanged(bool),
    DismissStatusInfo,
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
    pub is_pane_open: bool,
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
            is_pane_open: true,
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
            ReactorMessage::TogglePane => {
                self.is_pane_open = !self.is_pane_open;
            }
            ReactorMessage::PaneOpenChanged(open) => {
                if self.is_pane_open != open {
                    self.is_pane_open = open;
                }
            }
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
            ReactorMessage::DismissStatusInfo => {
                self.status_message = None;
                if self.connection_status == ConnectionStatus::Error {
                    self.connection_status = ConnectionStatus::Disconnected;
                }
            }
            ReactorMessage::Noop => {}
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        let s = self.language.strings();

        // Fluent Header & Status Infobar (Windows 11 Contextual Status Feedback)
        let (info_title, info_msg, info_severity, is_info_open) = match (&self.connection_status, &self.status_message) {
            (ConnectionStatus::Connected, Some(msg)) => (s.status_connected, msg.as_str(), InfoBarSeverity::Success, true),
            (ConnectionStatus::Connected, None) => (s.status_connected, s.status_ready, InfoBarSeverity::Success, false),
            (ConnectionStatus::Connecting, _) => (s.status_connecting, s.status_negotiating, InfoBarSeverity::Informational, true),
            (ConnectionStatus::Disconnected, Some(msg)) => (s.status_disconnected, msg.as_str(), InfoBarSeverity::Warning, true),
            (ConnectionStatus::Disconnected, None) => (s.status_disconnected, s.status_no_link, InfoBarSeverity::Informational, false),
            (ConnectionStatus::Error, Some(msg)) => (s.status_error, msg.as_str(), InfoBarSeverity::Error, true),
            (ConnectionStatus::Error, None) => (s.status_error, s.status_error, InfoBarSeverity::Error, true),
        };

        let status_infobar = InfoBar::new()
            .title(info_title)
            .message(info_msg)
            .severity(info_severity)
            .is_open(is_info_open)
            .is_closable(true)
            .on_closed(context.message(ReactorMessage::DismissStatusInfo));

        // Fluent NavigationView Items (Windows 11 Navigation Architecture with native SymbolIcons)
        let nav_items = [
            KeyedView::new(
                "0",
                NavigationViewItem::new()
                    .tag("0")
                    .is_selected(self.selected_tab == 0)
                    .slots([
                        SlotView::new(
                            NavigationViewItemSlot::Icon,
                            SymbolIcon::new().symbol(Symbol::Home),
                        ),
                        SlotView::new(
                            NavigationViewItemSlot::Content,
                            TextBlock::new().text(s.nav_dashboard),
                        ),
                    ]),
            ),
            KeyedView::new(
                "1",
                NavigationViewItem::new()
                    .tag("1")
                    .is_selected(self.selected_tab == 1)
                    .slots([
                        SlotView::new(
                            NavigationViewItemSlot::Icon,
                            SymbolIcon::new().symbol(Symbol::Orientation),
                        ),
                        SlotView::new(
                            NavigationViewItemSlot::Content,
                            TextBlock::new().text(s.nav_calibration),
                        ),
                    ]),
            ),
            KeyedView::new(
                "2",
                NavigationViewItem::new()
                    .tag("2")
                    .is_selected(self.selected_tab == 2)
                    .slots([
                        SlotView::new(
                            NavigationViewItemSlot::Icon,
                            SymbolIcon::new().symbol(Symbol::Setting),
                        ),
                        SlotView::new(
                            NavigationViewItemSlot::Content,
                            TextBlock::new().text(s.nav_settings),
                        ),
                    ]),
            ),
            KeyedView::new(
                "3",
                NavigationViewItem::new()
                    .tag("3")
                    .is_selected(self.selected_tab == 3)
                    .slots([
                        SlotView::new(
                            NavigationViewItemSlot::Icon,
                            SymbolIcon::new().symbol(Symbol::View),
                        ),
                        SlotView::new(
                            NavigationViewItemSlot::Content,
                            TextBlock::new().text(s.nav_diagnostics),
                        ),
                    ]),
            ),
        ];

        // Tab Content
        let tab_content: View = match self.selected_tab {
            0 => self.render_dashboard_tab(context, s),
            1 => self.render_calibration_tab(s),
            2 => self.render_settings_tab(context, s),
            _ => self.render_diagnostics_tab(context, s),
        };

        // Responsive scrolling content container
        let content_area = ScrollViewer::new()
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
            .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
            .content(
                Border::new()
                    .padding(Thickness::new(24.0, 16.0, 24.0, 24.0))
                    .content(
                        StackPanel::new()
                            .spacing(16.0)
                            .children((
                                status_infobar,
                                tab_content,
                            )),
                    ),
            );

        // Dynamic subtitle reflecting current page and device state
        let dynamic_subtitle = match self.selected_tab {
            0 => match self.connection_status {
                ConnectionStatus::Connected => {
                    if let Some(data) = &self.latest_data {
                        format!("{} - {} ({} ms)", s.nav_dashboard, s.status_connected, data.timestamp)
                    } else {
                        format!("{} - {}", s.nav_dashboard, s.status_connected)
                    }
                }
                ConnectionStatus::Connecting => format!("{} - {}", s.nav_dashboard, s.status_connecting),
                ConnectionStatus::Disconnected => {
                    if self.is_scanning {
                        format!("{} - {}", s.nav_dashboard, s.scan_button)
                    } else {
                        format!("{} - {}", s.nav_dashboard, s.app_subtitle)
                    }
                }
                ConnectionStatus::Error => format!("{} - {}", s.nav_dashboard, s.status_error),
            },
            1 => format!("{} - {}", s.nav_calibration, s.touch_cal_status),
            2 => format!("{} - {}", s.nav_settings, self.language.display_name()),
            _ => format!("{} - {}", s.nav_diagnostics, s.imu_diag_title),
        };

        // TitleBar RightHeader: Status capsule badge & Quick actions
        let (status_badge_text, ring_active) = match self.connection_status {
            ConnectionStatus::Connected => (s.status_connected.to_string(), false),
            ConnectionStatus::Connecting => (s.status_connecting.to_string(), true),
            ConnectionStatus::Disconnected => {
                if self.is_scanning {
                    (s.scan_button.to_string(), true)
                } else {
                    (s.status_disconnected.to_string(), false)
                }
            }
            ConnectionStatus::Error => (s.status_error.to_string(), false),
        };

        let status_icon: View = if ring_active {
            ProgressRing::new()
                .is_active(true)
                .width(12.0)
                .height(12.0)
                .into()
        } else {
            InfoBadge::new().into()
        };

        let status_pill = Border::new()
            .background(ThemeBrush::CardBackground)
            .border_brush(ThemeBrush::CardStroke)
            .border_thickness(1.0)
            .corner_radius(12.0)
            .padding(Thickness::new(10.0, 4.0, 10.0, 4.0))
            .content(
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(6.0)
                    .children((
                        status_icon,
                        TextBlock::new()
                            .text(status_badge_text)
                            .font_size(12.0)
                            .font_weight(FontWeight::SEMI_BOLD),
                    )),
            );

        let quick_bt_btn = Button::new()
            .style(ButtonStyle::Subtle)
            .on_click(context.message(ReactorMessage::OpenBtSettings))
            .content(s.open_bt_settings);

        let title_bar_right = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                status_pill,
                quick_bt_btn,
            ));

        // Windows 11 Settings & Microsoft Store Custom TitleBar
        let title_bar = TitleBar::new()
            .grid_row(0)
            .title(s.app_title)
            .subtitle(dynamic_subtitle)
            .preferred_height(WindowTitleBarHeight::Tall)
            .is_back_button_visible(false)
            .is_pane_toggle_button_visible(true)
            .on_pane_toggle_requested(context.message(ReactorMessage::TogglePane))
            .slot(TitleBarSlot::RightHeader, title_bar_right);

        let nav_view = NavigationView::new()
            .grid_row(1)
            .open_pane_length(240.0)
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .is_pane_open(self.is_pane_open)
            .is_pane_toggle_button_visible(false)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .is_settings_visible(false)
            .on_is_pane_open_changed(context.callback(ReactorMessage::PaneOpenChanged))
            .on_selected_tag_changed(context.callback(ReactorMessage::NavSelectionChanged))
            .slots([
                SlotView::collection(NavigationViewSlot::MenuItems, nav_items),
                SlotView::new(NavigationViewSlot::Content, content_area),
            ]);

        Grid::new()
            .rows([GridLength::Auto, GridLength::STAR])
            .children((
                title_bar,
                nav_view,
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

    /// Fluent SettingsCard Pattern for Toggle Switches (Aligned right as per WinUI 3 Guidelines)
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
            .padding(Thickness::new(16.0, 14.0, 16.0, 14.0))
            .content(
                Grid::new()
                    .columns([GridLength::STAR, GridLength::Auto])
                    .children((
                        StackPanel::new()
                            .grid_column(0)
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
                        toggle.grid_column(1),
                    )),
            )
            .into()
    }

    fn format_telemetry(data: Option<&ControllerData>, s: &I18nStrings) -> (String, String, String) {
        if let Some(d) = data {
            let tp = format!(
                "Touchpad: Normalized (X: {:+.3}, Y: {:+.3}) | Raw: ({}, {})",
                d.processed_touchpad_x, d.processed_touchpad_y, d.touchpad_x, d.touchpad_y
            );
            let btn = format!(
                "Buttons: Trigger: {} | Back: {} | Home: {} | Touchpad: {} | Vol+: {} | Vol-: {}",
                if d.trigger_button { "Active" } else { "Inactive" },
                if d.back_button { "Active" } else { "Inactive" },
                if d.home_button { "Active" } else { "Inactive" },
                if d.touchpad_button { "Active" } else { "Inactive" },
                if d.volume_up_button { "Active" } else { "Inactive" },
                if d.volume_down_button { "Active" } else { "Inactive" },
            );
            let sample = format!("Timestamp: {} ms | Status: OK", d.timestamp);
            (tp, btn, sample)
        } else {
            (
                s.telemetry_awaiting.to_string(),
                s.buttons_idle.to_string(),
                s.timestamp_no_tx.to_string(),
            )
        }
    }

    fn format_imu_diagnostics(data: Option<&ControllerData>) -> (String, String, String) {
        if let Some(d) = data {
            (
                format!("Accelerometer (g):     X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.accel_x, d.accel_y, d.accel_z),
                format!("Gyroscope (rad/s):       X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.gyro_x, d.gyro_y, d.gyro_z),
                format!("Magnetometer (uT):       X: {:+.4} | Y: {:+.4} | Z: {:+.4}", d.mag_x, d.mag_y, d.mag_z),
            )
        } else {
            (
                "Accelerometer (g):     Waiting for data...".to_string(),
                "Gyroscope (rad/s):       Waiting for data...".to_string(),
                "Magnetometer (uT):       Waiting for data...".to_string(),
            )
        }
    }

    fn render_dashboard_tab(&self, context: &mut ViewContext<Self>, s: &I18nStrings) -> View {
        // Card 1: Connection & Bluetooth Scanning
        let connect_button = if self.connection_status == ConnectionStatus::Connected {
            Button::new()
                .style(ButtonStyle::Default)
                .on_click(context.message(ReactorMessage::Disconnect))
                .content(s.disconnect_button)
        } else {
            Button::new()
                .style(ButtonStyle::Accent)
                .on_click(context.message(ReactorMessage::Connect))
                .content(s.connect_button)
        };

        let scan_button = Button::new()
            .on_click(context.message(ReactorMessage::ToggleScan))
            .content(if self.is_scanning { s.stop_scan_button } else { s.scan_button });

        let scan_ring = if self.is_scanning {
            ProgressRing::new().is_active(true).width(18.0).height(18.0)
        } else {
            ProgressRing::new().is_active(false).width(18.0).height(18.0)
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
                    .style(if self.current_mode == ControlMode::Mouse {
                        ButtonStyle::Accent
                    } else {
                        ButtonStyle::Default
                    })
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Mouse)))
                    .content(s.mode_air_mouse),
                Button::new()
                    .style(if self.current_mode == ControlMode::Touchpad {
                        ButtonStyle::Accent
                    } else {
                        ButtonStyle::Default
                    })
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Touchpad)))
                    .content(s.mode_trackpad),
                Button::new()
                    .style(if self.current_mode == ControlMode::Presentation {
                        ButtonStyle::Accent
                    } else {
                        ButtonStyle::Default
                    })
                    .on_click(context.message(ReactorMessage::ChangeMode(ControlMode::Presentation)))
                    .content(s.mode_presenter),
            ));

        let mode_card = Self::render_card(
            s.mode_card_title,
            s.mode_card_desc,
            mode_buttons.into(),
        );

        // Card 3: Real-Time Input Telemetry Monitor
        let (tp_text, btn_text, sample_text) = Self::format_telemetry(self.latest_data.as_ref(), s);

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
        // Dynamic Touchpad Calibration: compute radial displacement [0.0, 100.0]
        let (touch_progress, touch_status_text) = if let Some(d) = &self.latest_data {
            let mag = (d.processed_touchpad_x.powi(2) + d.processed_touchpad_y.powi(2)).sqrt();
            let val = (mag.min(1.0) * 100.0) as f64;
            (
                val,
                format!(
                    "{} (X: {:+.2}, Y: {:+.2})",
                    s.touch_cal_status, d.processed_touchpad_x, d.processed_touchpad_y
                ),
            )
        } else {
            (100.0, s.touch_cal_status.to_string())
        };

        let touch_card = Self::render_card(
            s.touch_cal_title,
            s.touch_cal_desc,
            StackPanel::new()
                .spacing(8.0)
                .children((
                    ProgressBar::new().value(touch_progress),
                    TextBlock::new()
                        .text(touch_status_text)
                        .font_size(12.0),
                ))
                .into(),
        );

        // Gyroscope Calibration: indeterminate running bar indicates real-time drift cancellation
        let imu_progress_bar = if self.connection_status == ConnectionStatus::Connected {
            ProgressBar::new().is_indeterminate(true)
        } else {
            ProgressBar::new().is_indeterminate(false).value(0.0)
        };

        let imu_card = Self::render_card(
            s.imu_cal_title,
            s.imu_cal_desc,
            StackPanel::new()
                .spacing(8.0)
                .children((
                    imu_progress_bar,
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
        let auto_label = format!("Auto ({})", resolved_lang.display_name());
        let lang_buttons = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                Button::new()
                    .style(if self.language == Language::Auto {
                        ButtonStyle::Accent
                    } else {
                        ButtonStyle::Default
                    })
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::Auto)))
                    .content(auto_label),
                Button::new()
                    .style(if self.language == Language::SimplifiedChinese {
                        ButtonStyle::Accent
                    } else {
                        ButtonStyle::Default
                    })
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::SimplifiedChinese)))
                    .content("简体中文"),
                Button::new()
                    .style(if self.language == Language::English {
                        ButtonStyle::Accent
                    } else {
                        ButtonStyle::Default
                    })
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::English)))
                    .content("English"),
                Button::new()
                    .style(if self.language == Language::Japanese {
                        ButtonStyle::Accent
                    } else {
                        ButtonStyle::Default
                    })
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::Japanese)))
                    .content("日本語"),
                Button::new()
                    .style(if self.language == Language::Korean {
                        ButtonStyle::Accent
                    } else {
                        ButtonStyle::Default
                    })
                    .on_click(context.message(ReactorMessage::SelectLanguage(Language::Korean)))
                    .content("한국어"),
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
        let (accel, gyro, mag) = Self::format_imu_diagnostics(self.latest_data.as_ref());

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

        let bt_infobar = InfoBar::new()
            .is_open(true)
            .is_closable(false)
            .severity(InfoBarSeverity::Warning)
            .title(s.bt_recovery_title)
            .message(s.bt_troubleshoot_hint);

        let bt_recovery_card = Self::render_card(
            s.bt_recovery_title,
            s.bt_recovery_desc,
            StackPanel::new()
                .spacing(12.0)
                .children((
                    bt_infobar,
                    Button::new()
                        .style(ButtonStyle::Default)
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
