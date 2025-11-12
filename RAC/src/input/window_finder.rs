use crate::input::handle::Handle;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use sysinfo::{ProcessesToUpdate, System};
use winapi::{
    shared::{minwindef::{DWORD, LPARAM}, windef::HWND},
    um::winuser::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible},
};

struct FindWindowData {
    pid: DWORD,
    hwnd: HWND,
    window_count: u32,
    require_visibility: bool,
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
    unsafe {
        let data = &mut *(lparam as *mut FindWindowData);
        let mut process_id: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == data.pid {
            let is_visible = IsWindowVisible(hwnd) != 0;
            if !data.require_visibility || is_visible {
                data.hwnd = hwnd;
                data.window_count += 1;
                return 1;
            }
        }
        1
    }
}

pub struct WindowFinder {
    target_process: Mutex<String>,
    target_process_lowercase: Mutex<String>,
    system: Arc<Mutex<System>>,
    last_found_pid: AtomicU32,
    require_visibility: bool,
}

impl WindowFinder {
    pub fn new(target_process: &str) -> Self {
        Self {
            target_process: Mutex::new(target_process.to_string()),
            target_process_lowercase: Mutex::new(target_process.to_lowercase()),
            system: Arc::new(Mutex::new(System::new_all())),
            last_found_pid: AtomicU32::new(0),
            require_visibility: true,
        }
    }

    pub fn update_target_process(&self, new_target_process: &str) -> bool {
        let current = self.target_process.lock().unwrap();
        if *current == new_target_process {
            return false;
        }
        drop(current);

        *self.target_process.lock().unwrap() = new_target_process.to_string();
        *self.target_process_lowercase.lock().unwrap() = new_target_process.to_lowercase();
        self.last_found_pid.store(0, Ordering::Relaxed);
        true
    }

    pub fn find_target_window(&self, hwnd_handle: &Arc<Mutex<Handle>>) -> Option<HWND> {
        let cached_pid = self.last_found_pid.load(Ordering::Relaxed);
        if cached_pid != 0 {
            if let Some(hwnd) = self.find_window_for_pid(cached_pid) {
                let mut hwnd_guard = hwnd_handle.lock().unwrap();
                hwnd_guard.set(hwnd);
                return Some(hwnd);
            } else {
                self.last_found_pid.store(0, Ordering::Relaxed);
            }
        }

        let mut sys = self.system.lock().unwrap();
        sys.refresh_processes(ProcessesToUpdate::All, false);

        let target_lower = self.target_process_lowercase.lock().unwrap().clone();

        let mut target_pid: Option<DWORD> = None;
        for (pid, process) in sys.processes() {
            let name_lower = process.name().to_string_lossy().to_lowercase();
            if name_lower == target_lower {
                target_pid = Some(pid.as_u32());
                break;
            }
        }
        drop(sys);

        if let Some(pid) = target_pid {
            self.last_found_pid.store(pid, Ordering::Relaxed);

            if let Some(hwnd) = self.find_window_for_pid(pid) {
                let mut hwnd_guard = hwnd_handle.lock().unwrap();
                hwnd_guard.set(hwnd);
                return Some(hwnd);
            }
        }

        let mut hwnd_guard = hwnd_handle.lock().unwrap();
        hwnd_guard.set(null_mut());
        None
    }

    fn find_window_for_pid(&self, pid: DWORD) -> Option<HWND> {
        let mut data = FindWindowData {
            pid,
            hwnd: null_mut(),
            window_count: 0,
            require_visibility: self.require_visibility,
        };
        unsafe {
            EnumWindows(Some(enum_windows_callback), &mut data as *mut _ as LPARAM);
            if !data.hwnd.is_null() {
                return Some(data.hwnd);
            }
        }
        None
    }
}