//! Native WinUI 3 Presentation Component using Windows Reactor
//!
//! Implements a modern Fluent Design 2 experience powered by the Windows App SDK
//! following official WinUI 3 guidelines (NavigationView, SettingsCard pattern,
//! 4-language i18n auto-detection, clean typography, zero emoji).

use crate::application::{bluetooth_worker::BluetoothWorker, event_bus::EventSender};
use crate::domain::i18n::{I18nStrings, Language};
use crate::domain::models::{
    AppEvent, BluetoothCommand, ConnectionStatus, ControlMode, ControllerData, MessageSeverity,
    ScannedDevice,
};
use crate::domain::settings::SettingsService;
use crate::presentation::winui::components::title_bar::render_title_bar;
use crate::presentation::winui::tokens::FluentTokens;
use crate::presentation::winui::views::{
    render_calibration_view, render_dashboard_view, render_diagnostics_view, render_settings_view,
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use windows_reactor::*;
static SMOKE_PASSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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
    FromAppEvents(Vec<AppEvent>),
    CalibrateImu,
    StartTouchCalibration,
    FinishTouchCalibration,
    RecoverBluetooth,
    RecoveryFinished(Result<String, String>),
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
    status_severity: MessageSeverity,
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
    pub bt_cmd_tx: Option<mpsc::Sender<BluetoothCommand>>,
    pub shared_event_rx: Arc<Mutex<mpsc::Receiver<AppEvent>>>,
    _worker: BluetoothWorker,
    pub imu_progress: Option<f32>,
    pub imu_completed: bool,
    pub recovery_running: bool,
    tray: Option<crate::presentation::tray::WindowsTrayManager>,
    smoke_deadline: Option<std::time::Instant>,
    worker_ready: bool,
}

impl Component for GearVRReactorApp {
    type Input = ();
    type Message = ReactorMessage;

