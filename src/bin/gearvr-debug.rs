//! Finite hardware probes. JSON lines on stdout; structured logs in a separate directory.
#[path = "gearvr-debug/args.rs"]
mod args;
#[path = "gearvr-debug/commands.rs"]
mod commands;
#[path = "gearvr-debug/output.rs"]
mod output;

use gear_vr_controller_rust::{domain::settings::LogSettings, infrastructure::logging};
use std::{process::ExitCode, time::Duration};
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

struct Apartment;
impl Apartment {
    fn initialize() -> windows::core::Result<Self> {
        // SAFETY: this guard is created and dropped on the same main thread.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: balances the successful initialization on this thread.
        unsafe { RoUninitialize() };
    }
}

fn run(output: &output::Output) -> anyhow::Result<()> {
    let Some(args) = args::Args::parse(std::env::args().skip(1))? else {
        output.emit("help", serde_json::json!({"usage": args::USAGE}))?;
        return Ok(());
    };
    let _apartment = Apartment::initialize()?;
    let mut settings = LogSettings {
        level: "debug".into(),
        file_name_prefix: "gearvr_debug".into(),
        console_logging_enabled: false,
        ..Default::default()
    };
    // Separate retention from GUI logs. RUST_LOG still overrides this process's level.
    settings.log_dir = logging::log_directory(&settings)
        .join("cli")
        .to_string_lossy()
        .into();
    let _logging = logging::init_logger(&settings)?;
    output.emit(
        "started",
        serde_json::json!({
            "command": args.command.name(), "log_directory": settings.log_dir,
            "logging": logging::diagnostics(), "timeout_seconds": args.timeout,
        }),
    )?;
    tracing::info!(
        event = "cli.started",
        command = args.command.name(),
        "Hardware probe started"
    );
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_time().build()?.block_on(async {
            tokio::time::timeout(Duration::from_secs(args.timeout), commands::run(&args, output))
                .await.map_err(|_| anyhow::anyhow!("Probe deadline exceeded; Windows may still be completing a Bluetooth operation"))?
        });
    tracing::info!(
        event = "cli.finished",
        success = result.is_ok(),
        "Hardware probe finished"
    );
    if let Err(error) = &result {
        tracing::error!(event = "cli.failed", error = %format!("{error:#}"), "Hardware probe failed");
    }
    output.emit(
        "logging_health",
        serde_json::to_value(logging::diagnostics())?,
    )?;
    result
}

fn main() -> ExitCode {
    let output = output::Output::new();
    match run(&output) {
        Ok(()) => {
            if output
                .emit("completed", serde_json::json!({"success": true}))
                .is_ok()
            {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            let _ = output.error(&error);
            ExitCode::FAILURE
        }
    }
}
