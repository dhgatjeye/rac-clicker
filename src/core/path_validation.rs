use crate::core::{RacError, RacResult};
use std::path::Path;

const FORBIDDEN_PATH_CHARS: &[char] = &['<', '>', '"', '|', '?', '*', '\0'];

const DANGEROUS_SHELL_CHARS: &[char] = &['\n', '\r', '\t', '`', '$', ';', '&', '#'];

const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "COM¹", "COM²", "COM³", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
    "LPT9", "LPT¹", "LPT²", "LPT³",
];

const MAX_PATH_LENGTH: usize = 32767;

pub fn validate_path(path: &Path) -> RacResult<()> {
    let path_str = path
        .to_str()
        .ok_or_else(|| RacError::ValidationError("Path contains invalid UTF-8".to_string()))?;

    if path_str.is_empty() {
        return Err(RacError::ValidationError(
            "Path cannot be empty".to_string(),
        ));
    }

    if path_str.len() > MAX_PATH_LENGTH {
        return Err(RacError::ValidationError(
            "Path exceeds maximum allowed length".to_string(),
        ));
    }

    if path_str.starts_with("\\\\") {
        return Err(RacError::ValidationError(
            "UNC paths are not permitted".to_string(),
        ));
    }

    if path_str.contains("..") {
        return Err(RacError::ValidationError(
            "Path contains directory traversal sequence".to_string(),
        ));
    }

    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let name_upper = file_name.to_uppercase();
        let name_without_ext = name_upper.split('.').next().unwrap_or(&name_upper);
        if RESERVED_NAMES.contains(&name_without_ext) {
            return Err(RacError::ValidationError(
                "Path contains Windows reserved filename".to_string(),
            ));
        }
    }

    for (idx, c) in path_str.chars().enumerate() {
        if FORBIDDEN_PATH_CHARS.contains(&c) {
            return Err(RacError::ValidationError(format!(
                "Path contains forbidden character at position {}",
                idx
            )));
        }

        if DANGEROUS_SHELL_CHARS.contains(&c) {
            return Err(RacError::ValidationError(format!(
                "Path contains shell character at position {}",
                idx
            )));
        }

        if (c as u32) < 32 {
            return Err(RacError::ValidationError(format!(
                "Path contains control character at position {}",
                idx
            )));
        }
    }

    Ok(())
}

pub fn remove_file(path: &Path) {
    if validate_path(path).is_ok() {
        let _ = std::fs::remove_file(path);
    }
}
