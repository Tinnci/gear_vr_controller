use crate::domain::models::TouchpadCalibration;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSettings {
    #[serde(default = "default_retention")]
    pub retention_days: u32,
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
            retention_days: default_retention(),
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
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join("GearVRController")
        .join("logs")
        .to_string_lossy()
        .into_owned()
}
fn default_retention() -> u32 {
    14
}
fn default_prefix() -> String {
    "gear_vr_controller".to_string()
}
fn default_rotation() -> String {
    "daily".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub mouse_sensitivity: f64,
    pub touchpad_calibration: TouchpadCalibration,
    pub known_bluetooth_addresses: Vec<u64>,
    pub last_connected_address: Option<u64>,
    pub enable_touchpad: bool,
    pub enable_buttons: bool,
    pub enable_gestures: bool,

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

    // Windows System Integration Settings
    #[serde(default = "default_true")]
    pub enable_presentation_anti_sleep: bool,
    #[serde(default = "default_false")]
    pub enable_auto_profile_switching: bool,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,

    // Interface Language
    #[serde(default)]
    pub language: crate::domain::i18n::Language,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            mouse_sensitivity: 2.0,
            touchpad_calibration: TouchpadCalibration::default(),
            known_bluetooth_addresses: Vec::new(),
            last_connected_address: None,
            enable_touchpad: true,
            enable_buttons: true,
            enable_gestures: true,
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

            // Windows System Integration Settings
            enable_presentation_anti_sleep: true,
            enable_auto_profile_switching: false,
            minimize_to_tray: true,

            // Interface Language
            language: crate::domain::i18n::Language::Auto,
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

pub trait SettingsStore: Send {
    fn load(&self) -> anyhow::Result<Settings>;
    fn save(&self, settings: &Settings) -> anyhow::Result<()>;
}

pub struct JsonFileSettingsStore {
    path: PathBuf,
}

impl JsonFileSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> anyhow::Result<PathBuf> {
        let mut path = windows_config_dir()?;
        path.push("GearVRController");
        fs::create_dir_all(&path)?;
        path.push("settings.json");
        Ok(path)
    }

    fn load_from_file(path: &Path) -> anyhow::Result<Settings> {
        let contents = fs::read_to_string(path)?;
        let settings = serde_json::from_str(&contents)?;
        Ok(settings)
    }
}

