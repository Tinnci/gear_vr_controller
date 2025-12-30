//! Gear VR Controller Protocol
//!
//! This module contains the protocol definitions for communicating with
//! the Gear VR Controller

use crate::domain::models::ControllerData;
use anyhow::Result;
use tracing::{debug, trace};
use windows::core::GUID;
use windows::Storage::Streams::{DataReader, IBuffer};

/// Gear VR Controller BLE Service UUID
/// Decoded: "OculusThreemote" in ASCII (4F 63 75 6C 75 73 20 54 68 72 65 65 6D 6F 74 65)
pub const SERVICE_UUID: &str = "4f63756c-7573-2054-6872-65656d6f7465";

/// Data Receive Characteristic UUID - where sensor data is received
pub const DATA_CHAR_UUID: &str = "c8c51726-81bc-483b-a052-f7a14ea3d281";

/// Command Send Characteristic UUID - where commands are sent
pub const COMMAND_CHAR_UUID: &str = "c8c51726-81bc-483b-a052-f7a14ea3d282";

/// Controller initialization and control commands
#[derive(Debug, Clone, Copy)]
pub enum ControllerCommand {
    /// Turn all modes off and stop sending data
    Off,
    /// Sensor mode - touchpad and buttons at lower rate
    SensorMode,
    /// Initiate firmware upgrade sequence (use with caution)
    FirmwareUpgrade,
    /// Calibration mode
    Calibration,
    /// Keep-alive command
    KeepAlive,
    /// Setting mode
    SettingMode,
    /// Low Power Mode Enable
    LpmEnable,
    /// Low Power Mode Disable
    LpmDisable,
    /// VR Mode Enable - high frequency data updates
    VrModeEnable,
    /// Optimize connection parameters
    OptimizeConnection,
}

impl ControllerCommand {
    /// Get the raw bytes for this command
    pub fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::Off => &[0x00, 0x00],
            Self::SensorMode => &[0x01, 0x00],
            Self::FirmwareUpgrade => &[0x02, 0x00],
            Self::Calibration => &[0x03, 0x00],
            Self::KeepAlive => &[0x04, 0x00],
            Self::SettingMode => &[0x05, 0x00],
            Self::LpmEnable => &[0x06, 0x00],
            Self::LpmDisable => &[0x07, 0x00],
            Self::VrModeEnable => &[0x08, 0x00],
            Self::OptimizeConnection => &[0x0A, 0x02],
        }
    }
}

/// Standard initialization sequence for the controller
pub const INIT_SEQUENCE: &[(ControllerCommand, u32)] = &[
    (ControllerCommand::SensorMode, 3), // Repeat 3 times
    (ControllerCommand::LpmEnable, 1),
    (ControllerCommand::LpmDisable, 1),
    (ControllerCommand::VrModeEnable, 3), // Repeat 3 times
];

/// Delay between commands in milliseconds
pub const COMMAND_DELAY_MS: u64 = 50;

/// IMU scaling factors
/// These are approximate values - actual values may need calibration
pub mod imu_scale {
    /// Accelerometer scale (±8G range, 13-bit resolution)
    pub const ACCEL: f32 = 1.0 / 4096.0;
    /// Gyroscope scale (±2000 dps typical)
    pub const GYRO: f32 = 1.0 / 900.0;
    /// Magnetometer scale
    pub const MAG: f32 = 1.0 / 1000.0;
}

/// Parse a 60-byte data packet from the controller
///
/// # Data Packet Structure (60 bytes)
///
/// ```text
/// [0-3]   : Timestamp (u32 little-endian, milliseconds)
/// [4-5]   : Temperature or unknown (i16 little-endian)
/// [6-7]   : Reserved
///
/// IMU Data (scaled 16-bit integers):
/// [8-9]   : Accel X (i16 little-endian)
/// [10-11] : Accel Y
/// [12-13] : Accel Z
/// [14-15] : Gyro X
/// [16-17] : Gyro Y
/// [18-19] : Gyro Z
/// [20-21] : Mag X
/// [22-23] : Mag Y
/// [24-25] : Mag Z
///
/// [26-53] : Additional IMU samples or reserved
///
/// [54-55] : Touchpad X (u16, 0-315 range)
/// [56-57] : Touchpad Y (u16, 0-315 range)
/// [58]    : Button state byte
///           bit 0: Trigger
///           bit 1: Touchpad pressed
///           bit 2: Back
///           bit 3: Home
///           bit 4: Volume Up
///           bit 5: Volume Down
/// [59]    : Touchpad touched (non-zero = touching)
/// ```
pub fn parse_data_packet(buffer: &IBuffer) -> Result<ControllerData> {
    let reader = DataReader::FromBuffer(buffer)?;
    let length = reader.UnconsumedBufferLength()? as usize;

    // 2-byte packets are command responses - ignore silently
    if length == 2 {
        return Err(anyhow::anyhow!("Command response packet"));
    }

    if length != 60 {
        debug!("Unexpected data length: {} (expected 60)", length);
        return Err(anyhow::anyhow!("Invalid packet size: {}", length));
    }

    let mut bytes = vec![0u8; length];
    reader.ReadBytes(&mut bytes)?;

    // Debug logging for protocol analysis
    #[cfg(debug_assertions)]
    trace!("Raw packet: {:02X?}", &bytes);

    parse_raw_bytes(&bytes)
}

