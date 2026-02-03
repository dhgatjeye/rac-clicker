use crate::error::{MigrationError, MigrationResult};
use crate::registry::MigrationRegistry;
use crate::version::{SchemaVersion, VersionDetector};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

#[allow(clippy::wrong_self_convention)]
pub trait MigrationStep: Send + Sync {
    fn from_version(&self) -> u32;
    fn to_version(&self) -> u32;
    fn migrate(&self, value: Value) -> MigrationResult<Value>;
    fn description(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct MigrationConfig {
    pub target_version: u32,
    pub create_backup: bool,
    pub backup_suffix: String,
    pub verbose: bool,
}

impl MigrationConfig {
    #[must_use]
    pub fn new(target_version: u32) -> Self {
        Self {
            target_version,
            create_backup: true,
            backup_suffix: ".pre-migration".to_string(),
            verbose: true,
        }
    }

    #[must_use]
    pub fn without_backup(mut self) -> Self {
        self.create_backup = false;
        self
    }

    #[must_use]
    pub fn with_backup_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.backup_suffix = suffix.into();
        self
    }

    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

#[derive(Debug, Clone)]
pub struct MigrationReport {
    pub migrated: bool,
    pub from_version: u32,
    pub to_version: u32,
    pub backup_path: Option<String>,
    pub migrations_applied: Vec<String>,
}

impl MigrationReport {
    #[must_use]
    pub fn no_migration_needed() -> Self {
        Self {
            migrated: false,
            from_version: 0,
            to_version: 0,
            backup_path: None,
            migrations_applied: Vec::new(),
        }
    }

    #[must_use]
    pub fn success(from: u32, to: u32, backup: Option<String>, descriptions: Vec<String>) -> Self {
        Self {
            migrated: true,
            from_version: from,
            to_version: to,
            backup_path: backup,
            migrations_applied: descriptions,
        }
    }
}

pub struct Migrator {
    config: MigrationConfig,
    detector: VersionDetector,
    registry: MigrationRegistry,
}

impl Migrator {
    #[must_use]
    pub fn new(config: MigrationConfig) -> Self {
        Self {
            config,
            detector: VersionDetector::new(),
            registry: MigrationRegistry::with_defaults(),
        }
    }

    #[must_use]
    pub fn with_registry(config: MigrationConfig, registry: MigrationRegistry) -> Self {
        Self {
            config,
            detector: VersionDetector::new(),
            registry,
        }
    }

    pub fn migrate_if_needed(&self, path: &Path) -> MigrationResult<MigrationReport> {
        if !path.exists() {
            if self.config.verbose {
                eprintln!("[Migration] Configuration file does not exist, skipping migration");
            }
            return Ok(MigrationReport::no_migration_needed());
        }

        let contents = Self::read_file(path)?;
        let value: Value = serde_json::from_str(&contents)
            .map_err(|e| MigrationError::parse_error(format!("parsing {}", path.display()), e))?;

        let detected_version = self.detector.detect(&value)?;
        let target_version = SchemaVersion::new(self.config.target_version);

        if self.config.verbose {
            eprintln!(
                "[Migration] Detected schema version: {detected_version}, target: {target_version}"
            );
        }

        if !self
            .detector
            .needs_migration(detected_version, target_version)
        {
            if self.config.verbose {
                eprintln!("[Migration] No migration needed");
            }
            return Ok(MigrationReport {
                migrated: false,
                from_version: detected_version.version(),
                to_version: target_version.version(),
                backup_path: None,
                migrations_applied: Vec::new(),
            });
        }

        let backup_path = if self.config.create_backup {
            Some(self.create_backup(path)?)
        } else {
            None
        };

        let migrated_value =
            self.registry
                .apply_migrations(value, detected_version, target_version)?;

        let descriptions: Vec<String> = self
            .registry
            .find_migration_path(detected_version, target_version)
            .iter()
            .map(|m| m.description().to_string())
            .collect();

        Self::write_atomic(path, &migrated_value)?;

        if self.config.verbose {
            eprintln!(
                "[Migration] Successfully migrated from {detected_version} to {target_version}"
            );
        }

        Ok(MigrationReport::success(
            detected_version.version(),
            target_version.version(),
            backup_path,
            descriptions,
        ))
    }

    fn read_file(path: &Path) -> MigrationResult<String> {
        let mut file = OpenOptions::new()
            .read(true)
            .open(path)
            .map_err(|e| MigrationError::io_error(format!("opening {}", path.display()), e))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| MigrationError::io_error(format!("reading {}", path.display()), e))?;

        Ok(contents)
    }

    fn create_backup(&self, path: &Path) -> MigrationResult<String> {
        let backup_path = path.with_extension(format!("json{}", self.config.backup_suffix));

        fs::copy(path, &backup_path).map_err(|e| MigrationError::BackupFailed {
            path: backup_path.display().to_string(),
            source: e,
        })?;

        if self.config.verbose {
            eprintln!("[Migration] Created backup at: {}", backup_path.display());
        }

        Ok(backup_path.display().to_string())
    }

    fn write_atomic(path: &Path, value: &Value) -> MigrationResult<()> {
        let json = serde_json::to_string_pretty(value)?;
        let temp_path = path.with_extension("json.migration-tmp");

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| {
                MigrationError::io_error(format!("creating temp file {}", temp_path.display()), e)
            })?;

        file.write_all(json.as_bytes()).map_err(|e| {
            MigrationError::io_error(format!("writing temp file {}", temp_path.display()), e)
        })?;

        file.sync_all().map_err(|e| {
            MigrationError::io_error(format!("syncing temp file {}", temp_path.display()), e)
        })?;

        fs::rename(&temp_path, path).map_err(|e| {
            MigrationError::io_error(
                format!("renaming {} to {}", temp_path.display(), path.display()),
                e,
            )
        })?;

        Ok(())
    }
}
