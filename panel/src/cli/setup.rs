use crate::{
    hash_password, port_in_use, save_users, validate_user_input, User, AUTH_FILE, DEFAULT_PORT,
    SETTINGS_FILE,
};
use serde_json::{json, Value};
pub(crate) fn cmd_port(args: &[String]) -> i32 {
    if let Some(p) = args.get(1) {
        match p.parse::<u16>() {
            Ok(port) if (1..=65535).contains(&port) => {
                if port != load_panel_port() && port_in_use(port) {
                    eprintln!("port {port} is in use, try another");
                    return 1;
                }
                match save_panel_port(port) {
                    Ok(()) => {
                        println!("panel port set to {port}, takes effect after restart (hyper-panel serve)");
                        0
                    }
                    Err(e) => {
                        eprintln!("set failed: {e}");
                        1
                    }
                }
            }
            _ => {
                eprintln!("invalid port: {p} (must be 1-65535)");
                1
            }
        }
    } else {
        println!("current panel port: {}", load_panel_port());
        0
    }
}

// Login: POST /api/login {user, pass} -> {token} (valid 1 day)

// Auth middleware: all /api/* require valid token (cookie/Bearer; login/static paths bypass)

// ===== User management (admin only) =====

// Get current logged-in user (injected into request header by middleware)

// Check whether current user is admin

// Whether node is visible to current user (single admin user; all nodes visible)

// Current user: GET /api/me -> {user, is_admin}

// Static assets (embedded frontend)

// ---------- CLI ----------

pub(crate) fn load_panel_port() -> u16 {
    if let Ok(content) = std::fs::read_to_string(SETTINGS_FILE) {
        if let Ok(v) = serde_json::from_str::<Value>(&content) {
            if let Some(p) = v.get("panel_port").and_then(|p| p.as_u64()) {
                return p as u16;
            }
        }
    }
    DEFAULT_PORT
}

pub(crate) fn save_panel_port(port: u16) -> Result<(), String> {
    let dir = std::path::Path::new(SETTINGS_FILE).parent().unwrap();
    std::fs::create_dir_all(dir).map_err(|e| format!("failed to create directory: {e}"))?;
    let v = json!({ "panel_port": port });
    crate::atomic_write(
        SETTINGS_FILE,
        &serde_json::to_string_pretty(&v).unwrap(),
        0o600,
    )
}

// Reset admin account: hyper-panel setup [--user <username>] (overwrites auth.json, keeps only that account)
pub(crate) fn cmd_setup(args: &[String]) -> i32 {
    let mut user = "admin".to_string();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--user" && i + 1 < args.len() {
            user = args[i + 1].clone();
            i += 2;
        } else {
            i += 1;
        }
    }
    if user.is_empty() {
        eprintln!("username cannot be empty");
        return 1;
    }

    println!("reset admin account (will overwrite all users)");
    println!("  username: {user}");
    print!("  password: ");
    use std::io::Write;
    std::io::stdout().flush().ok();
    let password = match rpassword::read_password() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("\nfailed to read password: {e}");
            return 1;
        }
    };
    println!();
    if let Err(e) = validate_user_input(&user, &password) {
        eprintln!("{e}");
        return 1;
    }

    // Random salt + salted hash (argon2, PHC format embeds salt)
    let hash = match hash_password(&password) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("password hashing failed: {error}");
            return 1;
        }
    };
    let users = vec![User {
        name: user,
        salt: String::new(),
        hash,
        is_admin: true,
    }];
    if let Err(e) = save_users(&users) {
        eprintln!("save failed: {e}");
        return 1;
    }
    println!("login info saved to {AUTH_FILE}");
    println!("takes effect after restarting hyper-panel (hyper-panel serve)");
    0
}

// ===== CLI user management =====

// Read password interactively
pub(crate) fn read_password_interactive(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    use std::io::Write;
    std::io::stdout().flush().ok();
    match rpassword::read_password() {
        Ok(p) => {
            println!();
            Ok(p)
        }
        Err(e) => Err(format!("failed to read password: {e}")),
    }
}

// hyper-panel user add <username> (interactive password)
