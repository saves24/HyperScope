use crate::{format_date, format_time, now_unix};
// Daily-rotation logging with periodic cleanup
use std::fs;
use std::time::Duration;

// Log config (cfg for cross-platform)
#[cfg(target_os = "linux")]
pub const LOG_DIR: &str = "/var/log/hyper-panel";
#[cfg(target_os = "windows")]
pub const LOG_DIR: &str = "C:\\ProgramData\\hyper-panel\\logs";
#[cfg(target_os = "linux")]
pub const LOG_CONFIG: &str = "/etc/hyper-panel/log.conf";
#[cfg(target_os = "windows")]
pub const LOG_CONFIG: &str = "C:\\ProgramData\\hyper-panel\\log.conf";
pub const DEFAULT_RETENTION_DAYS: u64 = 7;

pub fn log_write(level: &str, msg: &str) {
    let _ = fs::create_dir_all(LOG_DIR);
    let path = format!("{LOG_DIR}/hyper-panel-{}.log", format_date(now_unix()));
    let line = format!("[{}] [{}] {}\n", format_time(now_unix()), level, msg);
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        let _ = f.write_all(line.as_bytes());
    }
    cleanup_logs();
}
pub fn get_retention_days() -> u64 {
    fs::read_to_string(LOG_CONFIG)
        .ok()
        .and_then(|c| c.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_RETENTION_DAYS)
}
pub fn set_retention_days(days: u64) -> Result<(), String> {
    let _ = fs::create_dir_all("/etc/hyper-panel");
    fs::write(LOG_CONFIG, format!("{days}\n")).map_err(|e| format!("failed to write config: {e}"))
}
pub(crate) static LAST_LOG_CLEAN: std::sync::Mutex<Option<std::time::Instant>> =
    std::sync::Mutex::new(None);

pub fn cleanup_logs() {
    // Run cleanup at most once per minute
    {
        let mut last = LAST_LOG_CLEAN.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_secs(60) {
                return;
            }
        }
        *last = Some(std::time::Instant::now());
    }
    let retention = get_retention_days();
    let cutoff = now_unix().saturating_sub(retention * 86400);
    let cutoff_date = format_date(cutoff);
    if let Ok(entries) = fs::read_dir(LOG_DIR) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some(date) = name
                .strip_prefix("hyper-panel-")
                .and_then(|s| s.strip_suffix(".log"))
            {
                if date.len() == 10 && date < cutoff_date.as_str() {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
    }
}
pub fn tail_log(lines: usize) -> String {
    let today = format!("{LOG_DIR}/hyper-panel-{}.log", format_date(now_unix()));
    let content = fs::read_to_string(&today).unwrap_or_default();
    let all: Vec<&str> = content.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].join("\n")
}
