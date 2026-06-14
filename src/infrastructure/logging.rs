use crate::domain::settings::LogSettings;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*};

pub struct LoggingGuard;

pub fn init_logger(settings: &LogSettings) -> anyhow::Result<LoggingGuard> {
    let level_filter = std::env::var("RUST_LOG")
        .ok()
        .and_then(|level| LevelFilter::from_str(level.trim()).ok())
        .or_else(|| LevelFilter::from_str(&settings.level).ok())
        .unwrap_or(LevelFilter::INFO);

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

    let file_layer = if settings.file_logging_enabled {
        let file_writer = RotatingFileWriter::new(settings)?;
        Some(
            fmt::layer()
                .with_writer(move || file_writer.clone())
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
        .with(level_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    tracing::info!("Logging initialized successfully");

    Ok(LoggingGuard)
}

#[derive(Clone)]
struct RotatingFileWriter {
    state: Arc<Mutex<RotatingFileState>>,
}

impl RotatingFileWriter {
    fn new(settings: &LogSettings) -> anyhow::Result<Self> {
        let state = RotatingFileState {
            dir: PathBuf::from(&settings.log_dir),
            prefix: settings.file_name_prefix.clone(),
            rotation: LogRotation::from_setting(&settings.rotation),
            current_bucket: None,
            file: None,
        };

        Ok(Self {
            state: Arc::new(Mutex::new(state)),
        })
    }
}

impl Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("Log writer lock poisoned"))?;
        state.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| io::Error::other("Log writer lock poisoned"))?;
        state.flush()
    }
}

struct RotatingFileState {
    dir: PathBuf,
    prefix: String,
    rotation: LogRotation,
    current_bucket: Option<u64>,
    file: Option<File>,
}

impl RotatingFileState {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.ensure_file()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(file) = &mut self.file {
            file.flush()
        } else {
            Ok(())
        }
    }

    fn ensure_file(&mut self) -> io::Result<&mut File> {
        let bucket = self.rotation.current_bucket();
        if self.current_bucket != Some(bucket) || self.file.is_none() {
            fs::create_dir_all(&self.dir)?;
            let path = self.log_path(bucket);
            self.file = Some(OpenOptions::new().create(true).append(true).open(path)?);
            self.current_bucket = Some(bucket);
        }

        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("log file was not initialized"))
    }

    fn log_path(&self, bucket: u64) -> PathBuf {
        let file_name = match self.rotation {
            LogRotation::Never => format!("{}.log", self.prefix),
            LogRotation::Minutely => format!("{}-minute-{}.log", self.prefix, bucket),
            LogRotation::Hourly => format!("{}-hour-{}.log", self.prefix, bucket),
            LogRotation::Daily => format!("{}-day-{}.log", self.prefix, bucket),
        };
        self.dir.join(file_name)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LogRotation {
    Never,
    Minutely,
    Hourly,
    Daily,
}

impl LogRotation {
    fn from_setting(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "never" => Self::Never,
            "minutely" => Self::Minutely,
            "hourly" => Self::Hourly,
            _ => Self::Daily,
        }
    }

    fn current_bucket(self) -> u64 {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        match self {
            Self::Never => 0,
            Self::Minutely => seconds / 60,
            Self::Hourly => seconds / 3_600,
            Self::Daily => seconds / 86_400,
        }
    }
}
