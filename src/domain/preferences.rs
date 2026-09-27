//! Editable preferences exclude device history and calibration data.
use super::{bindings::ButtonBindings, i18n::Language, settings::Settings};

#[derive(Debug, Clone, PartialEq)]
pub struct InputPreferences {
    pub air_sensitivity: f64,
    pub touch_sensitivity: f64,
    pub dead_zone: f64,
    pub smoothing: bool,
    pub smoothing_samples: usize,
    pub acceleration: bool,
    pub acceleration_power: f64,
    pub natural_scroll: bool,
    pub edge_motion: bool,
}

impl From<&Settings> for InputPreferences {
    fn from(settings: &Settings) -> Self {
        Self {
            air_sensitivity: settings.air_sensitivity(),
            touch_sensitivity: settings.touch_sensitivity(),
            dead_zone: settings.dead_zone,
            smoothing: settings.enable_smoothing,
            smoothing_samples: settings.smoothing_factor,
            acceleration: settings.enable_acceleration,
            acceleration_power: settings.acceleration_power,
            natural_scroll: settings.natural_scroll,
            edge_motion: settings.touchpad_edge_motion,
        }
    }
}
impl InputPreferences {
    pub fn apply(&self, settings: &mut Settings) {
        settings.air_mouse_sensitivity = Some(self.air_sensitivity);
        settings.touchpad_sensitivity = Some(self.touch_sensitivity);
        settings.dead_zone = self.dead_zone;
        settings.enable_smoothing = self.smoothing;
        settings.smoothing_factor = self.smoothing_samples;
        settings.enable_acceleration = self.acceleration;
        settings.acceleration_power = self.acceleration_power;
        settings.natural_scroll = self.natural_scroll;
        settings.touchpad_edge_motion = self.edge_motion;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UserPreferences {
    pub input: InputPreferences,
    pub bindings: ButtonBindings,
    pub language: Language,
    pub anti_sleep: bool,
    pub auto_profile: bool,
    pub tray: bool,
    pub auto_connect: bool,
    pub auto_reconnect: bool,
}
impl From<&Settings> for UserPreferences {
    fn from(settings: &Settings) -> Self {
        Self {
            input: settings.into(),
            bindings: settings.button_bindings.clone(),
            language: settings.language,
            anti_sleep: settings.enable_presentation_anti_sleep,
            auto_profile: settings.enable_auto_profile_switching,
            tray: settings.minimize_to_tray,
            auto_connect: settings.auto_connect,
            auto_reconnect: settings.auto_reconnect,
        }
    }
}
impl UserPreferences {
    pub fn apply(&self, settings: &mut Settings) {
        self.input.apply(settings);
        settings.button_bindings = self.bindings.clone();
        settings.language = self.language;
        settings.enable_presentation_anti_sleep = self.anti_sleep;
        settings.enable_auto_profile_switching = self.auto_profile;
        settings.minimize_to_tray = self.tray;
        settings.auto_connect = self.auto_connect;
        settings.auto_reconnect = self.auto_reconnect;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saving_preferences_preserves_new_device_and_calibration_data() {
        let mut current = Settings::default();
        let mut draft = UserPreferences::from(&current);
        draft.input.air_sensitivity = 3.0;
        current.last_connected_address = Some(42);
        current.touchpad_calibration.center_x = 150;
        draft.apply(&mut current);
        assert_eq!(current.air_sensitivity(), 3.0);
        assert_eq!(current.last_connected_address, Some(42));
        assert_eq!(current.touchpad_calibration.center_x, 150);
    }
}
