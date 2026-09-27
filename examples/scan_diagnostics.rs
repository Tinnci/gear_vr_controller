//! Read-only scan probe for reproducing discovery issues without desktop interaction.
use gear_vr_controller_rust::{
    application::event_bus::EventSender,
    domain::{models::AppEvent, settings::LogSettings},
    infrastructure::{
        bluetooth::{diagnostics, scanner::BleScanner},
        logging,
    },
};
use std::time::{Duration, Instant};
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

struct Apartment;
impl Apartment {
    fn initialize() -> windows::core::Result<Self> {
        // SAFETY: the matching apartment guard stays on this same main thread.
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}
fn main() -> anyhow::Result<()> {
    let _apartment = Apartment::initialize()?;
    let _logging = logging::init_logger(&LogSettings {
        file_logging_enabled: false,
        console_logging_enabled: true,
        ansi_colors: false,
        ..Default::default()
    })?;
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?
        .block_on(diagnostics::adapter_snapshot("scan_probe"));
    let (sender, mut events) = EventSender::channel(16);
    let mut scanner = BleScanner::new(sender);
    scanner.start(None, true, vec![], None)?;
    let started = Instant::now();
    let mut snapshots = 0;
    let mut devices = vec![];
    while started.elapsed() < Duration::from_secs(15) {
        scanner.poll();
        while let Ok(event) = events.try_recv() {
            if let AppEvent::DevicesUpdated(snapshot) = event {
                snapshots += 1;
                devices = snapshot;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    scanner.stop()?;
    while let Ok(event) = events.try_recv() {
        if let AppEvent::DevicesUpdated(snapshot) = event {
            devices = snapshot;
        }
    }
    // Device names and addresses are deliberately absent from diagnostic output.
    println!(
        "{}",
        serde_json::json!({
            "snapshots": snapshots, "devices": devices.len(),
            "compatible": devices.iter().filter(|device| device.matches_service).count(),
            "unnamed": devices.iter().filter(|device| device.name.is_empty()).count(),
            "available": devices.iter().filter(|device| device.available).count(),
        })
    );
    Ok(())
}
