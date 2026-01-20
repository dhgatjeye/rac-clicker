mod error;
mod info;
mod session;
mod types;

pub use error::{RebootReason, RestartManagerError};
use session::RmSession;

use crate::core::RacResult;
use crate::update::restart_manager::types::{FileLockStatus, ProcessLockInfo};
use std::path::Path;

pub struct RestartManager;

impl RestartManager {
    pub fn check_file_locks(file_path: &Path) -> RacResult<Option<FileLockStatus>> {
        let session = match RmSession::new() {
            Ok(s) => s,
            Err(e) => {
                println!(
                    "Restart Manager unavailable: {}. Falling back to retry mechanism.",
                    e
                );

                return Ok(None);
            }
        };

        if let Err(e) = session.register_file(file_path) {
            println!(
                "Failed to register file with Restart Manager: {}. Falling back.",
                e
            );
            return Ok(None);
        }

        match session.get_processes() {
            Ok(processes) => {
                if processes.is_empty() {
                    Ok(Some(FileLockStatus::NotLocked))
                } else {
                    let lock_info: Vec<ProcessLockInfo> = processes
                        .into_iter()
                        .map(ProcessLockInfo::from_process_info)
                        .collect();
                    Ok(Some(FileLockStatus::LockedBy(lock_info)))
                }
            }
            Err(RestartManagerError::CriticalProcessDetected) => Ok(Some(
                FileLockStatus::RequiresReboot(RebootReason::CriticalProcess),
            )),
            Err(RestartManagerError::RebootRequired(reason)) => {
                Ok(Some(FileLockStatus::RequiresReboot(reason)))
            }
            Err(e) => {
                println!("Failed to query processes: {}. Falling back.", e);
                std::thread::sleep(std::time::Duration::from_secs(5));
                Ok(None)
            }
        }
    }

    pub fn release_file_locks(file_path: &Path) -> RacResult<bool> {
        match Self::check_file_locks(file_path)? {
            None => {
                std::thread::sleep(std::time::Duration::from_secs(3));
                Ok(false)
            }
            Some(FileLockStatus::NotLocked) => {
                println!("File is not locked");
                Ok(true)
            }
            Some(FileLockStatus::LockedBy(processes)) => {
                println!("File is locked by {} process(es):", processes.len());
                for proc in &processes {
                    println!(
                        "  - PID {}: {} (type: {:?}, can_shutdown: {})",
                        proc.process_id, proc.process_name, proc.app_type, proc.can_shutdown
                    );
                }

                let has_critical = processes.iter().any(|p| !p.can_shutdown);
                if has_critical {
                    return Err(crate::core::RacError::UpdateError(
                        "File is locked by critical system process. System reboot required."
                            .to_string(),
                    ));
                }

                println!("All locking processes can be shut down. Proceeding with update.");
                std::thread::sleep(std::time::Duration::from_secs(10));
                Ok(true)
            }
            Some(FileLockStatus::RequiresReboot(reason)) => Err(
                crate::core::RacError::UpdateError(format!("System reboot required: {:?}", reason)),
            ),
        }
    }
}
