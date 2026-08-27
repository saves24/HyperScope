use crate::{
    gen_random_password, now_unix, sha256_hex, valid_user_name, validate_password_only,
    validate_user_input, validate_user_name_only, Duration, SharedState, User, AUTH_FILE,
};
// Auth: login/users/token/argon2
use axum::{
    extract::{ConnectInfo, Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex as StdMutex};

// Login rate limit per username+IP: 5 failed attempts within a minute locks that source for 1 minute
static LOGIN_FAILS: LazyLock<StdMutex<HashMap<String, (u32, u64)>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

pub(crate) async fn login_handler(
    State(app): State<SharedState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let auth_guard = app.auth.lock().await.clone();
    let input_user = body.get("user").and_then(|v| v.as_str()).unwrap_or("");
    let input_pass = body.get("pass").and_then(|v| v.as_str()).unwrap_or("");
    // Key on source IP + username so one attacker can't lock out an account for everyone
    let fail_key = format!("{}|{}", addr.ip(), input_user);
    let now = now_unix();
    // Rate limit: locked source returns 429
    {
        let mut fails = LOGIN_FAILS.lock().unwrap();
        // Opportunistic cleanup: drop expired entries and cap the map size
        if fails.len() > 5000 {
            fails.retain(|_, (_, lock)| *lock > now);
        }
        if let Some((_, lock_until)) = fails.get(&fail_key) {
            if *lock_until > now {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(json!({"error": "too many attempts, try again later"})),
                )
                    .into_response();
            }
        }
    }
    // Match against user list
    let matched = auth_guard
        .iter()
        .any(|u| u.name == input_user && check_password(&u.salt, &u.hash, input_pass));
    if !matched {
        // Brute-force mitigation: constant delay + failure counter
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut fails = LOGIN_FAILS.lock().unwrap();
        let entry = fails.entry(fail_key).or_insert((0, 0));
        entry.0 += 1;
        if entry.0 >= 5 {
            entry.0 = 0;
            entry.1 = now + 60; // lock 1 minute
        }
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "invalid username or password"})),
        )
            .into_response();
    }
    // Success resets the counter
    LOGIN_FAILS.lock().unwrap().remove(&fail_key);
    let token = gen_random_password(32);
    let expires = now_unix() + 86400;
    let mut tokens = app.tokens.lock().await;
    tokens.retain(|(_, exp, _)| *exp > now_unix());
    if tokens.len() >= 50 {
        tokens.clear();
    }
    tokens.push((token.clone(), expires, input_user.to_string()));
    drop(tokens);
    (
        StatusCode::OK,
        [(
            axum::http::header::SET_COOKIE,
            format!("ts-token={token}; Max-Age=86400; Path=/; HttpOnly; SameSite=Lax"),
        )],
        Json(json!({"ok": true})),
    )
        .into_response()
}
pub(crate) async fn auth_middleware(
    State(app): State<SharedState>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let path = req.uri().path().to_string();
    if path == "/api/login"
        || path == "/api/logout"
        || path == "/api/setup"
        || path == "/api/status"
        || path == "/api/push"
        || path == "/ws"
        || path == "/"
        || path.starts_with("/static/")
        || path == "/health"
    {
        return next.run(req).await;
    }
    // Read token from cookie (browser-native); also support Authorization Bearer header
    let cookie_header = req
        .headers()
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let cookie_token = cookie_header.split(';').find_map(|part| {
        let p = part.trim();
        p.strip_prefix("ts-token=").map(|v| v.to_string())
    });
    let header_token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    let token = cookie_token.or(header_token);
    let now = now_unix();
    let mut tokens = app.tokens.lock().await;
    tokens.retain(|(_, exp, _)| *exp > now);
    let current_user = match token {
        Some(t) => tokens
            .iter()
            .find(|(s, _, _)| *s == t)
            .map(|(_, _, u)| u.clone()),
        None => None,
    };
    let ok = current_user.is_some();
    if ok {
        drop(tokens);
        // Inject current user into request header for handlers to check admin
        let mut req = req;
        if let Ok(v) = axum::http::HeaderValue::from_str(&current_user.unwrap_or_default()) {
            req.headers_mut().insert("x-current-user", v);
        }
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "unauthorized"})),
        )
            .into_response()
    }
}
pub(crate) async fn logout_handler(
    State(app): State<SharedState>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let cookie_header = req
        .headers()
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let cookie_token = cookie_header
        .split(';')
        .find_map(|part| part.trim().strip_prefix("ts-token=").map(|v| v.to_string()));
    let header_token = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    if let Some(t) = cookie_token.or(header_token) {
        app.tokens.lock().await.retain(|(s, _, _)| *s != t);
    }
    // Clear browser cookie
    (
        StatusCode::OK,
        [(
            axum::http::header::SET_COOKIE,
            "ts-token=; Max-Age=0; Path=/; HttpOnly; SameSite=Lax".to_string(),
        )],
        Json(json!({"ok": true})),
    )
        .into_response()
}
pub(crate) fn current_user(headers: &HeaderMap) -> String {
    headers
        .get("x-current-user")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}
