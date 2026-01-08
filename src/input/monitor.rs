use crate::core::{ClickMode, MouseButton, ToggleMode};
use crate::input::HotkeyManager;
use crate::thread::ThreadManager;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_Q};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

pub struct InputMonitor {
    hotkey_manager: RefCell<HotkeyManager>,
    toggle_mode: ToggleMode,
    click_mode: ClickMode,
    toggle_hotkey: i32,
    left_hotkey: i32,
    right_hotkey: i32,
    rac_enabled: bool,
    should_stop: Arc<AtomicBool>,
    exit_signal: Arc<(Mutex<bool>, Condvar)>,
}

impl InputMonitor {
    pub fn with_stop_signal(
        toggle_mode: ToggleMode,
        click_mode: ClickMode,
        left_hotkey: i32,
        right_hotkey: i32,
        toggle_hotkey: i32,
        should_stop: Arc<AtomicBool>,
        exit_signal: Arc<(Mutex<bool>, Condvar)>,
    ) -> Self {
        let mut hotkey_manager = HotkeyManager::new();

        if toggle_hotkey != 0 {
            hotkey_manager.register(toggle_hotkey);
        }
        if left_hotkey != 0 {
            hotkey_manager.register(left_hotkey);
        }
        if right_hotkey != 0 {
            hotkey_manager.register(right_hotkey);
        }

        Self {
            hotkey_manager: RefCell::new(hotkey_manager),
            toggle_mode,
            click_mode,
            toggle_hotkey,
            left_hotkey,
            right_hotkey,
            rac_enabled: false,
            should_stop,
            exit_signal,
        }
    }

