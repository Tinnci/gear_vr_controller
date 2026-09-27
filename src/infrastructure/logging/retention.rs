//! Retention only touches regular files with this application's exact naming scheme.
use crate::domain::settings::LogSettings;
use std::{
    fs, io,
    path::Path,
    time::{Duration, SystemTime},
};

pub(super) fn owned_name(name: &str, prefix: &str) -> bool {
    if name == format!("{prefix}.log") {
        return true;
    }
    let Some(rest) = name
        .strip_prefix(prefix)
        .and_then(|s| s.strip_prefix('-'))
        .and_then(|s| s.strip_suffix(".log"))
    else {
        return false;
    };
    let parts: Vec<_> = rest.split('-').collect();
    matches!(parts.first(), Some(&"day" | &"hour" | &"minute" | &"never"))
        && matches!(parts.len(), 2 | 5)
        && parts[1..]
            .iter()
            .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
}

pub(super) fn prune(
    settings: &LogSettings,
    active: Option<&Path>,
    now: SystemTime,
) -> io::Result<()> {
    let cutoff = now.checked_sub(Duration::from_secs(
        u64::from(settings.retention_days) * 86400,
    ));
    let mut files = Vec::new();
    for entry in fs::read_dir(&settings.log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if active == Some(path.as_path())
            || !entry.file_type()?.is_file()
            || !owned_name(
                &entry.file_name().to_string_lossy(),
                &settings.file_name_prefix,
            )
        {
            continue;
        }
        let metadata = entry.metadata()?;
        let modified = metadata.modified()?;
        if cutoff.is_some_and(|cutoff| modified < cutoff) {
            fs::remove_file(path)?;
        } else {
            files.push((modified, path, metadata.len()));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    // Reserve one whole segment for the active writer, including on startup.
    let budget = settings.max_file_size_bytes * settings.max_files.saturating_sub(1) as u64;
    let mut bytes: u64 = files.iter().map(|entry| entry.2).sum();
    let mut count = files.len();
    for (_, path, size) in files {
        if count < settings.max_files && bytes <= budget {
            break;
        }
        fs::remove_file(path)?;
        count -= 1;
        bytes -= size;
    }
    Ok(())
}
