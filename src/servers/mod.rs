pub mod craftrise;
pub mod sonoyuncu;

use crate::core::{RacError, RacResult, ServerType};

pub trait ServerTiming: Send {
    fn hold_duration_us(&self) -> (u64, i64); // (base, jitter_range) for left
    fn right_hold_duration_us(&self) -> (u64, i64); // (base, jitter_range) for right

    fn use_left_combo_pattern(&self) -> bool;
    fn use_right_combo_pattern(&self) -> bool;

    fn left_combo_interval(&self) -> u8;
    fn right_combo_interval(&self) -> u8;

    fn left_combo_pause_us(&self) -> (u64, u64);
    fn right_combo_pause_us(&self) -> (u64, u64);

    fn left_first_hit_boost(&self) -> u8;
    fn right_first_hit_boost(&self) -> u8;

    fn release_penalty_ms(&self) -> u64;
    fn left_cps_limits(&self) -> (u8, u8, u8); // (min, avg, max)
    fn right_cps_limits(&self) -> (u8, u8, u8); // (min, avg, max)
}

pub fn get_server_timing(server_type: ServerType) -> RacResult<Box<dyn ServerTiming>> {
    match server_type {
        ServerType::Craftrise => Ok(Box::new(craftrise::CraftriseTiming)),
        ServerType::Sonoyuncu => Ok(Box::new(sonoyuncu::SonoyuncuTiming)),
        ServerType::Custom => Err(RacError::ConfigError(
            "Custom servers not yet supported".to_string(),
        )),
    }
}
