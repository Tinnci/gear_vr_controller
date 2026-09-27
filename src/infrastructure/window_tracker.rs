//! Foreground Window Tracking
//!
//! Provides OS-level detection of the current foreground application window
//! to support context-aware automatic profile and control mode switching.

use std::path::Path;
use std::time::{Duration, Instant};
use tracing::trace;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Trait abstracting foreground window inspection (Dependency Inversion Principle)
pub trait ForegroundWatcher: Send + Sync {
    /// Returns the executable name (in lowercase, e.g. "powerpnt.exe") of the active window
    fn get_foreground_process_name(&mut self) -> Option<String>;
}

/// Windows Win32 implementation of `ForegroundWatcher` with throttle caching
pub struct WindowsForegroundWatcher {
    last_check: Option<Instant>,
    cached_name: Option<String>,
    throttle_duration: Duration,
}

impl WindowsForegroundWatcher {
    pub fn new(throttle_duration: Duration) -> Self {
        Self {
            last_check: None,
            cached_name: None,
            throttle_duration,
        }
    }

    /// Query the OS for the current foreground window's executable name
    fn query_os_foreground_process() -> Option<String> {
        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }

            let mut process_id: u32 = 0;
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            if process_id == 0 {
                return None;
            }

            let Ok(process_handle) =
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process_id)
            else {
                return None;
            };

            let mut buffer = [0u16; 1024];
            let mut size = buffer.len() as u32;

            let success = QueryFullProcessImageNameW(
                process_handle,
                PROCESS_NAME_FORMAT(0),
                PWSTR(buffer.as_mut_ptr()),
                &mut size,
            );

            let _ = CloseHandle(process_handle);

            if success.is_err() || size == 0 {
                return None;
            }

            let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
            let exe_name = Path::new(&full_path)
                .file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.to_ascii_lowercase());

            trace!("Foreground process detected: {:?}", exe_name);
            exe_name
        }
    }
}

impl Default for WindowsForegroundWatcher {
    fn default() -> Self {
        Self::new(Duration::from_millis(500))
    }
}

impl ForegroundWatcher for WindowsForegroundWatcher {
    fn get_foreground_process_name(&mut self) -> Option<String> {
        let now = Instant::now();
        if let Some(last) = self.last_check {
            if now.duration_since(last) < self.throttle_duration {
                return self.cached_name.clone();
            }
        }

        self.last_check = Some(now);
        self.cached_name = Self::query_os_foreground_process();
        self.cached_name.clone()
    }
}
