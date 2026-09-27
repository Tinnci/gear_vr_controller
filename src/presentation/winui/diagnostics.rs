//! Local diagnostic export deliberately excludes device identifiers and raw input.
use super::state::UiState;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn export_summary(state: &UiState) -> anyhow::Result<PathBuf> {
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("GearVRController/diagnostics");
    std::fs::create_dir_all(&dir)?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let path = dir.join(format!("summary-{timestamp}.json"));
    let summary = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"), "created_unix_ms": timestamp,
        "connection": format!("{:?}", state.connection), "mode": format!("{:?}", state.mode),
        "output": format!("{:?}", state.output), "worker_ready": state.worker_ready,
        "calibration": format!("{:?}", state.calibration),
        "scanning": state.scanning, "has_sensor_data": state.latest.is_some(),
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&summary)?)?;
    Ok(path)
}
