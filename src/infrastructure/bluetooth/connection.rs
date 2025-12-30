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
use windows::Devices::Enumeration::DevicePairingResultStatus;
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

        // Step 2: Handle pairing
        self.handle_pairing(&device).await?;

        // Step 3: Get GATT services and characteristics
        let (data_char, cmd_char) = self.get_characteristics(&device).await?;

        // Step 4: Send initialization commands
        self.send_init_commands(&cmd_char).await?;

        // Step 5: Enable notifications
        self.enable_notifications(&data_char).await?;

        Ok(ConnectionResult {
            device,
            data_characteristic: data_char,
            command_characteristic: cmd_char,
        })
    }

    /// Connect to BLE device
    async fn connect_device(&self, address: u64) -> Result<BluetoothLEDevice> {
        let device_async = BluetoothLEDevice::FromBluetoothAddressAsync(address)?;
        let device = device_async.await?;
        Ok(device)
    }

    /// Handle device pairing with retry logic
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

        // Attempt custom pairing with retries
        self.send_log("Pairing with controller...", MessageSeverity::Info);

        for attempt in 1..=self.config.max_pairing_retries {
            info!(
                "Pairing attempt {}/{}",
                attempt, self.config.max_pairing_retries
            );

            let custom_pairing = pairing.Custom()?;

            match custom_pairing
                .PairAsync(windows::Devices::Enumeration::DevicePairingKinds::ConfirmOnly)?
                .await
            {
                Ok(pair_result) => {
                    let status = pair_result.Status()?;
                    info!("Pairing attempt {} result: {:?}", attempt, status);

                    match status {
                        DevicePairingResultStatus::Paired => {
                            self.send_log("Pairing successful!", MessageSeverity::Success);
                            return Ok(());
                        }
                        DevicePairingResultStatus::AlreadyPaired => {
                            info!("Device was already paired");
                            return Ok(());
                        }
                        DevicePairingResultStatus::Failed => {
                            // Status 19 - BLE devices often don't need traditional pairing
                            info!(
                                "Pairing 'Failed' - BLE device may not require traditional pairing"
                            );
                            self.send_log(
                                "Device may not require pairing, continuing...",
                                MessageSeverity::Info,
                            );
                            return Ok(());
                        }
                        DevicePairingResultStatus::AccessDenied => {
                            warn!("Pairing access denied");
                            self.send_log(
                                "Access denied. Try removing device from Windows Bluetooth settings.",
                                MessageSeverity::Warning,
                            );
                        }
                        DevicePairingResultStatus::AuthenticationFailure => {
                            warn!("Pairing authentication failed");
                        }
                        DevicePairingResultStatus::ConnectionRejected => {
                            warn!("Device rejected connection");
                        }
                        DevicePairingResultStatus::RequiredHandlerNotRegistered => {
                            info!("No handler required, continuing...");
                            return Ok(());
                        }
                        _ => {
                            warn!("Unexpected pairing status: {:?}", status);
                        }
                    }
                }
                Err(e) => {
                    let error_str = format!("{:?}", e);
                    warn!("Pairing attempt {} failed: {}", attempt, error_str);

                    if error_str.contains("800704C7") {
                        self.send_log(
                            "Pairing was cancelled. Please accept when prompted.",
                            MessageSeverity::Warning,
                        );
                    }
                }
            }

            // Retry delay
            if attempt < self.config.max_pairing_retries {
                tokio::time::sleep(tokio::time::Duration::from_millis(
                    self.config.pairing_retry_delay_ms,
                ))
                .await;
            }
        }

        warn!("All pairing attempts completed, continuing anyway...");
        self.send_log(
            "Pairing incomplete, attempting connection...",
            MessageSeverity::Warning,
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

    /// Enable notifications on data characteristic
    async fn enable_notifications(&self, data_char: &GattCharacteristic) -> Result<()> {
        info!("Enabling notifications...");

        let status = data_char
            .WriteClientCharacteristicConfigurationDescriptorAsync(
                GattClientCharacteristicConfigurationDescriptorValue::Notify,
            )?
            .await?;

        if status != GattCommunicationStatus::Success {
            error!("Failed to enable notifications: {:?}", status);
            anyhow::bail!("Failed to enable notifications");
        }

        info!("Notifications enabled successfully");
        self.send_log("Connection established!", MessageSeverity::Success);
        Ok(())
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
