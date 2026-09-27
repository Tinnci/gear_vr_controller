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
        _device: &BluetoothLEDevice,
    ) -> Result<()> {
        info!(
            event = "ble.notifications.started",
            "Notification subscription started"
        );

        let max_attempts = self.config.max_pairing_retries.max(1);

        for attempt in 1..=max_attempts {
            tracing::debug!(
                event = "ble.notifications.attempt",
                attempt,
                max_attempts,
                "Notification subscription attempt started"
            );
            match self.write_notify_descriptor(data_char).await {
                Ok(status) => {
                    if self.handle_notify_status(status, was_paired).await? {
                        return Ok(());
                    }
                    if attempt == max_attempts {
                        super::gatt::check_status("Subscribe to controller notifications", status)?;
                    }

                    self.sleep_before_retry(attempt, max_attempts).await;
                }
                Err(e) => {
                    self.handle_notify_error(attempt, max_attempts, &e).await?;
                }
            }
        }

        error!(
            event = "ble.notifications.exhausted",
            attempts = max_attempts,
            "Notification retries exhausted"
        );
        anyhow::bail!("Failed to enable notifications")
    }

    async fn write_notify_descriptor(
        &self,
        data_char: &GattCharacteristic,
    ) -> Result<GattCommunicationStatus, windows::core::Error> {
        super::super::diagnostics::operation("write_notification_descriptor", async {
            data_char
                .WriteClientCharacteristicConfigurationDescriptorAsync(
                    GattClientCharacteristicConfigurationDescriptorValue::Notify,
                )?
                .await
        })
        .await
    }

    async fn handle_notify_status(
        &self,
        status: GattCommunicationStatus,
        was_paired: bool,
    ) -> Result<bool> {
        if status == GattCommunicationStatus::Success {
            info!(
                event = "ble.notifications.enabled",
                "Controller notifications enabled"
            );
            self.send_log("Connection established!", MessageSeverity::Success);
            return Ok(true);
        }

        warn!(event = "ble.notifications.rejected", status = ?status, "Notification subscription rejected");

        if status == GattCommunicationStatus::Unreachable && was_paired {
            warn!(
                event = "ble.pairing.unreachable",
                "Paired device is unreachable; notification retry is pending"
            );
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
            event = "ble.notifications.failed", attempt, max_attempts, error = %error_str,
            "Notification subscription attempt failed"
        );

        if error_str.contains("800704C7") {
            self.send_log(
                "Please accept the pairing dialog when it appears",
                MessageSeverity::Info,
            );
        }

        if attempt < max_attempts {
            info!(
                event = "ble.notifications.retry",
                delay_ms = self.config.pairing_retry_delay_ms,
                "Notification retry scheduled"
            );
            self.sleep_for_retry_delay().await;
            return Ok(());
        }

        error!(event = "ble.notifications.exhausted", attempts = attempt, error = %error, "Notification retries exhausted");
        anyhow::bail!("Failed to enable notifications: {}", error)
    }

    async fn sleep_before_retry(&self, attempt: u32, max_attempts: u32) {
        if attempt < max_attempts {
            info!(
                event = "ble.notifications.retry",
                attempt, max_attempts, "Retrying notification subscription"
            );
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
