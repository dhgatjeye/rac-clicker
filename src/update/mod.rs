pub mod version;
pub mod checker;
pub mod downloader;
pub mod installer;

pub use version::{Version, VersionError};
pub use checker::{UpdateChecker, ReleaseInfo};
pub use downloader::{Downloader, ProgressCallback, verify_checksum};
pub use installer::UpdateInstaller;

use crate::core::{RacResult, RacError};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub struct UpdateManager {
    checker: UpdateChecker,
    downloader: Downloader,
    installer: UpdateInstaller,
    auto_check_enabled: Arc<AtomicBool>,
}

impl UpdateManager {
    pub fn new() -> RacResult<Self> {
        Ok(Self {
            checker: UpdateChecker::new("dhgatjeye", "rac-clicker"),
            downloader: Downloader::new(),
            installer: UpdateInstaller::new()?,
            auto_check_enabled: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn check_for_updates(&self) -> RacResult<Option<ReleaseInfo>> {
        self.checker.check_for_updates()
    }

    pub fn download_and_install(
        &self,
        release: &ReleaseInfo,
        progress_callback: Option<ProgressCallback>,
    ) -> RacResult<()> {
        println!("\nDownloading update v{}...", release.version);
        
        let version_str = release.version.to_string();
        if !version_str.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err(RacError::UpdateError("Invalid version format".to_string()));
        }

        let temp_dir = std::env::temp_dir().join("rac-update");
        if !temp_dir.exists() {
            std::fs::create_dir_all(&temp_dir)
                .map_err(|e| RacError::UpdateError(format!("Failed to create temp dir: {}", e)))?;
        }

        let download_path = temp_dir.join(format!("rac-clicker-v{}.exe", version_str));
        
        if !release.download_url.starts_with("https://github.com/") &&
           !release.download_url.starts_with("https://objects.githubusercontent.com/") {
            return Err(RacError::UpdateError("Invalid download URL: must be from GitHub".to_string()));
        }

        println!("Downloading from: {}", release.download_url);
        let downloaded = self.downloader.download(
            &release.download_url,
            &download_path,
            progress_callback,
        )?;

        println!("✓ Download complete!");

        if let Some(ref checksum) = release.checksum {
            println!("Verifying checksum...");
            if !verify_checksum(&downloaded, checksum)? {
                std::fs::remove_file(&downloaded).ok();
                return Err(RacError::UpdateError("Checksum verification failed!".to_string()));
            }
            println!("✓ Checksum verified!");
        }

        println!("Installing update...");
        self.installer.install_update(&downloaded)?;

        Ok(())
    }

    pub fn auto_update(&self, silent: bool) -> RacResult<bool> {
        match self.check_for_updates()? {
            Some(release) => {
                if !silent {
                    println!("\nNew version available: v{}", release.version);
                    println!("Current version: v{}", Version::current());
                    println!("\nRelease Notes:");
                    println!("{}", release.release_notes);
                    println!("\nDownload size: {:.2} MB", release.asset_size as f64 / 1024.0 / 1024.0);

                    print!("\nDo you want to update now? [Y/n]: ");
                    use std::io::{self, Write};
                    io::stdout().flush().ok();

                    let mut input = String::new();
                    io::stdin().read_line(&mut input).ok();
                    let input = input.trim().to_lowercase();

                    if input == "n" || input == "no" {
                        println!("Update cancelled.");
                        return Ok(false);
                    }
                }

                let progress_cb = if !silent {
                    Some(Arc::new(|current: u64, total: u64| {
                        if total > 0 {
                            let percent = (current as f64 / total as f64) * 100.0;
                            print!("\rProgress: {:.1}% ({:.2}/{:.2} MB)   ",
                                percent,
                                current as f64 / 1024.0 / 1024.0,
                                total as f64 / 1024.0 / 1024.0
                            );
                            use std::io::{self, Write};
                            io::stdout().flush().ok();
                        }
                    }) as ProgressCallback)
                } else {
                    None
                };

                self.download_and_install(&release, progress_cb)?;
                Ok(true)
            }
            None => {
                if !silent {
                    println!("✓ You are running the latest version (v{})", Version::current());
                }
                Ok(false)
            }
        }
    }

    pub fn start_background_checker(&self, check_interval: Duration) {
        let checker = self.checker.clone();
        let enabled = Arc::clone(&self.auto_check_enabled);

        enabled.store(true, Ordering::Release);

        thread::spawn(move || {
            while enabled.load(Ordering::Acquire) {
                thread::sleep(check_interval);

                if !enabled.load(Ordering::Acquire) {
                    break;
                }

                if let Ok(Some(release)) = checker.check_for_updates() {
                    println!("\nUpdate available: v{}", release.version);
                    println!("Run update command to install.");
                }
            }
        });
    }

    pub fn stop_background_checker(&self) {
        self.auto_check_enabled.store(false, Ordering::Release);
    }

    pub fn rollback(&self) -> RacResult<()> {
        println!("🔄 Rolling back to previous version...");
        let backup = self.installer.rollback_to_backup()?;
        println!("✓ Rolled back to: {}", backup.display());
        println!("⚠ Please restart RAC to complete rollback.");
        Ok(())
    }

    pub fn current_version(&self) -> Version {
        Version::current()
    }

    pub fn list_backups(&self) -> RacResult<Vec<PathBuf>> {
        self.installer.list_backups()
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new().expect("Failed to create UpdateManager")
    }
}

impl Clone for UpdateChecker {
    fn clone(&self) -> Self {
        Self {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
            current_version: self.current_version,
        }
    }
}