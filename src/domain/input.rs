//! Pure input mapping; OS injection is performed by the application worker.
use super::{
    controller::TouchpadProcessor,
    gestures::{GestureDirection, GestureRecognizer},
    imu::ImuProcessor,
    models::{ControlMode, ControllerData},
    settings::Settings,
};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    Move(i32, i32),
    Left(bool),
    RightClick,
    Scroll(i32),
    Key(u16),
    ShowDesktop,
}

pub struct InputMapper {
    pub mode: ControlMode,
    touchpad: TouchpadProcessor,
    imu: ImuProcessor,
    gestures: GestureRecognizer,
    previous: ControllerData,
    left_held: bool,
    buttons_suspended: bool,
    back_started: Option<Instant>,
    scroll_y: Option<f64>,
    scroll_remainder: f64,
    touch_calibration: Option<Vec<(u16, u16)>>,
}

impl Default for InputMapper {
    fn default() -> Self {
        Self {
            mode: ControlMode::Mouse,
            touchpad: TouchpadProcessor::new(),
            imu: ImuProcessor::new(),
            gestures: GestureRecognizer::new(),
            previous: ControllerData::default(),
            left_held: false,
            buttons_suspended: false,
            back_started: None,
            scroll_y: None,
            scroll_remainder: 0.0,
            touch_calibration: None,
        }
    }
}

impl InputMapper {
    pub fn reset(&mut self) -> Vec<InputAction> {
        let actions = if self.left_held {
            vec![InputAction::Left(false)]
        } else {
            vec![]
        };
        self.left_held = false;
        self.buttons_suspended = true;
        self.touch_calibration = None;
        self.imu.cancel_calibration();
        self.previous = ControllerData::default();
        self.back_started = None;
        self.touchpad = TouchpadProcessor::new();
        self.gestures = GestureRecognizer::new();
        self.imu.reset_orientation();
        self.scroll_y = None;
        self.scroll_remainder = 0.0;
        actions
    }

    pub fn set_mode(&mut self, mode: ControlMode) -> Vec<InputAction> {
        let actions = self.reset();
        self.mode = mode;
        actions
    }

    pub fn start_imu_calibration(&mut self) -> Vec<InputAction> {
        let actions = self.reset();
        self.imu.start_calibration();
        actions
    }

    pub fn imu_progress(&self) -> Option<f32> {
        self.imu
            .is_calibrating()
            .then(|| self.imu.calibration_progress())
    }

    pub fn start_touch_calibration(&mut self) -> Vec<InputAction> {
        let actions = self.reset();
        self.touch_calibration = Some(Vec::new());
        actions
    }

    pub fn finish_touch_calibration(
        &mut self,
    ) -> anyhow::Result<super::models::TouchpadCalibration> {
        let samples = self
            .touch_calibration
            .take()
            .ok_or_else(|| anyhow::anyhow!("Touchpad calibration has not started"))?;
        anyhow::ensure!(
            samples.len() >= 20,
            "Collect at least 20 touch samples around the pad before saving"
        );
        let min_x = samples.iter().map(|s| s.0).min().unwrap_or(0);
        let max_x = samples.iter().map(|s| s.0).max().unwrap_or(0);
        let min_y = samples.iter().map(|s| s.1).min().unwrap_or(0);
        let max_y = samples.iter().map(|s| s.1).max().unwrap_or(0);
        anyhow::ensure!(
            max_x.saturating_sub(min_x) >= 100 && max_y.saturating_sub(min_y) >= 100,
            "Move around the full touchpad before saving"
        );
        Ok(super::models::TouchpadCalibration {
            min_x,
            max_x,
            min_y,
            max_y,
            center_x: min_x + (max_x - min_x) / 2,
            center_y: min_y + (max_y - min_y) / 2,
        })
    }

