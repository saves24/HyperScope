use crate::{read_file, KEY_DIR, KEY_FILE};
// Key management and authentication
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
