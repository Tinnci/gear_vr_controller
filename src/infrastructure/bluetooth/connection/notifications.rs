use super::BleConnection;
use crate::domain::models::MessageSeverity;
use anyhow::Result;
use tracing::{error, info, warn};
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
    GattCommunicationStatus,
};

impl BleConnection {
    /// Enable notifications on the data characteristic with retry logic.
    pub(super) async fn enable_notifications(
        &self,
        data_char: &GattCharacteristic,
        was_paired: bool,
        device: &BluetoothLEDevice,
    ) -> Result<()> {
        info!("Enabling notifications...");

        let max_attempts = self.config.max_pairing_retries.max(1);

        for attempt in 1..=max_attempts {
            match self.write_notify_descriptor(data_char).await {
                Ok(status) => {
                    if self
                        .handle_notify_status(status, was_paired, device)
                        .await?
                    {
                        return Ok(());
                    }

                    self.sleep_before_retry(attempt, max_attempts).await;
                }
                Err(e) => {
                    self.handle_notify_error(attempt, max_attempts, &e).await?;
                }
            }
        }

        error!("Failed to enable notifications after all attempts");
        anyhow::bail!("Failed to enable notifications")
    }

    async fn write_notify_descriptor(
        &self,
        data_char: &GattCharacteristic,
    ) -> Result<GattCommunicationStatus, windows::core::Error> {
        data_char
            .WriteClientCharacteristicConfigurationDescriptorAsync(
                GattClientCharacteristicConfigurationDescriptorValue::Notify,
            )?
            .await
    }

    async fn handle_notify_status(
        &self,
        status: GattCommunicationStatus,
        was_paired: bool,
        device: &BluetoothLEDevice,
    ) -> Result<bool> {
        if status == GattCommunicationStatus::Success {
            info!("Notifications enabled successfully");
            self.send_log("Connection established!", MessageSeverity::Success);
            return Ok(true);
        }

        warn!("Notification subscription returned status: {:?}", status);

        if status == GattCommunicationStatus::Unreachable && was_paired {
            let warn_msg = "检测到设备已在系统中配对，请尝试在 Windows 设置中‘删除设备’后重试。";
            self.send_log(warn_msg, MessageSeverity::Error);
            warn!("{}", warn_msg);
            let _ = self.unpair_device(device).await;
        }

        Ok(false)
    }

    async fn handle_notify_error(
        &self,
        attempt: u32,
        max_attempts: u32,
        error: &windows::core::Error,
    ) -> Result<()> {
        let error_str = format!("{:?}", error);
        warn!(
            "Notification subscription attempt {} failed: {}",
            attempt, error_str
        );

        if error_str.contains("800704C7") {
            self.send_log(
                "Please accept the pairing dialog when it appears",
                MessageSeverity::Warning,
            );
        }

        if attempt < max_attempts {
            info!("Retrying in {} ms...", self.config.pairing_retry_delay_ms);
            self.sleep_for_retry_delay().await;
            return Ok(());
        }

        error!("Failed to enable notifications after {} attempts", attempt);
        anyhow::bail!("Failed to enable notifications: {}", error)
    }

    async fn sleep_before_retry(&self, attempt: u32, max_attempts: u32) {
        if attempt < max_attempts {
            info!("Retrying notification subscription...");
            self.sleep_for_retry_delay().await;
        }
    }

    async fn sleep_for_retry_delay(&self) {
        tokio::time::sleep(tokio::time::Duration::from_millis(
            self.config.pairing_retry_delay_ms,
        ))
        .await;
    }
}
