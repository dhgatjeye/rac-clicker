use crate::config::constants::defaults;
use std::fs;
use std::io::{self};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct LogCleaner {
    max_size_bytes: usize,
    check_interval_secs: u64,
    running: Arc<AtomicBool>,
}

impl LogCleaner {
    pub fn new(max_size_bytes: usize, check_interval_secs: u64) -> Self {
        Self {
            max_size_bytes,
            check_interval_secs,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }

        self.running.store(true, Ordering::Relaxed);

        let max_size = self.max_size_bytes;
        let interval = self.check_interval_secs;
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            if let Err(e) = Self::clean_logs(max_size) {
                eprintln!("Error cleaning logs: {}", e);
            }

            while running.load(Ordering::Relaxed) {
                for _ in 0..interval {
                    if !running.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(Duration::from_secs(1));
                }

                if let Err(e) = Self::clean_logs(max_size) {
                    eprintln!("Error cleaning logs: {}", e);
                }
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    fn clean_logs(max_size: usize) -> io::Result<()> {
        let log_path = Self::get_log_file_path()?;

        if !log_path.exists() {
            return Ok(());
        }

        let metadata = fs::metadata(&log_path)?;

        if metadata.len() > max_size as u64 {
            fs::write(&log_path, "--- Log file cleaned due to size limit ---\n")?;
        }

        Ok(())
    }

    fn get_log_file_path() -> io::Result<PathBuf> {
        let local_app_data = dirs::data_local_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find AppData/Local directory"))?;

        let log_path = local_app_data.join(defaults::RAC_DIR).join(defaults::RAC_LOG_PATH);
        Ok(log_path)
    }
}

impl Drop for LogCleaner {
    fn drop(&mut self) {
        self.stop();
    }
}
