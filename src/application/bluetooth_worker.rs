//! Coordinates transport, input routing and recovery on one owned worker thread.
use crate::{
    application::{
        context_switcher::ContextProfileService, event_bus::EventSender, input_output::InputOutput,
        reconnect::ReconnectPolicy,
    },
    domain::{
        calibration::{CalibrationFailure, CalibrationStatus},
        input::InputMapper,
        models::{
            AppEvent, BluetoothCommand, ConnectionStatus, ControlMode, MessageSeverity,
            OutputTarget, StatusMessage,
        },
        settings::{Settings, SettingsService},
    },
    infrastructure::{
        bluetooth::BluetoothService,
        power::{PowerInhibitor, WindowsPowerManager},
        window_tracker::WindowsForegroundWatcher,
    },
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tracing::Instrument;

static CONNECTION_ATTEMPT: AtomicU64 = AtomicU64::new(1);

pub struct BluetoothWorker {
    shutdown: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for BluetoothWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub fn spawn_bluetooth_worker(
    ui: EventSender,
    commands: mpsc::Receiver<BluetoothCommand>,
    settings: Arc<Mutex<SettingsService>>,
) -> BluetoothWorker {
    let shutdown = Arc::new(AtomicBool::new(false));
    let worker_shutdown = shutdown.clone();
    let thread = std::thread::spawn(move || {
        use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
        // SAFETY: the apartment is initialized and released on this same worker thread.
        if let Err(error) = unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
            report(
                &ui,
                format!("Cannot initialize Windows Runtime: {error}"),
                MessageSeverity::Error,
            );
            return;
        }
        match tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
        {
            Ok(runtime) => runtime.block_on(run(ui, commands, settings, worker_shutdown)),
            Err(error) => report(&ui, error.to_string(), MessageSeverity::Error),
        }
        unsafe {
            RoUninitialize();
        }
    });
    BluetoothWorker {
        shutdown,
        thread: Some(thread),
    }
}
async fn cancelled(shutdown: &AtomicBool) {
    while !shutdown.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
fn report(ui: &EventSender, message: impl Into<String>, severity: MessageSeverity) {
    let message = message.into();
    match severity {
        MessageSeverity::Error => {
            tracing::error!(event = "worker.status", error = %message, "Worker operation failed")
        }
        MessageSeverity::Warning => {
            tracing::warn!(event = "worker.status", detail = %message, "Worker warning")
        }
        MessageSeverity::Info | MessageSeverity::Success => {
            tracing::info!(event = "worker.status", detail = %message, ?severity, "Worker status updated")
        }
    }
    let _ = ui.send(AppEvent::LogMessage(StatusMessage { message, severity }));
}

struct WorkerState {
    service: BluetoothService,
    mapper: InputMapper,
    output: InputOutput,
    reconnect: ReconnectPolicy,
    profiles: ContextProfileService<WindowsForegroundWatcher>,
    power: WindowsPowerManager,
    settings: Arc<Mutex<SettingsService>>,
    ui: EventSender,
    connected: bool,
    address: Option<u64>,
    last_packet: Instant,
    scan_deadline: Option<Instant>,
    calibration_deadline: Option<Instant>,
    last_power_error: Option<Instant>,
    published_calibration: CalibrationStatus,
    latest: Option<crate::domain::models::ControllerData>,
    manual_mode: bool,
    applied_bindings: crate::domain::bindings::ButtonBindings,
    preview_preferences: Option<crate::domain::preferences::InputPreferences>,
}
impl WorkerState {
    fn snapshot(&self) -> Settings {
        self.settings
            .lock()
            .map(|settings| settings.get().clone())
            .unwrap_or_default()
    }
    fn pause(&mut self, target: OutputTarget) {
        let previous = self.output.target;
        if let Err(error) = self.output.set_target(target, &mut self.mapper) {
            report(&self.ui, error.to_string(), MessageSeverity::Error);
        }
        let _ = self.ui.send(AppEvent::OutputChanged(self.output.target));
        if previous != self.output.target {
            tracing::info!(event = "output.changed", ?previous, target = ?self.output.target,
                "Input output changed");
        }
    }
    fn publish_calibration(&mut self) {
        let status = self.mapper.calibration_status();
        if status != &self.published_calibration {
            match status {
                CalibrationStatus::Failed { .. } => {
                    tracing::warn!(event = "calibration.changed", ?status, "Calibration failed")
                }
                CalibrationStatus::Collecting { .. } => tracing::trace!(
                    event = "calibration.progress",
                    ?status,
                    "Calibration progress updated"
                ),
                _ => tracing::info!(
                    event = "calibration.changed",
                    ?status,
                    "Calibration state changed"
                ),
            }
            self.published_calibration = status.clone();
            let _ = self.ui.send(AppEvent::CalibrationStatus(status.clone()));
        }
        if !status.is_collecting() {
            self.calibration_deadline = None;
        }
    }
    fn disconnected(&mut self) {
        self.mapper
            .fail_calibration(CalibrationFailure::Disconnected);
        self.publish_calibration();
        self.pause(OutputTarget::Paused);
        self.service.disconnect();
        self.connected = false;
        self.latest = None;
    }
    fn tick(&mut self, transport: &EventSender) -> Option<BluetoothCommand> {
        let now = Instant::now();
        let settings = self.snapshot();
        let transport_overflow = transport.take_overflow();
        let ui_overflow = self.ui.take_overflow();
        let stream_timeout = self.connected && self.last_packet.elapsed() > Duration::from_secs(2);
        if transport_overflow || ui_overflow || stream_timeout {
            tracing::error!(
                event = "input.stream.failed",
                transport_overflow,
                ui_overflow,
                stream_timeout,
                silence_ms = self.last_packet.elapsed().as_millis() as u64,
                "Input stream stopped; output paused"
            );
            self.disconnected();
            self.reconnect.lost(settings.auto_reconnect, now);
            let _ = self.ui.send(AppEvent::LogMessage(StatusMessage {
                message: "The input stream stopped. Input is paused. Connect the controller again."
                    .into(),
                severity: MessageSeverity::Error,
            }));
        }
        if self.scan_deadline.is_some_and(|deadline| now >= deadline) {
            let _ = self.service.stop_scan();
            self.scan_deadline = None;
            let _ = self.ui.send(AppEvent::ScanState(false));
        }
        if self
            .calibration_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.mapper.fail_calibration(CalibrationFailure::Timeout);
        }
        self.publish_calibration();
        if self.connected && !self.manual_mode && !self.mapper.calibration_status().is_collecting()
        {
            if let Some(mode) = self
                .profiles
                .evaluate_context(settings.enable_auto_profile_switching)
            {
                self.pause(self.output.target);
                self.mapper.set_mode(mode);
                let _ = self.ui.send(AppEvent::AutomaticModeChanged(mode));
            }
        } else if !settings.enable_auto_profile_switching {
            self.manual_mode = false;
        }
        let prevent = self.connected
            && self.mapper.mode == ControlMode::Presentation
            && self.output.target == OutputTarget::Desktop
            && settings.enable_presentation_anti_sleep;
        let result = if prevent {
            self.power.prevent_sleep()
        } else {
            self.power.allow_sleep()
        };
        match result {
            Err(error)
                if self
                    .last_power_error
                    .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(30)) =>
            {
                self.last_power_error = Some(now);
                report(&self.ui, format!("{error:#}"), MessageSeverity::Error);
            }
            Ok(()) => self.last_power_error = None,
            Err(_) => {}
        }
        if let Some(data) = self.latest.take() {
            let _ = self.ui.send(AppEvent::ControllerData(data));
        }
        if self.output.target == OutputTarget::Preview {
            let _ = self
                .ui
                .send(AppEvent::InputPreview(self.output.preview.clone()));
        }
        if !settings.auto_reconnect && !settings.auto_connect {
            self.reconnect.disable();
        }
        self.reconnect.take_due(now).map(|(address, attempt)| {
            tracing::info!(
                event = "connection.retry",
                attempt,
                "Connection retry scheduled"
            );
            let _ = self.ui.send(AppEvent::ReconnectAttempt(attempt));
            BluetoothCommand::Connect(address)
        })
    }
    fn event(&mut self, event: AppEvent) {
        match event {
            AppEvent::ControllerData(mut data) if self.connected => {
                self.last_packet = Instant::now();
                let mut settings = self.snapshot();
                if self.output.target == OutputTarget::Preview {
                    if let Some(preferences) = &self.preview_preferences {
                        preferences.apply(&mut settings);
                    }
                }
                if settings.button_bindings != self.applied_bindings {
                    self.pause(OutputTarget::Paused);
                    self.applied_bindings = settings.button_bindings.clone();
                }
                let mode = self.mapper.mode;
                let actions = self.mapper.process(&mut data, &settings, self.last_packet);
                if let Err(error) = self.output.dispatch(actions) {
                    self.pause(OutputTarget::Paused);
                    report(&self.ui, error.to_string(), MessageSeverity::Error);
                }
                if mode != self.mapper.mode {
                    self.manual_mode = true;
                    let _ = self.ui.send(AppEvent::ModeChanged(self.mapper.mode));
                }
                self.latest = Some(data);
            }
            AppEvent::ControllerData(_) => {}
            AppEvent::ConnectionStatus(status) => {
                tracing::info!(
                    event = "connection.changed",
                    ?status,
                    "Connection state changed"
                );
                let lost = self.connected && status != ConnectionStatus::Connected;
                self.connected = status == ConnectionStatus::Connected;
                self.last_packet = Instant::now();
                if lost {
                    self.disconnected();
                    self.reconnect
                        .lost(self.snapshot().auto_reconnect, Instant::now());
                }
                if self.connected {
                    if let Some(address) = self.address {
                        self.reconnect.connected(address);
                        let _ = self.ui.send(AppEvent::ConnectedDevice(address));
                    }
                }
                let _ = self.ui.send(AppEvent::ConnectionStatus(status));
            }
            event => {
                let _ = self.ui.send(event);
            }
        }
    }
    fn calibrate(&mut self, gyro: bool) {
        self.pause(OutputTarget::Paused);
        if gyro {
            self.mapper.start_imu_calibration();
        } else {
            self.mapper.start_touch_calibration();
        }
        self.calibration_deadline =
            Some(Instant::now() + Duration::from_secs(if gyro { 10 } else { 60 }));
        self.publish_calibration();
    }
    fn save_touch(&mut self) -> anyhow::Result<()> {
        let calibration = self.mapper.touch_calibration_result()?;
        let mut settings = self
            .settings
            .lock()
            .map_err(|_| anyhow::anyhow!("Settings lock failed"))?;
        let mut snapshot = settings.get().clone();
        snapshot.touchpad_calibration = calibration;
        settings.replace(snapshot)?;
        self.mapper.finish_touch_calibration()?;
        Ok(())
    }
    fn command(&mut self, command: BluetoothCommand) -> anyhow::Result<()> {
        match command {
            BluetoothCommand::Disconnect => {
                self.reconnect.disconnect();
                self.disconnected();
            }
            BluetoothCommand::StartScan => {
                self.service.start_scan()?;
                self.scan_deadline = Some(Instant::now() + Duration::from_secs(15));
                let _ = self.ui.send(AppEvent::ScanState(true));
            }
            BluetoothCommand::StopScan => {
                self.service.stop_scan()?;
                self.scan_deadline = None;
                let _ = self.ui.send(AppEvent::ScanState(false));
            }
            BluetoothCommand::ChangeMode(mode) => {
                tracing::info!(event = "mode.changed", previous = ?self.mapper.mode, ?mode,
                    source = "user", "Control mode changed");
                self.manual_mode = true;
                self.pause(self.output.target);
                self.mapper.set_mode(mode);
                let _ = self.ui.send(AppEvent::ModeChanged(mode));
            }
            BluetoothCommand::SetOutput(target) => {
                anyhow::ensure!(
                    !self.mapper.calibration_status().is_collecting(),
                    "Finish or cancel calibration first"
                );
                self.pause(target);
                if target != OutputTarget::Preview {
                    self.preview_preferences = None;
                }
            }
            BluetoothCommand::PreviewPreferences(preferences) => {
                let mut settings = self.snapshot();
                preferences.apply(&mut settings);
                settings.validate()?;
                self.preview_preferences = Some(preferences);
            }
            BluetoothCommand::CalibrateImu | BluetoothCommand::StartTouchCalibration => {
                anyhow::ensure!(self.connected, "Connect the controller first");
                self.calibrate(matches!(command, BluetoothCommand::CalibrateImu));
            }
            BluetoothCommand::FinishTouchCalibration => self.save_touch()?,
            BluetoothCommand::CancelCalibration => {
                self.mapper.cancel_calibration();
                self.calibration_deadline = None;
            }
            BluetoothCommand::Connect(_) => anyhow::bail!("Connect requires the asynchronous path"),
        }
        self.publish_calibration();
        Ok(())
    }
    async fn connect(
        &mut self,
        address: u64,
        shutdown: &AtomicBool,
        commands: &mut mpsc::Receiver<BluetoothCommand>,
    ) -> Option<BluetoothCommand> {
        self.disconnected();
        self.address = Some(address);
        let _ = self
            .ui
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Connecting));
        let attempt_id = CONNECTION_ATTEMPT.fetch_add(1, Ordering::Relaxed);
        let started = Instant::now();
        tracing::info!(
            event = "connection.started",
            attempt_id,
            timeout_ms = 30_000,
            "Connection attempt started"
        );
        let result = tokio::select! {
            _ = cancelled(shutdown) => {
                tracing::info!(event = "connection.cancelled", attempt_id, reason = "shutdown",
                    "Connection attempt cancelled");
                return None;
            },
            next = commands.recv() => {
                tracing::info!(event = "connection.cancelled", attempt_id, reason = "command",
                    "Connection attempt cancelled");
                self.reconnect.disconnect();
                let _ = self.ui.send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
                return next;
            }
            result = tokio::time::timeout(Duration::from_secs(30), self.service.connect(address)
                .instrument(tracing::info_span!("connection", attempt_id))) => {
                result.unwrap_or_else(|_| Err(anyhow::anyhow!("Connection timed out. Check Windows Bluetooth pairing.")))
            }
        };
        tracing::info!(
            event = "connection.finished",
            attempt_id,
            elapsed_ms = started.elapsed().as_millis() as u64,
            success = result.is_ok(),
            "Connection attempt finished"
        );
        if let Err(error) = result {
            self.reconnect
                .lost(self.snapshot().auto_reconnect, Instant::now());
            let _ = self
                .ui
                .send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
            let message = format!("{error:#}");
            tracing::error!(event = "connection.failed", attempt_id, error = %message,
                "Connection attempt failed");
            let _ = self.ui.send(AppEvent::LogMessage(StatusMessage {
                message,
                severity: MessageSeverity::Error,
            }));
        }
        None
    }
}
async fn run(
    ui: EventSender,
    mut commands: mpsc::Receiver<BluetoothCommand>,
    settings: Arc<Mutex<SettingsService>>,
    shutdown: Arc<AtomicBool>,
) {
    let (transport, mut events) = EventSender::channel(256);
    let service = BluetoothService::new(transport.clone(), settings.clone());
    let mut state = WorkerState {
        service,
        settings: settings.clone(),
        ui,
        mapper: InputMapper::default(),
        output: InputOutput::default(),
        reconnect: ReconnectPolicy::default(),
        profiles: ContextProfileService::new(WindowsForegroundWatcher::default()),
        power: WindowsPowerManager::new(),
        connected: false,
        address: None,
        last_packet: Instant::now(),
        scan_deadline: None,
        calibration_deadline: None,
        last_power_error: None,
        published_calibration: CalibrationStatus::Idle,
        latest: None,
        manual_mode: false,
        applied_bindings: settings
            .lock()
            .map(|service| service.get().button_bindings.clone())
            .unwrap_or_default(),
        preview_preferences: None,
    };
    state.pause(OutputTarget::Paused);
    let snapshot = state.snapshot();
    if snapshot.auto_connect {
        if let Some(address) = snapshot.last_connected_address {
            state.reconnect.start(address, Instant::now());
        }
    }
    let _ = state.ui.send(AppEvent::WorkerReady);
    let mut pending = None;
    let mut tick = tokio::time::interval(Duration::from_millis(33));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if let Some(command) = pending.take() {
            if let BluetoothCommand::Connect(address) = command {
                while events.try_recv().is_ok() {}
                pending = state.connect(address, &shutdown, &mut commands).await;
            } else if let Err(error) = state.command(command) {
                report(&state.ui, error.to_string(), MessageSeverity::Error);
            }
            continue;
        }
        tokio::select! {
            biased;
            _ = cancelled(&shutdown) => break,
            _ = tick.tick() => pending = state.tick(&transport),
            command = commands.recv() => {
                let Some(command) = command else { break; };
                // Manual connect restarts the retry budget.
                if let BluetoothCommand::Connect(address) = command { state.reconnect.start(address, Instant::now()); state.reconnect.disable(); }
                pending = Some(command);
            }
            event = events.recv() => {
                let Some(event) = event else { break; };
                state.event(event);
            }
        }
    }
    state.disconnected();
    let _ = state.service.stop_scan();
    let _ = state.power.allow_sleep();
}