    pub fn process(
        &mut self,
        data: &mut ControllerData,
        settings: &Settings,
        now: Instant,
    ) -> Vec<InputAction> {
        self.touchpad.process(data, &settings.touchpad_calibration);
        if let Some(samples) = self.touch_calibration.as_mut() {
            if data.touchpad_touched && samples.len() < 10_000 {
                samples.push((data.touchpad_x, data.touchpad_y));
            }
            self.previous = data.clone();
            return vec![];
        }
        if self.imu.is_calibrating() {
            self.imu.calculate_airmouse_delta(data, settings);
            self.previous = data.clone();
            return vec![];
        }
        let mut actions = Vec::new();
        self.map_buttons(data, settings, now, &mut actions);
        if !data.back_button {
            match self.mode {
                ControlMode::Mouse => {
                    if let Some((x, y)) = self.imu.calculate_airmouse_delta(data, settings) {
                        actions.push(InputAction::Move(x, y));
                    }
                    if settings.enable_touchpad {
                        self.map_scroll(data, &mut actions);
                    }
                }
                ControlMode::Touchpad if settings.enable_touchpad => {
                    if let Some((x, y)) = self.touchpad.calculate_mouse_delta(data, settings) {
                        actions.push(InputAction::Move(x, y));
                    }
                }
                ControlMode::Presentation
                    if settings.enable_touchpad && settings.enable_gestures =>
                {
                    if let Some(direction) = self.gestures.process(data, settings.mouse_sensitivity)
                    {
                        match direction {
                            GestureDirection::Left | GestureDirection::Up => {
                                actions.push(InputAction::Key(0x21))
                            }
                            GestureDirection::Right | GestureDirection::Down => {
                                actions.push(InputAction::Key(0x22))
                            }
                            GestureDirection::None => {}
                        }
                    }
                }
                _ => {}
            }
        }
        self.previous = data.clone();
        actions
    }

    fn map_buttons(
        &mut self,
        data: &ControllerData,
        settings: &Settings,
        now: Instant,
        actions: &mut Vec<InputAction>,
    ) {
        if !settings.enable_buttons {
            if self.left_held {
                actions.push(InputAction::Left(false));
                self.left_held = false;
            }
            self.back_started = None;
            return;
        }
        if self.buttons_suspended {
            self.buttons_suspended = data.trigger_button
                || data.touchpad_button
                || data.back_button
                || data.home_button
                || data.volume_up_button
                || data.volume_down_button;
            return;
        }
        if self.mode == ControlMode::Presentation {
            if data.trigger_button && !self.previous.trigger_button {
                actions.push(InputAction::Key(0x22));
            }
            if data.touchpad_button && !self.previous.touchpad_button {
                actions.push(InputAction::Key(0xB3));
            }
        } else {
            let held = data.trigger_button || data.touchpad_button;
            if held != self.left_held {
                actions.push(InputAction::Left(held));
                self.left_held = held;
            }
        }
        if data.back_button && !self.previous.back_button {
            self.back_started = Some(now);
        }
        if !data.back_button && self.previous.back_button {
            if let Some(start) = self.back_started.take() {
                if now.duration_since(start) >= Duration::from_millis(600) {
                    let mode = if data.touchpad_touched {
                        if data.processed_touchpad_y < -0.3 {
                            ControlMode::Mouse
                        } else if data.processed_touchpad_x < 0.0 {
                            ControlMode::Touchpad
                        } else {
                            ControlMode::Presentation
                        }
                    } else {
                        match self.mode {
                            ControlMode::Mouse => ControlMode::Touchpad,
                            ControlMode::Touchpad => ControlMode::Presentation,
                            _ => ControlMode::Mouse,
                        }
                    };
                    actions.extend(self.set_mode(mode));
                } else if self.mode == ControlMode::Presentation {
                    actions.push(InputAction::Key(0x21));
                } else {
                    actions.push(InputAction::RightClick);
                }
            }
        }
        if data.home_button && !self.previous.home_button {
            match self.mode {
                ControlMode::Mouse => actions.push(InputAction::Key(0x5B)),
                ControlMode::Touchpad => actions.push(InputAction::ShowDesktop),
                _ => {}
            }
        }
        for (pressed, previous, volume_key, scroll) in [
            (
                data.volume_up_button,
                self.previous.volume_up_button,
                0xAF,
                1,
            ),
            (
                data.volume_down_button,
                self.previous.volume_down_button,
                0xAE,
                -1,
            ),
        ] {
            if pressed && !previous {
                actions.push(if self.mode == ControlMode::Touchpad {
                    InputAction::Scroll(scroll)
                } else {
                    InputAction::Key(volume_key)
                });
            }
        }
    }

