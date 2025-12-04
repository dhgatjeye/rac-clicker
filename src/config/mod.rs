pub mod server;
pub mod settings;
pub mod profiles;

pub use server::{ServerConfig, ServerRegistry};
pub use settings::{Settings, SettingsManager};
pub use profiles::ConfigProfile;

pub use crate::core::{ServerType, ToggleMode, ClickMode};