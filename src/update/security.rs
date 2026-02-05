use crate::core::{RacError, RacResult, validate_path};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, GetDiskFreeSpaceExW,
    GetFileAttributesW, INVALID_FILE_ATTRIBUTES,
};
use windows::core::HSTRING;

const DISK_SPACE_SAFETY_MULTIPLIER: u64 = 3;

pub fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

pub fn is_reparse_point(path: &Path) -> bool {
    let path_str = match path.to_str() {
        Some(s) => s,
        None => return false,
    };

    let wide_path = HSTRING::from(path_str);
    let attributes = unsafe { GetFileAttributesW(&wide_path) };

    if attributes == INVALID_FILE_ATTRIBUTES {
        return false;
    }

    (attributes & FILE_ATTRIBUTE_REPARSE_POINT.0) != 0
}

pub fn check_path_for_reparse_points(path: &Path) -> RacResult<()> {
    validate_path(path)?;

    let mut current = path.to_path_buf();

    loop {
        validate_path(&current)?;

        if current.exists() && is_reparse_point(&current) {
            return Err(RacError::UpdateError(
                "Path component is a symbolic link or junction. Please remove it and try again."
                    .to_string(),
            ));
        }

        match current.parent() {
            Some(parent) => {
                if parent == current {
                    break;
                }
                current = parent.to_path_buf();
            }
            None => break,
        }
    }

    Ok(())
}

pub fn create_dir(path: &Path) -> RacResult<()> {
    validate_path(path)?;

    check_path_for_reparse_points(path)?;

    if path.exists() {
        if is_reparse_point(path) {
            return Err(RacError::UpdateError(
                "Directory is a symbolic link or junction.".to_string(),
            ));
        }
        return Ok(());
    }

    fs::create_dir_all(path)
        .map_err(|e| RacError::UpdateError(format!("Failed to create directory: {}", e)))?;

    if is_reparse_point(path) {
        let _ = fs::remove_dir(path);
        return Err(RacError::UpdateError(
            "Directory became a reparse point after creation.".to_string(),
        ));
    }

    Ok(())
}

pub fn file_write_check(path: &Path) -> RacResult<()> {
    validate_path(path)?;

    if let Some(parent) = path.parent() {
        check_path_for_reparse_points(parent)?;
    }

    if path.exists() && is_reparse_point(path) {
        return Err(RacError::UpdateError(
            "File is a symbolic link.".to_string(),
        ));
    }

    Ok(())
}

pub fn write_file(path: &Path, contents: &[u8]) -> RacResult<()> {
    validate_path(path)?;

    if let Some(parent) = path.parent() {
        check_path_for_reparse_points(parent)?;
    }

    if path.exists() {
        let file_result = OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .open(path);

        if let Ok(_file) = file_result
            && is_reparse_point(path)
        {
            return Err(RacError::UpdateError(
                "File is a symbolic link. Refusing to overwrite.".to_string(),
            ));
        }
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|e| RacError::UpdateError(format!("Failed to create file: {}", e)))?;

    if is_reparse_point(path) {
        drop(file);
        return Err(RacError::UpdateError(
            "File became a symbolic link during creation.".to_string(),
        ));
    }

    file.write_all(contents)
        .map_err(|e| RacError::UpdateError(format!("Failed to write to file: {}", e)))?;

    file.sync_all()
        .map_err(|e| RacError::UpdateError(format!("Failed to sync file: {}", e)))?;

    Ok(())
}

pub fn copy_file(src: &Path, dst: &Path) -> RacResult<u64> {
    validate_path(src)?;
    validate_path(dst)?;

    if is_reparse_point(src) {
        return Err(RacError::UpdateError(
            "Source file is a symbolic link.".to_string(),
        ));
    }

    if let Some(parent) = dst.parent() {
        check_path_for_reparse_points(parent)?;
    }

    if dst.exists() && is_reparse_point(dst) {
        return Err(RacError::UpdateError(
            "Destination is a symbolic link.".to_string(),
        ));
    }

    let mut src_file = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(src)
        .map_err(|e| RacError::UpdateError(format!("Failed to open source: {}", e)))?;

    if is_reparse_point(src) {
        return Err(RacError::UpdateError(
            "Source is a symbolic link.".to_string(),
        ));
    }

    let mut dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)
        .map_err(|e| RacError::UpdateError(format!("Failed to create destination: {}", e)))?;

    if is_reparse_point(dst) {
        drop(dst_file);
        return Err(RacError::UpdateError(
            "Destination became a symbolic link.".to_string(),
        ));
    }

    let bytes_copied = std::io::copy(&mut src_file, &mut dst_file)
        .map_err(|e| RacError::UpdateError(format!("Failed to copy file: {}", e)))?;

    dst_file
        .sync_all()
        .map_err(|e| RacError::UpdateError(format!("Failed to sync file: {}", e)))?;

    Ok(bytes_copied)
}

pub fn create_file_exclusively(path: &Path) -> RacResult<File> {
    validate_path(path)?;

    if let Some(parent) = path.parent() {
        check_path_for_reparse_points(parent)?;
    }

    if path.exists() {
        if is_reparse_point(path) {
            return Err(RacError::UpdateError(
                "File is a symbolic link. Refusing to overwrite.".to_string(),
            ));
        }

        fs::remove_file(path)
            .map_err(|e| RacError::UpdateError(format!("Failed to remove existing file: {}", e)))?;
    }

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(path)
        .map_err(|e| {
            RacError::UpdateError(format!(
                "Failed to create file: {}. \
                 If file exists unexpectedly, a race condition may have occurred.",
                e
            ))
        })?;

    if is_reparse_point(path) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(RacError::UpdateError(
            "File became a reparse point after creation.".to_string(),
        ));
    }

    Ok(file)
}

pub fn verify_disk_space(path: &Path, required_bytes: u64) -> RacResult<()> {
    validate_path(path)?;

    let drive_root = get_drive_root(path)?;

    let mut free_bytes_available: u64 = 0;

    let result = unsafe {
        GetDiskFreeSpaceExW(
            &HSTRING::from(drive_root.as_str()),
            Some(&mut free_bytes_available),
            None,
            None,
        )
    };

    if result.is_err() {
        eprintln!(
            "Could not verify disk space on '{}'. Proceeding with update.",
            drive_root
        );
        return Ok(());
    }

    let required_with_margin = required_bytes.saturating_mul(DISK_SPACE_SAFETY_MULTIPLIER);

    if free_bytes_available < required_with_margin {
        let free_mb = free_bytes_available / (1024 * 1024);
        let required_mb = required_with_margin / (1024 * 1024);

        return Err(RacError::UpdateError(format!(
            "Insufficient disk space for update.\n\
             Required: {} MB (including safety margin for backup)\n\
             Available: {} MB\n\
             Please free up at least {} MB on drive {} and try again.",
            required_mb,
            free_mb,
            required_mb.saturating_sub(free_mb),
            drive_root
        )));
    }

    Ok(())
}

fn get_drive_root(path: &Path) -> RacResult<String> {
    validate_path(path)?;

    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    if let Some(path_str) = abs_path.to_str() {
        if path_str.len() >= 2 {
            let bytes = path_str.as_bytes();
            if bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
                return Ok(format!("{}:\\", (bytes[0] as char).to_ascii_uppercase()));
            }
        }

        if path_str.starts_with("\\\\") {
            return Err(RacError::ValidationError(
                "UNC paths are not permitted".to_string(),
            ));
        }
    }

    Ok("C:\\".to_string())
}
