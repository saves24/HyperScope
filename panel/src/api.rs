use crate::{
    current_user, is_admin, SharedState, DEFAULT_PORT, INDEX_HTML, PANEL_JS, STYLE_CSS, VERSION,
};
// Misc API handlers: status/setup/settings/static
use axum::{
    extract::{connect_info::ConnectInfo, Json, State, WebSocketUpgrade},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use hyper_panel_core::{PushPayload, PROTOCOL_VERSION};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time::timeout;

pub(crate) const UTILS_JS: &str = include_str!("../static/utils.js");
pub(crate) const API_JS: &str = include_str!("../static/api.js");
pub(crate) const NODES_JS: &str = include_str!("../static/nodes.js");
pub(crate) const DASHBOARD_JS: &str = include_str!("../static/dashboard.js");
pub(crate) const HISTORY_JS: &str = include_str!("../static/history.js");
pub(crate) const LOGS_JS: &str = include_str!("../static/logs.js");
pub(crate) const FORGE_JS: &str = include_str!("../static/forge.min.js");

pub(crate) async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}
pub(crate) async fn status_handler(State(app): State<SharedState>) -> impl IntoResponse {
    let auth = app.auth.lock().await.clone();
    Json(json!({
        "auth_required": !auth.is_empty(),
        "version": VERSION,
    }))
}
pub(crate) async fn settings_get_handler(
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    if !is_admin(&app, &current_user(&headers)).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "only admin can view settings"})),
        )
            .into_response();
    }
    Json(json!({ "panel_port": crate::cli::load_panel_port() })).into_response()
}
pub(crate) fn port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("0.0.0.0", port)).is_err()
}
pub(crate) async fn settings_post_handler(
    headers: HeaderMap,
    State(app): State<SharedState>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    if !is_admin(&app, &current_user(&headers)).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "only admin can change settings"})),
        )
            .into_response();
    }
    let panel_port = body
        .get("panel_port")
        .and_then(|v| v.as_u64())
        .filter(|p| (1..=65535).contains(p))
        .map(|p| p as u16)
        .unwrap_or(DEFAULT_PORT);
    if panel_port == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "port cannot be 0"})),
        )
            .into_response();
    }
    let old_port = crate::cli::load_panel_port();
    // Port conflict check: reject if target port is held by another process (current port excluded)
    if panel_port != old_port && port_in_use(panel_port) {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": format!("port {panel_port} is in use, try another")})),
        )
            .into_response();
    }
    match crate::cli::save_panel_port(panel_port) {
        Ok(()) => {
            let changed = panel_port != old_port;
            let restarting = if changed { trigger_restart() } else { false };
            (
                StatusCode::OK,
                Json(json!({"ok": true, "restarting": restarting, "port": panel_port})),
            )
                .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response(),
    }
}
pub(crate) fn trigger_restart() -> bool {
    // Confirm systemd service exists
    let check = std::process::Command::new("systemctl")
        .args(["is-active", "hyper-panel"])
        .output();
    let active = matches!(check, Ok(o) if String::from_utf8_lossy(&o.stdout).trim() == "active");
    if !active {
        return false;
    }
    let restart = std::process::Command::new("sh")
        .args(["-c", "sleep 1 && systemctl restart hyper-panel"])
        .spawn();
    restart.is_ok()
}
pub(crate) async fn index_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        INDEX_HTML,
    )
}
pub(crate) async fn static_js_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        PANEL_JS,
    )
}

