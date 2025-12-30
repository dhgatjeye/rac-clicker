use crate::servers::ServerTiming;

pub struct CraftriseTiming;

impl ServerTiming for CraftriseTiming {
    fn hold_duration_us(&self) -> (u64, i64) {
        (43, 5)
    }

    fn right_hold_duration_us(&self) -> (u64, i64) {
        (43, 2)
    }

    fn use_left_combo_pattern(&self) -> bool {
        true
    }

    fn use_right_combo_pattern(&self) -> bool {
        false
    }

    fn left_combo_interval(&self) -> u8 {
        3
    }

    fn right_combo_interval(&self) -> u8 {
        3
    }

    fn left_combo_pause_us(&self) -> (u64, u64) {
        (800, 2400)
    }

    fn right_combo_pause_us(&self) -> (u64, u64) {
        (400, 800)
    }

    fn left_first_hit_boost(&self) -> u8 {
        15
    }

    fn right_first_hit_boost(&self) -> u8 {
        50
    }

    fn release_penalty_ms(&self) -> u64 {
        300
    }

    fn left_cps_limits(&self) -> (u8, u8, u8) {
        (14, 15, 16)
    }

    fn right_cps_limits(&self) -> (u8, u8, u8) {
        (18, 19, 20)
    }
}
