// Basic utilities shared across modules
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn read_file(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("failed to read {path}: {e}"))
}
pub(crate) fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
pub(crate) fn format_date(secs: u64) -> String {
    // Simplified date format (UTC): YYYY-MM-DD
    let days = secs / 86400;
    // Compute from 1970-01-01 (ignore timezone; logs rotate by UTC date)
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}
pub(crate) fn format_time(secs: u64) -> String {
    let days = secs / 86400;
    let rem = secs % 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}")
}
pub(crate) fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

// Cross-platform file permission setter (no-op on Windows)
#[cfg(target_os = "linux")]
pub(crate) fn chmod(path: &str, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}
#[cfg(target_os = "windows")]
pub(crate) fn chmod(_path: &str, _mode: u32) {}
