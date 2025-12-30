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
        Some(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_file(settings.show_file_line)
                .with_line_number(settings.show_file_line)
                .with_thread_ids(settings.show_thread_ids)
                .with_target(settings.show_target)
                .with_ansi(settings.ansi_colors),
        )
    } else {
        None
    };

    // File layer
    let file_layer = if settings.file_logging_enabled {
        let rotation = match settings.rotation.to_lowercase().as_str() {
            "hourly" => tracing_appender::rolling::Rotation::HOURLY,
            "minutely" => tracing_appender::rolling::Rotation::MINUTELY,
            "never" => tracing_appender::rolling::Rotation::NEVER,
            _ => tracing_appender::rolling::Rotation::DAILY,
        };

        let file_appender = tracing_appender::rolling::RollingFileAppender::new(
            rotation,
            &settings.log_dir,
            &settings.file_name_prefix,
        );
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
        guards.push(guard);
        Some(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false) // File logs shouldn't have ANSI colors
                .with_file(settings.show_file_line)
                .with_line_number(settings.show_file_line)
                .with_thread_ids(settings.show_thread_ids)
                .with_target(settings.show_target),
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