    fn should_stop(&self) -> bool {
        self.should_stop.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn is_rac_focused(&self) -> bool {
        unsafe {
            let foreground = GetForegroundWindow();
            if foreground.0.is_null() {
                return false;
            }

            let mut title: [u16; 512] = [0; 512];
            let len = GetWindowTextW(foreground, &mut title);

            if len > 0 {
                const PATTERN: &[u16] = &[
                    b'R' as u16,
                    b'A' as u16,
                    b'C' as u16,
                    b' ' as u16,
                    b'v' as u16,
                    b'2' as u16,
                    b' ' as u16,
                    b'M' as u16,
                    b'a' as u16,
                    b'i' as u16,
                    b'n' as u16,
                    b' ' as u16,
                    b'M' as u16,
                    b'e' as u16,
                    b'n' as u16,
                    b'u' as u16,
                ];

                let title_slice = &title[..len as usize];
                return title_slice.windows(PATTERN.len()).any(|w| w == PATTERN);
            }

            false
        }
    }

    pub fn monitor_loop(&mut self, thread_manager: Arc<Mutex<ThreadManager>>) {
        let auto_enable =
            self.toggle_hotkey == 0 && (self.left_hotkey != 0 || self.right_hotkey != 0);

        if auto_enable {
            self.rac_enabled = true;
            self.enable_workers(&thread_manager);
        }

        loop {
            if self.should_stop() {
                break;
            }

            if self.check_exit_hotkey() {
                self.trigger_exit();
                break;
            }

            if self.toggle_hotkey != 0 {
                self.process_toggle_key(&thread_manager);
            }

            if self.rac_enabled {
                self.process_action_keys(&thread_manager);
            }

            thread::sleep(Duration::from_millis(1));
        }
    }

    #[inline(always)]
    fn check_exit_hotkey(&self) -> bool {
        unsafe {
            let ctrl_pressed = GetAsyncKeyState(VK_CONTROL.0 as i32) < 0;
            let q_pressed = GetAsyncKeyState(VK_Q.0 as i32) < 0;

            ctrl_pressed && q_pressed && self.is_rac_focused()
        }
    }

    fn trigger_exit(&self) {
        let (lock, cvar) = &*self.exit_signal;
        if let Ok(mut exit_requested) = lock.lock() {
            *exit_requested = true;
            cvar.notify_all();
        }
    }

    fn process_toggle_key(&mut self, thread_manager: &Arc<Mutex<ThreadManager>>) {
        if self
            .hotkey_manager
            .borrow_mut()
            .check_toggle(self.toggle_hotkey)
        {
            self.rac_enabled = !self.rac_enabled;

            let tm = match thread_manager.lock() {
                Ok(tm) => tm,
                Err(_) => return,
            };

            if self.rac_enabled {
                self.start_workers(&tm);
            } else {
                self.stop_workers(&tm);
            }
        }
    }

    fn process_action_keys(&mut self, thread_manager: &Arc<Mutex<ThreadManager>>) {
        let tm = match thread_manager.lock() {
            Ok(tm) => tm,
            Err(_) => return,
        };

        match self.toggle_mode {
            ToggleMode::HotkeyHold => {
                self.process_hotkey_hold_mode(&tm);
            }
            ToggleMode::MouseHold => {
                self.process_mouse_hold_mode(&tm);
            }
        }
    }

    fn process_hotkey_hold_mode(&self, tm: &ThreadManager) {
        if self.click_mode.is_left_active()
            && self.left_hotkey != 0
            && let Some(worker) = tm.get_worker(MouseButton::Left)
        {
            worker.set_active(self.hotkey_manager.borrow().is_pressed(self.left_hotkey));
        }

        if self.click_mode.is_right_active()
            && self.right_hotkey != 0
            && let Some(worker) = tm.get_worker(MouseButton::Right)
        {
            worker.set_active(self.hotkey_manager.borrow().is_pressed(self.right_hotkey));
        }
    }

    fn process_mouse_hold_mode(&self, tm: &ThreadManager) {
        let same_hotkey = self.left_hotkey != 0 && self.left_hotkey == self.right_hotkey;

        if self.click_mode.is_left_active() && self.left_hotkey != 0 {
            if self
                .hotkey_manager
                .borrow_mut()
                .check_toggle(self.left_hotkey)
            {
                if let Some(worker) = tm.get_worker(MouseButton::Left) {
                    let current = worker.is_active();
                    worker.set_active(!current);
                }

                if same_hotkey
                    && self.click_mode.is_right_active()
                    && let Some(worker) = tm.get_worker(MouseButton::Right)
                {
                    let current = worker.is_active();
                    worker.set_active(!current);
                }
            }
        } else if self.click_mode.is_right_active()
            && self.right_hotkey != 0
            && self
                .hotkey_manager
                .borrow_mut()
                .check_toggle(self.right_hotkey)
            && let Some(worker) = tm.get_worker(MouseButton::Right)
        {
            let current = worker.is_active();
            worker.set_active(!current);
        }
    }

    fn enable_workers(&self, thread_manager: &Arc<Mutex<ThreadManager>>) {
        let tm = match thread_manager.lock() {
            Ok(tm) => tm,
            Err(_) => return,
        };

        self.start_workers(&tm);
    }

    fn start_workers(&self, tm: &ThreadManager) {
        if self.click_mode.is_left_active() {
            let _ = tm.start_signal(MouseButton::Left);
            if self.toggle_mode == ToggleMode::MouseHold
                && let Some(worker) = tm.get_worker(MouseButton::Left)
            {
                worker.set_active(true);
            }
        }

        if self.click_mode.is_right_active() {
            let _ = tm.start_signal(MouseButton::Right);
            if self.toggle_mode == ToggleMode::MouseHold
                && let Some(worker) = tm.get_worker(MouseButton::Right)
            {
                worker.set_active(true);
            }
        }
    }

    fn stop_workers(&self, tm: &ThreadManager) {
        let _ = tm.pause_signal(MouseButton::Left);
        let _ = tm.pause_signal(MouseButton::Right);

        if let Some(worker) = tm.get_worker(MouseButton::Left) {
            worker.set_active(false);
        }
        if let Some(worker) = tm.get_worker(MouseButton::Right) {
            worker.set_active(false);
        }
    }
}
