use crate::core::ClickPattern;
use crate::core::{MouseButton, ServerType};
use crate::servers::{get_server_timing, ServerTiming};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::time::{Duration, Instant};

pub struct DelayCalculator {
    pattern: ClickPattern,
    button: MouseButton,
    server_timing: Box<dyn ServerTiming>,
    penalty_until: Option<Instant>,
    was_pressed: bool,
    combo_counter: u8,
    rng: SmallRng,
}

impl DelayCalculator {
    pub fn new(
        pattern: ClickPattern,
        button: MouseButton,
        server_type: ServerType,
    ) -> crate::core::RacResult<Self> {
        let server_timing = get_server_timing(server_type)?;
        let mut thread_rng = rand::rng();
        let rng = SmallRng::from_rng(&mut thread_rng);

        Ok(Self {
            pattern,
            button,
            server_timing,
            penalty_until: None,
            was_pressed: false,
            combo_counter: 0,
            rng,
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
        let (base_down_time, jitter_range) = self.server_timing.hold_duration_us();
        let down_jitter = self.rng.random_range(-jitter_range..=jitter_range);
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
        let boost = self.server_timing.first_hit_boost();

        let base_cps_delay = self.calculate_base_cps_delay(target_cps);

        let boosted_delay = if target_cps > min_cps && target_cps < hard_limit {
            (base_cps_delay * (100 - boost as u64)) / 100
        } else {
            base_cps_delay
        };

        let down_time = self.server_timing.hold_duration_us().0;
        let min_delay = self.calculate_min_delay(down_time);
        let final_delay = boosted_delay.max(min_delay);

        Duration::from_micros(final_delay)
    }

    fn apply_combo_pause(&mut self, delay: u64) -> u64 {
        if self.server_timing.use_combo_pattern() && self.combo_counter == 0 {
            let (min_pause, max_pause) = self.server_timing.combo_pause_us();
            let micro_pause = self.rng.random_range(min_pause..=max_pause);
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

    pub fn next_delay(&mut self) -> Duration {
        if let Some(remaining) = self.check_penalty() {
            return remaining;
        }

        if !self.was_pressed {
            self.was_pressed = true;
            self.combo_counter = 0;
            return self.calculate_first_hit_delay();
        }

        if self.server_timing.use_combo_pattern() {
            let interval = self.server_timing.combo_interval();
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

        Duration::from_micros(adjusted_delay)
    }

    pub fn hold_duration(&mut self) -> Duration {
        let (base_hold, jitter_range) = match self.button {
            MouseButton::Left => self.server_timing.hold_duration_us(),
            MouseButton::Right => self.server_timing.right_hold_duration_us(),
        };

        let jitter = self.rng.random_range(-jitter_range..=jitter_range);
        let hold_time = base_hold.saturating_add_signed(jitter);

        Duration::from_micros(hold_time)
    }

    pub fn reset_on_release(&mut self) {
        self.was_pressed = false;
        self.combo_counter = 0;

        let penalty_ms = self.server_timing.release_penalty_ms();
        if penalty_ms > 0 {
            self.penalty_until = Some(Instant::now() + Duration::from_millis(penalty_ms));
        }
    }
}