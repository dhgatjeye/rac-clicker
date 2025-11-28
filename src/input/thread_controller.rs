use std::thread;
use std::time::{Duration, Instant};

pub struct ThreadController;

impl ThreadController {
    pub fn new() -> Self {
        Self
    }

    pub fn clone(&self) -> Self {
        Self
    }

    pub fn smart_sleep(&self, duration: Duration) {
        if duration.as_micros() < 1 {
            return;
        }

        if duration.as_micros() < 1000 {
            let sleep_duration = duration.saturating_mul(58) / 100;
            if sleep_duration.as_micros() > 0 {
                thread::sleep(sleep_duration);
            }

            let deadline = Instant::now() + duration;
            while Instant::now() < deadline {
                std::hint::spin_loop();
            }
            return;
        }

        thread::sleep(duration);
    }
}