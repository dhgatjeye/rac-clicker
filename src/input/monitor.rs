use crate::core::{ToggleMode, ClickMode, MouseButton};
use crate::input::HotkeyManager;
use crate::thread::ThreadManager;
use std::sync::Arc;
use std::time::Duration;

pub struct InputMonitor {
    hotkey_manager: HotkeyManager,
    toggle_mode: ToggleMode,
    click_mode: ClickMode,
    toggle_hotkey: i32,
    left_hotkey: i32,
    right_hotkey: i32,
    rac_enabled: bool
}

impl InputMonitor {
    pub fn new(
        toggle_mode: ToggleMode,
        click_mode: ClickMode,
        toggle_hotkey: i32,
        left_hotkey: i32,
        right_hotkey: i32,
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
            hotkey_manager,
            toggle_mode,
            click_mode,
            toggle_hotkey,
            left_hotkey,
            right_hotkey,
            rac_enabled: false,
        }
    }

    pub fn update_config(
        &mut self,
        toggle_mode: ToggleMode,
        click_mode: ClickMode,
        toggle_hotkey: i32,
        left_hotkey: i32,
        right_hotkey: i32,
    ) {
        self.toggle_mode = toggle_mode;
        self.click_mode = click_mode;

        self.hotkey_manager.clear();
        self.hotkey_manager.register(toggle_hotkey);
        self.hotkey_manager.register(left_hotkey);
        self.hotkey_manager.register(right_hotkey);

        self.toggle_hotkey = toggle_hotkey;
        self.left_hotkey = left_hotkey;
        self.right_hotkey = right_hotkey;
    }

    pub fn monitor_loop(&mut self, thread_manager: Arc<std::sync::Mutex<ThreadManager>>) {
        let auto_enable = self.toggle_hotkey == 0
            && (self.left_hotkey != 0 || self.right_hotkey != 0);

        if auto_enable {
            self.rac_enabled = true;
            
            let tm = match thread_manager.lock() {
                Ok(tm) => tm,
                Err(_) => return,
            };

            if self.click_mode.is_left_active() {
                let _ = tm.start_signal(MouseButton::Left);
                if self.toggle_mode == ToggleMode::MouseHold {
                    if let Some(worker) = tm.get_worker(MouseButton::Left) {
                        worker.set_active(true);
                    }
                }
            }
            if self.click_mode.is_right_active() {
                let _ = tm.start_signal(MouseButton::Right);
                if self.toggle_mode == ToggleMode::MouseHold {
                    if let Some(worker) = tm.get_worker(MouseButton::Right) {
                        worker.set_active(true);
                    }
                }
            }
        }

        loop {
            std::thread::sleep(Duration::from_millis(10));
            
            if self.toggle_hotkey != 0 {
                if self.hotkey_manager.check_toggle(self.toggle_hotkey) {
                    self.rac_enabled = !self.rac_enabled;

                    let tm = match thread_manager.lock() {
                        Ok(tm) => tm,
                        Err(_) => continue,
                    };

                    if self.rac_enabled {
                        if self.click_mode.is_left_active() {
                            let _ = tm.start_signal(MouseButton::Left);
                            if self.toggle_mode == ToggleMode::MouseHold {
                                if let Some(worker) = tm.get_worker(MouseButton::Left) {
                                    worker.set_active(true);
                                }
                            }
                        }
                        if self.click_mode.is_right_active() {
                            let _ = tm.start_signal(MouseButton::Right);
                            if self.toggle_mode == ToggleMode::MouseHold {
                                if let Some(worker) = tm.get_worker(MouseButton::Right) {
                                    worker.set_active(true);
                                }
                            }
                        }
                    } else {
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
            }
            
            if self.rac_enabled {
                let tm = match thread_manager.lock() {
                    Ok(tm) => tm,
                    Err(_) => continue,
                };

                match self.toggle_mode {
                    ToggleMode::HotkeyHold => {
                        if self.click_mode.is_left_active() && self.left_hotkey != 0 {
                            if let Some(worker) = tm.get_worker(MouseButton::Left) {
                                worker.set_active(self.hotkey_manager.is_pressed(self.left_hotkey));
                            }
                        }
                        
                        if self.click_mode.is_right_active() && self.right_hotkey != 0 {
                            if let Some(worker) = tm.get_worker(MouseButton::Right) {
                                worker.set_active(self.hotkey_manager.is_pressed(self.right_hotkey));
                            }
                        }
                    }
                    ToggleMode::MouseHold => {
                        let same_hotkey = self.left_hotkey != 0 
                            && self.left_hotkey == self.right_hotkey;
                        
                        if self.click_mode.is_left_active() && self.left_hotkey != 0 {
                            if self.hotkey_manager.check_toggle(self.left_hotkey) {
                                if let Some(worker) = tm.get_worker(MouseButton::Left) {
                                    let current = worker.is_active();
                                    worker.set_active(!current);
                                }
                                if same_hotkey && self.click_mode.is_right_active() {
                                    if let Some(worker) = tm.get_worker(MouseButton::Right) {
                                        let current = worker.is_active();
                                        worker.set_active(!current);
                                    }
                                }
                            }
                        } else if self.click_mode.is_right_active() && self.right_hotkey != 0 {
                            if self.hotkey_manager.check_toggle(self.right_hotkey) {
                                if let Some(worker) = tm.get_worker(MouseButton::Right) {
                                    let current = worker.is_active();
                                    worker.set_active(!current);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    pub fn is_enabled(&self) -> bool {
        self.rac_enabled
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.rac_enabled = enabled;
    }
}