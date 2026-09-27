//! Calibration results are explicit. Cancellation and failure cannot imply success.
use super::models::TouchpadCalibration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationKind {
    Gyroscope,
    Touchpad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationFailure {
    Movement,
    InsufficientTravel,
    Timeout,
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum CalibrationStatus {
    #[default]
    Idle,
    Collecting {
        kind: CalibrationKind,
        progress: f32,
        ready: bool,
    },
    Complete(CalibrationKind),
    Failed {
        kind: CalibrationKind,
        reason: CalibrationFailure,
    },
    Cancelled,
}

impl CalibrationStatus {
    pub fn is_collecting(&self) -> bool {
        matches!(self, Self::Collecting { .. })
    }
}

#[derive(Default)]
pub struct TouchCalibration {
    samples: usize,
    bounds: Option<(u16, u16, u16, u16)>,
}

impl TouchCalibration {
    pub fn collect(&mut self, x: u16, y: u16) {
        self.samples = self.samples.saturating_add(1);
        let (min_x, max_x, min_y, max_y) = self.bounds.get_or_insert((x, x, y, y));
        *min_x = (*min_x).min(x);
        *max_x = (*max_x).max(x);
        *min_y = (*min_y).min(y);
        *max_y = (*max_y).max(y);
    }

    pub fn progress(&self) -> f32 {
        let Some((min_x, max_x, min_y, max_y)) = self.bounds else {
            return 0.0;
        };
        ((self.samples as f32 / 20.0).min(1.0)
            + (f32::from(max_x - min_x) / 100.0).min(1.0)
            + (f32::from(max_y - min_y) / 100.0).min(1.0))
            / 3.0
    }

    pub fn result(&self) -> Option<TouchpadCalibration> {
        let (min_x, max_x, min_y, max_y) = self.bounds?;
        if self.samples < 20 || max_x - min_x < 100 || max_y - min_y < 100 {
            return None;
        }
        Some(TouchpadCalibration {
            min_x,
            max_x,
            min_y,
            max_y,
            center_x: min_x + (max_x - min_x) / 2,
            center_y: min_y + (max_y - min_y) / 2,
        })
    }
}
