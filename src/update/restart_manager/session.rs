use super::error::RestartManagerError;
use crate::update::restart_manager::info::ProcessInfo;
use std::mem::MaybeUninit;
use std::path::Path;
use windows::Win32::System::RestartManager::{
    RM_PROCESS_INFO, RmEndSession, RmGetList, RmRegisterResources, RmStartSession,
};
use windows::core::{PCWSTR, PWSTR};

const CCH_RM_SESSION_KEY: usize = 32;

const RM_REBOOT_REASON_PERMISSION_DENIED: u32 = 0x1;
const RM_REBOOT_REASON_SESSION_MISMATCH: u32 = 0x2;
const RM_REBOOT_REASON_CRITICAL_PROCESS: u32 = 0x4;
const RM_REBOOT_REASON_CRITICAL_SERVICE: u32 = 0x8;
const RM_REBOOT_REASON_DETECTED_SELF: u32 = 0x10;

const MAX_PROCESS_COUNT: usize = 1024;

pub struct RmSession {
    handle: u32,
}

impl RmSession {
    pub fn new() -> Result<Self, RestartManagerError> {
        let mut handle: u32 = 0;
        let mut key = [0u16; CCH_RM_SESSION_KEY];

        unsafe {
            let result = RmStartSession(&mut handle, None, PWSTR(key.as_mut_ptr()));

            if result.0 != 0 {
                return Err(RestartManagerError::SessionCreationFailed(result.0));
            }
        }

        Ok(Self { handle })
    }

    pub fn register_file(&self, path: &Path) -> Result<(), RestartManagerError> {
        let path_str = path.to_str().ok_or(RestartManagerError::InvalidPath)?;

        let mut wide_path: Vec<u16> = path_str.encode_utf16().collect();
        wide_path.push(0);

        unsafe {
            let pcwstr = PCWSTR(wide_path.as_ptr());
            let result = RmRegisterResources(self.handle, Some(&[pcwstr]), None, None);

            if result.0 != 0 {
                return Err(RestartManagerError::RegistrationFailed(result.0));
            }
        }

        Ok(())
    }

    pub fn get_processes(&self) -> Result<Vec<ProcessInfo>, RestartManagerError> {
        for attempt in 0..3 {
            let mut processes_needed: u32 = 0;
            let mut reboot_reason: u32 = 0;

            unsafe {
                let result = RmGetList(
                    self.handle,
                    &mut processes_needed,
                    &mut 0,
                    None,
                    &mut reboot_reason,
                );

                if result.0 != 0 && result.0 != 234 {
                    return Err(RestartManagerError::QueryFailed(result.0));
                }
            }

            if processes_needed == 0 {
                return Ok(Vec::new());
            }

            let processes_needed_usize = processes_needed as usize;
            if processes_needed_usize > MAX_PROCESS_COUNT {
                return Err(RestartManagerError::QueryFailed(0));
            }

            let buffer_size = processes_needed_usize
                .saturating_add(4)
                .min(MAX_PROCESS_COUNT);

            let mut process_info_buffer: Vec<MaybeUninit<RM_PROCESS_INFO>> =
                Vec::with_capacity(buffer_size);

            unsafe {
                process_info_buffer.set_len(buffer_size);
            }

            let mut processes_returned: u32 = buffer_size as u32;

            unsafe {
                let result = RmGetList(
                    self.handle,
                    &mut processes_needed,
                    &mut processes_returned,
                    Some(process_info_buffer.as_mut_ptr() as *mut RM_PROCESS_INFO),
                    &mut reboot_reason,
                );

                if result.0 == 234 && attempt < 2 {
                    continue;
                }

                if result.0 != 0 {
                    return Err(RestartManagerError::QueryFailed(result.0));
                }
            }

            let filtered_reason = reboot_reason & !RM_REBOOT_REASON_DETECTED_SELF;

            if filtered_reason != 0 {
                return Err(self.parse_reboot_reason(reboot_reason));
            }

            let valid_count = (processes_returned as usize).min(buffer_size);
            let current_pid = std::process::id();

            let processes: Vec<ProcessInfo> = process_info_buffer
                .iter()
                .take(valid_count)
                .map(|maybe_uninit| {
                    let info = unsafe { maybe_uninit.assume_init_ref() };
                    ProcessInfo::from_rm_process_info(info)
                })
                .filter(|p| p.process_id != current_pid)
                .collect();

            return Ok(processes);
        }

        Err(RestartManagerError::QueryFailed(234))
    }

    fn parse_reboot_reason(&self, reason: u32) -> RestartManagerError {
        let reason = reason & !RM_REBOOT_REASON_DETECTED_SELF;

        debug_assert!(reason != 0, "parse_reboot_reason called with reason=0");

        if reason == 0 {
            return RestartManagerError::QueryFailed(reason);
        }

        if reason & RM_REBOOT_REASON_CRITICAL_PROCESS != 0 {
            return RestartManagerError::CriticalProcessDetected;
        }

        if reason & RM_REBOOT_REASON_CRITICAL_SERVICE != 0 {
            return RestartManagerError::RebootRequired(
                super::error::RebootReason::CriticalService,
            );
        }

        if reason & RM_REBOOT_REASON_PERMISSION_DENIED != 0 {
            return RestartManagerError::RebootRequired(
                super::error::RebootReason::PermissionDenied,
            );
        }

        if reason & RM_REBOOT_REASON_SESSION_MISMATCH != 0 {
            return RestartManagerError::RebootRequired(
                super::error::RebootReason::SessionMismatch,
            );
        }

        RestartManagerError::QueryFailed(reason)
    }
}

impl Drop for RmSession {
    fn drop(&mut self) {
        unsafe {
            let _ = RmEndSession(self.handle);
        }
    }
}
