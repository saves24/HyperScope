// WebSocket server + local Unix socket bridge.
//
// External peers (nodes / Android / web panel) connect over WebSocket.
// The relay routes messages by node name and wakes the local hyper-node
// process through a Unix socket when a command or collect request arrives.
use crate::protocol::Registry;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        ConnectInfo, State,
    },
    routing::get,
    Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Confirmation requests expire after this many seconds (protocol §确认超时).
/// Enforced by the collector; kept here as the shared protocol constant.
#[allow(dead_code)]
pub const CONFIRM_TIMEOUT_SECS: u64 = 60;
/// Peer is considered dead after this many seconds without any message.
pub const PEER_IDLE_TIMEOUT_SECS: u64 = 45;
/// Send a keepalive ping after this much idle time.
pub const KEEPALIVE_INTERVAL_SECS: u64 = 15;
/// Max collector-waking queries per second (spawning a process per query).
pub const QUERY_MAX_PER_SEC: u32 = 10;

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<Mutex<Registry>>,
    pub peers: Arc<Mutex<HashMap<u64, PeerHandle>>>,
    pub next_peer: Arc<std::sync::atomic::AtomicU64>,
    /// Sliding-window query budget: (window_start_unix, count). Spawning a
    /// collector per query is expensive, so queries are rate-limited to
    /// QUERY_MAX_PER_SEC per window.
    pub query_budget: Arc<std::sync::Mutex<(u64, u32)>>,
    /// Recently seen command nonces (replay protection); bounded size.
    pub cmd_nonces: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

#[derive(Clone)]
pub struct PeerHandle {
    #[allow(dead_code)] // node name kept for logging/debug; routing is by id
    pub node: String,
    #[allow(dead_code)] // peer send channel; routing is by id
    pub tx: tokio::sync::mpsc::UnboundedSender<String>,
}

pub async fn run_with_tls(
    addr: SocketAddr,
    cert_path: Option<&str>,
    key_path: Option<&str>,
) -> Result<(), String> {
    let state = AppState {
        registry: Arc::new(Mutex::new(Registry::new())),
        peers: Arc::new(Mutex::new(HashMap::new())),
        next_peer: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        query_budget: Arc::new(std::sync::Mutex::new((0, 0))),
        cmd_nonces: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
    };

    let app = Router::new()
        .route("/ws", get(ws_upgrade))
        .route("/api/nodes", get(list_nodes))
        .route("/health", get(health))
        .with_state(state);

    match (cert_path, key_path) {
        (Some(cert), Some(key)) => {
            // TLS (WSS): axum-server binds the listener itself (it needs the
            // TLS acceptor before accept). Do NOT pre-bind the port here —
            // that would collide with axum-server's own bind.
            use std::io::BufReader;
            let cert_file =
                std::fs::File::open(cert).map_err(|e| format!("open cert {cert}: {e}"))?;
            let key_file = std::fs::File::open(key).map_err(|e| format!("open key {key}: {e}"))?;
            let certs: Vec<rustls::pki_types::CertificateDer<'static>> =
                rustls_pemfile::certs(&mut BufReader::new(cert_file))
                    .collect::<Result<_, _>>()
                    .map_err(|e| format!("parse cert: {e}"))?;
            let key: rustls::pki_types::PrivateKeyDer<'static> =
                rustls_pemfile::private_key(&mut BufReader::new(key_file))
                    .map_err(|e| format!("parse key: {e}"))?
                    .ok_or_else(|| "no private key found".to_string())?;
            let config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .map_err(|e| format!("tls config: {e}"))?;
            println!("hyper-relay listening on {addr} (wss/TLS)");
            axum_server::bind_rustls(
                addr,
                axum_server::tls_rustls::RustlsConfig::from_config(std::sync::Arc::new(config)),
            )
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await
            .map_err(|e| e.to_string())?;
            Ok(())
        }
        _ => {
            // TLS is mandatory: refusing to serve plaintext avoids accidental
            // unencrypted relay traffic on the public network path.
            Err("relay requires --tls-cert and --tls-key (plaintext WS is not supported)".into())
        }
    }
}

async fn health() -> &'static str {
    "{\"status\":\"ok\"}"
}

