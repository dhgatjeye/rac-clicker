use std::fmt;

pub type RacResult<T> = Result<T, RacError>;

#[derive(Debug, Clone)]
pub enum RacError {
    ConfigError(String),
    IoError(String),
    SyncError(String),
    WindowError(String),
    InvalidInput(String),
    ValidationError(String),
    ThreadError(String),
    SerdeError(String),
    UpdateError(String),
    UpdateRestart,
    UserExit,
}

impl fmt::Display for RacError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
            Self::SyncError(msg) => write!(f, "Synchronization error: {}", msg),
            Self::WindowError(msg) => write!(f, "Window error: {}", msg),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            Self::ThreadError(msg) => write!(f, "Thread error: {}", msg),
            Self::SerdeError(msg) => write!(f, "Serialization error: {}", msg),
            Self::UpdateError(msg) => write!(f, "Update error: {}", msg),
            Self::UpdateRestart => write!(f, "Restarting for update"),
            Self::UserExit => write!(f, "User requested exit"),
        }
    }
}

impl std::error::Error for RacError {}

impl From<std::io::Error> for RacError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for RacError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerdeError(err.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for RacError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        Self::SyncError(format!("Mutex poisoned: {}", err))
    }
}
