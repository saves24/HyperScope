// Webhook delivery for alert notifications.
//
// A node (or the global panel setting) can carry a webhook URL (ntfy / Bark /
// Server Chan / Slack / generic JSON endpoint). When an alert transitions, the
// panel POSTs a small JSON payload to that URL. Delivery is best-effort: it
// runs in a detached task, has a short timeout, and failures only log — an
// alert must never block the poller or fail the panel because a webhook is
// unreachable.
use crate::client::connect_stream;
use crate::{NodeConfig, SharedState};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Fire-and-forget webhook POST. Returns () immediately; the send happens on
/// a detached tokio task so the caller never waits on an external endpoint.
///
/// The `target` may be either a plain URL (ntfy/Bark/custom webhook) or a JSON
/// config string from the UI, e.g. {"type":"pushplus","token":"..."} /
/// {"type":"serverchan","sendkey":"..."} / {"type":"telegram","bot_token":"...","chat_id":"..."}.
pub fn spawn_webhook(target: &str, payload: Value) {
    let target = target.to_string();
    tokio::spawn(async move {
        match route_target(&target, &payload).await {
            Ok(()) => {}
            Err(e) => eprintln!("webhook send failed ({target}): {e}"),
        }
    });
}

/// Routes a target (URL or JSON config) to the right channel and sends.
async fn route_target(target: &str, payload: &Value) -> Result<(), String> {
    if let Ok(cfg) = serde_json::from_str::<Value>(target) {
        if let Some(t) = cfg.get("type").and_then(|v| v.as_str()) {
            let msg = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("HyperScope alert")
                .to_string();
            let node = payload
                .get("node")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let title = format!("⚠️ {node} alert");
            return match t {
                "pushplus" => {
                    let token = cfg.get("token").and_then(|v| v.as_str()).unwrap_or("");
                    if token.is_empty() {
                        return Err("pushplus token missing".to_string());
                    }
                    send_form(
                        "https://www.pushplus.plus/send",
                        &[
                            ("token", token),
                            ("title", &title),
                            ("content", &msg),
                            ("template", "html"),
                        ],
                    )
                    .await
                }
                "serverchan" => {
                    let key = cfg.get("sendkey").and_then(|v| v.as_str()).unwrap_or("");
                    if key.is_empty() {
                        return Err("serverchan sendkey missing".to_string());
                    }
                    send_form(
                        &format!("https://sctapi.ftqq.com/{key}.send"),
                        &[("title", &title), ("desp", &msg)],
                    )
                    .await
                }
                "telegram" => {
                    let token = cfg.get("bot_token").and_then(|v| v.as_str()).unwrap_or("");
                    let chat = cfg.get("chat_id").and_then(|v| v.as_str()).unwrap_or("");
                    if token.is_empty() || chat.is_empty() {
                        return Err("telegram bot_token/chat_id missing".to_string());
                    }
                    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
                    let body = json!({"chat_id": chat, "text": format!("{title}\n{msg}")});
                    send_webhook(&url, &body).await
                }
                _ => Err(format!("unknown notify type {t}")),
            };
        }
    }
    // Plain URL: generic JSON POST.
    send_webhook(target, payload).await
}

/// Sends an application/x-www-form-urlencoded POST (PushPlus / Server Chan).
/// Hosts are hardcoded well-known endpoints, but the same SSRF guard as
/// send_webhook is applied for defense in depth.
async fn send_form(url: &str, fields: &[(&str, &str)]) -> Result<(), String> {
    let body = fields
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&");
    let parsed = parse_url(url)?;
    if let Err(e) = crate::nodes::resolve_safe_addr(&parsed.host, parsed.port) {
        return Err(format!("webhook target blocked: {e}"));
    }
    let mut stream = connect_stream(
        &parsed.host,
        parsed.port,
        Duration::from_secs(5),
        parsed.tls,
        "",
    )
    .await?;
    let port_hdr = if parsed.tls {
        String::new()
    } else {
        format!(":{}", parsed.port)
    };
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}{}\r\nContent-Type: application/x-www-form-urlencoded\r\nUser-Agent: hyper-panel/1.0\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        parsed.path,
        parsed.host,
        port_hdr,
        body.len(),
        body,
    );
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    let mut buf = [0u8; 2048];
    let _ = stream.read(&mut buf).await;
    Ok(())
}

/// Minimal percent-encoder for form fields (space -> +, safe chars kept).
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Sends one JSON POST to a webhook URL (http:// or https://). Short timeout;
/// errors are returned to the caller (spawn_webhook logs them).
/// The target host is validated with the same safe-address rules used when
/// adding a node, so a webhook cannot be pointed at loopback/link-local/
/// metadata/internal ranges (SSRF protection).
async fn send_webhook(url: &str, payload: &Value) -> Result<(), String> {
    let parsed = parse_url(url)?;
    // SSRF guard: resolve the host and reject blocked IP ranges before any
    // connection is made. Same policy as node addresses.
    if let Err(e) = crate::nodes::resolve_safe_addr(&parsed.host, parsed.port) {
        return Err(format!("webhook target blocked: {e}"));
    }
    let body = serde_json::to_string(payload).map_err(|e| e.to_string())?;

    let mut stream = connect_stream(
        &parsed.host,
        parsed.port,
        Duration::from_secs(5),
        parsed.tls,
        "",
    )
    .await?;

    let port_hdr = if parsed.tls {
        String::new()
    } else {
        format!(":{}", parsed.port)
    };
    let req = format!(
        "POST {} HTTP/1.1\r\nHost: {}{}\r\nContent-Type: application/json\r\nUser-Agent: hyper-panel/1.0\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        parsed.path,
        parsed.host,
        port_hdr,
        body.len(),
        body,
    );
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| e.to_string())?;

    // Read response (ignore body, just drain until connection closes).
    let mut buf = [0u8; 2048];
    let _ = stream.read(&mut buf).await;
    Ok(())
}

struct ParsedUrl {
    host: String,
    port: u16,
    path: String,
    tls: bool,
}

/// Minimal URL parser for http(s) webhook targets.
fn parse_url(url: &str) -> Result<ParsedUrl, String> {
    let (tls, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Err("webhook must be http(s)://".to_string());
    };
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => {
            let port = p
                .parse::<u16>()
                .map_err(|_| "invalid webhook port".to_string())?;
            (h.to_string(), port)
        }
        None => (authority.to_string(), if tls { 443 } else { 80 }),
    };
    if host.is_empty() {
        return Err("invalid webhook host".to_string());
    }
    Ok(ParsedUrl {
        host,
        port,
        path: path.to_string(),
        tls,
    })
}

/// Builds the alert webhook payload shared by all channels.
pub fn alert_payload(node: &str, key: &str, message: &str) -> Value {
    json!({
        "event": "alert",
        "node": node,
        "key": key,
        "message": message,
        "time": crate::format_time(crate::now_unix()),
    })
}

/// Returns the webhook URL for a node: per-node `webhook` if set, else the
/// global panel setting (from panel.json), else empty.
pub fn webhook_url_for(_app: &SharedState, node: &NodeConfig) -> String {
    if !node.webhook.is_empty() {
        return node.webhook.clone();
    }
    // Global fallback from panel.json (best-effort read; no caching).
    std::fs::read_to_string(crate::SETTINGS_FILE)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| {
            v.get("webhook")
                .and_then(|w| w.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}
