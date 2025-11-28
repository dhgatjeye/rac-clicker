use std::sync::atomic::{AtomicPtr, Ordering};
use windows::Win32::Foundation::HWND;

pub struct Handle {
    handle: AtomicPtr<std::ffi::c_void>,
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Handle {
    pub fn new() -> Self {
        Self {
            handle: AtomicPtr::new(std::ptr::null_mut())
        }
    }

    pub fn get(&self) -> HWND {
        HWND(self.handle.load(Ordering::Acquire))
    }

    pub fn set(&self, handle: HWND) {
        self.handle.store(handle.0, Ordering::Release);
    }
}