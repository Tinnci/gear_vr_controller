use crate::domain::models::TouchpadCalibration;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSettings {
    pub level: String, // "trace", "debug", "info", "warn", "error"
    pub file_logging_enabled: bool,
    pub console_logging_enabled: bool,
    pub log_dir: String,
    pub file_name_prefix: String,
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file_logging_enabled: true,
            console_logging_enabled: true,
            log_dir: "logs".to_string(),
            file_name_prefix: "gear_vr_controller".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub mouse_sensitivity: f64,
    pub touchpad_calibration: TouchpadCalibration,
    pub known_bluetooth_addresses: Vec<u64>,
    pub enable_touchpad: bool,
    pub enable_buttons: bool,

    // Logging Settings
    #[serde(default)]
    pub log_settings: LogSettings,

    // Phase 2: Input Polish Settings
    pub dead_zone: f64,
    pub enable_smoothing: bool,
    pub smoothing_factor: usize,
    pub enable_acceleration: bool,
    pub acceleration_power: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 2.0,
            touchpad_calibration: TouchpadCalibration::default(),
            known_bluetooth_addresses: Vec::new(),
            enable_touchpad: true,
            enable_buttons: true,
            log_settings: LogSettings::default(),
            // Defaults based on C# implementation
            dead_zone: 0.1, // 10%
            enable_smoothing: true,
            smoothing_factor: 5, // 5 samples
            enable_acceleration: true,
            acceleration_power: 1.5,
        }
    }
}

pub struct SettingsService {
    settings: Settings,
    settings_path: PathBuf,
}

impl SettingsService {
    pub fn new() -> anyhow::Result<Self> {
        let settings_path = Self::get_settings_path()?;
        let settings = Self::load_from_file(&settings_path).unwrap_or_default();

        Ok(Self {
            settings,
            settings_path,
        })
    }

    fn get_settings_path() -> anyhow::Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
        path.push("GearVRController");
        fs::create_dir_all(&path)?;
        path.push("settings.json");
        Ok(path)
    }

    fn load_from_file(path: &PathBuf) -> anyhow::Result<Settings> {
        let contents = fs::read_to_string(path)?;
        let settings = serde_json::from_str(&contents)?;
        Ok(settings)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(&self.settings)?;
        fs::write(&self.settings_path, json)?;
        Ok(())
    }

    pub fn get(&self) -> &Settings {
        &self.settings
    }

    pub fn get_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn update_calibration(&mut self, calibration: TouchpadCalibration) -> anyhow::Result<()> {
        self.settings.touchpad_calibration = calibration;
        self.save()
    }

    pub fn add_known_address(&mut self, address: u64) -> anyhow::Result<()> {
        if !self.settings.known_bluetooth_addresses.contains(&address) {
            self.settings.known_bluetooth_addresses.push(address);
            self.save()?;
        }
        Ok(())
    }
}
