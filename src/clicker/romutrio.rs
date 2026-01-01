//! RomuTrio Random Number Generator
//!
//! Implementation of the RomuTrio algorithm
//!
//! # Algorithm
//! Algorithm by Mark A. Overton
//!
//! # References
//! - <http://www.romu-random.org/>
//! - <https://arxiv.org/abs/2002.11331>
//! - Overton, M. A. (2020). "Romu: Fast Nonlinear Pseudo-Random Number Generators"

pub struct RomuTrio {
    x: u64,
    y: u64,
    z: u64,
}

impl RomuTrio {
    pub fn new(seed: u64) -> Self {
        let mut sm = SplitMix64::new(seed);
        Self {
            x: sm.next(),
            y: sm.next(),
            z: sm.next(),
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
    pub fn next_u64(&mut self) -> u64 {
        let xp = self.x;
        let yp = self.y;
        let zp = self.z;

        self.x = 15241094284759029579u64.wrapping_mul(zp);
        self.y = yp.wrapping_sub(xp).rotate_left(12);
        self.z = zp.wrapping_sub(yp).rotate_left(44);

        xp
    }

    #[inline]
    pub fn random_range_i64(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            return min;
        }

        let umin = min as u64;
        let umax = max as u64;
        let range = umax.wrapping_sub(umin).wrapping_add(1);

        if range.is_power_of_two() {
            let mask = range - 1;
            return umin.wrapping_add(self.next_u64() & mask) as i64;
        }

        let x = self.next_u64();
        let m = (x as u128) * (range as u128);
        let l = m as u64;

        if l < range {
            let t = range.wrapping_neg() % range;
            let mut x = x;
            let mut l = l;
            while l < t {
                x = self.next_u64();
                let m = (x as u128) * (range as u128);
                l = m as u64;
            }
            return umin.wrapping_add((((x as u128) * (range as u128)) >> 64) as u64) as i64;
        }

        umin.wrapping_add((m >> 64) as u64) as i64
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
