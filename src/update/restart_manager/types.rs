use crate::update::restart_manager::RebootReason;
use crate::update::restart_manager::info::ProcessInfo;
use windows::Win32::System::RestartManager::RM_APP_TYPE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationType {
    Unknown,
    MainWindow,
    OtherWindow,
    Service,
    Explorer,
    Console,
    Critical,
}

impl ApplicationType {
    pub(crate) fn from_rm_app_type(app_type: RM_APP_TYPE) -> Self {
        match app_type.0 {
            0 => Self::Unknown,
            1 => Self::MainWindow,
            2 => Self::OtherWindow,
            3 => Self::Service,
            4 => Self::Explorer,
            5 => Self::Console,
            1000 => Self::Critical,
            _ => Self::Unknown,
        }
    }

    pub fn can_shutdown(&self) -> bool {
        !matches!(self, Self::Critical)
    }
}

#[derive(Debug)]
pub enum FileLockStatus {
    NotLocked,
    LockedBy(Vec<ProcessLockInfo>),
    RequiresReboot(RebootReason),
}

#[derive(Debug, Clone)]
pub struct ProcessLockInfo {
    pub process_id: u32,
    pub process_name: String,
    pub app_type: ApplicationType,
    pub can_shutdown: bool,
}

impl ProcessLockInfo {
    pub fn from_process_info(info: ProcessInfo) -> Self {
        Self {
            process_id: info.process_id,
            process_name: info.process_name,
            app_type: info.app_type,
            can_shutdown: info.app_type.can_shutdown(),
        }
    }
}
