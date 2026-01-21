use crate::clicker::{ClickHistory, Xoshiro256};
use crate::core::ClickPattern;
use crate::core::{MouseButton, ServerType};
use crate::servers::{ServerTiming, get_server_timing};
use std::time::{Duration, Instant};

const MICROS_PER_SECOND: u64 = 1_000_000;

const SERVER_TICK_US: u64 = 50_000;

const PHI_BASE: f64 = 1.618_033_988_749_895;
const PHI_INV_BASE: f64 = 0.618_033_988_749_894_9;
const SQRT_2_INV: f64 = std::f64::consts::FRAC_1_SQRT_2;
const PLANCK_BASE: f64 = 137.035999084;
const EULER_GAMMA: f64 = 0.5772156649015329;

#[repr(align(64))]
pub struct DelayCalculator {
    pattern: ClickPattern,
    button: MouseButton,
    server_timing: Box<dyn ServerTiming>,
    penalty_until: Option<Instant>,
    was_pressed: bool,
    combo_counter: u8,
    rng: Xoshiro256,
    click_history: ClickHistory,
    psi_real: [f64; 8],
    psi_imag: [f64; 8],
    phase_acc: f64,
    entropy_pool: u64,
    coherence: f64,
    last_collapse: u64,
    hit_streak: u8,
    estimated_tick_offset: i64,
    phi: f64,
    phi_inv: f64,
    planck_scale: f64,
    hit_priority_window: f64,
    session_fingerprint: u64,
}

impl DelayCalculator {
    pub fn new(
        pattern: ClickPattern,
        button: MouseButton,
        server_type: ServerType,
    ) -> crate::core::RacResult<Self> {
        let server_timing = get_server_timing(server_type)?;
        let mut rng = Xoshiro256::from_entropy();

        let mut psi_real = [0.0f64; 8];
        let mut psi_imag = [0.0f64; 8];
        let norm = SQRT_2_INV / 2.0;
        for i in 0..8 {
            let theta = (i as f64) * std::f64::consts::FRAC_PI_4;
            psi_real[i] = norm * theta.cos();
            psi_imag[i] = norm * theta.sin();
        }

        let entropy_pool = rng.random_range_u64(0, u64::MAX);

        let session_fingerprint = rng.random_range_u64(0, u64::MAX);

        let perturbation = |base: f64, fp: u64, shift: u32| -> f64 {
            let noise = ((fp >> shift) & 0xFFFF) as f64 / 65535.0;
            base * (0.98 + noise * 0.04)
        };

        let phi = perturbation(PHI_BASE, session_fingerprint, 0);
        let phi_inv = perturbation(PHI_INV_BASE, session_fingerprint, 16);
        let planck_scale = perturbation(PLANCK_BASE, session_fingerprint, 32);
        let hit_priority_window = perturbation(0.45, session_fingerprint, 48);

        Ok(Self {
            pattern,
            button,
            server_timing,
            penalty_until: None,
            was_pressed: false,
            combo_counter: 0,
            rng,
            click_history: ClickHistory::new(),
            psi_real,
            psi_imag,
            phase_acc: 0.0,
            entropy_pool,
            coherence: 1.0,
            last_collapse: 0,
            hit_streak: 0,
            estimated_tick_offset: 0,
            phi,
            phi_inv,
            planck_scale,
            hit_priority_window,
            session_fingerprint,
        })
    }

    fn get_cps_limits(&self) -> (u8, u8, u8) {
        match self.button {
            MouseButton::Left => self.server_timing.left_cps_limits(),
            MouseButton::Right => self.server_timing.right_cps_limits(),
        }
    }

    #[inline(always)]
    fn evolve_wavefunction(&mut self, dt_us: u64) {
        let omega = std::f64::consts::TAU * self.planck_scale / MICROS_PER_SECOND as f64;
        let phase_delta = omega * dt_us as f64;

        let (sin_p, cos_p) = phase_delta.sin_cos();

        for i in 0..8 {
            let re = self.psi_real[i];
            let im = self.psi_imag[i];
            self.psi_real[i] = re * cos_p - im * sin_p;
            self.psi_imag[i] = re * sin_p + im * cos_p;
        }

        self.phase_acc = (self.phase_acc + phase_delta) % std::f64::consts::TAU;
        self.coherence *= (-(dt_us as f64) / (MICROS_PER_SECOND as f64 * self.phi)).exp();
        self.coherence = self.coherence.max(0.1);
    }

