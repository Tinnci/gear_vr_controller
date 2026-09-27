//! BLE Connection Module
//!
//! Handles device connection, pairing, and GATT service access.

mod gatt;
mod init;
mod notifications;
mod pairing;
mod types;

use crate::application::event_bus::EventSender;
use crate::domain::models::{AppEvent, MessageSeverity, StatusMessage};
use anyhow::Result;
use tracing::{info, warn};

pub use types::{ConnectionConfig, ConnectionResult};

/// BLE Connection handler
pub struct BleConnection {
    event_sender: EventSender,
    config: ConnectionConfig,
}

impl BleConnection {
    /// Create a new connection handler
    pub fn new(event_sender: EventSender, config: ConnectionConfig) -> Self {
        Self {
            event_sender,
            config,
        }
    }

    /// Connect to a device by Bluetooth address
    pub async fn connect(&self, address: u64) -> Result<ConnectionResult> {
        info!(event = "ble.connect", "Connecting to controller");
        self.send_log("Connecting to device...", MessageSeverity::Info);
        super::diagnostics::adapter_snapshot("before_connect").await;
        tracing::debug!(event = "ble.connection.config",
            service_uuid = %self.config.service_uuid,
            data_uuid = %self.config.data_char_uuid, command_uuid = %self.config.command_char_uuid,
            max_retries = self.config.max_pairing_retries, retry_delay_ms = self.config.pairing_retry_delay_ms,
            "Controller protocol configuration");

        let device = self.connect_device(address).await?;
        let mut device_guard = DeviceGuard(Some(device.clone()));
        info!(
            event = "ble.device.opened",
            "Bluetooth device handle opened"
        );
        super::diagnostics::device_snapshot(&device, "opened");

        let session = match self.create_gatt_session(&device).await {
            Ok(session) => {
                super::diagnostics::session_snapshot(&session);
                Some(session)
            }
            Err(error) => {
                tracing::debug!(event = "ble.session.unavailable", error = %error,
                    "GATT session unavailable; direct service discovery continues");
                None
            }
        };
        let mut session_guard = SessionGuard(session);

        let was_paired = self.handle_pairing(&device).await?;

        let (data_char, cmd_char, service) = self.get_characteristics(&device).await?;
        device_guard.0 = None;
        let result = ConnectionResult {
            device,
            data_characteristic: data_char,
            session: session_guard.0.take(),
            service,
        };
        // Try notifications before init because it can surface the Windows pairing dialog earlier.
        let notifications_enabled = self
            .try_enable_notifications(&result.data_characteristic, was_paired, &result.device)
            .await;

        self.send_init_commands(&cmd_char).await?;
        if !notifications_enabled {
            self.retry_notifications_after_init(
                &result.data_characteristic,
                was_paired,
                &result.device,
            )
            .await?;
        }
        Ok(result)
    }

    async fn try_enable_notifications(
        &self,
        data_char: &windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristic,
        was_paired: bool,
        device: &windows::Devices::Bluetooth::BluetoothLEDevice,
    ) -> bool {
        match self
            .enable_notifications(data_char, was_paired, device)
            .await
        {
            Ok(()) => true,
            Err(e) => {
                warn!(
                    event = "ble.notifications.deferred", error = %e,
                    "Retry notifications after controller initialization"
                );
                false
            }
        }
    }

    async fn retry_notifications_after_init(
        &self,
        data_char: &windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristic,
        was_paired: bool,
        device: &windows::Devices::Bluetooth::BluetoothLEDevice,
    ) -> Result<()> {
        info!(
            event = "ble.notifications.retry",
            stage = "after_init",
            "Retrying notification subscription"
        );
        self.enable_notifications(data_char, was_paired, device)
            .await
    }

    pub(super) fn send_log(&self, message: &str, severity: MessageSeverity) {
        let _ = self.event_sender.send(AppEvent::LogMessage(StatusMessage {
            message: message.to_string(),
            severity,
        }));
    }
}

struct DeviceGuard(Option<windows::Devices::Bluetooth::BluetoothLEDevice>);
struct SessionGuard(Option<windows::Devices::Bluetooth::GenericAttributeProfile::GattSession>);
impl Drop for SessionGuard {
    fn drop(&mut self) {
        if let Some(session) = &self.0 {
            let _ = session.SetMaintainConnection(false);
            let _ = session.Close();
        }
    }
}
impl Drop for DeviceGuard {
    fn drop(&mut self) {
        if let Some(device) = &self.0 {
            let _ = device.Close();
        }
    }
}
