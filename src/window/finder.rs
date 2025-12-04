use crate::core::{RacResult, RacError};
use crate::window::WindowHandle;
use windows::Win32::Foundation::{HWND, LPARAM, CloseHandle};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
    PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId, IsWindowVisible};
use windows::core::BOOL;
use std::sync::{Arc, Mutex};

pub struct WindowFinder {
    target_process: Arc<Mutex<String>>,
    last_found_pid: Arc<Mutex<Option<u32>>>,
}

impl WindowFinder {
    pub fn new(process_name: impl Into<String>) -> Self {
        Self {
            target_process: Arc::new(Mutex::new(process_name.into())),
            last_found_pid: Arc::new(Mutex::new(None)),
        }
    }
    
    pub fn set_target_process(&self, process_name: impl Into<String>) {
        if let Ok(mut target) = self.target_process.lock() {
            *target = process_name.into();
            if let Ok(mut pid) = self.last_found_pid.lock() {
                *pid = None;
            }
        }
    }
    
    pub fn find_window(&self, window_handle: &WindowHandle) -> RacResult<bool> {
        let target = match self.target_process.lock() {
            Ok(t) => t.clone(),
            Err(_) => return Err(RacError::SyncError("Failed to lock target process".to_string())),
        };
        
        let cached_pid = self.last_found_pid.lock().ok().and_then(|p| *p);
        
        if let Some(pid) = cached_pid {
            if let Some(hwnd) = self.find_window_for_pid(pid)? {
                window_handle.set(hwnd);
                return Ok(true);
            } else {
                if let Ok(mut p) = self.last_found_pid.lock() {
                    *p = None;
                }
            }
        }
        
        if let Some(pid) = self.find_process_by_name(&target)? {
            if let Ok(mut p) = self.last_found_pid.lock() {
                *p = Some(pid);
            }
            
            if let Some(hwnd) = self.find_window_for_pid(pid)? {
                window_handle.set(hwnd);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    fn find_process_by_name(&self, process_name: &str) -> RacResult<Option<u32>> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| RacError::WindowError(format!("Failed to create snapshot: {}", e)))?;

            let mut process_entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            if Process32FirstW(snapshot, &mut process_entry).is_ok() {
                loop {
                    let exe_name = String::from_utf16_lossy(
                        &process_entry.szExeFile
                            .iter()
                            .take_while(|&&c| c != 0)
                            .copied()
                            .collect::<Vec<u16>>(),
                    );

                    if exe_name.eq_ignore_ascii_case(process_name) {
                        let pid = process_entry.th32ProcessID;
                        let _ = CloseHandle(snapshot);
                        return Ok(Some(pid));
                    }

                    if Process32NextW(snapshot, &mut process_entry).is_err() {
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
    
    pub fn target_process(&self) -> String {
        self.target_process
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default()
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
        
        if process_id == data.pid {
            if IsWindowVisible(hwnd).as_bool() {
                data.hwnd = hwnd;
                return BOOL(1); 
            }
        }

        BOOL(1)
    }
}