use crate::core::{MouseButton, RacError, RacResult};
use crate::thread::PrecisionSleep;
use std::time::Duration;
use std::time::Instant;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

pub struct ClickExecutor;

impl Default for ClickExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ClickExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute_click(
        &self,
        hwnd: HWND,
        button: MouseButton,
        hold_duration: Duration,
    ) -> RacResult<Instant> {
        if hwnd.is_invalid() {
            return Err(RacError::WindowError("Invalid window handle".to_string()));
        }

        let (down_msg, up_msg, flags) = match button {
            MouseButton::Left => (WM_LBUTTONDOWN, WM_LBUTTONUP, MK_LBUTTON),
            MouseButton::Right => (WM_RBUTTONDOWN, WM_RBUTTONUP, MK_RBUTTON),
        };

        unsafe {
            let down_instant = Instant::now();

            PostMessageA(Some(hwnd), down_msg, WPARAM(flags.0 as usize), LPARAM(0))
                .map_err(|e| RacError::WindowError(format!("Failed to send button down: {}", e)))?;

            PrecisionSleep::sleep(hold_duration);

            PostMessageA(Some(hwnd), up_msg, WPARAM(0), LPARAM(0))
                .map_err(|e| RacError::WindowError(format!("Failed to send button up: {}", e)))?;

            Ok(down_instant)
        }
    }
}
