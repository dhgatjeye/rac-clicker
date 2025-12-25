use std::time::Instant;

pub struct ClickHistory {
    timestamps: [u64; 64],
    head: u8,
    pub count: u8,
    pub epoch: Instant,
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
            timestamps: [0u64; 64],
            head: 0,
            count: 0,
            epoch: Instant::now(),
        }
    }

    #[inline]
    pub fn push(&mut self, now: Instant) {
        let elapsed_us = now.duration_since(self.epoch).as_micros() as u64;

        unsafe {
            *self.timestamps.get_unchecked_mut(self.head as usize) = elapsed_us;
        }

        self.head = (self.head + 1) & 63;
        self.count = self.count.saturating_add(1).min(64);
    }

    #[inline]
    pub fn get_nth_from_end(&self, n: u8) -> Option<u64> {
        if n >= self.count {
            return None;
        }

        let idx = self.head.wrapping_sub(n + 1) & 63;
        Some(unsafe { *self.timestamps.get_unchecked(idx as usize) })
    }

    #[inline]
    pub fn clear(&mut self) {
        self.count = 0;
        self.head = 0;
    }

    #[inline]
    pub fn get_last_timestamp(&self) -> Option<u64> {
        if self.count == 0 {
            return None;
        }

        let last_idx = self.head.wrapping_sub(1) & 63;
        Some(unsafe { *self.timestamps.get_unchecked(last_idx as usize) })
    }
}
