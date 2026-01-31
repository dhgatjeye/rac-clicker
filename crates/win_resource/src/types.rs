use std::env;
use std::path::PathBuf;

pub fn is_msvc_windows_target() -> bool {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    target_os == "windows" && target_env == "msvc"
}

pub fn get_out_dir() -> Option<PathBuf> {
    env::var("OUT_DIR").ok().map(PathBuf::from)
}

pub fn get_manifest_dir() -> Option<PathBuf> {
    env::var("CARGO_MANIFEST_DIR").ok().map(PathBuf::from)
}

pub fn get_profile() -> Option<String> {
    env::var("PROFILE").ok()
}

pub fn is_debug_build() -> bool {
    get_profile().map(|p| p == "debug").unwrap_or(false)
}

pub mod resource_types {
    pub const RT_MANIFEST: u32 = 24;
    pub const RT_ICON: u32 = 3;
    pub const RT_GROUP_ICON: u32 = 14;
    pub const RT_VERSION: u32 = 16;
    pub const RT_STRING: u32 = 6;
    pub const RT_DIALOG: u32 = 5;
    pub const RT_BITMAP: u32 = 2;
    pub const RT_CURSOR: u32 = 1;
}

pub mod resource_ids {
    pub const CREATEPROCESS_MANIFEST_RESOURCE_ID: u32 = 1;
    pub const ISOLATIONAWARE_MANIFEST_RESOURCE_ID: u32 = 2;
}

pub mod language_ids {
    pub const LANG_ENGLISH_US: u16 = 0x0409;
    pub const CP_UNICODE: u16 = 0x04B0;
}

pub mod version_flags {
    pub const VS_FF_DEBUG: u32 = 0x00000001;
    pub const VS_FF_PRERELEASE: u32 = 0x00000002;
    pub const VS_FF_PATCHED: u32 = 0x00000004;
    pub const VS_FF_PRIVATEBUILD: u32 = 0x00000008;
    pub const VS_FF_INFOINFERRED: u32 = 0x00000010;
    pub const VS_FF_SPECIALBUILD: u32 = 0x00000020;
    pub const VS_FFI_FILEFLAGSMASK: u32 = 0x0000003F;
}

pub mod file_types {
    pub const VFT_APP: u32 = 0x00000001;
    pub const VFT_DLL: u32 = 0x00000002;
    pub const VFT_DRV: u32 = 0x00000003;
    pub const VFT_FONT: u32 = 0x00000004;
    pub const VFT_STATIC_LIB: u32 = 0x00000007;
    pub const VFT_UNKNOWN: u32 = 0x00000000;
}

pub mod os_types {
    pub const VOS__WINDOWS32: u32 = 0x00000004;
    pub const VOS_NT: u32 = 0x00040000;
    pub const VOS_NT_WINDOWS32: u32 = 0x00040004;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_values() {
        assert_eq!(resource_types::RT_MANIFEST, 24);
        assert_eq!(resource_types::RT_ICON, 3);
        assert_eq!(resource_types::RT_VERSION, 16);
    }

    #[test]
    fn test_resource_id_values() {
        assert_eq!(resource_ids::CREATEPROCESS_MANIFEST_RESOURCE_ID, 1);
        assert_eq!(resource_ids::ISOLATIONAWARE_MANIFEST_RESOURCE_ID, 2);
    }
}
