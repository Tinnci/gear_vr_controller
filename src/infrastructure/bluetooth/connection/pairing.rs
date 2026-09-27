use super::BleConnection;
use crate::domain::models::MessageSeverity;
use anyhow::Result;
use tracing::info;
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::GattSession;

impl BleConnection {
    /// Connect to BLE device.
    pub(super) async fn connect_device(&self, address: u64) -> Result<BluetoothLEDevice> {
        let device = super::super::diagnostics::operation("open_device", async {
            BluetoothLEDevice::FromBluetoothAddressAsync(address)?.await
        })
        .await?;
        Ok(device)
    }

    /// Create a GattSession to maintain the BLE connection.
    pub(super) async fn create_gatt_session(
        &self,
        device: &BluetoothLEDevice,
    ) -> Result<GattSession> {
        let device_id = device.BluetoothDeviceId()?;
        let session = super::super::diagnostics::operation("create_session", async {
            GattSession::FromDeviceIdAsync(&device_id)?.await
        })
        .await?;
        session.SetMaintainConnection(true)?;
        Ok(session)
    }

    /// Handle device pairing.
    ///
    /// Gear VR controllers often work through direct GATT access without traditional pairing.
    pub(super) async fn handle_pairing(&self, device: &BluetoothLEDevice) -> Result<bool> {
        let device_info = device.DeviceInformation()?;
        let pairing = device_info.Pairing()?;
        let is_paired = pairing.IsPaired()?;

        info!(
            event = "ble.pairing.status",
            paired = is_paired,
            "Pairing status read"
        );

        if is_paired {
            info!(event = "ble.pairing.reused", "Existing pairing used");
            self.send_log("Device reports as paired", MessageSeverity::Info);
        } else {
            info!(
                event = "ble.pairing.direct",
                "Attempting direct GATT access"
            );
            self.send_log(
                "Connecting without traditional pairing...",
                MessageSeverity::Info,
            );
        }

        Ok(is_paired)
    }
}
