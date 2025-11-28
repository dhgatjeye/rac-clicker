use crate::input::handle::Handle;
use std::sync::{Arc, Mutex};
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible};

struct FindWindowData {
    pid: u32,
    hwnd: HWND,
    window_count: u32,
    require_visibility: bool,
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let data = &mut *(lparam.0 as *mut FindWindowData);
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
        if process_id == data.pid {
            let is_visible = IsWindowVisible(hwnd).as_bool();
            if !data.require_visibility || is_visible {
                data.hwnd = hwnd;
                data.window_count += 1;
                return true.into();
            }
        }
        true.into()
    }
}

pub struct WindowFinder {
    target_process: Mutex<String>,
    last_found_pid: Mutex<Option<u32>>,
    require_visibility: bool,
}

impl WindowFinder {
    pub fn new(target_process: &str) -> Self {
        Self {
            target_process: Mutex::new(target_process.to_string()),
            last_found_pid: Mutex::new(None),
            require_visibility: true,
        }
    }

    pub fn update_target_process(&self, new_target_process: &str) -> bool {
        let mut target = self.target_process.lock().unwrap();
        if *target == new_target_process {
            return false;
        }
        *target = new_target_process.to_string();
        *self.last_found_pid.lock().unwrap() = None;
        true
    }

    pub fn find_target_window(&self, hwnd_handle: &Arc<Mutex<Handle>>) -> Option<HWND> {
        let last_pid = *self.last_found_pid.lock().unwrap();

        if let Some(pid) = last_pid {
            if let Some(hwnd) = self.find_window_for_pid(pid) {
                let hwnd_guard = hwnd_handle.lock().unwrap();
                hwnd_guard.set(hwnd);
                return Some(hwnd);
            } else {
                *self.last_found_pid.lock().unwrap() = None;
            }
        }

        let target_process_name = self.target_process.lock().unwrap().clone();
        let target_pid = self.find_process_by_name(&target_process_name);

        if let Some(pid) = target_pid {
            *self.last_found_pid.lock().unwrap() = Some(pid);

            if let Some(hwnd) = self.find_window_for_pid(pid) {
                let hwnd_guard = hwnd_handle.lock().unwrap();
                hwnd_guard.set(hwnd);
                return Some(hwnd);
            }
        }

        let hwnd_guard = hwnd_handle.lock().unwrap();
        hwnd_guard.set(HWND::default());
        None
    }

    fn find_process_by_name(&self, process_name: &str) -> Option<u32> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;

            let mut process_entry = PROCESSENTRY32W::default();
            process_entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snapshot, &mut process_entry).is_ok() {
                loop {
                    let exe_name = String::from_utf16_lossy(
                        &process_entry.szExeFile
                            .iter()
                            .take_while(|&&c| c != 0)
                            .map(|&c| c)
                            .collect::<Vec<u16>>()
                    );

                    if exe_name.to_lowercase() == process_name.to_lowercase() {
                        let _ = CloseHandle(snapshot);
                        return Some(process_entry.th32ProcessID);
                    }

                    if Process32NextW(snapshot, &mut process_entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
            None
        }
    }

    fn find_window_for_pid(&self, pid: u32) -> Option<HWND> {
        let mut data = FindWindowData {
            pid,
            hwnd: HWND::default(),
            window_count: 0,
            require_visibility: self.require_visibility,
        };
        unsafe {
            let _ = EnumWindows(Some(enum_windows_callback), LPARAM(&mut data as *mut _ as isize));
            if !data.hwnd.is_invalid() {
                return Some(data.hwnd);
            }
        }
        None
    }
}