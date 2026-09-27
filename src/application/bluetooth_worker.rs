//! Owns BLE, input injection and power state on one cancellable worker thread.
use crate::{
    application::{context_switcher::ContextProfileService, event_bus::EventSender},
    domain::{
        input::{InputAction, InputMapper},
        models::{AppEvent, BluetoothCommand, ConnectionStatus, MessageSeverity, StatusMessage},
        settings::SettingsService,
    },
    infrastructure::{
        bluetooth::BluetoothService,
        input_simulator::InputSimulator,
        power::{PowerInhibitor, WindowsPowerManager},
        window_tracker::WindowsForegroundWatcher,
    },
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

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
        // SAFETY: initialize and uninitialize this worker's WinRT apartment on the same thread.
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
            Err(error) => tracing::error!(%error, "Cannot start Bluetooth runtime"),
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
    tracing::info!(%message, ?severity);
    let _ = ui.send(AppEvent::LogMessage(StatusMessage { message, severity }));
}
fn inject(mapper: &mut InputMapper, actions: Vec<InputAction>, ui: &EventSender) {
    let simulator = InputSimulator::new();
    for action in actions {
        if let Err(error) = simulator.execute(action) {
            for release in mapper.reset() {
                let _ = simulator.execute(release);
            }
            report(
                ui,
                format!("Input injection failed: {error}"),
                MessageSeverity::Error,
            );
            break;
        }
    }
}
async fn run(
    ui: EventSender,
    mut commands: mpsc::Receiver<BluetoothCommand>,
    settings: Arc<Mutex<SettingsService>>,
    shutdown: Arc<AtomicBool>,
) {
    let (transport, mut events) = EventSender::channel(256);
    let mut service = BluetoothService::new(transport.clone(), settings.clone());
    let mut mapper = InputMapper::default();
    let mut power = WindowsPowerManager::new();
    let mut profiles = ContextProfileService::new(WindowsForegroundWatcher::default());
    let mut connected = false;
    let mut latest = None;
    let mut last_packet = Instant::now();
    let mut pending_command = None;
    let mut tick = tokio::time::interval(Duration::from_millis(33));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let _ = ui.send(AppEvent::WorkerReady);
    loop {
        if let Some(command) = pending_command.take() {
            connected = false;
            latest = None;
            while events.try_recv().is_ok() {}
            pending_command = handle_command(
                command,
                &mut service,
                &mut mapper,
                &ui,
                &settings,
                &shutdown,
                &mut commands,
            )
            .await;
            continue;
        }
        tokio::select! {
            biased;
            _ = cancelled(&shutdown) => break,
            _ = tick.tick() => {
                if transport.take_overflow() || ui.take_overflow() || (connected && last_packet.elapsed() > Duration::from_secs(2)) {
                    let actions = mapper.reset(); inject(&mut mapper, actions, &ui);
                    service.disconnect(); connected = false; latest = None;
                    report(&ui, "Input stream interrupted; disconnected to release held buttons", MessageSeverity::Error);
                }
                let snapshot = settings.lock().ok().map(|s| s.get().clone());
                if let Some(snapshot) = snapshot {
                    if let Some(mode) = profiles.evaluate_context(snapshot.enable_auto_profile_switching) {
                        let actions = mapper.set_mode(mode); inject(&mut mapper, actions, &ui);
                        let _ = ui.send(AppEvent::ModeChanged(mode));
                    }
                    let prevent = connected && mapper.mode == crate::domain::models::ControlMode::Presentation && snapshot.enable_presentation_anti_sleep;
                    let result = if prevent { power.prevent_sleep() } else { power.allow_sleep() };
                    if let Err(error) = result { report(&ui, error.to_string(), MessageSeverity::Error); }
                }
                if let Some(data) = latest.take() { let _ = ui.send(AppEvent::ControllerData(data)); }
                let _ = ui.send(AppEvent::CalibrationProgress(mapper.imu_progress()));
            }
            command = commands.recv() => {
                let Some(command) = command else { break; };
                if matches!(command, BluetoothCommand::Disconnect | BluetoothCommand::Connect(_)) {
                    connected = false; latest = None;
                    while events.try_recv().is_ok() {}
                }
                pending_command = handle_command(command, &mut service, &mut mapper, &ui, &settings, &shutdown, &mut commands).await;
            }
            event = events.recv() => {
                let Some(event) = event else { break; };
                match event {
                    AppEvent::ControllerData(mut data) if connected => {
                        last_packet = Instant::now();
                        let snapshot = settings.lock().ok().map(|s| s.get().clone());
                        if let Some(snapshot) = snapshot {
                            let mode = mapper.mode;
                            let actions = mapper.process(&mut data, &snapshot, last_packet);
                            inject(&mut mapper, actions, &ui);
                            if mode != mapper.mode { let _ = ui.send(AppEvent::ModeChanged(mapper.mode)); }
                            latest = Some(data);
                        }
                    }
                    AppEvent::ControllerData(_) => {}
                    AppEvent::ConnectionStatus(status) => {
                        connected = status == ConnectionStatus::Connected;
                        last_packet = Instant::now();
                        if !connected { let actions = mapper.reset(); inject(&mut mapper, actions, &ui); latest = None; }
                        let _ = ui.send(AppEvent::ConnectionStatus(status));
                    }
                    event => { let _ = ui.send(event); }
                }
            }
        }
    }
    let actions = mapper.reset();
    inject(&mut mapper, actions, &ui);
    service.disconnect();
    let _ = service.stop_scan();
    let _ = power.allow_sleep();
}
async fn handle_command(
    command: BluetoothCommand,
    service: &mut BluetoothService,
    mapper: &mut InputMapper,
    ui: &EventSender,
    settings: &Arc<Mutex<SettingsService>>,
    shutdown: &AtomicBool,
    commands: &mut mpsc::Receiver<BluetoothCommand>,
) -> Option<BluetoothCommand> {
    let connecting = matches!(command, BluetoothCommand::Connect(_));
    let result = match command {
        BluetoothCommand::Connect(address) => {
            let actions = mapper.reset();
            inject(mapper, actions, ui);
            service.disconnect();
            let _ = ui.send(AppEvent::ConnectionStatus(ConnectionStatus::Connecting));
            tokio::select! {
                _ = cancelled(shutdown) => return None,
                next = commands.recv() => {
                    report(ui, "Connection attempt cancelled", MessageSeverity::Info);
                    let _ = ui.send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
                    return next;
                }
                result = tokio::time::timeout(Duration::from_secs(30), service.connect(address)) => result.unwrap_or_else(|_| Err(anyhow::anyhow!("Bluetooth connection timed out"))),
            }
        }
        BluetoothCommand::Disconnect => {
            let actions = mapper.reset();
            inject(mapper, actions, ui);
            service.disconnect();
            Ok(())
        }
        BluetoothCommand::StartScan => {
            let result = service.start_scan();
            let _ = ui.send(AppEvent::ScanState(result.is_ok()));
            result
        }
        BluetoothCommand::StopScan => {
            let result = service.stop_scan();
            let _ = ui.send(AppEvent::ScanState(false));
            result
        }
        BluetoothCommand::ChangeMode(mode) => {
            let actions = mapper.set_mode(mode);
            inject(mapper, actions, ui);
            let _ = ui.send(AppEvent::ModeChanged(mode));
            Ok(())
        }
        BluetoothCommand::CalibrateImu => {
            let actions = mapper.start_imu_calibration();
            inject(mapper, actions, ui);
            Ok(())
        }
        BluetoothCommand::StartTouchCalibration => {
            let actions = mapper.start_touch_calibration();
            inject(mapper, actions, ui);
            Ok(())
        }
        BluetoothCommand::FinishTouchCalibration => {
            mapper.finish_touch_calibration().and_then(|calibration| {
                settings
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Settings lock poisoned"))?
                    .update_calibration(calibration)?;
                report(ui, "Touchpad calibration saved", MessageSeverity::Success);
                Ok(())
            })
        }
    };
    if let Err(error) = result {
        report(ui, error.to_string(), MessageSeverity::Error);
        if connecting {
            let _ = ui.send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
        }
    }
    None
}
