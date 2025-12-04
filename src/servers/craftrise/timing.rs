use crate::servers::ServerTiming;

pub struct CraftriseTiming;

impl ServerTiming for CraftriseTiming {
    /// Hold duration: 85µs ± 10µs (75-95µs range)
    /// Optimal for Craftrise hit detection
    fn hold_duration_us(&self) -> (u64, i64) {
        (85, 10)  // base: 85µs, jitter: ±10µs
    }

    /// Combo pattern enabled for better PvP performance
    fn use_combo_pattern(&self) -> bool {
        true
    }
    
    /// Combo every 4 hits (3 normal + 1 with pause)
    fn combo_interval(&self) -> u8 {
        4
    }
    
    /// Combo pause: 5-8ms micro-pause
    /// Breaks opponent's timing, improves hit registration
    fn combo_pause_us(&self) -> (u64, u64) {
        (5000, 8000)  // 5-8ms in microseconds
    }
    
    /// First hit 10% faster for engagement advantage
    fn first_hit_boost(&self) -> u8 {
        10  // 10% speed boost
    }
    
    /// 170ms penalty after button release
    fn release_penalty_ms(&self) -> u64 {
        170
    }
    
    /// Left click: min=13, max=16, hard_limit=16
    /// PvP optimized with combo pattern
    fn left_cps_limits(&self) -> (u8, u8, u8) {
        (13, 16, 16)  // (min, max, hard_limit)
    }
    
    /// Right click: min=15, max=20, hard_limit=20
    /// Faster for blockhitting and building
    fn right_cps_limits(&self) -> (u8, u8, u8) {
        (15, 20, 20)  // (min, max, hard_limit)
    }
}
