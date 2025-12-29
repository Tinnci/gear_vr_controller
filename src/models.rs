use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub struct ControllerData {
    // Accelerometer data
    pub accel_x: f32,
    pub accel_y: f32,
    pub accel_z: f32,

    // Gyroscope data
    pub gyro_x: f32,
    pub gyro_y: f32,
    pub gyro_z: f32,

    // Button states
    pub trigger_button: bool,
    pub home_button: bool,
    pub back_button: bool,
    pub touchpad_button: bool,
    pub touchpad_touched: bool,
    pub volume_up_button: bool,
    pub volume_down_button: bool,

    // Raw touchpad coordinates
    pub touchpad_x: u16,
    pub touchpad_y: u16,

    // Processed touchpad coordinates (normalized to [-1, 1])
    pub processed_touchpad_x: f64,
    pub processed_touchpad_y: f64,

    // Timestamp (Unix milliseconds)
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ControllerData(ControllerData),
    ConnectionStatus(ConnectionStatus),
    LogMessage(StatusMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TouchpadCalibration {
    pub min_x: u16,
    pub max_x: u16,
    pub min_y: u16,
    pub max_y: u16,
    pub center_x: u16,
    pub center_y: u16,
}

impl Default for TouchpadCalibration {
    fn default() -> Self {
        Self {
            min_x: 0,
            max_x: 315,
            min_y: 0,
            max_y: 315,
            center_x: 157,
            center_y: 157,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub message: String,
    pub severity: MessageSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSeverity {
    Info,
    Success,
    Warning,
    Error,
}
