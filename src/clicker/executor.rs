use crate::core::{MouseButton, RacError, RacResult};
use crate::thread::PrecisionSleep;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::System::SystemServices::{MK_LBUTTON, MK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
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

    #[inline]
    fn get_click_position(hwnd: HWND) -> RacResult<LPARAM> {
        unsafe {
            let mut cursor_pos = POINT::default();

            GetCursorPos(&mut cursor_pos).map_err(|e| {
                RacError::WindowError(format!("Failed to get cursor position: {}", e))
            })?;

            let result = ScreenToClient(hwnd, &mut cursor_pos);
            if !result.as_bool() {
                return Err(RacError::WindowError(
                    "Failed to convert to client coordinates".to_string(),
                ));
            }

            let lparam = ((cursor_pos.y as u32) << 16) | ((cursor_pos.x as u32) & 0xFFFF);

            Ok(LPARAM(lparam as isize))
        }
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

        let lparam = Self::get_click_position(hwnd)?;

        unsafe {
            PostMessageA(Some(hwnd), down_msg, WPARAM(flags.0 as usize), lparam)
                .map_err(|e| RacError::WindowError(format!("Failed to send button down: {}", e)))?;

            PrecisionSleep::sleep(hold_duration);

            PostMessageA(Some(hwnd), up_msg, WPARAM(0), lparam)
                .map_err(|e| RacError::WindowError(format!("Failed to send button up: {}", e)))?;

            Ok(())
        }
    }
}
