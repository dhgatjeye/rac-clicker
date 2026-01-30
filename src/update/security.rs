use crate::core::{RacError, RacResult};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, GetFileAttributesW,
    INVALID_FILE_ATTRIBUTES,
};
use windows::core::HSTRING;

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
    let mut current = path.to_path_buf();

    loop {
        if current.exists() && is_reparse_point(&current) {
            return Err(RacError::UpdateError(format!(
                "'{}' is a symbolic link or junction. Please remove it and try again.",
                current.display()
            )));
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
    check_path_for_reparse_points(path)?;

    if path.exists() {
        if is_reparse_point(path) {
            return Err(RacError::UpdateError(format!(
                "Directory '{}' is a symbolic link or junction.",
                path.display()
            )));
        }
        return Ok(());
    }

    fs::create_dir_all(path).map_err(|e| {
        RacError::UpdateError(format!(
            "Failed to create directory '{}': {}",
            path.display(),
            e
        ))
    })?;

    if is_reparse_point(path) {
        let _ = fs::remove_dir(path);
        return Err(RacError::UpdateError(format!(
            "Directory '{}' became a reparse point after creation.",
            path.display()
        )));
    }

    Ok(())
}

pub fn file_write_check(path: &Path) -> RacResult<()> {
    if let Some(parent) = path.parent() {
        check_path_for_reparse_points(parent)?;
    }

    if path.exists() && is_reparse_point(path) {
        return Err(RacError::UpdateError(format!(
            "File '{}' is a symbolic link.",
            path.display()
        )));
    }

    Ok(())
}

pub fn write_file(path: &Path, contents: &[u8]) -> RacResult<()> {
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
            return Err(RacError::UpdateError(format!(
                "File '{}' is a symbolic link. Refusing to overwrite.",
                path.display()
            )));
        }
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(|e| {
            RacError::UpdateError(format!("Failed to create file '{}': {}", path.display(), e))
        })?;

    if is_reparse_point(path) {
        drop(file);
        return Err(RacError::UpdateError(format!(
            "File '{}' became a symbolic link during creation.",
            path.display()
        )));
    }

    file.write_all(contents).map_err(|e| {
        RacError::UpdateError(format!(
            "Failed to write to file '{}': {}",
            path.display(),
            e
        ))
    })?;

    file.sync_all().map_err(|e| {
        RacError::UpdateError(format!(
            "Failed to sync_all file '{}': {}",
            path.display(),
            e
        ))
    })?;

    Ok(())
}

pub fn copy_file(src: &Path, dst: &Path) -> RacResult<u64> {
    if is_reparse_point(src) {
        return Err(RacError::UpdateError(format!(
            "Source file '{}' is a symbolic link.",
            src.display()
        )));
    }

    if let Some(parent) = dst.parent() {
        check_path_for_reparse_points(parent)?;
    }

    if dst.exists() && is_reparse_point(dst) {
        return Err(RacError::UpdateError(format!(
            "Destination '{}' is a symbolic link.",
            dst.display()
        )));
    }

    let mut src_file = OpenOptions::new()
        .read(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
        .open(src)
        .map_err(|e| {
            RacError::UpdateError(format!("Failed to open source '{}': {}", src.display(), e))
        })?;

    if is_reparse_point(src) {
        return Err(RacError::UpdateError(format!(
            "Source '{}' is a symbolic link.",
            src.display()
        )));
    }

    let mut dst_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)
        .map_err(|e| {
            RacError::UpdateError(format!(
                "Failed to create destination '{}': {}",
                dst.display(),
                e
            ))
        })?;

    if is_reparse_point(dst) {
        drop(dst_file);
        return Err(RacError::UpdateError(format!(
            "Destination '{}' became a symbolic link.",
            dst.display()
        )));
    }

    let bytes_copied = std::io::copy(&mut src_file, &mut dst_file).map_err(|e| {
        RacError::UpdateError(format!(
            "Failed to copy '{}' to '{}': {}",
            src.display(),
            dst.display(),
            e
        ))
    })?;

    dst_file.sync_all().map_err(|e| {
        RacError::UpdateError(format!(
            "Failed to sync_all file '{}': {}",
            dst.display(),
            e
        ))
    })?;

    Ok(bytes_copied)
}
