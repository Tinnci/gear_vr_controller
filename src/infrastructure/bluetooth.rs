use crate::domain::models::{
    AppEvent, ConnectionStatus, ControllerData, MessageSeverity, ScannedDevice, StatusMessage,
};
use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use windows::core::GUID;
use windows::Devices::Bluetooth::Advertisement::{
    BluetoothLEAdvertisementReceivedEventArgs, BluetoothLEAdvertisementWatcher,
    BluetoothLEScanningMode,
};
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue,
    GattCommunicationStatus, GattDeviceService, GattValueChangedEventArgs,
};
use windows::Devices::Bluetooth::{BluetoothConnectionStatus, BluetoothLEDevice};
use windows::Foundation::TypedEventHandler;
use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};

// Gear VR Controller BLE Service and Characteristic UUIDs
// Constants representing the Gear VR Controller UUIDs are now loaded from Settings.
// Default values are in domain/settings.rs

use crate::domain::settings::SettingsService;
use std::sync::{Arc, Mutex};

pub struct BluetoothService {
    device: Option<BluetoothLEDevice>,
    data_characteristic: Option<GattCharacteristic>,
    data_sender: mpsc::UnboundedSender<AppEvent>,
    watcher: Option<BluetoothLEAdvertisementWatcher>,
    settings: Arc<Mutex<SettingsService>>,
}

impl BluetoothService {
    pub fn new(
        data_sender: mpsc::UnboundedSender<AppEvent>,
        settings: Arc<Mutex<SettingsService>>,
    ) -> Self {
        Self {
            device: None,
            data_characteristic: None,
            data_sender,
            watcher: None,
            settings,
        }
    }

