use crate::core::{RacResult, RacError};
use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use std::env;

pub struct UpdateInstaller {
    backup_dir: PathBuf,
}

impl UpdateInstaller {
    pub fn new() -> RacResult<Self> {
        let local_appdata = env::var("LOCALAPPDATA")
            .map_err(|e| RacError::UpdateError(format!("Cannot find LOCALAPPDATA: {}", e)))?;

        let backup_dir = PathBuf::from(local_appdata)
            .join("RAC")
            .join("backups");

        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir)
                .map_err(|e| RacError::UpdateError(format!("Failed to create backup dir: {}", e)))?;
        }

        Ok(Self { backup_dir })
    }
    
    pub fn install_update(&self, new_exe_path: &Path) -> RacResult<()> {
        let current_exe = env::current_exe()
            .map_err(|e| RacError::UpdateError(format!("Cannot get current exe path: {}", e)))?;
        
        if !new_exe_path.exists() {
            return Err(RacError::UpdateError("Update file not found".to_string()));
        }
        
        let backup_path = self.create_backup(&current_exe)?;
        
        self.create_updater_script(&current_exe, new_exe_path, &backup_path)?;

        Ok(())
    }
    
    fn create_backup(&self, current_exe: &Path) -> RacResult<PathBuf> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let exe_name = current_exe
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| RacError::UpdateError("Invalid executable name".to_string()))?;

        let backup_path = self.backup_dir.join(format!("{}.backup.{}", exe_name, timestamp));

        fs::copy(current_exe, &backup_path)
            .map_err(|e| RacError::UpdateError(format!("Failed to create backup: {}", e)))?;
        
        self.cleanup_old_backups()?;

        Ok(backup_path)
    }
    
    fn create_updater_script(
        &self,
        current_exe: &Path,
        new_exe: &Path,
        backup_path: &Path,
    ) -> RacResult<()> {
        let script_path = self.backup_dir.join("updater.ps1");

        let current_exe_str = current_exe.to_str()
            .ok_or_else(|| RacError::UpdateError("Invalid current exe path".to_string()))?;
        let new_exe_str = new_exe.to_str()
            .ok_or_else(|| RacError::UpdateError("Invalid new exe path".to_string()))?;
        let backup_str = backup_path.to_str()
            .ok_or_else(|| RacError::UpdateError("Invalid backup path".to_string()))?;

        let new_exe_name = new_exe.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("rac-clicker.exe");
        
        let current_dir = current_exe.parent()
            .ok_or_else(|| RacError::UpdateError("Cannot get current exe directory".to_string()))?;
        
        let new_target_path = current_dir.join(new_exe_name);
        let new_target_str = new_target_path.to_str()
            .ok_or_else(|| RacError::UpdateError("Invalid new target path".to_string()))?;
        
        let script = format!(r#"
# RAC Auto-Update Script
# Wait for main process to exit
Start-Sleep -Seconds 2

$maxRetries = 10
$retryCount = 0

# Wait until file is not locked and copy to new versioned filename
while ($retryCount -lt $maxRetries) {{
    try {{
        # Copy to new versioned filename
        Copy-Item -Path "{new_exe}" -Destination "{new_target}" -Force
        break
    }}
    catch {{
        Write-Host "Waiting for file lock to release... ($retryCount/$maxRetries)"
        Start-Sleep -Seconds 1
        $retryCount++
    }}
}}

if ($retryCount -ge $maxRetries) {{
    # Rollback on failure
    Write-Host "Update failed! Rolling back..."
    Copy-Item -Path "{backup}" -Destination "{current_exe}" -Force
    Write-Host "Rollback complete. Press any key to exit..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    exit 1
}}

# If old exe has different name than new one, remove old exe
if ("{current_exe}" -ne "{new_target}") {{
    Remove-Item -Path "{current_exe}" -Force -ErrorAction SilentlyContinue
}}

# Cleanup downloaded temp file
Remove-Item -Path "{new_exe}" -Force -ErrorAction SilentlyContinue
Remove-Item -Path $PSCommandPath -Force -ErrorAction SilentlyContinue

# Restart application with new versioned filename
Start-Process -FilePath "{new_target}"

Write-Host "Update complete!"
exit 0
"#, 
            new_exe = new_exe_str, 
            current_exe = current_exe_str, 
            backup = backup_str,
            new_target = new_target_str
        );

        fs::write(&script_path, script)
            .map_err(|e| RacError::UpdateError(format!("Failed to write updater script: {}", e)))?;
        
        let result = Command::new("powershell.exe")
            .args(&[
                "-WindowStyle", "Hidden",
                "-ExecutionPolicy", "Bypass",
                "-File", script_path.to_str().unwrap_or(""),
            ])
            .spawn();

        match result {
            Ok(_) => {
                std::process::exit(0);
            }
            Err(e) => Err(RacError::UpdateError(format!("Failed to launch updater: {}", e)))
        }
    }
    
    fn cleanup_old_backups(&self) -> RacResult<()> {
        let mut backups: Vec<_> = fs::read_dir(&self.backup_dir)
            .map_err(|e| RacError::UpdateError(format!("Failed to read backup dir: {}", e)))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.contains(".backup."))
                    .unwrap_or(false)
            })
            .collect();
        
        backups.sort_by_key(|entry| {
            entry.metadata()
                .and_then(|m| m.modified())
                .ok()
        });
        backups.reverse();

        for backup in backups.iter().skip(1) {
            let _ = fs::remove_file(backup.path());
        }

        Ok(())
    }
    
    pub fn rollback_to_backup(&self) -> RacResult<PathBuf> {
        let current_exe = env::current_exe()
            .map_err(|e| RacError::UpdateError(format!("Cannot get current exe: {}", e)))?;
        
        let latest_backup = fs::read_dir(&self.backup_dir)
            .map_err(|e| RacError::UpdateError(format!("Failed to read backups: {}", e)))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.contains(".backup."))
                    .unwrap_or(false)
            })
            .max_by_key(|entry| {
                entry.metadata()
                    .and_then(|m| m.modified())
                    .ok()
            })
            .ok_or_else(|| RacError::UpdateError("No backup found".to_string()))?;

        let backup_path = latest_backup.path();
        
        fs::copy(&backup_path, &current_exe)
            .map_err(|e| RacError::UpdateError(format!("Failed to restore backup: {}", e)))?;

        Ok(backup_path)
    }

    pub fn list_backups(&self) -> RacResult<Vec<PathBuf>> {
        let mut backups: Vec<_> = fs::read_dir(&self.backup_dir)
            .map_err(|e| RacError::UpdateError(format!("Failed to read backups: {}", e)))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.contains(".backup."))
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .collect();

        backups.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
        });
        backups.reverse();

        Ok(backups)
    }
}

impl Default for UpdateInstaller {
    fn default() -> Self {
        Self::new().expect("Failed to create UpdateInstaller")
    }
}