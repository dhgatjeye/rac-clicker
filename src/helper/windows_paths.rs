use std::path::PathBuf;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath, KNOWN_FOLDER_FLAG};

pub fn get_local_appdata() -> Option<PathBuf> {
    unsafe {
        let path_ptr = SHGetKnownFolderPath(
            &FOLDERID_LocalAppData,
            KNOWN_FOLDER_FLAG(0),
            None,
        ).ok()?;

        let path_str = path_ptr.to_string().ok()?;
        CoTaskMemFree(Some(path_ptr.as_ptr() as *const _));

        Some(PathBuf::from(path_str))
    }
}