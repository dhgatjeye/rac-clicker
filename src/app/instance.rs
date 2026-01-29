use crate::core::{RacError, RacResult};
use crate::update::security::{check_path_for_reparse_points, create_dir};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::sync::OnceLock;

struct LockFileGuard {
    _file: File,
    path: PathBuf,
}

impl Drop for LockFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

static INSTANCE_LOCK: OnceLock<LockFileGuard> = OnceLock::new();

fn get_lock_file_path() -> RacResult<PathBuf> {
    let local_appdata = std::env::var("LOCALAPPDATA")
        .map_err(|_| RacError::UpdateError("Cannot find LOCALAPPDATA".to_string()))?;

    let lock_dir = PathBuf::from(local_appdata).join("RAC");

    check_path_for_reparse_points(&lock_dir)?;

    if !lock_dir.exists() {
        create_dir(&lock_dir)?;
    }

    Ok(lock_dir.join(".session.lock"))
}

pub fn is_first_instance() -> bool {
    let lock_path = match get_lock_file_path() {
        Ok(path) => path,
        Err(_) => return false,
    };

    if let Some(parent) = lock_path.parent()
        && check_path_for_reparse_points(parent).is_err()
    {
        return false;
    }

    match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&lock_path)
    {
        Ok(file) => {
            if try_lock_file(&file) {
                let guard = LockFileGuard {
                    _file: file,
                    path: lock_path,
                };
                INSTANCE_LOCK.set(guard).is_ok()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

fn try_lock_file(file: &File) -> bool {
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
        .is_ok()
    }
}
