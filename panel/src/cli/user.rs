use crate::cli::setup::read_password_interactive;
use crate::{hash_password, load_users, save_users, validate_password_only};
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
