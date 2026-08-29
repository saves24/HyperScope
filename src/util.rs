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
#[allow(dead_code)]
pub(crate) fn chmod(path: &str, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}
#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub(crate) fn chmod(_path: &str, _mode: u32) {}

/// Best-effort local IPv4 address (used to advertise temporary direct ports).
/// Picks the address that remote LAN clients are most likely to reach:
/// - UDP-connect trick gives the primary outbound interface address.
/// - falls back to the first private non-docker address.
#[cfg(unix)]
#[allow(dead_code)]
pub(crate) fn local_ip() -> Option<String> {
    use std::net::UdpSocket;
    // 1. UDP connect trick: kernel picks the interface used to reach the
    //    default route — exactly what remote clients will use.
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = sock.local_addr() {
                let ip = addr.ip().to_string();
                if !ip.starts_with("127.") {
                    return Some(ip);
                }
            }
        }
    }
    // 2. Fallback: first private address that is not docker/tailscale-ish.
    let mut best: Option<String> = None;
    unsafe {
        let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifap) == 0 {
            let mut cur = ifap;
            while !cur.is_null() {
                let ifa = &*cur;
                if !ifa.ifa_addr.is_null() && (*ifa.ifa_addr).sa_family as i32 == libc::AF_INET {
                    let addr = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                    let ip = std::net::Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes());
                    let s = ip.to_string();
                    if ip.is_private() && !s.starts_with("172.") && !s.starts_with("100.") {
                        best = Some(s.clone());
                    } else if best.is_none() {
                        best = Some(s);
                    }
                }
                cur = ifa.ifa_next;
            }
            libc::freeifaddrs(ifap);
        }
    }
    best
}
#[cfg(windows)]
#[allow(dead_code)]
pub(crate) fn local_ip() -> Option<String> {
    None
}
