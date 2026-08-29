// Identity & trust management for the P2P relay protocol.
//
// The collector holds an Ed25519 identity key (signs data/confirmations) and a
// trusted-device list (trusted.toml). Commands are only executed when signed by
// a trusted device; high-risk actions additionally require an admin
// confirmation signature. The relay and web panel hold no keys.
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Where the node's Ed25519 identity key lives (0600, root-owned).
#[cfg(unix)]
pub const IDENTITY_KEY_FILE: &str = "/etc/hyper-node/identity.key";
#[cfg(windows)]
pub const IDENTITY_KEY_FILE: &str = "C:\\ProgramData\\hyper-node\\identity.key";

/// Where the trusted-device list lives.
#[cfg(unix)]
pub const TRUSTED_FILE: &str = "/etc/hyper-node/trusted.toml";
#[cfg(windows)]
pub const TRUSTED_FILE: &str = "C:\\ProgramData\\hyper-node\\trusted.toml";

/// Roles a device can have. Higher tier = more power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Read-only: can subscribe to data, never issue commands.
    Viewer,
    /// Can issue regular + high-risk commands (high-risk needs another admin
    /// confirmation). Can authorize new viewers.
    Admin,
    /// Full control: everything Admin can, plus authorize/remove devices.
    Owner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedDevice {
    pub id: String,
    pub pubkey: String, // base64 Ed25519 public key
    pub role: Role,
    pub added_by: String,
    pub added_at: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TrustedList {
    #[serde(default)]
    pub devices: Vec<TrustedDevice>,
}

/// Load the trusted-device list, or return an empty list if absent.
pub fn load_trusted() -> TrustedList {
    std::fs::read_to_string(TRUSTED_FILE)
        .ok()
        .and_then(|s| toml_parse(&s))
        .unwrap_or_default()
}

fn toml_parse(s: &str) -> Option<TrustedList> {
    // Minimal TOML-ish parser for our flat [[devices]] table.
    let mut list = TrustedList::default();
    let mut cur: Option<TrustedDevice> = None;
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("[[devices]]") {
            if let Some(d) = cur.take() {
                list.devices.push(d);
            }
            cur = Some(TrustedDevice {
                id: String::new(),
                pubkey: String::new(),
                role: Role::Viewer,
                added_by: String::new(),
                added_at: 0,
            });
            continue;
        }
        if let Some(d) = cur.as_mut() {
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim();
                let v = v.trim().trim_matches('"');
                match k {
                    "id" => d.id = v.to_string(),
                    "pubkey" => d.pubkey = v.to_string(),
                    "role" => {
                        d.role = match v {
                            "owner" => Role::Owner,
                            "admin" => Role::Admin,
                            _ => Role::Viewer,
                        }
                    }
                    "added_by" => d.added_by = v.to_string(),
                    "added_at" => d.added_at = v.parse().unwrap_or(0),
                    _ => {}
                }
            }
        }
    }
    if let Some(d) = cur.take() {
        list.devices.push(d);
    }
    Some(list)
}

/// Persist the trusted-device list.
pub fn save_trusted(list: &TrustedList) -> Result<(), String> {
    let mut out = String::new();
    for d in &list.devices {
        out.push_str("[[devices]]\n");
        out.push_str(&format!("id = \"{}\"\n", d.id));
        out.push_str(&format!("pubkey = \"{}\"\n", d.pubkey));
        out.push_str(&format!(
            "role = \"{}\"\n",
            match d.role {
                Role::Owner => "owner",
                Role::Admin => "admin",
                Role::Viewer => "viewer",
            }
        ));
        out.push_str(&format!("added_by = \"{}\"\n", d.added_by));
        out.push_str(&format!("added_at = {}\n", d.added_at));
    }
    crate::atomic_write(TRUSTED_FILE, &out, 0o600)?;
    Ok(())
}

/// Ensure an Ed25519 identity key exists; create it if missing.
/// Returns the verifying key (public key) base64.
pub fn ensure_identity() -> Result<String, String> {
    if Path::new(IDENTITY_KEY_FILE).exists() {
        let bytes = std::fs::read(IDENTITY_KEY_FILE).map_err(|e| e.to_string())?;
        if bytes.len() == 32 {
            let arr: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| "identity.key corrupted")?;
            let sk = SigningKey::from_bytes(&arr);
            return Ok(B64.encode(sk.verifying_key().as_bytes()));
        }
        return Err("identity.key corrupted".to_string());
    }
    let mut csprng = rand::rngs::OsRng;
    let sk = SigningKey::generate(&mut csprng);
    let bytes = sk.to_bytes();
    // Write raw 32-byte key; atomic_write is text-oriented, so use fs::write
    // then fix permissions (0600, root-only).
    std::fs::write(IDENTITY_KEY_FILE, bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(IDENTITY_KEY_FILE, std::fs::Permissions::from_mode(0o600));
    }
    Ok(B64.encode(sk.verifying_key().as_bytes()))
}

/// Sign a message with the node identity key. Returns base64 signature.
pub fn sign_with_identity(msg: &str) -> Result<String, String> {
    let bytes = std::fs::read(IDENTITY_KEY_FILE).map_err(|e| e.to_string())?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "identity.key corrupted")?;
    let sk = SigningKey::from_bytes(&arr);
    let sig = sk.sign(msg.as_bytes());
    Ok(B64.encode(sig.to_bytes()))
}

