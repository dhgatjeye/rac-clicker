pub mod craftrise;
pub mod sonoyuncu;

use crate::core::{RacError, RacResult, ServerType};

pub trait ServerTiming: Send {
    /// Get hold duration in microseconds (with jitter range)
    fn hold_duration_us(&self) -> (u64, i64); // (base, jitter_range)

    /// Get combo pattern enabled
    fn use_combo_pattern(&self) -> bool;

    /// Get combo interval (every N hits)
    fn combo_interval(&self) -> u8;

    /// Get combo pause duration in microseconds
    fn combo_pause_us(&self) -> (u64, u64); // (min, max)

    /// Get first-hit speed boost percentage (0-100)
    fn first_hit_boost(&self) -> u8;

    /// Get release penalty in milliseconds
    fn release_penalty_ms(&self) -> u64;

    /// Get left click CPS limits (min, max, hard_limit)
    fn left_cps_limits(&self) -> (u8, u8, u8);

    /// Get right click CPS limits (min, max, hard_limit)
    fn right_cps_limits(&self) -> (u8, u8, u8);
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
