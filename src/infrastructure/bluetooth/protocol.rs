//! Gear VR Controller Protocol
//!
//! This module contains the protocol definitions for communicating with
//! the Gear VR Controller

use crate::domain::models::ControllerData;
use anyhow::Result;
use tracing::debug;
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
#[allow(dead_code)]
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

/// IMU scaling factors from decompiled Samsung APK
/// Based on: com.samsung.android.app.vr.input.service/ui/c.class
#[allow(dead_code)]
pub mod imu_scale {
    /// Accelerometer combined scale factor: value * 10000.0 * 9.80665 / 2048.0 * 0.00001
    pub const ACCEL_SCALE: f32 = 10000.0 * 9.80665 / 2048.0 * 0.00001;
    pub const ACCEL_RAW: f32 = 10000.0 * 9.80665 / 2048.0;
    pub const ACCEL_FACTOR: f32 = 0.00001;

    /// Gyroscope combined scale factor: value * 10000.0 * 0.017453292 / 14.285 * 0.0001
    pub const GYRO_SCALE: f32 = 10000.0 * 0.017453292 / 14.285 * 0.0001;
    pub const GYRO_RAW: f32 = 10000.0 * 0.017453292 / 14.285;
    pub const GYRO_FACTOR: f32 = 0.0001;

    /// Magnetometer: value * 0.06
    pub const MAG: f32 = 0.06;
}

/// Parse a 60-byte data packet from the controller
///
/// # Data Packet Structure (60 bytes)
///
/// ```text
/// [0-3]   : Timestamp (u32 little-endian, milliseconds)
/// [4-9]   : Accelerometer XYZ (i16 little-endian)
/// [10-15] : Gyroscope XYZ (i16 little-endian)
///
/// [16-31] : Additional IMU samples
/// [32-37] : Magnetometer XYZ (i16 little-endian)
///
/// [38-53] : Additional samples / reserved
///
/// [54-56] : Packed 10-bit touchpad X and Y; typical calibrated range 0-315
/// [57]    : Temperature byte
/// [58]    : Button state byte
///           bit 0: Trigger
///           bit 1: Home
///           bit 2: Back
///           bit 3: Touchpad pressed
///           bit 4: Volume Up
///           bit 5: Volume Down
/// [59]    : Reserved; touch is inferred from nonzero coordinates
/// ```
pub fn parse_data_packet(buffer: &IBuffer) -> Result<ControllerData> {
    let reader = DataReader::FromBuffer(buffer)?;
    let length = reader.UnconsumedBufferLength()? as usize;

    // 2-byte packets are command responses - ignore silently
    if length == 2 {
        return Err(anyhow::anyhow!("Command response packet"));
    }

    if length != 60 {
        debug!(
            event = "ble.packet.invalid",
            actual_bytes = length,
            expected_bytes = 60,
            "Controller packet rejected"
        );
        return Err(anyhow::anyhow!("Invalid packet size: {}", length));
    }

    // Zero-heap-allocation: read directly into a stack-allocated 60-byte buffer
    let mut bytes = [0u8; 60];
    reader.ReadBytes(&mut bytes)?;

    parse_raw_bytes(&bytes)
}

