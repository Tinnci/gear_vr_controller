//! BLE Connection Module
//!
//! Handles device connection, pairing, and GATT service access.

use crate::domain::models::{AppEvent, MessageSeverity, StatusMessage};
use crate::infrastructure::bluetooth::protocol::{self, COMMAND_DELAY_MS, INIT_SEQUENCE};
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
    GattCommunicationStatus,
};
use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothLEDevice};
use windows::Storage::Streams::DataWriter;

/// Configuration for connection behavior
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Maximum pairing retry attempts
    pub max_pairing_retries: u32,
    /// Delay between pairing retries in milliseconds
    pub pairing_retry_delay_ms: u64,
    /// Service UUID to look for
    pub service_uuid: String,
    /// Data characteristic UUID
    pub data_char_uuid: String,
    /// Command characteristic UUID
    pub command_char_uuid: String,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            max_pairing_retries: 3,
            pairing_retry_delay_ms: 1000,
            service_uuid: protocol::SERVICE_UUID.to_string(),
            data_char_uuid: protocol::DATA_CHAR_UUID.to_string(),
            command_char_uuid: protocol::COMMAND_CHAR_UUID.to_string(),
        }
    }
}

/// Result of a successful connection
pub struct ConnectionResult {
    pub device: BluetoothLEDevice,
    pub data_characteristic: GattCharacteristic,
    pub command_characteristic: GattCharacteristic,
}

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

        // Step 1: Connect to BLE device
        let device = self.connect_device(address).await?;
        info!("Device connected: {:?}", device.Name()?);

        // Step 2: Create GattSession to maintain connection
        // This helps prevent Windows from requiring additional pairing
        if let Ok(session) = self.create_gatt_session(&device).await {
            info!("GattSession created, MaintainConnection set to true");
            // Keep session alive by not dropping it
            std::mem::forget(session);
        } else {
            warn!("Failed to create GattSession, continuing anyway...");
        }

        // Step 3: Handle pairing (simplified - just logs status)
        self.handle_pairing(&device).await?;

        // Step 4: Get GATT services and characteristics
        let (data_char, cmd_char) = self.get_characteristics(&device).await?;

        // Step 5: Try enabling notifications BEFORE sending init commands
        // Some devices need this order, and it may trigger the pairing dialog earlier
        let notifications_enabled = match self.enable_notifications(&data_char).await {
            Ok(()) => true,
            Err(e) => {
                warn!(
                    "Could not enable notifications: {}. Will try after init commands.",
                    e
                );
                false
            }
        };

        // Step 6: Send initialization commands
        self.send_init_commands(&cmd_char).await?;

        // Step 7: If notifications weren't enabled earlier, try again
        if !notifications_enabled {
            info!("Retrying notification subscription after init commands...");
            if let Err(e) = self.enable_notifications(&data_char).await {
                // If still failing, log warning but continue - device may auto-send data
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

        Ok(ConnectionResult {
            device,
            data_characteristic: data_char,
            command_characteristic: cmd_char,
        })
    }

    /// Create a GattSession to maintain the BLE connection
    async fn create_gatt_session(
        &self,
        device: &BluetoothLEDevice,
    ) -> Result<windows::Devices::Bluetooth::GenericAttributeProfile::GattSession> {
        use windows::Devices::Bluetooth::GenericAttributeProfile::GattSession;

        let device_id = device.BluetoothDeviceId()?;
        let session = GattSession::FromDeviceIdAsync(&device_id)?.await?;
        session.SetMaintainConnection(true)?;
        Ok(session)
    }

    /// Connect to BLE device
    async fn connect_device(&self, address: u64) -> Result<BluetoothLEDevice> {
        let device_async = BluetoothLEDevice::FromBluetoothAddressAsync(address)?;
        let device = device_async.await?;
        Ok(device)
    }

    /// Handle device pairing
    ///
    /// For BLE devices, traditional pairing is often not needed.
    /// We skip pairing and directly access GATT services.
    /// If that fails due to access issues, we can try pairing then.
    async fn handle_pairing(&self, device: &BluetoothLEDevice) -> Result<()> {
        let device_info = device.DeviceInformation()?;
        let pairing = device_info.Pairing()?;
        let is_paired = pairing.IsPaired()?;

        info!("Device pairing status - IsPaired: {}", is_paired);

        if is_paired {
            info!("Device already paired");
            self.send_log("Device already paired", MessageSeverity::Info);
            return Ok(());
        }

        // For BLE devices like Gear VR Controller, we often don't need traditional pairing
        // The device uses "Just Works" pairing or no pairing at all
        // Skip pairing attempt and proceed directly to GATT access
        info!("BLE device not paired - will attempt direct GATT access (no traditional pairing needed)");
        self.send_log(
            "Connecting without traditional pairing...",
            MessageSeverity::Info,
        );

        Ok(())
    }

    /// Get GATT characteristics
    async fn get_characteristics(
        &self,
        device: &BluetoothLEDevice,
    ) -> Result<(GattCharacteristic, GattCharacteristic)> {
        let service_uuid = protocol::parse_uuid(&self.config.service_uuid)?;
        let data_uuid = protocol::parse_uuid(&self.config.data_char_uuid)?;
        let cmd_uuid = protocol::parse_uuid(&self.config.command_char_uuid)?;

        // Get services
        let services_result = device.GetGattServicesForUuidAsync(service_uuid)?.await?;

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
        info!("Found controller service");

        // Request access
        info!("Requesting service access...");
        let access_status = service.RequestAccessAsync()?.await?;
        info!("Service access status: {:?}", access_status);

        // Get characteristics
        let chars_result = service.GetCharacteristicsAsync()?.await?;
        if chars_result.Status()? != GattCommunicationStatus::Success {
            anyhow::bail!("Failed to get characteristics");
        }

        let characteristics = chars_result.Characteristics()?;
        info!("Found {} characteristics", characteristics.Size()?);

        let mut data_char = None;
        let mut cmd_char = None;

        for i in 0..characteristics.Size()? {
            let c = characteristics.GetAt(i)?;
            let uuid = c.Uuid()?;

            if uuid == data_uuid {
                data_char = Some(c);
                info!("Found data characteristic");
            } else if uuid == cmd_uuid {
                cmd_char = Some(c.clone());
                info!("Found command characteristic");
            }
        }

        let data = data_char.ok_or_else(|| anyhow::anyhow!("Data characteristic not found"))?;
        let cmd = cmd_char.ok_or_else(|| anyhow::anyhow!("Command characteristic not found"))?;

        Ok((data, cmd))
    }

    /// Send initialization commands to the controller
    async fn send_init_commands(&self, cmd_char: &GattCharacteristic) -> Result<()> {
        info!("Sending initialization commands...");
        self.send_log("Initializing controller...", MessageSeverity::Info);

        for (command, repeat) in INIT_SEQUENCE {
            for _ in 0..*repeat {
                let writer = DataWriter::new()?;
                writer.WriteBytes(command.as_bytes())?;
                let buffer = writer.DetachBuffer()?;

                // Fire-and-forget write
                let _ = cmd_char.WriteValueAsync(&buffer)?;
                tokio::time::sleep(tokio::time::Duration::from_millis(COMMAND_DELAY_MS)).await;
            }
        }

        info!("Initialization commands sent");
        Ok(())
    }

    /// Enable notifications on data characteristic with retry logic
    async fn enable_notifications(&self, data_char: &GattCharacteristic) -> Result<()> {
        info!("Enabling notifications...");

        // Retry up to 3 times for notification subscription
        for attempt in 1..=3 {
            match data_char
                .WriteClientCharacteristicConfigurationDescriptorAsync(
                    GattClientCharacteristicConfigurationDescriptorValue::Notify,
                )?
                .await
            {
                Ok(status) => {
                    if status == GattCommunicationStatus::Success {
                        info!("Notifications enabled successfully");
                        self.send_log("Connection established!", MessageSeverity::Success);
                        return Ok(());
                    } else {
                        warn!("Notification subscription returned status: {:?}", status);
                        if attempt < 3 {
                            info!("Retrying notification subscription...");
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    }
                }
                Err(e) => {
                    let error_str = format!("{:?}", e);
                    warn!(
                        "Notification subscription attempt {} failed: {}",
                        attempt, error_str
                    );

                    // Check for user cancelled error (0x800704C7)
                    if error_str.contains("800704C7") {
                        self.send_log(
                            "Please accept the pairing dialog when it appears",
                            MessageSeverity::Warning,
                        );
                    }

                    if attempt < 3 {
                        info!("Retrying in 1 second...");
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
                    } else {
                        // On final attempt failure, return error
                        error!("Failed to enable notifications after {} attempts", attempt);
                        anyhow::bail!("Failed to enable notifications: {}", e);
                    }
                }
            }
        }

        error!("Failed to enable notifications after all attempts");
        anyhow::bail!("Failed to enable notifications")
    }

    /// Check if device is connected
    pub fn is_connected(device: &BluetoothLEDevice) -> bool {
        device
            .ConnectionStatus()
            .map(|s| s == BluetoothConnectionStatus::Connected)
            .unwrap_or(false)
    }

    /// Send a log message
    fn send_log(&self, message: &str, severity: MessageSeverity) {
        let _ = self.event_sender.send(AppEvent::LogMessage(StatusMessage {
            message: message.to_string(),
            severity,
        }));
    }
}
