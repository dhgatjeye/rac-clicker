use crate::error::ResourceError;
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Debug, Clone)]
pub struct WindowsSdk {
    pub root: PathBuf,
    pub version: String,
    pub rc_exe: PathBuf,
    pub include_paths: Vec<PathBuf>,
}

impl WindowsSdk {
    pub fn discover() -> Result<Self, ResourceError> {
        let mut searched_locations = Vec::new();

        if let Some(sdk) = Self::from_environment(&mut searched_locations) {
            return Ok(sdk);
        }

        if let Some(sdk) = Self::from_registry(&mut searched_locations) {
            return Ok(sdk);
        }

        if let Some(sdk) = Self::from_known_paths(&mut searched_locations) {
            return Ok(sdk);
        }

        Err(ResourceError::SdkNotFound { searched_locations })
    }

    fn from_environment(searched: &mut Vec<PathBuf>) -> Option<Self> {
        let sdk_dir = env::var("WindowsSdkDir").ok()?;
        let sdk_path = PathBuf::from(&sdk_dir);

        if !sdk_path.exists() {
            searched.push(sdk_path);
            return None;
        }

        let version = env::var("WindowsSdkVersion")
            .ok()
            .map(|v| v.trim_end_matches('\\').to_string())
            .or_else(|| Self::find_latest_sdk_version(&sdk_path))?;

        Self::validate_sdk_installation(sdk_path, version, searched)
    }

    #[cfg(windows)]
    fn from_registry(searched: &mut Vec<PathBuf>) -> Option<Self> {
        use std::ptr::null_mut;

        const KEY_PATH: &str = r"SOFTWARE\Microsoft\Windows Kits\Installed Roots";
        const VALUE_NAME: &str = "KitsRoot10";

        unsafe {
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;

            type HKEY = *mut std::ffi::c_void;
            type DWORD = u32;
            type LONG = i32;
            type LPCWSTR = *const u16;
            type LPDWORD = *mut DWORD;
            type LPBYTE = *mut u8;

            const HKEY_LOCAL_MACHINE: HKEY = 0x80000002u32 as HKEY;
            const KEY_READ: DWORD = 0x20019;
            const KEY_WOW64_64KEY: DWORD = 0x0100;
            const ERROR_SUCCESS: LONG = 0;
            const REG_SZ: DWORD = 1;

            #[link(name = "advapi32")]
            unsafe extern "system" {
                #[allow(non_snake_case)]
                fn RegOpenKeyExW(
                    hKey: HKEY,
                    lpSubKey: LPCWSTR,
                    ulOptions: DWORD,
                    samDesired: DWORD,
                    phkResult: *mut HKEY,
                ) -> LONG;

                #[allow(non_snake_case)]
                fn RegQueryValueExW(
                    hKey: HKEY,
                    lpValueName: LPCWSTR,
                    lpReserved: LPDWORD,
                    lpType: LPDWORD,
                    lpData: LPBYTE,
                    lpcbData: LPDWORD,
                ) -> LONG;

                #[allow(non_snake_case)]
                fn RegCloseKey(hKey: HKEY) -> LONG;
            }

            fn to_wide(s: &str) -> Vec<u16> {
                OsStr::new(s).encode_wide().chain(Some(0)).collect()
            }

            let key_path_wide = to_wide(KEY_PATH);
            let value_name_wide = to_wide(VALUE_NAME);

            let mut hkey: HKEY = null_mut();

            for &access in &[KEY_READ | KEY_WOW64_64KEY, KEY_READ] {
                let result = RegOpenKeyExW(
                    HKEY_LOCAL_MACHINE,
                    key_path_wide.as_ptr(),
                    0,
                    access,
                    &mut hkey,
                );

                if result == ERROR_SUCCESS {
                    break;
                }
            }

            if hkey.is_null() {
                return None;
            }

            let mut value_type: DWORD = 0;
            let mut data_size: DWORD = 0;

            let result = RegQueryValueExW(
                hkey,
                value_name_wide.as_ptr(),
                null_mut(),
                &mut value_type,
                null_mut(),
                &mut data_size,
            );

            if result != ERROR_SUCCESS || value_type != REG_SZ || data_size == 0 {
                RegCloseKey(hkey);
                return None;
            }

            let mut buffer: Vec<u16> = vec![0; (data_size / 2) as usize];
            let result = RegQueryValueExW(
                hkey,
                value_name_wide.as_ptr(),
                null_mut(),
                null_mut(),
                buffer.as_mut_ptr() as LPBYTE,
                &mut data_size,
            );

            RegCloseKey(hkey);

            if result != ERROR_SUCCESS {
                return None;
            }

            let sdk_path_str = String::from_utf16_lossy(&buffer)
                .trim_end_matches('\0')
                .to_string();

            let sdk_path = PathBuf::from(sdk_path_str);

            if !sdk_path.exists() {
                searched.push(sdk_path);
                return None;
            }

            let version = Self::find_latest_sdk_version(&sdk_path)?;

            Self::validate_sdk_installation(sdk_path, version, searched)
        }
    }

