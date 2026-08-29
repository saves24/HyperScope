// Resource + container alert detection.
//
// Runs inside the poller: inspects the latest node snapshot and, when a metric
// crosses a threshold, records an "alert" event (once per transition, tracked
// in active_alerts so it does not re-fire every poll cycle). When the metric
// returns below the recovery threshold the alert key is cleared, so the next
// abnormal reading re-triggers.
use crate::record_notification;
use crate::webhook::{alert_payload, spawn_webhook, webhook_url_for};
use crate::{NodeConfig, SharedState};
use serde_json::Value;

// Alert thresholds (trigger) — global defaults; a node can override each via
// NodeConfig.alert_* so a machine that normally runs hot can raise its own bar.
const CPU_HIGH: f64 = 90.0;
const MEM_HIGH: f64 = 90.0;
const DISK_HIGH: f64 = 90.0;
const TEMP_HIGH: f64 = 85.0;

/// Returns the alert keys currently raised for this node snapshot.
/// A key encodes the metric + rounded value, e.g. "cpu:93" / "docker:redis".
fn detect_alert_keys(data: &Value, docker: Option<&[Value]>, cfg: &NodeConfig) -> Vec<String> {
    let cpu_th = cfg.alert_cpu.unwrap_or(CPU_HIGH);
    let mem_th = cfg.alert_mem.unwrap_or(MEM_HIGH);
    let disk_th = cfg.alert_disk.unwrap_or(DISK_HIGH);
    let temp_th = cfg.alert_temp.unwrap_or(TEMP_HIGH);
    let mut keys: Vec<String> = Vec::new();
    if let Some(cpu) = data.get("cpu").and_then(|v| v.as_f64()) {
        if cpu >= cpu_th {
            keys.push(format!("cpu:{}", cpu.round()));
        }
    }
    if let Some(mem) = data.get("mem_percent").and_then(|v| v.as_f64()) {
        if mem >= mem_th {
            keys.push(format!("mem:{}", mem.round()));
        }
    }
    if let Some(disk) = data.get("disk_percent").and_then(|v| v.as_f64()) {
        if disk >= disk_th {
            keys.push(format!("disk:{}", disk.round()));
        }
    }
    if let Some(temp) = data.get("cpu_temp_raw").and_then(|v| v.as_f64()) {
        if temp >= temp_th {
            keys.push(format!("temp:{}", temp.round()));
        }
    }
    if let Some(containers) = docker {
        for c in containers {
            let running = c.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
            if !running {
                let name = c
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("container");
                keys.push(format!("docker:{}", name));
            }
        }
    }
    keys
}

/// Turns a raised alert key into a human message.
fn alert_message(key: &str) -> String {
    if let Some(v) = key.strip_prefix("cpu:") {
        format!("CPU high: {}%", v)
    } else if let Some(v) = key.strip_prefix("mem:") {
        format!("Memory high: {}%", v)
    } else if let Some(v) = key.strip_prefix("disk:") {
        format!("Disk high: {}%", v)
    } else if let Some(v) = key.strip_prefix("temp:") {
        format!("Temperature high: {}°C", v)
    } else if let Some(v) = key.strip_prefix("docker:") {
        format!("Container not running: {}", v)
    } else {
        key.to_string()
    }
}

/// Checks the node snapshot for anomalies and records "alert" events on
/// transitions (newly raised keys only).
pub async fn check_alerts(
    app: &SharedState,
    node_id: &str,
    node_name: &str,
    data: &Value,
    docker: Option<&[Value]>,
    cfg: &NodeConfig,
) {
    let raised = detect_alert_keys(data, docker, cfg);

    // Compute which keys are new (not yet active) while holding the lock,
    // then release before firing events (record_event takes its own locks).
    let mut to_fire: Vec<String> = Vec::new();
    {
        let mut active = app.active_alerts.lock().await;
        let prev = active.get(node_id).cloned().unwrap_or_default();
        for key in &raised {
            if !prev.contains(key) {
                to_fire.push(key.clone());
            }
        }
        active.insert(node_id.to_string(), raised);
    }

    if to_fire.is_empty() {
        return;
    }

    for key in &to_fire {
        record_notification(app, node_name, alert_message(key)).await;
    }

    // Best-effort external delivery: if the node (or the global setting) has a
    // webhook URL, POST each new alert there. Never blocks the poller.
    let url = webhook_url_for(app, cfg);
    if !url.is_empty() {
        for key in &to_fire {
            spawn_webhook(&url, alert_payload(node_name, key, &alert_message(key)));
        }
    }
}