/// Parse raw bytes into ControllerData
pub fn parse_raw_bytes(bytes: &[u8]) -> Result<ControllerData> {
    if bytes.len() != 60 {
        return Err(anyhow::anyhow!("Invalid packet size: {}", bytes.len()));
    }

    // Timestamp
    let timestamp = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64;

    // Temperature (optional sensor data)
    let temperature = Some(i16::from_le_bytes([bytes[4], bytes[5]]));

    // Parse IMU as 16-bit integers
    let raw_accel_x = i16::from_le_bytes([bytes[8], bytes[9]]);
    let raw_accel_y = i16::from_le_bytes([bytes[10], bytes[11]]);
    let raw_accel_z = i16::from_le_bytes([bytes[12], bytes[13]]);

    let raw_gyro_x = i16::from_le_bytes([bytes[14], bytes[15]]);
    let raw_gyro_y = i16::from_le_bytes([bytes[16], bytes[17]]);
    let raw_gyro_z = i16::from_le_bytes([bytes[18], bytes[19]]);

    let raw_mag_x = i16::from_le_bytes([bytes[20], bytes[21]]);
    let raw_mag_y = i16::from_le_bytes([bytes[22], bytes[23]]);
    let raw_mag_z = i16::from_le_bytes([bytes[24], bytes[25]]);

    // Detect if data is 16-bit integers or 32-bit floats
    // 16-bit IMU values should be within reasonable range
    let (accel_x, accel_y, accel_z, gyro_x, gyro_y, gyro_z) =
        if raw_accel_x.abs() < 32000 && raw_accel_y.abs() < 32000 && raw_accel_z.abs() < 32000 {
            // 16-bit integer format - apply scaling
            (
                raw_accel_x as f32 * imu_scale::ACCEL,
                raw_accel_y as f32 * imu_scale::ACCEL,
                raw_accel_z as f32 * imu_scale::ACCEL,
                raw_gyro_x as f32 * imu_scale::GYRO,
                raw_gyro_y as f32 * imu_scale::GYRO,
                raw_gyro_z as f32 * imu_scale::GYRO,
            )
        } else {
            // Fallback to 32-bit float interpretation
            (
                f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
                f32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
                f32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
                f32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
                f32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
                f32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            )
        };

    // Magnetometer
    let (mag_x, mag_y, mag_z) = (
        raw_mag_x as f32 * imu_scale::MAG,
        raw_mag_y as f32 * imu_scale::MAG,
        raw_mag_z as f32 * imu_scale::MAG,
    );

    // Touchpad
    let touchpad_x = u16::from_le_bytes([bytes[54], bytes[55]]);
    let touchpad_y = u16::from_le_bytes([bytes[56], bytes[57]]);

    // Buttons
    let button_byte = bytes[58];
    let trigger_button = (button_byte & 0x01) != 0;
    let touchpad_button = (button_byte & 0x02) != 0;
    let back_button = (button_byte & 0x04) != 0;
    let home_button = (button_byte & 0x08) != 0;
    let volume_up_button = (button_byte & 0x10) != 0;
    let volume_down_button = (button_byte & 0x20) != 0;

    let touchpad_touched = bytes[59] != 0;

    Ok(ControllerData {
        timestamp,
        temperature,
        accel_x,
        accel_y,
        accel_z,
        gyro_x,
        gyro_y,
        gyro_z,
        mag_x,
        mag_y,
        mag_z,
        touchpad_x,
        touchpad_y,
        trigger_button,
        touchpad_button,
        back_button,
        home_button,
        volume_up_button,
        volume_down_button,
        touchpad_touched,
        #[cfg(debug_assertions)]
        raw_bytes: Some(bytes.to_vec()),
        ..Default::default()
    })
}

/// Parse a UUID string into a Windows GUID
pub fn parse_uuid(uuid_str: &str) -> Result<GUID> {
    let uuid_str = uuid_str.replace('-', "");

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uuid() {
        let guid = parse_uuid(SERVICE_UUID).unwrap();
        assert_eq!(guid.data1, 0x4f63756c);
    }

    #[test]
    fn test_command_bytes() {
        assert_eq!(ControllerCommand::Off.as_bytes(), &[0x00, 0x00]);
        assert_eq!(ControllerCommand::VrModeEnable.as_bytes(), &[0x08, 0x00]);
    }
}
