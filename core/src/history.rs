// Historical metrics persistence (SQLite) + aggregate trend queries
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
const HISTORY_DB: &str = "/etc/hyper-panel/history.db";
#[cfg(target_os = "windows")]
const HISTORY_DB: &str = "C:\\ProgramData\\hyper-panel\\history.db";
const HISTORY_RETENTION_DAYS: i64 = 90;

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// Open DB (create schema if needed)
pub fn history_open() -> Result<Connection, String> {
    let conn = Connection::open(HISTORY_DB).map_err(|e| format!("history db open: {e}"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS metrics (
            ts INTEGER NOT NULL,
            node TEXT NOT NULL,
            metric TEXT NOT NULL,
            value REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_metrics ON metrics(node, metric, ts);
        ",
    )
    .map_err(|e| format!("history db schema: {e}"))?;
    Ok(conn)
}

// One snapshot of per-node metrics (NaN = unavailable, skipped on write)
pub struct NodeSnapshot<'a> {
    pub node: &'a str,
    pub cpu: f64,
    pub mem: f64,
    pub disk: f64,
    pub rx_mbs: f64,
    pub tx_mbs: f64,
    pub tcp: f64,
    pub temp: f64,
}

// Write a snapshot row per metric for one node
pub fn history_record(s: NodeSnapshot) -> Result<(), String> {
    let ts = now_ts();
    let conn = history_open()?;
    let metrics = [
        ("cpu", s.cpu),
        ("mem", s.mem),
        ("disk", s.disk),
        ("net_rx", s.rx_mbs),
        ("net_tx", s.tx_mbs),
        ("tcp", s.tcp),
        ("temp", s.temp),
    ];
    for (name, val) in metrics {
        if val.is_finite() {
            conn.execute(
                "INSERT INTO metrics (ts, node, metric, value) VALUES (?1, ?2, ?3, ?4)",
                params![ts, s.node, name, val],
            )
            .map_err(|e| format!("history insert failed: {e}"))?;
        }
    }
    // Periodic cleanup (cheap: once per write, deletes expired rows)
    let cutoff = ts - HISTORY_RETENTION_DAYS * 86400;
    conn.execute("DELETE FROM metrics WHERE ts < ?1", params![cutoff])
        .map_err(|e| format!("history cleanup failed: {e}"))?;
    Ok(())
}

// Aggregate query: range = 1h|24h|7d|30d; returns points with avg/max/min per bucket
pub fn history_query(node: &str, metric: &str, range: &str) -> Value {
    history_query_range(node, metric, range, 0, 0)
}

/// Query with optional explicit window (start/end unix seconds; both 0 =
/// derive from range). Used by the compare overlay to fetch the previous
/// equal-length window.
pub fn history_query_range(
    node: &str,
    metric: &str,
    range: &str,
    start_override: i64,
    end_override: i64,
) -> Value {
    let seconds: i64 = match range {
        "1h" => 3600,
        "24h" => 86400,
        "7d" => 7 * 86400,
        "30d" => 30 * 86400,
        _ => 86400,
    };
    // bucket size: 1h -> 1min, 24h -> 15min, 7d -> 2h, 30d -> 8h
    let bucket: i64 = match range {
        "1h" => 60,
        "24h" => 900,
        "7d" => 7200,
        "30d" => 28800,
        _ => 900,
    };
    let now = now_ts();
    let start = if start_override > 0 {
        start_override
    } else {
        now - seconds
    };
    let end = if end_override > 0 { end_override } else { now };
    let conn = match history_open() {
        Ok(c) => c,
        Err(_) => return json!({"points": [], "error": "db unavailable"}),
    };
    let mut stmt = match conn.prepare(
        "SELECT (ts / ?3) * ?3 AS bucket, AVG(value), MAX(value), MIN(value)
         FROM metrics WHERE node = ?1 AND metric = ?2 AND ts >= ?4 AND ts <= ?5
         GROUP BY bucket ORDER BY bucket",
    ) {
        Ok(s) => s,
        Err(e) => return json!({"points": [], "error": e.to_string()}),
    };
    let rows = stmt
        .query_map(params![node, metric, bucket, start, end], |row| {
            Ok(json!({
                "ts": row.get::<_, i64>(0)?,
                "avg": row.get::<_, f64>(1)?,
                "max": row.get::<_, f64>(2)?,
                "min": row.get::<_, f64>(3)?,
            }))
        })
        .map(|it| it.filter_map(|r| r.ok()).collect::<Vec<_>>())
        .unwrap_or_default();
    json!({"metric": metric, "range": range, "points": rows})
}

// Export query: node metric points as CSV text (ts,value rows)
pub fn history_export(node: &str, metric: &str, range: &str) -> String {
    let seconds: i64 = match range {
        "1h" => 3600,
        "24h" => 86400,
        "7d" => 7 * 86400,
        "30d" => 30 * 86400,
        _ => 86400,
    };
    let start = now_ts() - seconds;
    let conn = match history_open() {
        Ok(c) => c,
        Err(_) => return "ts,value\n".to_string(),
    };
    let mut out = String::from("ts,value\n");
    if let Ok(mut stmt) = conn.prepare(
        "SELECT ts, value FROM metrics WHERE node = ?1 AND metric = ?2 AND ts >= ?3 ORDER BY ts",
    ) {
        if let Ok(rows) = stmt.query_map(params![node, metric, start], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        }) {
            for r in rows.flatten() {
                out.push_str(&format!("{},{:.3}\n", r.0, r.1));
            }
        }
    }
    out
}
