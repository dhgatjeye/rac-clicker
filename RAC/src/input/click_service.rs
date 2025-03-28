use crate::input::click_executor::{ClickExecutor, MouseButton, GameMode};
use crate::input::delay_provider::DelayProvider;
use crate::input::handle::Handle;
use crate::input::sync_controller::SyncController;
use crate::input::thread_controller::ThreadController;
use crate::input::window_finder::WindowFinder;
use crate::logger::logger::{log_error, log_info};
use crate::config::settings::Settings;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};
use winapi::um::winuser::GetAsyncKeyState;

pub struct ClickServiceConfig {
    pub target_process: String,
    pub window_check_active_interval: Duration,
    pub window_check_idle_interval: Duration,
    pub adaptive_cpu_mode: bool,
}

impl Default for ClickServiceConfig {
    fn default() -> Self {
        let settings = Settings::load().unwrap_or_else(|_| Settings::default());

        Self {
            target_process: settings.target_process,
            window_check_active_interval: Duration::from_secs(1),
            window_check_idle_interval: Duration::from_secs(3),
            adaptive_cpu_mode: settings.adaptive_cpu_mode,
        }
    }
}

pub struct ClickService {
    sync_controller: Arc<SyncController>,
    pub(crate) delay_provider: Arc<Mutex<DelayProvider>>,
    hwnd: Arc<Mutex<Handle>>,
    window_finder: Arc<WindowFinder>,
    pub(crate) click_executor: Arc<ClickExecutor>,
    config: ClickServiceConfig,
    settings: Arc<Mutex<Settings>>,
    window_finder_running: Arc<AtomicBool>,
    left_click_enabled: Arc<AtomicBool>,
    right_click_enabled: Arc<AtomicBool>,
    left_click_controller: Arc<SyncController>,
    right_click_controller: Arc<SyncController>,
    left_delay_provider: Arc<Mutex<DelayProvider>>,
    right_delay_provider: Arc<Mutex<DelayProvider>>,
    left_thread_controller: Arc<ThreadController>,
    right_thread_controller: Arc<ThreadController>,
    pub(crate) left_click_executor: Arc<ClickExecutor>,
    pub(crate) right_click_executor: Arc<ClickExecutor>
}

impl ClickService {
    pub fn new(config: ClickServiceConfig) -> Arc<Self> {
        let context = "ClickService::new";
        let settings = Settings::load().unwrap_or_else(|_| Settings::default());
        let settings_clone = settings.clone();
        let adaptive_cpu_mode = config.adaptive_cpu_mode;

        let left_thread_controller = Arc::new(ThreadController::new(adaptive_cpu_mode));
        let right_thread_controller = Arc::new(ThreadController::new(adaptive_cpu_mode));

        let service = Arc::new(Self {
            sync_controller: Arc::new(SyncController::new()),
            delay_provider: Arc::new(Mutex::new(DelayProvider::new())),
            hwnd: Arc::new(Mutex::new(Handle::new())),
            window_finder: Arc::new(WindowFinder::new(&config.target_process)),
            click_executor: Arc::new(ClickExecutor::new((*left_thread_controller).clone())),
            config,
            settings: Arc::new(Mutex::new(settings)),
            window_finder_running: Arc::new(AtomicBool::new(true)),
            left_click_enabled: Arc::new(AtomicBool::new(false)),
            right_click_enabled: Arc::new(AtomicBool::new(false)),
            left_click_controller: Arc::new(SyncController::new()),
            right_click_controller: Arc::new(SyncController::new()),
            left_delay_provider: Arc::new(Mutex::new(DelayProvider::new())),
            right_delay_provider: Arc::new(Mutex::new(DelayProvider::new())),
            left_thread_controller: left_thread_controller.clone(),
            right_thread_controller: right_thread_controller.clone(),
            left_click_executor: Arc::new(ClickExecutor::new((*left_thread_controller).clone())),
            right_click_executor: Arc::new(ClickExecutor::new((*right_thread_controller).clone())),
        });

        let left_click_executor = Arc::clone(&service.left_click_executor);
        left_click_executor.set_max_cps(settings_clone.left_max_cps);
        left_click_executor.set_mouse_button(MouseButton::Left);
        let left_mode = match settings_clone.left_game_mode.as_str() {
            "Combo" => GameMode::Combo,
            _ => GameMode::Default,
        };
        left_click_executor.set_game_mode(left_mode);

        let right_click_executor = Arc::clone(&service.right_click_executor);
        right_click_executor.set_max_cps(settings_clone.right_max_cps);
        right_click_executor.set_mouse_button(MouseButton::Right);
        let right_mode = match settings_clone.right_game_mode.as_str() {
            "Combo" => GameMode::Combo,
            _ => GameMode::Default,
        };
        right_click_executor.set_game_mode(right_mode);

        let service_clone = service.clone();
        match thread::Builder::new()
            .name("WindowFinderThread".to_string())
            .spawn(move || {
                service_clone.window_finder_loop();
            }) {
            Ok(_) => {
                log_info("Window finder thread spawned successfully", context);
            }
            Err(e) => {
                log_error(&format!("Failed to spawn window finder thread: {}", e), context);
            }
        }

        let service_clone = service.clone();
        match thread::Builder::new()
            .name("SettingsSyncThread".to_string())
            .spawn(move || {
                service_clone.settings_sync_loop();
            }) {
            Ok(_) => {
                log_info("Settings synchronization thread spawned successfully", context);
            }
            Err(e) => {
                log_error(&format!("Failed to spawn settings sync thread: {}", e), context);
            }
        }

        let service_clone = service.clone();
        Self::spawn_click_thread("LeftClickThread", service_clone.clone(), MouseButton::Left);

        let service_clone = service.clone();
        Self::spawn_click_thread("RightClickThread", service_clone.clone(), MouseButton::Right);

        service
    }