    #[inline(always)]
    fn collapse_wavefunction(&mut self, measurement: u64) -> f64 {
        let mut prob_density = [0.0f64; 8];
        let mut total_prob = 0.0f64;

        for (i, prob) in prob_density.iter_mut().enumerate() {
            *prob = self.psi_real[i].powi(2) + self.psi_imag[i].powi(2);
            total_prob += *prob;
        }

        if total_prob > 1e-10 {
            let inv_total = 1.0 / total_prob;
            for prob in &mut prob_density {
                *prob *= inv_total;
            }
        }

        let rand_bits = self.rng.random_range_u64(0, 1000);
        let selector = (rand_bits as f64) / 1000.0;

        let mut cumulative = 0.0f64;
        let mut collapsed_state = 0usize;
        for (i, &prob) in prob_density.iter().enumerate() {
            cumulative += prob;
            if selector <= cumulative {
                collapsed_state = i;
                break;
            }
        }

        let golden_phase = (measurement as f64 * self.phi_inv) % 1.0;
        let amplitude = self.coherence * (1.0 + golden_phase * EULER_GAMMA);

        self.entropy_pool = self.entropy_pool.rotate_left(7) ^ measurement;
        self.last_collapse = measurement;

        let state_contribution = (collapsed_state as f64 + 1.0) / 8.0;

        (amplitude * state_contribution * self.phi).clamp(0.95, 1.05)
    }

    #[inline(always)]
    fn apply_carrier_modulation(&mut self, base_delay: u64) -> u64 {
        let now_us = Instant::now()
            .duration_since(self.click_history.epoch)
            .as_micros() as u64;

        let dt = now_us.saturating_sub(self.last_collapse).max(1);
        self.evolve_wavefunction(dt);

        let collapse_factor = self.collapse_wavefunction(now_us);

        let phase_mod = (self.phase_acc.sin() * 0.04 + 1.0).clamp(0.96, 1.04);

        let entropy_bits = (self.entropy_pool & 0xFF) as f64 / 255.0;
        let entropy_mod = 0.98 + entropy_bits * 0.04;

        let coherent_jitter = collapse_factor * phase_mod * entropy_mod;

        ((base_delay as f64) * coherent_jitter) as u64
    }

