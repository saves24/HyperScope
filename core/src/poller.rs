use crate::alerts::check_alerts;
use crate::{
    config_mtime, history, load_nodes, log_write, record_event, save_nodes, NodeConfig, NodeState,
    SharedState,
};
// Background polling of nodes
use serde_json::Value;

pub const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);

// Network history sample: (node name, Option<(rx_mbs, tx_mbs, tcp_conns)>)

pub async fn poll_node(_app: &SharedState, state: &NodeConfig) -> (Option<Value>, String, String) {
    // All nodes are reached through their hyper-relay (installed on the same
    // machine as the collector by the installer). No direct HTTP or local
    // socket path — the relay wakes the collector on demand per poll.
    poll_via_relay(state).await
}
/// Fetch metrics for a relay-mode node (push=true, no listening port).
/// Asks the node's hyper-relay (same machine, port 8686) to wake the local
/// collector and return a fresh snapshot.
async fn poll_via_relay(state: &NodeConfig) -> (Option<Value>, String, String) {
    let relay_addr = format!("{}:8686", state.addr);
    match crate::relay_client::query_direct(
        &relay_addr,
        &state.name,
        state.tls,
        Some(&state.cert_fp),
    )
    .await
    {
        Some((data, fp)) => (Some(data), "online".to_string(), fp),
        None => {
            log_write(
                "WARN",
                &format!("node {} relay collect failed (relay offline?)", state.name),
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
            // All nodes are polled through their hyper-relay (the collector is
            // woken on demand per poll).
            nodes.iter().map(|ns| ns.config.clone()).collect()
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
        // Index results by node id
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
        // Traffic/io come from the poll snapshot itself (the relay collects a
        // full snapshot including traffic + io per poll). No direct HTTP.
        for c in &need_net {
            if let Some((Some(data), _, _)) = by_id.get(&c.id) {
                let rx = data
                    .get("traffic")
                    .and_then(|t| t.get("speed_rx"))
                    .and_then(|x| x.as_f64())
                    .unwrap_or(f64::NAN)
                    / 1048576.0;
                let tx = data
                    .get("traffic")
                    .and_then(|t| t.get("speed_tx"))
                    .and_then(|x| x.as_f64())
                    .unwrap_or(f64::NAN)
                    / 1048576.0;
                let tcp = data
                    .get("io")
                    .and_then(|i| i.get("tcp_conns"))
                    .and_then(|x| x.as_f64())
                    .unwrap_or(f64::NAN);
                net_rates.insert(c.id.clone(), (rx, tx, tcp));
            }
        }

        // Write back results (short lock)
        let mut nodes = app.nodes.lock().await;
        let mut alerts_todo: Vec<(NodeConfig, Value)> = Vec::new();
        for ns in nodes.iter_mut() {
            let Some((data, status, fp)) = by_id.get(&ns.config.id) else {
                continue;
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
                // collect for post-lock alert check (system snapshot)
                alerts_todo.push((ns.config.clone(), data.clone().unwrap()));
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
        // Alert check: docker containers come from the poll snapshot itself
        // (no direct HTTP); run anomaly detection for each online node.
        if !alerts_todo.is_empty() {
            let docker_by_id: std::collections::HashMap<String, Vec<Value>> = alerts_todo
                .iter()
                .map(|(c, data)| {
                    let containers = data
                        .get("docker")
                        .and_then(|d| d.get("containers"))
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default();
                    (c.id.clone(), containers)
                })
                .collect();
            for (c, data) in alerts_todo.iter() {
                let dock = docker_by_id.get(&c.id).map(|v| v.as_slice());
                check_alerts(&app, &c.id, &c.name, data, dock, c).await;
            }
        }
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
