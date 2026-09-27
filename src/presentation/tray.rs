//! UI-thread-owned tray icon; minimizing hides the window, closing exits normally.
use crate::domain::{
    i18n::Language,
    models::{ConnectionStatus, ControlMode, OutputTarget},
};
use crate::presentation::winui::text::{mode_text, Text};
use std::cell::Cell;
use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::Threading::GetCurrentThreadId,
        UI::{
            Shell::{
                DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass, Shell_NotifyIconW,
                NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
            },
            WindowsAndMessaging::*,
        },
    },
};

const CALLBACK: u32 = WM_USER + 101;
const SUBCLASS: usize = 0x475652;
struct TrayState {
    main: Cell<HWND>,
    enabled: Cell<bool>,
    exit: Cell<bool>,
    pause_requested: Cell<bool>,
    language: Cell<Language>,
    mode: Cell<ControlMode>,
    connected: Cell<bool>,
    output: Cell<OutputTarget>,
}
pub struct WindowsTrayManager {
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    state: Box<TrayState>,
    tooltip: String,
}

unsafe extern "system" fn main_proc(
    hwnd: HWND,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    // SAFETY: state belongs to the UI-thread manager and the subclass is removed before drop.
    let state = &*(data as *mut TrayState);
    if message == WM_SIZE && wp.0 == SIZE_MINIMIZED as usize && state.enabled.get() {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
    DefSubclassProc(hwnd, message, wp, lp)
}
unsafe extern "system" fn find_main(hwnd: HWND, data: LPARAM) -> windows::core::BOOL {
    let state = &*(data.0 as *mut TrayState);
    if IsWindowVisible(hwnd).as_bool()
        && GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| owner.0.is_null())
    {
        state.main.set(hwnd);
        return false.into();
    }
    true.into()
}
unsafe extern "system" fn tray_proc(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let data = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if message == CALLBACK && data != 0 {
        let state = &*(data as *mut TrayState);
        match lp.0 as u32 {
            WM_LBUTTONUP | WM_LBUTTONDBLCLK => restore(state.main.get()),
            WM_RBUTTONUP => {
                if let Ok(menu) = CreatePopupMenu() {
                    let language = state.language.get();
                    let label =
                        |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
                    let status = label(mode_text(state.mode.get()).get(language));
                    let _ = AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, PCWSTR(status.as_ptr()));
                    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
                    for (id, text) in [
                        (1, Text::Open),
                        (
                            3,
                            if state.output.get() == OutputTarget::Paused {
                                Text::Resume
                            } else {
                                Text::Pause
                            },
                        ),
                        (2, Text::Exit),
                    ] {
                        let text = label(text.get(language));
                        let flags = if id == 3 && !state.connected.get() {
                            MF_STRING | MF_GRAYED
                        } else {
                            MF_STRING
                        };
                        let _ = AppendMenuW(menu, flags, id, PCWSTR(text.as_ptr()));
                    }
                    let mut point = windows::Win32::Foundation::POINT::default();
                    let _ = GetCursorPos(&mut point);
                    let _ = SetForegroundWindow(hwnd);
                    let choice = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_RIGHTBUTTON,
                        point.x,
                        point.y,
                        Some(0),
                        hwnd,
                        None,
                    )
                    .0;
                    if choice == 1 {
                        restore(state.main.get());
                    }
                    if choice == 3 {
                        state.pause_requested.set(true);
                    }
                    if choice == 2 {
                        state.exit.set(true);
                        restore(state.main.get());
                    }
                    let _ = DestroyMenu(menu);
                    let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
                }
            }
            _ => {}
        }
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
unsafe fn restore(hwnd: HWND) {
    if !hwnd.0.is_null() {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}
impl WindowsTrayManager {
    pub fn new(name: &str) -> anyhow::Result<Self> {
        // SAFETY: all HWND and callback state is created, accessed and destroyed on the UI thread.
        unsafe {
            let class = WNDCLASSW {
                lpfnWndProc: Some(tray_proc),
                lpszClassName: w!("GearVRTrayWindow"),
                ..Default::default()
            };
            let _ = RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class.lpszClassName,
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
            let state = Box::new(TrayState {
                main: Cell::new(HWND::default()),
                enabled: Cell::new(true),
                exit: Cell::new(false),
                pause_requested: Cell::new(false),
                language: Cell::new(Language::Auto),
                mode: Cell::new(ControlMode::Mouse),
                connected: Cell::new(false),
                output: Cell::new(OutputTarget::Paused),
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&*state as *const TrayState) as isize);
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: CALLBACK,
                hIcon: crate::presentation::icon::resource_icon()?,
                ..Default::default()
            };
            for (slot, character) in nid.szTip.iter_mut().take(127).zip(name.encode_utf16()) {
                *slot = character;
            }
            if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                let _ = DestroyWindow(hwnd);
                anyhow::bail!("Cannot register tray icon");
            }
            Ok(Self {
                hwnd,
                nid,
                state,
                tooltip: name.to_string(),
            })
        }
    }
    pub fn poll(&mut self, enabled: bool) {
        self.state.enabled.set(enabled);
        unsafe {
            if self.state.main.get().0.is_null() {
                let pointer = (&*self.state as *const TrayState) as isize;
                let _ = EnumThreadWindows(GetCurrentThreadId(), Some(find_main), LPARAM(pointer));
                if !self.state.main.get().0.is_null()
                    && !SetWindowSubclass(
                        self.state.main.get(),
                        Some(main_proc),
                        SUBCLASS,
                        pointer as usize,
                    )
                    .as_bool()
                {
                    self.state.main.set(HWND::default());
                }
            }
            if !enabled {
                restore(self.state.main.get());
            }
        }
    }
    pub fn set_control_state(
        &mut self,
        language: Language,
        mode: ControlMode,
        connection: ConnectionStatus,
        output: OutputTarget,
    ) {
        self.state.language.set(language);
        self.state.mode.set(mode);
        self.state
            .connected
            .set(connection == ConnectionStatus::Connected);
        self.state.output.set(output);
        let status = if connection == ConnectionStatus::Connected {
            Text::Connected
        } else {
            Text::Disconnected
        };
        let input = match output {
            OutputTarget::Desktop => Text::InputActive,
            OutputTarget::Preview => Text::TestActive,
            OutputTarget::Paused => Text::InputPaused,
        };
        let tooltip = format!(
            "{} / {} / {} / {}",
            Text::AppName.get(language),
            mode_text(mode).get(language),
            status.get(language),
            input.get(language)
        );
        if tooltip != self.tooltip {
            self.nid.szTip.fill(0);
            for (slot, character) in self
                .nid
                .szTip
                .iter_mut()
                .take(127)
                .zip(tooltip.encode_utf16())
            {
                *slot = character;
            }
            unsafe {
                let _ = Shell_NotifyIconW(NIM_MODIFY, &self.nid);
            }
            self.tooltip = tooltip;
        }
    }
    pub fn take_pause_requested(&self) -> bool {
        self.state.pause_requested.replace(false)
    }
    pub fn exit_requested(&self) -> bool {
        self.state.exit.get()
    }
}
impl Drop for WindowsTrayManager {
    fn drop(&mut self) {
        unsafe {
            if !self.state.main.get().0.is_null() {
                let _ = RemoveWindowSubclass(self.state.main.get(), Some(main_proc), SUBCLASS);
            }
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.nid);
            let _ = SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
