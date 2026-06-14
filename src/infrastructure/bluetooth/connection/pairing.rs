use super::BleConnection;
use crate::domain::models::MessageSeverity;
use anyhow::Result;
use tracing::{error, info, warn};
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::GattSession;
use windows::Devices::Enumeration::{DeviceInformation, DeviceUnpairingResultStatus};

impl BleConnection {
    /// Connect to BLE device.
    pub(super) async fn connect_device(&self, address: u64) -> Result<BluetoothLEDevice> {
        let device_async = BluetoothLEDevice::FromBluetoothAddressAsync(address)?;
        let device = device_async.await?;
        Ok(device)
    }

    /// Create a GattSession to maintain the BLE connection.
    pub(super) async fn create_gatt_session(
        &self,
        device: &BluetoothLEDevice,
    ) -> Result<GattSession> {
        let device_id = device.BluetoothDeviceId()?;
        let session = GattSession::FromDeviceIdAsync(&device_id)?.await?;
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

        info!("Device reports pairing status - IsPaired: {}", is_paired);

        if is_paired {
            info!("Device already paired according to handle");
            self.send_log("Device reports as paired", MessageSeverity::Info);
        } else {
            info!("BLE device not paired - will attempt direct GATT access");
            self.send_log(
                "Connecting without traditional pairing...",
                MessageSeverity::Info,
            );
        }

        Ok(is_paired)
    }

    /// Check system-wide paired status through the Windows PnP database.
    pub(super) async fn check_system_paired_status(
        &self,
        target_address: u64,
    ) -> Result<Option<DeviceInformation>> {
        let aqs_filter = BluetoothLEDevice::GetDeviceSelectorFromPairingState(true)?;
        let devices = DeviceInformation::FindAllAsyncAqsFilter(&aqs_filter)?.await?;

        for device_info in devices {
            if let Ok(le_device) = BluetoothLEDevice::FromIdAsync(&device_info.Id()?)?.await {
                if le_device.BluetoothAddress()? == target_address {
                    return Ok(Some(device_info));
                }
            }
        }

        Ok(None)
    }

    /// Clear stale pairing metadata when Windows has a system record but the active handle is not paired.
    pub(super) async fn clear_stale_pairing_record(
        &self,
        system_device_info: Option<DeviceInformation>,
    ) -> Result<()> {
        let ghost_msg = "检测到残留配对信息（幽灵设备），正在尝试自动清理...";
        warn!("{}", ghost_msg);
        self.send_log(ghost_msg, MessageSeverity::Warning);

        let Some(ghost_info) = system_device_info else {
            return Ok(());
        };

        info!("Attempting to unpair stale system record for device");
        match ghost_info.Pairing()?.UnpairAsync()?.await {
            Ok(result) => {
                let status = result.Status()?;
                info!("Stale pairing cleanup result: {:?}", status);
                if status == DeviceUnpairingResultStatus::Unpaired
                    || status == DeviceUnpairingResultStatus::AlreadyUnpaired
                {
                    self.send_log("残留配对已清除！请立刻重试连接。", MessageSeverity::Success);
                    anyhow::bail!("已清除残留系统配对。请点击‘连接’重试。");
                }

                self.send_log(
                    "自动清理失败，请在Windows设置中手动删除设备。",
                    MessageSeverity::Error,
                );
            }
            Err(e) => {
                error!("Stale pairing cleanup failed: {:?}", e);
                self.send_log(
                    "自动清理出错，请手动检查Windows设置。",
                    MessageSeverity::Error,
                );
            }
        }

        Ok(())
    }

    /// Attempt to unpair the device.
    pub(super) async fn unpair_device(&self, device: &BluetoothLEDevice) -> Result<()> {
        let device_info = device.DeviceInformation()?;
        let pairing = device_info.Pairing()?;

        info!("Attempting to unpair device...");
        self.send_log(
            "Attempting to unpair device to fix connection...",
            MessageSeverity::Warning,
        );

        match pairing.UnpairAsync()?.await {
            Ok(result) => {
                let status = result.Status()?;
                info!("Unpair status: {:?}", status);

                if status == DeviceUnpairingResultStatus::Unpaired
                    || status == DeviceUnpairingResultStatus::AlreadyUnpaired
                {
                    self.send_log(
                        "Device successfully unpaired. Please restart the application.",
                        MessageSeverity::Success,
                    );
                } else {
                    let msg = format!("Unpair failed with status: {:?}", status);
                    warn!("{}", msg);
                    self.send_log(&msg, MessageSeverity::Error);
                }
            }
            Err(e) => {
                error!("Unpair error: {:?}", e);
                self.send_log(
                    "Failed to unpair device (may require setup in Windows settings).",
                    MessageSeverity::Error,
                );
            }
        }

        Ok(())
    }
}
