use crate::{
    cert_fingerprint, ensure_cert, generate_key, get_retention_days, load_key, log_write, save_key,
    set_retention_days, DEFAULT_PORT, DEFAULT_RETENTION_DAYS, KEY_FILE, LOG_DIR, VERSION,
};
use std::fs;
// CLI commands: key/cert/mode/trust/log management
use futures_util::SinkExt;
// ---------- CLI ----------

pub(crate) fn print_help() {
    println!(
        r#"hyper-node - system monitoring collector (v{VERSION})

Usage:
  hyper-node <command> [options]

Commands:
  key setup [KEY] [--plain]  set API key. Generates random key when KEY is not given.
                    default generates certificate-bound key (key includes cert fingerprint, for TLS nodes);
                    --plain generates legacy plaintext key (for non-TLS nodes)
  key show          show current API key (with certificate fingerprint format)
  cert gen          generate/renew TLS certificate (self-signed, written to /etc/hyper-node/)
  cert show         show current certificate SHA256 fingerprint
  mode [tls|plain]  view or set connection mode (tls=encrypted / plain=plaintext, takes effect after restart)
  serve [--port N] [--no-tls]  start collector service, default HTTPS (auto-generates cert if missing)
                    default listen 0.0.0.0:{DEFAULT_PORT}; --no-tls downgrades to plaintext
  log retention N   set log retention days (default {DEFAULT_RETENTION_DAYS}, auto cleanup)
  log show          show log retention config
  help              show this help

Config:
  key file:  {KEY_FILE} (mode 600)
  log dir:   {LOG_DIR} (daily rotation)

Auth:
  Panel requests must carry the key:
    Authorization: Bearer ***
    or X-API-Key: ***
"#
    );
}

pub(crate) fn cmd_key_setup(arg: Option<&str>, plain: bool) -> i32 {
    let key = match arg {
        Some(k) if !k.is_empty() => k.to_string(),
        _ => match generate_key() {
            Ok(k) => k,
            Err(e) => {
                eprintln!("error: {e}");
                return 1;
            }
        },
    };
    // Default generates key with cert fingerprint; --plain generates plaintext key
    if !plain {
        if let Err(e) = ensure_cert() {
            eprintln!("warning: certificate generation failed: {e}");
        }
    }
    match save_key(&key) {
        Ok(()) => {
            println!("key set successfully");
            if !plain {
                // Print full key: secret|cert-fingerprint (panel pastes this key for encrypted connection)
                let fp = cert_fingerprint().unwrap_or_default();
                if arg.is_none() {
                    println!("generated key: {key}");
                }
                if !fp.is_empty() {
                    println!("panel key (with certificate fingerprint): {key}|{fp}");
                }
            } else {
                println!("plaintext mode (no certificate): key for non-TLS nodes only");
                if arg.is_none() {
                    println!("generated key: {key}");
                }
            }
            log_write("INFO", "API key set");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            log_write("ERROR", &format!("failed to set key: {e}"));
            1
        }
    }
}

pub(crate) fn cmd_key_show() -> i32 {
    match load_key() {
        Ok(k) => {
            // Print full key with cert fingerprint (for panel config)
            let fp = cert_fingerprint().unwrap_or_default();
            if fp.is_empty() {
                println!("{k}");
                println!(
                    "hint: run 'hyper-node cert gen' to generate certificate for encrypted key"
                );
            } else {
                println!("{k}|{fp}");
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            println!("hint: key not set, run 'hyper-node key setup' first");
            1
        }
    }
}

// ---------- TLS certificate ----------

// Read trusted client certificate fingerprint list

pub(crate) fn cmd_log_retention(arg: Option<&str>) -> i32 {
    let days = match arg.and_then(|s| s.parse::<u64>().ok()) {
        Some(d) if d > 0 => d,
        _ => {
            eprintln!("usage: hyper-node log retention <days> (positive integer)");
            return 1;
        }
    };
    match set_retention_days(days) {
        Ok(()) => {
            println!("log retention set to {days} days");
            log_write("INFO", &format!("log retention set to {days} days"));
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

pub(crate) fn cmd_log_show() -> i32 {
    let retention = get_retention_days();
    let log_size = dir_size(LOG_DIR);
    println!("log retention: {retention} days (default {DEFAULT_RETENTION_DAYS})");
    println!("log dir: {LOG_DIR}");
    println!("current usage: {:.1} MB", log_size as f64 / 1024.0 / 1024.0);
    if let Ok(entries) = fs::read_dir(LOG_DIR) {
        let files: Vec<_> = entries.flatten().collect();
        println!("log files: {}", files.len());
        for f in files.iter().take(10) {
            if let Ok(md) = f.metadata() {
                println!(
                    "  {} ({:.1} KB)",
                    f.file_name().to_string_lossy(),
                    md.len() as f64 / 1024.0
                );
            }
        }
    }
    0
}

pub(crate) fn dir_size(path: &str) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for e in entries.flatten() {
            if let Ok(md) = e.metadata() {
                total += md.len();
            }
        }
    }
    total
}

pub(crate) async fn cmd_connect(panel_url: &str, name: &str, key: &str) -> i32 {
    // Reverse push: node has no listening port. Primary: WebSocket (real-time);
    // fallback: periodic HTTP POST (keeps metrics flowing if WS is unavailable).
    use std::time::Duration;
    let ws_url = format!(
        "{}/ws",
        panel_url.trim_end_matches('/').replacen("http", "ws", 1)
    );
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    eprintln!("connect mode: ws={ws_url} node={name} (5s interval, POST fallback)");
    loop {
        match tokio_tungstenite::connect_async(&ws_url).await {
            Ok((mut ws, _)) => {
                // Auth + push loop over the long-lived connection
                loop {
                    interval.tick().await;
                    let data = crate::metrics::get_system_cached();
                    let msg = serde_json::json!({"protocol_version": crate::PROTOCOL_VERSION, "name": name, "key": key, "ts": crate::now_unix(), "data": data});
                    if ws
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            msg.to_string(),
                        ))
                        .await
                        .is_err()
                    {
                        eprintln!("ws disconnected; reconnecting");
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("ws connect failed ({e}); falling back to POST");
                push_post(panel_url, name, key, &mut interval).await;
            }
        }
    }
}

async fn push_post(panel_url: &str, name: &str, key: &str, interval: &mut tokio::time::Interval) {
    use std::time::Duration;
    let url = format!("{}/api/push", panel_url.trim_end_matches('/'));
    for _ in 0..12 {
        interval.tick().await;
        let body = push_payload(name, key);
        let client = reqwest::Client::new();
        match client
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(8))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => {}
            Ok(r) => eprintln!("push failed: HTTP {}", r.status()),
            Err(e) => eprintln!("push error: {e}"),
        }
    }
}

// Collect full metric set for reverse push (system + live traffic + io)
fn push_payload(name: &str, key: &str) -> serde_json::Value {
    // Push mode focuses on system metrics (real-time); traffic/io are not pushed
    let system = crate::metrics::get_system_cached();
    serde_json::json!({
        "protocol_version": crate::PROTOCOL_VERSION,
        "name": name,
        "key": key,
        "ts": crate::now_unix(),
        "data": system,
    })
}