pub(crate) async fn static_utils_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        UTILS_JS,
    )
}
pub(crate) async fn static_api_js_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        API_JS,
    )
}
pub(crate) async fn static_nodes_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        NODES_JS,
    )
}
pub(crate) async fn static_forge_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        FORGE_JS,
    )
}
pub(crate) async fn static_dashboard_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        DASHBOARD_JS,
    )
}
pub(crate) async fn static_history_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        HISTORY_JS,
    )
}
pub(crate) async fn static_logs_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        LOGS_JS,
    )
}
pub(crate) async fn static_css_handler() -> impl IntoResponse {
    (
        [
            (axum::http::header::CONTENT_TYPE, "text/css"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        STYLE_CSS,
    )
}

// Reverse push: node actively POSTs metrics (no listening port required on the node)
pub(crate) async fn push_handler(
    State(app): State<SharedState>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let payload = match serde_json::from_value::<PushPayload>(body.clone()) {
        Ok(payload) => payload,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid push payload"})),
            )
                .into_response();
        }
    };
    let protocol_version = body
        .get("protocol_version")
        .and_then(Value::as_str)
        .unwrap_or("1");
    if protocol_version != PROTOCOL_VERSION {
        return (
            StatusCode::UPGRADE_REQUIRED,
            Json(json!({"error": "unsupported protocol version", "protocol_version": PROTOCOL_VERSION})),
        )
            .into_response();
    }
    let name = payload.name.as_str();
    let key = payload.key.as_str();
    if name.is_empty() || key.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "missing credentials"})),
        )
            .into_response();
    }
    let ts = payload.ts;
    let mut nodes = app.nodes.lock().await;
    if let Some(ns) = nodes.iter_mut().find(|n| n.config.name == name) {
        if crate::auth::constant_time_eq(&ns.config.key, key) {
            if let Some(d) = body.get("data") {
                // Dedupe: only accept newer push (WS and POST may both deliver)
                if ts == 0 || ts >= ns.data_ts {
                    ns.data = Some(d.clone());
                    ns.data_ts = ts;
                }
            }
            ns.status = "online".to_string();
            return (StatusCode::OK, Json(json!({"ok": true, "name": name}))).into_response();
        }
    }
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "unauthorized"})),
    )
        .into_response()
}

// WebSocket: node keeps a long-lived connection and pushes metrics in real time
const WS_AUTH_TIMEOUT: Duration = Duration::from_secs(30);
const WS_MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MB
const WS_MAX_CONNECTIONS_PER_IP: u32 = 5;
const WS_AUTH_BAN_THRESHOLD: u32 = 5;
const WS_AUTH_BAN_DURATION: std::time::Duration = std::time::Duration::from_secs(300); // 5 min

async fn is_ip_banned(app: &SharedState, addr: &SocketAddr) -> bool {
    let mut failures = app.ws_auth_failures.lock().await;
    if let Some(&(count, ban_until)) = failures.get(addr) {
        if count >= WS_AUTH_BAN_THRESHOLD && std::time::Instant::now() < ban_until {
            return true;
        }
        if std::time::Instant::now() >= ban_until {
            failures.remove(addr);
        }
    }
    false
}

async fn record_auth_failure(app: &SharedState, addr: &SocketAddr) {
    let mut failures = app.ws_auth_failures.lock().await;
    let entry = failures
        .entry(*addr)
        .or_insert((0, std::time::Instant::now()));
    entry.0 += 1;
    if entry.0 >= WS_AUTH_BAN_THRESHOLD {
        entry.1 = std::time::Instant::now() + WS_AUTH_BAN_DURATION;
    }
}

pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Resolve real IP: use X-Forwarded-For when proxy mode is enabled
    let real_addr = if app.allow_trusted_proxies {
        headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .and_then(|v| v.trim().parse::<SocketAddr>().ok())
            .unwrap_or(addr)
    } else {
        addr
    };
    // Circuit breaker: reject banned IPs
    if is_ip_banned(&app, &real_addr).await {
        return axum::response::Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .body(axum::body::Body::from(
                "too many authentication failures, try again later",
            ))
            .unwrap();
    }
    // Rate limit: reject when IP already has too many WS connections
    {
        let mut ws_conns = app.ws_connections.lock().await;
        let count = ws_conns.entry(real_addr).or_insert(0);
        if *count >= WS_MAX_CONNECTIONS_PER_IP {
            return axum::response::Response::builder()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .body(axum::body::Body::from(
                    "too many WebSocket connections from this IP",
                ))
                .unwrap();
        }
        *count += 1;
    }
    ws.on_upgrade(move |socket| handle_ws(socket, app, real_addr))
}