    pub async fn connect(&mut self, address: u64) -> Result<()> {
        info!("Connecting to Bluetooth device: {:#X}", address);
        let _ = self.data_sender.send(AppEvent::LogMessage(StatusMessage {
            message: format!("Connecting to device {:#X}...", address),
            severity: MessageSeverity::Info,
        }));

        // Fetch UUIDs from settings
        let (service_uuid_str, data_uuid_str) = {
            let settings_guard = self
                .settings
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock settings"))?;
            let settings = settings_guard.get();
            (
                settings.ble_service_uuid.clone(),
                settings.ble_data_char_uuid.clone(),
            )
        };

        // 1. 连接到 BLE 设备 (正确使用 WinRT async/await)
        let device_async = BluetoothLEDevice::FromBluetoothAddressAsync(address)?;
        let device = device_async.await?;

        info!("Device connected: {:?}", device.Name()?);

        // 2. 获取 GATT 服务 (使用 Uncached 模式以避免缓存问题)
        let service_uuid = parse_uuid(&service_uuid_str)?;
        let services_result = device.GetGattServicesForUuidAsync(service_uuid)?.await?;

        if services_result.Status()? != GattCommunicationStatus::Success {
            error!(
                "Failed to get GATT services. Status: {:?}",
                services_result.Status()?
            );
            anyhow::bail!("Failed to get GATT services");
        }

        let services = services_result.Services()?;
        info!(
            "Found {} services matching controller UUID",
            services.Size()?
        );

        if services.Size()? == 0 {
            anyhow::bail!("Controller service not found");
        }

        let service = services.GetAt(0)?;
        info!("Found controller service");
        let _ = self.data_sender.send(AppEvent::LogMessage(StatusMessage {
            message: "Controller service found".to_string(),
            severity: MessageSeverity::Info,
        }));

        // 4. 获取特征值 (使用 Uncached 模式)
        let char_uuid = parse_uuid(&data_uuid_str)?;
        let chars_result = service.GetCharacteristicsForUuidAsync(char_uuid)?.await?;

        if chars_result.Status()? != GattCommunicationStatus::Success {
            error!(
                "Failed to get characteristics. Status: {:?}",
                chars_result.Status()?
            );
            anyhow::bail!("Failed to get characteristics");
        }

        let characteristics = chars_result.Characteristics()?;
        info!("Found {} data characteristics", characteristics.Size()?);

        if characteristics.Size()? == 0 {
            anyhow::bail!("Data characteristic not found");
        }

        let data_characteristic = characteristics.GetAt(0)?;
        let props = data_characteristic.CharacteristicProperties()?;
        info!("Data characteristic properties: {:?}", props);

        // 诊断: 尝试先读取特征值以测试基本通信
        info!("Attempting to read characteristic value for diagnostics...");
        let read_result = data_characteristic.ReadValueAsync()?.await?;
        match read_result.Status()? {
            GattCommunicationStatus::Success => {
                info!("Read test successful! Communication is working.");
            }
            other => {
                warn!(
                    "Read test failed with status: {:?}. This may indicate pairing issues.",
                    other
                );
            }
        }

        // 请求设备访问权限 (可能触发配对对话框)
        info!("Requesting device access...");
        let access_status = device.RequestAccessAsync()?.await?;
        info!("Device access status: {:?}", access_status);

        if access_status != windows::Devices::Enumeration::DeviceAccessStatus::Allowed {
            warn!(
                "Device access not allowed. Status: {:?}. Trying to continue anyway...",
                access_status
            );
        }

        // 检查并请求配对
        let device_info = device.DeviceInformation()?;
        let pairing = device_info.Pairing()?;
        info!(
            "Device pairing status - IsPaired: {:?}, CanPair: {:?}",
            pairing.IsPaired()?,
            pairing.CanPair()?
        );

        if !pairing.IsPaired()? && pairing.CanPair()? {
            info!("Device is not paired. Attempting to pair...");
            let pair_result = pairing.PairAsync()?.await?;
            info!("Pairing result: {:?}", pair_result.Status()?);

            // 配对后需要重新获取服务
            if pair_result.Status()?
                == windows::Devices::Enumeration::DevicePairingResultStatus::Paired
            {
                info!("Pairing successful! Reconnecting to get updated services...");
                // 配对成功后，返回错误让调用者重试连接
                anyhow::bail!("Pairing completed. Please reconnect.");
            }
        }

        // 6. 启用通知 (带重试逻辑)
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let mut notify_enabled = false;
        for attempt in 1..=3 {
            info!(
                "Attempting to enable notifications (attempt {}/3)...",
                attempt
            );

            let status = data_characteristic
                .WriteClientCharacteristicConfigurationDescriptorAsync(
                    GattClientCharacteristicConfigurationDescriptorValue::Notify,
                )?
                .await?;

            if status == GattCommunicationStatus::Success {
                notify_enabled = true;
                break;
            } else {
                warn!(
                    "Notification enable attempt {} failed. Status: {:?}",
                    attempt, status
                );
                if attempt < 3 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }

        if !notify_enabled {
            error!("Failed to enable notifications after 3 attempts");
            anyhow::bail!("Failed to enable notifications. Try removing the device from Windows Bluetooth settings and reconnect.");
        }

        info!("Notifications enabled successfully");
        let _ = self.data_sender.send(AppEvent::LogMessage(StatusMessage {
            message: "Notifications enabled. Handshake complete.".to_string(),
            severity: MessageSeverity::Info,
        }));

        // 7. 注册通知事件处理器 (在启用通知后)
        let sender = self.data_sender.clone();
        let handler = TypedEventHandler::new(
            move |_: windows::core::Ref<GattCharacteristic>,
                  args: windows::core::Ref<GattValueChangedEventArgs>| {
                if let Some(args) = args.as_ref() {
                    if let Ok(value) = args.CharacteristicValue() {
                        if let Ok(data) = Self::parse_controller_data(&value) {
                            let _ = sender.send(AppEvent::ControllerData(data));
                        }
                    }
                }
                Ok(())
            },
        );

        data_characteristic.ValueChanged(&handler)?;

        // 8. 可选：发送启动命令
        if let Err(e) = self.send_start_command(&service).await {
            warn!("Failed to send start command: {}", e);
        }

        // 9. Register connection status change handler
        let sender = self.data_sender.clone();
        device.ConnectionStatusChanged(&TypedEventHandler::new(
            move |dev: windows::core::Ref<BluetoothLEDevice>, _| {
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
            },
        ))?;

        self.device = Some(device);
        self.data_characteristic = Some(data_characteristic);

        let _ = self
            .data_sender
            .send(AppEvent::ConnectionStatus(ConnectionStatus::Connected));

        Ok(())
    }

    pub fn start_scan(&mut self) -> Result<()> {
        if self.watcher.is_some() {
            self.stop_scan()?;
        }

        // Fetch UUID and Debug setting from settings
        let (service_uuid_str, debug_show_all) = {
            let settings_guard = self
                .settings
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock settings"))?;
            let s = settings_guard.get();
            (s.ble_service_uuid.clone(), s.debug_show_all_devices)
        };

        info!(
            "Starting Bluetooth LE scan for service UUID: {}",
            service_uuid_str
        );
        let _ = self.data_sender.send(AppEvent::LogMessage(StatusMessage {
            message: "Scanning for Gear VR Controller...".to_string(),
            severity: MessageSeverity::Info,
        }));

        let watcher = BluetoothLEAdvertisementWatcher::new()?;
        watcher.SetScanningMode(BluetoothLEScanningMode::Active)?;

        let sender = self.data_sender.clone();
        let service_uuid = parse_uuid(&service_uuid_str)?;

        let handler = TypedEventHandler::new(
            move |_: windows::core::Ref<BluetoothLEAdvertisementWatcher>,
                  args: windows::core::Ref<BluetoothLEAdvertisementReceivedEventArgs>| {
                if let Some(args) = args.as_ref() {
                    let adv = args.Advertisement()?;
                    let service_uuids = adv.ServiceUuids()?;

                    let mut found = debug_show_all;
                    if !found {
                        for i in 0..service_uuids.Size()? {
                            if service_uuids.GetAt(i)? == service_uuid {
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

    pub fn stop_scan(&mut self) -> Result<()> {
        if let Some(watcher) = self.watcher.take() {
            info!("Stopping Bluetooth LE scan...");
            let _ = self.data_sender.send(AppEvent::LogMessage(StatusMessage {
                message: "Scan stopped.".to_string(),
                severity: MessageSeverity::Info,
            }));
            watcher.Stop()?;
        }
        Ok(())
    }

    async fn send_start_command(&self, service: &GattDeviceService) -> Result<()> {
        // 查找命令特征值
        let cmd_uuid_str = {
            let settings_guard = self
                .settings
                .lock()
                .map_err(|_| anyhow::anyhow!("Failed to lock settings"))?;
            settings_guard.get().ble_command_char_uuid.clone()
        };
        let char_uuid = parse_uuid(&cmd_uuid_str)?;
        let chars_result = service.GetCharacteristicsAsync()?.await?;

        if chars_result.Status()? != GattCommunicationStatus::Success {
            return Err(anyhow::anyhow!("Failed to get command characteristic"));
        }

        let characteristics = chars_result.Characteristics()?;
        let mut cmd_char = None;

        for i in 0..characteristics.Size()? {
            let characteristic = characteristics.GetAt(i)?;
            if characteristic.Uuid()? == char_uuid {
                cmd_char = Some(characteristic);
                break;
            }
        }

        if let Some(cmd_characteristic) = cmd_char {
            // Helper to send bytes
            // Note: C# code sends these 4 init commands + optimize command
            let commands: [&[u8]; 5] = [
                &[0x01, 0x00], // CmdInit1
                &[0x06, 0x00], // CmdInit2
                &[0x07, 0x00], // CmdInit3
                &[0x08, 0x00], // CmdInit4
                &[0x0A, 0x02], // CmdOptimizeConnection
            ];

            for cmd in commands {
                let writer = DataWriter::new()?;
                writer.WriteBytes(cmd)?;
                let buffer = writer.DetachBuffer()?;
                // Write without response or with? C# uses WriteValueAsync which usually defaults to WithResponse if property set,
                // but here we just await it.
                let _ = cmd_characteristic.WriteValueAsync(&buffer)?.await?;

                // C# uses 50ms delay
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }

            info!("Initialization sequence completed successfully");
            let _ = self.data_sender.send(AppEvent::LogMessage(StatusMessage {
                message: "Ready to use".to_string(),
                severity: MessageSeverity::Success,
            }));
        }

        Ok(())
    }

    pub fn disconnect(&mut self) {
        if !self.is_connected() {
            return;
        }

        if let Some(device) = self.device.take() {
            let _ = device.Close();
        }
        self.data_characteristic = None;
        info!("Disconnected from device");
        let _ = self.data_sender.send(AppEvent::LogMessage(StatusMessage {
            message: "Disconnected from device".to_string(),
            severity: MessageSeverity::Info,
        }));
    }

    pub fn is_connected(&self) -> bool {
        self.device
            .as_ref()
            .and_then(|d| d.ConnectionStatus().ok())
            .map(|s| s == BluetoothConnectionStatus::Connected)
            .unwrap_or(false)
    }

    fn parse_controller_data(buffer: &IBuffer) -> Result<ControllerData> {
        let reader = DataReader::FromBuffer(buffer)?;
        let length = reader.UnconsumedBufferLength()? as usize;

        // 2-byte packets are command responses, not sensor data - ignore silently
        if length == 2 {
            return Err(anyhow::anyhow!(
                "Command response packet (2 bytes), not sensor data"
            ));
        }

        if length != 60 {
            debug!("Unexpected data length: {} (expected 60)", length);
            return Err(anyhow::anyhow!("Invalid data packet size: {}", length));
        }

        let mut bytes = vec![0u8; length];
        reader.ReadBytes(&mut bytes)?;

        // Parse the 60-byte data packet (Gear VR Controller protocol)
        let data = ControllerData {
            timestamp: i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64,
            accel_x: f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            accel_y: f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            accel_z: f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            gyro_x: f32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            gyro_y: f32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            gyro_z: f32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            touchpad_x: u16::from_le_bytes([bytes[54], bytes[55]]),
            touchpad_y: u16::from_le_bytes([bytes[56], bytes[57]]),
            trigger_button: (bytes[58] & 0x01) != 0,
            touchpad_button: (bytes[58] & 0x02) != 0,
            back_button: (bytes[58] & 0x04) != 0,
            home_button: (bytes[58] & 0x08) != 0,
            volume_up_button: (bytes[58] & 0x10) != 0,
            volume_down_button: (bytes[58] & 0x20) != 0,
            touchpad_touched: bytes[59] != 0,
            ..Default::default()
        };

        Ok(data)
    }
}

/// 将 UUID 字符串转换为 Windows GUID
fn parse_uuid(uuid_str: &str) -> Result<GUID> {
    // UUID 格式: "4f63756c-7573-2054-6872-65656d6f7465"
    let uuid_str = uuid_str.replace("-", "");

    if uuid_str.len() != 32 {
        return Err(anyhow::anyhow!("Invalid UUID format"));
    }

    let d1 = u32::from_str_radix(&uuid_str[0..8], 16)?;
    let d2 = u16::from_str_radix(&uuid_str[8..12], 16)?;
    let d3 = u16::from_str_radix(&uuid_str[12..16], 16)?;

    let mut d4 = [0u8; 8];
    for i in 0..8 {
        d4[i] = u8::from_str_radix(&uuid_str[16 + i * 2..18 + i * 2], 16)?;
    }

    Ok(GUID {
        data1: d1,
        data2: d2,
        data3: d3,
        data4: d4,
    })
}
