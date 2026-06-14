use crate::admin_ipc::{NamedPipe, PIPE_NAME};
use crate::admin_worker::{AdminCommand, AdminResponse};
use anyhow::Result;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use tracing::info;
use windows::core::PCWSTR;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

pub struct AdminClient {
    stream: Option<NamedPipe>,
}

impl AdminClient {
    pub fn new() -> Self {
        Self { stream: None }
    }

    /// Try to connect to the running admin worker.
    /// If not running, returns Ok(false).
    pub fn try_connect(&mut self) -> Result<bool> {
        if self.stream.is_some() {
            return Ok(true);
        }

        match NamedPipe::connect(PIPE_NAME) {
            Ok(stream) => {
                info!("Connected to Admin Worker!");
                self.stream = Some(stream);
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }

    /// Wait for the worker to become available (polling)
    pub fn wait_for_worker(&mut self, timeout_ms: u64) -> Result<bool> {
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_millis(timeout_ms) {
            if self.try_connect().unwrap_or(false) {
                return Ok(true);
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        Ok(false)
    }

    /// Launch the admin worker with UAC prompt
    pub fn launch_worker(&self) -> Result<()> {
        info!("Requesting UAC elevation to start worker...");

        let exe_path = std::env::current_exe()?;
        let operation = wide_string("runas");
        let file = wide_os_string(exe_path.as_os_str());
        let parameters = wide_string("--admin-worker");

        let result = unsafe {
            ShellExecuteW(
                None,
                PCWSTR(operation.as_ptr()),
                PCWSTR(file.as_ptr()),
                PCWSTR(parameters.as_ptr()),
                PCWSTR::null(),
                SW_HIDE,
            )
        };

        if result.0 as isize <= 32 {
            anyhow::bail!(
                "Failed to launch elevated worker: ShellExecuteW returned {:?}",
                result
            );
        }

        Ok(())
    }

    /// Send a command to the worker and await response
    pub fn send_command(&mut self, cmd: AdminCommand) -> Result<AdminResponse> {
        if self.stream.is_none() {
            // Try one last reconnect
            if !self.try_connect()? {
                anyhow::bail!("Not connected to Admin Worker");
            }
        }

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected to Admin Worker"))?;
        let json_cmd = serde_json::to_string(&cmd)?;
        stream.write_line(&json_cmd)?;

        let buffer = stream
            .read_line()?
            .ok_or_else(|| anyhow::anyhow!("Admin Worker closed the pipe"))?;
        let response: AdminResponse = serde_json::from_str(&buffer)?;
        Ok(response)
    }

    /// Helper: Restart Bluetooth Service
    pub fn restart_bluetooth_service(&mut self) -> Result<String> {
        match self.send_command(AdminCommand::RestartBluetoothService)? {
            AdminResponse::Success(msg) => Ok(msg),
            AdminResponse::Error(e) => anyhow::bail!("Service restart failed: {}", e),
            _ => anyhow::bail!("Unexpected response"),
        }
    }

    /// Helper: Nuke Ghost Device
    #[allow(dead_code)]
    pub fn remove_ghost_device(&mut self, instance_id: &str) -> Result<String> {
        match self.send_command(AdminCommand::RemoveGhostDevice(instance_id.to_string()))? {
            AdminResponse::Success(msg) => Ok(msg),
            AdminResponse::Error(e) => anyhow::bail!("Device removal failed: {}", e),
            _ => anyhow::bail!("Unexpected response"),
        }
    }
}

fn wide_string(value: &str) -> Vec<u16> {
    wide_os_string(OsStr::new(value))
}

fn wide_os_string(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}
