use std::thread;
use std::time::{Duration, Instant};

const TAU_NS: f64 = 15_000_000.0;

const THRESHOLD_MICRO: u64 = 80;
const THRESHOLD_BALANCED: u64 = 400;

const MIN_SPINS: u32 = 4;
const MAX_SPINS: u32 = 64;

#[derive(Debug, Clone)]
pub struct StochasticRefiner {
    survival_prob: f64,
    observations: u32,
    uncertainty_ns: f64,
    initial_duration_ns: f64,
    tau_ns: f64,
}

impl StochasticRefiner {
    #[inline]
    pub fn new(duration_ns: u64) -> Self {
        Self {
            survival_prob: 1.0,
            observations: 0,
            uncertainty_ns: duration_ns as f64,
            initial_duration_ns: duration_ns as f64,
            tau_ns: TAU_NS,
        }
    }

    #[inline]
    pub fn observe(&mut self, remaining_ns: u64) {
        self.observations += 1;

        let decay_factor = (-1.0 / (self.observations as f64).sqrt()).exp();
        self.uncertainty_ns *= decay_factor;

        let delta_t = self.uncertainty_ns;
        let exponent = -(self.observations as f64) * delta_t / self.tau_ns;
        self.survival_prob = exponent.exp().clamp(0.0, 1.0);

        self.uncertainty_ns = self.uncertainty_ns.min(remaining_ns as f64);
    }

    #[inline]
    pub fn spin_count(&self) -> u32 {
        let normalized = (self.uncertainty_ns / self.initial_duration_ns).clamp(0.0, 1.0);

        let range = (MAX_SPINS - MIN_SPINS) as f64;
        let spins = MIN_SPINS as f64 + (1.0 - normalized) * range;

        spins as u32
    }

    #[inline]
    pub fn survival_probability(&self) -> f64 {
        self.survival_prob
    }

    #[inline]
    pub fn uncertainty(&self) -> f64 {
        self.uncertainty_ns
    }
}

pub struct PrecisionSleep;

impl PrecisionSleep {
    pub fn sleep(duration: Duration) {
        let nanos = duration.as_nanos();

        if nanos == 0 {
            return;
        }

        let micros = (nanos / 1_000) as u64;

        if micros < THRESHOLD_MICRO {
            Self::micro_sleep(duration);
            return;
        }

        if micros < THRESHOLD_BALANCED {
            Self::balanced_hybrid(duration);
            return;
        }

        Self::efficient_hybrid(duration);
    }

    #[inline]
    fn micro_sleep(duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        let micros = nanos / 1000;

        let sleep_portion = micros / 5;
        if sleep_portion >= 10 {
            thread::sleep(Duration::from_micros(sleep_portion));
        }

        let deadline = Instant::now() + Duration::from_nanos(nanos - sleep_portion * 1000);
        let mut refiner = StochasticRefiner::new(nanos - sleep_portion * 1000);

        Self::refinement_loop(deadline, &mut refiner);
    }

    #[inline]
    fn balanced_hybrid(duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        let micros = nanos / 1000;

        let sleep_micros = (micros * 40) / 100;
        thread::sleep(Duration::from_micros(sleep_micros));

        let remaining_nanos = nanos - sleep_micros * 1000;
        let deadline = Instant::now() + Duration::from_nanos(remaining_nanos);
        let mut refiner = StochasticRefiner::new(remaining_nanos);

        Self::refinement_loop(deadline, &mut refiner);
    }

    #[inline]
    fn efficient_hybrid(duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        let micros = nanos / 1000;

        let sleep_micros = (micros * 80) / 100;
        thread::sleep(Duration::from_micros(sleep_micros));

        let remaining_nanos = nanos - sleep_micros * 1000;
        let deadline = Instant::now() + Duration::from_nanos(remaining_nanos);
        let mut refiner = StochasticRefiner::new(remaining_nanos);

        Self::refinement_loop(deadline, &mut refiner);
    }

    #[inline]
    fn refinement_loop(deadline: Instant, refiner: &mut StochasticRefiner) {
        loop {
            let spins = refiner.spin_count();

            for _ in 0..spins {
                std::hint::spin_loop();
            }

            let now = Instant::now();

            if now >= deadline {
                break;
            }

            let remaining = deadline.saturating_duration_since(now);
            refiner.observe(remaining.as_nanos() as u64);

            if refiner.uncertainty() < 1000.0 {
                while Instant::now() < deadline {
                    std::hint::spin_loop();
                }
                break;
            }

            if remaining.as_micros() > 50 && refiner.survival_probability() > 0.9 {
                thread::yield_now();
            }
        }
    }
}
