use crate::core::{MouseButton, RacError, RacResult};
use crate::thread::PrecisionSleep;
use std::time::Duration;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
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
            let mut rect = RECT::default();

            GetClientRect(hwnd, &mut rect)
                .map_err(|e| RacError::WindowError(format!("Failed to get client rect: {}", e)))?;

            let x = (rect.right - rect.left) / 2;
            let y = (rect.bottom - rect.top) / 2;

            let lparam = ((y as u32) << 16) | ((x as u32) & 0xFFFF);

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
