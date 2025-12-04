use crate::core::{RacResult, RacError, MouseButton};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
};
use std::time::Duration;

const MK_LBUTTON: u32 = 0x0001;
const MK_RBUTTON: u32 = 0x0002;

pub struct ClickExecutor;

impl ClickExecutor {
    pub fn new() -> Self {
        Self
    }
    
    pub fn execute_click(
        &self,
        hwnd: HWND,
        button: MouseButton,
        hold_duration: Duration,
    ) -> RacResult<()> {
        if hwnd.is_invalid() {
            return Err(RacError::WindowError("Invalid window handle".to_string()));
        }

        let (down_msg, up_msg, flags) = match button {
            MouseButton::Left => (WM_LBUTTONDOWN, WM_LBUTTONUP, MK_LBUTTON),
            MouseButton::Right => (WM_RBUTTONDOWN, WM_RBUTTONUP, MK_RBUTTON),
        };

        unsafe {
            PostMessageA(Some(hwnd), down_msg, WPARAM(flags as usize), LPARAM(0))
                .map_err(|e| RacError::WindowError(format!("Failed to send button down: {}", e)))?;

            std::thread::sleep(hold_duration);
            
            PostMessageA(Some(hwnd), up_msg, WPARAM(0), LPARAM(0))
                .map_err(|e| RacError::WindowError(format!("Failed to send button up: {}", e)))?;
        }

        Ok(())
    }
}