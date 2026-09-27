use super::BleConnection;
use crate::infrastructure::bluetooth::protocol;
use anyhow::Result;
use tracing::{error, info};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattCommunicationStatus, GattDeviceService,
};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothLEDevice};

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

        if services_result.Status()? != GattCommunicationStatus::Success {
            error!(
                event = "ble.service.failed", status = ?services_result.Status()?,
                "Cannot read GATT services"
            );
            anyhow::bail!("Failed to get GATT services");
        }

        let services = services_result.Services()?;
        if services.Size()? == 0 {
            anyhow::bail!("Controller service not found");
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

        let chars_result = service
            .GetCharacteristicsWithCacheModeAsync(BluetoothCacheMode::Uncached)?
            .await?;
        if chars_result.Status()? != GattCommunicationStatus::Success {
            anyhow::bail!("Failed to get characteristics");
        }

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

        let data = data_char.ok_or_else(|| anyhow::anyhow!("Data characteristic not found"))?;
        let cmd = cmd_char.ok_or_else(|| anyhow::anyhow!("Command characteristic not found"))?;

        Ok((data, cmd, service))
    }
}