/// Parse raw bytes into ControllerData
pub fn parse_raw_bytes(bytes: &[u8]) -> Result<ControllerData> {
    if bytes.len() != 60 {
        return Err(anyhow::anyhow!("Invalid packet size: {}", bytes.len()));
    }

    // Timestamp
    let timestamp = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as i64;

    // Temperature (byte 57 according to JS implementation)
    let temperature = Some(bytes[57] as i16);

    // Parse IMU as 16-bit integers using Samsung APK formula
    // Accelerometer at bytes 4-9 (3 samples at offsets 0, 16, 32 - we use first)
    // JS: getAccelerometerFloatWithOffsetFromArrayBufferAtIndex(buffer, 4/6/8, 0)
    let raw_accel_x = i16::from_le_bytes([bytes[4], bytes[5]]);
    let raw_accel_y = i16::from_le_bytes([bytes[6], bytes[7]]);
    let raw_accel_z = i16::from_le_bytes([bytes[8], bytes[9]]);

    // Gyroscope at bytes 10-15
    let raw_gyro_x = i16::from_le_bytes([bytes[10], bytes[11]]);
    let raw_gyro_y = i16::from_le_bytes([bytes[12], bytes[13]]);
    let raw_gyro_z = i16::from_le_bytes([bytes[14], bytes[15]]);

    // Apply precomputed scaling constants (single float multiplication per axis)
    let accel_x = raw_accel_x as f32 * imu_scale::ACCEL_SCALE;
    let accel_y = raw_accel_y as f32 * imu_scale::ACCEL_SCALE;
    let accel_z = raw_accel_z as f32 * imu_scale::ACCEL_SCALE;

    let gyro_x = raw_gyro_x as f32 * imu_scale::GYRO_SCALE;
    let gyro_y = raw_gyro_y as f32 * imu_scale::GYRO_SCALE;
    let gyro_z = raw_gyro_z as f32 * imu_scale::GYRO_SCALE;

    // Magnetometer at bytes 32-37 (JS: offset 32 + 0/2/4)
    let raw_mag_x = i16::from_le_bytes([bytes[32], bytes[33]]);
    let raw_mag_y = i16::from_le_bytes([bytes[34], bytes[35]]);
    let raw_mag_z = i16::from_le_bytes([bytes[36], bytes[37]]);

    let mag_x = raw_mag_x as f32 * imu_scale::MAG;
    let mag_y = raw_mag_y as f32 * imu_scale::MAG;
    let mag_z = raw_mag_z as f32 * imu_scale::MAG;

    // Touchpad coordinates - CORRECTED based on JS reference implementation
    // The touchpad data is packed across multiple bytes using bit operations
    // Max observed value = 315 (touchpad dimension in mm)
    // JS: axisX = (((eventData[54] & 0xF) << 6) + ((eventData[55] & 0xFC) >> 2)) & 0x3FF
    // JS: axisY = (((eventData[55] & 0x3) << 8) + ((eventData[56] & 0xFF) >> 0)) & 0x3FF
    let touchpad_x =
        ((((bytes[54] & 0x0F) as u16) << 6) + (((bytes[55] & 0xFC) as u16) >> 2)) & 0x3FF;
    let touchpad_y = ((((bytes[55] & 0x03) as u16) << 8) + bytes[56] as u16) & 0x3FF;

    // Button states - CORRECTED based on JS reference implementation
    // JS mapping: trigger=bit0, home=bit1, back=bit2, touchpad=bit3, volUp=bit4, volDown=bit5
    let button_byte = bytes[58];
    let trigger_button = (button_byte & (1 << 0)) != 0;
    let home_button = (button_byte & (1 << 1)) != 0;
    let back_button = (button_byte & (1 << 2)) != 0;
    let touchpad_button = (button_byte & (1 << 3)) != 0;
    let volume_up_button = (button_byte & (1 << 4)) != 0;
    let volume_down_button = (button_byte & (1 << 5)) != 0;

    // Touchpad touched state (byte 59 is not used in JS, but we check it anyway)
    // Note: In JS, temperature is at byte 57, and touchpad touch state might be implicit
    let touchpad_touched = touchpad_x > 0 || touchpad_y > 0;

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

    if uuid_str.len() != 32 || !uuid_str.bytes().all(|byte| byte.is_ascii_hexdigit()) {
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
    fn test_parse_uuid() -> Result<()> {
        let guid = parse_uuid(SERVICE_UUID)?;
        assert_eq!(guid.data1, 0x4f63756c);
        Ok(())
    }

    #[test]
    fn test_command_bytes() {
        assert_eq!(ControllerCommand::Off.as_bytes(), &[0x00, 0x00]);
        assert_eq!(ControllerCommand::VrModeEnable.as_bytes(), &[0x08, 0x00]);
    }

    #[test]
    fn packet_decodes_signed_imu_packed_coordinates_and_buttons() -> Result<()> {
        let mut bytes = [0u8; 60];
        bytes[..4].copy_from_slice(&1234u32.to_le_bytes());
        bytes[4..6].copy_from_slice(&(-2048i16).to_le_bytes());
        bytes[10..12].copy_from_slice(&100i16.to_le_bytes());
        bytes[54] = 4;
        bytes[55] = 1;
        bytes[56] = 59;
        bytes[57] = 25;
        bytes[58] = 0b0010_1011;
        let data = parse_raw_bytes(&bytes)?;
        assert_eq!(data.timestamp, 1234);
        assert_eq!((data.touchpad_x, data.touchpad_y), (256, 315));
        assert!(data.accel_x < 0.0 && data.gyro_x > 0.0);
        assert!(
            data.trigger_button
                && data.home_button
                && data.touchpad_button
                && data.volume_down_button
        );
        assert!(!data.back_button && !data.volume_up_button);
        assert_eq!(data.temperature, Some(25));
        assert!(data.touchpad_touched);
        Ok(())
    }

    #[test]
    fn invalid_packets_and_non_ascii_uuids_are_errors() {
        for length in [0, 2, 59, 61] {
            assert!(parse_raw_bytes(&vec![0; length]).is_err());
        }
        assert!(parse_uuid("€€€€€€€€€€ab").is_err());
        assert!(parse_uuid("z0000000000000000000000000000000").is_err());
    }
}
