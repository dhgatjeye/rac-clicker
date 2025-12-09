use std::thread;
use std::time::{Duration, Instant};

pub struct PrecisionSleep;

impl PrecisionSleep {
    #[inline]
    pub fn sleep(duration: Duration) {
        let micros = duration.as_micros();

        if micros < 1 {
            return;
        }

        if micros < 30 {
            Self::spin_wait(duration);
            return;
        }

        if micros < 150 {
            Self::hybrid_sleep_short(duration);
            return;
        }

        if micros < 500 {
            Self::hybrid_sleep_medium(duration);
            return;
        }

        Self::hybrid_sleep_long(duration);
    }

    #[inline(always)]
    fn spin_wait(duration: Duration) {
        let deadline = Instant::now() + duration;

        let mut iterations = 0u32;

        loop {
            std::hint::spin_loop();

            iterations = iterations.wrapping_add(1);

            if iterations & 0x7 == 0 && Instant::now() >= deadline {
                break;
            }
        }
    }

    #[inline]
    fn hybrid_sleep_short(duration: Duration) {
        let micros = duration.as_micros() as u64;

        let sleep_micros = (micros * 30) / 100;

        if sleep_micros >= 15 {
            thread::sleep(Duration::from_micros(sleep_micros));
        }

        let deadline = Instant::now() + Duration::from_micros(micros - sleep_micros);
        let mut iterations = 0u32;

        loop {
            std::hint::spin_loop();
            iterations = iterations.wrapping_add(1);

            if iterations & 0x7 == 0 && Instant::now() >= deadline {
                break;
            }
        }
    }

    #[inline]
    fn hybrid_sleep_medium(duration: Duration) {
        let micros = duration.as_micros() as u64;

        let sleep_micros = micros / 2;

        thread::sleep(Duration::from_micros(sleep_micros));

        let deadline = Instant::now() + Duration::from_micros(micros - sleep_micros);
        let mut iterations = 0u32;

        loop {
            std::hint::spin_loop();
            iterations = iterations.wrapping_add(1);

            if iterations & 0xF == 0 && Instant::now() >= deadline {
                break;
            }
        }
    }

    #[inline]
    fn hybrid_sleep_long(duration: Duration) {
        let micros = duration.as_micros() as u64;

        let sleep_micros = (micros * 85) / 100;

        thread::sleep(Duration::from_micros(sleep_micros));

        let deadline = Instant::now() + Duration::from_micros(micros - sleep_micros);
        let mut iterations = 0u32;

        loop {
            std::hint::spin_loop();
            iterations = iterations.wrapping_add(1);

            if iterations & 0x1F == 0 && Instant::now() >= deadline {
                break;
            }
        }
    }
}
