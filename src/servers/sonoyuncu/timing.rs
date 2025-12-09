use crate::servers::ServerTiming;

pub struct SonoyuncuTiming;

impl ServerTiming for SonoyuncuTiming {
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
        15
    }

    fn release_penalty_ms(&self) -> u64 {
        200
    }

    fn left_cps_limits(&self) -> (u8, u8, u8) {
        (14, 16, 16)
    }

    fn right_cps_limits(&self) -> (u8, u8, u8) {
        (18, 20, 20)
    }
}