pub(crate) async fn is_admin(app: &SharedState, user: &str) -> bool {
    let users = app.auth.lock().await;
    users.iter().any(|u| u.name == user && u.is_admin)
}
pub(crate) async fn me_handler(
    headers: HeaderMap,
    State(app): State<SharedState>,
) -> impl IntoResponse {
    let user = current_user(&headers);
    if user.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "not logged in"})),
        )
            .into_response();
    }
    let admin = is_admin(&app, &user).await;
    Json(json!({ "user": user, "is_admin": admin })).into_response()
}
pub(crate) async fn users_list_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_admin(&app, &current_user(&headers)).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "only admin can manage users"})),
        )
            .into_response();
    }
    let users = app.auth.lock().await.clone();
    let list: Vec<Value> = users
        .iter()
        .map(|u| json!({ "name": u.name, "is_admin": u.is_admin }))
        .collect();
    Json(json!({ "users": list })).into_response()
}
pub(crate) async fn users_add_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    if !is_admin(&app, &current_user(&headers)).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "only admin can manage users"})),
        )
            .into_response();
    }
    let new_user = body
        .get("user")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let new_pass = body.get("pass").and_then(|v| v.as_str()).unwrap_or("");
    if let Err(e) = validate_user_input(&new_user, new_pass) {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e}))).into_response();
    }
    let mut users = app.auth.lock().await.clone();
    if users.iter().any(|u| u.name == new_user) {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": "user already exists"})),
        )
            .into_response();
    }
    let salt = String::new();
    let hash = match hash_password(new_pass) {
        Ok(hash) => hash,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": error})),
            )
                .into_response();
        }
    };
    users.push(User {
        name: new_user,
        salt,
        hash,
        is_admin: false,
    });
    if let Err(e) = save_users(&users) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response();
    }
    *app.auth.lock().await = users;
    Json(json!({"ok": true})).into_response()
}
pub(crate) async fn users_update_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    if !is_admin(&app, &current_user(&headers)).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "only admin can manage users"})),
        )
            .into_response();
    }
    let mut users = app.auth.lock().await.clone();
    let Some(idx) = users.iter().position(|u| u.name == name) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "user not found"})),
        )
            .into_response();
    };
    // Rename (admin-role users cannot be renamed)
    if let Some(new_name) = body.get("new_name").and_then(|v| v.as_str()) {
        let new_name = new_name.trim();
        if let Err(e) = validate_user_name_only(new_name) {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": e}))).into_response();
        }
        if users[idx].is_admin && new_name != users[idx].name {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "cannot rename an admin user"})),
            )
                .into_response();
        }
        if users.iter().any(|u| u.name == new_name) {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error": "user already exists"})),
            )
                .into_response();
        }
        users[idx].name = new_name.to_string();
    }
    // Change password
    if let Some(new_pass) = body.get("pass").and_then(|v| v.as_str()) {
        if let Err(e) = validate_password_only(new_pass) {
            return (StatusCode::BAD_REQUEST, Json(json!({"error": e}))).into_response();
        }
        let hash = match hash_password(new_pass) {
            Ok(hash) => hash,
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": error})),
                )
                    .into_response();
            }
        };
        users[idx].salt = String::new();
        users[idx].hash = hash;
    }
    if let Err(e) = save_users(&users) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response();
    }
    *app.auth.lock().await = users;
    Json(json!({"ok": true})).into_response()
}
pub(crate) async fn users_delete_handler(
    State(app): State<SharedState>,
    headers: HeaderMap,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> impl IntoResponse {
    if !is_admin(&app, &current_user(&headers)).await {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "only admin can manage users"})),
        )
            .into_response();
    }
    if !valid_user_name(&name) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "username may only contain letters and digits"})),
        )
            .into_response();
    }
    let mut users = app.auth.lock().await.clone();
    // Cannot delete an admin-role user
    if users.iter().any(|u| u.name == name && u.is_admin) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "cannot delete an admin user"})),
        )
            .into_response();
    }
    if users.len() <= 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "cannot delete the last user"})),
        )
            .into_response();
    }
    let Some(idx) = users.iter().position(|u| u.name == name) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "user not found"})),
        )
            .into_response();
    };
    users.remove(idx);
    if let Err(e) = save_users(&users) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))).into_response();
    }
    *app.auth.lock().await = users;
    Json(json!({"ok": true})).into_response()
}
pub(crate) fn load_users() -> Vec<User> {
    let Ok(content) = std::fs::read_to_string(AUTH_FILE) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<Value>(&content) else {
        return Vec::new();
    };
    // New format: {"users": [...]}
    if let Some(users) = v.get("users").and_then(|u| u.as_array()) {
        return users
            .iter()
            .filter_map(|u| {
                Some(User {
                    name: u.get("user")?.as_str()?.to_string(),
                    // Legacy files (pre-is_admin) treat "admin" as admin
                    is_admin: u
                        .get("is_admin")
                        .and_then(|v| v.as_bool())
                        .unwrap_or_else(|| u.get("user").and_then(|s| s.as_str()) == Some("admin")),
                    salt: u
                        .get("salt")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string(),
                    hash: u.get("pass_hash")?.as_str()?.to_string(),
                })
            })
            .collect();
    }
    // Legacy single-user format: {"user": ..., "salt": ..., "pass_hash": ...}
    if let (Some(user), Some(hash)) = (
        v.get("user").and_then(|u| u.as_str()),
        v.get("pass_hash").and_then(|h| h.as_str()),
    ) {
        let users = vec![User {
            name: user.to_string(),
            is_admin: true,
            salt: v
                .get("salt")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            hash: hash.to_string(),
        }];
        let _ = save_users(&users);
        return users;
    }
    Vec::new()
}

