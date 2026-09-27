//! Tab Views for Gear VR Controller WinUI 3 Application

pub mod calibration;
pub mod dashboard;
pub mod diagnostics;
pub mod settings;

pub use calibration::render_calibration_view;
pub use dashboard::render_dashboard_view;
pub use diagnostics::render_diagnostics_view;
pub use settings::render_settings_view;
