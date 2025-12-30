use crate::admin_worker::{AdminCommand, AdminResponse, PIPE_NAME};
use anyhow::{Context, Result};
use interprocess::local_socket::{
    traits::Stream, GenericFilePath, Stream as LocalStream, ToFsName,
};
use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use tracing::info;

pub struct AdminClient {
    stream: Option<LocalStream>,
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

        let pipe_name = PIPE_NAME.to_fs_name::<GenericFilePath>()?;
        match LocalStream::connect(pipe_name) {
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
        info!("Requesting legacy UAC elevation to start worker...");

        let exe_path = std::env::current_exe()?;
        let exe_str = exe_path.to_str().context("Invalid exe path")?;

        // Use ShellExecute via PowerShell to trigger UAC "RunAs"
        // This is a standard trick to elevate from code without linking shell32.lib directly just for this.
        let ps_script = format!(
            "Start-Process -FilePath '{}' -ArgumentList '--admin-worker' -Verb RunAs -WindowStyle Hidden",
            exe_str
        );

        Command::new("powershell")
            .args(&["-Command", &ps_script])
            .spawn()
            .context("Failed to launch elevated worker")?;

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

        let stream = self.stream.as_mut().unwrap();

        // Serialize command
        let json_cmd = serde_json::to_string(&cmd)? + "\n";

        // Write
        stream.write_all(json_cmd.as_bytes())?;
        stream.flush()?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut buffer = String::new();
        reader.read_line(&mut buffer)?;

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
    pub fn remove_ghost_device(&mut self, instance_id: &str) -> Result<String> {
        match self.send_command(AdminCommand::RemoveGhostDevice(instance_id.to_string()))? {
            AdminResponse::Success(msg) => Ok(msg),
            AdminResponse::Error(e) => anyhow::bail!("Device removal failed: {}", e),
            _ => anyhow::bail!("Unexpected response"),
        }
    }
}
