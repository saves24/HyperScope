// Shared utilities

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
pub fn civil_from_days(z: i64) -> (i64, u64, u64) {
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
pub fn format_date(secs: u64) -> String {
    let days = secs / 86400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}
pub fn format_time(secs: u64) -> String {
    let days = secs / 86400;
    let rem = secs % 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    format!("{y:04}-{m:02}-{d:02} {hh:02}:{mm:02}:{ss:02}")
}
pub fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
pub fn sha256_hex(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn gen_random_password(len: usize) -> String {
    use rand::{rngs::OsRng, Rng};
    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789!@#$%^&*";
    let mut rng = OsRng;
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}
pub fn valid_user_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric())
}
pub fn valid_password(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric())
}
pub fn validate_user_input(user: &str, pass: &str) -> Result<(), String> {
    if user.is_empty() || pass.is_empty() {
        return Err("username and password cannot be empty".to_string());
    }
    if !valid_user_name(user) {
        return Err("username may only contain letters and digits".to_string());
    }
    if !valid_password(pass) {
        return Err("password may only contain letters and digits".to_string());
    }
    if pass.len() < 4 {
        return Err("password must be at least 4 characters".to_string());
    }
    Ok(())
}
pub fn validate_user_name_only(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("username cannot be empty".to_string());
    }
    if !valid_user_name(name) {
        return Err("username may only contain letters and digits".to_string());
    }
    Ok(())
}
pub fn validate_password_only(pass: &str) -> Result<(), String> {
    if pass.is_empty() {
        return Err("password cannot be empty".to_string());
    }
    if !valid_password(pass) {
        return Err("password may only contain letters and digits".to_string());
    }
    if pass.len() < 4 {
        return Err("password must be at least 4 characters".to_string());
    }
    Ok(())
}

// Atomic file write: temp + fsync + rename + fsync dir (survives power loss mid-write)
pub fn atomic_write(path: &str, content: &str, _mode: u32) -> Result<(), String> {
    use std::io::Write;
    let tmp = format!("{path}.tmp");
    let mut f = std::fs::File::create(&tmp).map_err(|e| format!("create failed: {e}"))?;
    f.write_all(content.as_bytes())
        .map_err(|e| format!("write failed: {e}"))?;
    f.sync_all().map_err(|e| format!("fsync failed: {e}"))?;
    drop(f);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(_mode))
            .map_err(|e| format!("set permissions failed: {e}"))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("rename failed: {e}"))?;
    // fsync parent directory so the rename itself is durable
    if let Some(dir) = std::path::Path::new(path).parent() {
        if let Ok(d) = std::fs::File::open(dir) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

#[cfg(unix)]
pub fn set_private_file_mode(path: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("set private file permissions failed: {e}"))
}

#[cfg(not(unix))]
pub fn set_private_file_mode(_path: &str) -> Result<(), String> {
    Ok(())
}

// Generate a stable node id (distinct from api keys/passwords/tokens; URL-safe alphanumeric)
pub fn generate_node_id() -> String {
    use rand::{rngs::OsRng, Rng};
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = OsRng;
    (0..10)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}
