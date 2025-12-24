//! PCG32 Random Number Generator
//!
//! Implementation of the Permuted Congruential Generator (PCG) algorithm
//!
//! # Algorithm
//! Algorithm by Melissa E. O'Neill
//!
//! # References
//! - <https://www.pcg-random.org>
//! - <https://www.cs.hmc.edu/~oneill/>
//! - <https://en.wikipedia.org/wiki/Permuted_congruential_generator>

pub struct PcgRng {
    state: u64,
    increment: u64,
}

impl PcgRng {
    fn new(seed: u64) -> Self {
        let mut rng = Self {
            state: 0,
            increment: (seed << 1) | 1,
        };
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn from_entropy() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        Self::new(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let old_state = self.state;
        self.state = old_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(self.increment);

        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;

        xorshifted.rotate_right(rot)
    }

    pub fn random_range_i64(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }
        let range = (max - min) as u64 + 1;
        let rand = self.next_u32() as u64;
        min + ((rand * range) >> 32) as i64
    }

    pub fn random_range_u64(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }
        let range = max - min + 1;
        let rand = self.next_u32() as u64;
        min + ((rand * range) >> 32)
    }
}
