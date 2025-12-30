use crate::domain::models::ControllerData;
use crate::domain::settings::SettingsService;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct TouchpadProcessor {
    settings: Arc<Mutex<SettingsService>>,
    last_processed_pos: Option<(f64, f64)>,
    delta_buffer_x: VecDeque<f64>,
    delta_buffer_y: VecDeque<f64>,
}

impl TouchpadProcessor {
    pub fn new(settings: Arc<Mutex<SettingsService>>) -> Self {
        Self {
            settings,
            last_processed_pos: None,
            delta_buffer_x: VecDeque::new(),
            delta_buffer_y: VecDeque::new(),
        }
    }

    /// Process raw controller data and update processed touchpad coordinates
    pub fn process(&mut self, data: &mut ControllerData) {
        let settings = self.settings.lock().unwrap();
        let calibration = &settings.get().touchpad_calibration;

        // Reset buffers if touch ended
        if !data.touchpad_touched {
            self.last_processed_pos = None;
            self.delta_buffer_x.clear();
            self.delta_buffer_y.clear();

            // Still process coordinates for display/debug
        }

        // Normalize touchpad coordinates to [-1, 1] range
        let x = data.touchpad_x;
        let y = data.touchpad_y;

        // Calculate normalized coordinates
        let center_x = calibration.center_x as f64;
        let center_y = calibration.center_y as f64;
        let range_x = (calibration.max_x - calibration.min_x) as f64 / 2.0;
        let range_y = (calibration.max_y - calibration.min_y) as f64 / 2.0;

        // Avoid division by zero
        let range_x = if range_x == 0.0 { 1.0 } else { range_x };
        let range_y = if range_y == 0.0 { 1.0 } else { range_y };

        data.processed_touchpad_x = ((x as f64) - center_x) / range_x;
        data.processed_touchpad_y = ((y as f64) - center_y) / range_y;

        // Clamp to [-1, 1]
        data.processed_touchpad_x = data.processed_touchpad_x.clamp(-1.0, 1.0);
        data.processed_touchpad_y = data.processed_touchpad_y.clamp(-1.0, 1.0);
    }

    /// Calculate mouse delta from touchpad movement with smoothing, deadzone, and acceleration
    /// Includes Joystick behavior when holding near edges.
    pub fn calculate_mouse_delta(&mut self, data: &ControllerData) -> Option<(i32, i32)> {
        if !data.touchpad_touched {
            return None;
        }

        // Correct for controller orientation (90 degree rotation often seen in Gear VR implementations)
        // If "Top" area moves it "Left", we need to rotate.
        // Let's assume standard orientation for now but refine based on user report.
        let current_x = data.processed_touchpad_x;
        let current_y = data.processed_touchpad_y;

        let mut total_dx = 0.0;
        let mut total_dy = 0.0;

        let settings_guard = self.settings.lock().unwrap();
        let settings = settings_guard.get();
        let sensitivity = settings.mouse_sensitivity;

        // 1. RELATIVE MOVEMENT (Trackpad Mode)
        if let Some((last_x, last_y)) = self.last_processed_pos {
            let mut rel_dx = current_x - last_x;
            let mut rel_dy = current_y - last_y;

            // Apply Smoothing to relative movement
            if settings.enable_smoothing {
                self.delta_buffer_x.push_back(rel_dx);
                self.delta_buffer_y.push_back(rel_dy);
                while self.delta_buffer_x.len() > settings.smoothing_factor {
                    self.delta_buffer_x.pop_front();
                    self.delta_buffer_y.pop_front();
                }
                rel_dx = self.delta_buffer_x.iter().sum::<f64>() / self.delta_buffer_x.len() as f64;
                rel_dy = self.delta_buffer_y.iter().sum::<f64>() / self.delta_buffer_y.len() as f64;
            }

            // Apply Acceleration
            if settings.enable_acceleration {
                let power = settings.acceleration_power;
                rel_dx = rel_dx.signum() * rel_dx.abs().powf(power);
                rel_dy = rel_dy.signum() * rel_dy.abs().powf(power);
            }

            let scale_factor = 800.0; // Adjusted for sensitivity
            total_dx += rel_dx * sensitivity * scale_factor;
            total_dy += rel_dy * sensitivity * scale_factor;
        }
        self.last_processed_pos = Some((current_x, current_y));

        // 2. ABSOLUTE MOVEMENT (Joystick Mode)
        // If finger is held near the edges (abs > 0.7), add continuous movement
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
