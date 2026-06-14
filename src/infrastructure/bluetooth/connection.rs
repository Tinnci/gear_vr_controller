//! BLE Connection Module
//!
//! Handles device connection, pairing, and GATT service access.

mod gatt;
mod init;
mod notifications;
mod pairing;
mod types;

use crate::domain::models::{AppEvent, MessageSeverity, StatusMessage};
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub use types::{ConnectionConfig, ConnectionResult};

/// BLE Connection handler
pub struct BleConnection {
    event_sender: mpsc::UnboundedSender<AppEvent>,
    config: ConnectionConfig,
}

impl BleConnection {
    /// Create a new connection handler
    pub fn new(event_sender: mpsc::UnboundedSender<AppEvent>, config: ConnectionConfig) -> Self {
        Self {
            event_sender,
            config,
        }
    }

    /// Connect to a device by Bluetooth address
    pub async fn connect(&self, address: u64) -> Result<ConnectionResult> {
        info!("Connecting to Bluetooth device: {:#X}", address);
        self.send_log("Connecting to device...", MessageSeverity::Info);

        let device = self.connect_device(address).await?;
        info!("Device connected: {:?}", device.Name()?);

        if let Ok(session) = self.create_gatt_session(&device).await {
            info!("GattSession created, MaintainConnection set to true");
            // Keep the WinRT session alive for the lifetime of the process.
            std::mem::forget(session);
        } else {
            warn!("Failed to create GattSession, continuing anyway...");
        }

        let system_device_info = self
            .check_system_paired_status(address)
            .await
            .unwrap_or(None);
        let system_paired = system_device_info.is_some();
        self.log_system_pairing_state(system_paired);

        let was_paired = self.handle_pairing(&device).await?;
        if system_paired && !was_paired {
            self.clear_stale_pairing_record(system_device_info).await?;
        }

        let (data_char, cmd_char) = self.get_characteristics(&device).await?;
        // Try notifications before init because it can surface the Windows pairing dialog earlier.
        let notifications_enabled = self
            .try_enable_notifications(&data_char, was_paired, &device)
            .await;

        self.send_init_commands(&cmd_char).await?;
        if !notifications_enabled {
            self.retry_notifications_after_init(&data_char, was_paired, &device)
                .await;
        }

        Ok(ConnectionResult {
            device,
            data_characteristic: data_char,
        })
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
                    "Could not enable notifications: {}. Will try after init commands.",
                    e
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
    ) {
        info!("Retrying notification subscription after init commands...");
        if let Err(e) = self
            .enable_notifications(data_char, was_paired, device)
            .await
        {
            warn!(
                "Notification subscription still failing: {}. Controller may still work.",
                e
            );
            self.send_log(
                "Connected (notifications may be limited)",
                MessageSeverity::Warning,
            );
        }
    }

    fn log_system_pairing_state(&self, system_paired: bool) {
        if system_paired {
            info!("System database confirms device is PAIRED");
        } else {
            info!("System database confirms device is NOT PAIRED");
        }
    }

    pub(super) fn send_log(&self, message: &str, severity: MessageSeverity) {
        let _ = self.event_sender.send(AppEvent::LogMessage(StatusMessage {
            message: message.to_string(),
            severity,
        }));
    }
}
