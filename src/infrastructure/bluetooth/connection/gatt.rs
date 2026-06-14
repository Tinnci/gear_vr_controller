use super::BleConnection;
use crate::infrastructure::bluetooth::protocol;
use anyhow::Result;
use tracing::{error, info};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattCommunicationStatus,
};
use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothLEDevice};

impl BleConnection {
    /// Get GATT characteristics required by the controller protocol.
    pub(super) async fn get_characteristics(
        &self,
        device: &BluetoothLEDevice,
    ) -> Result<(GattCharacteristic, GattCharacteristic)> {
        let service_uuid = protocol::parse_uuid(&self.config.service_uuid)?;
        let data_uuid = protocol::parse_uuid(&self.config.data_char_uuid)?;
        let cmd_uuid = protocol::parse_uuid(&self.config.command_char_uuid)?;

        let services_result = device
            .GetGattServicesForUuidWithCacheModeAsync(service_uuid, BluetoothCacheMode::Uncached)?
            .await?;

        if services_result.Status()? != GattCommunicationStatus::Success {
            error!(
                "Failed to get GATT services: {:?}",
                services_result.Status()?
            );
            anyhow::bail!("Failed to get GATT services");
        }

        let services = services_result.Services()?;
        if services.Size()? == 0 {
            anyhow::bail!("Controller service not found");
        }

        let service = services.GetAt(0)?;
        info!("Found controller service (cache refreshed)");

        info!("Requesting service access...");
        let access_status = service.RequestAccessAsync()?.await?;
        info!("Service access status: {:?}", access_status);

        let chars_result = service
            .GetCharacteristicsWithCacheModeAsync(BluetoothCacheMode::Uncached)?
            .await?;
        if chars_result.Status()? != GattCommunicationStatus::Success {
            anyhow::bail!("Failed to get characteristics");
        }

        let characteristics = chars_result.Characteristics()?;
        info!("Found {} characteristics", characteristics.Size()?);

        let mut data_char = None;
        let mut cmd_char = None;

        for i in 0..characteristics.Size()? {
            let characteristic = characteristics.GetAt(i)?;
            let uuid = characteristic.Uuid()?;

            if uuid == data_uuid {
                data_char = Some(characteristic);
                info!("Found data characteristic");
            } else if uuid == cmd_uuid {
                cmd_char = Some(characteristic.clone());
                info!("Found command characteristic");
            }
        }

        let data = data_char.ok_or_else(|| anyhow::anyhow!("Data characteristic not found"))?;
        let cmd = cmd_char.ok_or_else(|| anyhow::anyhow!("Command characteristic not found"))?;

        Ok((data, cmd))
    }
}
