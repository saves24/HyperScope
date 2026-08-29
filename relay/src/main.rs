// hyper-relay: relay agent for the HyperScope P2P protocol.
//
// Runs as a system service on the same machine as hyper-node. It is the only
// persistent process and exposes a single port; the collector is woken on
// demand through a local Unix socket. The relay holds no signing keys — it is
// a zero-privilege pipe for addresses, registrations and command forwarding.
mod local_wake;
mod protocol;
mod server;
#[cfg(target_os = "windows")]
mod windows_service;

use std::net::SocketAddr;

pub(crate) const DEFAULT_PORT: u16 = 8686;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(|s| s.as_str()) {
        None | Some("help") | Some("--help") | Some("-h") => {
            print_help();
            0
        }
        Some("serve") => {
            let mut port = DEFAULT_PORT;
            let mut cert: Option<String> = None;
            let mut key: Option<String> = None;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--port" => {
                        if let Some(p) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                            port = p;
                        }
                        i += 2;
                    }
                    "--tls-cert" => {
                        cert = args.get(i + 1).cloned();
                        i += 2;
                    }
                    "--tls-key" => {
                        key = args.get(i + 1).cloned();
                        i += 2;
                    }
                    _ => i += 1,
                }
            }
            serve_tls(port, cert.as_deref(), key.as_deref()).await
        }
        Some("service") => {
            // Windows SCM entry: run the relay as a native service.
            #[cfg(target_os = "windows")]
            {
                match windows_service::run_windows_service() {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("service error: {e}");
                        1
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                eprintln!("'service' is only supported on Windows");
                1
            }
        }
        _ => {
            eprintln!("unknown command; run 'hyper-relay help'");
            1
        }
    };
    std::process::exit(code);
}

fn print_help() {
    println!("hyper-relay — HyperScope relay agent (zero-privilege pipe)");
    println!("Usage: hyper-relay <command> [args]");
    println!();
    println!("Commands:");
    println!("  serve [--port N]      start the relay service (default {DEFAULT_PORT})");
    println!("        [--tls-cert P]  PEM certificate for WSS (TLS) mode");
    println!("        [--tls-key P]   PEM private key for WSS (TLS) mode");
    println!("  service               run as a Windows native service (SCM)");
    println!("  help                  show this help");
    println!();
    println!("The relay runs on the same machine as hyper-node and wakes the");
    println!("collector on demand by spawning a one-shot process (`hyper-node collect`");
    println!("/ `hyper-node control`). It exposes a single port and holds no signing");
    println!("keys. With --tls-cert/--tls-key the port serves WSS (wss://) instead of");
    println!("plain WS.");
}

async fn serve_tls(port: u16, cert: Option<&str>, key: Option<&str>) -> i32 {
    // Install the default rustls CryptoProvider (ring). Required before
    // building any ServerConfig with TLS enabled.
    if cert.is_some() || key.is_some() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse().expect("invalid addr");
    if let Err(e) = server::run_with_tls(addr, cert, key).await {
        eprintln!("hyper-relay stopped: {e}");
        return 1;
    }
    0
}
