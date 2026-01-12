use crate::core::{RacError, RacResult};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use windows::Win32::UI::Shell::{SHCNE_UPDATEDIR, SHCNF_FLAGS, SHChangeNotify};

#[derive(Clone)]
pub struct UpdateInstaller {
    backup_dir: PathBuf,
}

impl UpdateInstaller {
    pub fn new() -> RacResult<Self> {
        let local_appdata = env::var("LOCALAPPDATA")
            .map_err(|e| RacError::UpdateError(format!("Cannot find LOCALAPPDATA: {}", e)))?;

        let backup_dir = PathBuf::from(local_appdata).join("RAC").join("backups");

        let created_new_dir = !backup_dir.exists();
        if created_new_dir {
            fs::create_dir_all(&backup_dir).map_err(|e| {
                RacError::UpdateError(format!("Failed to create backup dir: {}", e))
            })?;
        }

        let installer = Self { backup_dir };

        if created_new_dir {
            installer.notify_file_change(&installer.backup_dir);
        }

        Ok(installer)
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

        let backup_path = self
            .backup_dir
            .join(format!("{}.backup.{}", exe_name, timestamp));

        fs::copy(current_exe, &backup_path)
            .map_err(|e| RacError::UpdateError(format!("Failed to create backup: {}", e)))?;

        self.notify_file_change(&backup_path);
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

        let validate_path = |path: &Path| -> RacResult<String> {
            let path_str = path
                .to_str()
                .ok_or_else(|| RacError::UpdateError("Invalid path encoding".to_string()))?;

            if path_str.contains(|c: char| {
                matches!(
                    c,
                    '"' | '\'' | '`' | ';' | '$' | '&' | '|' | '<' | '>' | '\n' | '\r'
                )
            }) {
                return Err(RacError::UpdateError(
                    "Path contains invalid characters".to_string(),
                ));
            }

            Ok(path_str.to_string())
        };

        let current_exe_str = validate_path(current_exe)?;
        let new_exe_str = validate_path(new_exe)?;
        let backup_str = validate_path(backup_path)?;

        let new_exe_name = new_exe
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| RacError::UpdateError("Invalid filename".to_string()))?;

        if !new_exe_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        {
            return Err(RacError::UpdateError(
                "Filename contains invalid characters".to_string(),
            ));
        }

        if !new_exe_name.ends_with(".exe") {
            return Err(RacError::UpdateError("Invalid file extension".to_string()));
        }

        let current_dir = current_exe
            .parent()
            .ok_or_else(|| RacError::UpdateError("Cannot get current exe directory".to_string()))?;

        let new_target_path = current_dir.join(new_exe_name);
        let new_target_str = validate_path(&new_target_path)?;

        let escape_ps_single_quote = |s: &str| s.replace('\'', "''");

