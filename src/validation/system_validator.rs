use crate::config::constants::defaults;
use crate::helper::windows_paths::get_local_appdata;
use crate::logger::logger::{log_error, log_info};
use crate::validation::validation_result::ValidationResult;
use std::path::PathBuf;
use windows::Wdk::System::SystemServices::RtlGetVersion;
use windows::Win32::Foundation::POINT;
use windows::Win32::System::SystemInformation::{GetSystemInfo, GlobalMemoryStatusEx, MEMORYSTATUSEX, OSVERSIONINFOW, SYSTEM_INFO};
use windows::Win32::UI::Input::KeyboardAndMouse::{mouse_event, MOUSEEVENTF_MOVE};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

pub struct SystemRequirements {
    minimum_windows_version: i32,
    required_directories: Vec<PathBuf>,
}

impl Default for SystemRequirements {
    fn default() -> Self {
        let context = "SystemRequirements::default";
        let rac_dir = get_local_appdata().unwrap().join(defaults::RAC_DIR);
        let logs_path = rac_dir.join(defaults::RAC_LOG_PATH);

        if !rac_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&rac_dir) {
                log_error(&format!("Failed to create RAC directory: {}", e), context);
            }
        }

        if !logs_path.exists() {
            if let Err(e) = std::fs::write(&logs_path, "") {
                log_error(&format!("Failed to create logs file: {}", e), context);
            }
        }

        Self {
            minimum_windows_version: 10,
            required_directories: vec![rac_dir],
        }
    }
}

pub struct SystemValidator {
    requirements: SystemRequirements,
}

impl SystemValidator {
    pub fn new() -> Self {
        let context = "SystemValidator::new";
        log_info("Initializing system validator", context);
        Self {
            requirements: SystemRequirements::default(),
        }
    }

    pub fn validate_system(&self) -> ValidationResult {
        let context = "SystemValidator::validate_system";
        let validations = [
            self.validate_operating_system(),
            self.validate_windows_version(),
            self.validate_directory_permissions(),
            self.validate_mouse_access(),
            self.validate_hardware_resources(),
        ];

        for result in validations {
            if !result.is_valid {
                if let Some(msg) = &result.message {
                    log_error(msg, context);
                }
                return result;
            }
        }

        log_info("System validation completed successfully", context);
        ValidationResult::with_message(true, "System validation successful")
    }

    fn validate_operating_system(&self) -> ValidationResult {
        let context = "SystemValidator::validate_operating_system";
        if !cfg!(windows) {
            let error_msg = format!("Unsupported operating system. Required: Windows, Current: {}", std::env::consts::OS);
            log_error(&error_msg, context);
            return ValidationResult::with_message(false, error_msg);
        }
        ValidationResult::new(true)
    }

    fn validate_windows_version(&self) -> ValidationResult {
        let context = "SystemValidator::validate_windows_version";

        unsafe {
            let mut version_info = OSVERSIONINFOW::default();
            version_info.dwOSVersionInfoSize = size_of::<OSVERSIONINFOW>() as u32;

            let status = RtlGetVersion(&mut version_info as *mut _);
            if status.is_err() {
                let error_msg = "Failed to get Windows version information";
                log_error(error_msg, context);
                return ValidationResult::with_message(false, error_msg);
            }

            let major_version = version_info.dwMajorVersion as i32;

            if major_version < self.requirements.minimum_windows_version {
                let error_msg = format!(
                    "Unsupported Windows version. Required: {}, Current: {}",
                    self.requirements.minimum_windows_version,
                    major_version
                );
                log_error(&error_msg, context);
                return ValidationResult::with_message(false, error_msg);
            }
        }

        ValidationResult::new(true)
    }

    fn validate_directory_permissions(&self) -> ValidationResult {
        let context = "SystemValidator::validate_directory_permissions";
        for dir in &self.requirements.required_directories {
            if let Err(e) = std::fs::create_dir_all(dir) {
                let error_msg = format!("Directory permission check failed for: {}", dir.display());
                log_error(&format!("{}: {}", error_msg, e), context);
                return ValidationResult::with_message(false, error_msg);
            }

            let test_file = dir.join(format!("test_{}.tmp", rand::random::<u64>()));
            if let Err(e) = std::fs::write(&test_file, "test") {
                let error_msg = format!("Failed to write test file in: {}", dir.display());
                log_error(&format!("{}: {}", error_msg, e), context);
                return ValidationResult::with_message(false, error_msg);
            }
            let _ = std::fs::remove_file(test_file);
        }
        ValidationResult::new(true)
    }

    fn validate_mouse_access(&self) -> ValidationResult {
        let context = "SystemValidator::validate_mouse_access";
        unsafe {
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point as *mut _).is_err() {
                let error_msg = "Failed to access mouse controls";
                log_error(error_msg, context);
                return ValidationResult::with_message(false, error_msg);
            }

            mouse_event(MOUSEEVENTF_MOVE, 1, 1, 0, 0);
            std::thread::sleep(std::time::Duration::from_millis(50));
            mouse_event(MOUSEEVENTF_MOVE, -1, -1, 0, 0);

            ValidationResult::new(true)
        }
    }

    fn validate_hardware_resources(&self) -> ValidationResult {
        let context = "SystemValidator::validate_hardware_resources";

        let mut mem_status = MEMORYSTATUSEX::default();
        mem_status.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        unsafe {
            if GlobalMemoryStatusEx(&mut mem_status).is_err() {
                let error_msg = "Failed to get memory status";
                log_error(error_msg, context);
                return ValidationResult::with_message(false, error_msg);
            }
        }
        let total_memory_mb = mem_status.ullTotalPhys / (1024 * 1024);

        let mut system_info = SYSTEM_INFO::default();
        unsafe {
            GetSystemInfo(&mut system_info);
        }
        let cpu_count = system_info.dwNumberOfProcessors as usize;

        if total_memory_mb < defaults::MIN_MEMORY_MB {
            let error_msg = format!(
                "Insufficient total memory. Required: {} MB (4 GB), Available: {} MB",
                defaults::MIN_MEMORY_MB,
                total_memory_mb
            );
            log_error(&error_msg, context);
            return ValidationResult::with_message(false, error_msg);
        }

        if cpu_count < defaults::MIN_CPU_CORES {
            let error_msg = format!(
                "Insufficient CPU cores. Required: {}, Available: {}",
                defaults::MIN_CPU_CORES,
                cpu_count
            );
            log_error(&error_msg, context);
            return ValidationResult::with_message(false, error_msg);
        }

        ValidationResult::new(true)
    }
}