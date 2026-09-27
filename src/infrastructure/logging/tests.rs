use super::*;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(super) struct TestDirectory(PathBuf);
impl TestDirectory {
    pub(super) fn new() -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "gear-log-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
    pub(super) fn settings(&self) -> LogSettings {
        LogSettings {
            log_dir: self.0.to_string_lossy().into_owned(),
            max_file_size_bytes: 64 * 1024,
            max_files: 3,
            rotation: "never".into(),
            console_logging_enabled: false,
            ..Default::default()
        }
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn size_rotation_keeps_complete_records_and_bounds_storage() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let settings = dir.settings();
    let mut writer = writer::RotatingWriter::new(&settings, Arc::default(), Arc::default())?;
    let record = format!("{{\"message\":\"{}\"}}\n", "x".repeat(16000));
    for _ in 0..40 {
        writer.write_all(record.as_bytes())?;
    }
    writer.flush()?;
    drop(writer);
    let files: Vec<_> = fs::read_dir(&dir.0)?.collect::<Result<_, _>>()?;
    assert!(files.len() <= settings.max_files);
    let mut bytes = 0;
    for file in files {
        let data = fs::read(file.path())?;
        assert!(data.len() as u64 <= settings.max_file_size_bytes);
        bytes += data.len() as u64;
        for line in std::str::from_utf8(&data)?.lines() {
            let _: serde_json::Value = serde_json::from_str(line)?;
        }
    }
    assert!(bytes <= settings.max_file_size_bytes * settings.max_files as u64);
    Ok(())
}

#[test]
fn retention_handles_legacy_never_files_and_leaves_other_files() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let settings = dir.settings();
    let owned = dir.0.join(format!("{}.log", settings.file_name_prefix));
    let unrelated = dir
        .0
        .join(format!("{}-notes.log", settings.file_name_prefix));
    fs::write(&owned, b"legacy")?;
    fs::write(&unrelated, b"keep")?;
    // Advance the policy's clock without a slow sleep or changing global time.
    retention::prune(
        &settings,
        None,
        SystemTime::now() + Duration::from_secs(15 * 86400),
    )?;
    assert!(!owned.exists());
    assert!(unrelated.exists());
    assert!(!retention::owned_name(
        "gear_vr_controller-other-day-1.log",
        &settings.file_name_prefix
    ));
    Ok(())
}

#[test]
fn retention_accounts_for_oversized_legacy_files() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let settings = dir.settings();
    let old = dir
        .0
        .join(format!("{}-day-1.log", settings.file_name_prefix));
    fs::write(&old, vec![0; settings.max_file_size_bytes as usize * 4])?;
    retention::prune(&settings, None, SystemTime::now())?;
    assert!(!old.exists());
    Ok(())
}

#[test]
fn json_output_escapes_messages_and_shutdown_drains_queue() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let settings = dir.settings();
    let disk = writer::RotatingWriter::new(&settings, Arc::default(), Arc::default())?;
    let (nonblocking, guard) = NonBlockingBuilder::default()
        .buffered_lines_limit(64)
        .finish(disk);
    let oversized = Arc::new(AtomicU64::new(0));
    let bounded = BoundedWriter {
        writer: nonblocking.clone(),
        oversized: oversized.clone(),
    };
    let subscriber = tracing_subscriber::registry().with(
        fmt::layer()
            .json()
            .with_writer(move || bounded.clone())
            .with_ansi(false),
    );
    tracing::subscriber::with_default(subscriber, || {
        let _span = tracing::info_span!("connection", attempt_id = 7).entered();
        tracing::error!(
            event = "connection.failed",
            error = "line one\nline two",
            "Connection failed"
        );
        tracing::info!(event = "test.large", message = %"x".repeat(MAX_EVENT_BYTES * 2));
    });
    drop(guard);
    assert_eq!(oversized.load(Ordering::Relaxed), 1);
    assert_eq!(nonblocking.error_counter().dropped_lines(), 0);
    let mut lines = Vec::new();
    for file in fs::read_dir(&dir.0)? {
        for line in fs::read_to_string(file?.path())?.lines() {
            lines.push(serde_json::from_str::<serde_json::Value>(line)?);
        }
    }
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["level"], "ERROR");
    assert_eq!(lines[0]["fields"]["event"], "connection.failed");
    assert_eq!(lines[0]["fields"]["error"], "line one\nline two");
    assert_eq!(lines[0]["span"]["attempt_id"], 7);
    Ok(())
}

#[test]
fn concurrent_writers_use_different_segments() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let settings = dir.settings();
    let mut first = writer::RotatingWriter::new(&settings, Arc::default(), Arc::default())?;
    let mut second = writer::RotatingWriter::new(&settings, Arc::default(), Arc::default())?;
    first.write_all(b"first\n")?;
    second.write_all(b"second\n")?;
    drop(first);
    drop(second);
    assert_eq!(fs::read_dir(&dir.0)?.count(), 2);
    Ok(())
}

#[test]
fn queue_saturation_drops_records_without_blocking_producer() -> anyhow::Result<()> {
    use std::sync::mpsc;
    struct PausedWriter {
        started: Option<mpsc::Sender<()>>,
        resume: mpsc::Receiver<()>,
    }
    impl Write for PausedWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if let Some(started) = self.started.take() {
                let _ = started.send(());
                self.resume
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(io::Error::other)?;
            }
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let (started_tx, started_rx) = mpsc::channel();
    let (resume_tx, resume_rx) = mpsc::channel();
    let (mut writer, guard) = NonBlockingBuilder::default()
        .buffered_lines_limit(2)
        .lossy(true)
        .finish(PausedWriter {
            started: Some(started_tx),
            resume: resume_rx,
        });
    writer.write_all(b"first\n")?;
    started_rx.recv_timeout(Duration::from_secs(5))?;
    for _ in 0..20 {
        writer.write_all(b"next\n")?;
    }
    let dropped = writer.error_counter().dropped_lines();
    resume_tx.send(())?;
    drop(guard);
    assert_eq!(dropped, 18);
    Ok(())
}

#[test]
fn disk_writer_counts_failed_writes() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let settings = dir.settings();
    let errors = Arc::new(AtomicU64::new(0));
    let mut writer = writer::RotatingWriter::new(&settings, errors.clone(), Arc::default())?;
    assert!(writer
        .write_all(&vec![0; settings.max_file_size_bytes as usize + 1])
        .is_err());
    assert_eq!(errors.load(Ordering::Relaxed), 1);
    Ok(())
}

#[test]
fn relative_log_folders_are_stable_and_absolute_folders_are_preserved() -> anyhow::Result<()> {
    let dir = TestDirectory::new()?;
    let mut settings = dir.settings();
    assert_eq!(log_directory(&settings), dir.0);
    settings.log_dir = "logs".into();
    let resolved = log_directory(&settings);
    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("GearVRController/logs"));
    assert_eq!(settings.log_dir, "logs");
    Ok(())
}
