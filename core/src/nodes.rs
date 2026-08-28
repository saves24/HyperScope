use crate::{format_time, now_unix, NodeConfig, SharedState, LOG_DIR};
// Node business logic (pure data operations; no HTTP handlers)
use serde_json::{json, Value};
use std::fs;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

#[cfg(target_os = "linux")]
pub const CONFIG_FILE: &str = "/etc/hyper-panel/nodes.json";
#[cfg(target_os = "windows")]
pub const CONFIG_FILE: &str = "C:\\ProgramData\\hyper-panel\\nodes.json";

// Event write counter (truncate file periodically to bound size)
pub(crate) static EVENT_WRITES: AtomicU64 = AtomicU64::new(0);

#[cfg(target_os = "linux")]
pub const EVENTS_FILE: &str = "/var/log/hyper-panel/events.json";
#[cfg(target_os = "windows")]
pub const EVENTS_FILE: &str = "C:\\ProgramData\\hyper-panel\\logs\\events.json";

#[cfg(target_os = "linux")]
pub const NOTIF_FILE: &str = "/var/log/hyper-panel/notifications.json";
#[cfg(target_os = "windows")]
pub const NOTIF_FILE: &str = "C:\\ProgramData\\hyper-panel\\logs\\notifications.json";

pub async fn record_event(app: &SharedState, node: &str, kind: &str, msg: String) {
    let mut events = app.events.lock().await;
    let ev = json!({
        "time": format_time(now_unix()),
        "node": node,
        "kind": kind, // online | offline | unauthorized | info
        "msg": msg,
    });
    events.push(ev.clone());
    let len = events.len();
    if len > 100 {
        events.drain(0..(len - 100));
    }
    drop(events);
    append_event_file(&ev);
}
pub async fn record_notification(app: &SharedState, node: &str, msg: String) {
    let mut notifs = app.notifications.lock().await;
    let ev = json!({
        "time": format_time(now_unix()),
        "node": node,
        "kind": "alert",
        "msg": msg,
    });
    notifs.push(ev.clone());
    let len = notifs.len();
    if len > 100 {
        notifs.drain(0..(len - 100));
    }
    drop(notifs);
    append_notif_file(&ev);
}
pub fn append_notif_file(ev: &Value) {
    if let Err(error) = fs::create_dir_all(LOG_DIR) {
        eprintln!("notification log directory unavailable: {error}");
        return;
    }
    let path = NOTIF_FILE.to_string();
    let line = format!("{}\n", serde_json::to_string(ev).unwrap_or_default());
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        if let Err(error) = f.write_all(line.as_bytes()) {
            eprintln!("notification log write failed: {error}");
        }
    }
    // Truncate check: keep the file bounded (same policy as events)
    if let Ok(content) = fs::read_to_string(&path) {
        let lines = content.lines().count();
        if lines > 500 {
            let keep: Vec<&str> = content.lines().rev().take(200).collect();
            let mut new = String::new();
            for l in keep.into_iter().rev() {
                new.push_str(l);
                new.push('\n');
            }
            let _ = crate::atomic_write(&path, &new, 0o600);
        }
    }
}
pub fn load_notifications_from_file() -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(content) = fs::read_to_string(NOTIF_FILE) {
        for line in content.lines() {
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if v.is_object() {
                    out.push(v);
                }
            }
        }
    }
    let len = out.len();
    if len > 100 {
        out.drain(0..(len - 100));
    }
    out
}
pub fn append_event_file(ev: &Value) {
    if let Err(error) = fs::create_dir_all(LOG_DIR) {
        eprintln!("event log directory unavailable: {error}");
        return;
    }
    let path = EVENTS_FILE.to_string();
    let line = format!("{}\n", serde_json::to_string(ev).unwrap_or_default());
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        use std::io::Write;
        if let Err(error) = f.write_all(line.as_bytes()) {
            eprintln!("event log write failed: {error}");
        }
    } else {
        eprintln!("event log file unavailable: {path}");
    }
    // Truncate check: run the full-file scan every 200 writes, not per event
    let n = EVENT_WRITES.fetch_add(1, AtomicOrdering::Relaxed) + 1;
    if n.is_multiple_of(200) {
        if let Ok(content) = fs::read_to_string(&path) {
            let lines = content.lines().count();
            if lines > 1000 {
                let keep: Vec<&str> = content.lines().rev().take(500).collect();
                let mut new = String::new();
                for l in keep.into_iter().rev() {
                    new.push_str(l);
                    new.push('\n');
                }
                let _ = crate::atomic_write(&path, &new, 0o600);
            }
        }
    }
}
pub fn load_events_from_file() -> Vec<Value> {
    let mut out = Vec::new();
    if let Ok(content) = fs::read_to_string(EVENTS_FILE) {
        for line in content.lines() {
            if let Ok(mut v) = serde_json::from_str::<Value>(line) {
                // Line format is one event object per line; skip non-objects (e.g. stale "[]")
                if !v.is_object() {
                    continue;
                }
                // Map legacy Chinese messages to English ids (frontend renders via i18n)
                if let Some(msg) = v.get("msg").and_then(|m| m.as_str()) {
                    let mapped = match msg {
                        "node recovered online" => "node online",
                        "node went offline" => "node offline",
                        "key auth failed" => "auth failed",
                        _ => msg,
                    };
                    if mapped != msg {
                        v["msg"] = json!(mapped);
                    }
                }
                out.push(v);
            }
        }
    }
    let len = out.len();
    if len > 100 {
        out.drain(0..(len - 100));
    }
    out
}

