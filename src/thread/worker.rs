use crate::core::{MouseButton, ClickPattern};
use crate::thread::sync::{SyncSignal};
use std::sync::Arc;

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
}