/// Return the raw 32-byte identity private key, base64-encoded. Used to embed
/// the panel identity into exported configs so other clients (Android) can sign
/// with the same identity and be trusted by nodes that already trust "panel".
pub fn identity_private_b64() -> Result<String, String> {
    let bytes = std::fs::read(IDENTITY_KEY_FILE).map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err("identity.key corrupted".to_string());
    }
    Ok(B64.encode(bytes))
}

/// Sign `msg` with a provided raw 32-byte private key (base64-encoded).
/// Used by clients that import a shared identity (e.g. Android importing the
/// panel's .hsxc config) so they can authenticate as that identity.
pub fn sign_with_private_key(key_b64: &str, msg: &str) -> Result<String, String> {
    let bytes = B64.decode(key_b64).map_err(|e| e.to_string())?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "invalid private key")?;
    let sk = SigningKey::from_bytes(&arr);
    let sig = sk.sign(msg.as_bytes());
    Ok(B64.encode(sig.to_bytes()))
}

/// Derive the public key from a raw 32-byte private key (base64-encoded).
pub fn pubkey_from_private(key_b64: &str) -> Result<String, String> {
    let bytes = B64.decode(key_b64).map_err(|e| e.to_string())?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "invalid private key")?;
    let sk = SigningKey::from_bytes(&arr);
    Ok(B64.encode(sk.verifying_key().as_bytes()))
}

/// Verify a device's Ed25519 signature over `msg`.
pub fn verify_device_signature(pubkey_b64: &str, msg: &str, sig_b64: &str) -> bool {
    let Ok(pk_bytes) = B64.decode(pubkey_b64) else {
        return false;
    };
    let Ok(sig_bytes) = B64.decode(sig_b64) else {
        return false;
    };
    let (Ok(pk_arr), Ok(sig_arr)) = (
        <[u8; 32]>::try_from(pk_bytes.as_slice()),
        <[u8; 64]>::try_from(sig_bytes.as_slice()),
    ) else {
        return false;
    };
    let pk = match VerifyingKey::from_bytes(&pk_arr) {
        Ok(p) => p,
        Err(_) => return false,
    };
    // ed25519-dalek v2: Signature::from_bytes returns Signature directly.
    let sig = Signature::from_bytes(&sig_arr);
    pk.verify(msg.as_bytes(), &sig).is_ok()
}

/// Look up a device by id.
pub fn find_device<'a>(list: &'a TrustedList, id: &str) -> Option<&'a TrustedDevice> {
    list.devices.iter().find(|d| d.id == id)
}

/// Check a device's role for a command.
pub fn can_issue_command(role: Role) -> bool {
    role != Role::Viewer
}

/// Authorize a new device (signed by an existing owner/admin).
/// Rules from the protocol design:
/// - signer must be owner (for owner/admin targets) or admin (viewer only)
/// - an owner can never be removed by another owner (checked in deauthorize)
pub fn authorize_device(
    list: &mut TrustedList,
    signer_id: &str,
    signer_role: Role,
    new_id: &str,
    new_pubkey: &str,
    new_role: Role,
    now: u64,
) -> Result<(), String> {
    if new_id.is_empty() || new_pubkey.is_empty() {
        return Err("device id and pubkey required".into());
    }
    if find_device(list, new_id).is_some() {
        return Err("device already trusted".into());
    }
    // Only owner may add owner/admin; admin may add viewer.
    let allowed = match (signer_role, new_role) {
        (Role::Owner, _) => true,
        (Role::Admin, Role::Viewer) => true,
        _ => return Err("insufficient role to authorize this device".into()),
    };
    if !allowed {
        return Err("insufficient role".into());
    }
    list.devices.push(TrustedDevice {
        id: new_id.to_string(),
        pubkey: new_pubkey.to_string(),
        role: new_role,
        added_by: signer_id.to_string(),
        added_at: now,
    });
    Ok(())
}

/// Remove a device (signed by an owner). An owner cannot remove another owner;
/// at least one owner must remain.
pub fn deauthorize_device(
    list: &mut TrustedList,
    signer_id: &str,
    signer_role: Role,
    target_id: &str,
) -> Result<(), String> {
    if signer_role != Role::Owner {
        return Err("only owner can remove devices".into());
    }
    let target = find_device(list, target_id)
        .ok_or_else(|| "device not found".to_string())?
        .clone();
    if target.role == Role::Owner && target.id != signer_id {
        return Err("owner cannot remove another owner".into());
    }
    // Prevent removing the last owner.
    let owners = list
        .devices
        .iter()
        .filter(|d| d.role == Role::Owner)
        .count();
    if target.role == Role::Owner && owners <= 1 {
        return Err("cannot remove the last owner".into());
    }
    list.devices.retain(|d| d.id != target_id);
    Ok(())
}

/// Unix timestamp (seconds).
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Build a device map for quick lookup.
pub fn device_map(list: &TrustedList) -> HashMap<String, Role> {
    list.devices
        .iter()
        .map(|d| (d.id.clone(), d.role))
        .collect()
}
