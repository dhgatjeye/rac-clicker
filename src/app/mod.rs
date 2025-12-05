mod instance;
mod runner;
mod updater;

pub use instance::check_single_instance;
pub use runner::RacApp;
pub use updater::check_and_update;