    fn window_finder_loop(&self) {
        let context = "ClickService::window_finder_loop";
        log_info("Window finder thread started", context);

        self.left_thread_controller.set_idle_priority();

        while !thread::panicking() && self.window_finder_running.load(Ordering::SeqCst) {
            let check_interval = if self.is_enabled() {
                self.config.window_check_active_interval
            } else {
                self.config.window_check_idle_interval
            };

            self.window_finder.find_target_window(&self.hwnd);

            thread::sleep(check_interval);
        }

        log_info("Window finder thread terminated", context);
    }

    fn settings_sync_loop(&self) {
        let context = "ClickService::settings_sync_loop";
        log_info("Settings synchronization thread started", context);

        self.left_thread_controller.set_idle_priority();

        while !thread::panicking() {
            self.check_and_update_settings();

            thread::sleep(Duration::from_secs(5));
        }

        log_error("Settings sync loop terminated due to thread panic", context);
    }

    fn check_and_update_settings(&self) {
        let context = "ClickService::check_and_update_settings";

        match Settings::load() {
            Ok(new_settings) => {
                let target_process;
                let target_process_new = new_settings.target_process.clone();
                let adaptive_cpu_mode;
                let click_delay_micros;
                let delay_range_min;
                let delay_range_max;
                let random_deviation_min;
                let random_deviation_max;

                {
                    let current_settings = self.settings.lock().unwrap();
                    target_process = current_settings.target_process.clone();
                    adaptive_cpu_mode = current_settings.adaptive_cpu_mode;
                    click_delay_micros = current_settings.click_delay_micros;
                    delay_range_min = current_settings.delay_range_min;
                    delay_range_max = current_settings.delay_range_max;
                    random_deviation_min = current_settings.random_deviation_min;
                    random_deviation_max = current_settings.random_deviation_max;
                }

                let target_process_changed = target_process != target_process_new;
                let adaptive_cpu_mode_changed = adaptive_cpu_mode != new_settings.adaptive_cpu_mode;
                let click_delay_changed = click_delay_micros != new_settings.click_delay_micros;
                let delay_range_changed =
                    delay_range_min != new_settings.delay_range_min ||
                        delay_range_max != new_settings.delay_range_max;
                let deviation_changed =
                    random_deviation_min != new_settings.random_deviation_min ||
                        random_deviation_max != new_settings.random_deviation_max;

                {
                    let mut current_settings = self.settings.lock().unwrap();
                    *current_settings = new_settings;
                }

                if target_process_changed {
                    let _ = self.window_finder.update_target_process(&target_process_new);
                }

                if adaptive_cpu_mode_changed {
                    self.left_thread_controller.set_adaptive_mode(!adaptive_cpu_mode);
                }

                if click_delay_changed || delay_range_changed || deviation_changed {
                    if delay_range_changed || deviation_changed {
                        if let Ok(mut delay_provider) = self.delay_provider.lock() {
                            delay_provider.update_settings(
                                delay_range_min,
                                delay_range_max,
                                random_deviation_min,
                                random_deviation_max
                            );
                        }
                    }

                    if click_delay_changed {
                        self.click_executor.update_delay(click_delay_micros);
                    }
                }
            },
            Err(e) => {
                log_error(&format!("Failed to reload settings: {}", e), context);
            }
        }
    }

