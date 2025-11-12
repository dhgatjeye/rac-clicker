use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub struct SyncController {
    enabled: AtomicBool,
    mutex: Mutex<()>, 
    condvar: Condvar,
}

impl SyncController {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            mutex: Mutex::new(()), 
            condvar: Condvar::new(),
        }
    }

    pub fn toggle(&self) -> bool {
        let new_state = !self.enabled.load(Ordering::Acquire);
        self.enabled.store(new_state, Ordering::Release);
        self.condvar.notify_all();
        new_state
    }

    pub fn force_enable(&self) -> bool {
        if self.enabled.load(Ordering::Acquire) {
            return true;
        }

        self.enabled.store(true, Ordering::Release);
        self.condvar.notify_all();
        true
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    pub fn wait_for_signal(&self, timeout: Duration) -> bool {
        let enabled = self.enabled.load(Ordering::Acquire);
        
        if !enabled {
            let guard = self.mutex.lock().unwrap();
            let (_guard, timeout_result) = self.condvar.wait_timeout(guard, timeout).unwrap();
            !timeout_result.timed_out() && self.enabled.load(Ordering::Acquire)
        } else {
            true
        }
    }
}