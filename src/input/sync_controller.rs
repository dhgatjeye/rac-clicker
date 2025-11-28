use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub struct SyncController {
    enabled: AtomicBool,
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl SyncController {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            condvar: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }

    pub fn toggle(&self) -> bool {
        let new_state = !self.enabled.load(Ordering::Relaxed);
        self.enabled.store(new_state, Ordering::Relaxed);

        self.condvar.notify_all();

        new_state
    }

    pub fn force_enable(&self) -> bool {
        if self.enabled.load(Ordering::Relaxed) {
            return true;
        }

        self.enabled.store(true, Ordering::Relaxed);

        self.condvar.notify_all();

        true
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn wait_for_signal(&self, timeout: Duration) -> bool {
        if self.enabled.load(Ordering::Relaxed) {
            return true;
        }

        let guard = self.mutex.lock().unwrap();

        if self.enabled.load(Ordering::Relaxed) {
            return true;
        }

        let (guard, _) = self.condvar.wait_timeout_while(
            guard,
            timeout,
            |_| !self.enabled.load(Ordering::Relaxed)
        ).unwrap();

        drop(guard);

        self.enabled.load(Ordering::Relaxed)
    }
}