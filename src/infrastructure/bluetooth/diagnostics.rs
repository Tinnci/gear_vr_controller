//! Best-effort, read-only observations. Diagnostics never change radio or pairing state.
mod windows_services;
use std::{
    collections::hash_map::RandomState,
    future::Future,
    hash::BuildHasher,
    sync::OnceLock,
    time::{Duration, Instant},
};
use tracing::debug;
use windows::Devices::{
    Bluetooth::{BluetoothAdapter, BluetoothLEDevice, GenericAttributeProfile::GattSession},
    Radios::RadioState,
};

/// Correlate a device in this process without persisting its address or name.
pub fn device_key(address: u64) -> String {
    static HASHER: OnceLock<RandomState> = OnceLock::new();
    format!(
        "{:016x}",
        HASHER.get_or_init(RandomState::new).hash_one(address)
    )
}

pub(super) async fn operation<T>(
    name: &str,
    future: impl Future<Output = windows::core::Result<T>>,
) -> windows::core::Result<T> {
    if !tracing::enabled!(tracing::Level::DEBUG) {
        return future.await;
    }
    debug!(
        event = "ble.operation.started",
        operation = name,
        "Windows Bluetooth operation started"
    );
    let started = Instant::now();
    let result = future.await;
    debug!(
        event = "ble.operation.finished",
        operation = name,
        elapsed_ms = started.elapsed().as_millis() as u64,
        api_success = result.is_ok(),
        hresult = result
            .as_ref()
            .err()
            .map(|error| format!("0x{:08X}", error.code().0 as u32)),
        "Windows Bluetooth operation finished"
    );
    result
}

pub(super) fn property<T>(stage: &str, name: &str, result: windows::core::Result<T>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            debug!(event = "ble.diagnostic.property.failed", stage, property = name,
                hresult = %format!("0x{:08X}", error.code().0 as u32),
                "Cannot read diagnostic property");
            None
        }
    }
}

/// One bounded observation per requested stage; disabled DEBUG performs no Windows calls.
pub async fn adapter_snapshot(stage: &str) {
    if !tracing::enabled!(tracing::Level::DEBUG) {
        return;
    }
    windows_services::snapshot(stage);
    let result = tokio::time::timeout(Duration::from_secs(2), async {
        let adapter = BluetoothAdapter::GetDefaultAsync()?.await?;
        debug!(
            event = "ble.adapter.snapshot",
            stage,
            architecture = std::env::consts::ARCH,
            classic = property(stage, "classic_supported", adapter.IsClassicSupported()),
            low_energy = property(stage, "le_supported", adapter.IsLowEnergySupported()),
            central = property(stage, "central_supported", adapter.IsCentralRoleSupported()),
            peripheral = property(
                stage,
                "peripheral_supported",
                adapter.IsPeripheralRoleSupported()
            ),
            "Windows Bluetooth adapter observed"
        );
        let radio = adapter.GetRadioAsync()?.await?;
        let state = property(stage, "radio_state", radio.State());
        debug!(
            event = "ble.radio.snapshot",
            stage,
            state = state.map_or("Unavailable", radio_state),
            state_code = state.map(|state| state.0),
            "Windows Bluetooth radio observed"
        );
        windows::core::Result::Ok(())
    })
    .await;
    match result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            property::<()>(stage, "adapter_or_radio", Err(error));
        }
        Err(_) => debug!(
            event = "ble.diagnostic.timeout",
            stage,
            timeout_ms = 2000,
            "Bluetooth diagnostic observation timed out"
        ),
    }
}

fn radio_state(state: RadioState) -> &'static str {
    match state {
        RadioState::On => "On",
        RadioState::Off => "Off",
        RadioState::Disabled => "Disabled",
        RadioState::Unknown => "Unknown",
        _ => "Unrecognized",
    }
}