impl SettingsStore for JsonFileSettingsStore {
    fn load(&self) -> anyhow::Result<Settings> {
        match Self::load_from_file(&self.path) {
            Ok(settings) => {
                settings.validate()?;
                Ok(settings)
            }
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                Ok(Settings::default())
            }
            Err(error) => Err(error.context(format!(
                "Cannot load {}; original file is preserved",
                self.path.display()
            ))),
        }
    }

    fn save(&self, settings: &Settings) -> anyhow::Result<()> {
        use std::io::Write;
        use std::os::windows::ffi::OsStrExt;
        use windows::{
            core::PCWSTR,
            Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            },
        };
        settings.validate()?;
        let json = serde_json::to_vec_pretty(settings)?;
        let temporary = self
            .path
            .with_extension(format!("{}.tmp", std::process::id()));
        if self.path.is_file() {
            fs::copy(&self.path, self.path.with_extension("json.bak"))?;
        }
        let result = (|| -> anyhow::Result<()> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(&json)?;
            file.sync_all()?;
            drop(file);
            let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let dest: Vec<u16> = self.path.as_os_str().encode_wide().chain(Some(0)).collect();
            // SAFETY: both paths are null-terminated and remain alive during the call.
            unsafe {
                MoveFileExW(
                    PCWSTR(source.as_ptr()),
                    PCWSTR(dest.as_ptr()),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result?;
        Ok(())
    }
}

fn windows_config_dir() -> anyhow::Result<PathBuf> {
    env::var_os("APPDATA")
        .or_else(|| env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Could not determine Windows config directory"))
}

struct MemorySettingsStore;

impl SettingsStore for MemorySettingsStore {
    fn load(&self) -> anyhow::Result<Settings> {
        Ok(Settings::default())
    }

    fn save(&self, _settings: &Settings) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct SettingsService {
    settings: Settings,
    store: Box<dyn SettingsStore>,
}

impl SettingsService {
    pub fn new() -> anyhow::Result<Self> {
        Self::with_store(Box::new(JsonFileSettingsStore::new(
            JsonFileSettingsStore::default_path()?,
        )))
    }

    pub fn with_store(store: Box<dyn SettingsStore>) -> anyhow::Result<Self> {
        let settings = store.load()?;
        settings.validate()?;

        Ok(Self { settings, store })
    }

    pub fn in_memory_defaults() -> Self {
        Self {
            settings: Settings::default(),
            store: Box::new(MemorySettingsStore),
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        self.store.save(&self.settings)
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

impl Settings {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.schema_version == 1,
            "Unsupported settings schema {}",
            self.schema_version
        );
        anyhow::ensure!(
            self.mouse_sensitivity.is_finite() && (0.1..=20.0).contains(&self.mouse_sensitivity),
            "Sensitivity must be between 0.1 and 20"
        );
        anyhow::ensure!(
            self.dead_zone.is_finite() && (0.0..=0.9).contains(&self.dead_zone),
            "Dead zone must be between 0 and 0.9"
        );
        anyhow::ensure!(
            (1..=64).contains(&self.smoothing_factor),
            "Smoothing samples must be between 1 and 64"
        );
        anyhow::ensure!(
            self.acceleration_power.is_finite() && (1.0..=3.0).contains(&self.acceleration_power),
            "Acceleration must be between 1 and 3"
        );
        anyhow::ensure!(
            (1..=10).contains(&self.pairing_max_retries)
                && (100..=10_000).contains(&self.pairing_retry_delay_ms),
            "Invalid pairing retry settings"
        );
        for uuid in [
            &self.ble_service_uuid,
            &self.ble_data_char_uuid,
            &self.ble_command_char_uuid,
        ] {
            let compact = uuid.replace('-', "");
            anyhow::ensure!(
                compact.len() == 32 && compact.bytes().all(|b| b.is_ascii_hexdigit()),
                "Invalid BLE UUID"
            );
        }
        let c = &self.touchpad_calibration;
        anyhow::ensure!(
            c.min_x < c.max_x
                && c.min_y < c.max_y
                && (c.min_x..=c.max_x).contains(&c.center_x)
                && (c.min_y..=c.max_y).contains(&c.center_y),
            "Invalid touchpad calibration"
        );
        anyhow::ensure!(
            self.known_bluetooth_addresses.len() <= 100
                && self
                    .known_bluetooth_addresses
                    .iter()
                    .all(|a| *a <= 0xFFFF_FFFF_FFFF)
                && self
                    .last_connected_address
                    .is_none_or(|a| a <= 0xFFFF_FFFF_FFFF),
            "Invalid Bluetooth history"
        );
        let logs = &self.log_settings;
        anyhow::ensure!(
            (1..=90).contains(&logs.retention_days),
            "Log retention must be between 1 and 90 days"
        );
        anyhow::ensure!(
            !logs.file_name_prefix.is_empty()
                && logs
                    .file_name_prefix
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
            "Invalid log file prefix"
        );
        anyhow::ensure!(
            ["trace", "debug", "info", "warn", "error"].contains(&logs.level.as_str()),
            "Invalid log level"
        );
        anyhow::ensure!(
            ["never", "minutely", "hourly", "daily"].contains(&logs.rotation.as_str()),
            "Invalid log rotation"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_settings_receive_defaults() -> anyhow::Result<()> {
        let settings: Settings = serde_json::from_str(r#"{"mouse_sensitivity":3.0}"#)?;
        settings.validate()?;
        assert_eq!(settings.mouse_sensitivity, 3.0);
        assert_eq!(settings.schema_version, 1);
        Ok(())
    }
    #[test]
    fn reject_invalid_ranges_and_future_schemas() {
        let mut settings = Settings {
            smoothing_factor: 0,
            ..Default::default()
        };
        assert!(settings.validate().is_err());
        settings.smoothing_factor = 5;
        settings.schema_version = 2;
        assert!(settings.validate().is_err());
    }
    #[test]
    fn atomic_save_keeps_backup_and_corrupt_file() -> anyhow::Result<()> {
        let dir = env::temp_dir().join(format!("gear-vr-settings-test-{}", std::process::id()));
        fs::create_dir_all(&dir)?;
        let path = dir.join("settings.json");
        let store = JsonFileSettingsStore::new(path.clone());
        let mut settings = Settings::default();
        store.save(&settings)?;
        settings.mouse_sensitivity = 3.0;
        store.save(&settings)?;
        assert_eq!(store.load()?.mouse_sensitivity, 3.0);
        assert_eq!(
            JsonFileSettingsStore::load_from_file(&path.with_extension("json.bak"))?
                .mouse_sensitivity,
            2.0
        );
        fs::write(&path, "broken json")?;
        assert!(store.load().is_err());
        assert_eq!(fs::read_to_string(&path)?, "broken json");
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
