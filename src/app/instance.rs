use std::sync::OnceLock;
use windows::Win32::Foundation::HANDLE;

struct MutexHandle(HANDLE);

unsafe impl Send for MutexHandle {}
unsafe impl Sync for MutexHandle {}

impl Drop for MutexHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

static INSTANCE_MUTEX: OnceLock<MutexHandle> = OnceLock::new();

pub fn check_single_instance() -> bool {
    use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::w;

    unsafe {
        let mutex_name = w!("Global\\RACv2ApplicationMutex");
        match CreateMutexW(None, true, mutex_name) {
            Ok(handle) => {
                if windows::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS {
                    let _ = windows::Win32::Foundation::CloseHandle(handle);
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