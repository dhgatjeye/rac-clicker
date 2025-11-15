use crate::logger::logger::log_error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority};
use windows::Win32::System::Threading::{THREAD_PRIORITY_BELOW_NORMAL, THREAD_PRIORITY_NORMAL, THREAD_PRIORITY_TIME_CRITICAL};

pub struct ThreadController {
    adaptive_mode: AtomicBool,
}

impl ThreadController {
    pub(crate) fn clone(&self) -> ThreadController {
        ThreadController {
            adaptive_mode: AtomicBool::new(self.adaptive_mode.load(Ordering::Relaxed)),
        }
    }
}

impl ThreadController {
    pub fn new(adaptive_mode: bool) -> Self {
        Self { 
            adaptive_mode: AtomicBool::new(adaptive_mode),
        }
    }

    pub fn set_adaptive_mode(&self, adaptive_mode: bool) {
        self.adaptive_mode.store(adaptive_mode, Ordering::Relaxed);
    }

    pub fn set_active_priority(&self) {
        let context = "ThreadController::set_active_priority";
        unsafe {
            let priority = if self.adaptive_mode.load(Ordering::Relaxed) {
                THREAD_PRIORITY_NORMAL
            } else {
                THREAD_PRIORITY_TIME_CRITICAL
            };

            if let Err(e) = SetThreadPriority(GetCurrentThread(), priority) {
                log_error(&format!("Failed to set active thread priority: {:?}", e), context);
            }
        }
    }

    pub fn set_normal_priority(&self) {
        let context = "ThreadController::set_normal_priority";
        unsafe {
            if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_NORMAL) {
                log_error(&format!("Failed to set normal thread priority: {:?}", e), context);
            }
        }
    }

    pub fn set_idle_priority(&self) {
        let context = "ThreadController::set_idle_priority";
        unsafe {
            if let Err(e) = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) {
                log_error(&format!("Failed to set idle thread priority: {:?}", e), context);
            }
        }
    }

    pub fn smart_sleep(&self, duration: Duration) {
        if duration.as_micros() < 1 {
            return;
        }

        if duration.as_micros() < 1000 {
            let sleep_duration = duration.saturating_mul(4) / 5;
            if sleep_duration.as_micros() > 0 {
                thread::sleep(sleep_duration);
            }
            
            let deadline = Instant::now() + duration;
            while Instant::now() < deadline {
                thread::yield_now();
            }
            return;
        }

        thread::sleep(duration);
    }
}