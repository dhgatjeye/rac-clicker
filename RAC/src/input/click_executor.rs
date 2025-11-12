use crate::config::settings::Settings;
use crate::input::thread_controller::ThreadController;
use crate::logger::logger::log_error;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use winapi::um::winuser::{MK_LBUTTON, MK_RBUTTON};
use winapi::{
    shared::windef::HWND,
    um::winuser::{PostMessageA, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP},
};

thread_local! {
    static CLICK_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::seed_from_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    ));
}

const CPS_DELAYS: [u64; 25] = [
    1_000_000, // 1 CPS
    500_000,   // 2 CPS
    333_333,   // 3 CPS
    250_000,   // 4 CPS
    200_000,   // 5 CPS
    166_667,   // 6 CPS
    142_857,   // 7 CPS
    125_000,   // 8 CPS
    111_111,   // 9 CPS
    100_000,   // 10 CPS
    90_909,    // 11 CPS
    83_333,    // 12 CPS
    76_923,    // 13 CPS
    71_429,    // 14 CPS
    66_667,    // 15 CPS
    62_500,    // 16 CPS
    58_824,    // 17 CPS
    55_556,    // 18 CPS
    52_632,    // 19 CPS
    50_000,    // 20 CPS
    47_619,    // 21 CPS
    45_455,    // 22 CPS
    43_478,    // 23 CPS
    41_667,    // 24 CPS
    40_000,    // 25 CPS
];

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum MouseButton {
    Left = 0,
    Right = 1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum GameMode {
    Combo = 0,
    Default = 1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PostMode {
    Bedwars,
    Default,
}

pub struct ClickExecutor {
    pub(crate) thread_controller: ThreadController,
    left_game_mode: AtomicU8,
    right_game_mode: AtomicU8,
    left_max_cps: AtomicU8,
    right_max_cps: AtomicU8,
    active: AtomicBool,
    current_button: AtomicU8,
    last_release_time: Mutex<Option<std::time::Instant>>,
    was_button_pressed: AtomicBool,
    pub(crate) post_mode: PostMode,
}

impl ClickExecutor {
    pub fn new(thread_controller: ThreadController) -> Self {
        let settings = Settings::load().unwrap_or_else(|_| Settings::default());

        let left_mode = match settings.left_game_mode.as_str() {
            "Combo" => GameMode::Combo,
            _ => GameMode::Default,
        };

        let right_mode = match settings.right_game_mode.as_str() {
            "Combo" => GameMode::Combo,
            _ => GameMode::Default,
        };

        let post_mode = match settings.post_mode.as_str() {
            "Bedwars" => PostMode::Bedwars,
            _ => PostMode::Default,
        };

        Self {
            thread_controller,
            left_game_mode: AtomicU8::new(left_mode as u8),
            right_game_mode: AtomicU8::new(right_mode as u8),
            left_max_cps: AtomicU8::new(settings.left_max_cps),
            right_max_cps: AtomicU8::new(settings.right_max_cps),
            active: AtomicBool::new(true),
            current_button: AtomicU8::new(MouseButton::Left as u8),
            last_release_time: Mutex::new(None),
            was_button_pressed: AtomicBool::new(false),
            post_mode,
        }
    }

    #[inline]
    pub fn set_left_max_cps(&self, max_cps: u8) {
        self.left_max_cps.store(max_cps, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_right_max_cps(&self, max_cps: u8) {
        self.right_max_cps.store(max_cps, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_max_cps(&self, max_cps: u8) {
        let button = unsafe {
            std::mem::transmute::<u8, MouseButton>(self.current_button.load(Ordering::Relaxed))
        };
        match button {
            MouseButton::Left => self.set_left_max_cps(max_cps),
            MouseButton::Right => self.set_right_max_cps(max_cps),
        }
    }

    #[inline]
    pub fn set_left_game_mode(&self, mode: GameMode) {
        self.left_game_mode.store(mode as u8, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_right_game_mode(&self, mode: GameMode) {
        self.right_game_mode.store(mode as u8, Ordering::Relaxed);
    }

    #[inline]
    pub fn set_game_mode(&self, mode: GameMode) {
        let button = unsafe {
            std::mem::transmute::<u8, MouseButton>(self.current_button.load(Ordering::Relaxed))
        };
        match button {
            MouseButton::Left => self.set_left_game_mode(mode),
            MouseButton::Right => self.set_right_game_mode(mode),
        }
    }

    #[inline]
    pub fn get_game_mode(&self) -> GameMode {
        let button = unsafe {
            std::mem::transmute::<u8, MouseButton>(self.current_button.load(Ordering::Relaxed))
        };
        match button {
            MouseButton::Left => unsafe {
                std::mem::transmute::<u8, GameMode>(self.left_game_mode.load(Ordering::Relaxed))
            },
            MouseButton::Right => unsafe {
                std::mem::transmute::<u8, GameMode>(self.right_game_mode.load(Ordering::Relaxed))
            },
        }
    }

    #[inline]
    pub fn set_mouse_button(&self, button: MouseButton) {
        self.current_button.store(button as u8, Ordering::Relaxed);
    }

    #[inline(always)]
    pub fn execute_click(&self, hwnd: HWND) -> bool {
        if hwnd.is_null() || !self.active.load(Ordering::Relaxed) {
            return false;
        }

        let button = unsafe {
            std::mem::transmute::<u8, MouseButton>(self.current_button.load(Ordering::Relaxed))
        };

        let (down_msg, up_msg, flags, max_cps, game_mode) = match button {
            MouseButton::Left => (
                WM_LBUTTONDOWN,
                WM_LBUTTONUP,
                MK_LBUTTON,
                self.left_max_cps.load(Ordering::Relaxed),
                unsafe {
                    std::mem::transmute::<u8, GameMode>(self.left_game_mode.load(Ordering::Relaxed))
                },
            ),
            MouseButton::Right => (
                WM_RBUTTONDOWN,
                WM_RBUTTONUP,
                MK_RBUTTON,
                self.right_max_cps.load(Ordering::Relaxed),
                unsafe {
                    std::mem::transmute::<u8, GameMode>(self.right_game_mode.load(Ordering::Relaxed))
                },
            ),
        };
        
        let base_cps_delay = if max_cps > 0 && max_cps <= 25 {
            CPS_DELAYS[(max_cps - 1) as usize]
        } else if max_cps > 25 {
            1_000_000 / max_cps as u64
        } else {
            1_000_000 
        };

        unsafe {
            if let Err(_) = std::panic::catch_unwind(|| {
                CLICK_RNG.with(|rng| {
                    let mut rng = rng.borrow_mut();

                    if !self.was_button_pressed.load(Ordering::Relaxed) {
                        self.thread_controller.smart_sleep(Duration::from_micros(base_cps_delay));
                        self.was_button_pressed.store(true, Ordering::Release);
                    }

                    match self.post_mode {
                        PostMode::Bedwars => {
                            PostMessageA(hwnd, down_msg, flags, 0);

                            self.thread_controller.smart_sleep(Duration::from_nanos(1));

                            PostMessageA(hwnd, up_msg, 0, 0);

                            let mut adjusted_delay = base_cps_delay.saturating_sub(1);

                            if game_mode == GameMode::Combo {
                                let jitter = rng.random_range(-10..=10);
                                adjusted_delay = adjusted_delay.saturating_add_signed(jitter);
                            }

                            self.thread_controller.smart_sleep(Duration::from_micros(adjusted_delay));
                        }
                        PostMode::Default => {
                            PostMessageA(hwnd, down_msg, flags, 0);

                            let down_time = rng.random_range(1..5);
                            self.thread_controller.smart_sleep(Duration::from_micros(down_time));

                            PostMessageA(hwnd, up_msg, 0, 0);

                            let mut adjusted_delay = base_cps_delay.saturating_sub(down_time);
                            if game_mode == GameMode::Combo {
                                let jitter = rng.random_range(-500..=500);
                                adjusted_delay = adjusted_delay.saturating_add_signed(jitter);
                            }

                            self.thread_controller.smart_sleep(Duration::from_micros(adjusted_delay));
                        }
                    }
                });
            }) {
                log_error("Failed to execute mouse event", "ClickExecutor::execute_click");
                return false;
            }
        }

        true
    }

    #[inline]
    pub fn handle_button_release(&self) {
        if let Ok(mut last_release) = self.last_release_time.lock() {
            *last_release = Some(std::time::Instant::now());
        }

        self.was_button_pressed.store(false, Ordering::Release);
    }

    #[inline]
    pub fn get_current_max_cps(&self) -> u8 {
        let button = unsafe {
            std::mem::transmute::<u8, MouseButton>(self.current_button.load(Ordering::Relaxed))
        };
        match button {
            MouseButton::Left => self.left_max_cps.load(Ordering::Relaxed),
            MouseButton::Right => self.right_max_cps.load(Ordering::Relaxed),
        }
    }

    #[inline]
    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn force_right_cps(&self, cps: u8) {
        self.right_max_cps.store(cps, Ordering::Relaxed);
    }
}