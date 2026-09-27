//! The same generated icon is used by the executable, window and tray.
use std::sync::OnceLock;
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::HINSTANCE,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{LoadIconW, HICON},
    },
};

pub fn icon_path() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("app-icon.ico")))
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "app-icon.ico".into())
    })
}

pub fn resource_icon() -> windows::core::Result<HICON> {
    // SAFETY: resource 1 is compiled into this module. LoadIcon returns a shared handle.
    unsafe {
        LoadIconW(
            Some(HINSTANCE(GetModuleHandleW(None)?.0)),
            PCWSTR(std::ptr::without_provenance(1)),
        )
    }
}
