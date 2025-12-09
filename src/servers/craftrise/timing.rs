use crate::servers::ServerTiming;

pub struct CraftriseTiming;

impl ServerTiming for CraftriseTiming {
    /// Hold duration: 60µs ± 8µs (52-68µs range)
    /// Optimized for maximum CPS and momentum
    fn hold_duration_us(&self) -> (u64, i64) {
        (55, 8) // base: 60µs (was 85µs), jitter: ±8µs
    }

    /// Combo pattern enabled for better PvP performance
    fn use_combo_pattern(&self) -> bool {
        true
    }

    /// Combo every 4 hits (3 normal + 1 with pause)
    fn combo_interval(&self) -> u8 {
        4
    }

    /// Combo pause: DISABLED for CraftRise
    /// No pause = sustained pressure, no momentum loss
    /// Prevents opponent counter-hitting during pause windows
    fn combo_pause_us(&self) -> (u64, u64) {
        (0, 0) // No pause (was 5-8ms)
    }

    /// First hit 7% faster -
    fn first_hit_boost(&self) -> u8 {
        10 // 7% speed boost
    }

    /// 170ms penalty after button release
    fn release_penalty_ms(&self) -> u64 {
        170
    }

    /// Left click: min=13, max=16, hard_limit=16
    /// PvP optimized with combo pattern
    fn left_cps_limits(&self) -> (u8, u8, u8) {
        (13, 16, 16) // (min, max, hard_limit)
    }

    /// Right click: min=15, max=20, hard_limit=20
    /// Faster for blockhitting and building
    fn right_cps_limits(&self) -> (u8, u8, u8) {
        (15, 20, 20) // (min, max, hard_limit)
    }
}
