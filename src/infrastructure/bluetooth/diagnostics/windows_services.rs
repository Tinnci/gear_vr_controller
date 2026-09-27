//! Fixed local service queries. Handles request no start/stop/configuration rights.
use windows::{
    core::{w, PCWSTR},
    Win32::System::Services::{
        CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatus, SC_HANDLE,
        SC_MANAGER_CONNECT, SERVICE_CONTINUE_PENDING, SERVICE_PAUSED, SERVICE_PAUSE_PENDING,
        SERVICE_QUERY_STATUS, SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_STATUS,
        SERVICE_STATUS_CURRENT_STATE, SERVICE_STOPPED, SERVICE_STOP_PENDING,
    },
};

struct ServiceHandle(SC_HANDLE);
impl Drop for ServiceHandle {
    fn drop(&mut self) {
        // SAFETY: this guard owns one successfully opened SCM/service handle.
        unsafe {
            let _ = CloseServiceHandle(self.0);
        }
    }
}

pub(super) fn snapshot(stage: &str) {
    // SAFETY: null machine/database selects the local default SCM; all handles
    // are owned by guards, service names are static, and status storage is valid.
    let manager = unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) };
    let Some(manager) = super::property(stage, "service_manager", manager).map(ServiceHandle)
    else {
        return;
    };
    for (name, native_name) in [("bthserv", w!("bthserv")), ("BthLEEnum", w!("BthLEEnum"))] {
        let service = unsafe { OpenServiceW(manager.0, native_name, SERVICE_QUERY_STATUS) };
        let Some(service) = super::property(stage, name, service).map(ServiceHandle) else {
            continue;
        };
        let mut status = SERVICE_STATUS::default();
        if super::property(stage, name, unsafe {
            QueryServiceStatus(service.0, &mut status)
        })
        .is_none()
        {
            continue;
        }
        tracing::debug!(
            event = "ble.windows_service.snapshot",
            stage,
            service = name,
            state = state_name(status.dwCurrentState),
            state_code = status.dwCurrentState.0,
            win32_exit_code = status.dwWin32ExitCode,
            service_exit_code = status.dwServiceSpecificExitCode,
            checkpoint = status.dwCheckPoint,
            wait_hint_ms = status.dwWaitHint,
            "Windows Bluetooth service observed"
        );
    }
}

fn state_name(state: SERVICE_STATUS_CURRENT_STATE) -> &'static str {
    match state {
        SERVICE_STOPPED => "Stopped",
        SERVICE_START_PENDING => "StartPending",
        SERVICE_STOP_PENDING => "StopPending",
        SERVICE_RUNNING => "Running",
        SERVICE_CONTINUE_PENDING => "ContinuePending",
        SERVICE_PAUSE_PENDING => "PausePending",
        SERVICE_PAUSED => "Paused",
        _ => "Unknown",
    }
}
