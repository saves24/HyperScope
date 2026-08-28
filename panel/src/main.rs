pub(crate) use hyper_panel_core::history;
// Shared core symbols re-exported for panel modules (explicit, not wildcard).
// Groups: state (SharedState/NodeConfig/...), persistence (load/save/events),
// client (http_request/...), util (time/id/hash/atomic), poller (background).
#[cfg(test)]
pub(crate) use hyper_panel_core::ssrf_blocked;
pub(crate) use hyper_panel_core::{
    atomic_write, background_poller, config_mtime, fetch_json, gen_random_password,
    generate_node_id, http_request_json, load_events_from_file, load_notifications_from_file,
    load_nodes, log_write, node_visible, now_unix, resolve_safe_addr, save_nodes,
    set_retention_days, sha256_hex, tail_log, urlencode, valid_user_name,
    validate_password_only, validate_user_input, validate_user_name_only, AppState,
    HttpOptions, NodeConfig, NodeState, SharedState, User, AUTH_FILE, EVENTS_FILE, LOG_DIR,
    NOTIF_FILE, SETTINGS_FILE,
};
// hyper-panel - system monitoring panel aggregator (Rust)
// Role: node management (CLI+API) + background agent polling + in-memory cache + frontend
// Commands: help / node add / node del / node list / node show / setup / log / serve

mod api;
mod auth;
mod cli;
mod nodes;

use api::*;
use auth::*;
use nodes::*;

use axum::{
    extract::{Json as JsonExtract, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{delete, get, post, put},
    Router,
};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

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

node_get_handler!(node_traffic_handler, 5, |cfg: &NodeConfig,
                                            params: &HashMap<
    String,
    String,
>| {
    let iface = params.get("iface").cloned().unwrap_or_default();
    format!(
        "{}/traffic{}",
        cfg.base_url(),
        if iface.is_empty() {
            String::new()
        } else {
            format!("?iface={iface}")
        }
    )
});
node_get_handler!(node_disks_handler, 5, |cfg: &NodeConfig,
                                          _params: &HashMap<
    String,
    String,
>| {
    format!("{}/disks", cfg.base_url())
});
node_get_handler!(node_processes_handler, 8, |cfg: &NodeConfig,
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
        "{}/processes?sort={}&limit={}&name={}",
        cfg.base_url(),
        urlencode(&sort),
        urlencode(&limit),
        urlencode(&name)
    )
});
node_get_handler!(
    node_io_handler,
    8,
    |cfg: &NodeConfig, _params: &HashMap<String, String>| { format!("{}/io", cfg.base_url()) }
);

// History trend query: GET /api/node/id/:node_id/history?metric=cpu&range=24h (idx legacy removed)
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
            let points = history::history_query(&ns.config.id, &metric, &range);
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

