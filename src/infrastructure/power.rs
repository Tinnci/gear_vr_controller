//! Power Management & Sleep Prevention
//!
//! Provides system-level power assertions to prevent the display from sleeping
//! or the machine from entering standby during presentations or active controller usage.

use anyhow::Result;
use tracing::{debug, info, warn};
use windows::Win32::System::Power::{
    SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED,
    EXECUTION_STATE,
};

/// Trait defining power inhibition capabilities (Interface Segregation / Dependency Inversion)
pub trait PowerInhibitor: Send + Sync {
    /// Prevent display turn-off and system sleep
    fn prevent_sleep(&mut self) -> Result<()>;

    /// Restore standard OS power-saving behavior
    fn allow_sleep(&mut self) -> Result<()>;

    /// Whether sleep prevention is currently active
    #[allow(dead_code)]
    fn is_preventing(&self) -> bool;
}

/// Windows Win32 implementation of `PowerInhibitor`
pub struct WindowsPowerManager {
    is_preventing: bool,
}

impl WindowsPowerManager {
    pub fn new() -> Self {
        Self {
            is_preventing: false,
        }
    }
}

impl Default for WindowsPowerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerInhibitor for WindowsPowerManager {
    fn prevent_sleep(&mut self) -> Result<()> {
        if self.is_preventing {
            return Ok(());
        }

        unsafe {
            let flags = ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED;
            let prev = SetThreadExecutionState(flags);
            if prev == EXECUTION_STATE(0) {
                warn!("SetThreadExecutionState failed to prevent sleep");
                anyhow::bail!("Failed to set thread execution state to prevent sleep");
            }
        }

        self.is_preventing = true;
        info!("System power state locked: sleep and display turn-off prevented");
        Ok(())
    }

    fn allow_sleep(&mut self) -> Result<()> {
        if !self.is_preventing {
            return Ok(());
        }

        unsafe {
            let prev = SetThreadExecutionState(ES_CONTINUOUS);
            if prev == EXECUTION_STATE(0) {
                warn!("SetThreadExecutionState failed to restore sleep");
                anyhow::bail!("Failed to reset thread execution state to continuous");
            }
        }

        self.is_preventing = false;
        debug!("System power state unlocked: normal sleep behavior restored");
        Ok(())
    }

    fn is_preventing(&self) -> bool {
        self.is_preventing
    }
}

impl Drop for WindowsPowerManager {
    fn drop(&mut self) {
        if self.is_preventing {
            let _ = self.allow_sleep();
        }
    }
}
