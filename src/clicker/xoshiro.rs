//! Xoshiro256++ Random Number Generator
//!
//! Implementation of the Xoshiro256++ algorithm
//!
//! # Algorithm
//! Algorithm by David Blackman and Sebastiano Vigna
//!
//! # References
//! - <https://prng.di.unimi.it/>
//! - <https://vigna.di.unimi.it/papers.php#BlVSLPNG>
//! - <https://en.wikipedia.org/wiki/Xorshift>

pub struct Xoshiro256 {
    s: [u64; 4],
}

impl Xoshiro256 {
    pub fn new(seed: u64) -> Self {
        let mut sm = SplitMix64::new(seed);

        Self {
            s: [sm.next(), sm.next(), sm.next(), sm.next()],
        }
    }

    pub fn from_entropy() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        Self::new(seed)
    }

    #[inline]
    pub fn random_range_i64(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }

        let range = (max - min) as u64 + 1;
        let rand = self.bounded_u64(range);

        min + rand as i64
    }

    #[inline]
    pub fn random_range_u64(&mut self, min: u64, max: u64) -> u64 {
        if min >= max {
            return min;
        }

        let range = max - min + 1;
        let rand = self.bounded_u64(range);

        min + rand
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        let result = self.s[0]
            .wrapping_add(self.s[3])
            .rotate_left(23)
            .wrapping_add(self.s[0]);

        let t = self.s[1] << 17;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];

        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);

        result
    }

    #[inline]
    fn bounded_u64(&mut self, range: u64) -> u64 {
        if range.is_power_of_two() {
            return self.next_u64() & (range - 1);
        }

        let mut x = self.next_u64();
        let mut m = (x as u128).wrapping_mul(range as u128);
        let mut l = m as u64;

        if l < range {
            let t = range.wrapping_neg() % range;

            while l < t {
                x = self.next_u64();
                m = (x as u128).wrapping_mul(range as u128);
                l = m as u64;
            }
        }

        (m >> 64) as u64
    }
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    #[inline]
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline]
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);

        let mut z = self.state;

        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}
