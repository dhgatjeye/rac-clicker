use windows::Win32::Foundation::HWND;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct WindowHandle {
    hwnd: Arc<Mutex<HWND>>,
}

impl WindowHandle {
    pub fn new() -> Self {
        Self {
            hwnd: Arc::new(Mutex::new(HWND(std::ptr::null_mut()))),
        }
    }

    pub fn from_hwnd(hwnd: HWND) -> Self {
        Self {
            hwnd: Arc::new(Mutex::new(hwnd)),
        }
    }

    pub fn get(&self) -> HWND {
        *self.hwnd.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn set(&self, hwnd: HWND) {
        if let Ok(mut guard) = self.hwnd.lock() {
            *guard = hwnd;
        }
    }

    pub fn is_valid(&self) -> bool {
        let hwnd = self.get();
        !hwnd.is_invalid() && !hwnd.0.is_null()
    }
}

impl Default for WindowHandle {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

