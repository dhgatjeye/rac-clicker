use crate::update::restart_manager::types::ApplicationType;
use windows::Win32::System::RestartManager::RM_PROCESS_INFO;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub process_id: u32,
    pub process_name: String,
    pub app_type: ApplicationType,
}

impl ProcessInfo {
    pub fn from_rm_process_info(info: &RM_PROCESS_INFO) -> Self {
        let app_name_len = info
            .strAppName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(info.strAppName.len());

        let process_name = String::from_utf16_lossy(&info.strAppName[..app_name_len]);

        Self {
            process_id: info.Process.dwProcessId,
            process_name,
            app_type: ApplicationType::from_rm_app_type(info.ApplicationType),
        }
    }
}
