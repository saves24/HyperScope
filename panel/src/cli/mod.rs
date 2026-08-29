// CLI commands: node/user/setup/log management + command dispatch.
// The dispatch lives here so main.rs stays focused on server bootstrap.
pub(crate) mod log;
pub(crate) mod node;
pub(crate) mod setup;
pub(crate) mod user;

pub(crate) use log::*;
pub(crate) use node::*;
pub(crate) use setup::*;
pub(crate) use user::*;

/// Dispatches non-async CLI commands. Returns (exit_code, Some(serve_command))
/// where serve_command is the port when the user asked to serve (handled by
/// the caller because it needs the tokio runtime).
pub(crate) fn dispatch(args: &[String]) -> (i32, Option<u16>) {
    let code = match args.first().map(|s| s.as_str()) {
        None | Some("help") | Some("--help") | Some("-h") => {
            print_help();
            0
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("add") => cmd_add_node(args),
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("link") => cmd_link_node(args),
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("show") => cmd_node_show(args),
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("del") => {
            cmd_remove_node_by_alias(args)
        }
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("rename") => cmd_rename_node(args),
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("ping") => cmd_ping_node(args),
        Some("node") if args.get(1).map(|s| s.as_str()) == Some("list") => cmd_nodes(),
        Some("identity") if args.get(1).map(|s| s.as_str()) == Some("show") => {
            // Show the panel's Ed25519 identity public key so it can be added
            // to each node's trusted-device list (hyper-node device add).
            match hyper_panel_core::identity::ensure_identity() {
                Ok(pubkey) => {
                    println!("{pubkey}");
                    0
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        Some("setup") => cmd_setup(args),
        Some("user") => match args.get(1).map(|s| s.as_str()) {
            Some("passwd") => cmd_user_passwd(args),
            _ => {
                eprintln!("usage: hyper-panel user passwd <username>");
                1
            }
        },
        Some("port") => cmd_port(args),
        Some("log") => match args.get(1).map(|s| s.as_str()) {
            Some("show") => cmd_log_show(args),
            Some("system") => cmd_log_system(args),
            Some("retention") => cmd_log_retention(args),
            _ => {
                eprintln!(
                    "usage: hyper-panel log show [N] | log system [N] | log retention <days>"
                );
                1
            }
        },
        Some("serve") => {
            let mut port = load_panel_port();
            let mut i = 1;
            while i < args.len() {
                if args[i] == "--port" {
                    if let Some(p) = args.get(i + 1).and_then(|s| s.parse().ok()) {
                        port = p;
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            return (0, Some(port));
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            eprintln!("run 'hyper-panel help' for usage");
            1
        }
    };
    (code, None)
}
