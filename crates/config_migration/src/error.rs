use std::fmt;
use std::io;

pub type MigrationResult<T> = Result<T, MigrationError>;

#[derive(Debug)]
pub enum MigrationError {
    IoError {
        operation: String,
        source: io::Error,
    },
    ParseError {
        context: String,
        source: serde_json::Error,
    },
    MigrationFailed {
        from_version: u32,
        to_version: u32,
        reason: String,
    },
    VersionError {
        reason: String,
    },
    BackupFailed {
        path: String,
        source: io::Error,
    },
    Corrupted {
        path: String,
        reason: String,
    },
    UnsupportedMigration {
        from_version: u32,
        to_version: u32,
    },
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError { operation, source } => {
                write!(f, "I/O error during {operation}: {source}")
            }
            Self::ParseError { context, source } => {
                write!(f, "JSON parse error ({context}): {source}")
            }
            Self::MigrationFailed {
                from_version,
                to_version,
                reason,
            } => {
                write!(
                    f,
                    "Migration from v{from_version} to v{to_version} failed: {reason}"
                )
            }
            Self::VersionError { reason } => {
                write!(f, "Version error: {reason}")
            }
            Self::BackupFailed { path, source } => {
                write!(f, "Failed to create backup at '{path}': {source}")
            }
            Self::Corrupted { path, reason } => {
                write!(f, "Configuration file '{path}' is corrupted: {reason}")
            }
            Self::UnsupportedMigration {
                from_version,
                to_version,
            } => {
                write!(
                    f,
                    "Migration from v{from_version} to v{to_version} is not supported"
                )
            }
        }
    }
}

impl std::error::Error for MigrationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError { source, .. } | Self::BackupFailed { source, .. } => Some(source),
            Self::ParseError { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for MigrationError {
    fn from(err: io::Error) -> Self {
        Self::IoError {
            operation: "file operation".to_string(),
            source: err,
        }
    }
}

impl From<serde_json::Error> for MigrationError {
    fn from(err: serde_json::Error) -> Self {
        Self::ParseError {
            context: "JSON parsing".to_string(),
            source: err,
        }
    }
}

impl MigrationError {
    pub fn io_error(operation: impl Into<String>, source: io::Error) -> Self {
        Self::IoError {
            operation: operation.into(),
            source,
        }
    }

    pub fn parse_error(context: impl Into<String>, source: serde_json::Error) -> Self {
        Self::ParseError {
            context: context.into(),
            source,
        }
    }

    pub fn migration_failed(from: u32, to: u32, reason: impl Into<String>) -> Self {
        Self::MigrationFailed {
            from_version: from,
            to_version: to,
            reason: reason.into(),
        }
    }

    pub fn version_error(reason: impl Into<String>) -> Self {
        Self::VersionError {
            reason: reason.into(),
        }
    }

    pub fn corrupted(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Corrupted {
            path: path.into(),
            reason: reason.into(),
        }
    }
}
