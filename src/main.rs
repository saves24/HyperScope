// hyper-node - system monitoring collector (Linux)
// Role: read system info + manage API key + provide authenticated API service
// Commands: help / key setup / key show / serve / log retention / log show

mod auth;
mod cli;
mod local_srv;
mod logging;
mod metrics;
mod platform;
mod tls;
mod util;

use auth::*;
use cli::*;
use logging::*;
use tls::*;
use util::*;

use hyper_panel_core::atomic_write;
use serde_json::{json, Value};
use std::time::Duration;

pub(crate) const VERSION: &str = "1.0.0";
pub(crate) const PROTOCOL_VERSION: &str = "1";
pub(crate) const CACHE_TTL: Duration = Duration::from_secs(1);
#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub(crate) const NET_SAMPLE_MS: u64 = 500;
pub(crate) const DEFAULT_RETENTION_DAYS: u64 = 7;

// System paths (binary runs directly on the host)
#[cfg(target_os = "linux")]
pub(crate) const KEY_DIR: &str = "/etc/hyper-node";
#[cfg(target_os = "windows")]
pub(crate) const KEY_DIR: &str = "C:\\ProgramData\\hyper-node";
#[cfg(target_os = "linux")]
pub(crate) const KEY_FILE: &str = "/etc/hyper-node/key";
#[cfg(target_os = "windows")]
pub(crate) const KEY_FILE: &str = "C:\\ProgramData\\hyper-node\\key";
#[cfg(target_os = "linux")]
pub(crate) const MODE_FILE: &str = "/etc/hyper-node/mode";
#[cfg(target_os = "windows")]
pub(crate) const MODE_FILE: &str = "C:\\ProgramData\\hyper-node\\mode";
#[cfg(target_os = "linux")]
pub(crate) const LOG_DIR: &str = "/var/log/hyper-node";
#[cfg(target_os = "windows")]
pub(crate) const LOG_DIR: &str = "C:\\ProgramData\\hyper-node\\log";
#[cfg(target_os = "linux")]
pub(crate) const CERT_FILE: &str = "/etc/hyper-node/cert.pem";
#[cfg(target_os = "windows")]
pub(crate) const CERT_FILE: &str = "C:\\ProgramData\\hyper-node\\cert.pem";
#[cfg(target_os = "linux")]
pub(crate) const KEY_PRIV_FILE: &str = "/etc/hyper-node/key.pem";
#[cfg(target_os = "windows")]
pub(crate) const KEY_PRIV_FILE: &str = "C:\\ProgramData\\hyper-node\\key.pem";

// ---------- Client certificate trust list (server-side verification) ----------

#[cfg(target_os = "linux")]
pub(crate) const TRUST_FILE: &str = "/etc/hyper-node/trust.json";
#[cfg(target_os = "windows")]
pub(crate) const TRUST_FILE: &str = "C:\\ProgramData\\hyper-node\\trust.json";

#[cfg(target_os = "linux")]
pub(crate) const CONFIG_FILE: &str = "/etc/hyper-node/config";
#[cfg(target_os = "windows")]
pub(crate) const CONFIG_FILE: &str = "C:\\ProgramData\\hyper-node\\config";
#[cfg(target_os = "linux")]
pub(crate) const HOSTNAME_FILE: &str = "/etc/hostname";

// ---------- Auth middleware ----------

// ---------- CORS (allow cross-origin only when origin is explicitly configured) ----------

/// Relay mode: the collector is not resident and opens no port. Metrics and
/// commands are served by one-shot processes (`hyper-node collect` /
/// `hyper-node control`) spawned by hyper-relay on the same machine. Running
/// this command manually keeps a minimal idle loop for foreground testing.
async fn cmd_relay() -> i32 {
    // Check whether key is set
    if load_key().map(|k| k.is_empty()).unwrap_or(true) {
        eprintln!("error: API key not set, run 'hyper-node key setup' first");
        return 1;
    }
    println!("hyper-node relay mode (no listening port; woken by hyper-relay on demand)");
    log_write("INFO", "relay mode started (no listening port)");

    // Keep the process alive forever for manual foreground runs; in normal
    // operation hyper-relay spawns one-shot collect/control processes instead.
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime");
    let code = rt.block_on(async_main());
    std::process::exit(code);
}

