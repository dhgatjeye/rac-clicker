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
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Config = @{{
    NewExePath      = "{new_exe}"
    CurrentExePath  = "{current_exe}"
    BackupPath      = "{backup}"
    NewTargetPath   = "{new_target}"
    MaxRetries      = 10
    RetryDelay      = 1
    InitialDelay    = 2
}}

function Write-Log {{
    param(
        [string]$Message,
        [ValidateSet('Info', 'Warning', 'Error', 'Success')]
        [string]$Level = 'Info'
    )

    $timestamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
    $colors = @{{
        'Info'    = 'White'
        'Warning' = 'Yellow'
        'Error'   = 'Red'
        'Success' = 'Green'
    }}

    Write-Host "[$timestamp] [$Level] $Message" -ForegroundColor $colors[$Level]
}}

function Invoke-Rollback {{
    param([string]$Reason)

    Write-Log "Update failed: $Reason" -Level Error
    Write-Log "Initiating rollback..." -Level Warning

    try {{
        if (Test-Path $Config.BackupPath) {{
            Copy-Item -Path $Config.BackupPath -Destination $Config.CurrentExePath -Force
            Write-Log "Rollback completed successfully" -Level Success
        }} else {{
            Write-Log "Backup file not found. Manual intervention required." -Level Error
        }}
    }}
    catch {{
        Write-Log "Rollback failed: $_" -Level Error
    }}

    Write-Host "`nPress any key to exit..." -ForegroundColor Yellow
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    exit 1
}}

function Remove-UpdateFiles {{
    $filesToClean = @(
        $Config.NewExePath,
        $Config.BackupPath,
        $PSCommandPath
    )

    foreach ($file in $filesToClean) {{
        if (Test-Path $file) {{
            try {{
                Remove-Item -Path $file -Force -ErrorAction SilentlyContinue
                Write-Log "Cleaned up: $file" -Level Info
            }}
            catch {{
                Write-Log "Could not remove: $file" -Level Warning
            }}
        }}
    }}
}}

try {{
    Write-Log "RAC Auto-Update Process Started" -Level Info
    Write-Log "Waiting for main process to exit..." -Level Info
    Start-Sleep -Seconds $Config.InitialDelay

    # Validate source file exists
    if (-not (Test-Path $Config.NewExePath)) {{
        Invoke-Rollback "Source update file not found: $($Config.NewExePath)"
    }}

    # Attempt to copy new version
    Write-Log "Installing new version..." -Level Info
    $retryCount = 0
    $success = $false

    while ($retryCount -lt $Config.MaxRetries) {{
        try {{
            Copy-Item -Path $Config.NewExePath -Destination $Config.NewTargetPath -Force
            $success = $true
            Write-Log "New version installed successfully" -Level Success
            break
        }}
        catch {{
            $retryCount++
            if ($retryCount -lt $Config.MaxRetries) {{
                Write-Log "Retry $retryCount/$($Config.MaxRetries): Waiting for file lock to release..." -Level Warning
                Start-Sleep -Seconds $Config.RetryDelay
            }}
        }}
    }}

    if (-not $success) {{
        Invoke-Rollback "Failed to install new version after $($Config.MaxRetries) attempts"
    }}

    if ($Config.CurrentExePath -ne $Config.NewTargetPath) {{
        if (Test-Path $Config.CurrentExePath) {{
            try {{
                Remove-Item -Path $Config.CurrentExePath -Force
                Write-Log "Removed old executable: $($Config.CurrentExePath)" -Level Info
            }}
            catch {{
                Write-Log "Could not remove old executable (non-critical): $_" -Level Warning
            }}
        }}
    }}

    Write-Log "Cleaning up temporary files..." -Level Info
    Remove-UpdateFiles

    if (-not (Test-Path $Config.NewTargetPath)) {{
        throw "New executable not found at expected location: $($Config.NewTargetPath)"
    }}

    Write-Log "Restarting application..." -Level Info
    Start-Process -FilePath $Config.NewTargetPath

    Write-Log "Update completed successfully!" -Level Success
    Start-Sleep -Seconds 2
    exit 0
}}
catch {{
    Invoke-Rollback $_.Exception.Message
}}
"#, new_exe = new_exe_str, current_exe = current_exe_str, backup = backup_str, new_target = new_target_str);

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