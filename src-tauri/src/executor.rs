use crate::types::MacroStep;

#[derive(Debug, thiserror::Error)]
pub enum ExecError {
    #[error("unsupported platform")]
    Unsupported,
    #[error("send_input failed")]
    SendInputFailed,
    #[error("unknown button {0}")]
    UnknownButton(String),
}

pub fn execute_step(step: &MacroStep) -> Result<(), ExecError> {
    #[cfg(windows)]
    {
        return imp::execute_step(step);
    }
    #[cfg(not(windows))]
    {
        let _ = step;
        Err(ExecError::Unsupported)
    }
}

#[cfg(windows)]
mod imp {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        MapVirtualKeyW, SendInput, INPUT, INPUT_0, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE,
        MOUSEINPUT,         MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN,
        MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
        MOUSEEVENTF_VIRTUALDESK,
        MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MAPVK_VK_TO_VSC, INPUT_KEYBOARD, INPUT_MOUSE,
        MOUSE_EVENT_FLAGS, VIRTUAL_KEY,
    };

    /// Mouse order value for SendInput X buttons (see MS docs; not re-exported as XBUTTON1/2 in windows 0.58).
    const XBUTTON1_DATA: u32 = 0x0001;
    const XBUTTON2_DATA: u32 = 0x0002;

    use crate::executor::ExecError;
    use crate::types::MacroStep;

    pub fn execute_step(step: &MacroStep) -> Result<(), ExecError> {
        match step {
            MacroStep::MouseDown { button } => {
                let (f, d) = mouse_down(button)?;
                send_mouse(f, d)?;
            }
            MacroStep::MouseUp { button } => {
                let (f, d) = mouse_up(button)?;
                send_mouse(f, d)?;
            }
            MacroStep::KeyDown { vk } => send_key(*vk, false)?,
            MacroStep::KeyUp { vk } => send_key(*vk, true)?,
            MacroStep::MouseMoveRel { dx, dy } => send_mouse_move_rel(*dx, *dy)?,
            MacroStep::Delay { .. } => {}
        }
        Ok(())
    }

    fn send_key(vk: u32, keyup: bool) -> Result<(), ExecError> {
        let scan = unsafe { MapVirtualKeyW(vk, MAPVK_VK_TO_VSC) } as u16;
        let mut flags = KEYEVENTF_SCANCODE;
        if keyup {
            flags |= KEYEVENTF_KEYUP;
        }
        let ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(0),
            wScan: scan,
            dwFlags: flags,
            time: 0,
            dwExtraInfo: 0usize,
        };
        send_input(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki },
        })
    }

    fn send_mouse(flags: MOUSE_EVENT_FLAGS, data: u32) -> Result<(), ExecError> {
        let mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: data,
            dwFlags: flags | MOUSEEVENTF_VIRTUALDESK,
            time: 0,
            dwExtraInfo: 0usize,
        };
        send_input(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi },
        })
    }

    fn send_mouse_move_rel(dx: i32, dy: i32) -> Result<(), ExecError> {
        let mi = MOUSEINPUT {
            dx,
            dy,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_MOVE,
            time: 0,
            dwExtraInfo: 0usize,
        };
        send_input(INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi },
        })
    }

    fn send_input(input: INPUT) -> Result<(), ExecError> {
        unsafe {
            let r = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            if r != 1 {
                return Err(ExecError::SendInputFailed);
            }
        }
        Ok(())
    }

    fn mouse_down(button: &str) -> Result<(MOUSE_EVENT_FLAGS, u32), ExecError> {
        Ok(match button.to_ascii_lowercase().as_str() {
            "left" | "lmb" => (MOUSEEVENTF_LEFTDOWN, 0),
            "right" | "rmb" => (MOUSEEVENTF_RIGHTDOWN, 0),
            "middle" | "mmb" => (MOUSEEVENTF_MIDDLEDOWN, 0),
            "x1" => (MOUSEEVENTF_XDOWN, XBUTTON1_DATA),
            "x2" => (MOUSEEVENTF_XDOWN, XBUTTON2_DATA),
            _ => return Err(ExecError::UnknownButton(button.into())),
        })
    }

    fn mouse_up(button: &str) -> Result<(MOUSE_EVENT_FLAGS, u32), ExecError> {
        Ok(match button.to_ascii_lowercase().as_str() {
            "left" | "lmb" => (MOUSEEVENTF_LEFTUP, 0),
            "right" | "rmb" => (MOUSEEVENTF_RIGHTUP, 0),
            "middle" | "mmb" => (MOUSEEVENTF_MIDDLEUP, 0),
            "x1" => (MOUSEEVENTF_XUP, XBUTTON1_DATA),
            "x2" => (MOUSEEVENTF_XUP, XBUTTON2_DATA),
            _ => return Err(ExecError::UnknownButton(button.into())),
        })
    }
}
