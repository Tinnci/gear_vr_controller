use crate::models::ControllerData;
use crate::settings::SettingsService;
use std::sync::{Arc, Mutex};

pub struct TouchpadProcessor {
    settings: Arc<Mutex<SettingsService>>,
    last_touchpad_pos: Option<(u16, u16)>,
}

impl TouchpadProcessor {
    pub fn new(settings: Arc<Mutex<SettingsService>>) -> Self {
        Self {
            settings,
            last_touchpad_pos: None,
        }
    }

    /// Process raw controller data and update processed touchpad coordinates
    pub fn process(&mut self, data: &mut ControllerData) {
        let settings = self.settings.lock().unwrap();
        let calibration = &settings.get().touchpad_calibration;

        // Normalize touchpad coordinates to [-1, 1] range
        let x = data.touchpad_x;
        let y = data.touchpad_y;

        // Calculate normalized coordinates
        let center_x = calibration.center_x as f64;
        let center_y = calibration.center_y as f64;
        let range_x = (calibration.max_x - calibration.min_x) as f64 / 2.0;
        let range_y = (calibration.max_y - calibration.min_y) as f64 / 2.0;

        data.processed_touchpad_x = ((x as f64) - center_x) / range_x;
        data.processed_touchpad_y = ((y as f64) - center_y) / range_y;

        // Clamp to [-1, 1]
        data.processed_touchpad_x = data.processed_touchpad_x.clamp(-1.0, 1.0);
        data.processed_touchpad_y = data.processed_touchpad_y.clamp(-1.0, 1.0);

        self.last_touchpad_pos = Some((x, y));
    }

    /// Calculate mouse delta from touchpad movement
    pub fn calculate_mouse_delta(&mut self, data: &ControllerData) -> Option<(i32, i32)> {
        if !data.touchpad_touched {
            self.last_touchpad_pos = None;
            return None;
        }

        if let Some((last_x, last_y)) = self.last_touchpad_pos {
            let settings = self.settings.lock().unwrap();
            let sensitivity = settings.get().mouse_sensitivity;

            let dx = (data.touchpad_x as i32 - last_x as i32) as f64 * sensitivity;
            let dy = (data.touchpad_y as i32 - last_y as i32) as f64 * sensitivity;

            Some((dx as i32, dy as i32))
        } else {
            self.last_touchpad_pos = Some((data.touchpad_x, data.touchpad_y));
            None
        }
    }
}
