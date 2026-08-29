pub(crate) use hyper_panel_core::history;
// Shared core symbols re-exported for panel modules (explicit, not wildcard).
// Groups: state (SharedState/NodeConfig/...), persistence (load/save/events),
// client (http_request/...), util (time/id/hash/atomic), poller (background).
#[cfg(test)]
pub(crate) use hyper_panel_core::ssrf_blocked;
pub(crate) use hyper_panel_core::{
    atomic_write, background_poller, config_mtime, gen_random_password, generate_node_id,
    http_request_json, load_events_from_file, load_nodes, load_notifications_from_file, log_write,
    node_visible, now_unix, record_admin_action, resolve_safe_addr, save_nodes, set_retention_days,
    sha256_hex, tail_log, urlencode, validate_password_only, validate_user_input, AppState,
    HttpOptions, NodeConfig, NodeState, SharedState, User, AUTH_FILE, EVENTS_FILE, LOG_DIR,
    NOTIF_FILE, SETTINGS_FILE,
};
// hyper-panel - system monitoring panel aggregator (Rust)
// Role: node management (CLI+API) + background agent polling + in-memory cache + frontend
// Commands: help / node add / node del / node list / node show / setup / log / serve

mod api;
mod auth;
mod cli;
mod events;
mod nodes;
mod serve;

use api::*;
use auth::*;
use events::*;
use nodes::*;

