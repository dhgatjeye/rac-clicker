use std::ptr::null_mut;
use std::sync::atomic::{AtomicPtr, Ordering};
use winapi::shared::windef::HWND;

pub struct Handle {
    handle: AtomicPtr<std::ffi::c_void>,
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Handle {
    pub fn new() -> Self {
        Self { 
            handle: AtomicPtr::new(null_mut()) 
        }
    }

    pub fn get(&self) -> HWND {
        self.handle.load(Ordering::Acquire) as HWND
    }

    pub fn set(&self, handle: HWND) {
        self.handle.store(handle as *mut std::ffi::c_void, Ordering::Release);
    }
}