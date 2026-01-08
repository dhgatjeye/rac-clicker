use crate::window::{WindowFinder, WindowHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const SEARCH_INTERVAL: Duration = Duration::from_millis(1000);
const FOUND_INTERVAL: Duration = Duration::from_secs(5);

pub struct WindowWatcher {
    stop_signal: Arc<AtomicBool>,
    condvar: Arc<(Mutex<()>, Condvar)>,
    handle: Option<JoinHandle<()>>,
}

impl WindowWatcher {
    pub fn new(
        finder: Arc<WindowFinder>,
        window_handle: Arc<WindowHandle>,
    ) -> std::io::Result<Self> {
        let stop_signal = Arc::new(AtomicBool::new(false));
        let condvar = Arc::new((Mutex::new(()), Condvar::new()));

        let stop = Arc::clone(&stop_signal);
        let cv = Arc::clone(&condvar);

        let handle = thread::Builder::new()
            .name("RacWindowWatcher".to_string())
            .spawn(move || {
                Self::watch_loop(finder, window_handle, stop, cv);
            })?;

        Ok(Self {
            stop_signal,
            condvar,
            handle: Some(handle),
        })
    }

    pub fn stop(&mut self) {
        self.stop_signal.store(true, Ordering::Release);
        self.condvar.1.notify_all();

        if let Some(handle) = self.handle.take()
            && let Err(_e) = handle.join()
        {
            #[cfg(debug_assertions)]
            eprintln!("WindowWatcher thread panicked: {:?}", _e);
        }
    }

    fn watch_loop(
        finder: Arc<WindowFinder>,
        window_handle: Arc<WindowHandle>,
        stop: Arc<AtomicBool>,
        cv: Arc<(Mutex<()>, Condvar)>,
    ) {
        let mut was_found = false;

        while !stop.load(Ordering::Acquire) {
            let found = finder.find_window(&window_handle).unwrap_or(false);

            let interval = if found {
                FOUND_INTERVAL
            } else {
                SEARCH_INTERVAL
            };

            if found != was_found {
                was_found = found;
            }

            if Self::interruptible_sleep(&stop, &cv, interval) {
                break;
            }
        }
    }

    fn interruptible_sleep(
        stop: &AtomicBool,
        cv: &(Mutex<()>, Condvar),
        duration: Duration,
    ) -> bool {
        if stop.load(Ordering::Acquire) {
            return true;
        }

        let guard = cv.0.lock().unwrap_or_else(|poisoned| {
            #[cfg(debug_assertions)]
            eprintln!("Mutex poisoned in window watcher: {}", poisoned);
            poisoned.into_inner()
        });

        let _ =
            cv.1.wait_timeout_while(guard, duration, |_| !stop.load(Ordering::Acquire));

        stop.load(Ordering::Acquire)
    }
}

impl Drop for WindowWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}