        let script = format!(
            r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Text.Encoding]::UTF8

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Config = @{{
    NewExePath      = '{new_exe}'
    CurrentExePath  = '{current_exe}'
    BackupPath      = '{backup}'
    NewTargetPath   = '{new_target}'
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

function Invoke-ShellNotify {{
    param(
        [string]$Path,
        [switch]$RefreshDesktop
    )

    try {{
        Add-Type -TypeDefinition @'
            using System;
            using System.Runtime.InteropServices;

            public class ShellNotify {{
                [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
                public static extern void SHChangeNotify(uint wEventId, uint uFlags, IntPtr dwItem1, IntPtr dwItem2);

                public const uint SHCNE_UPDATEDIR = 0x00001000;
                public const uint SHCNE_ASSOCCHANGED = 0x08000000;
                public const uint SHCNF_PATHW = 0x0005;
                public const uint SHCNF_IDLIST = 0x0000;
            }}
        '@ -ErrorAction SilentlyContinue

        if ($RefreshDesktop) {{
            [ShellNotify]::SHChangeNotify([ShellNotify]::SHCNE_ASSOCCHANGED, [ShellNotify]::SHCNF_IDLIST, [IntPtr]::Zero, [IntPtr]::Zero)
        }}
        elseif ($Path) {{
            $parentPath = Split-Path -Parent $Path
            if ($parentPath) {{
                $pathPtr = [System.Runtime.InteropServices.Marshal]::StringToHGlobalUni($parentPath)
                [ShellNotify]::SHChangeNotify([ShellNotify]::SHCNE_UPDATEDIR, [ShellNotify]::SHCNF_PATHW, $pathPtr, [IntPtr]::Zero)
                [System.Runtime.InteropServices.Marshal]::FreeHGlobal($pathPtr)
            }}
        }}
    }}
    catch {{
        Write-Log "Shell notification failed: $_" -Level Warning
    }}
}}

function Invoke-Rollback {{
    param([string]$Reason)

    Write-Log "Update failed: $Reason" -Level Error
    Write-Log "Initiating rollback..." -Level Warning

    try {{
        if (Test-Path $Config.BackupPath) {{
            Copy-Item -Path $Config.BackupPath -Destination $Config.CurrentExePath -Force
            Invoke-ShellNotify -Path $Config.CurrentExePath
            Invoke-ShellNotify -RefreshDesktop
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

    $tempDir = Split-Path -Parent $Config.NewExePath
    if ((Test-Path $tempDir) -and ((Get-ChildItem -Path $tempDir -Force | Measure-Object).Count -eq 0)) {{
        try {{
            Remove-Item -Path $tempDir -Force -ErrorAction SilentlyContinue
            Write-Log "Cleaned up temp directory: $tempDir" -Level Info
        }}
        catch {{
            Write-Log "Could not remove temp directory" -Level Warning
        }}
    }}
}}

function Wait-ForProcessToExit {{
    param([string]$ExePath)

    $exeName = [System.IO.Path]::GetFileNameWithoutExtension($ExePath)
    Write-Log "Waiting for $exeName processes to exit..." -Level Info

    $maxWait = 30
    $waited = 0

    while ($waited -lt $maxWait) {{
        $processes = @(Get-Process -Name $exeName -ErrorAction SilentlyContinue)

        if ($processes.Count -eq 0) {{
            Write-Log "All $exeName processes have exited" -Level Success
            return $true
        }}

        Write-Log "Found $($processes.Count) running process(es), waiting..." -Level Info
        Start-Sleep -Seconds 1
        $waited++
    }}

    Write-Log "Timeout waiting for processes to exit. Attempting force kill..." -Level Warning
    Get-Process -Name $exeName -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2

    return $true
}}

try {{
    Write-Log "RAC Auto-Update Process Started" -Level Info

    if (-not (Wait-ForProcessToExit -ExePath $Config.CurrentExePath)) {{
        Invoke-Rollback "Could not terminate old process"
    }}

    Write-Log "Main process terminated successfully" -Level Success
    Start-Sleep -Seconds 1

    if (-not (Test-Path $Config.NewExePath)) {{
        Invoke-Rollback "Source update file not found: $($Config.NewExePath)"
    }}

    Write-Log "Installing new version..." -Level Info
    $retryCount = 0
    $success = $false

    while ($retryCount -lt $Config.MaxRetries) {{
        try {{
            Copy-Item -Path $Config.NewExePath -Destination $Config.NewTargetPath -Force
            Invoke-ShellNotify -Path $Config.NewTargetPath
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
                Invoke-ShellNotify -Path $Config.CurrentExePath
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

    Write-Log "Refreshing explorer..." -Level Info
    Invoke-ShellNotify -RefreshDesktop
    Start-Sleep -Milliseconds 500

    Write-Log "Restarting application..." -Level Info
    Start-Process -FilePath $Config.NewTargetPath

    Write-Log "Update completed successfully!" -Level Success
    Start-Sleep -Seconds 2
    exit 0
}}
catch {{
    Invoke-Rollback $_.Exception.Message
}}
"#,
            new_exe = escape_ps_single_quote(&new_exe_str),
            current_exe = escape_ps_single_quote(&current_exe_str),
            backup = escape_ps_single_quote(&backup_str),
            new_target = escape_ps_single_quote(&new_target_str)
        );

        fs::write(&script_path, script.as_bytes())
            .map_err(|e| RacError::UpdateError(format!("Failed to write updater script: {}", e)))?;

        let script_path_str = script_path
            .to_str()
            .ok_or_else(|| RacError::UpdateError("Invalid script path".to_string()))?;

        let result = Command::new("powershell.exe")
            .args([
                "-WindowStyle",
                "Hidden",
                "-ExecutionPolicy",
                "Bypass",
                "-NoProfile",
                "-InputFormat",
                "None",
                "-OutputFormat",
                "Text",
                "-Command",
                &format!(
                    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; & '{}'",
                    script_path_str.replace('\'', "''")
                ),
            ])
            .spawn();

        match result {
            Ok(_) => Err(RacError::UpdateRestart),
            Err(e) => Err(RacError::UpdateError(format!(
                "Failed to launch updater: {}",
                e
            ))),
        }
    }

    fn cleanup_old_backups(&self) -> RacResult<()> {
        let mut backups: Vec<_> = fs::read_dir(&self.backup_dir)
            .map_err(|e| RacError::UpdateError(format!("Failed to read backup dir: {}", e)))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.contains(".backup."))
                    .unwrap_or(false)
            })
            .collect();

        backups.retain(|entry| entry.metadata().and_then(|m| m.modified()).is_ok());

        backups.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).unwrap());
        backups.reverse();

        for backup in backups.iter().skip(1) {
            if fs::remove_file(backup.path()).is_ok() {
                self.notify_file_change(&backup.path());
            }
        }

        Ok(())
    }

    fn notify_file_change(&self, path: &Path) {
        if let Some(parent) = path.parent()
            && let Some(path_str) = parent.to_str()
        {
            let mut wide: Vec<u16> = path_str.encode_utf16().collect();
            wide.push(0);

            unsafe {
                SHChangeNotify(
                    SHCNE_UPDATEDIR,
                    SHCNF_FLAGS(0x0005),
                    Some(wide.as_ptr() as *const std::ffi::c_void),
                    None,
                );
            }
        }
    }
}
