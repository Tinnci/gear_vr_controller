//! BLE Scanner Module
//!
//! Handles Bluetooth LE device discovery for Gear VR Controllers.

use crate::domain::models::{AppEvent, MessageSeverity, ScannedDevice, StatusMessage};
use crate::infrastructure::bluetooth::protocol;
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::info;
use windows::Devices::Bluetooth::Advertisement::{
    BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
    BluetoothLEScanningMode,
};
use windows::Foundation::TypedEventHandler;

/// BLE Scanner for discovering Gear VR Controllers
pub struct BleScanner {
    watcher: Option<BluetoothLEAdvertisementWatcher>,
    event_sender: mpsc::UnboundedSender<AppEvent>,
}

impl BleScanner {
    /// Create a new scanner
    pub fn new(event_sender: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self {
            watcher: None,
            event_sender,
        }
    }

    /// Start scanning for BLE devices
    ///
    /// # Arguments
    /// * `service_uuid` - The service UUID to filter for (or None to show all devices)
    /// * `show_all_devices` - If true, show all BLE devices regardless of service UUID
    pub fn start(&mut self, service_uuid: Option<&str>, show_all_devices: bool) -> Result<()> {
        // Stop any existing scan
        self.stop()?;

        let uuid_str = service_uuid.unwrap_or(protocol::SERVICE_UUID);
        info!("Starting BLE scan for service UUID: {}", uuid_str);

        let _ = self.event_sender.send(AppEvent::LogMessage(StatusMessage {
            message: "Scanning for Gear VR Controller...".to_string(),
            severity: MessageSeverity::Info,
        }));

        let watcher = BluetoothLEAdvertisementWatcher::new()?;
        watcher.SetScanningMode(BluetoothLEScanningMode::Active)?;

        let sender = self.event_sender.clone();
        let target_uuid = protocol::parse_uuid(uuid_str)?;

        let handler = TypedEventHandler::new(
            move |_: windows::core::Ref<BluetoothLEAdvertisementWatcher>,
                  args: windows::core::Ref<BluetoothLEAdvertisementReceivedEventArgs>| {
                if let Some(args) = args.as_ref() {
                    let adv = args.Advertisement()?;
                    let service_uuids = adv.ServiceUuids()?;

                    // Check if this device matches our target service
                    let mut found = show_all_devices;
                    if !found {
                        for i in 0..service_uuids.Size()? {
                            if service_uuids.GetAt(i)? == target_uuid {
                                found = true;
                                break;
                            }
                        }
                    }

                    if found {
                        let name = adv.LocalName()?.to_string();
                        let address = args.BluetoothAddress()?;
                        let rssi = args.RawSignalStrengthInDBm()?;

                        let device = ScannedDevice {
                            name: if name.is_empty() {
                                "Unknown".to_string()
                            } else {
                                name
                            },
                            address,
                            signal_strength: rssi,
                        };

                        let _ = sender.send(AppEvent::DeviceFound(device));
                    }
                }
                Ok(())
            },
        );

        watcher.Received(&handler)?;
        watcher.Start()?;
        self.watcher = Some(watcher);

        Ok(())
    }

    /// Stop scanning
    pub fn stop(&mut self) -> Result<()> {
        if let Some(watcher) = self.watcher.take() {
            info!("Stopping BLE scan...");
            let _ = self.event_sender.send(AppEvent::LogMessage(StatusMessage {
                message: "Scan stopped.".to_string(),
                severity: MessageSeverity::Info,
            }));
            watcher.Stop()?;
        }
        Ok(())
    }

    /// Check if currently scanning
    pub fn is_scanning(&self) -> bool {
        self.watcher.is_some()
    }
}

impl Drop for BleScanner {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
