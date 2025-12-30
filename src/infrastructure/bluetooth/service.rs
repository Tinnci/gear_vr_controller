//! Bluetooth Service Module
//!
//! Main service that coordinates scanning, connection, and data handling
//! for the Gear VR Controller.

use crate::domain::models::{AppEvent, ConnectionStatus, MessageSeverity, StatusMessage};
use crate::domain::settings::SettingsService;
use crate::infrastructure::bluetooth::{
    connection::{BleConnection, ConnectionConfig, ConnectionResult},
    protocol,
    scanner::BleScanner,
};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::info;
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattValueChangedEventArgs,
};
use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothLEDevice};
use windows::Foundation::TypedEventHandler;

/// Main Bluetooth service coordinating all BLE operations
pub struct BluetoothService {
    device: Option<BluetoothLEDevice>,
    data_characteristic: Option<GattCharacteristic>,
    scanner: BleScanner,
    event_sender: mpsc::UnboundedSender<AppEvent>,
    settings: Arc<Mutex<SettingsService>>,
}

impl BluetoothService {
    /// Create a new Bluetooth service
    pub fn new(
        event_sender: mpsc::UnboundedSender<AppEvent>,
        settings: Arc<Mutex<SettingsService>>,
    ) -> Self {
        Self {
            device: None,
            data_characteristic: None,
            scanner: BleScanner::new(event_sender.clone()),
            event_sender,
            settings,
        }
    }

    /// Start scanning for devices
    pub fn start_scan(&mut self) -> Result<()> {
        let (service_uuid, show_all) = {
            let settings = self
                .settings
                .lock()
                .map_err(|_| anyhow::anyhow!("Lock error"))?;
            let s = settings.get();
            (s.ble_service_uuid.clone(), s.debug_show_all_devices)
        };

        self.scanner.start(Some(&service_uuid), show_all)
    }

    /// Stop scanning
    pub fn stop_scan(&mut self) -> Result<()> {
        self.scanner.stop()
    }

    /// Connect to a device by address
    pub async fn connect(&mut self, address: u64) -> Result<()> {
        // Get configuration from settings
        let config = {
            let settings = self
                .settings
                .lock()
                .map_err(|_| anyhow::anyhow!("Lock error"))?;
            let s = settings.get();
            ConnectionConfig {
                max_pairing_retries: s.pairing_max_retries,
                pairing_retry_delay_ms: s.pairing_retry_delay_ms,
                service_uuid: s.ble_service_uuid.clone(),
                data_char_uuid: s.ble_data_char_uuid.clone(),
                command_char_uuid: s.ble_command_char_uuid.clone(),
            }
        };

        // Create connection handler and connect
        let connection = BleConnection::new(self.event_sender.clone(), config);
        let result = connection.connect(address).await?;

        // Set up event handlers
        self.setup_event_handlers(&result)?;

        // Store references
        self.device = Some(result.device);
        self.data_characteristic = Some(result.data_characteristic);

        // Notify connection success
        let _ = self
            .event_sender
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Connected));

        Ok(())
    }

    /// Set up event handlers for data and connection status
    fn setup_event_handlers(&self, result: &ConnectionResult) -> Result<()> {
        // Data notification handler
        let sender = self.event_sender.clone();
        let data_handler = TypedEventHandler::new(
            move |_: windows::core::Ref<GattCharacteristic>,
                  args: windows::core::Ref<GattValueChangedEventArgs>| {
                if let Some(args) = args.as_ref() {
                    if let Ok(value) = args.CharacteristicValue() {
                        if let Ok(data) = protocol::parse_data_packet(&value) {
                            let _ = sender.send(AppEvent::ControllerData(data));
                        }
                    }
                }
                Ok(())
            },
        );
        result.data_characteristic.ValueChanged(&data_handler)?;

        // Connection status handler
        let sender = self.event_sender.clone();
        let status_handler =
            TypedEventHandler::new(move |dev: windows::core::Ref<BluetoothLEDevice>, _| {
                if let Some(dev) = dev.as_ref() {
                    if let Ok(status) = dev.ConnectionStatus() {
                        let app_status = match status {
                            BluetoothConnectionStatus::Connected => ConnectionStatus::Connected,
                            BluetoothConnectionStatus::Disconnected => {
                                ConnectionStatus::Disconnected
                            }
                            _ => ConnectionStatus::Error,
                        };
                        let _ = sender.send(AppEvent::ConnectionStatus(app_status));
                    }
                }
                Ok(())
            });
        result.device.ConnectionStatusChanged(&status_handler)?;

        Ok(())
    }

    /// Disconnect from the current device
    pub fn disconnect(&mut self) {
        if !self.is_connected() {
            return;
        }

        if let Some(device) = self.device.take() {
            let _ = device.Close();
        }
        self.data_characteristic = None;

        info!("Disconnected from device");
        let _ = self.event_sender.send(AppEvent::LogMessage(StatusMessage {
            message: "Disconnected from device".to_string(),
            severity: MessageSeverity::Info,
        }));
        let _ = self
            .event_sender
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.device
            .as_ref()
            .and_then(|d| d.ConnectionStatus().ok())
            .map(|s| s == BluetoothConnectionStatus::Connected)
            .unwrap_or(false)
    }
}
