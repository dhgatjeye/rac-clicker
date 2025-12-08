pub mod profiles;
pub mod server;
pub mod settings;

pub use profiles::ConfigProfile;
pub use server::{ServerConfig, ServerRegistry};
pub use settings::{Settings, SettingsManager};

pub use crate::core::{ClickMode, ServerType, ToggleMode};
