// Relay client: how the panel talks to remote collectors through hyper-relay.
//
// Used when a node cannot be reached directly (no listening port, NAT, or
// firewall). The panel asks the node's hyper-relay to wake the local
// collector and return a fresh metrics snapshot.
use futures::StreamExt;
use serde_json::{json, Value};

/// Relay protocol version.
pub const RELAY_PROTOCOL_VERSION: &str = "1";

/// Ask a node's hyper-relay to collect and return the metrics snapshot.
/// `relay_addr` is "host:port" of the relay service on the node machine.
/// All relays serve WSS (TLS) — self-signed certs are accepted.
/// Returns Some((system data, cert fingerprint)) when the relay spawned the
/// collector successfully; the fingerprint (SHA256 hex, uppercase) is captured
/// during the handshake for TOFU pinning.
pub async fn query_direct(relay_addr: &str, node: &str, _tls: bool) -> Option<(Value, String)> {
    let url = format!("wss://{relay_addr}/ws");
    let verifier = crate::relay_tls::CaptureCertVerifier::new();
    let verifier_arc = std::sync::Arc::new(verifier);
    let (ws, _) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        // Accept self-signed certs (relay uses a self-signed cert unless a
        // CA-signed one is configured) and capture the fingerprint.
        let connector = tokio_tungstenite::Connector::Rustls(std::sync::Arc::new(
            rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(verifier_arc.clone())
                .with_no_client_auth(),
        ));
        tokio_tungstenite::connect_async_tls_with_config(&url, None, false, Some(connector)).await
    })
    .await
    .ok()?
    .ok()?;
    let fp = verifier_arc
        .fingerprint
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_default();
    let (mut write, mut read) = ws.split();
    // QUERY the relay: it wakes the local collector and returns the snapshot.
    let msg = json!({
        "type": "query",
        "node": node,
    });
    if futures::SinkExt::send(
        &mut write,
        tokio_tungstenite::tungstenite::Message::Text(msg.to_string()),
    )
    .await
    .is_err()
    {
        return None;
    }
    // Read the response (data or error).
    if let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(t))) =
        futures::StreamExt::next(&mut read).await
    {
        let v: Value = serde_json::from_str(&t).ok()?;
        if v["type"] == "data" {
            return v.get("data").cloned().map(|d| (d, fp));
        }
    }
    None
}

/// Send a control command through the node's hyper-relay. The relay spawns
/// the collector process (`hyper-node control <path>`) and returns the result.
/// Returns Ok(result object) on success.
pub async fn send_command(
    relay_addr: &str,
    node: &str,
    path: &str,
    _tls: bool,
    device_id: &str,
    signature: &str,
) -> Result<Value, String> {
    let url = format!("wss://{relay_addr}/ws");
    let verifier = crate::relay_tls::CaptureCertVerifier::new();
    let verifier_arc = std::sync::Arc::new(verifier);
    let (ws, _) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let connector = tokio_tungstenite::Connector::Rustls(std::sync::Arc::new(
            rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(verifier_arc.clone())
                .with_no_client_auth(),
        ));
        tokio_tungstenite::connect_async_tls_with_config(&url, None, false, Some(connector)).await
    })
    .await
    .map_err(|_| "relay connect timeout".to_string())?
    .map_err(|e| e.to_string())?;
    let (mut write, mut read) = ws.split();
    let msg = json!({
        "type": "cmd",
        "node": node,
        "cmd": path,
        "device_id": device_id,
        "signature": signature,
    });
    futures::SinkExt::send(
        &mut write,
        tokio_tungstenite::tungstenite::Message::Text(msg.to_string()),
    )
    .await
    .map_err(|e| e.to_string())?;
    if let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(t))) =
        futures::StreamExt::next(&mut read).await
    {
        let v: Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
        if v["type"] == "cmd_result" {
            return Ok(v.get("result").cloned().unwrap_or(json!({})));
        }
        return Err(v["msg"].as_str().unwrap_or("relay error").to_string());
    }
    Err("no response from relay".to_string())
}
