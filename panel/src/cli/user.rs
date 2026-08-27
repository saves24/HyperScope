use crate::cli::setup::read_password_interactive;
use crate::{
    hash_password, load_users, save_users, valid_user_name, validate_password_only,
    validate_user_input, validate_user_name_only, User,
};
pub(crate) fn cmd_user_add(args: &[String]) -> i32 {
    let Some(name) = args.get(2) else {
        eprintln!("usage: hyper-panel user add <username>");
        return 1;
    };
    let name = name.trim();
    let Ok(password) = read_password_interactive(&format!("password for {name}: ")) else {
        return 1;
    };
    if let Err(e) = validate_user_input(name, &password) {
        eprintln!("{e}");
        return 1;
    }
    let mut users = load_users();
    if users.iter().any(|u| u.name == name) {
        eprintln!("user {name} already exists");
        return 1;
    }
    let hash = match hash_password(&password) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("password hashing failed: {error}");
            return 1;
        }
    };
    users.push(User {
        name: name.to_string(),
        salt: String::new(),
        hash,
        is_admin: false,
    });
    if let Err(e) = save_users(&users) {
        eprintln!("save failed: {e}");
        return 1;
    }
    println!("user {name} added, takes effect after restarting hyper-panel");
    0
}

// hyper-panel user del <username>
pub(crate) fn cmd_user_del(args: &[String]) -> i32 {
    let Some(name) = args.get(2) else {
        eprintln!("usage: hyper-panel user del <username>");
        return 1;
    };
    if !valid_user_name(name) {
        eprintln!("username may only contain letters and digits");
        return 1;
    }
    let mut users = load_users();
    if users.iter().any(|u| u.name == *name && u.is_admin) {
        eprintln!("cannot delete an admin account");
        return 1;
    }
    if users.len() <= 1 {
        eprintln!("cannot delete the last user");
        return 1;
    }
    let Some(idx) = users.iter().position(|u| u.name == *name) else {
        eprintln!("user {name} not found");
        return 1;
    };
    users.remove(idx);
    if let Err(e) = save_users(&users) {
        eprintln!("save failed: {e}");
        return 1;
    }
    println!("user {name} deleted, takes effect after restarting hyper-panel");
    0
}

// hyper-panel user passwd <username> (interactive new password)
pub(crate) fn cmd_user_passwd(args: &[String]) -> i32 {
    let Some(name) = args.get(2) else {
        eprintln!("usage: hyper-panel user passwd <username>");
        return 1;
    };
    let mut users = load_users();
    let Some(idx) = users.iter().position(|u| u.name == *name) else {
        eprintln!("user {name} not found");
        return 1;
    };
    let Ok(password) = read_password_interactive(&format!("new password for {name}: ")) else {
        return 1;
    };
    if let Err(e) = validate_password_only(&password) {
        eprintln!("{e}");
        return 1;
    }
    let hash = match hash_password(&password) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("password hashing failed: {error}");
            return 1;
        }
    };
    users[idx].salt = String::new();
    users[idx].hash = hash;
    if let Err(e) = save_users(&users) {
        eprintln!("save failed: {e}");
        return 1;
    }
    println!("password for {name} changed, takes effect after restarting hyper-panel");
    0
}

// hyper-panel user rename <username> <new-username>
pub(crate) fn cmd_user_rename(args: &[String]) -> i32 {
    let Some(old) = args.get(2) else {
        eprintln!("usage: hyper-panel user rename <old-username> <new-username>");
        return 1;
    };
    let Some(new_name) = args.get(3) else {
        eprintln!("usage: hyper-panel user rename <old-username> <new-username>");
        return 1;
    };
    if let Err(e) = validate_user_name_only(new_name) {
        eprintln!("{e}");
        return 1;
    }
    let mut users = load_users();
    let Some(idx) = users.iter().position(|u| u.name == *old) else {
        eprintln!("user {old} not found");
        return 1;
    };
    // Admin-role users cannot be renamed
    if users[idx].is_admin {
        eprintln!("cannot rename an admin user");
        return 1;
    }
    if users.iter().any(|u| u.name == *new_name) {
        eprintln!("user {new_name} already exists");
        return 1;
    }
    users[idx].name = new_name.clone();
    if let Err(e) = save_users(&users) {
        eprintln!("save failed: {e}");
        return 1;
    }
    println!("user {old} renamed to {new_name}, takes effect after restarting hyper-panel");
    0
}

// hyper-panel user list
pub(crate) fn cmd_user_list() -> i32 {
    let users = load_users();
    if users.is_empty() {
        println!("no users");
        return 0;
    }
    println!("users ({}):", users.len());
    for u in &users {
        let tag = if u.is_admin { " [admin]" } else { "" };
        println!("  {}{}", u.name, tag);
    }
    0
}

// Read user list (auth.json); auto-convert legacy single-user format

// Save user list to auth.json (permissions 600)

// Username validity: letters and digits only

// Password validity: letters and digits only

// Validate: username + password (on create/add)

// Validate: username only (on rename)

// Validate: password only (on password change)

// Password hashing: argon2 (PHC format, embeds random salt)
// Legacy uses SHA256(salt:pass); new passwords always use argon2

// Constant-time string comparison (timing-attack resistant)

// Verify password: argon2 first (new format), fallback to legacy SHA256 (64-hex)
