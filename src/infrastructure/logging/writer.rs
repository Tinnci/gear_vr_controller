use super::retention;
use crate::domain::settings::LogSettings;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Only the appender worker owns this writer; there is no shared disk mutex.
pub(super) struct RotatingWriter {
    settings: LogSettings,
    session: u128,
    sequence: u64,
    bucket: u64,
    file: Option<File>,
    path: Option<PathBuf>,
    bytes: u64,
    last_cleanup: Instant,
    io_errors: Arc<AtomicU64>,
    cleanup_errors: Arc<AtomicU64>,
}
impl RotatingWriter {
    pub(super) fn new(
        settings: &LogSettings,
        io_errors: Arc<AtomicU64>,
        cleanup_errors: Arc<AtomicU64>,
    ) -> io::Result<Self> {
        fs::create_dir_all(&settings.log_dir)?;
        let mut writer = Self {
            settings: settings.clone(),
            session: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            sequence: 0,
            bucket: 0,
            file: None,
            path: None,
            bytes: 0,
            last_cleanup: Instant::now(),
            io_errors,
            cleanup_errors,
        };
        // Surface a bad directory/permissions before starting the worker.
        writer.rotate(current_bucket(&settings.rotation, SystemTime::now()))?;
        Ok(writer)
    }
    fn cleanup(&mut self) {
        if let Err(error) =
            retention::prune(&self.settings, self.path.as_deref(), SystemTime::now())
        {
            self.cleanup_errors.fetch_add(1, Ordering::Relaxed);
            eprintln!("Log retention failed: {error}");
        }
        self.last_cleanup = Instant::now();
    }
    fn rotate(&mut self, bucket: u64) -> io::Result<()> {
        self.flush()?;
        // create_new also protects concurrent instances from sharing/truncating a file.
        loop {
            let kind = match self.settings.rotation.as_str() {
                "hourly" => "hour",
                "minutely" => "minute",
                "never" => "never",
                _ => "day",
            };
            let path = PathBuf::from(&self.settings.log_dir).join(format!(
                "{}-{kind}-{bucket}-{}-{}-{}.log",
                self.settings.file_name_prefix,
                self.session,
                std::process::id(),
                self.sequence
            ));
            self.sequence += 1;
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    self.file = Some(file);
                    self.path = Some(path);
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        self.bucket = bucket;
        self.bytes = 0;
        self.cleanup();
        Ok(())
    }
    fn write_record(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() as u64 > self.settings.max_file_size_bytes {
            return Err(io::Error::other("Log record exceeds segment size"));
        }
        let bucket = current_bucket(&self.settings.rotation, SystemTime::now());
        if bucket != self.bucket
            || self.bytes + buf.len() as u64 > self.settings.max_file_size_bytes
        {
            self.rotate(bucket)?;
        } else if self.last_cleanup.elapsed() >= Duration::from_secs(3600) {
            self.cleanup();
        }
        // Count the attempted bytes conservatively, even if a disk error causes a partial write.
        self.bytes += buf.len() as u64;
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("Log file unavailable"))?
            .write_all(buf)?;
        Ok(buf.len())
    }
}
impl Write for RotatingWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = self.write_record(buf);
        if result.is_err() {
            self.io_errors.fetch_add(1, Ordering::Relaxed);
        }
        result
    }
    fn flush(&mut self) -> io::Result<()> {
        let result = self.file.as_mut().map_or(Ok(()), Write::flush);
        if result.is_err() {
            self.io_errors.fetch_add(1, Ordering::Relaxed);
        }
        result
    }
}
fn current_bucket(rotation: &str, now: SystemTime) -> u64 {
    let seconds = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    match rotation {
        "never" => 0,
        "minutely" => seconds / 60,
        "hourly" => seconds / 3600,
        _ => seconds / 86400,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn time_rotation_opens_new_segment_before_next_record() -> anyhow::Result<()> {
        let dir = super::super::tests::TestDirectory::new()?;
        let mut settings = dir.settings();
        settings.rotation = "daily".into();
        let mut writer = RotatingWriter::new(&settings, Arc::default(), Arc::default())?;
        writer.write_all(b"before\n")?;
        let previous_path = writer
            .path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing log path"))?;
        // Simulate an open segment from the previous UTC day, without waiting for midnight.
        writer.bucket = writer.bucket.saturating_sub(1);
        writer.write_all(b"after\n")?;
        writer.flush()?;
        let next_path = writer
            .path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Missing log path"))?;
        assert_ne!(previous_path, next_path);
        assert_eq!(fs::read(previous_path)?, b"before\n");
        assert_eq!(fs::read(next_path)?, b"after\n");
        Ok(())
    }
}