async fn handle_ws(mut socket: axum::extract::ws::WebSocket, app: SharedState, addr: SocketAddr) {
    use axum::extract::ws::Message;
    // Auth timeout: first message must arrive within 30 seconds
    let auth_msg = match timeout(WS_AUTH_TIMEOUT, socket.recv()).await {
        Ok(Some(Ok(Message::Text(t)))) => t.to_string(),
        Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
            let _ = socket.close().await;
            cleanup_ws(&app, addr).await;
            return;
        }
        Ok(Some(Err(_))) => {
            let _ = socket.close().await;
            cleanup_ws(&app, addr).await;
            return;
        }
        Ok(Some(Ok(_))) => {
            // Binary, Ping, Pong — ignore and wait for auth
            let _ = socket.close().await;
            cleanup_ws(&app, addr).await;
            return;
        }
        Err(_) => {
            // Auth timeout
            let _ = socket
                .send(Message::Text("{\"error\":\"auth timeout\"}".into()))
                .await;
            let _ = socket.close().await;
            cleanup_ws(&app, addr).await;
            return;
        }
    };
    let v: Value = match serde_json::from_str(&auth_msg) {
        Ok(v) => v,
        Err(_) => {
            let _ = socket.close().await;
            cleanup_ws(&app, addr).await;
            return;
        }
    };
    let protocol_version = v["protocol_version"].as_str().unwrap_or("1");
    if protocol_version != PROTOCOL_VERSION {
        let _ = socket
            .send(Message::Text(
                "{\"error\":\"unsupported protocol version\"}".into(),
            ))
            .await;
        cleanup_ws(&app, addr).await;
        return;
    }
    let name = v["name"].as_str().unwrap_or("");
    let key = v["key"].as_str().unwrap_or("");
    // Auth check
    let authed = {
        let nodes = app.nodes.lock().await;
        nodes
            .iter()
            .any(|n| n.config.name == name && crate::auth::constant_time_eq(&n.config.key, key))
    };
    if !authed {
        record_auth_failure(&app, &addr).await;
        let _ = socket
            .send(Message::Text("{\"error\":\"unauthorized\"}".into()))
            .await;
        cleanup_ws(&app, addr).await;
        return;
    }
    // Authed: apply data carried in the auth message, then keep receiving
    if let Some(d) = v.get("data") {
        let ts = v["ts"].as_u64().unwrap_or(0);
        let mut nodes = app.nodes.lock().await;
        if let Some(ns) = nodes.iter_mut().find(|n| n.config.name == name) {
            if ts == 0 || ts >= ns.data_ts {
                ns.data = Some(d.clone());
                ns.data_ts = ts;
            }
            ns.status = "online".to_string();
        }
    }
    let _ = socket.send(Message::Text("{\"ok\":true}".into())).await;
    loop {
        match socket.recv().await {
            Some(Ok(Message::Text(t))) => {
                if t.len() > WS_MAX_MESSAGE_SIZE {
                    continue;
                }
                if let Ok(d) = serde_json::from_str::<Value>(&t) {
                    let ts = d["ts"].as_u64().unwrap_or(0);
                    let mut nodes = app.nodes.lock().await;
                    if let Some(ns) = nodes.iter_mut().find(|n| n.config.name == name) {
                        if let Some(data) = d.get("data") {
                            if ts == 0 || ts >= ns.data_ts {
                                ns.data = Some(data.clone());
                                ns.data_ts = ts;
                            }
                            ns.traffic_cache = d.get("traffic").cloned();
                            ns.io_cache = d.get("io").cloned();
                        }
                        ns.status = "online".to_string();
                    }
                }
            }
            Some(Ok(Message::Close(_))) | None => {
                cleanup_ws(&app, addr).await;
                return;
            }
            _ => {}
        }
    }
}

async fn cleanup_ws(app: &SharedState, addr: SocketAddr) {
    let mut ws_conns = app.ws_connections.lock().await;
    if let Some(count) = ws_conns.get_mut(&addr) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            ws_conns.remove(&addr);
        }
    }
}
