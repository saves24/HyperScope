use crate::{
    format_date, format_time, now_unix, read_file, CONFIG_FILE, DEFAULT_RETENTION_DAYS, KEY_DIR,
    LOG_DIR,
};
// Daily-rotation logging with periodic cleanup
use std::fs;
use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub(crate) fn log_write(level: &str, msg: &str) {
    let _ = fs::create_dir_all(LOG_DIR);
    let date = format_date(now_unix());
    let path = format!("{LOG_DIR}/hyper-node-{date}.log");
    let line = format!("[{}] [{}] {}\n", format_time(now_unix()), level, msg);
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
    // Run cleanup after each log write
    cleanup_logs();
}
pub(crate) fn get_retention_days() -> u64 {
    read_file(CONFIG_FILE)
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("retention="))
                .and_then(|l| l.split('=').nth(1).and_then(|v| v.trim().parse().ok()))
        })
        .unwrap_or(DEFAULT_RETENTION_DAYS)
}
pub(crate) fn set_retention_days(days: u64) -> Result<(), String> {
    let _ = fs::create_dir_all(KEY_DIR);
    let content = format!("retention={days}\n");
    fs::write(CONFIG_FILE, content).map_err(|e| format!("failed to write config: {e}"))
}
pub(crate) static LAST_LOG_CLEAN: Mutex<Option<Instant>> = Mutex::new(None);

pub(crate) fn cleanup_logs() {
    // Run cleanup at most once per minute
    {
        let mut last = LAST_LOG_CLEAN.lock().unwrap();
        if let Some(t) = *last {
            if t.elapsed() < Duration::from_secs(60) {
                return;
            }
        }
        *last = Some(Instant::now());
    }
    let retention = get_retention_days();
    let cutoff = now_unix().saturating_sub(retention * 86400);
    let cutoff_date = format_date(cutoff);
    if let Ok(entries) = fs::read_dir(LOG_DIR) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            // Filenames look like hyper-node-YYYY-MM-DD.log
            if let Some(date) = name
                .strip_prefix("hyper-node-")
                .and_then(|s| s.strip_suffix(".log"))
            {
                if date.len() == 10 && date < cutoff_date.as_str() {
                    let _ = fs::remove_file(e.path());
                }
            }
        }
    }
}
