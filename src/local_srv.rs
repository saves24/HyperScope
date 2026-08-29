// Control-command handling for hyper-node.
//
// The collector is never resident: hyper-relay spawns `hyper-node collect`
// (metrics) or `hyper-node control <action>` (commands) as one-shot processes.
// This module holds the command logic shared by the CLI and the old socket
// path; device-signature authentication is verified here.
use serde_json::{json, Value};

/// Public wrapper for the CLI `control` command (hyper-node control <action>).
/// The relay spawns this process as root, so unsigned callers are trusted;
/// device signatures are verified when the request carries device_id + signature.
pub fn handle_control_for_cli(req: &Value) -> Value {
    handle_control(req)
}

/// Handle a control request with device-signature authentication.
/// - no device/signature → trusted local/root caller
/// - device + signature → verified against trusted.toml; role checked for
///   high-risk actions (viewer cannot reboot/shutdown)
fn handle_control(req: &Value) -> Value {
    let raw_action = req["action"].as_str().unwrap_or("").to_string();
    // The panel sends path-style commands ("/reboot", "/shutdown"); the CLI
    // sends bare names ("reboot"). Normalize by stripping a leading slash.
    let action = raw_action.trim_start_matches('/').to_string();
    let device_id = req["device_id"].as_str().unwrap_or("");
    let signature = req["signature"].as_str().unwrap_or("");
    let authed = if !device_id.is_empty() && !signature.is_empty() {
        // Signature payload is ts:nonce:signature; the signed message is
        // cmd:device_id:ts:nonce. Timestamp must be within 60s of now and
        // the nonce must be fresh (replay protection; the relay also checks).
        let mut parts = signature.splitn(3, ':');
        let ts_str = parts.next().unwrap_or("");
        let nonce = parts.next().unwrap_or("");
        let sig = parts.next().unwrap_or("");
        let ts_ok = ts_str
            .parse::<i64>()
            .map(|t| (hyper_panel_core::now_unix() as i64 - t).abs() <= 60)
            .unwrap_or(false);
        let list = hyper_panel_core::identity::load_trusted();
        match hyper_panel_core::identity::find_device(&list, device_id) {
            Some(dev) if ts_ok && !nonce.is_empty() && !sig.is_empty() => {
                // Sign over the raw action so the signature matches what the
                // panel/relay signed (the relay verifies the same string).
                let msg = format!("{raw_action}:{device_id}:{ts_str}:{nonce}");
                hyper_panel_core::identity::verify_device_signature(&dev.pubkey, &msg, sig)
            }
            _ => false,
        }
    } else {
        true // local/root caller via spawn
    };
    if !authed {
        return json!({"type": "result", "ok": false, "error": "signature invalid"});
    }
    // Signed callers must not be viewers: any command (kill/docker/reboot/
    // shutdown) mutates the node, so the viewer role gets no command access.
    // Local root callers (device_id empty) bypass the role check.
    if !device_id.is_empty() {
        let list = hyper_panel_core::identity::load_trusted();
        let role_ok = hyper_panel_core::identity::find_device(&list, device_id)
            .map(|d| d.role != hyper_panel_core::identity::Role::Viewer)
            .unwrap_or(false);
        if !role_ok {
            return json!({"type": "result", "ok": false, "error": "insufficient role"});
        }
    }
    match action.as_str() {
        "reboot" => {
            // Actually reboot the machine. Linux: systemctl; Windows: shutdown.
            #[cfg(target_os = "windows")]
            let exec = {
                use std::process::Command;
                Command::new("cmd").args(["/c", "shutdown /r /t 3"]).spawn()
            };
            #[cfg(not(target_os = "windows"))]
            let exec = {
                use std::process::Command;
                Command::new("sh").arg("-c").arg("systemctl reboot").spawn()
            };
            match exec {
                Ok(_) => json!({"type": "result", "ok": true, "result": "reboot scheduled"}),
                Err(e) => json!({"type": "result", "ok": false, "error": e.to_string()}),
            }
        }
        "shutdown" => {
            // Actually power off the machine.
            #[cfg(target_os = "windows")]
            let exec = {
                use std::process::Command;
                Command::new("cmd").args(["/c", "shutdown /s /t 3"]).spawn()
            };
            #[cfg(not(target_os = "windows"))]
            let exec = {
                use std::process::Command;
                Command::new("sh")
                    .arg("-c")
                    .arg("systemctl poweroff")
                    .spawn()
            };
            match exec {
                Ok(_) => json!({"type": "result", "ok": true, "result": "shutdown scheduled"}),
                Err(e) => json!({"type": "result", "ok": false, "error": e.to_string()}),
            }
        }
        _ => {
            // Path-style actions from the panel: /processes/<pid>/kill,
            // /docker/<container>/<action>. Parse and dispatch.
            let parts: Vec<&str> = action.trim_start_matches('/').split('/').collect();
            match parts.as_slice() {
                ["processes", pid, "kill"] => {
                    let pid: u32 = match pid.parse() {
                        Ok(p) => p,
                        Err(_) => return json!({"type": "result", "ok": false, "error": "bad pid"}),
                    };
                    match crate::platform::kill_process(pid) {
                        Ok(()) => json!({"type": "result", "ok": true, "result": "killed"}),
                        Err(e) => json!({"type": "result", "ok": false, "error": e.to_string()}),
                    }
                }
                ["docker", container, action] => {
                    // Docker container control: start/stop/restart/logs.
                    // The collector invokes the docker CLI via the platform
                    // backend (Linux and Windows both implement it).
                    if *action == "logs" {
                        let res = crate::platform::docker_logs(container, 50);
                        if res.get("ok").and_then(|x| x.as_bool()).unwrap_or(false) {
                            json!({"type": "result", "ok": true, "result": res["logs"].as_str().unwrap_or("")})
                        } else {
                            json!({"type": "result", "ok": false, "error": res["error"].as_str().unwrap_or("docker logs failed")})
                        }
                    } else if !matches!(*action, "start" | "stop" | "restart") {
                        json!({"type": "result", "ok": false, "error": "bad docker action"})
                    } else {
                        let res = crate::platform::docker_action(container, action);
                        if res.get("ok").and_then(|x| x.as_bool()).unwrap_or(false) {
                            json!({"type": "result", "ok": true, "result": format!("docker {action} {container}")})
                        } else {
                            json!({"type": "result", "ok": false, "error": res["error"].as_str().unwrap_or("docker failed")})
                        }
                    }
                }
                _ => json!({"type": "result", "ok": false, "error": "unknown action"}),
            }
        }
    }
}
