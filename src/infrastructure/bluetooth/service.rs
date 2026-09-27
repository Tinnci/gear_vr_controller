//! Bluetooth Service Module
//!
//! Main service that coordinates scanning, connection, and data handling
//! for the Gear VR Controller.

use crate::application::event_bus::EventSender;
use crate::domain::models::{AppEvent, ConnectionStatus, MessageSeverity, StatusMessage};
use crate::domain::settings::SettingsService;
use crate::infrastructure::bluetooth::{
    connection::{BleConnection, ConnectionConfig, ConnectionResult},
    protocol,
    scanner::BleScanner,
};
use anyhow::Result;
use std::sync::{Arc, Mutex};
use tracing::info;
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattValueChangedEventArgs,
};
use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothLEDevice};
use windows::Foundation::TypedEventHandler;

/// Main Bluetooth service coordinating all BLE operations
pub struct BluetoothService {
    connection: Option<ConnectionResult>,
    data_token: Option<i64>,
    status_token: Option<i64>,
    scanner: BleScanner,
    event_sender: EventSender,
    settings: Arc<Mutex<SettingsService>>,
}

impl BluetoothService {
    /// Create a new Bluetooth service
    pub fn new(event_sender: EventSender, settings: Arc<Mutex<SettingsService>>) -> Self {
        Self {
            connection: None,
            data_token: None,
            status_token: None,
            scanner: BleScanner::new(event_sender.clone()),
            event_sender,
            settings,
        }
    }

    /// Start scanning for devices
    pub fn start_scan(&mut self) -> Result<()> {
        let (service_uuid, show_all, known, last_used) = {
            let settings = self
                .settings
                .lock()
                .map_err(|_| anyhow::anyhow!("Lock error"))?;
            let s = settings.get();
            (
                s.ble_service_uuid.clone(),
                s.debug_show_all_devices,
                s.known_bluetooth_addresses.clone(),
                s.last_connected_address,
            )
        };

        self.scanner
            .start(Some(&service_uuid), show_all, known, last_used)
    }

    /// Stop scanning
    pub fn stop_scan(&mut self) -> Result<()> {
        self.scanner.stop()
    }
    pub fn poll_scan(&self) {
        self.scanner.poll();
    }

    /// Connect to a device by address
    pub async fn connect(&mut self, address: u64) -> Result<()> {
        self.disconnect();
        self.stop_scan()?;
        let _ = self.event_sender.send(AppEvent::ScanState(false));
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
        self.connection = Some(result);

        // Save to history on successful connection
        {
            if let Ok(mut settings) = self.settings.lock() {
                if let Err(error) = settings.record_connection(address) {
                    tracing::warn!(event = "settings.history.failed", error = %format!("{error:#}"), "Cannot save connection history");
                    let _ = self.event_sender.send(AppEvent::LogMessage(StatusMessage {
                        message: format!("Connected, but cannot save history: {error}"),
                        severity: MessageSeverity::Warning,
                    }));
                }
            }
        }

        // Notify connection success
        let _ = self
            .event_sender
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Connected));

        Ok(())
    }

    /// Set up event handlers for data and connection status
    fn setup_event_handlers(&mut self, result: &ConnectionResult) -> Result<()> {
        // Data notification handler
        let sender = self.event_sender.clone();
        #[cfg(debug_assertions)]
        let settings = self.settings.clone();
        let data_handler = TypedEventHandler::new(
            move |_: windows::core::Ref<GattCharacteristic>,
                  args: windows::core::Ref<GattValueChangedEventArgs>| {
                if let Some(args) = args.as_ref() {
                    if let Ok(value) = args.CharacteristicValue() {
                        if let Ok(data) = protocol::parse_data_packet(&value) {
                            #[cfg(debug_assertions)]
                            if settings
                                .lock()
                                .is_ok_and(|svc| svc.get().debug_raw_data_logging)
                            {
                                tracing::trace!(event = "ble.packet", raw = ?data.raw_bytes, "Controller packet received");
                            }
                            let _ = sender.send(AppEvent::ControllerData(data));
                        }
                    }
                }
                Ok(())
            },
        );
        let data_token = result.data_characteristic.ValueChanged(&data_handler)?;

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
        match result.device.ConnectionStatusChanged(&status_handler) {
            Ok(token) => {
                self.data_token = Some(data_token);
                self.status_token = Some(token);
            }
            Err(error) => {
                let _ = result.data_characteristic.RemoveValueChanged(data_token);
                return Err(error.into());
            }
        }

        Ok(())
    }

    /// Disconnect from the current device
    pub fn disconnect(&mut self) {
        if let Some(connection) = self.connection.take() {
            if let Some(token) = self.data_token.take() {
                let _ = connection.data_characteristic.RemoveValueChanged(token);
            }
            if let Some(token) = self.status_token.take() {
                let _ = connection.device.RemoveConnectionStatusChanged(token);
            }
            drop(connection);
            info!(event = "ble.disconnected", "Controller disconnected");
            let _ = self.event_sender.send(AppEvent::LogMessage(StatusMessage {
                message: "Disconnected from device".to_string(),
                severity: MessageSeverity::Info,
            }));
        }

        let _ = self
            .event_sender
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection
            .as_ref()
            .and_then(|c| c.device.ConnectionStatus().ok())
            .map(|s| s == BluetoothConnectionStatus::Connected)
            .unwrap_or(false)
    }
}

impl Drop for BluetoothService {
    fn drop(&mut self) {
        self.disconnect();
    }
}