async fn async_main() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(|s| s.as_str()) {
        None | Some("help") | Some("--help") | Some("-h") => {
            print_help();
            0
        }
        Some("--version") | Some("-v") | Some("version") => {
            println!("hyper-node {}", VERSION);
            0
        }
        Some("key") => match args.get(1).map(|s| s.as_str()) {
            Some("setup") => {
                // Support: key setup [KEY] | key setup --plain [KEY]
                let plain = args.iter().any(|a| a == "--plain");
                let key_arg = args
                    .iter()
                    .skip(2)
                    .find(|a| !a.starts_with("--"))
                    .map(|s| s.as_str());
                cmd_key_setup(key_arg, plain)
            }
            Some("show") => cmd_key_show(),
            _ => {
                eprintln!("usage: hyper-node key setup [KEY] | key show");
                1
            }
        },
        Some("cert") => match args.get(1).map(|s| s.as_str()) {
            Some("gen") => {
                let _ = ensure_cert();
                0
            }
            Some("import") => {
                // Import a shared certificate (copy from another machine).
                // Certificates are machine-agnostic, so one generated cert can
                // be reused across every node in a lab for easier management.
                let cert_src = args.get(2).map(|s| s.as_str()).unwrap_or("");
                let key_src = args.get(3).map(|s| s.as_str()).unwrap_or("");
                if cert_src.is_empty() || key_src.is_empty() {
                    eprintln!("usage: hyper-node cert import <cert.pem> <key.pem>");
                    1
                } else {
                    match (
                        std::fs::copy(cert_src, CERT_FILE),
                        std::fs::copy(key_src, KEY_FILE),
                    ) {
                        (Ok(_), Ok(_)) => {
                            println!("certificate imported: {CERT_FILE}");
                            println!("fingerprint: {}", cert_fingerprint().unwrap_or_default());
                            0
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            eprintln!("error importing certificate: {e}");
                            1
                        }
                    }
                }
            }
            Some("show") => match cert_fingerprint() {
                Ok(fp) => {
                    println!("{fp}");
                    0
                }
                Err(e) => {
                    eprintln!("error: {e} (run hyper-node cert gen first)");
                    1
                }
            },
            _ => {
                eprintln!(
                    "usage: hyper-node cert gen | cert import <cert.pem> <key.pem> | cert show"
                );
                std::process::exit(1);
            }
        },
        Some("identity") => match args.get(1).map(|s| s.as_str()) {
            // identity init | identity show | identity sign <msg>
            Some("init") => match hyper_panel_core::identity::ensure_identity() {
                Ok(pubkey) => {
                    println!("identity ready");
                    println!("pubkey: {pubkey}");
                    0
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            },
            Some("show") => {
                let pubkey = hyper_panel_core::identity::ensure_identity();
                match pubkey {
                    Ok(pk) => {
                        println!("{pk}");
                        0
                    }
                    Err(e) => {
                        eprintln!("error: {e} (run hyper-node identity init first)");
                        1
                    }
                }
            }
            Some("sign") => {
                let msg = args.get(2).map(|s| s.as_str()).unwrap_or("");
                match hyper_panel_core::identity::sign_with_identity(msg) {
                    Ok(sig) => {
                        println!("{sig}");
                        0
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        1
                    }
                }
            }
            _ => {
                eprintln!("usage: hyper-node identity init | identity show | identity sign <msg>");
                1
            }
        },
        Some("device") => match args.get(1).map(|s| s.as_str()) {
            // device list | device add <id> <pubkey> <role> | device remove <id>
            Some("list") => {
                let list = hyper_panel_core::identity::load_trusted();
                if list.devices.is_empty() {
                    println!("no trusted devices");
                }
                for d in &list.devices {
                    println!(
                        "{} role={:?} added_by={} at={}",
                        d.id, d.role, d.added_by, d.added_at
                    );
                }
                0
            }
            Some("add") => {
                let id = args.get(2).map(|s| s.as_str()).unwrap_or("");
                let pubkey = args.get(3).map(|s| s.as_str()).unwrap_or("");
                let role_str = args.get(4).map(|s| s.as_str()).unwrap_or("viewer");
                let role = match role_str {
                    "owner" => hyper_panel_core::identity::Role::Owner,
                    "admin" => hyper_panel_core::identity::Role::Admin,
                    "viewer" => hyper_panel_core::identity::Role::Viewer,
                    _ => {
                        eprintln!("role must be owner|admin|viewer");
                        return 1;
                    }
                };
                // CLI add is trusted by definition (local admin).
                let mut list = hyper_panel_core::identity::load_trusted();
                match hyper_panel_core::identity::authorize_device(
                    &mut list,
                    "local-cli",
                    hyper_panel_core::identity::Role::Owner,
                    id,
                    pubkey,
                    role,
                    hyper_panel_core::identity::now_unix(),
                ) {
                    Ok(()) => match hyper_panel_core::identity::save_trusted(&list) {
                        Ok(()) => {
                            println!("device {id} added (role={role_str})");
                            0
                        }
                        Err(e) => {
                            eprintln!("error saving: {e}");
                            1
                        }
                    },
                    Err(e) => {
                        eprintln!("error: {e}");
                        1
                    }
                }
            }
            Some("remove") => {
                let id = args.get(2).map(|s| s.as_str()).unwrap_or("");
                let mut list = hyper_panel_core::identity::load_trusted();
                match hyper_panel_core::identity::deauthorize_device(
                    &mut list,
                    "local-cli",
                    hyper_panel_core::identity::Role::Owner,
                    id,
                ) {
                    Ok(()) => match hyper_panel_core::identity::save_trusted(&list) {
                        Ok(()) => {
                            println!("device {id} removed");
                            0
                        }
                        Err(e) => {
                            eprintln!("error saving: {e}");
                            1
                        }
                    },
                    Err(e) => {
                        eprintln!("error: {e}");
                        1
                    }
                }
            }
            _ => {
                eprintln!(
                    "usage: hyper-node device list | device add <id> <pubkey> <role> | device remove <id>"
                );
                1
            }
        },
        Some("trust") => match args.get(1).map(|s| s.as_str()) {
            Some("add") => match args.get(2) {
                None => {
                    eprintln!("usage: hyper-node trust add <SHA256:fingerprint>");
                    1
                }
                Some(fp) => {
                    let mut list = match load_trust() {
                        Ok(list) => list,
                        Err(e) => {
                            eprintln!("error: {e}");
                            return 1;
                        }
                    };
                    if !list.contains(fp) {
                        list.push(fp.clone());
                    }
                    match save_trust(&list) {
                        Ok(()) => {
                            println!("client certificate trusted: {fp}");
                            println!("hint: restart serve to take effect (mutual TLS)");
                            0
                        }
                        Err(e) => {
                            eprintln!("error: {e}");
                            1
                        }
                    }
                }
            },
            Some("list") => {
                let list = match load_trust() {
                    Ok(list) => list,
                    Err(e) => {
                        eprintln!("error: {e}");
                        return 1;
                    }
                };
                if list.is_empty() {
                    println!("trust list is empty (client certificate verification disabled)");
                } else {
                    println!("trusted client certificates ({}):", list.len());
                    for fp in &list {
                        println!("  {fp}");
                    }
                }
                0
            }
            Some("clear") => match save_trust(&[]) {
                Ok(()) => {
                    println!("trust list cleared (client verification disabled)");
                    0
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            },
            _ => {
                eprintln!("usage: hyper-node trust add <fingerprint> | trust list | trust clear");
                1
            }
        },
        Some("mode") => {
            // The listening-server mode was removed (protocol v0.2: relay mode,
            // no listening port). The mode marker is legacy; report it only.
            let mode = std::fs::read_to_string(MODE_FILE).unwrap_or_else(|_| "relay".to_string());
            println!("mode: {}", mode.trim());
            println!("(relay mode — no listening port; the 'serve' listener was removed)");
            0
        }
        Some("collect") => {
            // One-shot metrics collection: print the full system snapshot as
            // JSON to stdout and exit. Used by hyper-relay to wake the
            // collector on demand (no resident process, no listening port).
            let v = metrics::get_system();
            println!("{}", serde_json::to_string(&v).unwrap_or_default());
            0
        }
        Some("control") => {
            // One-shot control command: executed by hyper-relay on demand.
            // The relay passes a device_id + signature when a remote client
            // sent a signed command; local/root callers omit them and are
            // trusted via the local spawn path. Device signatures are verified
            // inside local_srv::handle_control.
            let action = args.get(1).map(|s| s.as_str()).unwrap_or("");
            let device_id = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let signature = args.get(3).map(|s| s.as_str()).unwrap_or("");
            let req = json!({
                "type": "control",
                "action": action,
                "device_id": device_id,
                "signature": signature,
            });
            let resp = local_srv::handle_control_for_cli(&req);
            println!("{}", serde_json::to_string(&resp).unwrap_or_default());
            0
        }
        Some("serve") | Some("relay") => {
            // Traditional listening mode was removed (protocol v0.2: no
            // listening port). "serve" is kept as an alias for compatibility
            // but now runs the collector in relay mode (local socket only).
            cmd_relay().await
        }
        Some("connect") => {
            let panel_url = args.get(1).map(|s| s.as_str()).unwrap_or("");
            let name = args.get(2).map(|s| s.as_str()).unwrap_or("");
            let key = args.get(3).map(|s| s.as_str()).unwrap_or("");
            if panel_url.is_empty() || name.is_empty() || key.is_empty() {
                eprintln!("usage: hyper-node connect <panel-url> <node-name> <node-key>");
                1
            } else {
                crate::cli::cmd_connect(panel_url, name, key).await
            }
        }
        #[cfg(target_os = "windows")]
        Some("service") => {
            // Windows service mode (ServiceMain): runs cmd_serve under SCM
            match crate::platform::service::run_windows_service() {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("service error: {e}");
                    1
                }
            }
        }
        Some("log") => match args.get(1).map(|s| s.as_str()) {
            Some("retention") => cmd_log_retention(args.get(2).map(|s| s.as_str())),
            Some("show") => cmd_log_show(),
            _ => {
                eprintln!("usage: hyper-node log retention <days> | log show");
                1
            }
        },
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("run 'hyper-node help' for usage");
            1
        }
    };
    std::process::exit(code);
}
