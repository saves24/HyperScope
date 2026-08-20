// Docker container monitoring and control
use serde_json::{json, Value};
use std::process::Command;

// List all containers (running + stopped) with status
pub(crate) fn get_docker_containers() -> Vec<Value> {
    let out = match Command::new("docker")
        .args(["ps", "-a", "--no-trunc", "--format", "{{json .}}"])
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };
    let mut result = Vec::new();
    for line in out.lines() {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            let name = v
                .get("Names")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let id = v
                .get("ID")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let image = v
                .get("Image")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let state = v
                .get("State")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let status = v
                .get("Status")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let ports = v
                .get("Ports")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            // Short ID (first 12 chars)
            let short_id: String = id.chars().take(12).collect();
            result.push(json!({
                "name": name,
                "id": id,
                "short_id": short_id,
                "image": image,
                "state": state,
                "status": status,
                "ports": ports,
                "running": state == "running",
            }));
        }
    }
    result
}

// Control a container: action = start | stop | restart
pub(crate) fn docker_action(container: &str, action: &str) -> Result<Value, String> {
    let valid = matches!(action, "start" | "stop" | "restart");
    if !valid {
        return Err(format!("invalid docker action: {action}"));
    }
    if container.is_empty() || container.contains(' ') {
        return Err("invalid container name".to_string());
    }
    let out = Command::new("docker")
        .args([action, container])
        .output()
        .map_err(|e| format!("docker command failed: {e}"))?;
    if out.status.success() {
        let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Ok(json!({ "ok": true, "action": action, "container": container, "message": msg }))
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(format!("docker {action} {container} failed: {err}"))
    }
}
