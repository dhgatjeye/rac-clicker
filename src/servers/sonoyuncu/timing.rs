use crate::servers::ServerTiming;

pub struct SonoyuncuTiming;

impl ServerTiming for SonoyuncuTiming {
    /// Hold duration: 70µs ± 8µs (62-78µs range)
    /// Slightly shorter for Sonoyuncu's faster hit detection
    fn hold_duration_us(&self) -> (u64, i64) {
        (70, 8) // base: 70µs, jitter: ±8µs
    }

    /// Combo pattern disabled - Sonoyuncu prefers consistent CPS
    fn use_combo_pattern(&self) -> bool {
        false
    }

    /// No combo interval (pattern disabled)
    fn combo_interval(&self) -> u8 {
        0
    }

    /// No combo pause (pattern disabled)
    fn combo_pause_us(&self) -> (u64, u64) {
        (0, 0)
    }

    /// First hit 5% faster (less aggressive than Craftrise)
    fn first_hit_boost(&self) -> u8 {
        5 // 5% speed boost
    }

    /// 170ms penalty after button release
    fn release_penalty_ms(&self) -> u64 {
        170
    }

    /// Left click: min=12, max=15, hard_limit=15
    /// More conservative for Sonoyuncu's anti-cheat
    fn left_cps_limits(&self) -> (u8, u8, u8) {
        (12, 15, 15) // (min, max, hard_limit)
    }

    /// Right click: min=15, max=18, hard_limit=18
    /// Slightly lower than Craftrise
    fn right_cps_limits(&self) -> (u8, u8, u8) {
        (15, 18, 18) // (min, max, hard_limit)
    }
}
