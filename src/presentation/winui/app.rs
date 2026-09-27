//! Native WinUI 3 Presentation Component using Windows Reactor
//!
//! Implements a modern Fluent Design 2 experience powered by the Windows App SDK
//! following official WinUI 3 guidelines (NavigationView, SettingsCard pattern,
//! 4-language i18n auto-detection, clean typography, zero emoji).

use crate::domain::i18n::Language;
use crate::domain::models::{
    AppEvent, BluetoothCommand, ConnectionStatus, ControllerData, ScannedDevice,
};
use crate::domain::settings::SettingsService;
use crate::presentation::radial_menu::ControlMode;
use crate::presentation::winui::components::title_bar::render_title_bar;
use crate::presentation::winui::tokens::FluentTokens;
use crate::presentation::winui::views::{
    render_calibration_view, render_dashboard_view, render_diagnostics_view, render_settings_view,
};
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
            ReactorMessage::ToggleAntiSleep(enable) => {
                self.enable_anti_sleep = enable;
                if let Ok(mut svc) = self.settings_service.lock() {
                    svc.get_mut().enable_presentation_anti_sleep = enable;
                    let _ = svc.save();
                }
            }
            ReactorMessage::ToggleAutoProfile(enable) => {
                self.enable_auto_profile = enable;
                if let Ok(mut svc) = self.settings_service.lock() {
                    svc.get_mut().enable_auto_profile_switching = enable;
                    let _ = svc.save();
                }
            }
            ReactorMessage::ToggleBackgroundTray(enable) => {
                self.enable_background_tray = enable;
                if let Ok(mut svc) = self.settings_service.lock() {
                    svc.get_mut().minimize_to_tray = enable;
                    let _ = svc.save();
                }
            }
            ReactorMessage::ChangeMode(mode) => {
                self.current_mode = mode;
            }
            ReactorMessage::OpenBtSettings => {
                let _ = std::process::Command::new("explorer.exe")
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

        // Tab Content Routing
        let tab_content: View = match self.selected_tab {
            0 => render_dashboard_view(self, context, s),
            1 => render_calibration_view(self, s),
            2 => render_settings_view(self, context, s),
            _ => render_diagnostics_view(self, context, s),
        };

        // Responsive scrolling content container
        let content_area = ScrollViewer::new()
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
            .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
            .content(
                Border::new()
                    .padding(FluentTokens::page_padding())
                    .content(
                        StackPanel::new()
                            .spacing(FluentTokens::SPACING_XXL)
                            .children((
                                status_infobar,
                                tab_content,
                            )),
                    ),
            );

        // Windows 11 Settings & Microsoft Store Custom TitleBar
        let title_bar = render_title_bar(self, context, s);

        let nav_view = NavigationView::new()
            .grid_row(1)
            .open_pane_length(FluentTokens::NAV_PANE_WIDTH)
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
}

/// Entrypoint to launch the WinUI 3 Native UI
pub fn run_reactor_app() -> anyhow::Result<()> {
    App::run_component::<GearVRReactorApp>(())?;
    Ok(())
}
