use crate::models::ControllerData;
use crate::settings::SettingsService;
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
    pub fn calculate_mouse_delta(&mut self, data: &ControllerData) -> Option<(i32, i32)> {
        if !data.touchpad_touched {
            return None;
        }

        let current_x = data.processed_touchpad_x;
        let current_y = data.processed_touchpad_y;

        if let Some((last_x, last_y)) = self.last_processed_pos {
            let settings_guard = self.settings.lock().unwrap();
            let settings = settings_guard.get();

            let mut dx = current_x - last_x;
            let mut dy = current_y - last_y;

            // 1. Apply Dead Zone (to the delta magnitude)
            // Note: Dead zone on delta prevents small jitters from causing movement
            let magnitude = (dx * dx + dy * dy).sqrt();
            if magnitude < settings.dead_zone / 100.0 {
                // Assuming settings.dead_zone is 0.0-1.0 or similar
                // Wait, in C# it was settings.dead_zone / 100.0 (percentage?)
                // In Rust settings initialized to 0.1 (10%?) or 0.1 as value?
                // Settings default is 0.1. Let's assume it's raw value 0.1 normalized unit?
                // Normalized unit is -1 to 1.
                // C#: deadZoneThreshold = _settingsService.DeadZone / 100.0;
                // Since I set default to 0.1 in Rust, let's treat it as the threshold directly for now
                // actually 0.1 is quite large for delta (range is 2.0).
                // A single pixel might be very small.
                // Let's stick to C# logic: if DeadZone is exposed as 0-100, then /100.
                // But I defined it as f64. Let's treat it as absolute normalized threshold.
                // dx = 0.0;
                // dy = 0.0;
                // If it's below deadzone, we ignore this movement but DO NOT update last_processed_pos?
                // C# retuns (0,0).
                return None;
            }

            // 2. Apply Smoothing
            if settings.enable_smoothing {
                self.delta_buffer_x.push_back(dx);
                self.delta_buffer_y.push_back(dy);

                while self.delta_buffer_x.len() > settings.smoothing_factor {
                    self.delta_buffer_x.pop_front();
                    self.delta_buffer_y.pop_front();
                }

                dx = self.delta_buffer_x.iter().sum::<f64>() / self.delta_buffer_x.len() as f64;
                dy = self.delta_buffer_y.iter().sum::<f64>() / self.delta_buffer_y.len() as f64;
            } else {
                self.delta_buffer_x.clear();
                self.delta_buffer_y.clear();
            }

            // 3. Apply Acceleration (Non-Linear Curve)
            if settings.enable_acceleration {
                let power = settings.acceleration_power;
                dx = dx.signum() * dx.abs().powf(power);
                dy = dy.signum() * dy.abs().powf(power);
            }

            // 4. Scale to pixels
            // Sensitivity needs to be high because dx is small (normalized units)
            // C# Scaling: delta * Sensitivity * ScalingFactor
            // Normalized range 2.0. Screen 1920.
            // If move full width: 2.0. Want 1920 pixels? scale ~ 1000.
            let scale_factor = 1000.0;
            let sensitivity = settings.mouse_sensitivity;

            let pixel_dx = dx * sensitivity * scale_factor;
            let pixel_dy = dy * sensitivity * scale_factor;

            self.last_processed_pos = Some((current_x, current_y));

            if pixel_dx.abs() < 1.0 && pixel_dy.abs() < 1.0 {
                return None;
            }

            Some((pixel_dx as i32, pixel_dy as i32))
        } else {
            // First touch frame
            self.last_processed_pos = Some((current_x, current_y));
            None
        }
    }
}
