use crate::error::{MigrationError, MigrationResult};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SchemaVersion(pub u32);

impl SchemaVersion {
    #[must_use]
    pub const fn new(version: u32) -> Self {
        Self(version)
    }
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.0
    }
}

impl From<u32> for SchemaVersion {
    fn from(version: u32) -> Self {
        Self(version)
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct FieldSignature {
    pub required_fields: Vec<&'static str>,
    pub absent_fields: Vec<&'static str>,
}

impl FieldSignature {
    pub const fn new(required: Vec<&'static str>, absent: Vec<&'static str>) -> Self {
        Self {
            required_fields: required,
            absent_fields: absent,
        }
    }

    #[must_use]
    pub fn matches(&self, obj: &serde_json::Map<String, Value>) -> bool {
        for field in &self.required_fields {
            if !obj.contains_key(*field) {
                return false;
            }
        }

        for field in &self.absent_fields {
            if obj.contains_key(*field) {
                return false;
            }
        }

        true
    }
}

pub struct VersionDetector {
    signatures: Vec<(SchemaVersion, FieldSignature)>,
}

impl VersionDetector {
    #[must_use]
    pub fn new() -> Self {
        Self::with_signatures(Self::default_signatures())
    }

    #[must_use]
    pub fn with_signatures(signatures: Vec<(SchemaVersion, FieldSignature)>) -> Self {
        Self { signatures }
    }

    fn default_signatures() -> Vec<(SchemaVersion, FieldSignature)> {
        vec![
            (
                SchemaVersion::new(1),
                FieldSignature {
                    required_fields: vec![
                        "active_server",
                        "toggle_mode",
                        "click_mode",
                        "toggle_hotkey",
                        "left_hotkey",
                        "right_hotkey",
                    ],
                    absent_fields: vec!["auto_update_check"],
                },
            ),
            (
                SchemaVersion::new(2),
                FieldSignature {
                    required_fields: vec![
                        "active_server",
                        "toggle_mode",
                        "click_mode",
                        "toggle_hotkey",
                        "left_hotkey",
                        "right_hotkey",
                        "auto_update_check",
                    ],
                    absent_fields: vec![],
                },
            ),
        ]
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn detect(&self, value: &Value) -> MigrationResult<SchemaVersion> {
        if let Some(version) = value.get("schema_version")
            && let Some(v) = version.as_u64()
        {
            return Ok(SchemaVersion::new(v as u32));
        }

        let obj = value.as_object().ok_or_else(|| {
            MigrationError::corrupted("config", "Expected JSON object at root, found other type")
        })?;

        for (version, signature) in self.signatures.iter().rev() {
            if signature.matches(obj) {
                return Ok(*version);
            }
        }

        if obj.is_empty() {
            return Err(MigrationError::corrupted(
                "config",
                "Empty configuration object",
            ));
        }

        let has_any_known_field = obj.contains_key("active_server")
            || obj.contains_key("toggle_mode")
            || obj.contains_key("click_mode");

        if !has_any_known_field {
            return Err(MigrationError::corrupted(
                "config",
                "No recognized configuration fields found",
            ));
        }

        Err(MigrationError::version_error(
            "Could not determine schema version from field structure",
        ))
    }

    #[must_use]
    pub fn needs_migration(&self, detected: SchemaVersion, target: SchemaVersion) -> bool {
        detected < target
    }
}

impl Default for VersionDetector {
    fn default() -> Self {
        Self::new()
    }
}
