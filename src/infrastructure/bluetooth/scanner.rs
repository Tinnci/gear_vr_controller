//! Coalesce advertisement fragments before publishing bounded UI snapshots.
use crate::{
    application::event_bus::EventSender,
    domain::{
        discovery::{Advertisement, DiscoveryCatalog, PUBLISH_INTERVAL},
        models::{AppEvent, BluetoothAddressKind, ScannedDevice},
    },
};
use anyhow::Result;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use windows::{
    core::GUID,
    Devices::Bluetooth::{
        Advertisement::{
            BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
            BluetoothLEScanningMode,
        },
        BluetoothAddressType,
    },
    Foundation::TypedEventHandler,
};

#[derive(Default)]
struct ScanState {
    catalog: DiscoveryCatalog,
    target: Option<GUID>,
    generation: u64,
    active: bool,
    received: u64,
    errors: u64,
    dropped: u64,
    started: Option<Instant>,
    published: Option<Instant>,
    snapshot: Vec<ScannedDevice>,
}
impl ScanState {
    fn publish(&mut self, sender: &EventSender, now: Instant, force: bool) {
        if !force
            && self
                .published
                .is_some_and(|last| now.duration_since(last) < PUBLISH_INTERVAL)
        {
            return;
        }
        let snapshot = self.catalog.snapshot(now);
        if snapshot != self.snapshot || force {
            if sender
                .send(AppEvent::DevicesUpdated(snapshot.clone()))
                .is_ok()
            {
                self.snapshot = snapshot;
            } else {
                self.dropped += 1;
            }
        }
        self.published = Some(now);
    }
}
pub struct BleScanner {
    watcher: Option<BluetoothLEAdvertisementWatcher>,
    token: Option<i64>,
    event_sender: EventSender,
    state: Arc<Mutex<ScanState>>,
}
impl BleScanner {
    pub fn new(event_sender: EventSender) -> Self {
        Self {
            watcher: None,
            token: None,
            event_sender,
            state: Arc::default(),
        }
    }
    pub fn start(
        &mut self,
        service_uuid: Option<&str>,
        show_all: bool,
        known: Vec<u64>,
        last_used: Option<u64>,
    ) -> Result<()> {
        self.stop()?;
        let target =
            super::protocol::parse_uuid(service_uuid.unwrap_or(super::protocol::SERVICE_UUID))?;
        let generation = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| anyhow::anyhow!("Scan state lock failed"))?;
            if state.target != Some(target) {
                state.catalog = DiscoveryCatalog::default();
            }
            state.target = Some(target);
            state
                .catalog
                .begin(known, last_used, show_all, Instant::now());
            state.generation += 1;
            state.active = true;
            state.started = Some(Instant::now());
            state.received = 0;
            state.errors = 0;
            state.dropped = 0;
            state.published = None;
            state.generation
        };
        let watcher = BluetoothLEAdvertisementWatcher::new()?;
        watcher.SetScanningMode(BluetoothLEScanningMode::Active)?;
        let shared = self.state.clone();
        let sender = self.event_sender.clone();
        let handler = TypedEventHandler::new(
            move |_: windows::core::Ref<BluetoothLEAdvertisementWatcher>,
                  args: windows::core::Ref<BluetoothLEAdvertisementReceivedEventArgs>| {
                let Some(args) = args.as_ref() else {
                    return Ok(());
                };
                let observation = read_advertisement(args, target);
                if let Ok(mut state) = shared.lock() {
                    if !state.active || state.generation != generation {
                        return Ok(());
                    }
                    state.received += 1;
                    match observation {
                        Ok(Some(advertisement)) => {
                            state.catalog.observe(advertisement, Instant::now())
                        }
                        Ok(None) => {}
                        Err(error) => {
                            state.errors += 1;
                            if state.errors == 1 {
                                tracing::warn!(event = "ble.scan.read.failed", scan_id = generation,
                            error = %error, "Cannot read advertisement; repeated errors counted in scan summary");
                            }
                        }
                    }
                    state.publish(&sender, Instant::now(), false);
                }
                Ok(())
            },
        );
        let token = watcher.Received(&handler)?;
        if let Err(error) = watcher.Start() {
            let _ = watcher.RemoveReceived(token);
            if let Ok(mut state) = self.state.lock() {
                state.active = false;
            }
            return Err(error.into());
        }
        self.token = Some(token);
        self.watcher = Some(watcher);
        if let Ok(mut state) = self.state.lock() {
            state.publish(&self.event_sender, Instant::now(), true);
        }
        tracing::info!(
            event = "ble.scan.started",
            scan_id = generation,
            show_all,
            "Controller scan started"
        );
        Ok(())
    }
    pub fn stop(&mut self) -> Result<()> {
        let Some(watcher) = self.watcher.take() else {
            return Ok(());
        };
        if let Ok(mut state) = self.state.lock() {
            state.active = false;
            state.catalog.finish(Instant::now());
            state.publish(&self.event_sender, Instant::now(), true);
            let devices = &state.snapshot;
            tracing::info!(
                event = "ble.scan.finished",
                scan_id = state.generation,
                elapsed_ms = state
                    .started
                    .map_or(0, |time| time.elapsed().as_millis() as u64),
                advertisements = state.received,
                read_errors = state.errors,
                dropped_updates = state.dropped,
                devices = devices.len(),
                compatible = devices
                    .iter()
                    .filter(|device| device.matches_service)
                    .count(),
                unnamed = devices
                    .iter()
                    .filter(|device| device.name.is_empty())
                    .count(),
                "Controller scan finished"
            );
        }
        if let Some(token) = self.token.take() {
            let _ = watcher.RemoveReceived(token);
        }
        watcher.Stop()?;
        Ok(())
    }
    pub fn poll(&self) {
        if let Ok(mut state) = self.state.lock() {
            if state.active {
                state.publish(&self.event_sender, Instant::now(), false);
            }
        }
    }
    pub fn log_connection_target(&self, address: u64) {
        if !tracing::enabled!(tracing::Level::DEBUG) {
            return;
        }
        if let Ok(state) = self.state.lock() {
            let snapshot = state.catalog.snapshot(Instant::now());
            let matches: Vec<_> = snapshot
                .iter()
                .filter(|device| device.address == address)
                .collect();
            tracing::debug!(
                event = "ble.connection.scan_context",
                scan_id = state.generation,
                visible_matches = matches.len(),
                scanning = state.active,
                "Selected device compared with current scan results"
            );
            for device in matches {
                tracing::debug!(event = "ble.connection.target", scan_id = state.generation,
                    address_type = ?device.address_kind, available = device.available,
                    known = device.known, service_match = device.matches_service,
                    rssi_dbm = device.signal_strength, has_name = !device.name.is_empty(),
                    "Selected device scan observation");
            }
        }
    }
}
fn read_advertisement(
    args: &BluetoothLEAdvertisementReceivedEventArgs,
    target: GUID,
) -> Result<Option<Advertisement>> {
    let address = args.BluetoothAddress()?;
    if address == 0 {
        return Ok(None);
    }
    let address_kind = match args
        .BluetoothAddressType()
        .unwrap_or(BluetoothAddressType::Unspecified)
    {
        BluetoothAddressType::Public => BluetoothAddressKind::Public,
        BluetoothAddressType::Random => BluetoothAddressKind::Random,
        _ => BluetoothAddressKind::Unknown,
    };
    let advertisement = args.Advertisement()?;
    let services = advertisement.ServiceUuids()?;
    let mut matches_service = false;
    for index in 0..services.Size()? {
        matches_service |= services.GetAt(index)? == target;
    }
    Ok(Some(Advertisement {
        address,
        address_kind,
        name: advertisement.LocalName()?.to_string(),
        rssi: args.RawSignalStrengthInDBm()?,
        matches_service,
    }))
}
impl Drop for BleScanner {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_coalesce_changes_and_age_silent_devices() -> anyhow::Result<()> {
        let now = Instant::now();
        let mut state = ScanState::default();
        state.catalog.begin(vec![], None, false, now);
        let observation = |rssi| Advertisement {
            address: 1,
            address_kind: BluetoothAddressKind::Public,
            name: String::new(),
            rssi,
            matches_service: true,
        };
        let (sender, mut events) = EventSender::channel(4);
        state.catalog.observe(observation(-60), now);
        state.publish(&sender, now, false);
        assert!(events.try_recv().is_ok());
        state.catalog.observe(observation(-40), now);
        state.publish(&sender, now + PUBLISH_INTERVAL / 2, false);
        assert!(events.try_recv().is_err());
        state.publish(&sender, now + PUBLISH_INTERVAL, false);
        assert!(events.try_recv().is_ok());
        state.publish(&sender, now + std::time::Duration::from_secs(9), false);
        let AppEvent::DevicesUpdated(devices) = events.try_recv()? else {
            anyhow::bail!("Expected a discovery snapshot");
        };
        assert_eq!(devices.len(), 1);
        assert!(!devices[0].available);
        Ok(())
    }
}