pub(super) fn device_snapshot(device: &BluetoothLEDevice, stage: &str) {
    if !tracing::enabled!(tracing::Level::DEBUG) {
        return;
    }
    let information = property(stage, "device_information", device.DeviceInformation());
    let pairing = information
        .as_ref()
        .and_then(|info| property(stage, "pairing", info.Pairing()));
    let access = property(stage, "device_access", device.DeviceAccessInformation());
    debug!(
        event = "ble.device.snapshot",
        stage,
        connection_status =
            property(stage, "connection_status", device.ConnectionStatus()).map(|s| s.0),
        address_type = property(stage, "address_type", device.BluetoothAddressType()).map(|s| s.0),
        enabled =
            information
                .as_ref()
                .and_then(|info| property(stage, "enabled", info.IsEnabled())),
        paired = pairing
            .as_ref()
            .and_then(|p| property(stage, "paired", p.IsPaired())),
        can_pair = pairing
            .as_ref()
            .and_then(|p| property(stage, "can_pair", p.CanPair())),
        protection_level = pairing
            .as_ref()
            .and_then(|p| property(stage, "pairing_protection", p.ProtectionLevel()))
            .map(|s| s.0),
        access_status = access
            .as_ref()
            .and_then(|a| property(stage, "access_status", a.CurrentStatus()))
            .map(|s| s.0),
        "Windows Bluetooth device observed"
    );
}

pub(super) fn session_snapshot(session: &GattSession) {
    if !tracing::enabled!(tracing::Level::DEBUG) {
        return;
    }
    debug!(
        event = "ble.session.snapshot",
        status = property("session", "session_status", session.SessionStatus()).map(|s| s.0),
        maintain_connection = property(
            "session",
            "maintain_connection",
            session.MaintainConnection()
        ),
        max_pdu_size = property("session", "max_pdu_size", session.MaxPduSize()),
        "Windows GATT session observed"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_are_stable_within_process_and_hide_raw_addresses() {
        let address = 0x2C41A1001234;
        assert_eq!(device_key(address), device_key(address));
        assert_ne!(device_key(address), device_key(address + 1));
        assert_eq!(device_key(address).len(), 16);
        assert!(!device_key(address).contains("2C41A1001234"));
    }
    #[test]
    fn property_failure_does_not_fabricate_a_disabled_state() {
        assert_eq!(property("test", "enabled", Ok(true)), Some(true));
        assert_eq!(
            property::<bool>(
                "test",
                "enabled",
                Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    0x80070005_u32 as i32
                )))
            ),
            None
        );
    }

    #[test]
    fn disabled_diagnostics_need_neither_windows_nor_timers() -> anyhow::Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        tracing::subscriber::with_default(tracing::subscriber::NoSubscriber::default(), || {
            runtime.block_on(adapter_snapshot("disabled"));
        });
        Ok(())
    }

    #[test]
    fn operation_failure_logs_typed_outcome_and_hresult() -> anyhow::Result<()> {
        use std::{
            io::Write,
            sync::{Arc, Mutex},
        };
        #[derive(Clone, Default)]
        struct Buffer(Arc<Mutex<Vec<u8>>>);
        impl Write for Buffer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0
                    .lock()
                    .map_err(|_| std::io::Error::other("Buffer lock failed"))?
                    .extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let output = Buffer::default();
        let writer = output.clone();
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(move || writer.clone())
            .finish();
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        let result = tracing::subscriber::with_default(subscriber, || {
            runtime.block_on(operation::<()>("open_device", async {
                Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    0x80070005_u32 as i32,
                )))
            }))
        });
        assert!(result.is_err());
        let bytes = output
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("Buffer lock failed"))?
            .clone();
        let records: Vec<serde_json::Value> = String::from_utf8(bytes)?
            .lines()
            .map(serde_json::from_str)
            .collect::<Result<_, _>>()?;
        let finished = records
            .iter()
            .find(|record| record["fields"]["event"] == "ble.operation.finished")
            .ok_or_else(|| anyhow::anyhow!("Missing operation outcome"))?;
        assert_eq!(finished["fields"]["api_success"], false);
        assert_eq!(finished["fields"]["hresult"], "0x80070005");
        assert_eq!(finished["fields"]["operation"], "open_device");
        assert!(finished["fields"]["elapsed_ms"].is_u64());
        Ok(())
    }
}
