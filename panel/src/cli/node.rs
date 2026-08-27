use crate::{
    generate_node_id, load_nodes, resolve_safe_addr, save_nodes, NodeConfig, DEFAULT_PORT,
    DEFAULT_RETENTION_DAYS, LOG_DIR, VERSION,
};
pub(crate) fn print_help() {
    println!(
        r#"hyper-panel - system monitoring panel aggregator (v{VERSION})

Usage:
  hyper-panel <command> [options]

Commands:
  node add <address> <key>              add node (default port 5000; batch: {{addr key}}{{addr key}}...; --tls enable encrypted connection)
  node link [--tls|--plain] <address> <key>  connect node (--tls encrypted / --plain plaintext test; default: auto TLS when key has fingerprint)
  node rename <name> <new-name>         rename node
  node ping <name>                      ping test node reachability
  node add -f <file>                    batch import nodes from file (one "address[:port] key" per line)
  node del <name>                       remove node from config
  node list                             list all configured nodes
  node show <name>                      show node details (including connectivity)
  setup [--user <username>]            reset admin account (overwrites all users, default admin, interactive password)
  user add <username>                  add user (interactive password)
  user del <username>                  delete user
  user passwd <username>               change user password (interactive)
  user rename <old> <new>              rename user
  user list                            list all users
  port [N]                             view/set panel port (default {DEFAULT_PORT}, takes effect on restart)
  log show [N]                         view panel log (last N lines, default 50)
  log system [N]                       view host systemd service log (journalctl -u hyper-panel, default 50)
  log retention <days>                 set log retention days (default {DEFAULT_RETENTION_DAYS})
  serve [--port N]                     start aggregator service (default {DEFAULT_PORT})
  help                                 show this help

Notes:
  - Nodes can also be added/removed from the web UI (same API/config)
  - Config changes auto-reload, no restart needed
  - Log dir {LOG_DIR}, daily rotation, auto cleanup
  - Login: default admin/admin auto-created on first start without accounts, change password with user passwd
  - User management: admin is unique, added users are regular (only view their own nodes)
"#
    );
}

// Parse single node entry: "address[:port] key" -> Option<(addr, port, key)>
pub(crate) fn parse_node_entry(s: &str) -> Option<(String, u16, String)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut parts = s.split_whitespace();
    let addr_input = parts.next()?;
    let key = parts.next()?.to_string();
    let (addr, port) = parse_node_address(addr_input)?;
    if addr.is_empty() {
        return None;
    }
    Some((addr, port, key))
}

// Add node: hyper-panel node add <address> <key> [--owner <username>]
// Batch: node add {addr1 key1}{addr2 key2}... ({} separates groups)
// File import: node add -f <file> (one "address key" per line)

// Parse "host:port" (default port 5000); handles IPv4, bare IPv6, and [IPv6]:port
pub(crate) fn parse_node_address(input: &str) -> Option<(String, u16)> {
    let input = input.trim();
    if input.starts_with('[') {
        // [IPv6]:port
        if let Some(rest) = input.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                let host = &rest[..end];
                let after = &rest[end + 1..];
                let port = if let Some(p) = after.strip_prefix(':') {
                    p.parse().ok()?
                } else {
                    5000
                };
                return Some((host.to_string(), port));
            }
        }
        return None;
    }
    if input.matches(':').count() > 1 {
        // Bare IPv6 (no port)
        return Some((input.to_string(), 5000));
    }
    match input.split_once(':') {
        Some((h, p)) => {
            let port: u16 = p.parse().ok()?;
            Some((h.to_string(), port))
        }
        None => Some((input.to_string(), 5000)),
    }
}

