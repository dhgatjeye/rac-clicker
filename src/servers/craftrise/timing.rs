use crate::servers::ServerTiming;

pub struct CraftriseTiming;

impl ServerTiming for CraftriseTiming {
    fn hold_duration_us(&self) -> (u64, i64) {
        (125, 4)
    }

    fn right_hold_duration_us(&self) -> (u64, i64) {
        (75, 3)
    }

    fn use_combo_pattern(&self) -> bool {
        true
    }

    fn combo_interval(&self) -> u8 {
        3
    }

    fn combo_pause_us(&self) -> (u64, u64) {
        (2000, 4000)
    }

    fn first_hit_boost(&self) -> u8 {
        20
    }

    fn release_penalty_ms(&self) -> u64 {
        300
    }

    fn left_cps_limits(&self) -> (u8, u8, u8) {
        (15, 16, 17)
    }

    fn right_cps_limits(&self) -> (u8, u8, u8) {
        (18, 20, 21)
    }
}
