use crate::core::{MouseButton, ToggleMode};
use crate::clicker::{ClickExecutor, DelayCalculator};
use crate::thread::worker::ClickWorker;
use crate::thread::sync::SmartSleep;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON};
use std::sync::Arc;
use std::time::Duration;

pub struct ClickController {
    executor: ClickExecutor,
    toggle_mode: ToggleMode,
}

impl ClickController {
    pub fn new(toggle_mode: ToggleMode, executor: ClickExecutor) -> Self {
        Self {
            executor,
            toggle_mode,
        }
    }
    
    pub fn run_loop(
        &self,
        worker: Arc<ClickWorker>,
        delay_calc: &mut DelayCalculator,
        hwnd_provider: impl Fn() -> HWND,
    ) {
        let mut last_button_state = false;

        loop {
            if worker.signal().is_stopped() {
                break;
            }
            
            if !worker.signal().wait_for_running(Duration::from_millis(100)) {
                std::thread::yield_now();
                continue;
            }
            
            let should_click = match self.toggle_mode {
                ToggleMode::HotkeyHold => {
                    worker.is_active()
                }
                ToggleMode::MouseHold => {
                    let is_pressed = self.is_button_pressed(worker.config().button);

                    if last_button_state && !is_pressed {
                        delay_calc.reset_on_release();
                        last_button_state = false;
                        std::thread::yield_now();
                        continue;
                    }

                    last_button_state = is_pressed;
                    is_pressed && worker.signal().is_running() && worker.is_active()
                }
            };

            if !should_click {
                std::thread::yield_now();
                continue;
            }
            
            let hwnd = hwnd_provider();
            if hwnd.is_invalid() {
                SmartSleep::sleep(Duration::from_millis(20));
                continue;
            }
            
            let hold_duration = delay_calc.hold_duration();
            if let Err(_) = self.executor.execute_click(
                hwnd,
                worker.config().button,
                hold_duration,
            ) {
                SmartSleep::sleep(Duration::from_millis(20));
                continue;
            }
            
            let delay = delay_calc.next_delay();
            SmartSleep::sleep(delay);
        }
    }
    
    fn is_button_pressed(&self, button: MouseButton) -> bool {
        unsafe {
            match button {
                MouseButton::Left => GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0,
                MouseButton::Right => GetAsyncKeyState(VK_RBUTTON.0 as i32) < 0,
            }
        }
    }
}