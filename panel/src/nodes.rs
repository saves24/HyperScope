use crate::{
    current_user, fetch_json, http_request_json, is_admin, log_write, node_visible,
    resolve_safe_addr, save_nodes, Duration, HttpOptions, JsonExtract, NodeConfig, NodeState,
    SharedState, StatusCode, EVENTS_FILE, NOTIF_FILE,
};
// Node configuration, state and API handlers
use axum::{
    extract::{Json, Path, State},
    http::HeaderMap,
    response::IntoResponse,
};
use serde_json::{json, Value};
use std::collections::HashMap;

// Write counter: only run the truncate scan every N writes instead of every event

pub(crate) async fn nodes_handler(
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    let user = current_user(&headers);
    // Lock order: nodes -> auth. Keep this order whenever both locks are needed.
    let nodes = app.nodes.lock().await;
    let admin = is_admin(&app, &user).await;
    let list: Vec<Value> = nodes
        .iter()
        .enumerate()
        .filter(|(_, ns)| node_visible(&ns.config, &user, admin))
        .map(|(_, ns)| {
                        // owner returned for admin only (regular users cannot see ownership)
            let owner = if admin { ns.config.owner.clone() } else { String::new() };
            json!({
                "id": ns.config.id,
                "name": ns.config.name,
                "owner": owner,
                "tls": ns.config.tls,
                "cert_verified": !ns.config.tls || !ns.config.cert_fp.is_empty(),
                "status": ns.status,
                "online": ns.status == "online",
                "node_name": ns.data.as_ref().and_then(|d| d["node_name"].as_str()).unwrap_or(&ns.config.name),
                "version": ns.data.as_ref().and_then(|d| d["version"].as_str()).unwrap_or(""),
            })
        })
        .collect();
    Json(json!({ "nodes": list }))
}
// Admin-only: full node list including addr/port/key for .hsxc export.
// Deliberately separate from /api/nodes (which omits keys) so ordinary listing
// never leaks credentials, while a signed-in admin can still export a backup.
pub(crate) async fn nodes_export_handler(
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    let user = current_user(&headers);
    let admin = is_admin(&app, &user).await;
    if !admin {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "admin required" })),
        )
            .into_response();
    }
    let nodes = app.nodes.lock().await;
    let list: Vec<Value> = nodes
        .iter()
        .map(|ns| {
            json!({
                "name": ns.config.name,
                "addr": ns.config.addr,
                "port": ns.config.port,
                "key": ns.config.key,
                "tls": ns.config.tls,
            })
        })
        .collect();
    (StatusCode::OK, Json(json!({ "nodes": list }))).into_response()
}
pub(crate) async fn add_node_handler(
    headers: HeaderMap,
    State(app): State<SharedState>,
    JsonExtract(body): JsonExtract<Value>,
) -> impl IntoResponse {
    // Simple check: request header carries admin token (optional)
    if let Some(token) = std::env::var("PANEL_TOKEN").ok().filter(|t| !t.is_empty()) {
        let provided = headers
            .get("x-panel-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided != token {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    }
    let Some(mut cfg) = NodeConfig::from_value(&body) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "JSON body required: {name, addr, port, key}"})),
        )
            .into_response();
    };
    cfg.ensure_id();
    // Attribute node to the currently logged-in user
    let user = current_user(&headers);
    cfg.owner = user.clone();

    // Resolve and persist the checked IP so later connections cannot race DNS.
    cfg.addr = match resolve_safe_addr(&cfg.addr, cfg.port) {
        Ok(addr) => addr,
        Err(error) => {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": error}))).into_response()
        }
    };

    // key contains cert fingerprint (secret|SHA256:fp) -> auto-enable TLS and record fingerprint
    let key_with_fp = cfg.key.clone();
    let mut key_has_fp = false;
    if let Some((pure_key, fp)) = key_with_fp.split_once('|') {
        cfg.key = pure_key.trim().to_string();
        cfg.tls = true;
        cfg.cert_fp = fp.trim().to_string();
        key_has_fp = true;
    }
    // Plaintext key cannot enable TLS (no fingerprint to verify; connection would fail)
    if cfg.tls && !key_has_fp && cfg.cert_fp.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "plain key cannot enable TLS, use hyper-node key show to get the full key with certificate fingerprint" }))).into_response();
    }

    // Protocol probe: plaintext fails but reverse TLS succeeds -> node is encrypted; refuse to add
    if !cfg.tls {
        let probe_plain = http_request_json(
            &app,
            HttpOptions {
                host: &cfg.addr,
                port: cfg.port,
                method: "GET",
                path: "/",
                key: &cfg.key,
                timeout: Duration::from_secs(3),
                tls: false,
                cert_fp: "",
                use_pool: true,
            },
        )
        .await;
        if probe_plain.is_err() {
            // Plaintext failed: try reverse TLS (TOFU, no fingerprint) to detect encrypted mode
            // TLS handshake OK + HTTP response received (even 401/503) -> server is encrypted
            let probe_tls = http_request_json(
                &app,
                HttpOptions {
                    host: &cfg.addr,
                    port: cfg.port,
                    method: "GET",
                    path: "/",
                    key: &cfg.key,
                    timeout: Duration::from_secs(3),
                    tls: true,
                    cert_fp: "",
                    use_pool: true,
                },
            )
            .await;
            let tls_ok = match &probe_tls {
                Ok(_) => true,
                Err(e) => e.starts_with("HTTP ") || e.starts_with("unauthorized"),
            };
            log_write(
                "INFO",
                &format!(
                    "node add probe {}:{} plain_err={:?} tls_ok={} tls_err={:?}",
                    cfg.addr,
                    cfg.port,
                    probe_plain.err(),
                    tls_ok,
                    probe_tls.err()
                ),
            );
            if tls_ok {
                return (StatusCode::BAD_REQUEST, Json(json!({"error": "node server is TLS encrypted, plain key cannot connect. On the node run: hyper-node key show, then paste the full key (with certificate fingerprint) and enable TLS" }))).into_response();
            }
            // Both failed: node offline, allow adding (offline nodes can be configured)
        }
    }

    // Lock order: nodes -> auth. Keep this order whenever both locks are needed.
    let mut nodes = app.nodes.lock().await;
    // Node names must be globally unique (name is the internal key for history/events)
    if nodes.iter().any(|n| n.config.name == cfg.name) {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": "node already exists"})),
        )
            .into_response();
    }
    let mut list: Vec<NodeConfig> = nodes.iter().map(|n| n.config.clone()).collect();
    list.push(cfg.clone());
    match save_nodes(&list) {
        Ok(()) => {
            nodes.push(NodeState {
                config: cfg.clone(),
                data: None,
                data_ts: 0,
                traffic_cache: None,
                io_cache: None,
                status: "unknown".to_string(),
            });
            (StatusCode::OK, Json(json!({"ok": true, "name": cfg.name}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response(),
    }
}
pub(crate) async fn rename_node_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
    axum::extract::Path(ident): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let Some(new_name) = body
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "field {name} required"})),
        )
            .into_response();
    };
    if new_name.is_empty() || new_name.len() > 100 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "invalid name"})),
        )
            .into_response();
    }
    let user = current_user(&headers);
    // Lock order: nodes -> auth.
    let mut nodes = app.nodes.lock().await;
    let admin = is_admin(&app, &user).await;
    // Check for duplicate name first (immutable borrow)
    let dup = nodes.iter().any(|n| n.config.name == new_name);
    let Some(pos) = nodes.iter().position(|n| {
        (n.config.id == ident || n.config.name == ident) && node_visible(&n.config, &user, admin)
    }) else {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "access denied"})),
        )
            .into_response();
    };
    if dup {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": "node name already exists"})),
        )
            .into_response();
    }
    let ns = nodes.get_mut(pos).unwrap();
    ns.config.name = new_name.clone();
    let list: Vec<NodeConfig> = nodes.iter().map(|n| n.config.clone()).collect();
    drop(nodes);
    match save_nodes(&list) {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true, "name": new_name}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response(),
    }
}
pub(crate) async fn ping_node_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
    axum::extract::Path(ident): axum::extract::Path<String>,
) -> impl IntoResponse {
    let nodes = app.nodes.lock().await;
    // Lock order: nodes -> auth.
    let user = current_user(&headers);
    let admin = is_admin(&app, &user).await;
    let Some(ns) = nodes.iter().find(|n| {
        (n.config.id == ident || n.config.name == ident) && node_visible(&n.config, &user, admin)
    }) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "node not found"})),
        )
            .into_response();
    };
    let addr = ns.config.addr.clone();
    drop(nodes);
    // Run ping (4 packets, max 2s each)
    let output = tokio::process::Command::new("ping")
        .args(["-c", "4", "-W", "2", &addr])
        .output()
        .await;
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let text = if !stdout.trim().is_empty() {
                stdout
            } else {
                stderr
            };
            // Reachable when output has "ttl=" or "time=" and exit succeeds
            let ok = out.status.success() || text.contains("ttl=") || text.contains("time=");
            (StatusCode::OK, Json(json!({"ok": ok, "output": text}))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("ping failed: {e}")})),
        )
            .into_response(),
    }
}
pub(crate) async fn remove_node_handler(
    Path(node_id): Path<String>,
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    if let Some(token) = std::env::var("PANEL_TOKEN").ok().filter(|t| !t.is_empty()) {
        let provided = headers
            .get("x-panel-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if provided != token {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    }
    // Lock order: nodes -> auth. Keep this order whenever both locks are needed.
    let mut nodes = app.nodes.lock().await;
    let before = nodes.len();
    let user = current_user(&headers);
    let admin = is_admin(&app, &user).await;
    let mut list: Vec<NodeConfig> = nodes.iter().map(|n| n.config.clone()).collect();
    // Remove only nodes visible to current user (admin can delete any; regular users only their own)
    let visible_ids: Vec<String> = list
        .iter()
        .filter(|n| node_visible(n, &user, admin))
        .map(|n| n.id.clone())
        .collect();
    list.retain(|n| !(n.id == node_id && node_visible(n, &user, admin)));
    if !visible_ids.contains(&node_id) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "no permission to delete this node"})),
        )
            .into_response();
    }
    if list.len() == before {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "node not found"})),
        )
            .into_response();
    }
    match save_nodes(&list) {
        Ok(()) => {
            nodes.retain(|n| !(n.config.id == node_id && node_visible(&n.config, &user, admin)));
            (StatusCode::OK, Json(json!({"ok": true, "id": node_id}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response(),
    }
}
// Resolve a node by either numeric index (legacy) or stable id (preferred)
pub(crate) async fn visible_node_by_ident(
    app: &SharedState,
    headers: &HeaderMap,
    ident: &str,
) -> Option<NodeState> {
    let nodes = app.nodes.lock().await;
    // Lock order: nodes -> auth.
    let user = current_user(headers);
    let admin = is_admin(app, &user).await;
    nodes
        .iter()
        .find(|ns| ns.config.id == ident)
        .filter(|ns| node_visible(&ns.config, &user, admin))
        .cloned()
}
pub(crate) async fn node_fetch_get(
    app: &SharedState,
    headers: &HeaderMap,
    ident: &str,
    params: &HashMap<String, String>,
    timeout: u64,
    url_fn: impl Fn(&NodeConfig, &HashMap<String, String>) -> String,
) -> axum::response::Response {
    match visible_node_by_ident(app, headers, ident).await {
        Some(ns) => {
            // Push-mode nodes have no listening port: serve cached pushed metrics
            if ns.config.push {
                let app_inner = app.nodes.lock().await;
                if let Some(cur) = app_inner.iter().find(|x| x.config.id == ns.config.id) {
                    let u = url_fn(&ns.config, params);
                    return if u.contains("/traffic") {
                        (
                            StatusCode::OK,
                            Json(cur.traffic_cache.clone().unwrap_or(json!({}))),
                        )
                            .into_response()
                    } else if u.contains("/io") {
                        (
                            StatusCode::OK,
                            Json(cur.io_cache.clone().unwrap_or(json!({}))),
                        )
                            .into_response()
                    } else {
                        (
                            StatusCode::OK,
                            Json(cur.data.clone().unwrap_or(json!({"error": "no data yet"}))),
                        )
                            .into_response()
                    };
                }
            }
            let url = url_fn(&ns.config, params);
            match fetch_json(
                app,
                &url,
                &ns.config.key,
                Duration::from_secs(timeout),
                &ns.config.cert_fp,
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
pub(crate) async fn node_fetch_post(
    app: &SharedState,
    headers: &HeaderMap,
    ident: &str,
    path: &str,
    action: &str,
) -> axum::response::Response {
    match visible_node_by_ident(app, headers, ident).await {
        Some(ns) => {
            match http_request_json(app, HttpOptions { host: &ns.config.addr, port: ns.config.port, method: "POST", path, key: &ns.config.key, timeout: Duration::from_secs(10), tls: ns.config.tls, cert_fp: &ns.config.cert_fp, use_pool: false }).await {
                Ok(v) => (StatusCode::OK, Json(json!({"ok": v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false), "action": action}))).into_response(),
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
pub(crate) async fn node_system_handler(
    Path(ident): Path<String>,
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    match visible_node_by_ident(&app, &headers, &ident).await {
        Some(ns) => match &ns.data {
            Some(d) => {
                let mut v = d.clone();
                v["status"] = json!(ns.status);
                (StatusCode::OK, Json(v)).into_response()
            }
            None => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"status": ns.status, "error": "node data unavailable"})),
            )
                .into_response(),
        },
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "node not found or access denied"})),
        )
            .into_response(),
    }
}
pub(crate) async fn node_reboot_handler(
    Path(ident): Path<String>,
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    node_fetch_post(&app, &headers, &ident, "/reboot", "reboot").await
}
pub(crate) async fn node_shutdown_handler(
    Path(ident): Path<String>,
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    node_fetch_post(&app, &headers, &ident, "/shutdown", "shutdown").await
}
pub(crate) async fn events_handler(
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    // Lock order: nodes -> auth. Snapshot visible names before releasing nodes.
    let nodes = app.nodes.lock().await;
    let user = current_user(&headers);
    let admin = is_admin(&app, &user).await;
    let visible: Vec<String> = if admin {
        Vec::new()
    } else {
        nodes
            .iter()
            .filter(|ns| node_visible(&ns.config, &user, false))
            .map(|ns| ns.config.name.clone())
            .collect()
    };
    drop(nodes);
    let events = app.events.lock().await;
    let list: Vec<Value> = if admin {
        events.clone()
    } else {
        events
            .iter()
            .filter(|ev| {
                let node = ev.get("node").and_then(|n| n.as_str()).unwrap_or("");
                visible.contains(&node.to_string())
            })
            .cloned()
            .collect()
    };
    drop(events);
    Json(json!({ "events": list }))
}

// Clear the event log (admin only)
pub(crate) async fn events_clear_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = current_user(&headers);
    let admin = is_admin(&app, &user).await;
    if !admin {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "admin required"})),
        )
            .into_response();
    }
    // Clear in-memory events
    {
        let mut ev = app.events.lock().await;
        ev.clear();
    }
    // Clear persisted event file
    let _ = crate::atomic_write(EVENTS_FILE, "", 0o600);
    (StatusCode::OK, Json(json!({"ok": true}))).into_response()
}

// GET /api/notifications — alert notifications only (separate from event log)
pub(crate) async fn notifications_handler(
    State(app): State<SharedState>,
) -> impl IntoResponse {
    let notifs = app.notifications.lock().await;
    Json(json!({ "notifications": notifs.clone() }))
}

// DELETE /api/notifications — clear alert notifications only (never touches events)
pub(crate) async fn notifications_clear_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = current_user(&headers);
    let admin = is_admin(&app, &user).await;
    if !admin {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "admin required"})),
        )
            .into_response();
    }
    app.notifications.lock().await.clear();
    let _ = crate::atomic_write(NOTIF_FILE, "", 0o600);
    (StatusCode::OK, Json(json!({"ok": true}))).into_response()
}
