use crate::core::{ClickMode, RacError, RacResult, ServerType, ToggleMode};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub active_server: ServerType,
    pub toggle_mode: ToggleMode,
    pub click_mode: ClickMode,
    pub toggle_hotkey: i32,
    pub left_hotkey: i32,
    pub right_hotkey: i32,
    pub left_cps_override: u8,
    pub right_cps_override: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            active_server: ServerType::Craftrise,
            toggle_mode: ToggleMode::MouseHold,
            click_mode: ClickMode::Both,
            toggle_hotkey: 0,
            left_hotkey: 0,
            right_hotkey: 0,
            left_cps_override: 0,
            right_cps_override: 0,
        }
    }
}

pub struct SettingsManager {
    settings_path: PathBuf,
}

impl SettingsManager {
    pub fn new() -> RacResult<Self> {
        Self::new_with_path(Self::get_settings_path()?)
    }

    pub fn new_with_path(path: PathBuf) -> RacResult<Self> {
        Ok(Self {
            settings_path: path,
        })
    }

    fn get_settings_path() -> RacResult<PathBuf> {
        let local_appdata = std::env::var("LOCALAPPDATA")
            .map_err(|e| RacError::ConfigError(format!("Cannot find LOCALAPPDATA: {}", e)))?;

        let rac_dir = PathBuf::from(local_appdata).join("RAC");

        if !rac_dir.exists() {
            std::fs::create_dir_all(&rac_dir)?;
        }

        Ok(rac_dir.join("settings_v2.json"))
    }

    pub fn load(&self) -> RacResult<Settings> {
        if !self.settings_path.exists() {
            return Ok(Settings::default());
        }

        let mut file = OpenOptions::new().read(true).open(&self.settings_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        match serde_json::from_str(&contents) {
            Ok(settings) => Ok(settings),
            Err(e) => {
                self.backup_corrupted_file()?;
                eprintln!("Warning: Settings file corrupted: {}. Using defaults.", e);
                Ok(Settings::default())
            }
        }
    }

    fn backup_corrupted_file(&self) -> RacResult<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let backup_path = self
            .settings_path
            .with_extension(format!("json.corrupt.{}", timestamp));

        std::fs::rename(&self.settings_path, &backup_path).map_err(|e| {
            RacError::IoError(format!("Failed to backup corrupted settings: {}", e))
        })?;

        Ok(())
    }

    pub fn save(&self, settings: &Settings) -> RacResult<()> {
        let json = serde_json::to_string_pretty(settings)?;

        let temp_path = self.settings_path.with_extension("tmp");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)?;

        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        drop(file);

        std::fs::rename(&temp_path, &self.settings_path)?;
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.settings_path
    }
}