pub(crate) fn cmd_add_node(args: &[String]) -> i32 {
    let mut entries: Vec<(String, u16, String)> = Vec::new();
    let mut owner = "admin".to_string();
    let mut push_mode = false;
    let mut i = 2;
    while i < args.len() {
        let a = &args[i];
        if a == "--owner" {
            if let Some(o) = args.get(i + 1) {
                owner = o.clone();
                i += 2;
                continue;
            }
        }
        if a == "--tls" {
            i += 1;
            continue;
        }
        if a == "--push" {
            push_mode = true;
            i += 1;
            continue;
        }
        if a == "-f" || a == "--file" {
            // File import: one "address key" per line
            if let Some(path) = args.get(i + 1) {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let mut added = 0;
                        for line in content.lines() {
                            if let Some(e) = parse_node_entry(line) {
                                entries.push(e);
                                added += 1;
                            }
                        }
                        if added == 0 {
                            eprintln!(
                                "no valid entries in {path} (one \"address[:port] key\" per line)"
                            );
                            return 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("failed to read file: {e}");
                        return 1;
                    }
                }
                i += 2;
                continue;
            }
        }
        // Brace batch: {addr1 key1}{addr2 key2}
        if a.starts_with('{') {
            let mut rest = a.as_str();
            let mut count = 0;
            while let Some(start) = rest.find('{') {
                rest = &rest[start + 1..];
                if let Some(end) = rest.find('}') {
                    let inner = &rest[..end];
                    if let Some(e) = parse_node_entry(inner) {
                        entries.push(e);
                        count += 1;
                    }
                    rest = &rest[end + 1..];
                } else {
                    break;
                }
            }
            if count == 0 {
                eprintln!(
                    "invalid brace format: {a} (expected {{addr[:port] key}}{{addr key}}...)"
                );
                return 1;
            }
            i += 1;
            continue;
        }
        // Single: address + key (two args)
        let addr_input = a.clone();
        let Some(key) = args.get(i + 1) else {
            eprintln!("usage: hyper-panel node add <address> <key>");
            eprintln!("  batch: hyper-panel node add {{addr1 key1}}{{addr2 key2}}...");
            eprintln!("  file: hyper-panel node add -f <file> (one \"address key\" per line)");
            return 1;
        };
        let key = key.clone();
        // key may follow address ("address key" or "address:port key" in one arg)
        let Some((addr, port)) = parse_node_address(&addr_input) else {
            eprintln!("invalid address or port: {addr_input}");
            return 1;
        };
        if addr.is_empty() {
            eprintln!("address cannot be empty");
            return 1;
        }
        entries.push((addr, port, key));
        i += 2;
    }

    if entries.is_empty() {
        eprintln!("usage: hyper-panel node add <address> <key>");
        eprintln!("  batch: hyper-panel node add {{addr1 key1}}{{addr2 key2}}...");
        eprintln!("  file: hyper-panel node add -f <file> (one \"address key\" per line)");
        return 1;
    }

    let mut nodes = load_nodes();
    let mut added = 0;
    let mut skipped = 0;
    for (addr, port, key) in entries {
        let name = addr.clone();
        if nodes.iter().any(|n| n.name == name) {
            eprintln!("node {name} already exists, skipped");
            skipped += 1;
            continue;
        }
        // --tls enables HTTPS; key with fingerprint (secret|SHA256:fp) auto-enables TLS
        let use_tls = args.iter().any(|a| a == "--tls");
        let mut cert_fp = String::new();
        let mut real_key = key.clone();
        let key_with_fp = real_key.clone();
        if let Some((pure_key, fp)) = key_with_fp.split_once('|') {
            real_key = pure_key.trim().to_string();
            cert_fp = fp.trim().to_string();
        }
        let tls_enabled = use_tls || !cert_fp.is_empty();
        let Ok(addr) = resolve_safe_addr(&addr, port) else {
            eprintln!("address not allowed or cannot be resolved: {addr}");
            continue;
        };
        nodes.push(NodeConfig {
            id: generate_node_id(),
            name: name.clone(),
            addr,
            port,
            key: real_key,
            owner: owner.clone(),
            tls: tls_enabled,
            cert_fp,
            push: push_mode,
        });
        println!("node {name} added (owner: {owner})");
        added += 1;
    }
    if added == 0 {
        eprintln!("no new nodes added ({skipped} already exist)");
        return 1;
    }
    match save_nodes(&nodes) {
        Ok(()) => {
            println!("batch done: {added} added, {skipped} skipped");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

// Link node: hyper-panel node link [--tls|--plain] <address[:port]> <key>
// --tls: force TLS; --plain: plaintext; default auto-detect (key with fingerprint -> TLS)
pub(crate) fn cmd_link_node(args: &[String]) -> i32 {
    let mode_tls = args.iter().any(|a| a == "--tls");
    let mode_plain = args.iter().any(|a| a == "--plain");
    let mut addr_arg: Option<String> = None;
    let mut key_arg: Option<String> = None;
    let mut owner_arg: Option<String> = None;
    let mut i = 3;
    while i < args.len() {
        let a = &args[i];
        if a == "--tls" || a == "--plain" {
            i += 1;
            continue;
        }
        if a == "--owner" {
            if let Some(v) = args.get(i + 1) {
                owner_arg = Some(v.clone());
            }
            i += 2;
            continue;
        }
        if addr_arg.is_none() {
            addr_arg = Some(a.clone());
        } else if key_arg.is_none() {
            key_arg = Some(a.clone());
        }
        i += 1;
    }
    let (Some(addr_input), Some(key)) = (addr_arg, key_arg) else {
        eprintln!("usage: hyper-panel node link [--tls|--plain] <address[:port]> <key>");
        return 1;
    };
    let Some((addr, port)) = parse_node_address(&addr_input) else {
        eprintln!("invalid address or port: {addr_input}");
        return 1;
    };
    if addr.is_empty() {
        eprintln!("address cannot be empty");
        return 1;
    }
    let name = addr.clone();
    let mut nodes = load_nodes();
    if nodes.iter().any(|n| n.name == name) {
        eprintln!("node {name} already exists, remove it first with node del");
        return 1;
    }
    // Parse fingerprint embedded in key
    let mut cert_fp = String::new();
    let mut real_key = key.clone();
    let key_with_fp = real_key.clone();
    if let Some((pure_key, fp)) = key_with_fp.split_once('|') {
        real_key = pure_key.trim().to_string();
        cert_fp = fp.trim().to_string();
    }
    // Connection mode: explicit --tls/--plain wins; otherwise key fingerprint implies TLS
    let tls_enabled = if mode_plain {
        false
    } else if mode_tls {
        true
    } else {
        !cert_fp.is_empty()
    };
    let mode_desc = if tls_enabled {
        "TLS encrypted"
    } else {
        "plain"
    };
    nodes.push(NodeConfig {
        id: generate_node_id(),
        name: name.clone(),
        addr,
        port,
        key: real_key,
        owner: owner_arg.unwrap_or_else(|| "admin".to_string()),
        tls: tls_enabled,
        cert_fp,
        push: false,
    });
    match save_nodes(&nodes) {
        Ok(()) => {
            println!("node {name} connected (mode: {mode_desc})");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

pub(crate) fn cmd_remove_node(args: &[String]) -> i32 {
    if args.len() < 3 {
        eprintln!("usage: hyper-panel node del <name>");
        return 1;
    }
    let name = args[2].clone();
    let mut nodes = load_nodes();
    let before = nodes.len();
    nodes.retain(|n| n.name != name);
    if nodes.len() == before {
        eprintln!("node {name} not found");
        return 1;
    }
    match save_nodes(&nodes) {
        Ok(()) => {
            println!("node {name} removed");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

// Delete node: node del <name>
pub(crate) fn cmd_remove_node_by_alias(args: &[String]) -> i32 {
    cmd_remove_node(args)
}

// Rename node: node rename <name> <new-name>
pub(crate) fn cmd_rename_node(args: &[String]) -> i32 {
    if args.len() < 4 {
        eprintln!("usage: hyper-panel node rename <name> <new-name>");
        return 1;
    }
    let old_name = args[2].clone();
    let new_name = args[3].trim().to_string();
    if new_name.is_empty() || new_name.len() > 100 {
        eprintln!("invalid new name");
        return 1;
    }
    let mut nodes = load_nodes();
    // Check duplicate name first (immutable borrow)
    if nodes.iter().any(|x| x.name == new_name) {
        eprintln!("node name {new_name} already exists");
        return 1;
    }
    let Some(n) = nodes.iter_mut().find(|n| n.name == old_name) else {
        eprintln!("node {old_name} not found");
        return 1;
    };
    n.name = new_name.clone();
    match save_nodes(&nodes) {
        Ok(()) => {
            println!("node {old_name} renamed to {new_name}");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

// Ping node: node ping <name>
pub(crate) fn cmd_ping_node(args: &[String]) -> i32 {
    if args.len() < 3 {
        eprintln!("usage: hyper-panel node ping <name>");
        return 1;
    }
    let name = args[2].clone();
    let nodes = load_nodes();
    let Some(n) = nodes.iter().find(|n| n.name == name) else {
        eprintln!("node {name} not found");
        return 1;
    };
    let addr = n.addr.clone();
    drop(nodes);
    println!("Pinging {name} ({addr}) ...");
    let output = std::process::Command::new("ping")
        .args(["-c", "4", "-W", "2", &addr])
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let text = if !stdout.trim().is_empty() {
                stdout
            } else {
                stderr
            };
            print!("{text}");
            let ok = out.status.success() || text.contains("ttl=") || text.contains("time=");
            println!();
            println!(
                "result: {}",
                if ok {
                    "reachable ✓"
                } else {
                    "unreachable ✗"
                }
            );
            if ok {
                0
            } else {
                1
            }
        }
        Err(e) => {
            eprintln!("ping failed: {e}");
            1
        }
    }
}

// Panel port config (panel.json)
pub(crate) fn cmd_nodes() -> i32 {
    let nodes = load_nodes();
    if nodes.is_empty() {
        println!("no nodes, add with 'hyper-panel node add <address> <key>'");
        return 0;
    }
    println!("{} nodes configured:", nodes.len());
    for (i, n) in nodes.iter().enumerate() {
        println!(
            "  [{i}] {}  {}:{}  (owner: {})",
            n.name, n.addr, n.port, n.owner
        );
    }
    0
}

// Show node details: hyper-panel node show <name>
pub(crate) fn cmd_node_show(args: &[String]) -> i32 {
    let name = match args.get(2) {
        Some(n) => n,
        None => {
            eprintln!("usage: hyper-panel node show <name>");
            return 1;
        }
    };
    let nodes = load_nodes();
    let node = match nodes.iter().find(|n| n.name == *name) {
        Some(n) => n,
        None => {
            eprintln!("node {name} not found, use 'hyper-panel node list' to see the list");
            return 1;
        }
    };
    println!("node: {}", node.name);
    println!("  address: {}:{}", node.addr, node.port);
    println!(
        "  key: {}",
        if node.key.is_empty() {
            "(empty)"
        } else {
            "configured"
        }
    );
    // Connectivity check
    match std::net::TcpStream::connect_timeout(
        &format!("{}:{}", node.addr, node.port)
            .parse()
            .unwrap_or_else(|_| {
                std::net::SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), 0)
            }),
        std::time::Duration::from_secs(3),
    ) {
        Ok(_) => println!("  status: online (port reachable)"),
        Err(e) => println!("  status: offline ({e})"),
    }
    0
}

// ---------- Log commands ----------