async fn list_nodes(State(state): State<AppState>) -> String {
    let reg = state.registry.lock().await;
    let list = reg.list();
    serde_json::json!({ "nodes": list }).to_string()
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    use axum::extract::ws::Message as WsMsg;

    // First message must be REGISTER or HEARTBEAT (auth / identity).
    let mut peer_id = 0u64;
    let mut node_name = String::new();
    let mut last_activity = tokio::time::Instant::now();

    loop {
        // Keepalive: send a ping after KEEPALIVE_INTERVAL_SECS of idle, and
        // treat the peer as dead after PEER_IDLE_TIMEOUT_SECS without messages.
        let idle = last_activity.elapsed();
        if idle.as_secs() >= PEER_IDLE_TIMEOUT_SECS {
            break; // no messages for too long → dead
        }
        let recv_timeout = if idle.as_secs() >= KEEPALIVE_INTERVAL_SECS {
            let _ = socket.send(WsMsg::Ping(vec![])).await;
            last_activity = tokio::time::Instant::now();
            tokio::time::Duration::from_secs(1)
        } else {
            tokio::time::Duration::from_secs(KEEPALIVE_INTERVAL_SECS - idle.as_secs())
        };
        let msg = match tokio::time::timeout(recv_timeout, socket.recv()).await {
            Ok(Some(Ok(msg))) => msg,
            Ok(Some(Err(_))) | Ok(None) => break, // transport error or closed
            Err(_) => continue,                   // idle timeout; loop re-checks
        };
        last_activity = tokio::time::Instant::now();
        // Answer pings so the peer's keepalive never times us out; ignore
        // binary/pong frames.
        if let WsMsg::Ping(payload) = &msg {
            let _ = socket.send(WsMsg::Pong(payload.clone())).await;
            continue;
        }
        let WsMsg::Text(text) = msg else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            let _ = socket
                .send(WsMsg::Text(
                    "{\"type\":\"error\",\"code\":1,\"msg\":\"bad json\"}".into(),
                ))
                .await;
            continue;
        };

        let t = v["type"].as_str().unwrap_or("");
        match t {
            "register" => {
                let name = v["node"].as_str().unwrap_or("").to_string();
                let pubkey = v["node_pubkey"].as_str().unwrap_or("").as_bytes().to_vec();
                let mode = v["mode"].as_str().unwrap_or("relay").to_string();
                if name.is_empty() {
                    let _ = socket
                        .send(WsMsg::Text(
                            "{\"type\":\"error\",\"code\":1,\"msg\":\"name required\"}".into(),
                        ))
                        .await;
                    continue;
                }
                node_name = name.clone();
                peer_id = state
                    .next_peer
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                {
                    let mut reg = state.registry.lock().await;
                    reg.register(&name, &pubkey, &mode, peer_id);
                }
                let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
                state.peers.lock().await.insert(
                    peer_id,
                    PeerHandle {
                        node: name.clone(),
                        tx,
                    },
                );
                let _ = socket
                    .send(WsMsg::Text(format!(
                        "{{\"type\":\"ok\",\"node\":\"{name}\"}}"
                    )))
                    .await;
                println!("node registered: {name} (peer {peer_id})");
            }
            "heartbeat" => {
                let name = v["node"].as_str().unwrap_or("").to_string();
                if peer_id == 0 {
                    // implicit register on first heartbeat
                    node_name = name.clone();
                    peer_id = state
                        .next_peer
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    state.peers.lock().await.insert(
                        peer_id,
                        PeerHandle {
                            node: name.clone(),
                            tx: tokio::sync::mpsc::unbounded_channel().0,
                        },
                    );
                }
                let mut reg = state.registry.lock().await;
                reg.heartbeat(&name, peer_id);
            }
            "offer" => {
                let name = v["node"].as_str().unwrap_or("");
                let addr = v["addr"].as_str().unwrap_or("");
                let proto = v["proto"].as_str().unwrap_or("tcp");
                let ttl = v["expires"].as_u64().unwrap_or(30);
                let mut reg = state.registry.lock().await;
                reg.set_offer(name, addr, proto, ttl);
            }
            "query" => {
                // On-demand data fetch: spawn the local hyper-node as a
                // one-shot process (`collect`) and return its JSON snapshot
                // directly. The collector is never resident and opens no
                // listening port — this is the protocol's wake-on-demand path.
                // Rate-limit to avoid an attacker exhausting the relay by
                // repeatedly spawning collector processes.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let limited = {
                    let mut budget = state.query_budget.lock().unwrap();
                    if now != budget.0 {
                        *budget = (now, 0);
                    }
                    if budget.1 >= QUERY_MAX_PER_SEC {
                        true
                    } else {
                        budget.1 += 1;
                        false
                    }
                };
                if limited {
                    let _ = socket
                        .send(WsMsg::Text(
                            "{\"type\":\"error\",\"code\":2,\"msg\":\"query rate limited\"}".into(),
                        ))
                        .await;
                    continue;
                }
                let name = v["node"].as_str().unwrap_or("");
                match crate::local_wake::wake_collector() {
                    Some(data) => {
                        let resp = serde_json::json!({
                            "type": "data",
                            "node": name,
                            "data": data,
                        });
                        let _ = socket.send(WsMsg::Text(resp.to_string())).await;
                    }
                    None => {
                        let _ = socket
                            .send(WsMsg::Text(
                                "{\"type\":\"error\",\"code\":3,\"msg\":\"collector unavailable\"}"
                                    .into(),
                            ))
                            .await;
                    }
                }
            }
            "cmd" => {
                // Execute a signed command by spawning the local collector
                // process (same machine by design). The collector is not
                // resident; every command is a fresh short-lived process.
                //
                // SECURITY: the relay is a remote network entry (0.0.0.0),
                // so every command MUST carry a device signature verified
                // against the node's trusted-device list. Without this any
                // LAN/internet client could trigger root reboot/shutdown.
                let name = v["node"].as_str().unwrap_or("");
                let device_id = v["device_id"].as_str().unwrap_or("");
                let signature = v["signature"].as_str().unwrap_or("");
                let cmd = v["cmd"].as_str().unwrap_or("");
                let trusted = hyper_panel_core::identity::load_trusted();
                // Signature payload is ts:nonce:signature. Verify against
                // cmd:device_id:ts:nonce and reject replayed nonces.
                let mut parts = signature.splitn(3, ':');
                let ts_str = parts.next().unwrap_or("");
                let nonce = parts.next().unwrap_or("");
                let sig = parts.next().unwrap_or("");
                let ts_ok = ts_str
                    .parse::<i64>()
                    .ok()
                    .map(|t| (hyper_panel_core::now_unix() as i64 - t).abs() <= 60);
                let authed = match hyper_panel_core::identity::find_device(&trusted, device_id) {
                    Some(dev) if ts_ok == Some(true) && !nonce.is_empty() && !sig.is_empty() => {
                        // Same signing message format as the collector's own
                        // control path, so signatures are interchangeable.
                        let msg = format!("{cmd}:{device_id}:{ts_str}:{nonce}");
                        let verified = hyper_panel_core::identity::verify_device_signature(
                            &dev.pubkey,
                            &msg,
                            sig,
                        );
                        // Reject reused nonces (replay protection).
                        if verified {
                            let mut seen = state.cmd_nonces.lock().unwrap();
                            if seen.contains(nonce) {
                                false
                            } else {
                                seen.insert(nonce.to_string());
                                if seen.len() > 4096 {
                                    seen.clear();
                                }
                                true
                            }
                        } else {
                            false
                        }
                    }
                    _ => false,
                };
                if !authed {
                    let _ = socket
                        .send(WsMsg::Text(
                            "{\"type\":\"error\",\"code\":4,\"msg\":\"command not authorized\"}"
                                .into(),
                        ))
                        .await;
                    continue;
                }
                match crate::local_wake::run_command(cmd, device_id, signature) {
                    Some(result) => {
                        let resp = serde_json::json!({
                            "type": "cmd_result",
                            "node": name,
                            "result": result,
                        });
                        let _ = socket.send(WsMsg::Text(resp.to_string())).await;
                    }
                    None => {
                        let _ = socket
                            .send(WsMsg::Text(
                                "{\"type\":\"error\",\"code\":3,\"msg\":\"collector unavailable\"}"
                                    .into(),
                            ))
                            .await;
                    }
                }
            }
            _ => {
                let _ = socket
                    .send(WsMsg::Text(
                        "{\"type\":\"error\",\"code\":9,\"msg\":\"unsupported\"}".into(),
                    ))
                    .await;
            }
        }
    }

    // Cleanup on disconnect.
    if peer_id != 0 {
        state.registry.lock().await.disconnect(peer_id);
        state.peers.lock().await.remove(&peer_id);
        if !node_name.is_empty() {
            println!("node disconnected: {node_name}");
        }
    }
}
