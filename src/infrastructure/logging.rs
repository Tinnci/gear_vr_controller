//! Structured events on the caller; bounded disk work on one background thread.
mod retention;
#[cfg(test)]
mod tests;
mod writer;

use crate::domain::settings::LogSettings;
use serde::Serialize;
use std::io::{self, Write};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, OnceLock,
};
use tracing_appender::non_blocking::{ErrorCounter, NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

const QUEUE_CAPACITY: usize = 4096;
const MAX_EVENT_BYTES: usize = 16 * 1024;
static HEALTH: OnceLock<LogHealth> = OnceLock::new();

#[derive(Clone)]
struct LogHealth {
    configuration: LoggingConfiguration,
    dropped: Option<ErrorCounter>,
    oversized: Arc<AtomicU64>,
    io_errors: Arc<AtomicU64>,
    cleanup_errors: Arc<AtomicU64>,
}

#[derive(Clone, Serialize)]
pub struct LoggingConfiguration {
    pub file_logging_enabled: bool,
    pub filter: String,
    pub rotation: String,
    pub retention_days: u32,
    pub max_files: usize,
    pub max_file_size_bytes: u64,
}

#[derive(Default, Serialize)]
pub struct LoggingDiagnostics {
    pub configuration: Option<LoggingConfiguration>,
    pub dropped_events: usize,
    pub oversized_events: u64,
    pub io_errors: u64,
    pub cleanup_errors: u64,
}

pub fn diagnostics() -> LoggingDiagnostics {
    HEALTH
        .get()
        .map_or_else(LoggingDiagnostics::default, |health| LoggingDiagnostics {
            configuration: Some(health.configuration.clone()),
            dropped_events: health
                .dropped
                .as_ref()
                .map_or(0, ErrorCounter::dropped_lines),
            oversized_events: health.oversized.load(Ordering::Relaxed),
            io_errors: health.io_errors.load(Ordering::Relaxed),
            cleanup_errors: health.cleanup_errors.load(Ordering::Relaxed),
        })
}

/// Keep this alive until producers stop. Normal drop drains the file queue.
pub struct LoggingGuard {
    _worker: Option<WorkerGuard>,
}

impl Drop for LoggingGuard {
    fn drop(&mut self) {
        let health = diagnostics();
        tracing::info!(
            event = "app.stopped",
            dropped_events = health.dropped_events,
            oversized_events = health.oversized_events,
            io_errors = health.io_errors,
            cleanup_errors = health.cleanup_errors,
            "Application stopped"
        );
    }
}

pub fn init_logger(settings: &LogSettings) -> anyhow::Result<LoggingGuard> {
    // Disable regex interpretation of field filters; module/level directives remain available.
    let directive = std::env::var("RUST_LOG").unwrap_or_else(|_| settings.level.clone());
    let (filter, filter_warning) = match EnvFilter::builder().with_regex(false).parse(directive) {
        Ok(filter) => (filter, None),
        Err(error) => (
            EnvFilter::builder()
                .with_regex(false)
                .parse(&settings.level)?,
            Some(error.to_string()),
        ),
    };
    let console_layer = settings.console_logging_enabled.then(|| {
        fmt::layer()
            .with_writer(std::io::stdout)
            .with_file(settings.show_file_line)
            .with_line_number(settings.show_file_line)
            .with_thread_ids(settings.show_thread_ids)
            .with_target(settings.show_target)
            .with_ansi(settings.ansi_colors)
    });
    let mut health = LogHealth {
        configuration: LoggingConfiguration {
            file_logging_enabled: settings.file_logging_enabled,
            filter: filter.to_string(),
            rotation: settings.rotation.clone(),
            retention_days: settings.retention_days,
            max_files: settings.max_files,
            max_file_size_bytes: settings.max_file_size_bytes,
        },
        dropped: None,
        oversized: Arc::default(),
        io_errors: Arc::default(),
        cleanup_errors: Arc::default(),
    };
    let mut worker_guard = None;
    let file_layer = if settings.file_logging_enabled {
        let writer = writer::RotatingWriter::new(
            settings,
            health.io_errors.clone(),
            health.cleanup_errors.clone(),
        )?;
        let (writer, guard) = NonBlockingBuilder::default()
            .buffered_lines_limit(QUEUE_CAPACITY)
            .lossy(true)
            .thread_name("log-writer")
            .finish(writer);
        health.dropped = Some(writer.error_counter());
        worker_guard = Some(guard);
        let bounded = BoundedWriter {
            writer,
            oversized: health.oversized.clone(),
        };
        Some(
            fmt::layer()
                .json()
                .with_writer(move || bounded.clone())
                .with_ansi(false)
                .with_file(settings.show_file_line)
                .with_line_number(settings.show_file_line)
                .with_thread_ids(settings.show_thread_ids)
                .with_target(settings.show_target),
        )
    } else {
        None
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .try_init()?;
    let _ = HEALTH.set(health);
    if let Some(error) = filter_warning {
        tracing::warn!(event = "logging.filter.invalid", %error, "Using configured log level after invalid RUST_LOG");
    }
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(event = "app.panic", error = %info, "Application panicked");
        previous_hook(info);
    }));
    tracing::info!(event = "app.started", version = env!("CARGO_PKG_VERSION"),
        pid = std::process::id(), file_logging = settings.file_logging_enabled,
        rotation = %settings.rotation, retention_days = settings.retention_days,
        max_file_size_bytes = settings.max_file_size_bytes, max_files = settings.max_files,
        "Application started");
    Ok(LoggingGuard {
        _worker: worker_guard,
    })
}

#[derive(Clone)]
struct BoundedWriter {
    writer: NonBlocking,
    oversized: Arc<AtomicU64>,
}
impl Write for BoundedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Drop the whole record rather than producing truncated, invalid JSON.
        if buf.len() > MAX_EVENT_BYTES {
            self.oversized.fetch_add(1, Ordering::Relaxed);
            return Ok(buf.len());
        }
        self.writer.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
