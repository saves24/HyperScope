//! Versioned wire DTOs shared by panel, node and reverse-push clients.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: &str = "1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthRequest {
    pub name: String,
    pub key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PushPayload {
    pub protocol_version: String,
    pub name: String,
    pub key: String,
    #[serde(default)]
    pub ts: u64,
    pub data: Option<MetricsData>,
    pub traffic: Option<Value>,
    pub io: Option<Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MetricsData {
    pub node_name: Option<String>,
    pub version: Option<String>,
    pub cpu: Option<f64>,
    pub mem_percent: Option<f64>,
    pub disk_percent: Option<f64>,
    pub cpu_temp_raw: Option<f64>,
    pub uptime: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct NodeMetrics {
    #[serde(flatten)]
    pub data: MetricsData,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HistoryPoint {
    pub ts: u64,
    pub avg: f64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HistoryResponse {
    pub points: Vec<HistoryPoint>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DockerContainer {
    pub name: String,
    pub image: Option<String>,
    pub state: Option<String>,
    pub ports: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DockerResponse {
    pub containers: Vec<DockerContainer>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PanelEvent {
    pub time: Option<String>,
    pub kind: Option<String>,
    pub node: Option<String>,
    pub msg: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EventListResponse {
    pub events: Vec<PanelEvent>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub owner: Option<String>,
    pub tls: bool,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PanelLoginRequest {
    pub user: String,
    pub pass: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PanelLoginResponse {
    pub ok: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NodeListResponse {
    pub nodes: Vec<NodeInfo>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OperationResponse {
    pub ok: bool,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{AuthRequest, MetricsData, PushPayload, PROTOCOL_VERSION};

    #[test]
    fn push_payload_roundtrips_with_protocol_version() {
        let payload = PushPayload {
            protocol_version: PROTOCOL_VERSION.to_string(),
            name: "node-a".to_string(),
            key: "secret".to_string(),
            ts: 42,
            data: Some(MetricsData {
                cpu: Some(12.5),
                ..MetricsData::default()
            }),
            traffic: None,
            io: None,
        };
        let value = serde_json::to_value(&payload).unwrap();
        let decoded: PushPayload = serde_json::from_value(value).unwrap();
        assert_eq!(decoded.protocol_version, PROTOCOL_VERSION);
        assert_eq!(decoded.data.and_then(|data| data.cpu), Some(12.5));
    }

    #[test]
    fn auth_request_requires_name_and_key() {
        let request: AuthRequest =
            serde_json::from_str(r#"{"name":"node-a","key":"secret"}"#).unwrap();
        assert_eq!(request.name, "node-a");
        assert_eq!(request.key, "secret");
    }
}
