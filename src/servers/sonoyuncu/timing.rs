use crate::servers::ServerTiming;

pub struct SonoyuncuTiming;

impl ServerTiming for SonoyuncuTiming {
    fn hold_duration_us(&self) -> (u64, i64) {
        (125, 4)
    }

    fn right_hold_duration_us(&self) -> (u64, i64) {
        (75, 3)
    }

    fn use_left_combo_pattern(&self) -> bool {
        true
    }

    fn use_right_combo_pattern(&self) -> bool {
        true
    }

    fn left_combo_interval(&self) -> u8 {
        3
    }

    fn right_combo_interval(&self) -> u8 {
        3
    }

    fn left_combo_pause_us(&self) -> (u64, u64) {
        (2000, 4000)
    }

    fn right_combo_pause_us(&self) -> (u64, u64) {
        (2000, 4000)
    }

    fn left_first_hit_boost(&self) -> u8 {
        20
    }

    fn right_first_hit_boost(&self) -> u8 {
        20
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
