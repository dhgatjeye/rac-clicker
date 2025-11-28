use crate::config::constants::defaults;
use crate::helper::windows_paths::get_local_appdata;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum LogLevel {
    Info,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Error => "ERROR"
        }
    }
}

static LOGGER: LazyLock<Mutex<Logger>> = LazyLock::new(|| Mutex::new(Logger::new()));

pub struct Logger {
    log_file: PathBuf,
}

impl Logger {
    fn new() -> Self {
        let log_path = get_local_appdata()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(defaults::RAC_DIR)
            .join(defaults::RAC_LOG_PATH);

        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("Failed to create log directory: {}", e);
            });
        }

        Self { log_file: log_path }
    }

    fn write_log(&self, level: LogLevel, message: &str, context: &str) {
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
        {
            let timestamp = Self::format_timestamp();
            let log_entry = format!(
                "[{}] [{}] {} in {}\n{}\n{}\n",
                timestamp,
                level.as_str(),
                message,
                context,
                "-".repeat(80),
                ""
            );

            if let Err(e) = file.write_all(log_entry.as_bytes()) {
                eprintln!("Failed to write log: {}", e);
            }
        }
    }

    fn format_timestamp() -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let total_secs = now.as_secs();
        let days_since_epoch = total_secs / 86400;

        let mut year = 1970;
        let mut days_left = days_since_epoch;

        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if days_left < days_in_year {
                break;
            }
            days_left -= days_in_year;
            year += 1;
        }

        let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut month = 1;
        let mut day = days_left + 1;

        for &days in &month_days {
            let days_in_month = if month == 2 && Self::is_leap_year(year) {
                29
            } else {
                days
            };

            if day <= days_in_month {
                break;
            }
            day -= days_in_month;
            month += 1;
        }

        let secs_today = total_secs % 86400;
        let hour = secs_today / 3600;
        let minute = (secs_today % 3600) / 60;
        let second = secs_today % 60;

        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                year, month, day, hour, minute, second)
    }

    fn is_leap_year(year: u64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
}

pub fn log_error(error: &str, context: &str) {
    if let Ok(logger) = LOGGER.lock() {
        logger.write_log(LogLevel::Error, error, context);
    }
}

pub fn log_info(message: &str, context: &str) {
    if let Ok(logger) = LOGGER.lock() {
        logger.write_log(LogLevel::Info, message, context);
    }
}