pub(crate) fn validate_auth_file() -> Result<bool, String> {
    let content = match std::fs::read_to_string(AUTH_FILE) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("failed to read auth config: {error}")),
    };
    let value: Value = serde_json::from_str(&content)
        .map_err(|error| format!("invalid auth config JSON: {error}"))?;
    let valid = value.get("users").and_then(Value::as_array).is_some()
        || (value.get("user").and_then(Value::as_str).is_some()
            && value.get("pass_hash").and_then(Value::as_str).is_some());
    if valid {
        Ok(true)
    } else {
        Err("auth config has an unsupported format".to_string())
    }
}
pub(crate) fn save_users(users: &[User]) -> Result<(), String> {
    let arr: Vec<Value> = users
        .iter()
        .map(|u| json!({ "user": u.name, "is_admin": u.is_admin, "salt": u.salt, "pass_hash": u.hash }))
        .collect();
    let data = json!({ "users": arr });
    let dir = std::path::Path::new(AUTH_FILE).parent().unwrap();
    std::fs::create_dir_all(dir).map_err(|e| format!("failed to create directory: {e}"))?;
    crate::atomic_write(
        AUTH_FILE,
        &serde_json::to_string_pretty(&data)
            .map_err(|e| format!("failed to serialize auth config: {e}"))?,
        0o600,
    )
}
pub(crate) fn hash_password(password: &str) -> Result<String, String> {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    use argon2::{Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| format!("password hashing failed: {e}"))
}
pub(crate) fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
pub(crate) fn check_password(salt: &str, pass_hash: &str, password: &str) -> bool {
    if pass_hash.starts_with("$argon2") {
        use argon2::{Argon2, PasswordVerifier};
        let parsed = argon2::PasswordHash::new(pass_hash);
        match parsed {
            Ok(ph) => Argon2::default()
                .verify_password(password.as_bytes(), &ph)
                .is_ok(),
            Err(_) => false,
        }
    } else {
        constant_time_eq(&sha256_hex(&format!("{salt}:{password}")), pass_hash)
    }
}
