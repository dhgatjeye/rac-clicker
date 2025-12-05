use crate::core::{RacError, RacResult};
use crate::window::WindowHandle;
use std::sync::atomic::{AtomicU32, Ordering};
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible};

pub struct WindowFinder {
    target_process: Box<str>,
    cached_pid: AtomicU32,
}

impl WindowFinder {
    pub fn new(process_name: &str) -> Self {
        Self {
            target_process: process_name.into(),
            cached_pid: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn target_process(&self) -> &str {
        &self.target_process
    }

    pub fn find_and_update(&self, handle: &WindowHandle) -> RacResult<bool> {
        let cached = self.cached_pid.load(Ordering::Relaxed);
        if cached != 0 {
            if let Some(hwnd) = self.find_window_for_pid(cached)? {
                handle.set(hwnd);
                return Ok(true);
            }
            self.cached_pid.store(0, Ordering::Relaxed);
        }

        if let Some(pid) = self.find_process_id()? {
            self.cached_pid.store(pid, Ordering::Relaxed);

            if let Some(hwnd) = self.find_window_for_pid(pid)? {
                handle.set(hwnd);
                return Ok(true);
            }
        }

        handle.clear();
        Ok(false)
    }

    #[inline]
    pub fn invalidate_cache(&self) {
        self.cached_pid.store(0, Ordering::Relaxed);
    }

    fn find_process_id(&self) -> RacResult<Option<u32>> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| RacError::WindowError(format!("Snapshot failed: {e}")))?;

            struct SnapshotGuard(windows::Win32::Foundation::HANDLE);
            impl Drop for SnapshotGuard {
                fn drop(&mut self) {
                    let _ = unsafe { CloseHandle(self.0) };
                }
            }
            let _guard = SnapshotGuard(snapshot);

            let mut entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut entry).is_err() {
                return Ok(None);
            }

            loop {
                let name = extract_process_name(&entry.szExeFile);

                if name.eq_ignore_ascii_case(&self.target_process) {
                    return Ok(Some(entry.th32ProcessID));
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }

            Ok(None)
        }
    }

    fn find_window_for_pid(&self, pid: u32) -> RacResult<Option<HWND>> {
        let mut context = EnumContext {
            target_pid: pid,
            found_hwnd: None,
        };

        unsafe {
            let _ = EnumWindows(
                Some(enum_callback),
                LPARAM(&mut context as *mut _ as isize),
            );
        }

        Ok(context.found_hwnd)
    }
}

struct EnumContext {
    target_pid: u32,
    found_hwnd: Option<HWND>,
}

unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = unsafe { &mut *(lparam.0 as *mut EnumContext) };

    let mut window_pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut window_pid)) };

    if window_pid == ctx.target_pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
        ctx.found_hwnd = Some(hwnd);
        return BOOL(0);
    }

    BOOL(1)
}

#[inline]
fn extract_process_name(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}