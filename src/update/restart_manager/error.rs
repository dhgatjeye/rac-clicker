use std::fmt;

#[derive(Debug, Clone)]
pub enum RestartManagerError {
    SessionCreationFailed(u32),
    RegistrationFailed(u32),
    QueryFailed(u32),
    ShutdownFailed(u32),
    CriticalProcessDetected,
    RebootRequired(RebootReason),
    InvalidPath,
    InsufficientBuffer,
    PermissionDenied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebootReason {
    PermissionDenied,
    SessionMismatch,
    CriticalProcess,
    CriticalService,
}

impl fmt::Display for RestartManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionCreationFailed(code) => {
                write!(
                    f,
                    "Failed to create Restart Manager session (error code: {})",
                    code
                )
            }
            Self::RegistrationFailed(code) => {
                write!(
                    f,
                    "Failed to register resources with Restart Manager (error code: {})",
                    code
                )
            }
            Self::QueryFailed(code) => {
                write!(
                    f,
                    "Failed to query processes from Restart Manager (error code: {})",
                    code
                )
            }
            Self::ShutdownFailed(code) => {
                write!(f, "Failed to shutdown processes (error code: {})", code)
            }
            Self::CriticalProcessDetected => {
                write!(
                    f,
                    "Critical system process detected - system reboot required"
                )
            }
            Self::RebootRequired(reason) => {
                write!(f, "System reboot required: {:?}", reason)
            }
            Self::InvalidPath => {
                write!(f, "Invalid file path provided")
            }
            Self::InsufficientBuffer => {
                write!(f, "Buffer too small for Restart Manager operation")
            }
            Self::PermissionDenied => {
                write!(f, "Permission denied to shutdown process")
            }
        }
    }
}

impl std::error::Error for RestartManagerError {}
