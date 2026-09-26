//! System Tray Management
//!
//! Provides a Windows Taskbar Notification Area (System Tray) icon and status tooltip
//! allowing the application to run minimized in the background.

use anyhow::Result;
use std::mem::size_of;
use tracing::{debug, info, warn};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, LoadIconW, RegisterClassW, HICON,
    IDI_APPLICATION, WINDOW_EX_STYLE, WM_USER, WNDCLASSW, WS_OVERLAPPED,
};

const WM_TRAY_CALLBACK: u32 = WM_USER + 101;
const TRAY_CLASS_NAME: PCWSTR = windows::core::w!("GearVRTrayWindowClass");

/// Trait defining Tray icon actions (Interface Segregation)
pub trait TrayController: Send + Sync {
    /// Update the text tooltip displayed when hovering over the tray icon
    fn update_tooltip(&mut self, text: &str) -> Result<()>;
}

/// Win32 System Tray implementation
pub struct WindowsTrayManager {
    hwnd: HWND,
    #[allow(dead_code)]
    icon: HICON,
    nid: NOTIFYICONDATAW,
    is_active: bool,
}

// Safety: Win32 HWND and HICON pointers are thread-safe handles managed by Windows subsystem
unsafe impl Send for WindowsTrayManager {}
unsafe impl Sync for WindowsTrayManager {}

unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

impl WindowsTrayManager {
    pub fn new(app_name: &str) -> Result<Self> {
        unsafe {
            // Register a dummy hidden window class to receive tray icon notification messages
            let wc = WNDCLASSW {
                lpfnWndProc: Some(tray_window_proc),
                lpszClassName: TRAY_CLASS_NAME,
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                TRAY_CLASS_NAME,
                PCWSTR::null(),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
            )?;

            let icon = LoadIconW(None, IDI_APPLICATION)?;

            let mut nid = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_TRAY_CALLBACK,
                hIcon: icon,
                ..Default::default()
            };

            // Set default tooltip text
            let tip_encoded: Vec<u16> = app_name.encode_utf16().chain(std::iter::once(0)).collect();
            let copy_len = tip_encoded.len().min(nid.szTip.len());
            nid.szTip[..copy_len].copy_from_slice(&tip_encoded[..copy_len]);

            let success = Shell_NotifyIconW(NIM_ADD, &nid).as_bool();
            if !success {
                warn!("Shell_NotifyIconW NIM_ADD failed");
                anyhow::bail!("Failed to create system tray icon");
            }

            info!("System tray icon registered successfully");

            Ok(Self {
                hwnd,
                icon,
                nid,
                is_active: true,
            })
        }
    }
}

impl TrayController for WindowsTrayManager {
    fn update_tooltip(&mut self, text: &str) -> Result<()> {
        if !self.is_active {
            return Ok(());
        }

        unsafe {
            let tip_encoded: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let copy_len = tip_encoded.len().min(self.nid.szTip.len());
            self.nid.szTip = [0; 128];
            self.nid.szTip[..copy_len].copy_from_slice(&tip_encoded[..copy_len]);
            self.nid.uFlags = NIF_TIP;

            let ok = Shell_NotifyIconW(NIM_MODIFY, &self.nid).as_bool();
            if !ok {
                debug!("Shell_NotifyIconW NIM_MODIFY failed");
            }
        }
        Ok(())
    }
}

impl Drop for WindowsTrayManager {
    fn drop(&mut self) {
        if self.is_active {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
                if !self.hwnd.0.is_null() {
                    let _ = DestroyWindow(self.hwnd);
                }
            }
            self.is_active = false;
            info!("System tray icon removed");
        }
    }
}
