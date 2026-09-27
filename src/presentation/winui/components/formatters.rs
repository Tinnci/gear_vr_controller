//! Pure formatting presenters for telemetry and diagnostic sensor feeds

use crate::domain::i18n::I18nStrings;
use crate::domain::models::ControllerData;

/// Formats real-time touchpad coordinates, button states, and timing metadata
pub fn format_telemetry(
    data: Option<&ControllerData>,
    s: &I18nStrings,
) -> (String, String, String) {
    if let Some(d) = data {
        let tp = format!(
            "Touchpad: Normalized (X: {:+.3}, Y: {:+.3}) | Raw: ({}, {})",
            d.processed_touchpad_x, d.processed_touchpad_y, d.touchpad_x, d.touchpad_y
        );
        let btn = format!(
            "Buttons: Trigger: {} | Back: {} | Home: {} | Touchpad: {} | Vol+: {} | Vol-: {}",
            if d.trigger_button {
                "Active"
            } else {
                "Inactive"
            },
            if d.back_button { "Active" } else { "Inactive" },
            if d.home_button { "Active" } else { "Inactive" },
            if d.touchpad_button {
                "Active"
            } else {
                "Inactive"
            },
            if d.volume_up_button {
                "Active"
            } else {
                "Inactive"
            },
            if d.volume_down_button {
                "Active"
            } else {
                "Inactive"
            },
        );
        let sample = format!("Timestamp: {} ms | Status: OK", d.timestamp);
        (tp, btn, sample)
    } else {
        (
            s.telemetry_awaiting.to_string(),
            s.buttons_idle.to_string(),
            s.timestamp_no_tx.to_string(),
        )
    }
}

/// Formats 9-axis IMU raw telemetry with standard physical engineering units
pub fn format_imu_diagnostics(data: Option<&ControllerData>) -> (String, String, String) {
    if let Some(d) = data {
        (
            format!(
                "Accelerometer (g):     X: {:+.4} | Y: {:+.4} | Z: {:+.4}",
                d.accel_x, d.accel_y, d.accel_z
            ),
            format!(
                "Gyroscope (rad/s):       X: {:+.4} | Y: {:+.4} | Z: {:+.4}",
                d.gyro_x, d.gyro_y, d.gyro_z
            ),
            format!(
                "Magnetometer (uT):       X: {:+.4} | Y: {:+.4} | Z: {:+.4}",
                d.mag_x, d.mag_y, d.mag_z
            ),
        )
    } else {
        (
            "Accelerometer (g):     Waiting for data...".to_string(),
            "Gyroscope (rad/s):       Waiting for data...".to_string(),
            "Magnetometer (uT):       Waiting for data...".to_string(),
        )
    }
}
