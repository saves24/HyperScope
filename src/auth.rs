use crate::{read_file, KEY_DIR, KEY_FILE};
// Key management and authentication
use axum::http::{header, HeaderMap, StatusCode};
use std::fs;

pub(crate) fn key_path() -> String {
    KEY_FILE.to_string()
}
pub(crate) fn load_key() -> Result<String, String> {
    read_file(&key_path()).map(|s| s.trim().to_string())
}
pub(crate) fn save_key(key: &str) -> Result<(), String> {
    let _ = fs::create_dir_all(KEY_DIR);
    crate::atomic_write(&key_path(), &format!("{key}\n"), 0o600)
}
pub(crate) fn generate_key() -> Result<String, String> {
    // 32 random bytes from the OS CSPRNG (cross-platform; hex-encoded 64 chars).
    // Fail loudly instead of silently returning an all-zero key.
    let mut buf = [0u8; 32];
    use rand::RngCore;
    rand::rngs::OsRng
        .try_fill_bytes(&mut buf)
        .map_err(|e| format!("failed to get randomness: {e}"))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}
pub(crate) fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
pub(crate) async fn auth_required(headers: HeaderMap) -> Result<(), StatusCode> {
    let stored = match load_key() {
        Ok(key) => key,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    if stored.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    // Supports Authorization: Bearer *** or X-API-Key: ***
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get("x-api-key")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    if constant_time_eq(&provided, &stored) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
