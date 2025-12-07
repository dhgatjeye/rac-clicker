use crate::{RacError, RacResult, UpdateManager, Version};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

enum DownloadProgress {
    Progress { current: u64, total: u64 },
    Complete,
    Error(String),
}

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
            if let Err(_e) = handle.join() {
                #[cfg(debug_assertions)]
                eprintln!("Animation thread panicked: {:?}", _e);
            }
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
    
    let (tx, rx) = mpsc::channel();

    let _check_handle = thread::Builder::new()
        .name("UpdateChecker".to_string())
        .spawn(move || {
            let result = perform_update_check_background();
            let _ = tx.send(result);
        })
        .map_err(|e| RacError::ThreadError(format!("Failed to spawn update checker: {}", e)))?;
    
    let result = match rx.recv() {
        Ok(check_result) => check_result,
        Err(_) => {
            animation.stop();
            thread::sleep(Duration::from_millis(50));
            println!("\r  Update check failed        ");
            println!("   Starting RAC normally...\n");
            thread::sleep(Duration::from_millis(500));
            Ok(())
        }
    };
    
    drop(animation);
    
    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            println!("\r  Could not check for updates: {}        ", e);
            println!("   Starting RAC normally...\n");
            thread::sleep(Duration::from_millis(800));
            Ok(())
        }
    }
}

fn perform_update_check_background() -> RacResult<()> {
    let update_mgr = match UpdateManager::new() {
        Ok(mgr) => mgr,
        Err(e) => {
            return Err(RacError::UpdateError(format!("Cannot initialize update system: {}", e)));
        }
    };

    match update_mgr.check_for_updates() {
        Ok(Some(release)) => {
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
            thread::sleep(Duration::from_millis(50));
            println!("\rYou're up to date! (v{})        ", Version::current());
            thread::sleep(Duration::from_millis(500));
            println!();
        }
        Err(e) => {
            return Err(e);
        }
    }

    Ok(())
}

fn download_and_install_update(
    update_mgr: &UpdateManager,
    release: &crate::ReleaseInfo,
) -> RacResult<()> {
    println!("\nDownloading update in background...\n");
    
    let (progress_tx, progress_rx): (Sender<DownloadProgress>, Receiver<DownloadProgress>) = mpsc::channel();
    
    let release_clone = release.clone();
    let update_mgr_clone = update_mgr.clone();

    let download_handle = thread::Builder::new()
        .name("UpdateDownloader".to_string())
        .spawn(move || {
            let tx = progress_tx.clone();
            let progress_cb = Arc::new(move |current: u64, total: u64| {
                let _ = tx.send(DownloadProgress::Progress { current, total });
            });
            
            let result = update_mgr_clone.download_and_install(&release_clone, Some(progress_cb));
            
            match &result {
                Ok(_) => {
                    let _ = progress_tx.send(DownloadProgress::Complete);
                }
                Err(e) => {
                    let _ = progress_tx.send(DownloadProgress::Error(format!("{}", e)));
                }
            }
            
            result
        })
        .map_err(|e| RacError::ThreadError(format!("Failed to spawn download thread: {}", e)))?;
    
    loop {
        match progress_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(DownloadProgress::Progress { current, total }) => {
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
            }
            Ok(DownloadProgress::Complete) => {
                break;
            }
            Ok(DownloadProgress::Error(err)) => {
                println!("\n\nUpdate failed: {}", err);
                println!("RAC will continue with current version.\n");
                thread::sleep(Duration::from_secs(2));
                
                if let Err(_e) = download_handle.join() {
                    #[cfg(debug_assertions)]
                    eprintln!("Download thread panicked after error: {:?}", _e);
                }
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                println!("\n\nUpdate communication error!");
                println!("RAC will continue with current version.\n");
                thread::sleep(Duration::from_secs(2));
                return Ok(());
            }
        }
    }
    
    match download_handle.join() {
        Ok(Ok(_)) => {
            println!("\n\nUpdate downloaded successfully!");
            println!("Restarting application...\n");
            thread::sleep(Duration::from_secs(1));
            Ok(())
        }
        Ok(Err(RacError::UpdateRestart)) => {
            println!("\n\nUpdate installed successfully!");
            println!("Restarting application...\n");
            thread::sleep(Duration::from_secs(1));
            Err(RacError::UpdateRestart)
        }
        Ok(Err(e)) => {
            println!("\n\nUpdate failed: {}", e);
            println!("RAC will continue with current version.\n");
            thread::sleep(Duration::from_secs(2));
            Ok(())
        }
        Err(_) => {
            println!("\n\nUpdate thread panicked!");
            println!("RAC will continue with current version.\n");
            thread::sleep(Duration::from_secs(2));
            Ok(())
        }
    }
}
