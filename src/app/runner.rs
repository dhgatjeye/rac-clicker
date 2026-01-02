use crate::clicker::{ClickController, ClickExecutor, DelayCalculator};
use crate::config::ConfigProfile;
use crate::core::{MouseButton, RacError, RacResult};
use crate::input::{HotkeyManager, InputMonitor};
use crate::thread::{ClickWorker, ThreadManager, WorkerConfig};
use crate::window::{WindowFinder, WindowHandle, WindowWatcher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

pub struct RacApp {
    profile: ConfigProfile,
    thread_manager: Arc<Mutex<ThreadManager>>,
    window_handle: Arc<WindowHandle>,
    window_finder: Arc<WindowFinder>,
    window_watcher: Option<WindowWatcher>,
    input_monitor_stop: Arc<AtomicBool>,
    input_monitor_handle: Option<thread::JoinHandle<()>>,
    exit_signal: Arc<(Mutex<bool>, Condvar)>,
}

impl Drop for RacApp {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl RacApp {
    pub fn new(profile: ConfigProfile) -> RacResult<Self> {
        let target_process = profile.get_target_process()?;

        Ok(Self {
            profile,
            thread_manager: Arc::new(Mutex::new(ThreadManager::new())),
            window_handle: Arc::new(WindowHandle::new()),
            window_finder: Arc::new(WindowFinder::new(&target_process)),
            window_watcher: None,
            input_monitor_stop: Arc::new(AtomicBool::new(false)),
            input_monitor_handle: None,
            exit_signal: Arc::new((Mutex::new(false), Condvar::new())),
        })
    }

    pub fn run(&mut self) -> RacResult<()> {
        self.print_startup_banner();

        println!("→ Initializing workers...");
        self.initialize_workers()?;

        println!("→ Starting worker threads...");
        self.start_workers()?;

        println!("→ Starting window watcher...");
        self.start_window_watcher()?;

        println!("→ Starting input monitor...");
        self.start_input_monitor()?;

        self.print_running_info()?;

        self.main_loop()
    }

    fn print_startup_banner(&self) {
        println!("\n╔════════════════════════════════════════════╗");
        println!("║         RAC v2 - Starting...               ║");
        println!("╚════════════════════════════════════════════╝\n");
    }

    fn print_running_info(&self) -> RacResult<()> {
        println!("\n✓ RAC v2 is now running!");
        println!("\nConfiguration:");
        println!(
            "  Server:       {}",
            self.profile.server_registry.active_server_type()
        );
        println!("  Toggle Mode:  {}", self.profile.settings.toggle_mode);
        println!("  Click Mode:   {}", self.profile.settings.click_mode);
        println!(
            "  Toggle Key:   {}",
            HotkeyManager::key_name(self.profile.settings.toggle_hotkey)
        );

        if self.profile.settings.click_mode.is_left_active() {
            let left_cps = self.profile.get_left_cps()?;
            println!("  Left CPS:     {}", left_cps);
        }
        if self.profile.settings.click_mode.is_right_active() {
            let right_cps = self.profile.get_right_cps()?;
            println!("  Right CPS:    {}", right_cps);
        }

        println!("\nPress Ctrl+Q to return to main menu...\n");
        Ok(())
    }

    fn initialize_workers(&mut self) -> RacResult<()> {
        let server_config = self.profile.server_registry.get_active()?;

        let left_cps = self.profile.get_left_cps()?;
        let right_cps = self.profile.get_right_cps()?;

        if self.profile.settings.click_mode.is_left_active() {
            let mut pattern = server_config.left_click;
            pattern.max_cps = left_cps;

            let config = WorkerConfig::left_click(pattern);
            let worker = ClickWorker::new(config);

            let mut tm = self.lock_thread_manager()?;
            tm.register_worker(worker);
        }

        if self.profile.settings.click_mode.is_right_active() {
            let mut pattern = server_config.right_click;
            pattern.max_cps = right_cps;

            let config = WorkerConfig::right_click(pattern);
            let worker = ClickWorker::new(config);

            let mut tm = self.lock_thread_manager()?;
            tm.register_worker(worker);
        }

        Ok(())
    }

    fn start_workers(&mut self) -> RacResult<()> {
        let toggle_mode = self.profile.settings.toggle_mode;
        let server_type = self.profile.settings.active_server;

        if self.profile.settings.click_mode.is_left_active() {
            self.start_button_worker(MouseButton::Left, toggle_mode, server_type)?;
        }

        if self.profile.settings.click_mode.is_right_active() {
            self.start_button_worker(MouseButton::Right, toggle_mode, server_type)?;
        }

        Ok(())
    }

    fn start_button_worker(
        &mut self,
        button: MouseButton,
        toggle_mode: crate::core::ToggleMode,
        server_type: crate::core::ServerType,
    ) -> RacResult<()> {
        let window_handle = Arc::clone(&self.window_handle);
        let controller = ClickController::new(toggle_mode, ClickExecutor::new());

        let tm = self.lock_thread_manager()?;

        if let Some(worker) = tm.get_worker(button) {
            let delay_calc = DelayCalculator::new(worker.config().pattern, button, server_type)?;
            worker.signal().pause();
            drop(tm);

            let mut tm_mut = self.lock_thread_manager()?;
            tm_mut.start_worker(button, move |worker| {
                let mut delay = delay_calc;
                controller.run_loop(worker, &mut delay, || window_handle.get());
            })?;
        }

        Ok(())
    }

    fn start_window_watcher(&mut self) -> RacResult<()> {
        let watcher = WindowWatcher::new(
            Arc::clone(&self.window_finder),
            Arc::clone(&self.window_handle),
        )
        .map_err(|e| RacError::ThreadError(format!("Failed to start window watcher: {}", e)))?;

        self.window_watcher = Some(watcher);
        Ok(())
    }

    fn start_input_monitor(&mut self) -> RacResult<()> {
        let settings = &self.profile.settings;
        let stop_signal = Arc::clone(&self.input_monitor_stop);
        let exit_signal = Arc::clone(&self.exit_signal);

        let mut input_monitor = InputMonitor::with_stop_signal(
            settings.toggle_mode,
            settings.click_mode,
            settings.left_hotkey,
            settings.right_hotkey,
            settings.toggle_hotkey,
            stop_signal,
            exit_signal,
        );

        let thread_manager = Arc::clone(&self.thread_manager);

        let handle = thread::Builder::new()
            .name("RacInputMonitor".to_string())
            .spawn(move || {
                input_monitor.monitor_loop(thread_manager);
            })
            .map_err(|e| RacError::ThreadError(format!("Failed to spawn input monitor: {}", e)))?;

        self.input_monitor_handle = Some(handle);
        Ok(())
    }

    fn main_loop(&mut self) -> RacResult<()> {
        let exit_signal = Arc::clone(&self.exit_signal);
        let (lock, cvar) = &*exit_signal;
        let mut exit_requested = lock.lock()?;

        while !*exit_requested {
            exit_requested = cvar.wait(exit_requested)?;
        }

        drop(exit_requested);

        println!("\n✓ Ctrl+Q detected - Stopping RAC v2...");
        self.shutdown();
        thread::sleep(Duration::from_millis(500));
        println!("✓ RAC v2 stopped successfully!");
        println!("✓ Returning to main menu...\n");
        Ok(())
    }

    fn shutdown(&mut self) {
        self.input_monitor_stop.store(true, Ordering::Release);

        if let Some(handle) = self.input_monitor_handle.take()
            && let Err(_e) = handle.join()
        {
            #[cfg(debug_assertions)]
            eprintln!("Input monitor thread panicked: {:?}", _e);
        }

        if let Some(mut watcher) = self.window_watcher.take() {
            watcher.stop();
        }

        if let Ok(mut tm) = self.thread_manager.lock() {
            let _ = tm.stop_all();
        } else {
            #[cfg(debug_assertions)]
            eprintln!("Failed to lock thread manager during shutdown.");
        }

        self.window_handle.clear();
    }

    fn lock_thread_manager(&self) -> RacResult<std::sync::MutexGuard<'_, ThreadManager>> {
        self.thread_manager
            .lock()
            .map_err(|e| RacError::SyncError(format!("Failed to lock thread manager: {}", e)))
    }
}

pub fn has_configured_hotkeys(profile: &ConfigProfile) -> bool {
    let has_toggle = profile.settings.toggle_hotkey != 0;
    let has_left = profile.settings.left_hotkey != 0;
    let has_right = profile.settings.right_hotkey != 0;

    has_toggle || has_left || has_right
}
