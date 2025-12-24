use std::thread;
use std::time::{Duration, Instant};

pub struct PrecisionSleep;

impl PrecisionSleep {
    #[inline]
    pub fn sleep(duration: Duration) {
        let nanos = duration.as_nanos();

        if nanos == 0 {
            return;
        }

        if nanos < 1_000 {
            Self::pure_spin(duration);
            return;
        }

        let micros = (nanos / 1_000) as u64;

        if micros < 20 {
            Self::calibrated_spin(duration);
            return;
        }

        if micros < 100 {
            Self::micro_sleep(duration);
            return;
        }

        if micros < 500 {
            Self::balanced_hybrid(duration);
            return;
        }

        Self::efficient_hybrid(duration);
    }

    fn pure_spin(duration: Duration) {
        let deadline = Instant::now() + duration;

        while Instant::now() < deadline {
            std::hint::spin_loop();
        }
    }

    fn calibrated_spin(duration: Duration) {
        let deadline = Instant::now() + duration;
        let mut check_counter = 0u32;

        loop {
            for _ in 0..8 {
                std::hint::spin_loop();
            }

            check_counter += 1;

            if check_counter & 0xF == 0 && Instant::now() >= deadline {
                break;
            }
        }
    }

    fn micro_sleep(duration: Duration) {
        let micros = duration.as_micros() as u64;

        let sleep_portion = micros / 5;
        if sleep_portion >= 10 {
            thread::sleep(Duration::from_micros(sleep_portion));
        }

        let deadline = Instant::now() + Duration::from_micros(micros - sleep_portion);
        let mut check_counter = 0u32;

        loop {
            for _ in 0..4 {
                std::hint::spin_loop();
            }

            check_counter += 1;

            if check_counter & 0x7 == 0 && Instant::now() >= deadline {
                break;
            }
        }
    }

    fn balanced_hybrid(duration: Duration) {
        let micros = duration.as_micros() as u64;

        let sleep_micros = (micros * 40) / 100;
        thread::sleep(Duration::from_micros(sleep_micros));

        let deadline = Instant::now() + Duration::from_micros(micros - sleep_micros);
        let mut check_counter = 0u32;

        loop {
            for _ in 0..8 {
                std::hint::spin_loop();
            }

            check_counter += 1;

            if check_counter & 0xF == 0 && Instant::now() >= deadline {
                break;
            }
        }
    }

    fn efficient_hybrid(duration: Duration) {
        let micros = duration.as_micros() as u64;

        let sleep_micros = (micros * 80) / 100;
        thread::sleep(Duration::from_micros(sleep_micros));

        let deadline = Instant::now() + Duration::from_micros(micros - sleep_micros);
        let mut check_counter = 0u32;

        loop {
            for _ in 0..16 {
                std::hint::spin_loop();
            }

            check_counter += 1;

            if check_counter & 0x1F == 0 {
                if Instant::now() >= deadline {
                    break;
                }

                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.as_micros() > 50 {
                    thread::yield_now();
                }
            }
        }
    }
}
