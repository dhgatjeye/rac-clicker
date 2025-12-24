use crate::clicker::Xoshiro256;
use crate::core::ClickPattern;
use crate::core::{MouseButton, ServerType};
use crate::servers::{ServerTiming, get_server_timing};
use std::time::{Duration, Instant};

pub struct DelayCalculator {
    pattern: ClickPattern,
    button: MouseButton,
    server_timing: Box<dyn ServerTiming>,
    penalty_until: Option<Instant>,
    was_pressed: bool,
    combo_counter: u8,
    rng: Xoshiro256,
    click_history: Vec<Instant>,
    last_click_time: Option<Instant>,
}

impl DelayCalculator {
    pub fn new(
        pattern: ClickPattern,
        button: MouseButton,
        server_type: ServerType,
    ) -> crate::core::RacResult<Self> {
        let server_timing = get_server_timing(server_type)?;
        let rng = Xoshiro256::from_entropy();

        Ok(Self {
            pattern,
            button,
            server_timing,
            penalty_until: None,
            was_pressed: false,
            combo_counter: 0,
            rng,
            click_history: Vec::with_capacity(32),
            last_click_time: None,
        })
    }

    fn get_cps_limits(&self) -> (u8, u8, u8) {
        match self.button {
            MouseButton::Left => self.server_timing.left_cps_limits(),
            MouseButton::Right => self.server_timing.right_cps_limits(),
        }
    }

    fn calculate_base_cps_delay(&self, target_cps: u8) -> u64 {
        if target_cps == 0 {
            1_000_000
        } else {
            1_000_000 / target_cps as u64
        }
    }

    fn calculate_down_time(&mut self) -> u64 {
        let (base_down_time, jitter_range) = match self.button {
            MouseButton::Left => self.server_timing.hold_duration_us(),
            MouseButton::Right => self.server_timing.right_hold_duration_us(),
        };

        let down_jitter = self.rng.random_range_i64(-jitter_range, jitter_range);
        base_down_time.saturating_add_signed(down_jitter)
    }

    fn calculate_min_delay(&self, down_time: u64) -> u64 {
        let (min_cps, _max_cps, hard_limit) = self.get_cps_limits();

        match self.pattern.max_cps {
            cps if cps >= hard_limit => (1_000_000 / hard_limit as u64).saturating_sub(down_time),
            _ => {
                let target_cps = self.pattern.max_cps.max(min_cps);
                (1_000_000 / target_cps as u64).saturating_sub(down_time)
            }
        }
    }

    fn calculate_first_hit_delay(&self) -> Duration {
        let (min_cps, _max_cps, hard_limit) = self.get_cps_limits();
        let target_cps = self.pattern.max_cps;

        let boost = match self.button {
            MouseButton::Left => self.server_timing.left_first_hit_boost(),
            MouseButton::Right => self.server_timing.right_first_hit_boost(),
        };

        let base_cps_delay = self.calculate_base_cps_delay(target_cps);

        let boosted_delay = if target_cps > min_cps && target_cps < hard_limit {
            (base_cps_delay * (100 - boost as u64)) / 100
        } else {
            base_cps_delay
        };

        let down_time = match self.button {
            MouseButton::Left => self.server_timing.hold_duration_us().0,
            MouseButton::Right => self.server_timing.right_hold_duration_us().0,
        };
        let min_delay = self.calculate_min_delay(down_time);

        let hard_limit_delay = (1_000_000 / hard_limit as u64).saturating_sub(down_time);

        let final_delay = boosted_delay.max(min_delay).max(hard_limit_delay);

        Duration::from_micros(final_delay)
    }

    fn apply_combo_pause(&mut self, delay: u64) -> u64 {
        let use_combo = match self.button {
            MouseButton::Left => self.server_timing.use_left_combo_pattern(),
            MouseButton::Right => self.server_timing.use_right_combo_pattern(),
        };

        if use_combo && self.combo_counter == 0 {
            let (min_pause, max_pause) = match self.button {
                MouseButton::Left => self.server_timing.left_combo_pause_us(),
                MouseButton::Right => self.server_timing.right_combo_pause_us(),
            };

            let micro_pause = self.rng.random_range_u64(min_pause, max_pause);
            delay.saturating_add(micro_pause)
        } else {
            delay
        }
    }

