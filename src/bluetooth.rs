use crate::models::{AppEvent, ConnectionStatus, ControllerData, ScannedDevice};
use anyhow::Result;
use log::{debug, info, warn};
use tokio::sync::mpsc;
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
const CONTROLLER_SERVICE_UUID: &str = "4f63756c-7573-2054-6872-65656d6f7465";
const CONTROLLER_DATA_CHARACTERISTIC_UUID: &str = "c8c51726-81bc-483b-a052-f7a14ea3d281";
const CONTROLLER_COMMAND_CHARACTERISTIC_UUID: &str = "c8c51726-81bc-483b-a052-f7a14ea3d282";

pub struct BluetoothService {
    device: Option<BluetoothLEDevice>,
    data_characteristic: Option<GattCharacteristic>,
    data_sender: mpsc::UnboundedSender<AppEvent>,
    watcher: Option<BluetoothLEAdvertisementWatcher>,
}

impl BluetoothService {
    pub fn new(data_sender: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self {
            device: None,
            data_characteristic: None,
            data_sender,
            watcher: None,
        }
    }

    pub async fn connect(&mut self, address: u64) -> Result<()> {
        info!("Connecting to Bluetooth device: {:#X}", address);

        // 1. 连接到 BLE 设备 (正确使用 WinRT async/await)
        let device_async = BluetoothLEDevice::FromBluetoothAddressAsync(address)?;
        let device = device_async.await?;

        info!("Device connected: {:?}", device.Name()?);

        // 2. 获取 GATT 服务
        let services_result = device.GetGattServicesAsync()?.await?;

        if services_result.Status()? != GattCommunicationStatus::Success {
            anyhow::bail!("Failed to get GATT services");
        }

        // 3. 查找控制器服务 - use block scope to drop services before await
        let service = {
            let services = services_result.Services()?;
            info!("Found {} services", services.Size()?);

            let service_uuid = parse_uuid(CONTROLLER_SERVICE_UUID)?;
            let mut target_service = None;

            for i in 0..services.Size()? {
                let svc = services.GetAt(i)?;
                if svc.Uuid()? == service_uuid {
                    target_service = Some(svc);
                    break;
                }
            }

            target_service.ok_or_else(|| anyhow::anyhow!("Controller service not found"))?
        };
        info!("Found controller service");

        // 4. 获取特征值
        let chars_result = service.GetCharacteristicsAsync()?.await?;

        if chars_result.Status()? != GattCommunicationStatus::Success {
            anyhow::bail!("Failed to get characteristics");
        }

        let characteristics = chars_result.Characteristics()?;
        info!("Found {} characteristics", characteristics.Size()?);

        // 5. 查找数据特征值
        let char_uuid = parse_uuid(CONTROLLER_DATA_CHARACTERISTIC_UUID)?;
        let mut data_char = None;

        for i in 0..characteristics.Size()? {
            let characteristic = characteristics.GetAt(i)?;
            if characteristic.Uuid()? == char_uuid {
                data_char = Some(characteristic);
                break;
            }
        }

        let data_characteristic =
            data_char.ok_or_else(|| anyhow::anyhow!("Data characteristic not found"))?;
        info!("Found data characteristic");

        // 6. 订阅通知 (使用 TypedEventHandler)
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

        // 7. 启用通知
        let status = data_characteristic
            .WriteClientCharacteristicConfigurationDescriptorAsync(
                GattClientCharacteristicConfigurationDescriptorValue::Notify,
            )?
            .await?;

        if status != GattCommunicationStatus::Success {
            anyhow::bail!("Failed to enable notifications");
        }

        info!("Notifications enabled successfully");

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

        info!("Starting Bluetooth LE scan...");
        let watcher = BluetoothLEAdvertisementWatcher::new()?;
        watcher.SetScanningMode(BluetoothLEScanningMode::Active)?;

        let sender = self.data_sender.clone();
        let service_uuid = parse_uuid(CONTROLLER_SERVICE_UUID)?;

        let handler = TypedEventHandler::new(
            move |_: windows::core::Ref<BluetoothLEAdvertisementWatcher>,
                  args: windows::core::Ref<BluetoothLEAdvertisementReceivedEventArgs>| {
                if let Some(args) = args.as_ref() {
                    let adv = args.Advertisement()?;
                    let service_uuids = adv.ServiceUuids()?;

                    let mut found = false;
                    for i in 0..service_uuids.Size()? {
                        if service_uuids.GetAt(i)? == service_uuid {
                            found = true;
                            break;
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
            watcher.Stop()?;
        }
        Ok(())
    }

    async fn send_start_command(&self, service: &GattDeviceService) -> Result<()> {
        // 查找命令特征值
        let char_uuid = parse_uuid(CONTROLLER_COMMAND_CHARACTERISTIC_UUID)?;
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
        }

        Ok(())
    }

    pub fn disconnect(&mut self) {
        if let Some(device) = self.device.take() {
            let _ = device.Close();
        }
        self.data_characteristic = None;
        info!("Disconnected from device");
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

        if length != 60 {
            debug!("Unexpected data length: {}", length);
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
