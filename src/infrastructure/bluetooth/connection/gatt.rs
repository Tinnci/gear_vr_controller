use super::BleConnection;
use crate::domain::connection_failure::{ConnectionFailure, ConnectionFailureKind};
use crate::infrastructure::bluetooth::protocol;
use anyhow::Result;
use tracing::{error, info};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattCommunicationStatus, GattDeviceService,
};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothLEDevice};
use windows::Devices::Enumeration::DeviceAccessStatus;

/// Keep the operation and native status in the error chain for diagnostics.
pub(super) fn check_status(stage: &str, status: GattCommunicationStatus) -> Result<()> {
    let (kind, name) = match status {
        GattCommunicationStatus::Success => return Ok(()),
        GattCommunicationStatus::Unreachable => (ConnectionFailureKind::Unreachable, "Unreachable"),
        GattCommunicationStatus::AccessDenied => {
            (ConnectionFailureKind::AccessDenied, "AccessDenied")
        }
        GattCommunicationStatus::ProtocolError => {
            (ConnectionFailureKind::Protocol, "ProtocolError")
        }
        _ => (ConnectionFailureKind::Other, "Unknown"),
    };
    Err(ConnectionFailure::new(kind, format!("{stage}: GATT {name} ({})", status.0)).into())
}

impl BleConnection {
    /// Get GATT characteristics required by the controller protocol.
    pub(super) async fn get_characteristics(
        &self,
        device: &BluetoothLEDevice,
    ) -> Result<(GattCharacteristic, GattCharacteristic, GattDeviceService)> {
        let service_uuid = protocol::parse_uuid(&self.config.service_uuid)?;
        let data_uuid = protocol::parse_uuid(&self.config.data_char_uuid)?;
        let cmd_uuid = protocol::parse_uuid(&self.config.command_char_uuid)?;

        let services_result = device
            .GetGattServicesForUuidWithCacheModeAsync(service_uuid, BluetoothCacheMode::Uncached)?
            .await?;

        let status = services_result.Status()?;
        if let Err(failure) = check_status("Read controller services", status) {
            error!(
                event = "ble.service.failed", status = status.0, error = %failure,
                "Cannot read GATT services"
            );
            return Err(failure);
        }

        let services = services_result.Services()?;
        if services.Size()? == 0 {
            return Err(ConnectionFailure::new(
                ConnectionFailureKind::Incompatible,
                "Configured controller service UUID not found",
            )
            .into());
        }

        let service = services.GetAt(0)?;
        info!(
            event = "ble.service.found",
            cache = "uncached",
            "Controller service found"
        );

        info!(event = "ble.access.started", "Requesting service access");
        let access_status = service.RequestAccessAsync()?.await?;
        info!(event = "ble.access.finished", status = ?access_status, "Service access request finished");
        if access_status != DeviceAccessStatus::Allowed {
            return Err(ConnectionFailure::new(
                ConnectionFailureKind::AccessDenied,
                format!("Controller service access not granted: {access_status:?}"),
            )
            .into());
        }

        let chars_result = service
            .GetCharacteristicsWithCacheModeAsync(BluetoothCacheMode::Uncached)?
            .await?;
        check_status("Read controller characteristics", chars_result.Status()?)?;

        let characteristics = chars_result.Characteristics()?;
        info!(
            event = "ble.characteristics.found",
            count = characteristics.Size()?,
            "Service characteristics found"
        );

        let mut data_char = None;
        let mut cmd_char = None;

        for i in 0..characteristics.Size()? {
            let characteristic = characteristics.GetAt(i)?;
            let uuid = characteristic.Uuid()?;

            if uuid == data_uuid {
                data_char = Some(characteristic);
                info!(
                    event = "ble.characteristic.found",
                    kind = "data",
                    "Controller characteristic found"
                );
            } else if uuid == cmd_uuid {
                cmd_char = Some(characteristic.clone());
                info!(
                    event = "ble.characteristic.found",
                    kind = "command",
                    "Controller characteristic found"
                );
            }
        }

        let data = data_char.ok_or_else(|| {
            ConnectionFailure::new(
                ConnectionFailureKind::Incompatible,
                "Data characteristic UUID not found",
            )
        })?;
        let cmd = cmd_char.ok_or_else(|| {
            ConnectionFailure::new(
                ConnectionFailureKind::Incompatible,
                "Command characteristic UUID not found",
            )
        })?;

        Ok((data, cmd, service))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_failures_keep_their_kind_and_stage() {
        assert!(check_status("Read services", GattCommunicationStatus::Success).is_ok());
        for (status, kind) in [
            (
                GattCommunicationStatus::Unreachable,
                ConnectionFailureKind::Unreachable,
            ),
            (
                GattCommunicationStatus::AccessDenied,
                ConnectionFailureKind::AccessDenied,
            ),
            (
                GattCommunicationStatus::ProtocolError,
                ConnectionFailureKind::Protocol,
            ),
            (GattCommunicationStatus(99), ConnectionFailureKind::Other),
        ] {
            let error = check_status("Read services", status)
                .err()
                .unwrap_or_else(|| unreachable!());
            assert_eq!(
                error.downcast_ref::<ConnectionFailure>().map(|e| e.kind),
                Some(kind)
            );
            assert!(error.to_string().contains("Read services"));
            assert!(error.to_string().contains(&format!("({})", status.0)));
        }
    }
}