    fn from_known_paths(searched: &mut Vec<PathBuf>) -> Option<Self> {
        let known_paths = [
            r"C:\Program Files (x86)\Windows Kits\10",
            r"C:\Program Files\Windows Kits\10",
            r"D:\Program Files (x86)\Windows Kits\10",
            r"D:\Program Files\Windows Kits\10",
        ];

        for path_str in &known_paths {
            let sdk_path = PathBuf::from(path_str);

            if !sdk_path.exists() {
                searched.push(sdk_path);
                continue;
            }

            if let Some(version) = Self::find_latest_sdk_version(&sdk_path) {
                if let Some(sdk) =
                    Self::validate_sdk_installation(sdk_path.clone(), version, searched)
                {
                    return Some(sdk);
                }
            } else {
                searched.push(sdk_path);
            }
        }

        None
    }

    fn find_latest_sdk_version(sdk_root: &Path) -> Option<String> {
        let include_dir = sdk_root.join("Include");

        if !include_dir.exists() {
            return None;
        }

        let mut versions: Vec<String> = fs::read_dir(&include_dir)
            .ok()?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().into_string().ok()?;

                if name.starts_with("10.") && entry.file_type().ok()?.is_dir() {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        versions.sort_by(|a, b| {
            let parse_version = |s: &str| -> Vec<u32> {
                s.split('.').filter_map(|part| part.parse().ok()).collect()
            };

            parse_version(a).cmp(&parse_version(b))
        });

        versions.pop()
    }

    fn validate_sdk_installation(
        sdk_root: PathBuf,
        version: String,
        searched: &mut Vec<PathBuf>,
    ) -> Option<Self> {
        let rc_exe = sdk_root
            .join("bin")
            .join(&version)
            .join("x64")
            .join("rc.exe");

        if !rc_exe.exists() {
            let alt_rc_exe = sdk_root.join("bin").join("x64").join("rc.exe");
            if !alt_rc_exe.exists() {
                searched.push(rc_exe);
                return None;
            }

            return Self::validate_sdk_installation_with_rc(
                sdk_root, version, alt_rc_exe, searched,
            );
        }

        Self::validate_sdk_installation_with_rc(sdk_root, version, rc_exe, searched)
    }

    fn validate_sdk_installation_with_rc(
        sdk_root: PathBuf,
        version: String,
        rc_exe: PathBuf,
        searched: &mut Vec<PathBuf>,
    ) -> Option<Self> {
        let include_base = sdk_root.join("Include").join(&version);

        let required_includes = ["um", "shared"];
        let mut include_paths = Vec::new();

        for dir in &required_includes {
            let path = include_base.join(dir);
            if !path.exists() {
                searched.push(path);
                return None;
            }
            include_paths.push(path);
        }

        let ucrt_path = include_base.join("ucrt");
        if ucrt_path.exists() {
            include_paths.push(ucrt_path);
        }

        Some(WindowsSdk {
            root: sdk_root,
            version,
            rc_exe,
            include_paths,
        })
    }

    pub fn resource_compiler(&self) -> &Path {
        &self.rc_exe
    }

    pub fn includes(&self) -> &[PathBuf] {
        &self.include_paths
    }
}

pub fn find_windows_sdk() -> Result<WindowsSdk, ResourceError> {
    WindowsSdk::discover()
}

pub fn get_target_arch() -> &'static str {
    let target = env::var("TARGET").unwrap_or_default();

    if target.starts_with("x86_64") || target.contains("x86_64") {
        "x64"
    } else if target.starts_with("i686") || target.starts_with("i586") {
        "x86"
    } else if target.starts_with("aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_target_arch() {
        let arch = get_target_arch();
        assert!(["x64", "x86", "arm64"].contains(&arch));
    }

    #[test]
    fn test_version_sorting() {
        let mut versions = vec![
            "10.0.19041.0".to_string(),
            "10.0.22621.0".to_string(),
            "10.0.17763.0".to_string(),
            "10.0.20348.0".to_string(),
        ];

        versions.sort_by(|a, b| {
            let parse_version = |s: &str| -> Vec<u32> {
                s.split('.').filter_map(|part| part.parse().ok()).collect()
            };

            parse_version(a).cmp(&parse_version(b))
        });

        assert_eq!(versions.last().unwrap(), "10.0.22621.0");
    }
}