use axum::{
    extract::{Json as JsonExtract, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde_json::{json, Value};
use std::{collections::HashMap, time::Duration};

pub(crate) const VERSION: &str = "1.0.0";
pub(crate) const DEFAULT_PORT: u16 = 8088;
pub(crate) const DEFAULT_RETENTION_DAYS: u64 = 7;

// Frontend assets
const INDEX_HTML: &str = include_str!("../html/index.html");
const PANEL_JS: &str = include_str!("../static/panel.js");
const STYLE_CSS: &str = include_str!("../static/style.css");

// Event file (append-only, keep most recent 1000 entries)

// Record event (with timestamp, keep most recent MAX_EVENTS; persisted to events.json)

// Append event to file; rewrite-truncate above 1000 lines (keep most recent 500)

// Load most recent MAX_EVENTS events from file at startup

// ---------- Logging (daily rotation + periodic cleanup) ----------

// Last cleanup time (avoid scanning directory on every log write)

// Read last N log lines

// ---------- Config read/write ----------

// ---------- Minimal HTTP client (plaintext/TLS + cert fingerprint verification) ----------

// Custom cert verifier: check node cert fingerprint matches (fingerprint pinning)

// Create TLS client connection: verify fingerprint; trust peer and return actual fingerprint when cert_fp is empty
// Present panel client certificate (for server-side identity verification)

// Unified HTTP request: connect (GET pooled / POST fresh) + send + read response + parse JSON

// Establish plaintext/TLS connection

// POST request (low-frequency ops: reboot/shutdown; fresh connection each time)

// ---------- Panel client certificate (mutual TLS server-side verification) ----------

// Generate panel client certificate (self-signed, for server-side identity verification)

// Compute SHA256 fingerprint of any PEM certificate file

// Load panel client certificate config (sent to server for verification)

// ---------- Background polling ----------

// Poll node: return (data, status, cert fingerprint (auto-acquired on TOFU; empty = nothing to record))

// ---------- HTTP routes ----------

// Rename node: PUT /api/node/id/:node_id/name {name}

// Ping node: POST /api/node/id/:node_id/ping -> run ping and return output

// Get node accessible to current user (admin sees all; regular users only their own); None = missing or forbidden

// Generic: auth + resolve node + GET forward (url_fn builds path, reads query params)

// Generic: auth + resolve node + POST forward (action used in response body)

// Macro: generate GET forward handler (handler_name, timeout, URL builder closure (cfg, params))
macro_rules! node_get_handler {
    ($name:ident, $timeout:expr, $url_fn:expr) => {
        async fn $name(
            Path(ident): Path<String>,
            Query(params): Query<HashMap<String, String>>,
            headers: HeaderMap,
            State(app): State<SharedState>,
        ) -> impl IntoResponse {
            node_fetch_get(&app, &headers, &ident, &params, $timeout, $url_fn).await
        }
    };
}

node_get_handler!(node_traffic_handler, 5, |_cfg: &NodeConfig,
                                            params: &HashMap<
    String,
    String,
>| {
    let iface = params.get("iface").cloned().unwrap_or_default();
    format!(
        "/traffic{}",
        if iface.is_empty() {
            String::new()
        } else {
            format!("?iface={iface}")
        }
    )
});
node_get_handler!(node_disks_handler, 5, |_cfg: &NodeConfig,
                                          _params: &HashMap<
    String,
    String,
>| { "/disks".to_string() });
node_get_handler!(node_processes_handler, 8, |_cfg: &NodeConfig,
                                              params: &HashMap<
    String,
    String,
>| {
    let sort = params
        .get("sort")
        .cloned()
        .unwrap_or_else(|| "mem".to_string());
    let limit = params
        .get("limit")
        .cloned()
        .unwrap_or_else(|| "20".to_string());
    let name = params.get("name").cloned().unwrap_or_default();
    format!(
        "/processes?sort={}&limit={}&name={}",
        urlencode(&sort),
        urlencode(&limit),
        urlencode(&name)
    )
});
node_get_handler!(
    node_io_handler,
    8,
    |_cfg: &NodeConfig, _params: &HashMap<String, String>| { "/io".to_string() }
);

// History trend query: GET /api/node/id/:node_id/history?metric=cpu&range=24h
// Reads from panel-local SQLite (persisted across restarts)
async fn node_history_handler(
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    State(app): State<SharedState>,
    Path(ident): Path<String>,
) -> impl IntoResponse {
    match visible_node_by_ident(&app, &headers, &ident).await {
        Some(ns) => {
            let metric = params
                .get("metric")
                .cloned()
                .unwrap_or_else(|| "cpu".to_string());
            let range = params
                .get("range")
                .cloned()
                .unwrap_or_else(|| "24h".to_string());
            // Optional explicit window (compare overlay): start/end unix secs.
            let start_ov = params
                .get("start")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let end_ov = params
                .get("end")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let points = if start_ov > 0 || end_ov > 0 {
                history::history_query_range(&ns.config.id, &metric, &range, start_ov, end_ov)
            } else {
                history::history_query(&ns.config.id, &metric, &range)
            };
            (StatusCode::OK, Json(points)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "node not found"})),
        )
            .into_response(),
    }
}
// Compare + export handlers (read panel-local SQLite; auth via middleware)

async fn history_export_handler(
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    State(app): State<SharedState>,
    Path(ident): Path<String>,
) -> impl IntoResponse {
    // Ownership check (same as node_history_handler): node must be visible to the current user
    match visible_node_by_ident(&app, &headers, &ident).await {
        Some(ns) => {
            let metric = params
                .get("metric")
                .cloned()
                .unwrap_or_else(|| "cpu".to_string());
            let range = params
                .get("range")
                .cloned()
                .unwrap_or_else(|| "24h".to_string());
            let csv = history::history_export(&ns.config.id, &metric, &range);
            (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "text/csv")],
                csv,
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "node not found"})),
        )
            .into_response(),
    }
}

node_get_handler!(node_docker_handler, 8, |_cfg: &NodeConfig,
                                           _params: &HashMap<
    String,
    String,
>| { "/docker".to_string() });

