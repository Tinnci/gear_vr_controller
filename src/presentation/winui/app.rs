//! Native component. Domain state, page layout and platform services remain separate.
use super::{
    state::{parse_address, BindingSlot, BooleanPreference, NumericPreference, Page, UiState},
    text::Text,
    views::{
        render_calibration_view, render_dashboard_view, render_diagnostics_view,
        render_settings_view,
    },
};
use crate::{
    application::{
        bluetooth_worker::{spawn_bluetooth_worker, BluetoothWorker},
        event_bus::EventSender,
    },
    domain::{
        bindings::{ButtonAction, ModeBindings},
        i18n::Language,
        models::{
            AppEvent, BluetoothCommand, ConnectionStatus, ControlMode, MessageSeverity,
            OutputTarget,
        },
        preferences::InputPreferences,
        settings::SettingsService,
    },
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use windows_reactor::*;
static SMOKE_PASSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Debug, Clone)]
pub enum ReactorMessage {
    Navigate(Page),
    NavChanged(Option<String>),
    PaneChanged(bool),
    TogglePane,
    WindowWidth(f64),
    Language(Option<usize>),
    Address(String),
    ConnectAddress,
    ConnectDevice(u64),
    Disconnect,
    ToggleScan,
    Mode(Option<usize>),
    SetOutput(OutputTarget),
    Number(NumericPreference, f64),
    Boolean(BooleanPreference, bool),
    SavePreferences,
    DiscardPreferences,
    RestoreInput,
    BindingMode(Option<usize>),
    Binding(BindingSlot, Option<usize>),
    RestoreBindings,
    CalibrateGyro,
    CalibrateTouch,
    SaveCalibration,
    CancelCalibration,
    ConfirmRecovery,
    CancelRecovery,
    RecoverBluetooth,
    RecoveryFinished(Result<String, String>),
    OpenBluetooth,
    OpenLogs,
    ExportDiagnostics,
    Events(Vec<AppEvent>),
    DismissNotice,
    Noop,
}
pub struct GearVRReactorApp {
    pub ui: UiState,
    pub pane_open: bool,
    settings: Arc<Mutex<SettingsService>>,
    commands: mpsc::Sender<BluetoothCommand>,
    events: Arc<Mutex<mpsc::Receiver<AppEvent>>>,
    _worker: BluetoothWorker,
    tray: Option<crate::presentation::tray::WindowsTrayManager>,
    smoke_deadline: Option<Instant>,
}
impl Component for GearVRReactorApp {
    type Input = ();
    type Message = ReactorMessage;
    fn create(_: &(), context: &ComponentContext<Self>) -> Self {
        let smoke = std::env::args().any(|arg| arg == "--smoke-test");
        let preview = std::env::args().any(|arg| arg == "--preview-ui");
        let settings_result = if smoke || preview {
            Ok(SettingsService::in_memory_defaults())
        } else {
            SettingsService::new()
        };
        let (settings, error) = match settings_result {
            Ok(settings) => (settings, None),
            Err(error) => (
                SettingsService::in_memory_defaults(),
                Some(error.to_string()),
            ),
        };
        let mut ui = UiState::new(settings.get());
        if let Some(error) = error {
            ui.error(error);
        }
        let settings = Arc::new(Mutex::new(settings));
        let (commands, command_rx) = mpsc::channel(32);
        let (event_tx, event_rx) = EventSender::channel(128);
        let events = Arc::new(Mutex::new(event_rx));
        let worker = spawn_bluetooth_worker(event_tx, command_rx, settings.clone());
        Self::listen(context, events.clone());
        Self {
            ui,
            pane_open: true,
            settings,
            commands,
            events,
            _worker: worker,
            tray: None,
            smoke_deadline: smoke.then(|| Instant::now() + Duration::from_secs(2)),
        }
    }
    fn update(&mut self, message: Self::Message, context: &ComponentContext<Self>) {
        if let ReactorMessage::Events(events) = message {
            for event in events {
                self.ui.event(event);
            }
            self.poll_platform(context);
            Self::listen(context, self.events.clone());
            return;
        }
        self.handle_message(message, context);
    }
    fn view(&self, _: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title(format!(
            "{} · {}",
            self.text(Text::AppName),
            self.text(self.ui.page.title())
        ));
        context.window_visuals(
            WindowVisuals::new()
                .icon(crate::presentation::icon::icon_path())
                .client_size(1100.0, 760.0)
                .constraints(WindowConstraints {
                    min_width: Some(520.0),
                    min_height: Some(480.0),
                    ..Default::default()
                }),
        );
        context.on_window_size(
            context.callback(|size: WindowSize| ReactorMessage::WindowWidth(size.width)),
        );
        let content = match self.ui.page {
            Page::Control => render_dashboard_view(self, context),
            Page::Tuning => render_calibration_view(self, context),
            Page::Settings => render_settings_view(self, context),
            Page::Help => render_diagnostics_view(self, context),
        };
        let language = self.ui.language();
        let items = Page::ALL.map(|page| {
            KeyedView::new(
                page.index().to_string(),
                NavigationViewItem::new()
                    .tag(page.index().to_string())
                    .is_selected(self.ui.page == page)
                    .slots([
                        SlotView::new(
                            NavigationViewItemSlot::Icon,
                            SymbolIcon::new().symbol(match page {
                                Page::Control => Symbol::Home,
                                Page::Tuning => Symbol::Orientation,
                                Page::Settings => Symbol::Setting,
                                Page::Help => Symbol::Help,
                            }),
                        ),
                        SlotView::new(
                            NavigationViewItemSlot::Content,
                            TextBlock::new().text(
                                if page == Page::Help {
                                    Text::HelpNav
                                } else {
                                    page.title()
                                }
                                .get(language),
                            ),
                        ),
                    ]),
            )
        });
        let notice = InfoBar::new()
            .title(self.ui.page.title().get(language))
            .message(
                self.ui
                    .notice
                    .map(|text| text.get(language))
                    .unwrap_or_default(),
            )
            .severity(match self.ui.severity {
                MessageSeverity::Error => InfoBarSeverity::Error,
                MessageSeverity::Warning => InfoBarSeverity::Warning,
                MessageSeverity::Success => InfoBarSeverity::Success,
                MessageSeverity::Info => InfoBarSeverity::Informational,
            })
            .is_open(self.ui.notice.is_some())
            .is_closable(true)
            .on_closed(context.message(ReactorMessage::DismissNotice));
        let heading = TextBlock::new()
            .text(self.ui.page.title().get(language))
            .font_size(26.0)
            .font_weight(FontWeight::SEMI_BOLD)
            .automation_heading_level(AutomationHeadingLevel::Level1);
        let nav = NavigationView::new()
            .grid_row(1)
            .open_pane_length(200.0)
            .pane_display_mode(NavigationViewPaneDisplayMode::Auto)
            .is_pane_open(self.pane_open)
            .is_pane_toggle_button_visible(false)
            .is_settings_visible(false)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .on_is_pane_open_changed(context.callback(ReactorMessage::PaneChanged))
            .on_selected_tag_changed(context.callback(ReactorMessage::NavChanged))
            .slots([
                SlotView::collection(NavigationViewSlot::MenuItems, items),
                SlotView::new(
                    NavigationViewSlot::Content,
                    ScrollViewer::new()
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .content(
                            Border::new()
                                .padding(Thickness::new(28.0, 24.0, 28.0, 32.0))
                                .content(
                                    StackPanel::new()
                                        .max_width(880.0)
                                        .horizontal_alignment(HorizontalAlignment::Stretch)
                                        .spacing(20.0)
                                        .children((heading, notice, content)),
                                ),
                        ),
                ),
            ]);
        Grid::new()
            .rows([GridLength::Auto, GridLength::STAR])
            .children((
                super::components::title_bar::render_title_bar(self, context),
                nav,
            ))
    }
}
impl GearVRReactorApp {
    pub fn text(&self, text: Text) -> &'static str {
        text.get(self.ui.language())
    }
    fn send(&mut self, command: BluetoothCommand) -> bool {
        match self.commands.try_send(command) {
            Ok(()) => true,
            Err(error) => {
                self.ui.error(error.to_string());
                false
            }
        }
    }
    fn connect(&mut self, address: u64) {
        if self.ui.connection == ConnectionStatus::Connecting {
            return;
        }
        if self.send(BluetoothCommand::Connect(address)) {
            self.ui.connection = ConnectionStatus::Connecting;
            self.ui.notice = None;
            self.ui.address = format!("{address:012X}");
        }
    }
    fn output(&mut self, target: OutputTarget) {
        if !self.ui.can_output() {
            return;
        }
        if target == OutputTarget::Preview {
            self.send(BluetoothCommand::PreviewPreferences(
                self.ui.draft.input.clone(),
            ));
        }
        if self.send(BluetoothCommand::SetOutput(target)) {
            self.ui.output_pending = true;
        }
    }
    fn preview_preferences(&mut self) {
        if self.ui.output == OutputTarget::Preview {
            self.send(BluetoothCommand::PreviewPreferences(
                self.ui.draft.input.clone(),
            ));
        }
    }
    fn start_calibration(&mut self, gyro: bool) {
        if !self.ui.connected() || self.ui.calibration.is_collecting() {
            return;
        }
        let command = if gyro {
            BluetoothCommand::CalibrateImu
        } else {
            BluetoothCommand::StartTouchCalibration
        };
        if self.send(command) {
            self.ui.calibration = crate::domain::calibration::CalibrationStatus::Collecting {
                kind: if gyro {
                    crate::domain::calibration::CalibrationKind::Gyroscope
                } else {
                    crate::domain::calibration::CalibrationKind::Touchpad
                },
                progress: 0.0,
                ready: false,
            };
        }
    }
    fn save_preferences(&mut self) {
        let bindings_changed = self.ui.saved.bindings != self.ui.draft.bindings;
        let result = self
            .settings
            .lock()
            .map_err(|_| anyhow::anyhow!("Settings lock failed"))
            .and_then(|mut service| {
                let mut snapshot = service.get().clone();
                self.ui.draft.apply(&mut snapshot);
                service.replace(snapshot)
            });
        match result {
            Ok(()) => {
                self.ui.saved = self.ui.draft.clone();
                if bindings_changed {
                    self.output(OutputTarget::Paused);
                }
                self.ui.set_notice(Text::Saved, MessageSeverity::Success);
            }
            Err(error) => self.ui.error(error.to_string()),
        }
    }
    fn handle_message(&mut self, message: ReactorMessage, context: &ComponentContext<Self>) {
        match message {
            ReactorMessage::Navigate(page) => self.ui.page = page,
            ReactorMessage::NavChanged(Some(tag)) => {
                if let Some(page) = Page::from_tag(&tag) {
                    self.ui.page = page;
                }
            }
            ReactorMessage::PaneChanged(open) => self.pane_open = open,
            ReactorMessage::TogglePane => self.pane_open = !self.pane_open,
            ReactorMessage::WindowWidth(width) if width < 760.0 => self.pane_open = false,
            ReactorMessage::Address(value) => self.ui.address = value,
            ReactorMessage::ConnectAddress => match parse_address(&self.ui.address) {
                Some(address) => self.connect(address),
                None => self
                    .ui
                    .set_notice(Text::InvalidAddress, MessageSeverity::Error),
            },
            ReactorMessage::ConnectDevice(address) => self.connect(address),
            ReactorMessage::Disconnect => {
                self.send(BluetoothCommand::Disconnect);
            }
            ReactorMessage::ToggleScan => {
                if self.ui.scan_pending {
                    return;
                }
                let command = if self.ui.scanning {
                    BluetoothCommand::StopScan
                } else {
                    BluetoothCommand::StartScan
                };
                if self.send(command) {
                    self.ui.scan_pending = true;
                    self.ui.notice = None;
                }
            }
            ReactorMessage::Mode(Some(index)) => {
                if let Some(mode) = modes().get(index).filter(|mode| {
                    **mode != self.ui.mode
                        && self.ui.connection != ConnectionStatus::Connecting
                        && !self.ui.calibration.is_collecting()
                }) {
                    self.send(BluetoothCommand::ChangeMode(*mode));
                }
            }
            ReactorMessage::SetOutput(target) => self.output(target),
            ReactorMessage::Number(field, value) => {
                self.ui.set_number(field, value);
                self.preview_preferences();
            }
            ReactorMessage::Boolean(field, value) => {
                self.ui.set_bool(field, value);
                self.preview_preferences();
            }
            ReactorMessage::Language(Some(index)) => {
                if let Some(language) = Language::ALL.get(index) {
                    self.ui.draft.language = *language;
                }
            }
            ReactorMessage::SavePreferences => self.save_preferences(),
            ReactorMessage::DiscardPreferences => {
                self.ui.draft = self.ui.saved.clone();
                self.preview_preferences();
            }
            ReactorMessage::RestoreInput => {
                self.ui.draft.input = InputPreferences::from(&Default::default());
                self.preview_preferences();
            }
            ReactorMessage::BindingMode(Some(index)) => {
                if let Some(mode) = modes().get(index) {
                    self.ui.binding_mode = *mode;
                }
            }
            ReactorMessage::Binding(slot, Some(index)) => self.edit_binding(slot, index),
            ReactorMessage::RestoreBindings => {
                *self.ui.draft.bindings.for_mode_mut(self.ui.binding_mode) =
                    ModeBindings::defaults(self.ui.binding_mode)
            }
            ReactorMessage::CalibrateGyro => {
                self.start_calibration(true);
            }
            ReactorMessage::CalibrateTouch => {
                self.start_calibration(false);
            }
            ReactorMessage::SaveCalibration => {
                self.send(BluetoothCommand::FinishTouchCalibration);
            }
            ReactorMessage::CancelCalibration => {
                self.send(BluetoothCommand::CancelCalibration);
            }
            ReactorMessage::ConfirmRecovery => self.ui.recovery_confirm = true,
            ReactorMessage::CancelRecovery => self.ui.recovery_confirm = false,
            ReactorMessage::RecoverBluetooth => self.recover(context),
            ReactorMessage::RecoveryFinished(result) => {
                self.ui.recovery_running = false;
                match result {
                    Ok(_) => self
                        .ui
                        .set_notice(Text::RecoveryDone, MessageSeverity::Success),
                    Err(error) => self.ui.error(error),
                }
            }
            ReactorMessage::OpenBluetooth => self.open_path("ms-settings:bluetooth"),
            ReactorMessage::OpenLogs => {
                let path = self
                    .settings
                    .lock()
                    .ok()
                    .map(|service| service.get().log_settings.log_dir.clone());
                if let Some(path) = path {
                    self.open_path(&path);
                }
            }
            ReactorMessage::ExportDiagnostics => self.export_diagnostics(),
            ReactorMessage::DismissNotice => self.ui.notice = None,
            _ => {}
        }
    }
    fn edit_binding(&mut self, slot: BindingSlot, index: usize) {
        if let Some(action) = ButtonAction::ALL.get(index) {
            let binding = self.ui.draft.bindings.for_mode_mut(self.ui.binding_mode);
            *match slot {
                BindingSlot::Trigger => &mut binding.trigger,
                BindingSlot::Touchpad => &mut binding.touchpad,
                BindingSlot::Back => &mut binding.back,
                BindingSlot::Home => &mut binding.home,
            } = *action;
        }
    }
    fn recover(&mut self, context: &ComponentContext<Self>) {
        if self.ui.recovery_running || !self.ui.recovery_confirm {
            return;
        }
        self.ui.recovery_running = true;
        self.ui.recovery_confirm = false;
        self.send(BluetoothCommand::Disconnect);
        context.spawn_background(|cancel| {
            ReactorMessage::RecoveryFinished(
                crate::admin_client::recover_bluetooth(&cancel).map_err(|error| error.to_string()),
            )
        });
    }
    fn open_path(&mut self, path: &str) {
        if let Err(error) = std::process::Command::new("explorer.exe").arg(path).spawn() {
            self.ui.error(error.to_string());
        }
    }
    fn export_diagnostics(&mut self) {
        let result = super::diagnostics::export_summary(&self.ui);
        match result {
            Ok(path) => {
                self.ui.diagnostic_details = path.display().to_string();
                self.ui
                    .set_notice(Text::ExportDone, MessageSeverity::Success);
            }
            Err(error) => self.ui.error(error.to_string()),
        }
    }
    fn poll_platform(&mut self, context: &ComponentContext<Self>) {
        let tray_enabled = self.ui.saved.tray;
        if tray_enabled && self.tray.is_none() {
            match crate::presentation::tray::WindowsTrayManager::new(self.text(Text::AppName)) {
                Ok(tray) => self.tray = Some(tray),
                Err(error) => self.ui.error(error.to_string()),
            }
        }
        if let Some(tray) = &mut self.tray {
            tray.poll(tray_enabled);
            tray.set_control_state(
                self.ui.saved.language,
                self.ui.mode,
                self.ui.connection,
                self.ui.output,
            );
            if tray.take_pause_requested() {
                self.output(if self.ui.output != OutputTarget::Paused {
                    OutputTarget::Paused
                } else {
                    OutputTarget::Desktop
                });
            }
        }
        if self.tray.as_ref().is_some_and(|tray| tray.exit_requested()) {
            let _ = context.window().request_close();
        }
        if !tray_enabled {
            self.tray = None;
        }
        if self
            .smoke_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            SMOKE_PASSED.store(self.ui.worker_ready, std::sync::atomic::Ordering::Release);
            let _ = context.window().request_close();
        }
    }
    fn listen(context: &ComponentContext<Self>, events: Arc<Mutex<mpsc::Receiver<AppEvent>>>) {
        context.spawn_background(move |cancel| {
            for _ in 0..5 {
                if cancel.is_cancelled() {
                    return ReactorMessage::Noop;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            let mut batch = vec![];
            if let Ok(mut receiver) = events.lock() {
                while let Ok(event) = receiver.try_recv() {
                    batch.push(event);
                }
            }
            ReactorMessage::Events(batch)
        });
    }
}
pub fn modes() -> [ControlMode; 3] {
    [
        ControlMode::Mouse,
        ControlMode::Touchpad,
        ControlMode::Presentation,
    ]
}
pub fn run_reactor_app() -> anyhow::Result<()> {
    let smoke = std::env::args().any(|arg| arg == "--smoke-test");
    let preview = std::env::args().any(|arg| arg == "--preview-ui");
    let logs = if smoke || preview {
        crate::domain::settings::LogSettings {
            log_dir: std::env::temp_dir()
                .join("GearVRController-smoke/logs")
                .to_string_lossy()
                .into_owned(),
            ..Default::default()
        }
    } else {
        SettingsService::new()
            .map(|service| service.get().log_settings.clone())
            .unwrap_or_default()
    };
    let _logging = crate::infrastructure::logging::init_logger(&logs)?;
    App::run_component::<GearVRReactorApp>(())?;
    if smoke {
        anyhow::ensure!(
            SMOKE_PASSED.load(std::sync::atomic::Ordering::Acquire),
            "Native smoke test did not initialize its worker"
        );
    }
    Ok(())
}
