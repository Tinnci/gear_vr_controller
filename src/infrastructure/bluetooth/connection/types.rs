use crate::infrastructure::bluetooth::protocol;
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::GattCharacteristic;

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
}
