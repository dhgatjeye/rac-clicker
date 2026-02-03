#![deny(unsafe_code)]
#![warn(clippy::all)]
#![allow(clippy::module_name_repetitions)]

mod error;
mod migration;
mod registry;
mod version;

pub use error::{MigrationError, MigrationResult};
pub use migration::{MigrationConfig, MigrationReport, MigrationStep, Migrator};
pub use registry::MigrationRegistry;
pub use version::{SchemaVersion, VersionDetector};

use std::path::Path;

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

#[cfg(feature = "migration")]
pub fn migrate_config_file(path: &Path, target_version: u32) -> MigrationResult<MigrationReport> {
    let config = MigrationConfig::new(target_version);
    let migrator = Migrator::new(config);
    migrator.migrate_if_needed(path)
}
