pub mod checker;
pub mod downloader;
pub mod installer;
pub mod version;

pub use checker::ReleaseInfo;
pub use downloader::ProgressCallback;
pub use version::Version;

use crate::core::RacResult;
use checker::UpdateChecker;
use downloader::Downloader;
use installer::UpdateInstaller;
use std::path::PathBuf;

struct TempFileGuard {
    path: PathBuf,
    keep: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }

    fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.keep && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[derive(Clone)]
pub struct UpdateManager {
    checker: UpdateChecker,
    downloader: Downloader,
    installer: UpdateInstaller,
}

impl UpdateManager {
    pub fn new() -> RacResult<Self> {
        Ok(Self {
            checker: UpdateChecker::new("dhgatjeye", "rac-clicker"),
            downloader: Downloader::new(),
            installer: UpdateInstaller::new()?,
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

        let temp_dir = std::env::temp_dir().join("rac-update");
        if !temp_dir.exists() {
            std::fs::create_dir_all(&temp_dir).map_err(|e| {
                crate::core::RacError::UpdateError(format!("Failed to create temp dir: {}", e))
            })?;
        }

        let download_path = temp_dir.join(format!("rac-clicker-v{}.exe", release.version));

        let mut temp_guard = TempFileGuard::new(download_path.clone());

        println!("Downloading from: {}", release.download_url);
        self.downloader
            .download(&release.download_url, temp_guard.path(), progress_callback)?;

        println!("✓ Download complete!");

        println!("Installing update...");
        self.installer.install_update(temp_guard.path())?;

        temp_guard.keep();

        Ok(())
    }
}