// Docker container control: POST /api/node/id/:node_id/docker/:container/:action
async fn node_docker_control_handler(
    Path(params): Path<(String, String, String)>,
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    let (ident, container, action) = params;
    match visible_node_by_ident(&app, &headers, &ident).await {
        Some(ns) => {
            let path = format!("/docker/{container}/{action}");
            // Control commands go through the relay (spawns the local
            // collector). No direct HTTP — the collector opens no port.
            let relay_addr = format!("{}:{}", ns.config.addr, 8686);
            let (device_id, signature) = match hyper_panel_core::identity::ensure_identity() {
                Ok(pubkey) => {
                    // Sign cmd:device_id:timestamp:nonce (replay protection).
                    let ts = hyper_panel_core::now_unix();
                    let nonce = format!("{ts:x}{}", rand::random::<u64>());
                    let msg = format!("{path}:panel:{ts}:{nonce}");
                    match hyper_panel_core::identity::sign_with_identity(&msg) {
                        Ok(sig) => ("panel".to_string(), format!("{ts}:{nonce}:{sig}")),
                        Err(_) => (pubkey, String::new()),
                    }
                }
                Err(_) => (String::new(), String::new()),
            };
            match hyper_panel_core::relay_client::send_command(
                &relay_addr,
                &ns.config.name,
                &path,
                ns.config.tls,
                &device_id,
                &signature,
                Some(&ns.config.cert_fp),
            )
            .await
            {
                Ok(v) => {
                    let inner = v
                        .as_str()
                        .and_then(|s| serde_json::from_str::<Value>(s).ok());
                    let ok = inner
                        .as_ref()
                        .and_then(|x| x.get("ok"))
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false);
                    (StatusCode::OK, Json(json!({"ok": ok, "action": action}))).into_response()
                }
                Err(e) => (StatusCode::BAD_GATEWAY, Json(json!({"error": e}))).into_response(),
            }
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "node not found or access denied"})),
        )
            .into_response(),
    }
}

// Reboot node: POST /api/node/id/:node_id/reboot

// Shutdown node: POST /api/node/id/:node_id/shutdown

// Status: frontend probes whether init/login is required

// Users are created/reset via CLI (user add / user passwd / setup); no web-based account bootstrap

// Settings: GET returns panel port (admin only)

// Check whether port is in use (try to bind)

// Settings: POST saves panel port (admin only; auto-restart to apply on change)

// Try to restart panel via systemd (detach, 1s delay, avoids killing self before response)

// CLI: hyper-panel port [N] - view/set panel port

// ---------- Startup ----------

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");
    let code = rt.block_on(async_main());
    std::process::exit(code);
}

