use crate::{
    auth_required, docker_action, get_docker_containers, get_io_stats, get_system_cached, header,
    platform, HashMap, HeaderValue, StatusCode, VERSION,
};
// API handlers for hyper-node (key-authenticated HTTP endpoints)
use axum::{
    extract::Query,
    http::HeaderMap,
    middleware,
    response::{IntoResponse, Json},
};
use serde_json::json;

pub(crate) async fn cors_middleware(
    request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let mut response = next.run(request).await;
    // No cross-origin by default; set NODE_AGENT_ALLOW_ORIGIN to allow a specific origin
    if let Ok(allow) = std::env::var("NODE_AGENT_ALLOW_ORIGIN") {
        if !allow.is_empty() {
            if let Ok(val) = HeaderValue::from_str(&allow) {
                let headers = response.headers_mut();
                headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, val);
                headers.insert(
                    header::ACCESS_CONTROL_ALLOW_METHODS,
                    HeaderValue::from_static("GET, OPTIONS"),
                );
                headers.insert(
                    header::ACCESS_CONTROL_ALLOW_HEADERS,
                    HeaderValue::from_static("Authorization, X-API-Key, Content-Type"),
                );
            }
        }
    }
    response
}

// ---------- Simple state cache ----------
// Process CPU sampling cache: (sample time, total CPU ticks, pid -> (proc ticks))

// ---------- Temperature ----------

// ---------- CPU ----------

// ---------- Memory ----------

// ---------- Disk ----------

// ---------- Network speed ----------

// ---------- Processes ----------

// ---------- System logs ----------

// ---------- Listening ports ----------

// ---------- System data aggregation ----------

// ---------- API routes (all require key auth) ----------

pub(crate) async fn status_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    let s = tokio::task::spawn_blocking(get_system_cached)
        .await
        .unwrap_or_default();
    (
        StatusCode::OK,
        Json(json!({
            "service": "hyper-node",
            "version": VERSION,
            "cpu": s["cpu"],
            "cpu_temp": s["cpu_temp"],
            "mem_percent": s["mem_percent"],
            "processes": s["processes"],
        })),
    )
        .into_response()
}

pub(crate) async fn system_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    let s = tokio::task::spawn_blocking(get_system_cached)
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(s)).into_response()
}

pub(crate) async fn traffic_handler(
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    let iface = params.get("iface").cloned();
    let v = tokio::task::spawn_blocking(move || platform::get_traffic(iface.as_deref()))
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(v)).into_response()
}

pub(crate) async fn disks_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    (
        StatusCode::OK,
        Json(json!({ "disks": platform::get_disks() })),
    )
        .into_response()
}

pub(crate) async fn processes_handler(
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    let sort = params.get("sort").map(|s| s.as_str()).unwrap_or("mem");
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20)
        .min(500);
    let name = params.get("name").map(|s| s.as_str()).unwrap_or("");
    (
        StatusCode::OK,
        Json(json!({ "processes": platform::get_processes(sort, limit, name) })),
    )
        .into_response()
}

// Disk I/O rate + TCP connection count
pub(crate) async fn io_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    let v = tokio::task::spawn_blocking(get_io_stats)
        .await
        .unwrap_or_default();
    (StatusCode::OK, Json(v)).into_response()
}

pub(crate) async fn ports_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    (StatusCode::OK, Json(crate::platform::get_ports())).into_response()
}
pub(crate) async fn wifi_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    (StatusCode::OK, Json(crate::platform::get_wifi_signal())).into_response()
}
pub(crate) async fn logs_handler(
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(50)
        .min(500);
    (StatusCode::OK, Json(crate::platform::get_event_log(limit))).into_response()
}

// Docker container list (all containers, running + stopped)
pub(crate) async fn docker_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    #[cfg(target_os = "windows")]
    {
        return (
            StatusCode::OK,
            Json(crate::platform::get_docker_containers()),
        )
            .into_response();
    }
    #[cfg(target_os = "linux")]
    {
        let containers = tokio::task::spawn_blocking(get_docker_containers)
            .await
            .unwrap_or_default();
        (StatusCode::OK, Json(json!({ "containers": containers }))).into_response()
    }
}

// Docker container control: POST /docker/:id/:action (start|stop|restart)
pub(crate) async fn docker_control_handler(
    axum::extract::Path(params): axum::extract::Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    #[cfg(target_os = "windows")]
    {
        let (container, action) = params;
        return (
            StatusCode::OK,
            Json(crate::platform::docker_action(&container, &action)),
        )
            .into_response();
    }
    #[cfg(target_os = "linux")]
    {
        let (container, action) = params;
        match docker_action(&container, &action) {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e}))).into_response(),
        }
    }
}

// Reboot/shutdown: require key auth, run system command
pub(crate) async fn reboot_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    #[cfg(target_os = "windows")]
    let res = std::process::Command::new("shutdown")
        .args(["/r", "/t", "0"])
        .status();
    #[cfg(target_os = "linux")]
    let res = std::process::Command::new("systemctl")
        .arg("reboot")
        .status();
    match res {
        Ok(s) if s.success() => (
            StatusCode::OK,
            Json(json!({"ok": true, "action": "reboot"})),
        )
            .into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "reboot command failed, need root?"})),
        )
            .into_response(),
    }
}

pub(crate) async fn shutdown_handler(headers: HeaderMap) -> impl IntoResponse {
    if let Err(code) = auth_required(headers).await {
        return (code, Json(json!({"error": "unauthorized"}))).into_response();
    }
    #[cfg(target_os = "windows")]
    let res = std::process::Command::new("shutdown")
        .args(["/s", "/t", "0"])
        .status();
    #[cfg(target_os = "linux")]
    let res = std::process::Command::new("systemctl")
        .arg("poweroff")
        .status();
    match res {
        Ok(s) if s.success() => (
            StatusCode::OK,
            Json(json!({"ok": true, "action": "shutdown"})),
        )
            .into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "shutdown command failed, need root?"})),
        )
            .into_response(),
    }
}

pub(crate) async fn health_handler() -> impl IntoResponse {
    match platform::get_cpu_temp() {
        Some(_) => (StatusCode::OK, Json(json!({"status": "ok"}))),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "error"})),
        ),
    }
}
