use crate::clicker::{ClickHistory, Xoshiro256};
use crate::core::ClickPattern;
use crate::core::{MouseButton, ServerType};
use crate::servers::{ServerTiming, get_server_timing};
use std::time::{Duration, Instant};

const MICROS_PER_SECOND: u64 = 1_000_000;

pub struct DelayCalculator {
    pattern: ClickPattern,
    button: MouseButton,
    server_timing: Box<dyn ServerTiming>,
    penalty_until: Option<Instant>,
    was_pressed: bool,
    combo_counter: u8,
    rng: Xoshiro256,
    click_history: ClickHistory,
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
            click_history: ClickHistory::new(),
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
            MICROS_PER_SECOND
        } else {
            MICROS_PER_SECOND / target_cps as u64
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
            cps if cps >= hard_limit => {
                (MICROS_PER_SECOND / hard_limit as u64).saturating_sub(down_time)
            }
            _ => {
                let target_cps = self.pattern.max_cps.max(min_cps);
                (MICROS_PER_SECOND / target_cps as u64).saturating_sub(down_time)
            }
        }
    }

    fn calculate_first_hit_delay(&self) -> Duration {
        let (_, _max_cps, hard_limit) = self.get_cps_limits();
        let target_cps = self.pattern.max_cps;

        let boost = match self.button {
            MouseButton::Left => self.server_timing.left_first_hit_boost(),
            MouseButton::Right => self.server_timing.right_first_hit_boost(),
        };

        let effective_cps = target_cps.max(hard_limit);
        let base_cps_delay = self.calculate_base_cps_delay(effective_cps);

        let boosted_delay = if target_cps < hard_limit {
            (base_cps_delay * (100 - boost as u64)) / 100
        } else {
            base_cps_delay
        };

        let down_time = match self.button {
            MouseButton::Left => self.server_timing.hold_duration_us().0,
            MouseButton::Right => self.server_timing.right_hold_duration_us().0,
        };

        let min_delay = self.calculate_min_delay(down_time);
        let hard_limit_delay = (MICROS_PER_SECOND / hard_limit as u64).saturating_sub(down_time);

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

    fn enforce_cps_limit(&mut self, mut delay: u64) -> u64 {
        let now = Instant::now();
        let (_min_cps, _max_cps, hard_limit) = self.get_cps_limits();
        let effective_cps = if self.pattern.max_cps >= hard_limit {
            hard_limit
        } else {
            self.pattern.max_cps
        };

        let target_period_us = MICROS_PER_SECOND / effective_cps as u64;

        if let Some(last_ts) = self.click_history.get_last_timestamp() {
            let current_us = now.duration_since(self.click_history.epoch).as_micros() as u64;
            let elapsed_us = current_us.saturating_sub(last_ts);

            if elapsed_us < target_period_us {
                let needed = target_period_us - elapsed_us;
                delay = delay.max(needed);
            }
        }

        if self.click_history.count >= effective_cps
            && let Some(oldest_ts) = self.click_history.get_nth_from_end(effective_cps - 1)
        {
            let current_us = now.duration_since(self.click_history.epoch).as_micros() as u64;
            let window_us = current_us.saturating_sub(oldest_ts);

            if window_us < MICROS_PER_SECOND {
                let needed = (MICROS_PER_SECOND - window_us) / 2;
                delay = delay.max(needed);
            }
        }

        delay
    }

    #[inline]
    pub fn record_click(&mut self) {
        let now = Instant::now();
        self.click_history.push(now);
    }

    pub fn next_delay(&mut self) -> Duration {
        if let Some(remaining) = self.check_penalty() {
            return remaining;
        }

        let now = Instant::now();
        let time_since_last_click = if let Some(last_ts) = self.click_history.get_last_timestamp() {
            let current_us = now.duration_since(self.click_history.epoch).as_micros() as u64;
            current_us.saturating_sub(last_ts)
        } else {
            u64::MAX
        };

        if time_since_last_click > 300_000 {
            self.was_pressed = false;
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

        let jitter_base = self.rng.random_range_i64(-60, 60);
        let jitter_extra = if self.rng.random_range_u64(0, 100) < 30 {
            self.rng.random_range_i64(-60, 60)
        } else {
            0
        };

        let jitter = jitter_base + jitter_extra;
        adjusted_delay = adjusted_delay.saturating_add_signed(jitter);

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
        self.click_history.clear();

        let penalty_ms = self.server_timing.release_penalty_ms();
        if penalty_ms > 0 {
            self.penalty_until = Some(Instant::now() + Duration::from_millis(penalty_ms));
        }
    }
}
