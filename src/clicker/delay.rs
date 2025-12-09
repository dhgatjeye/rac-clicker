use crate::core::ClickPattern;
use crate::core::{MouseButton, ServerType};
use crate::servers::{ServerTiming, get_server_timing};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::time::{Duration, Instant};

pub struct DelayCalculator {
    pattern: ClickPattern,
    button: MouseButton,
    server_timing: Box<dyn ServerTiming>,
    last_release_time: Option<Instant>,
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
            last_release_time: None,
            was_pressed: false,
            combo_counter: 0,
            rng,
        })
    }

    pub fn next_delay(&mut self) -> Duration {
        let penalty_ms = self.server_timing.release_penalty_ms();

        if let Some(release_time) = self.last_release_time {
            let elapsed = release_time.elapsed();
            if elapsed < Duration::from_millis(penalty_ms) {
                let remaining = Duration::from_millis(penalty_ms) - elapsed;
                self.combo_counter = 0;
                return remaining;
            } else {
                self.last_release_time = None;
            }
        }

        if !self.was_pressed {
            self.was_pressed = true;
            self.combo_counter = 0;

            let boost = self.server_timing.first_hit_boost();
            let base_cps_delay = if self.pattern.max_cps == 0 {
                1_000_000
            } else {
                1_000_000 / self.pattern.max_cps as u64
            };

            let boosted_delay = (base_cps_delay * (100 - boost as u64)) / 100;
            return Duration::from_micros(boosted_delay);
        }

        if self.server_timing.use_combo_pattern() {
            let interval = self.server_timing.combo_interval();
            self.combo_counter = (self.combo_counter + 1) % interval;
        }

        let base_cps_delay = if self.pattern.max_cps == 0 {
            1_000_000
        } else {
            1_000_000 / self.pattern.max_cps as u64
        };

        let (base_down_time, jitter_range) = self.server_timing.hold_duration_us();
        let down_jitter = self.rng.random_range(-jitter_range..=jitter_range);
        let down_time = base_down_time.saturating_add_signed(down_jitter);

        let mut adjusted_delay = base_cps_delay.saturating_sub(down_time);

        if self.server_timing.use_combo_pattern() && self.combo_counter == 0 {
            let (min_pause, max_pause) = self.server_timing.combo_pause_us();
            let micro_pause = self.rng.random_range(min_pause..=max_pause);
            adjusted_delay = adjusted_delay.saturating_add(micro_pause);
        }

        let (min_cps, _max_cps, hard_limit) = match self.button {
            MouseButton::Left => self.server_timing.left_cps_limits(),
            MouseButton::Right => self.server_timing.right_cps_limits(),
        };

        let min_delay = match self.pattern.max_cps {
            cps if cps >= hard_limit => (1_000_000 / hard_limit as u64).saturating_sub(down_time),
            _ => {
                let target_cps = self.pattern.max_cps.max(min_cps);
                (1_000_000 / target_cps as u64).saturating_sub(down_time)
            }
        };

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
        self.last_release_time = Some(Instant::now());
    }
}
