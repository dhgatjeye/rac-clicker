use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WorkerState {
    Stopped = 0,
    Running = 1,
    Paused = 2,
}

impl From<u8> for WorkerState {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Stopped,
            1 => Self::Running,
            2 => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

#[derive(Debug)]
pub struct SyncSignal {
    state: AtomicU8,
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl Default for SyncSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncSignal {
    pub fn new() -> Self {
        Self {
            state: AtomicU8::new(WorkerState::Stopped as u8),
            condvar: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }

    pub fn state(&self) -> WorkerState {
        WorkerState::from(self.state.load(Ordering::Acquire))
    }

    pub fn set_state(&self, new_state: WorkerState) {
        self.state.store(new_state as u8, Ordering::Release);
        self.condvar.notify_all();
    }

    pub fn start(&self) {
        self.set_state(WorkerState::Running);
    }

    pub fn pause(&self) {
        self.set_state(WorkerState::Paused);
    }

    pub fn stop(&self) {
        self.set_state(WorkerState::Stopped);
    }

    pub fn is_running(&self) -> bool {
        self.state() == WorkerState::Running
    }

    pub fn is_stopped(&self) -> bool {
        self.state() == WorkerState::Stopped
    }

    pub fn wait_for_running(&self, timeout: Duration) -> bool {
        let guard = match self.mutex.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };

        if self.is_running() {
            return true;
        }

        let result = self
            .condvar
            .wait_timeout_while(guard, timeout, |_| !self.is_running());

        match result {
            Ok((_, timeout_result)) => !timeout_result.timed_out() && self.is_running(),
            Err(_) => false,
        }
    }
}
