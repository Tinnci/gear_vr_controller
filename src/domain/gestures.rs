use crate::domain::models::ControllerData;
use crate::domain::settings::SettingsService;
use std::collections::VecDeque;
use std::f64::consts::PI;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GestureDirection {
    None,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
struct TouchpadPoint {
    x: f64,
    y: f64,
    is_touched: bool,
}

pub struct GestureRecognizer {
    settings: Arc<Mutex<SettingsService>>,
    points: VecDeque<TouchpadPoint>,
    start_point: Option<TouchpadPoint>,
    is_gesture_in_progress: bool,

    // Constants
    sample_count: usize,
    min_gesture_distance: f64,
}

impl GestureRecognizer {
    pub fn new(settings: Arc<Mutex<SettingsService>>) -> Self {
        Self {
            settings,
            points: VecDeque::new(),
            start_point: None,
            is_gesture_in_progress: false,
            sample_count: 5,
            min_gesture_distance: 0.2, // Normalized distance (range 2.0)
        }
    }

    fn get_recognition_threshold(&self) -> f64 {
        if let Ok(settings_guard) = self.settings.lock() {
            let settings = settings_guard.get();
            // Scale threshold inversely with sensitivity
            // Base sensitivity is 2.0.
            let scale_factor = settings.mouse_sensitivity.max(0.1) / 2.0;
            self.min_gesture_distance / scale_factor
        } else {
            self.min_gesture_distance
        }
    }

    pub fn process(&mut self, data: &ControllerData) -> Option<GestureDirection> {
        let point = TouchpadPoint {
            x: data.processed_touchpad_x,
            y: data.processed_touchpad_y,
            is_touched: data.touchpad_touched,
        };

        if !self.is_gesture_in_progress && point.is_touched {
            self.start_gesture(point);
            None
        } else if self.is_gesture_in_progress {
            if point.is_touched {
                self.update_gesture(point);
                None
            } else {
                self.end_gesture()
            }
        } else {
            None
        }
    }

    fn start_gesture(&mut self, point: TouchpadPoint) {
        self.start_point = Some(point);
        self.points.clear();
        self.points.push_back(point);
        self.is_gesture_in_progress = true;
    }

    fn update_gesture(&mut self, point: TouchpadPoint) {
        self.points.push_back(point);
        if self.points.len() > self.sample_count {
            self.points.pop_front();
        }
    }

    fn end_gesture(&mut self) -> Option<GestureDirection> {
        let mut result = GestureDirection::None;

        if self.points.len() >= 2 {
            if let Some(start) = self.start_point {
                // Use the last point in buffer as end point
                if let Some(end) = self.points.back() {
                    result = self.calculate_direction(start, *end);
                }
            }
        }

        self.is_gesture_in_progress = false;
        self.points.clear();

        if result != GestureDirection::None {
            Some(result)
        } else {
            None
        }
    }

    fn calculate_direction(&self, start: TouchpadPoint, end: TouchpadPoint) -> GestureDirection {
        let dx = end.x - start.x;
        // Invert Y because screen Y (down is positive) vs standard math (up is positive)?
        // Touchpad Y: 0 (top) to 315 (bottom).
        // Normalized Y: -1 (top) to 1 (bottom)?
        // Let's check normalization in controller.rs.
        // center = 157. (y - center) / range.
        // If y=0 (top), norm = -1. If y=315 (bottom), norm = 1.
        // So Y increases downwards.
        // Math.Atan2(y, x).
        // If gesture is UP (swiping from bottom to top), end.y < start.y -> dy is negative.
        // If gesture is DOWN (swiping top to bottom), dy is positive.
        let dy = end.y - start.y;

        let distance = (dx * dx + dy * dy).sqrt();

        // TODO: Get sensitivity from settings if needed to scale threshold
        let threshold = self.get_recognition_threshold();

        if distance < threshold {
            return GestureDirection::None;
        }

        let angle = dy.atan2(dx);
        let mut degrees = angle * 180.0 / PI;

        if degrees < 0.0 {
            degrees += 360.0;
        }

        // Top is -1 (start) -> 1 (end)? No.
        // UP: Swipe UP. Finger moves from Bottom (1) to Top (-1). dy < 0.
        // atan2(-1, 0) -> -90 deg -> 270 deg. Correct.

        // DOWN: Swipe DOWN. Finger moves from Top to Bottom. dy > 0.
        // atan2(1, 0) -> 90 deg. Correct.

        // RIGHT: Swipe RIGHT. Left to Right. dx > 0.
        // atan2(0, 1) -> 0 deg. Correct.

        // LEFT: Swipe LEFT. Right to Left. dx < 0.
        // atan2(0, -1) -> 180 deg. Correct.

        let tolerance = 30.0; // +/- 30 degrees (total 60 degree cone)

        if degrees >= (360.0 - tolerance) || degrees < tolerance {
            GestureDirection::Right
        } else if degrees >= (90.0 - tolerance) && degrees < (90.0 + tolerance) {
            GestureDirection::Down
        } else if degrees >= (180.0 - tolerance) && degrees < (180.0 + tolerance) {
            GestureDirection::Left
        } else if degrees >= (270.0 - tolerance) && degrees < (270.0 + tolerance) {
            GestureDirection::Up
        } else {
            GestureDirection::None
        }
    }
}
