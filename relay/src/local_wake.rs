// Local collector wake-up: hyper-relay spawns the same-machine hyper-node as a
// one-shot process (`hyper-node collect`) and reads the JSON snapshot from its
// stdout. The collector is never resident and never opens a listening port;
// every poll is a fresh short-lived process.
use std::process::Command;

/// Path of the collector binary that this relay wakes on demand.
#[cfg(unix)]
fn collector_bin() -> &'static str {
    "/usr/local/bin/hyper-node"
}
#[cfg(windows)]
fn collector_bin() -> &'static str {
    "C:\\ProgramData\\hyper-node\\hyper-node.exe"
}

/// Collect a full metrics snapshot by spawning `hyper-node collect`.
/// Returns Some(parsed JSON) on success.
pub fn wake_collector() -> Option<serde_json::Value> {
    let bin = collector_bin();
    // Use Command::output(): it waits for the child and collects stdout+stderr
    // (proven to work on Windows; manual piped reads can deadlock on some
    // service/session combinations).
    let out = match Command::new(bin).arg("collect").output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("wake_collector: spawn {bin} failed: {e}");
            return None;
        }
    };
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.stderr.is_empty() {
        eprintln!(
            "wake_collector: collector stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    match serde_json::from_str(stdout.trim()) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!(
                "wake_collector: JSON parse failed ({e}); out_len={}",
                stdout.len()
            );
            None
        }
    }
}

/// Execute a signed control command by spawning the collector process
/// (`hyper-node control <cmd> <device_id> <signature>`). The device signature
/// is passed through so the collector verifies it against its trusted-device
/// list (the relay already verified it, this is defense in depth).
/// Returns Some(result string) on success.
pub fn run_command(cmd: &str, device_id: &str, signature: &str) -> Option<String> {
    // Use Command::output() so both stdout and stderr are drained: reading
    // only stdout can deadlock when the child fills its stderr pipe buffer
    // (the child blocks, stdout never reaches EOF).
    let out = Command::new(collector_bin())
        .arg("control")
        .arg(cmd)
        .arg(device_id)
        .arg(signature)
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}
