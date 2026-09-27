//! Authenticated single-action elevated helper; exits after one response.
use crate::admin_ipc::NamedPipe;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsStr,
    mem::size_of,
    os::windows::ffi::OsStrExt,
    thread,
    time::{Duration, Instant},
};
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

#[derive(Serialize, Deserialize, Debug)]
pub enum AdminCommand {
    RestartBluetoothService,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum AdminResponse {
    Success(String),
    Error(String),
}

pub fn run_admin_worker() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let argument = |name: &str| -> Result<&str> {
        args.windows(2)
            .find(|pair| pair[0] == name)
            .map(|pair| pair[1].as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing helper argument {name}"))
    };
    let parent: u32 = argument("--parent-pid")?.parse()?;
    let name = argument("--pipe")?;
    anyhow::ensure!(
        name.starts_with(&format!(r"\\.\pipe\gear_vr_recovery_{parent}_")),
        "Invalid recovery pipe"
    );
    let mut pipe = NamedPipe::connect(name)?;
    pipe.verify_peer(parent, true)?;
    let command: AdminCommand =
        serde_json::from_str(&pipe.read_line(Duration::from_secs(15), || false)?)?;
    let response = match command {
        AdminCommand::RestartBluetoothService => match restart_windows_service("bthserv") {
            Ok(()) => AdminResponse::Success("Bluetooth service restarted".to_string()),
            Err(error) => AdminResponse::Error(error.to_string()),
        },
    };
    pipe.write_line(&serde_json::to_string(&response)?)?;
    Ok(())
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
