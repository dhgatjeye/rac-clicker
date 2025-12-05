use crate::{
    ClickController, ClickExecutor, ClickWorker, ConfigProfile, DelayCalculator,
    HotkeyManager, InputMonitor, MonitorConfig, MouseButton, RacError, RacResult,
    ThreadManager, WindowFinder, WindowHandle, WindowWatcher, WorkerConfig,
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct RacApp {
    profile: ConfigProfile,
    thread_manager: Arc<Mutex<ThreadManager>>,
    window_handle: Arc<WindowHandle>,
    window_watcher: Option<WindowWatcher>,
    input_monitor: Option<InputMonitor>,
}

impl Drop for RacApp {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl RacApp {
    pub fn new(profile: ConfigProfile) -> RacResult<Self> {
        Ok(Self {
            profile,
            thread_manager: Arc::new(Mutex::new(ThreadManager::new())),
            window_handle: Arc::new(WindowHandle::new()),
            window_watcher: None,
            input_monitor: None,
        })
    }

    pub fn run(&mut self) -> RacResult<()> {
        self.print_startup_banner();
        self.initialize()?;
        self.print_configuration()?;
        self.wait_for_exit()
    }

    fn initialize(&mut self) -> RacResult<()> {
        println!("→ Initializing workers...");
        self.initialize_workers()?;

        println!("→ Starting worker threads...");
        self.start_workers()?;

        println!("→ Starting window watcher...");
        self.start_window_watcher()?;

        println!("→ Starting input monitor...");
        self.start_input_monitor()?;

        Ok(())
    }

    fn shutdown(&mut self) {
        self.input_monitor.take();
        self.window_watcher.take();

        let mut tm = self
            .thread_manager
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = tm.stop_all();
    }

    fn initialize_workers(&mut self) -> RacResult<()> {
        let server_config = self.profile.server_registry.get_active()?;
        let left_cps = self.profile.get_left_cps()?;
        let right_cps = self.profile.get_right_cps()?;

        let mut tm = self
            .thread_manager
            .lock()
            .map_err(|e| RacError::SyncError(format!("Failed to lock thread manager: {}", e)))?;

        if self.profile.settings.click_mode.is_left_active() {
            let mut pattern = server_config.left_click;
            pattern.max_cps = left_cps;
            tm.register_worker(ClickWorker::new(WorkerConfig::left_click(pattern)));
        }

        if self.profile.settings.click_mode.is_right_active() {
            let mut pattern = server_config.right_click;
            pattern.max_cps = right_cps;
            tm.register_worker(ClickWorker::new(WorkerConfig::right_click(pattern)));
        }

        Ok(())
    }

    fn start_workers(&mut self) -> RacResult<()> {
        let click_mode = self.profile.settings.click_mode;

        if click_mode.is_left_active() {
            self.start_worker_for_button(MouseButton::Left)?;
        }

        if click_mode.is_right_active() {
            self.start_worker_for_button(MouseButton::Right)?;
        }

        Ok(())
    }

    fn start_worker_for_button(&mut self, button: MouseButton) -> RacResult<()> {
        let toggle_mode = self.profile.settings.toggle_mode;
        let server_type = self.profile.settings.active_server;
        let window_handle = Arc::clone(&self.window_handle);

        let tm = self
            .thread_manager
            .lock()
            .map_err(|e| RacError::SyncError(e.to_string()))?;

        let worker = match tm.get_worker(button) {
            Some(w) => w,
            None => return Ok(()),
        };

        let delay_calc = DelayCalculator::new(worker.config().pattern, button, server_type)?;
        let controller = ClickController::new(toggle_mode, ClickExecutor::new());

        worker.signal().pause();
        drop(tm);

        let mut tm = self
            .thread_manager
            .lock()
            .map_err(|e| RacError::SyncError(e.to_string()))?;

        tm.start_worker(button, move |worker| {
            let mut delay = delay_calc;
            controller.run_loop(worker, &mut delay, || window_handle.get());
        })
    }

    fn start_window_watcher(&mut self) -> RacResult<()> {
        let target_process = self.profile.get_target_process()?;
        let finder = WindowFinder::new(&target_process);

        self.window_watcher = Some(WindowWatcher::spawn_default(
            finder,
            Arc::clone(&self.window_handle),
        ));

        Ok(())
    }

    fn start_input_monitor(&mut self) -> RacResult<()> {
        let settings = &self.profile.settings;

        let config = MonitorConfig {
            toggle_mode: settings.toggle_mode,
            click_mode: settings.click_mode,
            toggle_hotkey: settings.toggle_hotkey,
            left_hotkey: settings.left_hotkey,
            right_hotkey: settings.right_hotkey,
        };

        let tm = self
            .thread_manager
            .lock()
            .map_err(|e| RacError::SyncError(e.to_string()))?;

        let left_worker = tm.get_worker(MouseButton::Left);
        let right_worker = tm.get_worker(MouseButton::Right);

        self.input_monitor = Some(InputMonitor::spawn(
            config,
            left_worker.as_ref(),
            right_worker.as_ref(),
        ));

        Ok(())
    }

    fn wait_for_exit(&mut self) -> RacResult<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_Q};

        loop {
            thread::sleep(Duration::from_millis(50));

            let exit_requested = unsafe {
                let ctrl = GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
                let q = GetAsyncKeyState(VK_Q.0 as i32) < 0;
                ctrl && q
            };

            if exit_requested {
                println!("\n✓ Ctrl+Q detected - Stopping RAC v2...");
                self.shutdown();
                thread::sleep(Duration::from_millis(500));
                println!("✓ RAC v2 stopped successfully!");
                println!("✓ Returning to main menu...\n");
                return Ok(());
            }
        }
    }

    fn print_startup_banner(&self) {
        println!("╔═══════════════════════════════════════════╗");
        println!("║            RAC v2 - Starting...           ║");
        println!("╚═══════════════════════════════════════════╝");
        println!("\n");
    }

    fn print_configuration(&self) -> RacResult<()> {
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
            println!("  Left CPS:     {}", self.profile.get_left_cps()?);
        }
        if self.profile.settings.click_mode.is_right_active() {
            println!("  Right CPS:    {}", self.profile.get_right_cps()?);
        }

        println!("\nPress Ctrl+Q to return to main menu...\n");
        Ok(())
    }
}