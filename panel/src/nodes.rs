use crate::{
    current_user, http_request_json, is_admin, log_write, node_visible, record_admin_action,
    resolve_safe_addr, save_nodes, Duration, HttpOptions, JsonExtract, NodeConfig, NodeState,
    SharedState, StatusCode,
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
                "webhook": ns.config.webhook,
                "alert_cpu": ns.config.alert_cpu,
                "alert_mem": ns.config.alert_mem,
                "alert_disk": ns.config.alert_disk,
                "alert_temp": ns.config.alert_temp,
                "group": ns.config.group,
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
                "push": ns.config.push,
                // Node Ed25519 identity public key (device trust for relay
                // commands). Read from the node when reachable; empty when not.
                "node_pubkey": "",
            })
        })
        .collect();
    // Embed the panel identity private key so imported clients (Android) can
    // sign commands as "panel" and be accepted by nodes that trust it. The
    // .hsxc file is AES-encrypted with a user passphrase, so the key is not
    // exposed in plaintext. Omit on error (identity not yet generated).
    let identity_key = hyper_panel_core::identity::identity_private_b64().unwrap_or_default();
    (
        StatusCode::OK,
        Json(json!({ "nodes": list, "identity_key": identity_key })),
    )
        .into_response()
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
pub(crate) async fn update_node_alerts_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
    axum::extract::Path(ident): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let user = current_user(&headers);
    let mut nodes = app.nodes.lock().await;
    let admin = is_admin(&app, &user).await;
    let Some(pos) = nodes.iter().position(|n| {
        (n.config.id == ident || n.config.name == ident) && node_visible(&n.config, &user, admin)
    }) else {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "access denied"})),
        )
            .into_response();
    };
    // Only alert-related fields may be updated here (webhook + thresholds).
    let ns = nodes.get_mut(pos).unwrap();
    if let Some(w) = body.get("webhook") {
        ns.config.webhook = w.as_str().unwrap_or("").to_string();
    }
    if let Some(v) = body.get("alert_cpu") {
        ns.config.alert_cpu = v.as_f64();
    }
    if let Some(v) = body.get("alert_mem") {
        ns.config.alert_mem = v.as_f64();
    }
    if let Some(v) = body.get("alert_disk") {
        ns.config.alert_disk = v.as_f64();
    }
    if let Some(v) = body.get("alert_temp") {
        ns.config.alert_temp = v.as_f64();
    }
    if let Some(g) = body.get("group") {
        ns.config.group = g.as_str().unwrap_or("").to_string();
    }
    let list: Vec<NodeConfig> = nodes.iter().map(|n| n.config.clone()).collect();
    drop(nodes);
    match save_nodes(&list) {
        Ok(()) => (StatusCode::OK, Json(json!({"ok": true}))).into_response(),
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
            let removed_name = list
                .iter()
                .find(|n| n.id == node_id)
                .map(|n| n.name.clone())
                .unwrap_or(node_id.clone());
            nodes.retain(|n| !(n.config.id == node_id && node_visible(&n.config, &user, admin)));
            drop(nodes);
            record_admin_action(
                &app,
                &user,
                format!("deleted node {removed_name} ({node_id})"),
            )
            .await;
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
    _timeout: u64,
    url_fn: impl Fn(&NodeConfig, &HashMap<String, String>) -> String,
) -> axum::response::Response {
    match visible_node_by_ident(app, headers, ident).await {
        Some(ns) => {
            // All nodes are reached through their hyper-relay: serve from the
            // cached polled snapshot (the relay collects a full snapshot per
            // poll). No direct HTTP path.
            if ns.config.push {
                let app_inner = app.nodes.lock().await;
                if let Some(cur) = app_inner.iter().find(|x| x.config.id == ns.config.id) {
                    let u = url_fn(&ns.config, params);
                    let data = cur.data.clone().unwrap_or(json!({}));
                    return if u.contains("/traffic") {
                        (
                            StatusCode::OK,
                            Json(data.get("traffic").cloned().unwrap_or(json!({}))),
                        )
                            .into_response()
                    } else if u.contains("/io") {
                        (
                            StatusCode::OK,
                            Json(data.get("io").cloned().unwrap_or(json!({}))),
                        )
                            .into_response()
                    } else if u.contains("/docker") {
                        (
                            StatusCode::OK,
                            Json(data.get("docker").cloned().unwrap_or(json!([]))),
                        )
                            .into_response()
                    } else if u.contains("/processes") {
                        (
                            StatusCode::OK,
                            Json(data.get("processes_list").cloned().unwrap_or(json!([]))),
                        )
                            .into_response()
                    } else if u.contains("/disks") {
                        (
                            StatusCode::OK,
                            Json(data.get("disks").cloned().unwrap_or(json!([]))),
                        )
                            .into_response()
                    } else {
                        (StatusCode::OK, Json(data)).into_response()
                    };
                }
            }
            // Non-relay fallback removed: all nodes run in relay mode and the
            // collector opens no listening port. Always serve from the polled
            // snapshot; if the node has no cached data yet, return empty.
            (StatusCode::OK, Json(json!({}))).into_response()
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
            // Control commands go through the node's hyper-relay (spawns the
            // collector process on the same machine). No direct HTTP. The
            // panel signs the command with its Ed25519 identity so the relay
            // and collector can verify it (the node must trust the panel
            // device: `hyper-node device add panel <pubkey> admin`).
            let relay_addr = format!("{}:{}", ns.config.addr, 8686);
            let (device_id, signature) = match hyper_panel_core::identity::ensure_identity() {
                Ok(pubkey) => {
                    let msg = format!("{path}:panel");
                    match hyper_panel_core::identity::sign_with_identity(&msg) {
                        Ok(sig) => ("panel".to_string(), sig),
                        Err(_) => (pubkey, String::new()),
                    }
                }
                Err(_) => (String::new(), String::new()),
            };
            match hyper_panel_core::relay_client::send_command(
                &relay_addr,
                &ns.config.name,
                path,
                ns.config.tls,
                &device_id,
                &signature,
            )
            .await
            {
                Ok(v) => {
                    // v is the relay's cmd_result.result: a JSON string like
                    // {"type":"result","ok":true,...}. Parse it for the ok flag.
                    let inner = v
                        .as_str()
                        .and_then(|s| serde_json::from_str::<Value>(s).ok());
                    let ok = inner
                        .as_ref()
                        .and_then(|x| x.get("ok"))
                        .and_then(|x| x.as_bool())
                        .unwrap_or(false);
                    let err = inner
                        .as_ref()
                        .and_then(|x| x.get("error"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("");
                    if !ok {
                        log_write(
                            "WARN",
                            &format!("cmd {action} on {} failed: {}", ns.config.name, err),
                        );
                    }
                    (
                        StatusCode::OK,
                        Json(json!({"ok": ok, "action": action, "error": err})),
                    )
                        .into_response()
                }
                Err(e) => {
                    log_write(
                        "WARN",
                        &format!("cmd {action} on {} relay error: {e}", ns.config.name),
                    );
                    (StatusCode::BAD_GATEWAY, Json(json!({"error": e}))).into_response()
                }
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
