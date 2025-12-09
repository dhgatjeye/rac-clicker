use std::sync::OnceLock;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::w;

struct MutexHandle(HANDLE);

unsafe impl Send for MutexHandle {}
unsafe impl Sync for MutexHandle {}

impl Drop for MutexHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

static INSTANCE_MUTEX: OnceLock<MutexHandle> = OnceLock::new();

pub fn is_first_instance() -> bool {
    unsafe {
        let mutex_name = w!("Global\\RACv2ApplicationMutex");

        match CreateMutexW(None, true, mutex_name) {
            Ok(handle) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let _ = CloseHandle(handle);
                    false
                } else {
                    let _ = INSTANCE_MUTEX.set(MutexHandle(handle));
                    true
                }
            }
            Err(_) => false,
        }
    }
}

pub fn flush_console_input() {
    use windows::Win32::System::Console::{
        FlushConsoleInputBuffer, GetStdHandle, STD_INPUT_HANDLE,
    };

    unsafe {
        if let Ok(handle) = GetStdHandle(STD_INPUT_HANDLE) {
            let _ = FlushConsoleInputBuffer(handle);
        }
    }
}
