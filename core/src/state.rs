// Shared state and data structures
use crate::client::AsyncReadWrite;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::Mutex;

#[cfg(target_os = "linux")]
pub const AUTH_FILE: &str = "/etc/hyper-panel/auth.json";
#[cfg(target_os = "windows")]
pub const AUTH_FILE: &str = "C:\\ProgramData\\hyper-panel\\auth.json";
pub const SETTINGS_FILE: &str = "/etc/hyper-panel/panel.json";

// Node configuration
#[derive(Clone, Debug)]
pub struct NodeConfig {
    pub id: String, // stable unique id (internal key for history/events; survives rename)
    pub name: String,
    pub addr: String,
    pub port: u16,
    pub key: String,
    pub owner: String,   // owning user (admin sees all)
    pub tls: bool,       // whether to use HTTPS connection
    pub cert_fp: String, // node cert SHA256 fingerprint (checked when TLS; empty = unchecked)
    pub push: bool,      // true = node pushes metrics (no listening port)
}

impl NodeConfig {
    pub fn base_url(&self) -> String {
        let scheme = if self.tls { "https" } else { "http" };
        // IPv6 addresses need brackets in URLs
        let host = if self.addr.contains(':') && !self.addr.starts_with('[') {
            format!("[{}]", self.addr)
        } else {
            self.addr.clone()
        };
        format!("{scheme}://{host}:{}", self.port)
    }
    pub fn from_value(v: &Value) -> Option<Self> {
        let name = v["name"].as_str()?.trim();
        let addr = v["addr"].as_str()?.trim();
        let key = v["key"].as_str()?;
        let port_raw = v["port"].as_u64()?;
        // Validate early: reject empty name/addr/key and out-of-range port
        if name.is_empty() || addr.is_empty() || key.is_empty() {
            return None;
        }
        if port_raw == 0 || port_raw > 65535 {
            return None; // reject truncation (70000 must not silently become 4464)
        }
        Some(Self {
            // Legacy configs without id get one generated (stable per file load)
            id: v
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or("")
                .to_string(),
            name: v["name"].as_str()?.to_string(),
            addr: v["addr"].as_str()?.to_string(),
            port: port_raw as u16,
            key: v["key"].as_str()?.to_string(),
            // owner defaults to admin when missing
            owner: v
                .get("owner")
                .and_then(|o| o.as_str())
                .unwrap_or("admin")
                .to_string(),
            // tls defaults to false (plaintext) when missing
            tls: v.get("tls").and_then(|t| t.as_bool()).unwrap_or(false),
            cert_fp: v
                .get("cert_fp")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string(),
            push: v.get("push").and_then(|p| p.as_bool()).unwrap_or(false),
        })
    }
    pub fn to_value(&self) -> Value {
        json!({ "id": self.id, "name": self.name, "addr": self.addr, "port": self.port, "key": self.key, "owner": self.owner, "tls": self.tls, "cert_fp": self.cert_fp, "push": self.push })
    }
    // Ensure a node has an id (legacy configs); called on load
    pub fn ensure_id(&mut self) {
        if self.id.is_empty() {
            self.id = crate::generate_node_id();
        }
    }
}

// Node state cache
#[derive(Clone)]
pub struct NodeState {
    pub config: NodeConfig,
    pub data: Option<Value>,
    pub data_ts: u64, // push timestamp (dedupe WS vs POST)
    pub traffic_cache: Option<Value>,
    pub io_cache: Option<Value>,
    pub status: String, // online | offline | unauthorized | unknown
}

// Auth user: name + salt + password hash + admin flag (admin is a role, not tied to username)
#[derive(Clone)]
pub struct User {
    pub name: String,
    pub salt: String,
    pub hash: String,
    pub is_admin: bool,
}

// Plaintext TCP and TLS streams share the same read/write trait

pub struct AppState {
    pub nodes: Mutex<Vec<NodeState>>,
    // Config file mtime, used to reload after external edits
    pub config_mtime: Mutex<Option<std::time::SystemTime>>,
    // Event log (node offline/online etc., keep at most 100 entries)
    pub events: Mutex<Vec<Value>>,
    // Login token: token + expiry + owning user
    pub tokens: Mutex<Vec<(String, u64, String)>>,
    // Auth config: user list (auth.json)
    pub auth: Mutex<Vec<User>>,
    // Node connection pool (keep-alive reuse): key = "host:port:tls"
    pub conns: Mutex<HashMap<String, Box<dyn AsyncReadWrite + Send + Unpin>>>,
}

// Shared state alias used across the core and the web/desktop layers
pub type SharedState = std::sync::Arc<AppState>;
