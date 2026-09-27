//! Presentation state and validation have no dependency on native controls.
use super::text::Text;
use crate::domain::{
    calibration::{CalibrationFailure, CalibrationStatus},
    connection_failure::ConnectionFailureKind,
    i18n::Language,
    models::{
        AppEvent, ConnectionStatus, ControlMode, ControllerData, InputPreview, MessageSeverity,
        OutputTarget, ScannedDevice,
    },
    preferences::UserPreferences,
    settings::Settings,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Control,
    Tuning,
    Settings,
    Help,
}
impl Page {
    pub const ALL: [Self; 4] = [Self::Control, Self::Tuning, Self::Settings, Self::Help];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn title(self) -> Text {
        match self {
            Self::Control => Text::Control,
            Self::Tuning => Text::Tuning,
            Self::Settings => Text::Settings,
            Self::Help => Text::Help,
        }
    }
    pub fn from_tag(tag: &str) -> Option<Self> {
        tag.parse::<usize>()
            .ok()
            .and_then(|index| Self::ALL.get(index).copied())
    }
}
/// A subpage is a view choice, not a controller command or a settings snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subpage {
    Input,
    Calibration,
    Test,
    General,
    Bindings,
    Background,
    Troubleshooting,
    Details,
}
impl Subpage {
    pub fn title(self) -> Text {
        match self {
            Self::Input => Text::InputTuning,
            Self::Calibration => Text::Calibration,
            Self::Test => Text::TestNav,
            Self::General => Text::General,
            Self::Bindings => Text::Bindings,
            Self::Background => Text::Background,
            Self::Troubleshooting => Text::Troubleshooting,
            Self::Details => Text::Details,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum NumericPreference {
    AirSpeed,
    TouchSpeed,
    DeadZone,
    SmoothingSamples,
    AccelerationPower,
}
#[derive(Debug, Clone, Copy)]
pub enum BooleanPreference {
    NaturalScroll,
    EdgeMotion,
    Smoothing,
    Acceleration,
    AntiSleep,
    AutoProfile,
    Tray,
    AutoConnect,
    AutoReconnect,
}
#[derive(Debug, Clone, Copy)]
pub enum BindingSlot {
    Trigger,
    Touchpad,
    Back,
    Home,
}

pub struct UiState {
    pub page: Page,
    pub tuning_page: Subpage,
    pub settings_page: Subpage,
    pub help_page: Subpage,
    pub advanced_input_open: bool,
    pub advanced_connection_open: bool,
    pub other_devices_open: bool,
    pub draft: UserPreferences,
    pub saved: UserPreferences,
    pub connection: ConnectionStatus,
    pub output: OutputTarget,
    pub output_pending: bool,
    pub mode: ControlMode,
    pub automatic_mode: bool,
    pub calibration: CalibrationStatus,
    pub preview: InputPreview,
    pub latest: Option<ControllerData>,
    pub devices: Vec<ScannedDevice>,
    pub address: String,
    pub last_address: Option<u64>,
    pub connected_address: Option<u64>,
    pub scanning: bool,
    pub scan_attempted: bool,
    pub scan_pending: bool,
    pub notice: Option<Text>,
    pub notice_title: Text,
    pub severity: MessageSeverity,
    pub diagnostic_details: String,
    pub diagnostic_export_path: String,
    pub recovery_running: bool,
    pub recovery_confirm: bool,
    pub worker_ready: bool,
    pub binding_mode: ControlMode,
}
impl UiState {
    pub fn new(settings: &Settings) -> Self {
        let draft = UserPreferences::from(settings);
        Self {
            page: Page::Control,
            tuning_page: Subpage::Input,
            settings_page: Subpage::General,
            help_page: Subpage::Troubleshooting,
            advanced_input_open: false,
            advanced_connection_open: false,
            other_devices_open: false,
            saved: draft.clone(),
            draft,
            connection: ConnectionStatus::Disconnected,
            output: OutputTarget::Paused,
            output_pending: false,
            mode: ControlMode::Mouse,
            automatic_mode: false,
            calibration: CalibrationStatus::Idle,
            preview: Default::default(),
            latest: None,
            devices: vec![],
            address: settings
                .last_connected_address
                .map(|a| format!("{a:012X}"))
                .unwrap_or_default(),
            last_address: settings.last_connected_address,
            connected_address: None,
            scanning: false,
            scan_attempted: false,
            scan_pending: false,
            notice: None,
            notice_title: Text::NoticeInfo,
            severity: MessageSeverity::Info,
            diagnostic_details: String::new(),
            diagnostic_export_path: String::new(),
            recovery_running: false,
            recovery_confirm: false,
            worker_ready: false,
            binding_mode: ControlMode::Mouse,
        }
    }
    pub fn subpages(&self) -> &'static [Subpage] {
        match self.page {
            Page::Control => &[],
            Page::Tuning => &[Subpage::Input, Subpage::Calibration, Subpage::Test],
            Page::Settings => &[Subpage::General, Subpage::Bindings, Subpage::Background],
            Page::Help => &[Subpage::Troubleshooting, Subpage::Details],
        }
    }
    pub fn active_subpage(&self) -> Option<Subpage> {
        match self.page {
            Page::Control => None,
            Page::Tuning => Some(self.tuning_page),
            Page::Settings => Some(self.settings_page),
            Page::Help => Some(self.help_page),
        }
    }
    /// Reactor reports selected text. Resolve it only within the current page and locale.
    pub fn select_subpage(&mut self, label: &str) {
        let selected = self
            .subpages()
            .iter()
            .copied()
            .find(|page| page.title().get(self.language()) == label);
        if let Some(selected) = selected {
            match self.page {
                Page::Control => {}
                Page::Tuning => self.tuning_page = selected,
                Page::Settings => self.settings_page = selected,
                Page::Help => self.help_page = selected,
            }
        }
    }
    pub fn language(&self) -> Language {
        self.draft.language
    }
    pub fn dirty(&self) -> bool {
        self.draft != self.saved
    }
    pub fn connected(&self) -> bool {
        self.connection == ConnectionStatus::Connected
    }
    pub fn can_output(&self) -> bool {
        self.connected() && !self.calibration.is_collecting() && !self.output_pending
    }
    pub fn set_notice(&mut self, text: Text, severity: MessageSeverity) {
        self.notice = Some(text);
        self.severity = severity;
        self.notice_title = match severity {
            MessageSeverity::Error => Text::NoticeError,
            MessageSeverity::Warning => Text::NoticeWarning,
            MessageSeverity::Success | MessageSeverity::Info => Text::NoticeInfo,
        };
    }
    pub fn error(&mut self, detail: impl Into<String>) {
        self.diagnostic_details = detail.into();
        tracing::error!(event = "ui.operation.failed", page = ?self.page,
            error = %self.diagnostic_details, "Interface operation failed");
        self.set_notice(Text::OperationFailed, MessageSeverity::Error);
        self.scan_pending = false;
        self.output_pending = false;
    }
    pub fn set_number(&mut self, field: NumericPreference, value: f64) {
        if !value.is_finite() {
            return;
        }
        match field {
            NumericPreference::AirSpeed => {
                self.draft.input.air_sensitivity = value.clamp(0.1, 20.0)
            }
            NumericPreference::TouchSpeed => {
                self.draft.input.touch_sensitivity = value.clamp(0.1, 20.0)
            }
            NumericPreference::DeadZone => self.draft.input.dead_zone = value.clamp(0.0, 0.9),
            NumericPreference::SmoothingSamples => {
                self.draft.input.smoothing_samples = (value.round() as usize).clamp(1, 64)
            }
            NumericPreference::AccelerationPower => {
                self.draft.input.acceleration_power = value.clamp(1.0, 3.0)
            }
        }
    }
    pub fn set_bool(&mut self, field: BooleanPreference, value: bool) {
        match field {
            BooleanPreference::NaturalScroll => self.draft.input.natural_scroll = value,
            BooleanPreference::EdgeMotion => self.draft.input.edge_motion = value,
            BooleanPreference::Smoothing => self.draft.input.smoothing = value,
            BooleanPreference::Acceleration => self.draft.input.acceleration = value,
            BooleanPreference::AntiSleep => self.draft.anti_sleep = value,
            BooleanPreference::AutoProfile => self.draft.auto_profile = value,
            BooleanPreference::Tray => self.draft.tray = value,
            BooleanPreference::AutoConnect => self.draft.auto_connect = value,
            BooleanPreference::AutoReconnect => self.draft.auto_reconnect = value,
        }
    }
    pub fn calibration_text(&self) -> Option<Text> {
        match self.calibration {
            CalibrationStatus::Idle => None,
            CalibrationStatus::Collecting {
                kind: crate::domain::calibration::CalibrationKind::Gyroscope,
                ..
            } => Some(Text::CollectingGyro),
            CalibrationStatus::Collecting { ready: true, .. } => Some(Text::ReadyToSave),
            CalibrationStatus::Collecting { .. } => Some(Text::CollectingTouch),
            CalibrationStatus::Complete(_) => Some(Text::CalibrationDone),
            CalibrationStatus::Cancelled => Some(Text::CalibrationCancelled),
            CalibrationStatus::Failed { reason, .. } => Some(match reason {
                CalibrationFailure::Movement => Text::CalibrationMoving,
                CalibrationFailure::Disconnected => Text::CalibrationLost,
                CalibrationFailure::Timeout => Text::CalibrationTimeout,
                CalibrationFailure::InsufficientTravel => Text::CollectingTouch,
            }),
        }
    }
    pub fn event(&mut self, event: AppEvent) {
        match event {
            AppEvent::ConnectionFailed(failure) => {
                self.diagnostic_details = failure.detail;
                let text = match failure.kind {
                    ConnectionFailureKind::Unreachable => Text::DeviceUnreachable,
                    ConnectionFailureKind::AccessDenied => Text::ConnectionDenied,
                    ConnectionFailureKind::Protocol => Text::ConnectionProtocol,
                    ConnectionFailureKind::Incompatible => Text::ConnectionIncompatible,
                    ConnectionFailureKind::Timeout => Text::ConnectionTimeout,
                    ConnectionFailureKind::Other => Text::OperationFailed,
                };
                self.set_notice(text, MessageSeverity::Error);
                self.notice_title = Text::ConnectionFailed;
                self.scan_pending = false;
                self.output_pending = false;
            }
            AppEvent::WorkerReady => self.worker_ready = true,
            AppEvent::ModeChanged(mode) => {
                self.mode = mode;
                self.automatic_mode = false;
            }
            AppEvent::AutomaticModeChanged(mode) => {
                self.mode = mode;
                self.automatic_mode = true;
            }
            AppEvent::CalibrationStatus(status) => self.calibration = status,
            AppEvent::OutputChanged(target) => {
                self.output = target;
                self.output_pending = false;
                self.preview = Default::default();
            }
            AppEvent::InputPreview(preview) => self.preview = preview,
            AppEvent::ScanState(scanning) => {
                self.scanning = scanning;
                self.scan_pending = false;
                self.scan_attempted = true;
            }
            AppEvent::ConnectedDevice(address) => {
                self.connected_address = Some(address);
                self.last_address = Some(address);
            }
            AppEvent::ReconnectAttempt(_) => {
                self.set_notice(Text::Reconnecting, MessageSeverity::Info)
            }
            AppEvent::ControllerData(data) => self.latest = Some(data),
            AppEvent::ConnectionStatus(status) => {
                self.connection = status;
                if status != ConnectionStatus::Connected {
                    self.latest = None;
                }
                if status == ConnectionStatus::Connected
                    && (self.notice_title == Text::ConnectionFailed
                        || self.notice == Some(Text::Reconnecting))
                {
                    self.notice = None;
                }
            }
            AppEvent::LogMessage(log) => {
                if matches!(
                    log.severity,
                    MessageSeverity::Error | MessageSeverity::Warning
                ) {
                    self.diagnostic_details = log.message;
                    self.set_notice(
                        if log.severity == MessageSeverity::Warning {
                            Text::OperationWarning
                        } else {
                            Text::OperationFailed
                        },
                        log.severity,
                    );
                    self.scan_pending = false;
                    self.output_pending = false;
                }
            }
            AppEvent::DevicesUpdated(devices) => self.devices = devices,
        }
    }
}

pub fn parse_address(text: &str) -> Option<u64> {
    let compact = text.trim().replace([':', '-'], "");
    if compact.len() != 12 || !compact.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(&compact, 16)
        .ok()
        .filter(|value| *value != 0)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{connection_failure::ConnectionFailure, models::StatusMessage};
    #[test]
    fn connection_error_survives_progress_and_clears_on_recovery() {
        let mut state = UiState::new(&Settings::default());
        state.event(AppEvent::ConnectionFailed(ConnectionFailure::new(
            ConnectionFailureKind::Unreachable,
            "Read services: GATT Unreachable (1)",
        )));
        state.event(AppEvent::LogMessage(StatusMessage {
            message: "Connecting without traditional pairing".into(),
            severity: MessageSeverity::Info,
        }));
        state.event(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
        state.page = Page::Help;
        assert_eq!(state.notice_title, Text::ConnectionFailed);
        assert_eq!(state.notice, Some(Text::DeviceUnreachable));
        assert!(state.diagnostic_details.contains("Unreachable (1)"));
        state.event(AppEvent::ConnectionStatus(ConnectionStatus::Connected));
        assert_eq!(state.notice, None);
        assert!(state.diagnostic_details.contains("Unreachable (1)"));
    }
    #[test]
    fn connection_warning_is_not_reported_as_failure() {
        let mut state = UiState::new(&Settings::default());
        state.event(AppEvent::LogMessage(StatusMessage {
            message: "Connected, but cannot save history".into(),
            severity: MessageSeverity::Warning,
        }));
        state.event(AppEvent::ConnectionStatus(ConnectionStatus::Connected));
        assert_eq!(state.notice, Some(Text::OperationWarning));
        assert_eq!(state.notice_title, Text::NoticeWarning);
        assert!(state.diagnostic_details.contains("cannot save history"));
    }
    #[test]
    fn subpage_navigation_preserves_drafts_and_device_operation() {
        let mut state = UiState::new(&Settings::default());
        state.page = Page::Tuning;
        state.draft.input.air_sensitivity = 4.0;
        state.connection = ConnectionStatus::Connected;
        state.output = OutputTarget::Preview;
        state.calibration = CalibrationStatus::Collecting {
            kind: crate::domain::calibration::CalibrationKind::Touchpad,
            progress: 0.5,
            ready: false,
        };
        state.select_subpage(Subpage::Test.title().get(state.language()));
        state.page = Page::Settings;
        state.select_subpage(Subpage::Bindings.title().get(state.language()));
        // A stale event from a different parent must not change this parent's route.
        state.select_subpage(Subpage::Calibration.title().get(state.language()));
        assert_eq!(state.active_subpage(), Some(Subpage::Bindings));
        state.draft.language = Language::Japanese;
        state.page = Page::Tuning;
        assert_eq!(state.active_subpage(), Some(Subpage::Test));
        assert_eq!(state.draft.input.air_sensitivity, 4.0);
        assert!(state.dirty());
        assert!(state.connected());
        assert_eq!(state.output, OutputTarget::Preview);
        assert!(state.calibration.is_collecting());
    }

    #[test]
    fn address_requires_a_complete_nonzero_address() {
        assert_eq!(parse_address(" 2C:41:A1:00:12:34 "), Some(0x2C41A1001234));
        for value in ["42", "000000000000", "😀00000000000", "00112233445566"] {
            assert_eq!(parse_address(value), None);
        }
    }
    #[test]
    fn disconnect_removes_stale_telemetry_and_cancel_is_not_success() {
        let mut state = UiState::new(&Settings::default());
        state.event(AppEvent::ControllerData(Default::default()));
        state.event(AppEvent::ConnectionStatus(ConnectionStatus::Disconnected));
        assert!(state.latest.is_none());
        state.event(AppEvent::CalibrationStatus(CalibrationStatus::Cancelled));
        assert_eq!(state.calibration_text(), Some(Text::CalibrationCancelled));
    }
}
