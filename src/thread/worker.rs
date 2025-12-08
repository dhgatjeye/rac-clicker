use crate::core::{ClickPattern, MouseButton};
use crate::thread::sync::SyncSignal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[repr(align(64))]
struct CacheAlignedAtomicBool(AtomicBool);

impl CacheAlignedAtomicBool {
    fn new(value: bool) -> Self {
        Self(AtomicBool::new(value))
    }

    fn load(&self, ordering: Ordering) -> bool {
        self.0.load(ordering)
    }

    fn store(&self, value: bool, ordering: Ordering) {
        self.0.store(value, ordering)
    }
}

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
    active: Arc<CacheAlignedAtomicBool>,
}

impl ClickWorker {
    pub fn new(config: WorkerConfig) -> Self {
        Self {
            config,
            signal: Arc::new(SyncSignal::new()),
            active: Arc::new(CacheAlignedAtomicBool::new(false)),
        }
    }

    pub fn signal(&self) -> Arc<SyncSignal> {
        Arc::clone(&self.signal)
    }

    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Release);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub fn config(&self) -> &WorkerConfig {
        &self.config
    }
}
