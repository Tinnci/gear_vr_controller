use tracing::{debug, trace};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT,
    MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

const WHEEL_DELTA: i32 = 120;

pub struct InputSimulator;

impl InputSimulator {
    pub fn new() -> Self {
        Self
    }

    /// Move mouse by relative offset
    pub fn move_mouse(&self, dx: i32, dy: i32) -> anyhow::Result<()> {
        trace!("Moving mouse by ({}, {})", dx, dy);
        self.send_mouse_input(dx, dy, 0, MOUSEEVENTF_MOVE)
    }

    /// Get current cursor position
    pub fn get_cursor_pos(&self) -> anyhow::Result<(i32, i32)> {
        unsafe {
            let mut point = POINT::default();
            GetCursorPos(&mut point)?;
            trace!("Got cursor pos: ({}, {})", point.x, point.y);
            Ok((point.x, point.y))
        }
    }

    /// Simulate left mouse button down
    pub fn mouse_left_down(&self) -> anyhow::Result<()> {
        debug!("Mouse Left Down");
        self.send_mouse_input(0, 0, 0, MOUSEEVENTF_LEFTDOWN)
    }

    /// Simulate left mouse button up
    pub fn mouse_left_up(&self) -> anyhow::Result<()> {
        debug!("Mouse Left Up");
        self.send_mouse_input(0, 0, 0, MOUSEEVENTF_LEFTUP)
    }

    /// Simulate left mouse click
    pub fn mouse_left_click(&self) -> anyhow::Result<()> {
        self.mouse_left_down()?;
        self.mouse_left_up()?;
        Ok(())
    }

    /// Simulate right mouse button down
    pub fn mouse_right_down(&self) -> anyhow::Result<()> {
        debug!("Mouse Right Down");
        self.send_mouse_input(0, 0, 0, MOUSEEVENTF_RIGHTDOWN)
    }

    /// Simulate right mouse button up
    pub fn mouse_right_up(&self) -> anyhow::Result<()> {
        debug!("Mouse Right Up");
        self.send_mouse_input(0, 0, 0, MOUSEEVENTF_RIGHTUP)
    }

    /// Simulate right mouse click
    pub fn mouse_right_click(&self) -> anyhow::Result<()> {
        self.mouse_right_down()?;
        self.mouse_right_up()?;
        Ok(())
    }

    /// Simulate mouse wheel scroll
    pub fn mouse_wheel(&self, delta: i32) -> anyhow::Result<()> {
        debug!("Mouse Wheel Scroll: {}", delta);
        self.send_mouse_input(0, 0, (delta * WHEEL_DELTA) as u32, MOUSEEVENTF_WHEEL)
    }

    /// Simulate horizontal mouse wheel scroll
    pub fn mouse_h_wheel(&self, delta: i32) -> anyhow::Result<()> {
        debug!("Mouse Horizontal Wheel Scroll: {}", delta);
        self.send_mouse_input(0, 0, (delta * WHEEL_DELTA) as u32, MOUSEEVENTF_HWHEEL)
    }

    /// Simulate key press
    pub fn key_down(&self, key: VIRTUAL_KEY) -> anyhow::Result<()> {
        debug!("Key Down: {:?}", key);
        self.send_key_input(key, KEYBD_EVENT_FLAGS::default())
    }

    /// Simulate key release
    pub fn key_up(&self, key: VIRTUAL_KEY) -> anyhow::Result<()> {
        debug!("Key Up: {:?}", key);
        self.send_key_input(key, KEYEVENTF_KEYUP)
    }

    /// Simulate key press and release
    pub fn key_press(&self, key: VIRTUAL_KEY) -> anyhow::Result<()> {
        self.key_down(key)?;
        self.key_up(key)?;
        Ok(())
    }

    fn send_mouse_input(
        &self,
        dx: i32,
        dy: i32,
        mouse_data: u32,
        flags: MOUSE_EVENT_FLAGS,
    ) -> anyhow::Result<()> {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx,
                    dy,
                    mouseData: mouse_data,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        self.send_input(input)
    }

    fn send_key_input(&self, key: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> anyhow::Result<()> {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: key,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        self.send_input(input)
    }

    fn send_input(&self, input: INPUT) -> anyhow::Result<()> {
        let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };

        if sent == 0 {
            anyhow::bail!("SendInput failed: {}", windows::core::Error::from_thread());
        }

        Ok(())
    }
}
