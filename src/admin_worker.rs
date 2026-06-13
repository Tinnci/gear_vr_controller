use crate::admin_ipc::{NamedPipe, PIPE_NAME};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{error, info};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{
    ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_NOT_ACTIVE, WIN32_ERROR,
};
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx,
    StartServiceW, SC_HANDLE, SC_MANAGER_CONNECT, SC_STATUS_PROCESS_INFO, SERVICE_CONTROL_STOP,
    SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START, SERVICE_STATUS,
    SERVICE_STATUS_CURRENT_STATE, SERVICE_STATUS_PROCESS, SERVICE_STOP, SERVICE_STOPPED,
};
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SYSTEMMODAL,
};

#[derive(Serialize, Deserialize, Debug)]
pub enum AdminCommand {
    Ping,
    RemoveGhostDevice(String), // InstanceId
    RestartBluetoothService,
    Quit,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AdminResponse {
    Pong,
    Success(String),
    Error(String),
}

/// Run the admin worker loop (this runs in the Elevated process)
pub fn run_admin_worker() -> Result<()> {
    // Setup logging for the worker (maybe to a file since no console)
    let log_path = std::env::temp_dir().join("gear_vr_admin.log");
    let file = std::fs::File::create(log_path).ok();
    tracing_subscriber::fmt()
        .with_writer(move || -> Box<dyn std::io::Write + Send + Sync> {
            if let Some(f) = file.as_ref().and_then(|f| f.try_clone().ok()) {
                Box::new(f)
            } else {
                Box::new(std::io::stdout())
            }
        })
        .init();

    let _ = show_msgbox("Gear VR Controller", "Admin Diagnostic Assistant Started.\n\nPlease wait for commands from the main application.");
    info!("Admin worker started");

    info!("Listening on named pipe...");

    loop {
        let pipe = NamedPipe::create_server(PIPE_NAME)?;
        pipe.wait_for_client()?;
        info!("Client connected");
        if let Err(e) = handle_connection(pipe) {
            error!("Connection error: {}", e);
        }
    }
}

fn handle_connection(mut stream: NamedPipe) -> Result<()> {
    loop {
        match stream.read_line()? {
            None => break,
            Some(buffer) => {
                if let Ok(cmd) = serde_json::from_str::<AdminCommand>(&buffer) {
                    info!("Received command: {:?}", cmd);
                    let should_quit = matches!(cmd, AdminCommand::Quit);
                    let response = execute_command(cmd);

                    if let AdminResponse::Success(ref msg) = response {
                        let _ = show_msgbox("Admin Action Success", msg);
                    } else if let AdminResponse::Error(ref err) = response {
                        let _ = show_msgbox("Admin Action Failed", err);
                    }

                    let json = serde_json::to_string(&response)?;
                    stream.write_line(&json)?;

                    if should_quit {
                        std::process::exit(0);
                    }
                }
            }
        }
    }
    stream.disconnect();
    Ok(())
}

fn execute_command(cmd: AdminCommand) -> AdminResponse {
    match cmd {
        AdminCommand::Ping => AdminResponse::Pong,
        AdminCommand::RemoveGhostDevice(instance_id) => {
            info!("Removing device: {}", instance_id);
            // pnputil /remove-device "InstanceID"
            match Command::new("pnputil")
                .args(["/remove-device", &instance_id])
                .output()
            {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if output.status.success() {
                        AdminResponse::Success(stdout.to_string())
                    } else {
                        AdminResponse::Error(stdout.to_string())
                    }
                }
                Err(e) => AdminResponse::Error(e.to_string()),
            }
        }
        AdminCommand::RestartBluetoothService => {
            info!("Restarting Bluetooth service...");
            match restart_windows_service("bthserv") {
                Ok(()) => AdminResponse::Success("Bluetooth service restarted".to_string()),
                Err(e) => AdminResponse::Error(e.to_string()),
            }
        }
        AdminCommand::Quit => AdminResponse::Success("Quitting".to_string()),
    }
}

fn restart_windows_service(service_name: &str) -> Result<()> {
    let scm = ServiceHandle::new(unsafe {
        OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT)?
    });

    let service_name = wide_string(service_name);
    let service = ServiceHandle::new(unsafe {
        OpenServiceW(
            scm.raw(),
            PCWSTR(service_name.as_ptr()),
            SERVICE_QUERY_STATUS | SERVICE_STOP | SERVICE_START,
        )?
    });

    if query_service_state(service.raw())? != SERVICE_STOPPED {
        let mut status = SERVICE_STATUS::default();
        match unsafe { ControlService(service.raw(), SERVICE_CONTROL_STOP, &mut status) } {
            Ok(()) => wait_for_service_state(service.raw(), SERVICE_STOPPED)?,
            Err(e) if WIN32_ERROR::from_error(&e) == Some(ERROR_SERVICE_NOT_ACTIVE) => {}
            Err(e) => return Err(e.into()),
        }
    }

    match unsafe { StartServiceW(service.raw(), None) } {
        Ok(()) => wait_for_service_state(service.raw(), SERVICE_RUNNING),
        Err(e) if WIN32_ERROR::from_error(&e) == Some(ERROR_SERVICE_ALREADY_RUNNING) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

fn wait_for_service_state(
    service: SC_HANDLE,
    expected: SERVICE_STATUS_CURRENT_STATE,
) -> Result<()> {
    let started = Instant::now();
    let timeout = Duration::from_secs(15);

    while started.elapsed() < timeout {
        if query_service_state(service)? == expected {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(250));
    }

    anyhow::bail!("Timed out waiting for service state {:?}", expected)
}

fn query_service_state(service: SC_HANDLE) -> Result<SERVICE_STATUS_CURRENT_STATE> {
    let mut status = SERVICE_STATUS_PROCESS::default();
    let mut bytes_needed = 0u32;
    let buffer = unsafe {
        std::slice::from_raw_parts_mut(
            (&mut status as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
            size_of::<SERVICE_STATUS_PROCESS>(),
        )
    };

    unsafe {
        QueryServiceStatusEx(
            service,
            SC_STATUS_PROCESS_INFO,
            Some(buffer),
            &mut bytes_needed,
        )?;
    }

    Ok(status.dwCurrentState)
}

struct ServiceHandle(SC_HANDLE);

impl ServiceHandle {
    fn new(handle: SC_HANDLE) -> Self {
        Self(handle)
    }

    fn raw(&self) -> SC_HANDLE {
        self.0
    }
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseServiceHandle(self.0) };
    }
}

fn wide_string(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn show_msgbox(title: &str, body: &str) -> i32 {
    let title_wide: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let body_wide: Vec<u16> = OsStr::new(body)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        MessageBoxW(
            None,
            windows::core::PCWSTR(body_wide.as_ptr()),
            windows::core::PCWSTR(title_wide.as_ptr()),
            MB_OK | MB_ICONINFORMATION | MB_SYSTEMMODAL,
        )
        .0 as i32
    }
}
