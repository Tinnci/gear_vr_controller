//! IMU (Inertial Measurement Unit) Processor
//!
//! Processes gyroscope and accelerometer data for air-mouse style control.

use crate::domain::models::ControllerData;
use crate::domain::settings::SettingsService;
use std::sync::{Arc, Mutex};

/// IMU Processor for air-mouse and motion-based control
pub struct ImuProcessor {
    settings: Arc<Mutex<SettingsService>>,

    // Calibration offsets (gyro drift compensation)
    gyro_offset_x: f32,
    gyro_offset_y: f32,
    gyro_offset_z: f32,

    // Accumulated rotation for absolute positioning (optional)
    accumulated_yaw: f32,
    accumulated_pitch: f32,

    // Smoothing buffers
    gyro_buffer_x: Vec<f32>,
    gyro_buffer_y: Vec<f32>,
    buffer_size: usize,

    // Calibration state
    calibration_samples: Vec<(f32, f32, f32)>,
    is_calibrating: bool,
    calibration_target: usize,
}

impl ImuProcessor {
    pub fn new(settings: Arc<Mutex<SettingsService>>) -> Self {
        Self {
            settings,
            gyro_offset_x: 0.0,
            gyro_offset_y: 0.0,
            gyro_offset_z: 0.0,
            accumulated_yaw: 0.0,
            accumulated_pitch: 0.0,
            gyro_buffer_x: Vec::new(),
            gyro_buffer_y: Vec::new(),
            buffer_size: 3,
            calibration_samples: Vec::new(),
            is_calibrating: false,
            calibration_target: 50, // 50 samples for calibration
        }
    }

    /// Start gyro calibration - controller should be still
    pub fn start_calibration(&mut self) {
        self.calibration_samples.clear();
        self.is_calibrating = true;
        tracing::info!("IMU Calibration started - keep controller still");
    }

    /// Check if calibration is complete
    pub fn is_calibrating(&self) -> bool {
        self.is_calibrating
    }

    /// Get calibration progress (0.0 to 1.0)
    pub fn calibration_progress(&self) -> f32 {
        self.calibration_samples.len() as f32 / self.calibration_target as f32
    }

    /// Process IMU data and return mouse delta for air-mouse mode
    pub fn calculate_airmouse_delta(&mut self, data: &ControllerData) -> Option<(i32, i32)> {
        // Handle calibration
        if self.is_calibrating {
            self.calibration_samples
                .push((data.gyro_x, data.gyro_y, data.gyro_z));

            if self.calibration_samples.len() >= self.calibration_target {
                self.finish_calibration();
            }
            return None;
        }

        // Apply calibration offset
        let gyro_x = data.gyro_x - self.gyro_offset_x;
        let gyro_y = data.gyro_y - self.gyro_offset_y;
        let _gyro_z = data.gyro_z - self.gyro_offset_z;

        // For air-mouse:
        // - Gyro Y (pitch) controls vertical mouse movement
        // - Gyro Z (yaw) controls horizontal mouse movement
        // Controller orientation matters - adjust mapping based on how user holds it

        // Get sensitivity from settings
        let sensitivity = {
            let s = self.settings.lock().unwrap();
            s.get().mouse_sensitivity
        };

        // Apply smoothing
        self.gyro_buffer_x.push(gyro_x);
        self.gyro_buffer_y.push(gyro_y);

        while self.gyro_buffer_x.len() > self.buffer_size {
            self.gyro_buffer_x.remove(0);
            self.gyro_buffer_y.remove(0);
        }

        let smoothed_x: f32 =
            self.gyro_buffer_x.iter().sum::<f32>() / self.gyro_buffer_x.len() as f32;
        let smoothed_y: f32 =
            self.gyro_buffer_y.iter().sum::<f32>() / self.gyro_buffer_y.len() as f32;

        // Dead zone to filter noise
        let dead_zone = 0.5; // Adjust based on gyro noise level
        let dx = if smoothed_x.abs() > dead_zone {
            smoothed_x
        } else {
            0.0
        };
        let dy = if smoothed_y.abs() > dead_zone {
            smoothed_y
        } else {
            0.0
        };

        if dx.abs() < 0.01 && dy.abs() < 0.01 {
            return None;
        }

        // Scale factor for converting gyro units to pixels
        // Gyro values are in radians/second after scaling
        // Typical gyro range: -2000 to +2000 deg/s raw, scaled down
        let scale = 50.0 * sensitivity as f32;

        // Map gyro axes to mouse axes
        // This mapping may need adjustment based on controller orientation
        let mouse_dx = (dx * scale) as i32;
        let mouse_dy = (dy * scale) as i32;

        Some((mouse_dx, mouse_dy))
    }

    /// Process IMU for tilt-based scrolling
    pub fn calculate_tilt_scroll(&mut self, data: &ControllerData) -> Option<i32> {
        // Use accelerometer to detect tilt
        // When tilted forward/backward, scroll up/down

        let accel_y = data.accel_y;

        // Tilt threshold (gravity component when tilted)
        let tilt_threshold = 0.3;
        let scroll_speed = 1;

        if accel_y > tilt_threshold {
            Some(scroll_speed) // Scroll up
        } else if accel_y < -tilt_threshold {
            Some(-scroll_speed) // Scroll down
        } else {
            None
        }
    }

    /// Detect shake gesture using accelerometer
    pub fn detect_shake(&mut self, data: &ControllerData) -> bool {
        // Calculate acceleration magnitude
        let magnitude = (data.accel_x * data.accel_x
            + data.accel_y * data.accel_y
            + data.accel_z * data.accel_z)
            .sqrt();

        // Shake threshold (significantly above gravity ~1.0)
        let shake_threshold = 2.5;

        magnitude > shake_threshold
    }

    /// Reset accumulated rotation (re-center)
    pub fn reset_orientation(&mut self) {
        self.accumulated_yaw = 0.0;
        self.accumulated_pitch = 0.0;
        tracing::info!("IMU orientation reset");
    }

    fn finish_calibration(&mut self) {
        if self.calibration_samples.is_empty() {
            self.is_calibrating = false;
            return;
        }

        // Calculate average offset
        let count = self.calibration_samples.len() as f32;
        let sum_x: f32 = self.calibration_samples.iter().map(|(x, _, _)| x).sum();
        let sum_y: f32 = self.calibration_samples.iter().map(|(_, y, _)| y).sum();
        let sum_z: f32 = self.calibration_samples.iter().map(|(_, _, z)| z).sum();

        self.gyro_offset_x = sum_x / count;
        self.gyro_offset_y = sum_y / count;
        self.gyro_offset_z = sum_z / count;

        self.is_calibrating = false;
        self.calibration_samples.clear();

        tracing::info!(
            "IMU Calibration complete. Offsets: ({:.4}, {:.4}, {:.4})",
            self.gyro_offset_x,
            self.gyro_offset_y,
            self.gyro_offset_z
        );
    }
}
