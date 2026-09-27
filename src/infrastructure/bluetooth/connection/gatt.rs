use super::BleConnection;
use crate::domain::connection_failure::{ConnectionFailure, ConnectionFailureKind};
use crate::infrastructure::bluetooth::diagnostics::{device_snapshot, operation, property};
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

        let services_result = operation("read_services", async {
            device
                .GetGattServicesForUuidWithCacheModeAsync(
                    service_uuid,
                    BluetoothCacheMode::Uncached,
                )?
                .await
        })
        .await?;

        let status = services_result.Status()?;
        let protocol_error = services_result
            .ProtocolError()
            .ok()
            .and_then(|code| code.Value().ok());
        tracing::debug!(event = "ble.service.result", status = status.0, protocol_error,
            service_uuid = ?service_uuid, cache = "uncached", "Controller service discovery result");
        if let Err(failure) = check_status("Read controller services", status) {
            device_snapshot(device, "service_discovery_failed");
            error!(
                event = "ble.service.failed", status = status.0, protocol_error, error = %failure,
                "Cannot read GATT services"
            );
            return Err(failure);
        }

        let services = services_result.Services()?;
        tracing::debug!(
            event = "ble.service.count",
            count = services.Size()?,
            "Matching services observed"
        );
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
        let access_status = operation("request_service_access", async {
            service.RequestAccessAsync()?.await
        })
        .await?;
        info!(event = "ble.access.finished", status = ?access_status, "Service access request finished");
        if access_status != DeviceAccessStatus::Allowed {
            return Err(ConnectionFailure::new(
                ConnectionFailureKind::AccessDenied,
                format!("Controller service access not granted: {access_status:?}"),
            )
            .into());
        }

        let (data, cmd) = self.read_characteristics(&service).await?;
        Ok((data, cmd, service))
    }

    async fn read_characteristics(
        &self,
        service: &GattDeviceService,
    ) -> Result<(GattCharacteristic, GattCharacteristic)> {
        let data_uuid = protocol::parse_uuid(&self.config.data_char_uuid)?;
        let cmd_uuid = protocol::parse_uuid(&self.config.command_char_uuid)?;
        let chars_result = operation("read_characteristics", async {
            service
                .GetCharacteristicsWithCacheModeAsync(BluetoothCacheMode::Uncached)?
                .await
        })
        .await?;
        tracing::debug!(
            event = "ble.characteristics.result",
            status = chars_result.Status()?.0,
            protocol_error = chars_result
                .ProtocolError()
                .ok()
                .and_then(|code| code.Value().ok()),
            "Controller characteristic discovery result"
        );
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
            if tracing::enabled!(tracing::Level::DEBUG) {
                tracing::debug!(
                    event = "ble.characteristic.snapshot",
                    index = i,
                    ?uuid,
                    handle = property(
                        "characteristic",
                        "attribute_handle",
                        characteristic.AttributeHandle()
                    ),
                    properties = property(
                        "characteristic",
                        "properties",
                        characteristic.CharacteristicProperties()
                    )
                    .map(|p| p.0),
                    protection = property(
                        "characteristic",
                        "protection_level",
                        characteristic.ProtectionLevel()
                    )
                    .map(|p| p.0),
                    "GATT characteristic observed"
                );
            }

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

        Ok((data, cmd))
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
