#![allow(clippy::upper_case_acronyms)]

mod compiler;
mod error;
mod toolchain;
mod types;

pub use compiler::ResourceCompiler;
pub use error::ResourceError;
pub use toolchain::{WindowsSdk, find_windows_sdk, get_target_arch};
pub use types::{
    file_types, get_manifest_dir, get_out_dir, get_profile, is_debug_build, is_msvc_windows_target,
    language_ids, os_types, resource_ids, resource_types, version_flags,
};

use std::path::Path;

pub fn compile(rc_file: impl AsRef<Path>) -> Result<(), ResourceError> {
    ResourceCompiler::new(rc_file).compile()
}

pub fn is_supported() -> bool {
    is_msvc_windows_target()
}
