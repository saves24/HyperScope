// Event + notification API handlers (separate from node CRUD).
use crate::{
    atomic_write, current_user, is_admin, node_visible, SharedState, EVENTS_FILE, NOTIF_FILE,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde_json::{json, Value};

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
    let _ = atomic_write(EVENTS_FILE, "", 0o600);
    (StatusCode::OK, Json(json!({"ok": true}))).into_response()
}

// GET /api/notifications — alert notifications only (separate from event log)
pub(crate) async fn notifications_handler(
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
    let notifs = app.notifications.lock().await;
    Json(json!({ "notifications": notifs.clone() })).into_response()
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
    let _ = atomic_write(NOTIF_FILE, "", 0o600);
    (StatusCode::OK, Json(json!({"ok": true}))).into_response()
}
