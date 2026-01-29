use crate::core::{RacError, RacResult};
use crate::update::security::{check_path_for_reparse_points, create_dir};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    First,
    AlreadyRunning,
}

struct LockFileGuard {
    _file: File,
    path: PathBuf,
}

impl Drop for LockFileGuard {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(&self.path) {
            eprintln!("Warning: Failed to remove lock file {:?}: {}", self.path, e);
        }
    }
}

static INSTANCE_LOCK: OnceLock<LockFileGuard> = OnceLock::new();

fn get_lock_file_path() -> RacResult<PathBuf> {
    let local_appdata = std::env::var("LOCALAPPDATA").map_err(|e| {
        RacError::UpdateError(format!(
            "Cannot find LOCALAPPDATA environment variable: {}",
            e
        ))
    })?;

    let lock_dir = PathBuf::from(local_appdata).join("RAC");

    check_path_for_reparse_points(&lock_dir)?;

    if !lock_dir.exists() {
        create_dir(&lock_dir)?;
    }

    Ok(lock_dir.join(".session.lock"))
}

pub fn is_first_instance() -> RacResult<InstanceStatus> {
    let lock_path = get_lock_file_path()
        .map_err(|e| RacError::IoError(format!("Failed to determine lock file path: {}", e)))?;

    if let Some(parent) = lock_path.parent() {
        check_path_for_reparse_points(parent).map_err(|e| {
            RacError::ValidationError(format!("Lock directory failed security validation: {}", e))
        })?;
    }

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&lock_path)
        .map_err(|e| {
            RacError::IoError(format!(
                "Failed to open lock file '{}': {}",
                lock_path.display(),
                e
            ))
        })?;

    match try_lock_file(&file) {
        Ok(()) => {
            let guard = LockFileGuard {
                _file: file,
                path: lock_path,
            };

            INSTANCE_LOCK.set(guard).map_err(|_| {
                RacError::SyncError("INSTANCE_LOCK already initialized".to_string())
            })?;

            Ok(InstanceStatus::First)
        }
        Err(e) => {
            if is_lock_held_error(&e) {
                Ok(InstanceStatus::AlreadyRunning)
            } else {
                Err(RacError::IoError(format!(
                    "Failed to acquire instance lock: {}",
                    e
                )))
            }
        }
    }
}

fn try_lock_file(file: &File) -> std::io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx,
    };

    let handle = HANDLE(file.as_raw_handle());
    let mut overlapped = unsafe { std::mem::zeroed() };

    unsafe {
        LockFileEx(
            handle,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            Some(0),
            1,
            0,
            &mut overlapped,
        )
        .map_err(|e| std::io::Error::from_raw_os_error(e.code().0))
    }
}

fn is_lock_held_error(error: &std::io::Error) -> bool {
    const ERROR_LOCK_VIOLATION: i32 = 33;
    const ERROR_SHARING_VIOLATION: i32 = 32;

    if let Some(code) = error.raw_os_error() {
        let win32_code = if code < 0 { code & 0xFFFF } else { code };

        matches!(win32_code, ERROR_LOCK_VIOLATION | ERROR_SHARING_VIOLATION)
    } else {
        false
    }
}
