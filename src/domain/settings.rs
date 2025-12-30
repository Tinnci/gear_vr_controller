use crate::domain::models::TouchpadCalibration;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSettings {
    #[serde(default = "default_level")]
    pub level: String, // "trace", "debug", "info", "warn", "error"
    #[serde(default = "default_true")]
    pub file_logging_enabled: bool,
    #[serde(default = "default_true")]
    pub console_logging_enabled: bool,
    #[serde(default = "default_log_dir")]
    pub log_dir: String,
    #[serde(default = "default_prefix")]
    pub file_name_prefix: String,
    #[serde(default = "default_true")]
    pub show_file_line: bool,
    #[serde(default = "default_false")]
    pub show_thread_ids: bool,
    #[serde(default = "default_true")]
    pub show_target: bool,
    #[serde(default = "default_true")]
    pub ansi_colors: bool,
    #[serde(default = "default_rotation")]
    pub rotation: String, // "daily", "hourly", "minutely", "never"
}

impl Default for LogSettings {
    fn default() -> Self {
        Self {
            level: default_level(),
            file_logging_enabled: default_true(),
            console_logging_enabled: default_true(),
            log_dir: default_log_dir(),
            file_name_prefix: default_prefix(),
            show_file_line: default_true(),
            show_thread_ids: default_false(),
            show_target: default_true(),
            ansi_colors: default_true(),
            rotation: default_rotation(),
        }
    }
}

fn default_level() -> String {
    "info".to_string()
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_log_dir() -> String {
    "logs".to_string()
}
fn default_prefix() -> String {
    "gear_vr_controller".to_string()
}
fn default_rotation() -> String {
    "daily".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub mouse_sensitivity: f64,
    pub touchpad_calibration: TouchpadCalibration,
    pub known_bluetooth_addresses: Vec<u64>,
    pub last_connected_address: Option<u64>,
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

    // Advanced BLE Settings
    #[serde(default = "default_service_uuid")]
    pub ble_service_uuid: String,
    #[serde(default = "default_data_uuid")]
    pub ble_data_char_uuid: String,
    #[serde(default = "default_command_uuid")]
    pub ble_command_char_uuid: String,
    #[serde(default = "default_false")]
    pub debug_show_all_devices: bool,

    // Debug Settings
    #[serde(default = "default_false")]
    pub debug_raw_data_logging: bool,

    // Pairing Settings
    #[serde(default = "default_pairing_max_retries")]
    pub pairing_max_retries: u32,
    #[serde(default = "default_pairing_retry_delay_ms")]
    pub pairing_retry_delay_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 2.0,
            touchpad_calibration: TouchpadCalibration::default(),
            known_bluetooth_addresses: Vec::new(),
            last_connected_address: None,
            enable_touchpad: true,
            enable_buttons: true,
            log_settings: LogSettings::default(),
            // Defaults based on C# implementation
            dead_zone: 0.1, // 10%
            enable_smoothing: true,
            smoothing_factor: 5, // 5 samples
            enable_acceleration: true,
            acceleration_power: 1.5,

            // Advanced BLE Settings
            ble_service_uuid: default_service_uuid(),
            ble_data_char_uuid: default_data_uuid(),
            ble_command_char_uuid: default_command_uuid(),
            debug_show_all_devices: false,

            // Debug Settings
            debug_raw_data_logging: false,

            // Pairing Settings
            pairing_max_retries: default_pairing_max_retries(),
            pairing_retry_delay_ms: default_pairing_retry_delay_ms(),
        }
    }
}

fn default_service_uuid() -> String {
    "4f63756c-7573-2054-6872-65656d6f7465".to_string()
}
fn default_data_uuid() -> String {
    "c8c51726-81bc-483b-a052-f7a14ea3d281".to_string()
}
fn default_command_uuid() -> String {
    "c8c51726-81bc-483b-a052-f7a14ea3d282".to_string()
}
fn default_pairing_max_retries() -> u32 {
    3
}
fn default_pairing_retry_delay_ms() -> u64 {
    1000
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
