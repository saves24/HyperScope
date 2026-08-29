// Protocol message types and the in-memory node registry.
//
// The relay understands only enough of each message to route it: it never
// interprets command payloads and holds no signing keys (see the protocol
// design doc outside the repository).
#![allow(dead_code)] // MsgType/constants are reserved for full protocol rollout.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROTOCOL_VERSION: &str = "1";
pub const HEARTBEAT_TIMEOUT_SECS: u64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MsgType {
    Register = 0x01,
    Heartbeat = 0x02,
    Query = 0x03,
    Offer = 0x04,
    Cmd = 0x05,
    CmdResult = 0x06,
    ConfirmReq = 0x07,
    Confirm = 0x08,
    Authorize = 0x09,
    Forward = 0x0A,
    Data = 0x0B,
    Error = 0x0C,
    AuthChallenge = 0x0D,
    AuthResponse = 0x0E,
    Subscribe = 0x0F,
    Unsubscribe = 0x10,
    Deauthorize = 0x11,
}

impl MsgType {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0x01 => Self::Register,
            0x02 => Self::Heartbeat,
            0x03 => Self::Query,
            0x04 => Self::Offer,
            0x05 => Self::Cmd,
            0x06 => Self::CmdResult,
            0x07 => Self::ConfirmReq,
            0x08 => Self::Confirm,
            0x09 => Self::Authorize,
            0x0A => Self::Forward,
            0x0B => Self::Data,
            0x0C => Self::Error,
            0x0D => Self::AuthChallenge,
            0x0E => Self::AuthResponse,
            0x0F => Self::Subscribe,
            0x10 => Self::Unsubscribe,
            0x11 => Self::Deauthorize,
            _ => return None,
        })
    }
}

/// A registered node: its public key, mode, and last-seen time.
#[derive(Debug, Clone)]
pub struct NodeEntry {
    pub name: String,
    pub pubkey: Vec<u8>,
    pub mode: String,
    pub last_seen: u64,
    /// Temporary direct-connect address (ip:port), if offered.
    pub offer: Option<OfferInfo>,
    /// Connected websocket peer id for command forwarding.
    pub conn: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct OfferInfo {
    pub addr: String,
    pub proto: String,
    pub expires: u64,
}

pub struct Registry {
    nodes: HashMap<String, NodeEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, pubkey: &[u8], mode: &str, conn: u64) {
        let entry = self.nodes.entry(name.to_string()).or_insert(NodeEntry {
            name: name.to_string(),
            pubkey: pubkey.to_vec(),
            mode: mode.to_string(),
            last_seen: now(),
            offer: None,
            conn: None,
        });
        entry.pubkey = pubkey.to_vec();
        entry.mode = mode.to_string();
        entry.last_seen = now();
        entry.conn = Some(conn);
    }

    pub fn heartbeat(&mut self, name: &str, conn: u64) {
        if let Some(e) = self.nodes.get_mut(name) {
            e.last_seen = now();
            e.conn = Some(conn);
        }
    }

    pub fn set_offer(&mut self, name: &str, addr: &str, proto: &str, ttl: u64) {
        if let Some(e) = self.nodes.get_mut(name) {
            e.offer = Some(OfferInfo {
                addr: addr.to_string(),
                proto: proto.to_string(),
                expires: now() + ttl,
            });
        }
    }

    pub fn query(&self, name: &str) -> Option<(&NodeEntry, Option<&OfferInfo>)> {
        let e = self.nodes.get(name)?;
        let offer = e.offer.as_ref().filter(|o| o.expires > now());
        Some((e, offer))
    }

    pub fn is_online(&self, name: &str) -> bool {
        self.nodes
            .get(name)
            .map(|e| now().saturating_sub(e.last_seen) < HEARTBEAT_TIMEOUT_SECS)
            .unwrap_or(false)
    }

    pub fn node_conn(&self, name: &str) -> Option<u64> {
        self.nodes.get(name).and_then(|e| e.conn)
    }

    pub fn list(&self) -> Vec<(String, bool)> {
        self.nodes
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    now().saturating_sub(v.last_seen) < HEARTBEAT_TIMEOUT_SECS,
                )
            })
            .collect()
    }

    pub fn disconnect(&mut self, conn: u64) {
        for e in self.nodes.values_mut() {
            if e.conn == Some(conn) {
                e.conn = None;
            }
        }
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