    fn map_scroll(&mut self, data: &ControllerData, actions: &mut Vec<InputAction>) {
        if !data.touchpad_touched {
            self.scroll_y = None;
            self.scroll_remainder = 0.0;
            return;
        }
        if let Some(y) = self.scroll_y {
            self.scroll_remainder += (y - data.processed_touchpad_y) * 8.0;
        }
        self.scroll_y = Some(data.processed_touchpad_y);
        let steps = self.scroll_remainder.trunc() as i32;
        if steps != 0 {
            self.scroll_remainder -= steps as f64;
            actions.push(InputAction::Scroll(steps));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trigger_edges_hold_and_release_on_disconnect() {
        let mut mapper = InputMapper::default();
        let mut data = ControllerData {
            trigger_button: true,
            ..Default::default()
        };
        let settings = Settings::default();
        assert_eq!(
            mapper.process(&mut data, &settings, Instant::now()),
            vec![InputAction::Left(true)]
        );
        assert!(mapper
            .process(&mut data, &settings, Instant::now())
            .is_empty());
        assert_eq!(mapper.reset(), vec![InputAction::Left(false)]);
        assert!(mapper.reset().is_empty());
    }
    #[test]
    fn presenter_maps_edges_without_repeating() {
        let mut mapper = InputMapper::default();
        mapper.set_mode(ControlMode::Presentation);
        mapper.process(
            &mut ControllerData::default(),
            &Settings::default(),
            Instant::now(),
        );
        let mut data = ControllerData {
            trigger_button: true,
            ..Default::default()
        };
        assert_eq!(
            mapper.process(&mut data, &Settings::default(), Instant::now()),
            vec![InputAction::Key(0x22)]
        );
        assert!(mapper
            .process(&mut data, &Settings::default(), Instant::now())
            .is_empty());
    }
    #[test]
    fn mode_switch_releases_drag() {
        let mut mapper = InputMapper::default();
        let mut data = ControllerData {
            trigger_button: true,
            ..Default::default()
        };
        mapper.process(&mut data, &Settings::default(), Instant::now());
        assert_eq!(
            mapper.set_mode(ControlMode::Presentation),
            vec![InputAction::Left(false)]
        );
        assert!(mapper
            .process(&mut data, &Settings::default(), Instant::now())
            .is_empty());
        mapper.process(
            &mut ControllerData::default(),
            &Settings::default(),
            Instant::now(),
        );
        assert_eq!(
            mapper.process(&mut data, &Settings::default(), Instant::now()),
            vec![InputAction::Key(0x22)]
        );
    }

    #[test]
    fn imu_calibration_suppresses_input_until_complete() {
        let mut mapper = InputMapper::default();
        mapper.start_imu_calibration();
        let mut data = ControllerData {
            gyro_x: 2.0,
            trigger_button: true,
            ..Default::default()
        };
        for _ in 0..50 {
            assert!(mapper
                .process(&mut data, &Settings::default(), Instant::now())
                .is_empty());
        }
        assert_eq!(mapper.imu_progress(), None);
        data.trigger_button = false;
        assert!(mapper
            .process(&mut data, &Settings::default(), Instant::now())
            .is_empty());
    }

    #[test]
    fn touch_calibration_requires_travel_then_normalizes() -> anyhow::Result<()> {
        let mut mapper = InputMapper::default();
        mapper.start_touch_calibration();
        assert!(mapper.finish_touch_calibration().is_err());
        mapper.start_touch_calibration();
        for index in 0..20 {
            let coordinate = if index % 2 == 0 { 10 } else { 310 };
            let mut data = ControllerData {
                touchpad_touched: true,
                touchpad_x: coordinate,
                touchpad_y: coordinate,
                ..Default::default()
            };
            assert!(mapper
                .process(&mut data, &Settings::default(), Instant::now())
                .is_empty());
        }
        let calibration = mapper.finish_touch_calibration()?;
        assert_eq!(
            (calibration.min_x, calibration.max_x, calibration.center_x),
            (10, 310, 160)
        );
        Ok(())
    }
    #[test]
    fn short_back_is_click_long_back_switches_mode() {
        let mut mapper = InputMapper::default();
        let now = Instant::now();
        let mut data = ControllerData {
            back_button: true,
            ..Default::default()
        };
        mapper.process(&mut data, &Settings::default(), now);
        data.back_button = false;
        assert_eq!(
            mapper.process(
                &mut data,
                &Settings::default(),
                now + Duration::from_millis(100)
            ),
            vec![InputAction::RightClick]
        );
        data.back_button = true;
        mapper.process(&mut data, &Settings::default(), now);
        data.back_button = false;
        mapper.process(
            &mut data,
            &Settings::default(),
            now + Duration::from_secs(1),
        );
        assert_eq!(mapper.mode, ControlMode::Touchpad);
    }
}