    #[inline(always)]
    fn harmonic_distribution(&mut self, min_delay: u64, max_delay: u64) -> u64 {
        let range = max_delay.saturating_sub(min_delay) as f64;

        let u1 = (self.rng.random_range_u64(1, 1_000_000) as f64) / 1_000_000.0;
        let u2 = (self.rng.random_range_u64(1, 1_000_000) as f64) / 1_000_000.0;

        let box_muller = (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos();

        let sigma = range * 0.2;
        let mu = range * 0.5;
        let gaussian = (box_muller * sigma + mu).clamp(0.0, range);

        let phase_offset = (self.phase_acc * self.phi).sin() * range * 0.03;

        let result = gaussian + phase_offset;

        min_delay + result.clamp(0.0, range) as u64
    }

    #[inline(always)]
    fn fibonacci_lattice_delay(&mut self, target_delay: u64) -> u64 {
        let n = (self.click_history.count as u64).max(1);

        let fib_phase = (n as f64 * self.phi_inv).fract();

        let lattice_offset = ((fib_phase - 0.5) * SQRT_2_INV * target_delay as f64 * 0.04) as i64;

        let modulated = target_delay.saturating_add_signed(lattice_offset);

        let golden_variance =
            (target_delay as f64 * 0.02 * (fib_phase * std::f64::consts::TAU).sin()) as i64;

        modulated.saturating_add_signed(golden_variance)
    }

    #[inline(always)]
    fn tick_align_delay(&mut self, base_delay: u64) -> u64 {
        let now_us = Instant::now()
            .duration_since(self.click_history.epoch)
            .as_micros() as u64;

        let tick_position =
            (now_us.wrapping_add(self.estimated_tick_offset as u64)) % SERVER_TICK_US;

        let time_to_next_tick = SERVER_TICK_US - tick_position;

        let projected_position = (tick_position + base_delay) % SERVER_TICK_US;
        let projected_fraction = projected_position as f64 / SERVER_TICK_US as f64;

        if projected_fraction <= self.hit_priority_window {
            let early_bonus = (self.hit_streak as u64).min(8) * 800;
            base_delay.saturating_sub(early_bonus)
        } else {
            let tick_entry_offset = self
                .rng
                .random_range_u64(0, (SERVER_TICK_US as f64 * 0.15) as u64);
            time_to_next_tick + tick_entry_offset
        }
    }

    #[inline(always)]
    fn update_tick_estimation(&mut self) {
        if self.click_history.count >= 3 {
            let mean_delta = self.click_history.mean_delta_us();
            if mean_delta > 0.0 {
                let phase_error =
                    (mean_delta % SERVER_TICK_US as f64) - (SERVER_TICK_US as f64 * 0.3);
                self.estimated_tick_offset =
                    (self.estimated_tick_offset as f64 * 0.9 + phase_error * 0.1) as i64;
            }
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

        let effective_cps = target_cps.min(hard_limit);
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
        self.hit_streak = self.hit_streak.saturating_add(1).min(10);
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

        let variance_range = adjusted_delay / 8;
        let min_with_variance = adjusted_delay.saturating_sub(variance_range);
        let max_with_variance = adjusted_delay.saturating_add(variance_range);
        adjusted_delay = self.harmonic_distribution(min_with_variance, max_with_variance);

        adjusted_delay = self.apply_carrier_modulation(adjusted_delay);
        adjusted_delay = self.fibonacci_lattice_delay(adjusted_delay);

        if self.button == MouseButton::Left {
            adjusted_delay = self.tick_align_delay(adjusted_delay);
            self.update_tick_estimation();
        }

        let (_, _, hard_limit) = self.get_cps_limits();
        let effective_limit = self.pattern.max_cps.min(hard_limit);
        let hold_base = match self.button {
            MouseButton::Left => self.server_timing.hold_duration_us().0,
            MouseButton::Right => self.server_timing.right_hold_duration_us().0,
        };
        let min_cycle_time = MICROS_PER_SECOND / effective_limit as u64;
        let hard_floor = min_cycle_time.saturating_sub(hold_base);
        adjusted_delay = adjusted_delay.max(hard_floor);

        Duration::from_micros(adjusted_delay)
    }

    pub fn hold_duration(&mut self) -> Duration {
        let (base_hold, jitter_range) = match self.button {
            MouseButton::Left => self.server_timing.hold_duration_us(),
            MouseButton::Right => self.server_timing.right_hold_duration_us(),
        };

        let jitter = self.rng.random_range_i64(-jitter_range, jitter_range);
        let mut hold_time = base_hold.saturating_add_signed(jitter);

        let phase_mod = ((self.phase_acc * self.phi_inv).sin() * 0.06 + 1.0).clamp(0.94, 1.06);
        hold_time = ((hold_time as f64) * phase_mod) as u64;

        Duration::from_micros(hold_time)
    }

    pub fn reset_on_release(&mut self) {
        self.was_pressed = false;
        self.combo_counter = 0;
        self.hit_streak = 0;
        self.click_history.clear();

        let norm = SQRT_2_INV / 2.0;
        for i in 0..8 {
            let theta = (i as f64) * std::f64::consts::FRAC_PI_4;
            self.psi_real[i] = norm * theta.cos();
            self.psi_imag[i] = norm * theta.sin();
        }
        self.coherence = 1.0;
        self.phase_acc = 0.0;

        self.session_fingerprint = self.rng.random_range_u64(0, u64::MAX);
        let perturbation = |base: f64, fp: u64, shift: u32| -> f64 {
            let noise = ((fp >> shift) & 0xFFFF) as f64 / 65535.0;
            base * (0.98 + noise * 0.04)
        };
        self.phi = perturbation(PHI_BASE, self.session_fingerprint, 0);
        self.phi_inv = perturbation(PHI_INV_BASE, self.session_fingerprint, 16);
        self.planck_scale = perturbation(PLANCK_BASE, self.session_fingerprint, 32);
        self.hit_priority_window = perturbation(0.6, self.session_fingerprint, 48);

        let penalty_ms = self.server_timing.release_penalty_ms();
        if penalty_ms > 0 {
            self.penalty_until = Some(Instant::now() + Duration::from_millis(penalty_ms));
        }
    }
}
