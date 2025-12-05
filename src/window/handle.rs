use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::HWND;

#[derive(Debug)]
pub struct WindowHandle {
    raw: AtomicIsize,
}

impl WindowHandle {
    #[inline]
    pub const fn new() -> Self {
        Self {
            raw: AtomicIsize::new(0),
        }
    }
    
    #[inline]
    pub fn get(&self) -> HWND {
        HWND(self.raw.load(Ordering::Acquire) as *mut _)
    }

    #[inline]
    pub fn set(&self, hwnd: HWND) {
        self.raw.store(hwnd.0 as isize, Ordering::Release);
    }
    
    #[inline]
    pub fn clear(&self) {
        self.raw.store(0, Ordering::Release);
    }

    #[inline]
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.raw.load(Ordering::Acquire) != 0
    }
    
    #[inline]
    pub fn swap(&self, hwnd: HWND) -> HWND {
        HWND(self.raw.swap(hwnd.0 as isize, Ordering::AcqRel) as *mut _)
    }
}

impl Default for WindowHandle {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}