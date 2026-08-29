// Server bootstrap: state construction, router wiring, and the HTTP listener.
// Split from main.rs so the entry file stays a thin dispatch layer.
use crate::{
    add_node_handler, auth_middleware, events_clear_handler, events_handler, health_handler,
    history_export_handler, index_handler, login_handler, logout_handler, me_handler,
    node_disks_handler, node_docker_control_handler, node_docker_handler, node_history_handler,
    node_io_handler, node_processes_handler, node_reboot_handler, node_shutdown_handler,
    node_system_handler, node_traffic_handler, nodes_export_handler, nodes_handler,
    notifications_clear_handler, notifications_handler, ping_node_handler, push_handler,
    remove_node_handler, rename_node_handler, settings_get_handler, settings_post_handler,
    static_api_js_handler, static_css_handler, static_dashboard_handler, static_forge_handler,
    static_history_handler, static_js_handler, static_logs_handler, static_nodes_handler,
    static_utils_handler, status_handler, update_node_alerts_handler, ws_handler,
};
use crate::{
    config_mtime, hash_password, load_events_from_file, load_nodes, load_notifications_from_file,
    load_users, log_write, save_users, validate_auth_file, AppState, NodeState, SharedState, User,
};
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

/// Creates the full router with all API/static routes and the auth middleware.
fn build_router(state: SharedState) -> Router {
    Router::new()
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
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
        .route("/api/nodes", get(nodes_handler).post(add_node_handler))
        .route("/api/nodes/export", get(nodes_export_handler))
        .route("/api/node/id/:node_id/name", put(rename_node_handler))
        .route(
            "/api/node/id/:node_id/alerts",
            put(update_node_alerts_handler),
        )
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
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

/// Runs the HTTP server until it stops; returns the process exit code.
pub(crate) async fn cmd_serve(port: u16) -> i32 {
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
    // Install the rustls CryptoProvider before any TLS (wss) relay connection.
    let _ = rustls::crypto::ring::default_provider().install_default();
    tokio::spawn(crate::background_poller(poller_state));

    let app = build_router(app_state);

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
