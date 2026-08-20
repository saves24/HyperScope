// Disk I/O rate and TCP connection count
use serde_json::{json, Value};
use std::fs;
use std::time::Duration;

// Sampling interval
const IO_SAMPLE_MS: u64 = 500;

// Read /proc/diskstats fields for a device (field numbers per /proc/diskstats docs)
// read sectors = field 6, write sectors = field 10; sector = 512 bytes
fn read_disk_sectors(device: &str) -> Option<(u64, u64)> {
    let content = fs::read_to_string("/proc/diskstats").ok()?;
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Format: major minor name ... read_completed read_merged read_sectors read_ms ... write_completed write_merged write_sectors write_ms ...
        if parts.len() >= 14 && parts.get(2).copied() == Some(device) {
            let read_sectors: u64 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
            let write_sectors: u64 = parts.get(9).and_then(|s| s.parse().ok()).unwrap_or(0);
            return Some((read_sectors, write_sectors));
        }
    }
    None
}

// Auto-detect main disk device (skip partitions; prefer mmcblk0/nvme0n1/sda/vda/xvda)
fn detect_disk_device() -> String {
    if let Ok(content) = fs::read_to_string("/proc/diskstats") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 4 {
                continue;
            }
            let name = parts[2].to_string();
            let is_main = ["mmcblk0", "nvme0n1", "sda", "vda", "xvda", "nvme0n1p2"]
                .contains(&name.as_str())
                || (name.starts_with("sd") && name.len() == 3)
                || (name.starts_with("nvme") && !name.contains('p'))
                || (name.starts_with("mmcblk") && !name.contains('p'));
            if is_main {
                return name;
            }
        }
    }
    "sda".to_string()
}

// TCP ESTABLISHED connection count (tcp + tcp6)
fn count_tcp_conns() -> u64 {
    let mut count = 0u64;
    for f in ["/proc/net/tcp", "/proc/net/tcp6"] {
        if let Ok(content) = fs::read_to_string(f) {
            for line in content.lines().skip(1) {
                // Each line: sl local_address rem_address st ...
                // st = 01 (ESTABLISHED), 0A (LISTEN)
                if let Some(st) = line.split_whitespace().nth(3) {
                    if st == "01" {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

// Disk I/O rate (MB/s) + TCP connection count
pub(crate) fn get_io_stats() -> Value {
    #[cfg(target_os = "windows")]
    {
        return crate::platform::get_io();
    }
    #[cfg(target_os = "linux")]
    {
        let device = detect_disk_device();
        let (r1, w1) = match read_disk_sectors(&device) {
            Some(v) => v,
            None => return json!({"error": "cannot read diskstats"}),
        };
        std::thread::sleep(Duration::from_millis(IO_SAMPLE_MS));
        let (r2, w2) = match read_disk_sectors(&device) {
            Some(v) => v,
            None => return json!({"error": "cannot read diskstats"}),
        };
        let dt = IO_SAMPLE_MS as f64 / 1000.0;
        // sector delta * 512 bytes / dt -> bytes/s -> MB/s
        let read_mbs = (r2.saturating_sub(r1)) as f64 * 512.0 / dt / 1024.0 / 1024.0;
        let write_mbs = (w2.saturating_sub(w1)) as f64 * 512.0 / dt / 1024.0 / 1024.0;
        json!({
            "device": device,
            "disk_read_mbs": (read_mbs * 100.0).round() / 100.0,
            "disk_write_mbs": (write_mbs * 100.0).round() / 100.0,
            "tcp_conns": count_tcp_conns(),
        })
    }
}
