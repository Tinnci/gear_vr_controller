//! IMU (Inertial Measurement Unit) Processor
//!
//! Processes gyroscope and accelerometer data for air-mouse style control.

use super::{calibration::CalibrationFailure, motion::PixelAccumulator};
use crate::domain::{models::ControllerData, settings::Settings};
use std::collections::VecDeque;
use std::time::Duration;

/// IMU Processor for air-mouse and motion-based control
pub struct ImuProcessor {
    // Calibration offsets (gyro drift compensation)
    gyro_offset_x: f32,
    gyro_offset_y: f32,
    gyro_offset_z: f32,

    // Smoothing ring buffers with rolling sum for O(1) performance
    gyro_buffer_x: VecDeque<f32>,
    gyro_buffer_y: VecDeque<f32>,
    gyro_sum_x: f32,
    gyro_sum_y: f32,

    // Calibration state
    calibration_samples: Vec<(f32, f32, f32)>,
    is_calibrating: bool,
    calibration_target: usize,
    calibration_failure: Option<CalibrationFailure>,
    pixels: PixelAccumulator,
}

impl Default for ImuProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl ImuProcessor {
    pub fn new() -> Self {
        Self {
            gyro_offset_x: 0.0,
            gyro_offset_y: 0.0,
            gyro_offset_z: 0.0,
            gyro_buffer_x: VecDeque::with_capacity(4),
            gyro_buffer_y: VecDeque::with_capacity(4),
            gyro_sum_x: 0.0,
            gyro_sum_y: 0.0,
            calibration_samples: Vec::new(),
            is_calibrating: false,
            calibration_target: 50, // 50 samples for calibration
            calibration_failure: None,
            pixels: PixelAccumulator::default(),
        }
    }

    /// Start gyro calibration - controller should be still
    pub fn start_calibration(&mut self) {
        self.calibration_samples.clear();
        self.is_calibrating = true;
        self.calibration_failure = None;
        tracing::info!("IMU Calibration started - keep controller still");
    }

    /// Check if calibration is complete
    pub fn is_calibrating(&self) -> bool {
        self.is_calibrating
    }
    pub fn cancel_calibration(&mut self) {
        self.is_calibrating = false;
        self.calibration_samples.clear();
    }

    pub fn calibration_failure(&self) -> Option<CalibrationFailure> {
        self.calibration_failure
    }

    /// Get calibration progress (0.0 to 1.0)
    pub fn calibration_progress(&self) -> f32 {
        self.calibration_samples.len() as f32 / self.calibration_target as f32
    }

    /// Process IMU data and return mouse delta for air-mouse mode with O(1) rolling average
    pub fn calculate_airmouse_delta(
        &mut self,
        data: &ControllerData,
        settings: &Settings,
        elapsed: Duration,
    ) -> Option<(i32, i32)> {
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

        // Apply O(1) rolling average smoothing
        self.gyro_buffer_x.push_back(gyro_x);
        self.gyro_sum_x += gyro_x;
        self.gyro_buffer_y.push_back(gyro_y);
        self.gyro_sum_y += gyro_y;

        let buffer_size = if settings.enable_smoothing {
            settings.smoothing_factor.max(1)
        } else {
            1
        };
        while self.gyro_buffer_x.len() > buffer_size {
            if let Some(old_x) = self.gyro_buffer_x.pop_front() {
                self.gyro_sum_x -= old_x;
            }
            if let Some(old_y) = self.gyro_buffer_y.pop_front() {
                self.gyro_sum_y -= old_y;
            }
        }

        let count = self.gyro_buffer_x.len() as f32;
        let smoothed_x: f32 = self.gyro_sum_x / count;
        let smoothed_y: f32 = self.gyro_sum_y / count;

        // Dead zone to filter noise
        let dead_zone = settings.dead_zone as f32 * 5.0;
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
        // Preserve the legacy scale at 60 Hz, then integrate by elapsed time.
        // Bound gaps so a stalled packet cannot produce a cursor jump.
        let scale = 3000.0 * settings.air_sensitivity() * elapsed.as_secs_f64().min(0.05);

        // Map gyro axes to mouse axes
        self.pixels
            .add(f64::from(dx) * scale, f64::from(dy) * scale)
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

    /// Detect shake gesture using accelerometer (using squared magnitude to avoid sqrt)
    pub fn detect_shake(&mut self, data: &ControllerData) -> bool {
        let mag_sq =
            data.accel_x * data.accel_x + data.accel_y * data.accel_y + data.accel_z * data.accel_z;

        // Shake threshold squared (significantly above gravity 1.0^2 = 1.0, 2.5^2 = 6.25)
        const SHAKE_THRESHOLD_SQ: f32 = 2.5 * 2.5;

        mag_sq > SHAKE_THRESHOLD_SQ
    }

    /// Reset filtered motion history.
    pub fn reset_orientation(&mut self) {
        self.gyro_buffer_x.clear();
        self.gyro_buffer_y.clear();
        self.gyro_sum_x = 0.0;
        self.gyro_sum_y = 0.0;
        self.pixels.reset();
        tracing::info!("IMU filter state reset");
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
        let mean = (sum_x / count, sum_y / count, sum_z / count);
        // Reject rotation and inconsistent samples; retain the previous valid offsets.
        let moving = [mean.0, mean.1, mean.2]
            .iter()
            .any(|v| !v.is_finite() || v.abs() > 0.35)
            || self.calibration_samples.iter().any(|(x, y, z)| {
                (x - mean.0).abs() > 0.1 || (y - mean.1).abs() > 0.1 || (z - mean.2).abs() > 0.1
            });
        if moving {
            self.calibration_failure = Some(CalibrationFailure::Movement);
        } else {
            self.gyro_offset_x = mean.0;
            self.gyro_offset_y = mean.1;
            self.gyro_offset_z = mean.2;
        }

        self.is_calibrating = false;
        self.calibration_samples.clear();

        if moving {
            tracing::warn!("Gyroscope calibration rejected: controller moved");
        } else {
            tracing::info!("Gyroscope calibration complete");
        }
    }
}
