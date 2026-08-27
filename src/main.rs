// hyper-node - system monitoring collector (Linux)
// Role: read system info + manage API key + provide authenticated API service
// Commands: help / key setup / key show / serve / log retention / log show

mod api;
mod auth;
mod cli;
#[cfg(target_os = "linux")]
mod docker;
mod io;
mod logging;
mod metrics;
mod platform;
mod tls;
mod util;

use api::*;
use auth::*;
use cli::*;
use logging::*;
use metrics::*;
use tls::*;
use util::*;

use axum::{
    http::{header, HeaderValue, StatusCode},
    middleware,
    routing::{get, post},
    Router,
};
use hyper_panel_core::atomic_write;
#[cfg(target_os = "linux")]
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tower::ServiceExt;

pub(crate) const VERSION: &str = "0.1.0";
pub(crate) const PROTOCOL_VERSION: &str = "1";
pub(crate) const DEFAULT_PORT: u16 = 5000;
pub(crate) const CACHE_TTL: Duration = Duration::from_secs(3);
#[cfg(target_os = "linux")]
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

async fn cmd_serve(port: u16, tls: bool) -> i32 {
    // Check whether key is set
    if load_key().map(|k| k.is_empty()).unwrap_or(true) {
        eprintln!("error: API key not set, run 'hyper-node key setup' first");
        return 1;
    }

    let app = Router::new()
        .route("/", get(status_handler))
        .route("/system", get(system_handler))
        .route("/traffic", get(traffic_handler))
        .route("/disks", get(disks_handler))
        .route("/processes", get(processes_handler))
        .route("/io", get(io_handler))
        .route("/ports", get(ports_handler))
        .route("/wifi", get(wifi_handler))
        .route("/logs", get(logs_handler))
        .route("/docker", get(docker_handler))
        .route("/docker/:container/:action", post(docker_control_handler))
        .route("/reboot", post(reboot_handler))
        .route("/shutdown", post(shutdown_handler))
        .route("/health", get(health_handler))
        .layer(middleware::from_fn(cors_middleware));

    let listener = match tokio::net::TcpListener::bind(("0.0.0.0", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind port {port}: {e}");
            return 1;
        }
    };

    if tls {
        // Ensure certificate exists (auto-generate if missing)
        if let Err(e) = ensure_cert() {
            eprintln!("TLS certificate setup failed: {e}");
            return 1;
        }
        let tls_cfg = match tls_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("TLS config failed: {e}");
                return 1;
            }
        };
        println!("hyper-node HTTPS service started on 0.0.0.0:{port} (TLS encrypted)");
        println!(
            "certificate fingerprint: {}",
            cert_fingerprint().unwrap_or_default()
        );
        log_write("INFO", &format!("HTTPS service started on 0.0.0.0:{port}"));
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(tls_cfg));
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => continue,
            };
            let acceptor = acceptor.clone();
            let app = app.clone();
            tokio::spawn(async move {
                let Ok(tls_stream) = acceptor.accept(stream).await else {
                    return;
                };
                let io = hyper_util::rt::TokioIo::new(tls_stream);
                let service = hyper::service::service_fn(move |req| {
                    let app = app.clone();
                    async move {
                        Ok::<_, std::convert::Infallible>(app.oneshot(req).await.unwrap_or_else(
                            |_| {
                                let mut response =
                                    axum::http::Response::new(axum::body::Body::empty());
                                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                                response
                            },
                        ))
                    }
                });
                let conn = hyper::server::conn::http1::Builder::new().serve_connection(io, service);
                if let Err(error) = conn.await {
                    eprintln!("TLS HTTP connection ended with error: {error}");
                }
            });
        }
    } else {
        println!("hyper-node service started on 0.0.0.0:{port} (plaintext)");
        log_write("INFO", &format!("service started on 0.0.0.0:{port}"));
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("HTTP server stopped: {e}");
            return 1;
        }
    }
    0
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
                eprintln!("usage: hyper-node cert gen | cert show");
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
            match args.get(1).map(|s| s.as_str()) {
                Some("tls") | Some("plain") => {
                    let mode = args[1].clone();
                    // Update mode marker file
                    if let Err(e) = crate::atomic_write(MODE_FILE, &mode, 0o600) {
                        eprintln!("error: cannot write mode file: {e}");
                        1
                    } else {
                        crate::chmod(MODE_FILE, 0o644);
                        #[cfg(target_os = "linux")]
                        {
                            // Try to update systemd service ExecStart
                            let service = "/etc/systemd/system/hyper-node.service";
                            if std::path::Path::new(service).exists() {
                                let content = std::fs::read_to_string(service).unwrap_or_default();
                                let new_exec = if mode == "tls" {
                                    "/usr/local/bin/hyper-node serve"
                                } else {
                                    "/usr/local/bin/hyper-node serve --no-tls"
                                };
                                let updated = if content.contains("ExecStart=") {
                                    let re = regex::Regex::new(r"(?m)^ExecStart=.*$").unwrap();
                                    re.replace(&content, format!("ExecStart={new_exec}"))
                                        .to_string()
                                } else {
                                    content
                                };
                                if let Err(e) = crate::atomic_write(service, &updated, 0o644) {
                                    eprintln!("warning: cannot update systemd service: {e}");
                                } else {
                                    println!("systemd service updated (restart to apply): systemctl restart hyper-node");
                                }
                            }
                        }
                        println!("mode set to {mode} (takes effect after restart)");
                        0
                    }
                }
                _ => {
                    // Show current mode
                    let mode =
                        std::fs::read_to_string(MODE_FILE).unwrap_or_else(|_| "tls".to_string());
                    println!("current mode: {}", mode.trim());
                    println!("usage: hyper-node mode tls | mode plain");
                    0
                }
            }
        }
        Some("serve") => {
            let mut port = DEFAULT_PORT;
            // Default reads mode marker (tls/plain); HTTPS if unset
            let mut tls = std::fs::read_to_string(MODE_FILE)
                .map(|m| m.trim() != "plain")
                .unwrap_or(true);
            let mut i = 1;
            while i < args.len() {
                if args[i] == "--port" {
                    if let Some(p) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                        port = p;
                    }
                    i += 2;
                } else if args[i] == "--no-tls" {
                    tls = false;
                    i += 1;
                } else {
                    i += 1;
                }
            }
            cmd_serve(port, tls).await
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