    pub fn click_loop(&self, button: MouseButton) {
        let context = match button {
            MouseButton::Left => "ClickService::left_click_loop",
            MouseButton::Right => "ClickService::right_click_loop",
        };

        let click_controller = match button {
            MouseButton::Left => Arc::clone(&self.left_click_controller),
            MouseButton::Right => Arc::clone(&self.right_click_controller),
        };

        let delay_provider = match button {
            MouseButton::Left => Arc::clone(&self.left_delay_provider),
            MouseButton::Right => Arc::clone(&self.right_delay_provider),
        };

        let thread_controller = match button {
            MouseButton::Left => Arc::clone(&self.left_thread_controller),
            MouseButton::Right => Arc::clone(&self.right_thread_controller),
        };

        let click_executor = match button {
            MouseButton::Left => Arc::clone(&self.left_click_executor),
            MouseButton::Right => Arc::clone(&self.right_click_executor),
        };

        thread_controller.set_active_priority();
        thread_controller.set_adaptive_mode(!self.config.adaptive_cpu_mode);

        let mut consecutive_failures = 0;
        let mut last_click = Instant::now();

        let settings = Settings::load().unwrap_or_default();
        match button {
            MouseButton::Left => {
                click_executor.set_max_cps(settings.left_max_cps);
                let mode = match settings.left_game_mode.as_str() {
                    "Combo" => GameMode::Combo,
                    _ => GameMode::Default,
                };
                click_executor.set_game_mode(mode);
            },
            MouseButton::Right => {
                click_executor.set_max_cps(settings.right_max_cps);
                let mode = match settings.right_game_mode.as_str() {
                    "Combo" => GameMode::Combo,
                    _ => GameMode::Default,
                };
                click_executor.set_game_mode(mode);
            }
        }

        let mut last_button_state = false;

        while !thread::panicking() {
            if !click_controller.wait_for_signal(Duration::from_millis(50)) {
                continue;
            }

            let settings = Settings::load().unwrap_or_default();
            let is_keyboard_hold_mode = settings.keyboard_hold_mode;

            let should_click = if is_keyboard_hold_mode {
                click_executor.is_active()
            } else {
                let is_pressed = match button {
                    MouseButton::Left => unsafe { GetAsyncKeyState(0x01) < 0 },
                    MouseButton::Right => unsafe { GetAsyncKeyState(0x02) < 0 },
                };

                if last_button_state && !is_pressed {
                    match button {
                        MouseButton::Left => self.left_click_executor.handle_button_release(),
                        MouseButton::Right => self.right_click_executor.handle_button_release(),
                    }
                }
                last_button_state = is_pressed;

                is_pressed && click_executor.is_active()
            };

            if !should_click {
                continue;
            }

            let hwnd = {
                let hwnd_guard = self.hwnd.lock().unwrap();
                hwnd_guard.get()
            };

            if click_executor.execute_click(hwnd) {
                consecutive_failures = 0;

                let delay = {
                    let mut delay_provider = delay_provider.lock().unwrap();
                    delay_provider.get_next_delay()
                };

                let elapsed = last_click.elapsed();
                if elapsed < delay {
                    thread_controller.smart_sleep(delay.saturating_sub(elapsed));
                }
                last_click = Instant::now();
            } else {
                consecutive_failures += 1;

                if consecutive_failures >= 3 {
                    consecutive_failures = 0;
                }

                thread_controller.smart_sleep(Duration::from_millis(20));
            }
        }

        self.window_finder_running.store(false, Ordering::SeqCst);
        log_error("Click loop terminated due to thread panic", &context);
    }

    pub fn toggle(&self) -> bool {
        self.sync_controller.toggle()
    }

    pub fn is_enabled(&self) -> bool {
        self.sync_controller.is_enabled()
    }

    pub fn force_enable_clicking(&self) -> bool {
        if self.is_enabled() {
            return true;
        }

        self.sync_controller.force_enable()
    }

    pub fn force_disable_clicking(&self) -> bool {
        if !self.is_enabled() {
            return true;
        }

        if self.sync_controller.is_enabled() {
            self.sync_controller.toggle();
        }

        true
    }

    pub fn force_enable_left_clicking(&self) -> bool {
        if self.left_click_controller.is_enabled() {
            return true;
        }
        self.left_click_controller.force_enable()
    }

    pub fn force_enable_right_clicking(&self) -> bool {
        if self.right_click_controller.is_enabled() {
            return true;
        }
        self.right_click_controller.force_enable()
    }

    pub fn force_disable_left_clicking(&self) -> bool {
        if !self.left_click_controller.is_enabled() {
            return true;
        }
        self.left_click_controller.toggle()
    }

    pub fn force_disable_right_clicking(&self) -> bool {
        if !self.right_click_controller.is_enabled() {
            return true;
        }
        self.right_click_controller.toggle()
    }

    pub fn get_left_click_executor(&self) -> Arc<ClickExecutor> {
        Arc::clone(&self.left_click_executor)
    }

    pub fn get_right_click_executor(&self) -> Arc<ClickExecutor> {
        Arc::clone(&self.right_click_executor)
    }

    pub fn set_left_click_cps(&self, cps: u8) {
        self.left_click_executor.set_max_cps(cps);
    }

    pub fn set_right_click_cps(&self, cps: u8) {
        self.right_click_executor.set_max_cps(cps);
    }

    pub fn start(&self) {
        self.left_click_executor.set_active(true);
        self.right_click_executor.set_active(true);
    }

    pub fn stop(&self) {
        self.left_click_executor.set_active(false);
        self.right_click_executor.set_active(false);
    }

    fn spawn_click_thread(name: &str, service: Arc<ClickService>, button: MouseButton) {
        let context = format!("ClickService::{}", name);

        match thread::Builder::new()
            .name(name.to_string())
            .spawn(move || {
                service.click_loop(button);
            }) {
            Ok(_) => {
                log_info(&format!("{} spawned successfully", name), &context);
            }
            Err(e) => {
                log_error(&format!("Failed to spawn {}: {}", name, e), &context);
            }
        }
    }
}