    fn create(_input: &(), context: &ComponentContext<Self>) -> Self {
        let (bt_cmd_tx, bt_cmd_rx) = mpsc::channel::<BluetoothCommand>(32);
        let (event_tx, event_rx) = EventSender::channel(128);
        let shared_event_rx = Arc::new(Mutex::new(event_rx));

        let settings_result = if std::env::args().any(|arg| arg == "--smoke-test") {
            Ok(SettingsService::in_memory_defaults())
        } else {
            SettingsService::new()
        };
        let (settings_service, startup_error) = match settings_result {
            Ok(service) => (service, None),
            Err(error) => (SettingsService::in_memory_defaults(), Some(format!("Settings could not be loaded. Changes are temporary; original file is preserved: {error}"))),
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
        let worker = crate::application::bluetooth_worker::spawn_bluetooth_worker(
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
            status_message: startup_error,
            status_severity: MessageSeverity::Error,
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
            _worker: worker,
            imu_progress: None,
            imu_completed: false,
            recovery_running: false,
            tray: None,
            worker_ready: false,
            smoke_deadline: std::env::args()
                .any(|arg| arg == "--smoke-test")
                .then(|| std::time::Instant::now() + std::time::Duration::from_secs(2)),
        }
    }

    fn update(&mut self, message: Self::Message, context: &ComponentContext<Self>) {
        match message {
            ReactorMessage::SelectTab(tab) => {
                self.selected_tab = tab;
            }
            ReactorMessage::SelectLanguage(lang) => {
                self.language = lang;
                self.edit_settings(|settings| settings.language = lang);
            }
            ReactorMessage::UpdateAddressInput(input) => {
                self.address_input = input;
            }
            ReactorMessage::PickAddress(addr) => {
                self.address_input = format!("{:X}", addr);
            }
            ReactorMessage::Connect => {
                self.connect_address();
            }
            ReactorMessage::Disconnect => {
                self.send_command(BluetoothCommand::Disconnect);
            }
            ReactorMessage::ToggleScan => {
                if self.is_scanning {
                    self.send_command(BluetoothCommand::StopScan);
                } else {
                    self.scanned_devices.clear();
                    self.send_command(BluetoothCommand::StartScan);
                }
            }
            ReactorMessage::ToggleAntiSleep(enable) => {
                self.enable_anti_sleep = enable;
                self.edit_settings(|settings| settings.enable_presentation_anti_sleep = enable);
            }
            ReactorMessage::ToggleAutoProfile(enable) => {
                self.enable_auto_profile = enable;
                self.edit_settings(|settings| settings.enable_auto_profile_switching = enable);
            }
            ReactorMessage::ToggleBackgroundTray(enable) => {
                self.enable_background_tray = enable;
                self.edit_settings(|settings| settings.minimize_to_tray = enable);
            }
            ReactorMessage::ChangeMode(mode) => {
                self.send_command(BluetoothCommand::ChangeMode(mode));
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
                self.handle_app_event(event);
                Self::spawn_event_listener(context, self.shared_event_rx.clone());
            }
            ReactorMessage::FromAppEvents(events) => {
                for event in events {
                    self.handle_app_event(event);
                }
                self.sync_tray(context);
                self.check_smoke(context);
                Self::spawn_event_listener(context, self.shared_event_rx.clone());
            }
            ReactorMessage::CalibrateImu => {
                self.imu_completed = false;
                self.send_command(BluetoothCommand::CalibrateImu);
            }
            ReactorMessage::StartTouchCalibration => {
                self.send_command(BluetoothCommand::StartTouchCalibration)
            }
            ReactorMessage::FinishTouchCalibration => {
                self.send_command(BluetoothCommand::FinishTouchCalibration)
            }
            ReactorMessage::RecoverBluetooth => {
                if !self.recovery_running {
                    self.recovery_running = true;
                    self.send_command(BluetoothCommand::Disconnect);
                    context.spawn_background(|cancel| {
                        let result = crate::admin_client::recover_bluetooth(&cancel);
                        ReactorMessage::RecoveryFinished(result.map_err(|error| error.to_string()))
                    });
                }
            }
            ReactorMessage::RecoveryFinished(result) => {
                self.recovery_running = false;
                self.status_severity = if result.is_ok() {
                    MessageSeverity::Success
                } else {
                    MessageSeverity::Error
                };
                self.status_message = Some(
                    result.unwrap_or_else(|error| format!("Bluetooth recovery failed: {error}")),
                );
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
        let (info_title, info_msg, info_severity, is_info_open) =
            match (&self.connection_status, &self.status_message) {
                (ConnectionStatus::Connected, Some(msg)) => (
                    s.status_connected,
                    msg.as_str(),
                    InfoBarSeverity::Success,
                    true,
                ),
                (ConnectionStatus::Connected, None) => (
                    s.status_connected,
                    s.status_ready,
                    InfoBarSeverity::Success,
                    false,
                ),
                (ConnectionStatus::Connecting, _) => (
                    s.status_connecting,
                    s.status_negotiating,
                    InfoBarSeverity::Informational,
                    true,
                ),
                (ConnectionStatus::Disconnected, Some(msg)) => (
                    s.status_disconnected,
                    msg.as_str(),
                    InfoBarSeverity::Warning,
                    true,
                ),
                (ConnectionStatus::Disconnected, None) => (
                    s.status_disconnected,
                    s.status_no_link,
                    InfoBarSeverity::Informational,
                    false,
                ),
                (ConnectionStatus::Error, Some(msg)) => {
                    (s.status_error, msg.as_str(), InfoBarSeverity::Error, true)
                }
                (ConnectionStatus::Error, None) => {
                    (s.status_error, s.status_error, InfoBarSeverity::Error, true)
                }
            };

        let status_infobar = InfoBar::new()
            .title(info_title)
            .message(info_msg)
            .severity(if self.status_message.is_some() {
                match self.status_severity {
                    MessageSeverity::Error => InfoBarSeverity::Error,
                    MessageSeverity::Warning => InfoBarSeverity::Warning,
                    MessageSeverity::Success => InfoBarSeverity::Success,
                    MessageSeverity::Info => InfoBarSeverity::Informational,
                }
            } else {
                info_severity
            })
            .is_open(is_info_open)
            .is_closable(true)
            .on_closed(context.message(ReactorMessage::DismissStatusInfo));

        let nav_items = Self::create_nav_items(self.selected_tab, s);

        // Tab Content Routing
        let tab_content: View = match self.selected_tab {
            0 => render_dashboard_view(self, context, s),
            1 => render_calibration_view(self, context, s),
            2 => render_settings_view(self, context, s),
            _ => render_diagnostics_view(self, context, s),
        };

        // Responsive scrolling content container
        let content_area = ScrollViewer::new()
            .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
            .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
            .content(
                Border::new().padding(FluentTokens::page_padding()).content(
                    StackPanel::new()
                        .spacing(FluentTokens::SPACING_XXL)
                        .children((status_infobar, tab_content)),
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
            .children((title_bar, nav_view))
    }
}

impl GearVRReactorApp {
    fn check_smoke(&self, context: &ComponentContext<Self>) {
        if self
            .smoke_deadline
            .is_some_and(|deadline| std::time::Instant::now() >= deadline)
        {
            SMOKE_PASSED.store(self.worker_ready, std::sync::atomic::Ordering::Release);
            tracing::info!(ready = self.worker_ready, "Native smoke test closing");
            let _ = context.window().request_close();
        }
    }
    fn connect_address(&mut self) {
        let sanitized = self.address_input.replace([':', '-'], "");
        if let Some(address) = u64::from_str_radix(&sanitized, 16)
            .ok()
            .filter(|address| *address > 0 && *address <= 0xFFFF_FFFF_FFFF)
        {
            self.send_command(BluetoothCommand::Connect(address));
        } else {
            self.status_severity = MessageSeverity::Error;
            self.status_message = Some("Enter a valid 48-bit Bluetooth address".to_string());
        }
    }
    fn edit_settings(&mut self, update: impl FnOnce(&mut crate::domain::settings::Settings)) {
        let result = self
            .settings_service
            .lock()
            .map_err(|_| anyhow::anyhow!("Settings lock poisoned"))
            .and_then(|mut service| {
                update(service.get_mut());
                service.save()
            });
        if let Err(error) = result {
            self.status_severity = MessageSeverity::Error;
            self.status_message = Some(format!("Cannot save settings: {error}"));
        }
    }
    fn send_command(&mut self, command: BluetoothCommand) {
        if let Some(tx) = &self.bt_cmd_tx {
            if let Err(error) = tx.try_send(command) {
                self.status_severity = MessageSeverity::Error;
                self.status_message = Some(format!("Cannot send command: {error}"));
            }
        }
    }

    fn sync_tray(&mut self, context: &ComponentContext<Self>) {
        if self.enable_background_tray && self.tray.is_none() {
            match crate::presentation::tray::WindowsTrayManager::new("Gear VR Controller") {
                Ok(tray) => self.tray = Some(tray),
                Err(error) => {
                    self.status_severity = MessageSeverity::Error;
                    self.status_message = Some(format!("Cannot enable tray: {error}"));
                    self.enable_background_tray = false;
                }
            }
        }
        if let Some(tray) = &mut self.tray {
            tray.poll(self.enable_background_tray);
            if tray.exit_requested() {
                let _ = context.window().request_close();
            }
        }
        if !self.enable_background_tray {
            self.tray = None;
        }
    }
    fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::WorkerReady => self.worker_ready = true,
            AppEvent::ModeChanged(mode) => self.current_mode = mode,
            AppEvent::CalibrationProgress(progress) => {
                if self.imu_progress.is_some() && progress.is_none() {
                    self.imu_completed = true;
                }
                self.imu_progress = progress;
            }
            AppEvent::ScanState(scanning) => self.is_scanning = scanning,
            AppEvent::ControllerData(data) => {
                self.latest_data = Some(data);
            }
            AppEvent::ConnectionStatus(status) => {
                self.connection_status = status;
                if status != ConnectionStatus::Connected {
                    self.latest_data = None;
                }
                if let ConnectionStatus::Connected = status {
                    self.status_severity = MessageSeverity::Success;
                    let s = self.language.strings();
                    self.status_message = Some(s.status_ready.to_string());
                }
            }
            AppEvent::LogMessage(log) => {
                self.status_severity = log.severity;
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
    }

    fn create_nav_items(selected_tab: usize, s: &I18nStrings) -> [KeyedView; 4] {
        [
            KeyedView::new(
                "0",
                NavigationViewItem::new()
                    .tag("0")
                    .is_selected(selected_tab == 0)
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
                    .is_selected(selected_tab == 1)
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
                    .is_selected(selected_tab == 2)
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
                    .is_selected(selected_tab == 3)
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
        ]
    }

    fn spawn_event_listener(
        context: &ComponentContext<Self>,
        rx: Arc<Mutex<mpsc::Receiver<AppEvent>>>,
    ) {
        context.spawn_background(move |cancel| {
            for _ in 0..5 {
                if cancel.is_cancelled() {
                    return ReactorMessage::Noop;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            let mut events = Vec::new();
            if let Ok(mut receiver) = rx.lock() {
                while let Ok(event) = receiver.try_recv() {
                    events.push(event);
                }
            }
            ReactorMessage::FromAppEvents(events)
        });
    }
}

/// Entrypoint to launch the WinUI 3 Native UI
pub fn run_reactor_app() -> anyhow::Result<()> {
    let logs = if std::env::args().any(|arg| arg == "--smoke-test") {
        crate::domain::settings::LogSettings {
            log_dir: std::env::temp_dir()
                .join("GearVRController-smoke/logs")
                .to_string_lossy()
                .into_owned(),
            ..Default::default()
        }
    } else {
        SettingsService::new()
            .map(|svc| svc.get().log_settings.clone())
            .unwrap_or_default()
    };
    let _logging = crate::infrastructure::logging::init_logger(&logs)?;
    App::run_component::<GearVRReactorApp>(())?;
    if std::env::args().any(|arg| arg == "--smoke-test") {
        anyhow::ensure!(
            SMOKE_PASSED.load(std::sync::atomic::Ordering::Acquire),
            "Native smoke test did not initialize its worker"
        );
    }
    Ok(())
}