async fn async_main() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (code, serve_port) = crate::cli::dispatch(&args);
    if let Some(port) = serve_port {
        return crate::serve::cmd_serve(port).await;
    }
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_node_address_ipv4() {
        let (a, p) = crate::cli::parse_node_address("192.168.1.10:5001").unwrap();
        assert_eq!(a, "192.168.1.10");
        assert_eq!(p, 5001);
    }

    #[test]
    fn parse_node_address_default_port() {
        let (a, p) = crate::cli::parse_node_address("10.0.0.5").unwrap();
        assert_eq!(a, "10.0.0.5");
        assert_eq!(p, 8686);
    }

    #[test]
    fn parse_node_address_ipv6_bracketed() {
        let (a, p) = crate::cli::parse_node_address("[2001:db8::1]:8443").unwrap();
        assert_eq!(a, "2001:db8::1");
        assert_eq!(p, 8443);
    }

    #[test]
    fn parse_node_address_bare_ipv6() {
        let (a, p) = crate::cli::parse_node_address("2001:db8::1234").unwrap();
        assert_eq!(a, "2001:db8::1234");
        assert_eq!(p, 8686);
    }

    #[test]
    fn parse_node_address_invalid_port() {
        assert!(crate::cli::parse_node_address("1.2.3.4:99999").is_none());
    }

    #[test]
    fn ssrf_guard_blocks_metadata_and_loopback() {
        assert!(ssrf_blocked("169.254.169.254"));
        assert!(ssrf_blocked("127.0.0.1"));
        assert!(ssrf_blocked("localhost"));
        assert!(ssrf_blocked("::1"));
        assert!(ssrf_blocked("fe80::1"));
        assert!(ssrf_blocked("0.0.0.0"));
    }

    #[test]
    fn ssrf_guard_allows_private_nets() {
        assert!(!ssrf_blocked("10.0.0.8"));
        assert!(!ssrf_blocked("172.16.5.5"));
        assert!(!ssrf_blocked("192.168.1.9"));
        assert!(!ssrf_blocked("ans.saves24.cc.cd"));
    }

    #[test]
    fn atomic_write_roundtrip() {
        let path = std::env::temp_dir().join(format!("hyper-test-{}.json", std::process::id()));
        let path = path.to_string_lossy().to_string();
        crate::atomic_write(&path, "{\"ok\":true}", 0o600).unwrap();
        let read = std::fs::read_to_string(&path).unwrap();
        assert!(read.contains("ok"));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{path}.tmp"));
    }

    #[test]
    fn ssrf_blocks_ipv4_mapped_and_unspecified() {
        // IPv4-mapped IPv6 forms of loopback/metadata must be blocked too
        assert!(ssrf_blocked("::ffff:127.0.0.1"));
        assert!(ssrf_blocked("::ffff:169.254.169.254"));
        assert!(ssrf_blocked("::"));
        assert!(ssrf_blocked("224.0.0.1")); // multicast
        assert!(ssrf_blocked("[::1]"));
    }

    #[test]
    fn ssrf_strips_scheme_userinfo_and_path() {
        assert!(ssrf_blocked("http://127.0.0.1/x"));
        assert!(ssrf_blocked("user@localhost"));
        assert!(ssrf_blocked("https://169.254.169.254/latest"));
    }

    #[test]
    fn node_config_validate_rejects_empty_and_bad_port() {
        // name/addr/key must be non-empty; port must be 1..=65535
        assert!(
            hyper_panel_core::NodeConfig::from_value(&serde_json::json!({
                "name": "", "addr": "1.2.3.4", "port": 8686, "key": "k"
            }))
            .is_none()
        );
        assert!(
            hyper_panel_core::NodeConfig::from_value(&serde_json::json!({
                "name": "n", "addr": "", "port": 8686, "key": "k"
            }))
            .is_none()
        );
        assert!(
            hyper_panel_core::NodeConfig::from_value(&serde_json::json!({
                "name": "n", "addr": "1.2.3.4", "port": 0, "key": "k"
            }))
            .is_none()
        );
        assert!(
            hyper_panel_core::NodeConfig::from_value(&serde_json::json!({
                "name": "n", "addr": "1.2.3.4", "port": 70000, "key": "k"
            }))
            .is_none()
        );
        assert!(
            hyper_panel_core::NodeConfig::from_value(&serde_json::json!({
                "name": "n", "addr": "1.2.3.4", "port": 8686, "key": "k"
            }))
            .is_some()
        );
    }

    #[test]
    fn constant_time_eq_matches_and_rejects() {
        assert!(crate::auth::constant_time_eq("abc123", "abc123"));
        assert!(!crate::auth::constant_time_eq("abc123", "abc124"));
        assert!(!crate::auth::constant_time_eq("abc", "abcd"));
        assert!(!crate::auth::constant_time_eq("", "x"));
    }

    #[test]
    fn password_hash_and_verify_roundtrip() {
        // New passwords use argon2 (PHC); legacy SHA256(salt:pass) must still verify
        let hash = crate::auth::hash_password("secret123").unwrap();
        assert!(hash.starts_with("$argon2"));
        let user = hyper_panel_core::User {
            name: "t".into(),
            salt: String::new(),
            hash,
            is_admin: false,
        };
        // password matches (check_password ignores salt for argon2 hashes)
        assert!(crate::auth::check_password(
            &user.salt,
            &user.hash,
            "secret123"
        ));
        // wrong password rejected
        assert!(!crate::auth::check_password(
            &user.salt, &user.hash, "wrong"
        ));
        // legacy SHA256(salt:pass) 64-hex still verifies
        let legacy_hash = sha256_hex("salt:legacy-pass");
        assert_eq!(legacy_hash.len(), 64);
        assert!(crate::auth::check_password(
            "salt",
            &legacy_hash,
            "legacy-pass"
        ));
    }
}
