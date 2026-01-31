use crate::core::{RacError, RacResult};
use crate::update::security::{
    base64_encode, check_path_for_reparse_points, copy_file, create_dir, file_write_check,
    write_file,
};
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
        create_dir(&backup_dir)?;

        let installer = Self { backup_dir };

        if created_new_dir {
            installer.notify_file_change(&installer.backup_dir);
        }

        Ok(installer)
    }

    pub fn install_update(&self, new_exe_path: &Path) -> RacResult<()> {
        check_path_for_reparse_points(&self.backup_dir)?;

        let current_exe = env::current_exe()
            .map_err(|e| RacError::UpdateError(format!("Cannot get current exe path: {}", e)))?;

        file_write_check(new_exe_path)?;

        if !new_exe_path.exists() {
            return Err(RacError::UpdateError("Update file not found".to_string()));
        }

        println!("Checking for file locks...");
        match crate::update::restart_manager::RestartManager::release_file_locks(&current_exe) {
            Ok(true) => {
                println!("File is ready for update");
            }
            Ok(false) => {
                std::thread::sleep(std::time::Duration::from_secs(5));
            }
            Err(e) => {
                std::thread::sleep(std::time::Duration::from_secs(5));
                return Err(e);
            }
        }

        let backup_path = self.create_backup(&current_exe)?;

        self.create_updater_script(&current_exe, new_exe_path, &backup_path)?;

        Ok(())
    }

    fn create_backup(&self, current_exe: &Path) -> RacResult<PathBuf> {
        check_path_for_reparse_points(&self.backup_dir)?;

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

        copy_file(current_exe, &backup_path)?;

        self.notify_file_change(&backup_path);
        self.cleanup_old_backups()?;

        Ok(backup_path)
    }

    fn validate_path(path: &Path) -> RacResult<String> {
        let path_str = path
            .to_str()
            .ok_or_else(|| RacError::UpdateError("Invalid path encoding (non-UTF8)".to_string()))?;

        const FORBIDDEN_CHARS: &[char] = &[
            '<', '>', '"', '|', '?', '*', '\0', '\n', '\r', '\t', '`', '$', '(', ')', '{', '}',
            ';', '&', '#', '@',
        ];

        const DANGEROUS_UNICODE: &[char] = &[
            '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{FF07}', '\u{FF02}', '\u{0060}',
            '\u{FF04}', '\u{FE69}', '\u{FF1B}', '\u{FF06}',
        ];

        for (idx, c) in path_str.chars().enumerate() {
            if FORBIDDEN_CHARS.contains(&c) {
                return Err(RacError::UpdateError(format!(
                    "Path contains forbidden character at position {}",
                    idx
                )));
            }

            if DANGEROUS_UNICODE.contains(&c) {
                return Err(RacError::UpdateError(format!(
                    "Path contains dangerous Unicode character at position {}",
                    idx
                )));
            }
        }

        for (idx, c) in path_str.chars().enumerate() {
            let is_allowed = c.is_ascii_alphanumeric()
                || matches!(c, '\\' | '/' | ':' | '.' | '-' | '_' | ' ' | '\'' | '~');

            if !is_allowed {
                return Err(RacError::UpdateError(format!(
                    "Path contains disallowed character '{}' at position {}",
                    c, idx
                )));
            }
        }

        if path_str.contains("..") {
            return Err(RacError::UpdateError(
                "Path contains directory traversal sequence '..'".to_string(),
            ));
        }

        if path_str.starts_with("\\\\") {
            return Err(RacError::UpdateError(
                "UNC paths are not permitted for security reasons".to_string(),
            ));
        }

        if path_str.len() > 32767 {
            return Err(RacError::UpdateError(
                "Path exceeds maximum allowed length".to_string(),
            ));
        }

        Ok(path_str.to_string())
    }

    fn validate_filename(name: &str) -> RacResult<()> {
        const ALLOWED_FILENAME_CHARS: &str =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.-_";
        const RESERVED_NAMES: &[&str] = &[
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
        ];

        if name.is_empty() {
            return Err(RacError::UpdateError(
                "Filename cannot be empty".to_string(),
            ));
        }

        for c in name.chars() {
            if !ALLOWED_FILENAME_CHARS.contains(c) {
                return Err(RacError::UpdateError(format!(
                    "Filename contains forbidden character '{}'",
                    c
                )));
            }
        }

        if !name.to_lowercase().ends_with(".exe") {
            return Err(RacError::UpdateError(
                "Update file must have .exe extension".to_string(),
            ));
        }

        let name_without_ext = &name[..name.len() - 4];
        if RESERVED_NAMES.contains(&name_without_ext.to_uppercase().as_str()) {
            return Err(RacError::UpdateError(
                "Filename uses Windows reserved name".to_string(),
            ));
        }

        Ok(())
    }

    #[inline]
    fn escape_ps_single_quote(s: &str) -> String {
        let mut result = String::with_capacity(s.len() + 16);

        for c in s.chars() {
            match c {
                '\'' => result.push_str("''"),
                '$' => result.push_str("`$"),
                '`' => result.push_str("``"),
                _ => result.push(c),
            }
        }

        result
    }

    fn create_updater_script(
        &self,
        current_exe: &Path,
        new_exe: &Path,
        backup_path: &Path,
    ) -> RacResult<()> {
        let script_path = self.backup_dir.join("updater.ps1");

        file_write_check(&script_path)?;

        let current_exe_str = Self::validate_path(current_exe)?;
        let new_exe_str = Self::validate_path(new_exe)?;
        let backup_str = Self::validate_path(backup_path)?;

        let new_exe_name = new_exe
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| RacError::UpdateError("Invalid filename".to_string()))?;

        Self::validate_filename(new_exe_name)?;

        let current_dir = current_exe
            .parent()
            .ok_or_else(|| RacError::UpdateError("Cannot get current exe directory".to_string()))?;

        let new_target_path = current_dir.join(new_exe_name);
        let new_target_str = Self::validate_path(&new_target_path)?;

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
            new_exe = Self::escape_ps_single_quote(&new_exe_str),
            current_exe = Self::escape_ps_single_quote(&current_exe_str),
            backup = Self::escape_ps_single_quote(&backup_str),
            new_target = Self::escape_ps_single_quote(&new_target_str)
        );

        write_file(&script_path, script.as_bytes())?;

        let script_path_str = Self::validate_path(&script_path)?;

        let launcher_command = format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; & '{}'",
            Self::escape_ps_single_quote(&script_path_str)
        );

        let utf16_bytes: Vec<u8> = launcher_command
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();

        let base64_command = base64_encode(&utf16_bytes);

        let result = Command::new("powershell.exe")
            .args([
                "-WindowStyle",
                "Hidden",
                "-ExecutionPolicy",
                "Bypass",
                "-NoProfile",
                "-NonInteractive",
                "-InputFormat",
                "None",
                "-OutputFormat",
                "Text",
                "-EncodedCommand",
                &base64_command,
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
