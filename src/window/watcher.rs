use crate::window::{WindowFinder, WindowHandle};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub found_interval: Duration,
    pub search_interval: Duration,
    pub shutdown_timeout: Duration,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            found_interval: Duration::from_secs(4),
            search_interval: Duration::from_millis(500),
            shutdown_timeout: Duration::from_secs(2),
        }
    }
}

#[derive(Debug, Default)]
pub struct WatcherStats {
    checks: AtomicU64,
    successful_finds: AtomicU64,
}

impl WatcherStats {
    pub fn total_checks(&self) -> u64 {
        self.checks.load(Ordering::Relaxed)
    }

    pub fn successful_finds(&self) -> u64 {
        self.successful_finds.load(Ordering::Relaxed)
    }

    fn record_check(&self) {
        self.checks.fetch_add(1, Ordering::Relaxed);
    }

    fn record_success(&self) {
        self.successful_finds.fetch_add(1, Ordering::Relaxed);
    }
}

struct WatcherState {
    should_stop: AtomicBool,
    is_window_found: AtomicBool,
    condvar: Condvar,
    mutex: Mutex<()>,
}

impl WatcherState {
    fn new() -> Self {
        Self {
            should_stop: AtomicBool::new(false),
            is_window_found: AtomicBool::new(false),
            condvar: Condvar::new(),
            mutex: Mutex::new(()),
        }
    }

    fn request_stop(&self) {
        self.should_stop.store(true, Ordering::Release);
        self.condvar.notify_all();
    }

    fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::Acquire)
    }

    fn set_window_found(&self, found: bool) {
        self.is_window_found.store(found, Ordering::Release);
    }

    fn is_window_found(&self) -> bool {
        self.is_window_found.load(Ordering::Acquire)
    }

    fn interruptible_sleep(&self, duration: Duration) -> bool {
        if self.should_stop() {
            return true;
        }

        let guard = self.mutex.lock().unwrap_or_else(|e| e.into_inner());

        let _ = self.condvar.wait_timeout_while(
            guard,
            duration,
            |_| !self.should_stop()
        );


        self.should_stop()
    }
}

pub struct WindowWatcher {
    state: Arc<WatcherState>,
    stats: Arc<WatcherStats>,
    config: WatcherConfig,
    handle: Option<JoinHandle<()>>,
    window_finder: Arc<WindowFinder>,
    window_handle: Arc<WindowHandle>,
}

impl WindowWatcher {
    pub fn new(window_finder: Arc<WindowFinder>, window_handle: Arc<WindowHandle>) -> Self {
        Self::with_config(window_finder, window_handle, WatcherConfig::default())
    }

    pub fn with_config(
        window_finder: Arc<WindowFinder>,
        window_handle: Arc<WindowHandle>,
        config: WatcherConfig,
    ) -> Self {
        Self {
            state: Arc::new(WatcherState::new()),
            stats: Arc::new(WatcherStats::default()),
            config,
            handle: None,
            window_finder,
            window_handle,
        }
    }

    pub fn start(&mut self) {
        if self.handle.is_some() {
            return;
        }

        let state = Arc::clone(&self.state);
        let stats = Arc::clone(&self.stats);
        let config = self.config.clone();
        let finder = Arc::clone(&self.window_finder);
        let handle = Arc::clone(&self.window_handle);

        let thread_handle = thread::Builder::new()
            .name("RacWindowWatcher".to_string())
            .spawn(move || {
                Self::watcher_loop(state, stats, config, finder, handle);
            })
            .expect("Failed to spawn window watcher thread");

        self.handle = Some(thread_handle);
    }

    pub fn stop(&mut self) {
        self.state.request_stop();

        if let Some(handle) = self.handle.take() {
            let start = Instant::now();
            while !handle.is_finished() {
                if start.elapsed() > self.config.shutdown_timeout {
                    break;
                }
            }

            if handle.is_finished() {
                let _ = handle.join();
            }
        }
    }

    pub fn is_running(&self) -> bool {
        self.handle.as_ref().map(|h| !h.is_finished()).unwrap_or(false)
    }

    pub fn is_window_found(&self) -> bool {
        self.state.is_window_found()
    }

    pub fn stats(&self) -> &WatcherStats {
        &self.stats
    }

    fn watcher_loop(
        state: Arc<WatcherState>,
        stats: Arc<WatcherStats>,
        config: WatcherConfig,
        finder: Arc<WindowFinder>,
        handle: Arc<WindowHandle>,
    ) {
        let mut consecutive_failures = 0u32;
        let mut last_state_change = Instant::now();

        while !state.should_stop() {
            stats.record_check();

            let found = finder.find_window(&handle).unwrap_or_else(|_| {
                false
            });

            let was_found = state.is_window_found();
            state.set_window_found(found);

            if found {
                stats.record_success();
                consecutive_failures = 0;

                if !was_found {
                    last_state_change = Instant::now();
                }
            } else {
                consecutive_failures = consecutive_failures.saturating_add(1);

                if was_found {
                    last_state_change = Instant::now();
                }
            }

            let sleep_duration = Self::calculate_sleep_duration(
                found,
                consecutive_failures,
                last_state_change.elapsed(),
                &config,
            );

            if state.interruptible_sleep(sleep_duration) {
                break;
            }
        }
    }

    fn calculate_sleep_duration(
        found: bool,
        consecutive_failures: u32,
        time_since_change: Duration,
        config: &WatcherConfig,
    ) -> Duration {
        if found {
            if time_since_change < Duration::from_secs(5) {
                config.found_interval / 2
            } else {
                config.found_interval
            }
        } else {
            match consecutive_failures {
                0..=5 => config.search_interval,
                6..=20 => config.search_interval * 2,
                21..=50 => Duration::from_secs(2),
                _ => Duration::from_secs(5)
            }
        }
    }
}

impl Drop for WindowWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}