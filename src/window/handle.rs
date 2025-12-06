use windows::Win32::Foundation::HWND;
use std::sync::atomic::{AtomicPtr, Ordering};

#[derive(Debug)]
pub struct WindowHandle {
    hwnd: AtomicPtr<std::ffi::c_void>,
}

impl WindowHandle {
    pub fn new() -> Self {
        Self {
            hwnd: AtomicPtr::new(std::ptr::null_mut()),
        }
    }
    
    pub fn get(&self) -> HWND {
        HWND(self.hwnd.load(Ordering::Acquire))
    }
    
    pub fn set(&self, hwnd: HWND) {
        self.hwnd.store(hwnd.0, Ordering::Release);
    }
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}