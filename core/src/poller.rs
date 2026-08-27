use crate::{
    config_mtime, fetch_json, history, load_nodes, log_write, record_event, save_nodes,
    tls_connect, NodeConfig, NodeState, SharedState,
};
// Background polling of nodes
use serde_json::Value;
use std::time::Duration;

pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

// Network history sample: (node name, Option<(rx_mbs, tx_mbs, tcp_conns)>)
type NetSample = (String, Option<(f64, f64, f64)>);

pub async fn poll_node(app: &SharedState, state: &NodeConfig) -> (Option<Value>, String, String) {
    let url = format!("{}/system", state.base_url());
    // TLS + no recorded fingerprint: connect first to obtain the actual fingerprint
    if state.tls && state.cert_fp.is_empty() {
        match tls_connect(&state.addr, state.port, "", Duration::from_secs(5)).await {
            Ok((_s, fp)) if !fp.is_empty() => {
                return (None, "unknown".to_string(), fp);
            }
            _ => {}
        }
    }
    match fetch_json(
        app,
        &url,
        &state.key,
        Duration::from_secs(5),
        &state.cert_fp,
    )
    .await
    {
        Ok(v) => (Some(v), "online".to_string(), String::new()),
        Err(e) if e == "unauthorized" => {
            log_write("WARN", &format!("node {} auth failed", state.name));
            (None, "unauthorized".to_string(), String::new())
        }
        Err(e) => {
            log_write(
                "WARN",
                &format!("node {} ({url}) poll failed: {e}", state.name),
            );
            (None, "offline".to_string(), String::new())
        }
    }
}
pub async fn background_poller(app: SharedState) {
    let mut ticker = tokio::time::interval(POLL_INTERVAL);
    let mut last_history: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    loop {
        ticker.tick().await;
        reload_nodes_if_changed(&app).await;

        // Snapshot node configs (short lock), then poll concurrently to avoid blocking the API
        let snapshots: Vec<NodeConfig> = {
            let nodes = app.nodes.lock().await;
            // Push-mode nodes have no listening port: they are updated via /api/push or /ws,
            // never polled (avoids futile connect attempts and false offline events).
            nodes
                .iter()
                .map(|ns| ns.config.clone())
                .filter(|c| !c.push)
                .collect()
        };

        // Poll all nodes concurrently
        let results: Vec<(String, Option<Value>, String, String)> =
            futures::future::join_all(snapshots.iter().map(|c| {
                let id = c.id.clone();
                let app2 = app.clone();
                async move {
                    let (data, status, fp) = poll_node(&app2, c).await;
                    (id, data, status, fp)
                }
            }))
            .await;
        // Index results by node id (push-mode nodes are not polled; a positional zip
        // would misalign results for any node after a push node in the list)
        let by_id: std::collections::HashMap<String, (Option<Value>, String, String)> = results
            .into_iter()
            .map(|(id, d, s, fp)| (id, (d, s, fp)))
            .collect();

        // Network + TCP rates for history (only when a node is due for a snapshot; concurrent)
        let now_min = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            / 60;
        let need_net: Vec<NodeConfig> = snapshots
            .iter()
            .filter(|c| last_history.get(&c.id).copied().unwrap_or(0) != now_min)
            .cloned()
            .collect();
        let mut net_rates: std::collections::HashMap<String, (f64, f64, f64)> =
            std::collections::HashMap::new();
        let io_results: Vec<NetSample> = {
            futures::future::join_all(need_net.iter().map(|c| {
                let app = app.clone();
                let c = c.clone();
                async move {
                    let traffic = fetch_json(
                        &app,
                        &format!("{}/traffic", c.base_url()),
                        &c.key,
                        Duration::from_secs(5),
                        &c.cert_fp,
                    )
                    .await;
                    let io = fetch_json(
                        &app,
                        &format!("{}/io", c.base_url()),
                        &c.key,
                        Duration::from_secs(5),
                        &c.cert_fp,
                    )
                    .await;
                    let rx = traffic
                        .as_ref()
                        .ok()
                        .and_then(|v| v.get("speed_rx"))
                        .and_then(|x| x.as_f64())
                        .unwrap_or(f64::NAN)
                        / 1048576.0;
                    let tx = traffic
                        .as_ref()
                        .ok()
                        .and_then(|v| v.get("speed_tx"))
                        .and_then(|x| x.as_f64())
                        .unwrap_or(f64::NAN)
                        / 1048576.0;
                    // /io returns {tcp_conns, ...}
                    let tcp = io
                        .as_ref()
                        .ok()
                        .and_then(|v| v.get("tcp_conns"))
                        .and_then(|x| x.as_f64())
                        .unwrap_or(f64::NAN);
                    (c.id.clone(), Some((rx, tx, tcp)))
                }
            }))
            .await
        };
        for (name, opt) in io_results {
            if let Some(v) = opt {
                net_rates.insert(name, v);
            }
        }

        // Write back results (short lock)
        let mut nodes = app.nodes.lock().await;
        for ns in nodes.iter_mut() {
            let Some((data, status, fp)) = by_id.get(&ns.config.id) else {
                continue; // push-mode node: updated via /api/push or /ws, not polled
            };
            // TOFU: auto-record fingerprint after first TLS connection
            if !fp.is_empty() && ns.config.cert_fp.is_empty() {
                ns.config.cert_fp = fp.clone();
                log_write(
                    "INFO",
                    &format!(
                        "node {} TLS certificate fingerprint recorded: {fp}",
                        ns.config.name
                    ),
                );
            }
            if *status != ns.status && ns.status != "unknown" {
                if status == "online" {
                    record_event(&app, &ns.config.name, "online", "node online".to_string()).await;
                } else if status == "offline" {
                    record_event(&app, &ns.config.name, "offline", "node offline".to_string())
                        .await;
                } else if status == "unauthorized" {
                    record_event(
                        &app,
                        &ns.config.name,
                        "unauthorized",
                        "auth failed".to_string(),
                    )
                    .await;
                }
            }
            // Keep existing data on poll failure (avoid empty frontend cards); update status only
            if data.is_some() {
                ns.data = data.clone();
            }
            ns.status = status.clone();
            // Persist history every ~60s per node (dedupe by node name + minute bucket)
            if let Some(d) = ns.data.as_ref() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let bucket_min = now / 60;
                if last_history.get(&ns.config.id).copied().unwrap_or(0) != bucket_min {
                    last_history.insert(ns.config.id.clone(), bucket_min);
                    let cpu = d.get("cpu").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
                    let mem = d
                        .get("mem_percent")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(f64::NAN);
                    let disk = d
                        .get("disk_percent")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(f64::NAN);
                    let temp = d
                        .get("cpu_temp_raw")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(f64::NAN);
                    // net/tcp come from /io endpoint (optional extra call below)
                    if let Err(error) = history::history_record(history::NodeSnapshot {
                        node: &ns.config.id, // stable id (survives rename)
                        cpu,
                        mem,
                        disk,
                        rx_mbs: net_rates
                            .get(&ns.config.id)
                            .map(|r| r.0)
                            .unwrap_or(f64::NAN),
                        tx_mbs: net_rates
                            .get(&ns.config.id)
                            .map(|r| r.1)
                            .unwrap_or(f64::NAN),
                        tcp: net_rates
                            .get(&ns.config.id)
                            .map(|r| r.2)
                            .unwrap_or(f64::NAN),
                        temp,
                    }) {
                        log_write(
                            "ERROR",
                            &format!("node {} history write failed: {error}", ns.config.id),
                        );
                    }
                }
            }
        }
        drop(nodes);
        // Persist newly recorded fingerprints to disk (read config once)
        let cfg_disk = load_nodes();
        let need_save = {
            let nodes = app.nodes.lock().await;
            nodes.iter().any(|ns| {
                ns.config.tls
                    && !ns.config.cert_fp.is_empty()
                    && cfg_disk
                        .iter()
                        .any(|c| c.name == ns.config.name && c.cert_fp.is_empty())
            })
        };
        if need_save {
            let mut cfg = cfg_disk;
            let nodes = app.nodes.lock().await;
            for c in cfg.iter_mut() {
                if let Some(ns) = nodes.iter().find(|ns| ns.config.name == c.name) {
                    if c.tls && c.cert_fp.is_empty() && !ns.config.cert_fp.is_empty() {
                        c.cert_fp = ns.config.cert_fp.clone();
                    }
                }
            }
            drop(nodes);
            let _ = save_nodes(&cfg);
        }
    }
}

// Reload node configs when nodes.json changes (after CLI/external edits)
async fn reload_nodes_if_changed(app: &SharedState) {
    let mtime = config_mtime();
    let mut last = app.config_mtime.lock().await;
    if *last != mtime {
        *last = mtime;
        let cfg = load_nodes();
        let mut nodes = app.nodes.lock().await;
        nodes.clear();
        nodes.extend(cfg.into_iter().map(|c| NodeState {
            config: c,
            data: None,
            data_ts: 0,
            traffic_cache: None,
            io_cache: None,
            status: "unknown".to_string(),
        }));
        log_write(
            "INFO",
            &format!("nodes.json changed, reloaded {} nodes", nodes.len()),
        );
    }
}