    fn check_penalty(&mut self) -> Option<Duration> {
        if let Some(until) = self.penalty_until {
            let now = Instant::now();
            if now < until {
                self.combo_counter = 0;
                return Some(until - now);
            } else {
                self.penalty_until = None;
            }
        }
        None
    }

    fn clean_old_history(&mut self) {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);
        self.click_history.retain(|&t| t > one_second_ago);

        if self.click_history.len() > 64 {
            self.click_history.drain(0..16);
        }
    }

    fn enforce_cps_limit(&mut self, mut delay: u64) -> u64 {
        let now = Instant::now();
        let (_min_cps, _max_cps, hard_limit) = self.get_cps_limits();
        let effective_cps = if self.pattern.max_cps >= hard_limit {
            hard_limit
        } else {
            self.pattern.max_cps
        };

        let target_period_us = 1_000_000 / effective_cps as u64;

        if let Some(last) = self.last_click_time {
            let elapsed = now.duration_since(last);
            let elapsed_us = elapsed.as_micros() as u64;

            if elapsed_us < target_period_us {
                let needed = target_period_us - elapsed_us;
                delay = delay.max(needed);
            }
        }

        if self.click_history.len() >= effective_cps as usize {
            let oldest_in_window =
                self.click_history[self.click_history.len() - effective_cps as usize];
            let window_duration = now.duration_since(oldest_in_window);
            let window_us = window_duration.as_micros() as u64;

            if window_us < 1_000_000 {
                let needed = (1_000_000 - window_us) / 2;
                delay = delay.max(needed);
            }
        }

        delay
    }

    pub fn record_click(&mut self) {
        let now = Instant::now();
        self.last_click_time = Some(now);
        self.click_history.push(now);
        self.clean_old_history();
    }

    pub fn next_delay(&mut self) -> Duration {
        if let Some(remaining) = self.check_penalty() {
            return remaining;
        }

        if !self.was_pressed {
            self.was_pressed = true;
            self.combo_counter = 0;
            return self.calculate_first_hit_delay();
        }

        if match self.button {
            MouseButton::Left => self.server_timing.use_left_combo_pattern(),
            MouseButton::Right => self.server_timing.use_right_combo_pattern(),
        } {
            let interval = match self.button {
                MouseButton::Left => self.server_timing.left_combo_interval(),
                MouseButton::Right => self.server_timing.right_combo_interval(),
            };
            self.combo_counter = (self.combo_counter + 1) % interval;
        }

        let base_cps_delay = self.calculate_base_cps_delay(self.pattern.max_cps);
        let down_time = self.calculate_down_time();

        let mut adjusted_delay = base_cps_delay.saturating_sub(down_time);
        adjusted_delay = self.apply_combo_pause(adjusted_delay);

        let min_delay = self.calculate_min_delay(down_time);
        if adjusted_delay < min_delay {
            adjusted_delay = min_delay;
        }

        adjusted_delay = self.enforce_cps_limit(adjusted_delay);

        Duration::from_micros(adjusted_delay)
    }

    pub fn hold_duration(&mut self) -> Duration {
        let (base_hold, jitter_range) = match self.button {
            MouseButton::Left => self.server_timing.hold_duration_us(),
            MouseButton::Right => self.server_timing.right_hold_duration_us(),
        };

        let jitter = self.rng.random_range_i64(-jitter_range, jitter_range);
        let hold_time = base_hold.saturating_add_signed(jitter);

        Duration::from_micros(hold_time)
    }

    pub fn reset_on_release(&mut self) {
        self.was_pressed = false;
        self.combo_counter = 0;
        self.last_click_time = None;
        self.click_history.clear();

        let penalty_ms = self.server_timing.release_penalty_ms();
        if penalty_ms > 0 {
            self.penalty_until = Some(Instant::now() + Duration::from_millis(penalty_ms));
        }
    }
}
