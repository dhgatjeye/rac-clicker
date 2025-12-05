use crate::core::{ClickMode, ToggleMode};
use crate::input::HotkeyManager;
use crate::thread::{ClickWorker, SyncSignal};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct MonitorConfig {
    pub toggle_mode: ToggleMode,
    pub click_mode: ClickMode,
    pub toggle_hotkey: i32,
    pub left_hotkey: i32,
    pub right_hotkey: i32,
}

struct WorkerControls {
    left_signal: Option<Arc<SyncSignal>>,
    left_active: Option<Arc<AtomicBool>>,
    right_signal: Option<Arc<SyncSignal>>,
    right_active: Option<Arc<AtomicBool>>,
}

impl WorkerControls {
    fn from_workers(left: Option<&Arc<ClickWorker>>, right: Option<&Arc<ClickWorker>>) -> Self {
        Self {
            left_signal: left.map(|w| w.signal()),
            left_active: left.map(|w| w.active_flag()),
            right_signal: right.map(|w| w.signal()),
            right_active: right.map(|w| w.active_flag()),
        }
    }

    #[inline]
    fn set_left_active(&self, active: bool) {
        if let Some(ref flag) = self.left_active {
            flag.store(active, Ordering::Release);
        }
    }

    #[inline]
    fn set_right_active(&self, active: bool) {
        if let Some(ref flag) = self.right_active {
            flag.store(active, Ordering::Release);
        }
    }

    #[inline]
    fn is_left_active(&self) -> bool {
        self.left_active
            .as_ref()
            .map(|f| f.load(Ordering::Acquire))
            .unwrap_or(false)
    }

    #[inline]
    fn is_right_active(&self) -> bool {
        self.right_active
            .as_ref()
            .map(|f| f.load(Ordering::Acquire))
            .unwrap_or(false)
    }

    #[inline]
    fn start_left(&self) {
        if let Some(ref signal) = self.left_signal {
            signal.start();
        }
    }

    #[inline]
    fn start_right(&self) {
        if let Some(ref signal) = self.right_signal {
            signal.start();
        }
    }

    #[inline]
    fn pause_left(&self) {
        if let Some(ref signal) = self.left_signal {
            signal.pause();
        }
    }

    #[inline]
    fn pause_right(&self) {
        if let Some(ref signal) = self.right_signal {
            signal.pause();
        }
    }

    fn start_all(&self, click_mode: ClickMode) {
        if click_mode.is_left_active() {
            self.start_left();
        }
        if click_mode.is_right_active() {
            self.start_right();
        }
    }

    fn pause_all(&self) {
        self.pause_left();
        self.pause_right();
        self.set_left_active(false);
        self.set_right_active(false);
    }
}

struct MonitorState {
    stop_signal: AtomicBool,
    rac_enabled: AtomicBool,
}

impl MonitorState {
    const fn new() -> Self {
        Self {
            stop_signal: AtomicBool::new(false),
            rac_enabled: AtomicBool::new(false),
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
    fn is_enabled(&self) -> bool {
        self.rac_enabled.load(Ordering::Acquire)
    }

    #[inline]
    fn set_enabled(&self, enabled: bool) {
        self.rac_enabled.store(enabled, Ordering::Release);
    }

    #[inline]
    fn toggle(&self) -> bool {
        let was_enabled = self.rac_enabled.fetch_xor(true, Ordering::AcqRel);
        !was_enabled
    }
}

pub struct InputMonitor {
    state: Arc<MonitorState>,
    thread: Option<thread::JoinHandle<()>>,
}

impl InputMonitor {
    pub fn spawn(
        config: MonitorConfig,
        left_worker: Option<&Arc<ClickWorker>>,
        right_worker: Option<&Arc<ClickWorker>>,
    ) -> Self {
        let state = Arc::new(MonitorState::new());
        let state_clone = Arc::clone(&state);
        let controls = WorkerControls::from_workers(left_worker, right_worker);

        let thread = thread::Builder::new()
            .name("InputMonitor".into())
            .spawn(move || {
                Self::monitor_loop(state_clone, config, controls);
            })
            .expect("Failed to spawn input monitor thread");

        Self {
            state,
            thread: Some(thread),
        }
    }

    #[inline]
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.state.is_enabled()
    }

    pub fn stop(&mut self) {
        self.state.request_stop();

        if let Some(thread) = self.thread.take() {
            thread::sleep(Duration::from_millis(15));
            let _ = thread.join();
        }
    }

    fn monitor_loop(state: Arc<MonitorState>, config: MonitorConfig, controls: WorkerControls) {
        let mut hotkeys = HotkeyManager::new();

        if config.toggle_hotkey != 0 {
            hotkeys.register(config.toggle_hotkey);
        }
        if config.left_hotkey != 0 {
            hotkeys.register(config.left_hotkey);
        }
        if config.right_hotkey != 0 {
            hotkeys.register(config.right_hotkey);
        }

        let auto_enable = config.toggle_hotkey == 0
            && (config.left_hotkey != 0 || config.right_hotkey != 0);

        if auto_enable {
            state.set_enabled(true);
            controls.start_all(config.click_mode);

            if config.toggle_mode == ToggleMode::MouseHold {
                if config.click_mode.is_left_active() {
                    controls.set_left_active(true);
                }
                if config.click_mode.is_right_active() {
                    controls.set_right_active(true);
                }
            }
        }

        while !state.should_stop() {
            thread::sleep(Duration::from_millis(10));

            if config.toggle_hotkey != 0 && hotkeys.check_toggle(config.toggle_hotkey) {
                let now_enabled = state.toggle();

                if now_enabled {
                    Self::enable_workers(&config, &controls);
                } else {
                    controls.pause_all();
                }
            }

            if state.is_enabled() {
                Self::handle_toggle_mode(&config, &controls, &mut hotkeys);
            }
        }
    }

    fn enable_workers(config: &MonitorConfig, controls: &WorkerControls) {
        controls.start_all(config.click_mode);

        if config.toggle_mode == ToggleMode::MouseHold {
            if config.click_mode.is_left_active() {
                controls.set_left_active(true);
            }
            if config.click_mode.is_right_active() {
                controls.set_right_active(true);
            }
        }
    }

    fn handle_toggle_mode(
        config: &MonitorConfig,
        controls: &WorkerControls,
        hotkeys: &mut HotkeyManager,
    ) {
        match config.toggle_mode {
            ToggleMode::HotkeyHold => {
                if config.click_mode.is_left_active() && config.left_hotkey != 0 {
                    controls.set_left_active(hotkeys.is_pressed(config.left_hotkey));
                }
                if config.click_mode.is_right_active() && config.right_hotkey != 0 {
                    controls.set_right_active(hotkeys.is_pressed(config.right_hotkey));
                }
            }
            ToggleMode::MouseHold => {
                let same_hotkey =
                    config.left_hotkey != 0 && config.left_hotkey == config.right_hotkey;

                if config.click_mode.is_left_active() && config.left_hotkey != 0 {
                    if hotkeys.check_toggle(config.left_hotkey) {
                        let new_state = !controls.is_left_active();
                        controls.set_left_active(new_state);

                        if same_hotkey && config.click_mode.is_right_active() {
                            controls.set_right_active(new_state);
                        }
                    }
                }
                
                if !same_hotkey && config.click_mode.is_right_active() && config.right_hotkey != 0 {
                    if hotkeys.check_toggle(config.right_hotkey) {
                        controls.set_right_active(!controls.is_right_active());
                    }
                }
            }
        }
    }
}

impl Drop for InputMonitor {
    fn drop(&mut self) {
        self.stop();
    }
}