// SSRF guard: block link-local metadata, loopback and panel-self; private nets (10/172.16/192.168) are allowed
pub fn ssrf_blocked(addr: &str) -> bool {
    use std::net::{IpAddr, ToSocketAddrs};
    // Trim and strip any scheme/userinfo (e.g. "http://" or "user@")
    let addr = addr.trim().to_lowercase();
    let addr = addr.split("://").last().unwrap_or(&addr);
    let addr = addr.rsplit_once('@').map(|(_, u)| u).unwrap_or(addr);
    let addr = addr.split('/').next().unwrap_or(addr);
    if addr == "localhost" {
        return true;
    }
    // Strip IPv6 brackets: [::1] / [2001:db8::1]:5000
    let bare = addr
        .strip_prefix('[')
        .and_then(|a| a.strip_suffix(']'))
        .unwrap_or(addr);
    // Direct IP literal first (handles ::1, fe80::1, ::ffff:127.0.0.1, 127.0.0.1, ...)
    if let Ok(ip) = bare.parse::<IpAddr>() {
        return ip_blocked(&ip);
    }
    // host:port form (IPv4 or hostname): take the host part before the first ':'
    let host = match bare.split_once(':') {
        Some((h, _)) => h,
        None => bare,
    };
    if host == "localhost" {
        return true;
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return ip_blocked(&ip);
    }
    // Hostname: resolve and block if ANY resolved address is sensitive
    // (defends against DNS rebinding to loopback/link-local/metadata)
    if let Ok(mut addrs) = (host, 0).to_socket_addrs() {
        return addrs.any(|sa| ip_blocked(&sa.ip()));
    }
    false
}

// Resolve once and return the exact IP address that callers should persist and connect to.
pub fn resolve_safe_addr(addr: &str, port: u16) -> Result<String, String> {
    use std::net::ToSocketAddrs;

    let host = addr.trim();
    let host = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    if host.is_empty() || host.contains('/') || host.contains('@') {
        return Err("invalid node address".to_string());
    }
    let mut resolved = None;
    for socket_addr in (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("cannot resolve node address: {e}"))?
    {
        if ip_blocked(&socket_addr.ip()) {
            return Err("address not allowed (link-local / loopback)".to_string());
        }
        resolved = Some(socket_addr.ip().to_string());
    }
    resolved.ok_or_else(|| "cannot resolve node address".to_string())
}

// Sensitive targets: loopback, link-local (incl. cloud metadata 169.254.169.254),
// unspecified, multicast. Private RFC1918 ranges remain allowed (home-lab design).
fn ip_blocked(ip: &IpAddr) -> bool {
    let ip = match ip {
        IpAddr::V6(v6) => v6
            .to_ipv4_mapped()
            .map(IpAddr::V4)
            .unwrap_or(IpAddr::V6(*v6)),
        other => *other,
    };
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback() || v4.is_link_local() || v4.is_unspecified() || v4.is_multicast()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unicast_link_local()
                || v6.is_unspecified()
                || v6.is_multicast()
        }
    }
}

pub fn load_nodes() -> Vec<NodeConfig> {
    match fs::read_to_string(CONFIG_FILE) {
        Ok(content) => {
            let mut out: Vec<NodeConfig> = serde_json::from_str::<Vec<Value>>(&content)
                .ok()
                .unwrap_or_default()
                .iter()
                .filter_map(NodeConfig::from_value)
                .collect();
            // Migrate legacy configs: assign stable ids and persist once
            let mut changed = false;
            for n in out.iter_mut() {
                if n.id.is_empty() {
                    n.ensure_id();
                    changed = true;
                }
            }
            if changed {
                let _ = save_nodes(&out);
            }
            out
        }
        Err(_) => vec![],
    }
}
pub fn save_nodes(nodes: &[NodeConfig]) -> Result<(), String> {
    let dir = std::path::Path::new(CONFIG_FILE)
        .parent()
        .unwrap_or(std::path::Path::new("/etc/hyper-panel"));
    fs::create_dir_all(dir).map_err(|e| format!("failed to create directory: {e}"))?;
    let arr: Vec<Value> = nodes.iter().map(|n| n.to_value()).collect();
    let content =
        serde_json::to_string_pretty(&arr).map_err(|e| format!("serialize failed: {e}"))?;
    // nodes.json contains node keys: atomic write + 600 permissions
    crate::atomic_write(CONFIG_FILE, &content, 0o600)
}
pub fn config_mtime() -> Option<std::time::SystemTime> {
    fs::metadata(CONFIG_FILE)
        .ok()
        .and_then(|m| m.modified().ok())
}

pub fn node_visible(config: &NodeConfig, user: &str, is_admin: bool) -> bool {
    is_admin || config.owner == user
}
