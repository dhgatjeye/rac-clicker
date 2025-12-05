use crate::{RacError, RacResult, UpdateManager, Version};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

struct AnimationGuard {
    flag: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl AnimationGuard {
    fn start() -> Self {
        let flag = Arc::new(AtomicBool::new(true));
        let flag_clone = Arc::clone(&flag);

        let handle = thread::spawn(move || {
            run_loading_animation(&flag_clone);
        });

        Self {
            flag,
            handle: Some(handle),
        }
    }

    fn stop(&self) {
        self.flag.store(false, Ordering::Release);
    }
}

impl Drop for AnimationGuard {
    fn drop(&mut self) {
        self.stop();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_loading_animation(running: &AtomicBool) {
    while running.load(Ordering::Acquire) {
        for _ in 0..3 {
            if !running.load(Ordering::Acquire) {
                return;
            }
            print!(".");
            io::stdout().flush().ok();
            thread::sleep(Duration::from_millis(300));
        }
        if running.load(Ordering::Acquire) {
            print!("\rChecking for updates   ");
            io::stdout().flush().ok();
        }
    }
}

pub fn check_and_update() -> RacResult<()> {
    print!("Checking for updates");
    io::stdout().flush().ok();

    let animation = AnimationGuard::start();
    let result = perform_update_check(&animation);
    drop(animation);

    result
}

fn perform_update_check(animation: &AnimationGuard) -> RacResult<()> {
    let update_mgr = match UpdateManager::new() {
        Ok(mgr) => mgr,
        Err(e) => {
            animation.stop();
            thread::sleep(Duration::from_millis(50));
            println!("\r   Could not initialize update system: {}        ", e);
            println!("   Starting RAC normally...\n");
            thread::sleep(Duration::from_millis(800));
            return Ok(());
        }
    };

    match update_mgr.check_for_updates() {
        Ok(Some(release)) => {
            animation.stop();
            thread::sleep(Duration::from_millis(50));

            println!("\r                                            ");
            println!("\n╔══════════════════════════════════════════╗");
            println!("║           NEW UPDATE AVAILABLE!            ║");
            println!("╚════════════════════════════════════════════╝");
            println!("\nCurrent Version:  v{}", Version::current());
            println!("New Version:      v{}", release.version);
            println!("Release Name:     {}", release.release_name);
            println!(
                "File Size:        {:.2} MB",
                release.asset_size as f64 / 1024.0 / 1024.0
            );

            if !release.release_notes.is_empty() {
                println!("\nRelease Notes:");
                println!("─────────────────────────────────────────────");
                println!("{}", release.release_notes);
                println!("─────────────────────────────────────────────");
            }

            println!("\nInstall update? [Y/n]: ");
            print!("> ");
            io::stdout().flush().ok();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_ok() {
                let answer = input.trim().to_lowercase();

                if answer == "y" || answer == "yes" || answer.is_empty() {
                    return download_and_install_update(&update_mgr, &release);
                }
                println!("\n⏭ Update skipped. Starting RAC...\n");
                thread::sleep(Duration::from_millis(800));
            }
        }
        Ok(None) => {
            animation.stop();
            thread::sleep(Duration::from_millis(50));
            println!("\rYou're up to date! (v{})        ", Version::current());
            thread::sleep(Duration::from_millis(500));
            println!();
        }
        Err(e) => {
            animation.stop();
            thread::sleep(Duration::from_millis(50));
            println!("\r  Could not check for updates: {}        ", e);
            println!("   Starting RAC normally...\n");
            thread::sleep(Duration::from_millis(800));
        }
    }

    Ok(())
}

fn download_and_install_update(
    update_mgr: &UpdateManager,
    release: &crate::ReleaseInfo,
) -> RacResult<()> {
    println!("\nDownloading update...\n");

    let progress_cb = Arc::new(|current: u64, total: u64| {
        if total > 0 {
            let percent = (current as f64 / total as f64) * 100.0;
            let mb_current = current as f64 / 1024.0 / 1024.0;
            let mb_total = total as f64 / 1024.0 / 1024.0;
            print!(
                "\rDownloading: {:.1}% ({:.2}/{:.2} MB)   ",
                percent, mb_current, mb_total
            );
            io::stdout().flush().ok();
        }
    });

    match update_mgr.download_and_install(release, Some(progress_cb)) {
        Ok(_) => {
            println!("\n\nUpdate downloaded successfully!");
            println!("Restarting application...\n");
            thread::sleep(Duration::from_secs(1));
            Ok(())
        }
        Err(RacError::UpdateRestart) => {
            println!("\n\nUpdate installed successfully!");
            println!("Restarting application...\n");
            thread::sleep(Duration::from_secs(1));
            Err(RacError::UpdateRestart)
        }
        Err(e) => {
            println!("\n\nUpdate failed: {}", e);
            println!("RAC will continue with current version.\n");
            thread::sleep(Duration::from_secs(2));
            Ok(())
        }
    }
}