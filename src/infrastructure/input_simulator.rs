use tracing::trace;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT,
    MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

const WHEEL_DELTA: i32 = 120;

#[derive(Debug, Clone, Copy, Default)]
pub struct InputSimulator;

impl InputSimulator {
    pub fn execute(&self, action: crate::domain::input::InputAction) -> anyhow::Result<()> {
        use crate::domain::input::InputAction;
        match action {
            InputAction::Move(x, y) => self.move_mouse(x, y),
            InputAction::Left(true) => self.mouse_left_down(),
            InputAction::Left(false) => self.mouse_left_up(),
            InputAction::RightClick => self.mouse_right_click(),
            InputAction::Scroll(steps) => self.mouse_wheel(steps),
            InputAction::Key(key) => self.key_press(VIRTUAL_KEY(key)),
            InputAction::ShowDesktop => {
                self.key_down(VIRTUAL_KEY(0x5B))?;
                let result = self.key_press(VIRTUAL_KEY(0x44));
                let release = self.key_up(VIRTUAL_KEY(0x5B));
                result.and(release)
            }
        }
    }
    pub fn new() -> Self {
        Self
    }

    /// Move mouse by relative offset
    pub fn move_mouse(&self, dx: i32, dy: i32) -> anyhow::Result<()> {
        trace!(event = "input.move", dx, dy, "Relative pointer movement");
        self.send_mouse_input(dx, dy, 0, MOUSEEVENTF_MOVE)
    }

    /// Get current cursor position
    pub fn get_cursor_pos(&self) -> anyhow::Result<(i32, i32)> {
        unsafe {
            let mut point = POINT::default();
            GetCursorPos(&mut point)?;
            trace!(
                event = "input.cursor",
                x = point.x,
                y = point.y,
                "Pointer position read"
            );
            Ok((point.x, point.y))
        }
    }

    /// Simulate left mouse button down
    pub fn mouse_left_down(&self) -> anyhow::Result<()> {
        trace!(
            event = "input.button",
            button = "left",
            pressed = true,
            "Pointer button changed"
        );
        self.send_mouse_input(0, 0, 0, MOUSEEVENTF_LEFTDOWN)
    }

    /// Simulate left mouse button up
    pub fn mouse_left_up(&self) -> anyhow::Result<()> {
        trace!(
            event = "input.button",
            button = "left",
            pressed = false,
            "Pointer button changed"
        );
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
        trace!(
            event = "input.button",
            button = "right",
            pressed = true,
            "Pointer button changed"
        );
        self.send_mouse_input(0, 0, 0, MOUSEEVENTF_RIGHTDOWN)
    }

    /// Simulate right mouse button up
    pub fn mouse_right_up(&self) -> anyhow::Result<()> {
        trace!(
            event = "input.button",
            button = "right",
            pressed = false,
            "Pointer button changed"
        );
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
        trace!(
            event = "input.scroll",
            axis = "vertical",
            delta,
            "Pointer scroll"
        );
        self.send_mouse_input(0, 0, (delta * WHEEL_DELTA) as u32, MOUSEEVENTF_WHEEL)
    }

    /// Simulate horizontal mouse wheel scroll
    pub fn mouse_h_wheel(&self, delta: i32) -> anyhow::Result<()> {
        trace!(
            event = "input.scroll",
            axis = "horizontal",
            delta,
            "Pointer scroll"
        );
        self.send_mouse_input(0, 0, (delta * WHEEL_DELTA) as u32, MOUSEEVENTF_HWHEEL)
    }

    /// Simulate key press
    pub fn key_down(&self, key: VIRTUAL_KEY) -> anyhow::Result<()> {
        trace!(event = "input.key", key = ?key, pressed = true, "Key state changed");
        self.send_key_input(key, KEYBD_EVENT_FLAGS::default())
    }

    /// Simulate key release
    pub fn key_up(&self, key: VIRTUAL_KEY) -> anyhow::Result<()> {
        trace!(event = "input.key", key = ?key, pressed = false, "Key state changed");
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
