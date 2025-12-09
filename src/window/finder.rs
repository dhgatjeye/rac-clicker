use crate::core::{RacError, RacResult};
use crate::window::WindowHandle;
use std::sync::atomic::{AtomicU32, Ordering};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
};
use windows::core::BOOL;

pub struct WindowFinder {
    target_process: String,
    cached_pid: AtomicU32,
}

impl WindowFinder {
    pub fn new(process_name: impl Into<String>) -> Self {
        Self {
            target_process: process_name.into(),
            cached_pid: AtomicU32::new(0),
        }
    }

    pub fn find_window(&self, window_handle: &WindowHandle) -> RacResult<bool> {
        let cached = self.cached_pid.load(Ordering::Relaxed);

        if cached != 0 {
            if let Some(hwnd) = self.find_window_for_pid(cached)? {
                window_handle.set(hwnd);
                return Ok(true);
            }
            self.cached_pid.store(0, Ordering::Relaxed);
        }

        if let Some(pid) = self.find_process_by_name()? {
            self.cached_pid.store(pid, Ordering::Relaxed);

            if let Some(hwnd) = self.find_window_for_pid(pid)? {
                window_handle.set(hwnd);
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn find_process_by_name(&self) -> RacResult<Option<u32>> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| RacError::WindowError(format!("Failed to create snapshot: {}", e)))?;

            let mut entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let name_len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let exe_name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);

                    if exe_name.eq_ignore_ascii_case(&self.target_process) {
                        let pid = entry.th32ProcessID;
                        let _ = CloseHandle(snapshot);
                        return Ok(Some(pid));
                    }

                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
            Ok(None)
        }
    }

    fn find_window_for_pid(&self, pid: u32) -> RacResult<Option<HWND>> {
        let mut data = FindWindowData {
            pid,
            hwnd: HWND(std::ptr::null_mut()),
        };

        unsafe {
            let _ = EnumWindows(
                Some(enum_windows_callback),
                LPARAM(&mut data as *mut _ as isize),
            );
        }

        if !data.hwnd.0.is_null() {
            Ok(Some(data.hwnd))
        } else {
            Ok(None)
        }
    }
}

struct FindWindowData {
    pid: u32,
    hwnd: HWND,
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let data = &mut *(lparam.0 as *mut FindWindowData);

        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        if process_id == data.pid && IsWindowVisible(hwnd).as_bool() {
            data.hwnd = hwnd;
            return BOOL(0);
        }

        BOOL(1)
    }
}
