use crate::domain::models::{ControllerData, TouchpadCalibration};
use crate::domain::settings::Settings;
use std::collections::VecDeque;

pub struct TouchpadProcessor {
    pub last_processed_pos: Option<(f64, f64)>,
    delta_buffer_x: VecDeque<f64>,
    delta_buffer_y: VecDeque<f64>,
    delta_sum_x: f64,
    delta_sum_y: f64,
}

impl Default for TouchpadProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl TouchpadProcessor {
    pub fn new() -> Self {
        Self {
            last_processed_pos: None,
            delta_buffer_x: VecDeque::new(),
            delta_buffer_y: VecDeque::new(),
            delta_sum_x: 0.0,
            delta_sum_y: 0.0,
        }
    }

    /// Process raw controller data and update processed touchpad coordinates without locking
    pub fn process(&mut self, data: &mut ControllerData, calibration: &TouchpadCalibration) {
        // Reset buffers if touch ended
        if !data.touchpad_touched {
            self.last_processed_pos = None;
            self.delta_buffer_x.clear();
            self.delta_buffer_y.clear();
            self.delta_sum_x = 0.0;
            self.delta_sum_y = 0.0;
        }

        // Normalize touchpad coordinates to [-1, 1] range
        let x = data.touchpad_x;
        let y = data.touchpad_y;

        // Calculate normalized coordinates
        let center_x = calibration.center_x as f64;
        let center_y = calibration.center_y as f64;
        let range_x = (calibration.max_x.saturating_sub(calibration.min_x)) as f64 / 2.0;
        let range_y = (calibration.max_y.saturating_sub(calibration.min_y)) as f64 / 2.0;

        // Avoid division by zero
        let range_x = if range_x == 0.0 { 1.0 } else { range_x };
        let range_y = if range_y == 0.0 { 1.0 } else { range_y };

        data.processed_touchpad_x = (((x as f64) - center_x) / range_x).clamp(-1.0, 1.0);
        data.processed_touchpad_y = (((y as f64) - center_y) / range_y).clamp(-1.0, 1.0);
    }

    /// Calculate mouse delta from touchpad movement with O(1) rolling average smoothing, deadzone, and acceleration
    /// Includes Joystick behavior when holding near edges.
    pub fn calculate_mouse_delta(
        &mut self,
        data: &ControllerData,
        settings: &Settings,
    ) -> Option<(i32, i32)> {
        if !data.touchpad_touched {
            return None;
        }

        let current_x = data.processed_touchpad_x;
        let current_y = data.processed_touchpad_y;

        let mut total_dx = 0.0;
        let mut total_dy = 0.0;

        let sensitivity = settings.mouse_sensitivity;

        // 1. RELATIVE MOVEMENT (Trackpad Mode)
        if let Some((last_x, last_y)) = self.last_processed_pos {
            let mut rel_dx = current_x - last_x;
            let mut rel_dy = current_y - last_y;

            // Apply O(1) Smoothing to relative movement
            if settings.enable_smoothing {
                self.delta_buffer_x.push_back(rel_dx);
                self.delta_sum_x += rel_dx;
                self.delta_buffer_y.push_back(rel_dy);
                self.delta_sum_y += rel_dy;

                while self.delta_buffer_x.len() > settings.smoothing_factor {
                    if let Some(old_x) = self.delta_buffer_x.pop_front() {
                        self.delta_sum_x -= old_x;
                    }
                    if let Some(old_y) = self.delta_buffer_y.pop_front() {
                        self.delta_sum_y -= old_y;
                    }
                }

                let count = self.delta_buffer_x.len() as f64;
                if count > 0.0 {
                    rel_dx = self.delta_sum_x / count;
                    rel_dy = self.delta_sum_y / count;
                }
            }

            // Apply Acceleration (fast-path for common power curve exponents)
            if settings.enable_acceleration {
                let power = settings.acceleration_power;
                let abs_x = rel_dx.abs();
                let abs_y = rel_dy.abs();

                let accel_x = if (power - 1.0).abs() < 1e-4 {
                    abs_x
                } else if (power - 2.0).abs() < 1e-4 {
                    abs_x * abs_x
                } else {
                    abs_x.powf(power)
                };

                let accel_y = if (power - 1.0).abs() < 1e-4 {
                    abs_y
                } else if (power - 2.0).abs() < 1e-4 {
                    abs_y * abs_y
                } else {
                    abs_y.powf(power)
                };

                rel_dx = rel_dx.signum() * accel_x;
                rel_dy = rel_dy.signum() * accel_y;
            }

            let scale_factor = 800.0; // Adjusted for sensitivity
            total_dx += rel_dx * sensitivity * scale_factor;
            total_dy += rel_dy * sensitivity * scale_factor;
        }
        self.last_processed_pos = Some((current_x, current_y));

        // 2. ABSOLUTE MOVEMENT (Joystick Mode)
        // If finger is held near the edges (abs > 0.6), add continuous movement
        let joy_threshold = 0.6;
        let joy_speed = 5.0; // Base speed for continuous movement

        if current_x.abs() > joy_threshold {
            total_dx +=
                current_x.signum() * (current_x.abs() - joy_threshold) * joy_speed * sensitivity;
        }
        if current_y.abs() > joy_threshold {
            total_dy +=
                current_y.signum() * (current_y.abs() - joy_threshold) * joy_speed * sensitivity;
        }

        if total_dx.abs() < 0.1 && total_dy.abs() < 0.1 {
            return None;
        }

        Some((total_dx as i32, total_dy as i32))
    }
}
