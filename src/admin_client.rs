//! Launch a single authenticated UAC helper for Bluetooth service recovery.
use crate::{
    admin_ipc::{wide, NamedPipe},
    admin_worker::{AdminCommand, AdminResponse},
};
use std::{ffi::OsStr, time::Duration};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::CloseHandle,
        System::Threading::GetProcessId,
        UI::{
            Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW},
            WindowsAndMessaging::SW_HIDE,
        },
    },
};

pub fn recover_bluetooth(cancel: &windows_reactor::CancellationToken) -> anyhow::Result<String> {
    let id = windows::core::GUID::new()?;
    let pipe_name = format!(r"\\.\pipe\gear_vr_recovery_{}_{id:?}", std::process::id());
    let mut pipe = NamedPipe::create_server(&pipe_name)?;
    let exe = wide(std::env::current_exe()?.as_os_str());
    let operation = wide(OsStr::new("runas"));
    let arguments = wide(OsStr::new(&format!(
        "--admin-worker --pipe \"{pipe_name}\" --parent-pid {}",
        std::process::id()
    )));
    let mut launch = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(operation.as_ptr()),
        lpFile: PCWSTR(exe.as_ptr()),
        lpParameters: PCWSTR(arguments.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    // SAFETY: launch and its null-terminated strings remain alive throughout ShellExecuteExW.
    unsafe {
        ShellExecuteExW(&mut launch)?;
    }
    let result = (|| {
        let pid = unsafe { GetProcessId(launch.hProcess) };
        anyhow::ensure!(pid != 0, "Cannot identify elevated helper");
        pipe.wait_for_client(Duration::from_secs(15), || cancel.is_cancelled())?;
        pipe.verify_peer(pid, false)?;
        pipe.write_line(&serde_json::to_string(
            &AdminCommand::RestartBluetoothService,
        )?)?;
        let response: AdminResponse = serde_json::from_str(
            &pipe.read_line(Duration::from_secs(40), || cancel.is_cancelled())?,
        )?;
        match response {
            AdminResponse::Success(message) => Ok(message),
            AdminResponse::Error(error) => anyhow::bail!(error),
        }
    })();
    unsafe {
        let _ = CloseHandle(launch.hProcess);
    }
    result
}
