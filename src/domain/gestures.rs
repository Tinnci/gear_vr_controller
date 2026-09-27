use crate::domain::models::ControllerData;
use std::collections::VecDeque;
use std::f64::consts::PI;
use tracing::{debug, trace};

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
    points: VecDeque<TouchpadPoint>,
    start_point: Option<TouchpadPoint>,
    is_gesture_in_progress: bool,

    // Constants
    sample_count: usize,
    min_gesture_distance: f64,
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl GestureRecognizer {
    pub fn new() -> Self {
        Self {
            points: VecDeque::new(),
            start_point: None,
            is_gesture_in_progress: false,
            sample_count: 5,
            min_gesture_distance: 0.2, // Normalized distance (range 2.0)
        }
    }

    fn get_recognition_threshold(&self, sensitivity: f64) -> f64 {
        let scale_factor = sensitivity.max(0.1) / 2.0;
        self.min_gesture_distance / scale_factor
    }

    pub fn process(&mut self, data: &ControllerData, sensitivity: f64) -> Option<GestureDirection> {
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
                self.end_gesture(sensitivity)
            }
        } else {
            None
        }
    }

    fn start_gesture(&mut self, point: TouchpadPoint) {
        trace!(
            event = "gesture.started",
            x = point.x,
            y = point.y,
            "Gesture started"
        );
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

    fn end_gesture(&mut self, sensitivity: f64) -> Option<GestureDirection> {
        let mut result = GestureDirection::None;

        if self.points.len() >= 2 {
            if let Some(start) = self.start_point {
                // Use the last point in buffer as end point
                if let Some(end) = self.points.back() {
                    result = self.calculate_direction(start, *end, sensitivity);
                }
            }
        }

        if result != GestureDirection::None {
            debug!(event = "gesture.recognized", ?result, "Gesture recognized");
        } else {
            trace!(
                event = "gesture.rejected",
                reason = "short_or_unclear",
                "Gesture rejected"
            );
        }

        self.is_gesture_in_progress = false;
        self.points.clear();

        if result != GestureDirection::None {
            Some(result)
        } else {
            None
        }
    }

    fn calculate_direction(
        &self,
        start: TouchpadPoint,
        end: TouchpadPoint,
        sensitivity: f64,
    ) -> GestureDirection {
        let dx = end.x - start.x;
        let dy = end.y - start.y;

        let dist_sq = dx * dx + dy * dy;
        let threshold = self.get_recognition_threshold(sensitivity);
        let threshold_sq = threshold * threshold;

        if dist_sq < threshold_sq {
            trace!(
                event = "gesture.rejected",
                dist_sq,
                threshold_sq,
                reason = "distance",
                "Gesture rejected"
            );
            return GestureDirection::None;
        }

        let angle = dy.atan2(dx);
        let mut degrees = angle * 180.0 / PI;

        if degrees < 0.0 {
            degrees += 360.0;
        }

        let tolerance = 30.0; // +/- 30 degrees (total 60 degree cone)

        let direction = if degrees >= (360.0 - tolerance) || degrees < tolerance {
            GestureDirection::Right
        } else if degrees >= (90.0 - tolerance) && degrees < (90.0 + tolerance) {
            GestureDirection::Down
        } else if degrees >= (180.0 - tolerance) && degrees < (180.0 + tolerance) {
            GestureDirection::Left
        } else if degrees >= (270.0 - tolerance) && degrees < (270.0 + tolerance) {
            GestureDirection::Up
        } else {
            GestureDirection::None
        };

        debug!(
            event = "gesture.evaluated", distance = dist_sq.sqrt(), angle_deg = degrees, result = ?direction,
            "Gesture evaluated"
        );
        direction
    }
}