node_get_handler!(node_docker_handler, 8, |cfg: &NodeConfig,
                                           _params: &HashMap<
    String,
    String,
>| {
    format!("{}/docker", cfg.base_url())
});

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
            match http_request_json(
                &app,
                HttpOptions {
                    host: &ns.config.addr,
                    port: ns.config.port,
                    method: "POST",
                    path: &path,
                    key: &ns.config.key,
                    timeout: Duration::from_secs(10),
                    tls: ns.config.tls,
                    cert_fp: &ns.config.cert_fp,
                    use_pool: false,
                },
            )
            .await
            {
                Ok(v) => (StatusCode::OK, Json(v)).into_response(),
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

async fn cmd_serve(port: u16) -> i32 {
    let auth_file_exists = match validate_auth_file() {
        Ok(exists) => exists,
        Err(error) => {
            eprintln!("failed to load authentication config: {error}");
            return 1;
        }
    };
    let mut auth = load_users();
    if auth_file_exists && auth.is_empty() {
        eprintln!("authentication config contains no valid users");
        return 1;
    }
    if auth.is_empty() {
        // Default admin account: admin/admin (change it with user passwd after first login)
        let hash = match hash_password("admin") {
            Ok(hash) => hash,
            Err(error) => {
                eprintln!("failed to create initial admin password: {error}");
                return 1;
            }
        };
        auth = vec![User {
            name: "admin".to_string(),
            salt: String::new(),
            hash,
            is_admin: true,
        }];
        if let Err(e) = save_users(&auth) {
            eprintln!("failed to create default admin account: {e}");
            return 1;
        }
        println!("default admin account created: admin / admin");
        println!("  change the password: hyper-panel user passwd admin");
    }
    let cfg = load_nodes();
    println!("loaded {} node configs", cfg.len());

    let app_state = Arc::new(AppState {
        nodes: Mutex::new(
            cfg.into_iter()
                .map(|c| NodeState {
                    config: c,
                    data: None,
                    data_ts: 0,
                    traffic_cache: None,
                    io_cache: None,
                    status: "unknown".to_string(),
                })
                .collect(),
        ),
        config_mtime: Mutex::new(config_mtime()),
        events: Mutex::new(load_events_from_file()),
        notifications: Mutex::new(load_notifications_from_file()),
        active_alerts: Mutex::new(HashMap::new()),
        tokens: Mutex::new(Vec::new()),
        auth: Mutex::new(auth),
        conns: Mutex::new(HashMap::new()),
        ws_connections: Mutex::new(HashMap::new()),
        ws_auth_failures: Mutex::new(HashMap::new()),
        allow_trusted_proxies: false,
    });

    let poller_state = app_state.clone();
    tokio::spawn(background_poller(poller_state));

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/static/utils.js", get(static_utils_handler))
        .route("/static/api.js", get(static_api_js_handler))
        .route("/static/nodes.js", get(static_nodes_handler))
        .route("/static/dashboard.js", get(static_dashboard_handler))
        .route("/static/history.js", get(static_history_handler))
        .route("/static/logs.js", get(static_logs_handler))
        .route("/static/forge.min.js", get(static_forge_handler))
        .route("/static/panel.js", get(static_js_handler))
        .route("/static/style.css", get(static_css_handler))
        .route("/api/login", post(login_handler))
        .route("/api/logout", post(logout_handler))
        .route("/api/status", get(status_handler))
        .route(
            "/api/settings",
            get(settings_get_handler).post(settings_post_handler),
        )
        .route("/api/me", get(me_handler))
        .route(
            "/api/users",
            get(users_list_handler).post(users_add_handler),
        )
        .route(
            "/api/users/:name",
            put(users_update_handler).delete(users_delete_handler),
        )
        .route("/api/nodes", get(nodes_handler).post(add_node_handler))
        .route("/api/node/id/:node_id/name", put(rename_node_handler))
        .route("/api/node/id/:node_id/ping", post(ping_node_handler))
        .route("/api/node/id/:node_id", delete(remove_node_handler))
        .route("/api/node/id/:node_id/system", get(node_system_handler))
        .route("/api/node/id/:node_id/traffic", get(node_traffic_handler))
        .route("/api/node/id/:node_id/disks", get(node_disks_handler))
        .route(
            "/api/node/id/:node_id/processes",
            get(node_processes_handler),
        )
        .route("/api/node/id/:node_id/io", get(node_io_handler))
        .route("/api/node/id/:node_id/history", get(node_history_handler))
        .route(
            "/api/node/id/:node_id/history/export",
            get(history_export_handler),
        )
        .route("/api/node/id/:node_id/docker", get(node_docker_handler))
        .route(
            "/api/node/id/:node_id/docker/:container/:action",
            post(node_docker_control_handler),
        )
        .route("/api/node/id/:node_id/reboot", post(node_reboot_handler))
        .route(
            "/api/node/id/:node_id/shutdown",
            post(node_shutdown_handler),
        )
        .route("/ws", get(ws_handler))
        .route("/api/push", post(push_handler))
        .route("/api/events/clear", delete(events_clear_handler))
        .route("/api/events", get(events_handler))
        .route(
            "/api/notifications",
            get(notifications_handler).delete(notifications_clear_handler),
        )
        .route("/health", get(health_handler))
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ))
        .with_state(app_state);

    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind port {port}: {e}");
            return 1;
        }
    };
    println!("hyper-panel started on 0.0.0.0:{port}");
    log_write("INFO", &format!("service started on 0.0.0.0:{port}"));
    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    {
        eprintln!("HTTP server stopped: {e}");
        return 1;
    }
    0
}

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
    let code = match args.first().map(|s| s.as_str()) {
        None | Some("help") | Some("--help") | Some("-h") => {
            crate::cli::print_help();
            0
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("add") => {
            crate::cli::cmd_add_node(&args)
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("link") => {
            crate::cli::cmd_link_node(&args)
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("show") => {
            crate::cli::cmd_node_show(&args)
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("del") => {
            crate::cli::cmd_remove_node_by_alias(&args)
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("rename") => {
            crate::cli::cmd_rename_node(&args)
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("ping") => {
            crate::cli::cmd_ping_node(&args)
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("list") => crate::cli::cmd_nodes(),
        Some("setup") => crate::cli::cmd_setup(&args),
        Some("user") => match args.get(1).map(|s| s.as_str()) {
            Some("add") => crate::cli::cmd_user_add(&args),
            Some("del") | Some("remove") => crate::cli::cmd_user_del(&args),
            Some("passwd") => crate::cli::cmd_user_passwd(&args),
            Some("rename") => crate::cli::cmd_user_rename(&args),
            Some("list") | Some("ls") => crate::cli::cmd_user_list(),
            _ => {
                eprintln!("usage: hyper-panel user <add|del|passwd|rename|list>");
                1
            }
        },
        Some("port") => crate::cli::cmd_port(&args),
        Some("log") => match args.get(1).map(|s| s.as_str()) {
            Some("show") => crate::cli::cmd_log_show(&args),
            Some("system") => crate::cli::cmd_log_system(&args),
            Some("retention") => crate::cli::cmd_log_retention(&args),
            _ => {
                eprintln!(
                    "usage: hyper-panel log show [N] | log system [N] | log retention <days>"
                );
                1
            }
        },
        Some("serve") => {
            let mut port = crate::cli::load_panel_port();
            let mut i = 1;
            while i < args.len() {
                if args[i] == "--port" {
                    if let Some(p) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                        port = p;
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            cmd_serve(port).await
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("run 'hyper-panel help' for usage");
            1
        }
    };
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
        assert_eq!(p, 5000);
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
        assert_eq!(p, 5000);
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
                "name": "", "addr": "1.2.3.4", "port": 5000, "key": "k"
            }))
            .is_none()
        );
        assert!(
            hyper_panel_core::NodeConfig::from_value(&serde_json::json!({
                "name": "n", "addr": "", "port": 5000, "key": "k"
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
                "name": "n", "addr": "1.2.3.4", "port": 5000, "key": "k"
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
