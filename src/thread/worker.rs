use crate::core::{MouseButton, ClickPattern};
use crate::thread::sync::{SyncSignal, SmartSleep};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub button: MouseButton,
    pub pattern: ClickPattern,
    pub name: String,
}

impl WorkerConfig {
    pub fn left_click(pattern: ClickPattern) -> Self {
        Self {
            button: MouseButton::Left,
            pattern,
            name: "LeftClickWorker".to_string(),
        }
    }

    pub fn right_click(pattern: ClickPattern) -> Self {
        Self {
            button: MouseButton::Right,
            pattern,
            name: "RightClickWorker".to_string(),
        }
    }
}

pub struct ClickWorker {
    config: WorkerConfig,
    signal: Arc<SyncSignal>,
    active: Arc<std::sync::atomic::AtomicBool>,
}

impl ClickWorker {
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config,
            signal: Arc::new(SyncSignal::new()),
            active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn signal(&self) -> Arc<SyncSignal> {
        Arc::clone(&self.signal)
    }

    pub fn set_active(&self, active: bool) {
        self.active.store(active, std::sync::atomic::Ordering::Release);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn config(&self) -> &WorkerConfig {
        &self.config
    }

    pub fn update_pattern(&mut self, pattern: ClickPattern) {
        self.config.pattern = pattern;
    }

    pub fn calculate_delay(&self) -> Duration {
        let base_delay = self.config.pattern.base_delay_us();

        if !self.config.pattern.randomize {
            return Duration::from_micros(base_delay);
        }

        use rand::Rng;
        let mut rng = rand::rng();
        let jitter = rng.random_range(-self.config.pattern.jitter_us..=self.config.pattern.jitter_us);

        let adjusted = if jitter < 0 {
            base_delay.saturating_sub(jitter.unsigned_abs())
        } else {
            base_delay.saturating_add(jitter as u64)
        };

        let final_delay = adjusted.max(self.config.pattern.min_delay_us);

        Duration::from_micros(final_delay)
    }

    pub fn hold_duration(&self) -> Duration {
        Duration::from_micros(self.config.pattern.hold_duration_us)
    }

    pub fn sleep(&self, duration: Duration) {
        SmartSleep::sleep(duration);
    }
}