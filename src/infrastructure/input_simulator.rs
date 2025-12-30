use tracing::{debug, trace};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

const WHEEL_DELTA: i32 = 120;

pub struct InputSimulator;

impl InputSimulator {
    pub fn new() -> Self {
        Self
    }

    /// Move mouse by relative offset
    pub fn move_mouse(&self, dx: i32, dy: i32) -> anyhow::Result<()> {
        trace!("Moving mouse by ({}, {})", dx, dy);
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx,
                        dy,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_MOVE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    /// Move mouse to absolute position
    pub fn set_cursor_pos(&self, x: i32, y: i32) -> anyhow::Result<()> {
        debug!("Setting cursor pos to ({}, {})", x, y);
        unsafe {
            SetCursorPos(x, y)?;
        }
        Ok(())
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
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_LEFTDOWN,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    /// Simulate left mouse button up
    pub fn mouse_left_up(&self) -> anyhow::Result<()> {
        debug!("Mouse Left Up");
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_LEFTUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
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
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_RIGHTDOWN,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    /// Simulate right mouse button up
    pub fn mouse_right_up(&self) -> anyhow::Result<()> {
        debug!("Mouse Right Up");
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: 0,
                        dwFlags: MOUSEEVENTF_RIGHTUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
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
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: 0,
                        dy: 0,
                        mouseData: (delta * WHEEL_DELTA) as u32,
                        dwFlags: MOUSEEVENTF_WHEEL,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    /// Simulate key press
    pub fn key_down(&self, key: VIRTUAL_KEY) -> anyhow::Result<()> {
        debug!("Key Down: {:?}", key);
        unsafe {
            let input = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        wScan: 0,
                        dwFlags: Default::default(),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    /// Simulate key release
    pub fn key_up(&self, key: VIRTUAL_KEY) -> anyhow::Result<()> {
        debug!("Key Up: {:?}", key);
        unsafe {
            let input = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: key,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }

    /// Simulate key press and release
    pub fn key_press(&self, key: VIRTUAL_KEY) -> anyhow::Result<()> {
        self.key_down(key)?;
        self.key_up(key)?;
        Ok(())
    }
}
