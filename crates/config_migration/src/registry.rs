use crate::error::{MigrationError, MigrationResult};
use crate::migration::MigrationStep;
use crate::version::SchemaVersion;
use serde_json::Value;

pub struct MigrationRegistry {
    migrations: Vec<Box<dyn MigrationStep>>,
}

impl MigrationRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(V1ToV2Migration));
        registry
    }

    pub fn register(&mut self, migration: Box<dyn MigrationStep>) {
        self.migrations.push(migration);
    }

    #[must_use]
    pub fn find_migration_path(
        &self,
        from: SchemaVersion,
        to: SchemaVersion,
    ) -> Vec<&dyn MigrationStep> {
        self.migrations
            .iter()
            .filter(|m| m.from_version() >= from.version() && m.to_version() <= to.version())
            .map(AsRef::as_ref)
            .collect()
    }

    pub fn apply_migrations(
        &self,
        mut value: Value,
        from: SchemaVersion,
        to: SchemaVersion,
    ) -> MigrationResult<Value> {
        let path = self.find_migration_path(from, to);

        if path.is_empty() && from != to {
            return Err(MigrationError::UnsupportedMigration {
                from_version: from.version(),
                to_version: to.version(),
            });
        }

        for migration in path {
            eprintln!(
                "[Migration] Applying migration: v{} -> v{}",
                migration.from_version(),
                migration.to_version()
            );
            value = migration.migrate(value)?;
        }

        Ok(value)
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

struct V1ToV2Migration;

impl MigrationStep for V1ToV2Migration {
    fn from_version(&self) -> u32 {
        1
    }

    fn to_version(&self) -> u32 {
        2
    }

    fn migrate(&self, mut value: Value) -> MigrationResult<Value> {
        let obj = value
            .as_object_mut()
            .ok_or_else(|| MigrationError::migration_failed(1, 2, "Expected JSON object"))?;

        if !obj.contains_key("auto_update_check") {
            obj.insert("auto_update_check".to_string(), Value::Bool(true));
            eprintln!("[Migration] Added 'auto_update_check' field with default value: true");
        }

        Ok(value)
    }

    fn description(&self) -> &'static str {
        "Add auto_update_check field for automatic update checking"
    }
}
