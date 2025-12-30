use crate::domain::settings::LogSettings;
use std::str::FromStr;
// use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub struct LoggingGuard {
    // We need to keep this guard alive for logs to be flushed
    _guards: Vec<WorkerGuard>,
}

pub fn init_logger(settings: &LogSettings) -> anyhow::Result<LoggingGuard> {
    let mut guards = Vec::new();

    // Parse log level
    let level_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::from_str(&settings.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Console layer
    let console_layer = if settings.console_logging_enabled {
        Some(fmt::layer().with_writer(std::io::stdout))
    } else {
        None
    };

    // File layer
    let file_layer = if settings.file_logging_enabled {
        let file_appender =
            tracing_appender::rolling::daily(&settings.log_dir, &settings.file_name_prefix);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        guards.push(guard);
        Some(
            fmt::layer().with_writer(non_blocking).with_ansi(false), // File logs shouldn't have ANSI colors
        )
    } else {
        None
    };

    tracing_subscriber::registry()
        .with(level_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("Logging initialized successfully");

    Ok(LoggingGuard { _guards: guards })
}
