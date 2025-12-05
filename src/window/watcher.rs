use crate::window::{WindowFinder, WindowHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct WatcherConfig {
    pub active_interval: Duration,
    pub search_interval: Duration,
}

impl WatcherConfig {
    pub const fn new(active_interval: Duration, search_interval: Duration) -> Self {
        Self {
            active_interval,
            search_interval,
        }
    }
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            active_interval: Duration::from_secs(3),
            search_interval: Duration::from_millis(500),
        }
    }
}

struct SharedState {
    stop_signal: AtomicBool,
    window_active: AtomicBool,
}

impl SharedState {
    const fn new() -> Self {
        Self {
            stop_signal: AtomicBool::new(false),
            window_active: AtomicBool::new(false),
        }
    }

    #[inline]
    fn should_stop(&self) -> bool {
        self.stop_signal.load(Ordering::Acquire)
    }

    #[inline]
    fn request_stop(&self) {
        self.stop_signal.store(true, Ordering::Release);
    }

    #[inline]
    fn set_active(&self, active: bool) {
        self.window_active.store(active, Ordering::Release);
    }

    #[inline]
    fn is_active(&self) -> bool {
        self.window_active.load(Ordering::Acquire)
    }
}

pub struct WindowWatcher {
    state: Arc<SharedState>,
    thread: Option<JoinHandle<()>>,
}

impl WindowWatcher {
    pub fn spawn(
        finder: WindowFinder,
        handle: Arc<WindowHandle>,
        config: WatcherConfig,
    ) -> Self {
        let state = Arc::new(SharedState::new());
        let state_clone = Arc::clone(&state);

        let thread = thread::Builder::new()
            .name("WindowWatcher".into())
            .spawn(move || {
                Self::watch_loop(state_clone, finder, handle, config);
            })
            .expect("Failed to spawn watcher thread");

        Self {
            state,
            thread: Some(thread),
        }
    }

    #[inline]
    pub fn spawn_default(finder: WindowFinder, handle: Arc<WindowHandle>) -> Self {
        Self::spawn(finder, handle, WatcherConfig::default())
    }

    #[inline]
    #[must_use]
    pub fn is_window_active(&self) -> bool {
        self.state.is_active()
    }

    pub fn stop(&mut self) {
        self.state.request_stop();

        if let Some(thread) = self.thread.take() {
            thread::sleep(Duration::from_millis(10));
            let _ = thread.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.thread
            .as_ref()
            .map(|t| !t.is_finished())
            .unwrap_or(false)
    }

    fn watch_loop(
        state: Arc<SharedState>,
        finder: WindowFinder,
        handle: Arc<WindowHandle>,
        config: WatcherConfig,
    ) {
        let mut was_active = false;

        while !state.should_stop() {
            let is_active = finder
                .find_and_update(&handle)
                .unwrap_or_else(|_| {
                    finder.invalidate_cache();
                    false
                });

            state.set_active(is_active);

            if is_active != was_active {
                was_active = is_active;
            }

            let sleep_duration = if is_active {
                config.active_interval
            } else {
                config.search_interval
            };

            Self::interruptible_sleep(&state, sleep_duration);
        }

        handle.clear();
    }

    fn interruptible_sleep(state: &SharedState, duration: Duration) {
        const CHECK_INTERVAL: Duration = Duration::from_millis(50);

        let mut remaining = duration;
        while remaining > Duration::ZERO && !state.should_stop() {
            let sleep_time = remaining.min(CHECK_INTERVAL);
            thread::sleep(sleep_time);
            remaining = remaining.saturating_sub(sleep_time);
        }
    }
}

impl Drop for WindowWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}