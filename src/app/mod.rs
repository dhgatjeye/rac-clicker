pub mod instance;
pub mod runner;
pub mod ui;
pub mod updater;

pub use instance::{flush_console_input, is_first_instance};
pub use runner::{RacApp, has_configured_hotkeys};
pub use updater::check_and_update;
