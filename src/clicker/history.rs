use std::time::Instant;

const HISTORY_CAPACITY: usize = 128;
const HISTORY_MASK: u8 = (HISTORY_CAPACITY - 1) as u8;

const _: () = assert!(HISTORY_CAPACITY.is_power_of_two());
const _: () = assert!(HISTORY_CAPACITY <= 256);

#[repr(C, align(64))]
pub struct ClickHistory {
    deltas_us: [u32; HISTORY_CAPACITY],
    head: u8,
    pub count: u8,
    capacity: u8,
    _pad1: u8,
    pub epoch: Instant,
    last_absolute_us: u64,
    total_clicks: u64,
    sum_deltas: u64,
    sum_squared: u64,
    min_delta: u32,
    max_delta: u32,
    outlier_count: u32,
    last_outlier_delta: u32,
}

impl Default for ClickHistory {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ClickHistory {
    #[inline]
    pub fn new() -> Self {
        Self {
            deltas_us: [0u32; HISTORY_CAPACITY],
            head: 0,
            count: 0,
            capacity: HISTORY_CAPACITY as u8,
            _pad1: 0,
            epoch: Instant::now(),
            last_absolute_us: 0,
            total_clicks: 0,
            sum_deltas: 0,
            sum_squared: 0,
            min_delta: u32::MAX,
            max_delta: 0,
            outlier_count: 0,
            last_outlier_delta: 0,
        }
    }

    #[inline(always)]
    fn mask(&self) -> u8 {
        HISTORY_MASK
    }

    #[inline]
    pub fn push(&mut self, now: Instant) {
        let absolute_us = now.duration_since(self.epoch).as_micros() as u64;

        let delta_us = if self.total_clicks == 0 {
            0u32
        } else {
            absolute_us
                .saturating_sub(self.last_absolute_us)
                .min(u32::MAX as u64) as u32
        };

        unsafe {
            *self.deltas_us.get_unchecked_mut(self.head as usize) = delta_us;
        }

        self.head = (self.head + 1) & self.mask();
        self.count = self.count.saturating_add(1).min(self.capacity);

        self.total_clicks += 1;

        if delta_us > 0 {
            self.sum_deltas += delta_us as u64;
            self.sum_squared += (delta_us as u64).pow(2);
            self.min_delta = self.min_delta.min(delta_us);
            self.max_delta = self.max_delta.max(delta_us);

            if self.count > 10 {
                let mean = self.mean_delta_us();
                let stddev = self.stddev_delta_us();

                if delta_us as f64 > mean + 3.0 * stddev {
                    self.outlier_count += 1;
                    self.last_outlier_delta = delta_us;
                }
            }
        }

        self.last_absolute_us = absolute_us;
    }

    #[inline]
    pub fn get_nth_from_end(&self, n: u8) -> Option<u64> {
        if n >= self.count {
            return None;
        }

        let mut absolute_us = self.last_absolute_us;

        for i in 0..=n {
            let delta_idx = self.head.wrapping_sub(i + 1) & self.mask();
            let delta = unsafe { *self.deltas_us.get_unchecked(delta_idx as usize) };

            if i < n {
                absolute_us = absolute_us.saturating_sub(delta as u64);
            }
        }

        Some(absolute_us)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.count = 0;
        self.head = 0;
        self.sum_deltas = 0;
        self.sum_squared = 0;
        self.min_delta = u32::MAX;
        self.max_delta = 0;
    }

    #[inline]
    pub fn get_last_timestamp(&self) -> Option<u64> {
        if self.count == 0 {
            return None;
        }
        Some(self.last_absolute_us)
    }

    #[inline]
    pub fn mean_delta_us(&self) -> f64 {
        if self.count <= 1 || self.sum_deltas == 0 {
            return 0.0;
        }
        self.sum_deltas as f64 / (self.count as f64 - 1.0)
    }

    #[inline]
    pub fn stddev_delta_us(&self) -> f64 {
        if self.count <= 2 {
            return 0.0;
        }
        let n = (self.count - 1) as f64;
        let mean = self.mean_delta_us();
        let variance = (self.sum_squared as f64 / n) - mean.powi(2);
        variance.max(0.0).sqrt()
    }
}
