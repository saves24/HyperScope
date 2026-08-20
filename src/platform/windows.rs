// Windows platform implementation (sysinfo + WMI/netstat)
// Same function signatures as linux.rs; callers use platform::xxx.
use serde_json::{json, Value};
use std::time::Duration;
use sysinfo::{Disks, Networks, System};

// Per-key TTL cache to avoid spawning PowerShell / full sysinfo refreshes on every poll.
// The panel polls system+io+traffic every 5s; without caching Windows spins up several
// child processes each cycle (high CPU -> laptop fans spin up).
// HashMap keyed by metric: cpu/io/wifi caches do NOT evict each other.
static CACHE: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, Value)>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
fn cached(key: &str, ttl_secs: u64, f: impl FnOnce() -> Value) -> Value {
    let mut c = CACHE.lock().unwrap();
    if let Some((t, v)) = c.get(key) {
        if t.elapsed() < std::time::Duration::from_secs(ttl_secs) {
            return v.clone();
        }
    }
    let v = f();
    c.insert(key.to_string(), (std::time::Instant::now(), v.clone()));
    v
}

// Shared sysinfo System: refreshed once per get_system (snapshot), read by get_memory/get_cpu_info/get_disk etc.
static SYSTEM: std::sync::Mutex<Option<System>> = std::sync::Mutex::new(None);

pub(crate) fn refresh_system() {
    let mut s = SYSTEM.lock().unwrap();
    if s.is_none() {
        *s = Some(System::new());
    }
    let sys = s.as_mut().unwrap();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
}

// ---------- temp (WMI; may be unavailable on some laptops) ----------
pub(crate) fn get_cpu_temp() -> Option<f64> {
    // PowerShell WMI query; returns Celsius if the hardware exposes a thermal zone
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile", "-Command",
            "(Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature).CurrentTemperature",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    s.parse::<f64>().ok().map(|v| (v / 10.0) - 273.15)
}
pub(crate) fn get_gpu_temp() -> Option<f64> {
    // NVIDIA GPUs via nvidia-smi (e.g. RTX 3050); others return None (N/A)
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout);
    s.lines().next()?.trim().parse::<f64>().ok()
}

// ---------- cpu ----------
pub(crate) fn get_cpu_usage() -> Option<f64> {
    // Cached 3s: system poll hits it between the 5s panel cycles
    let v = cached("cpu", 3, || {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        std::thread::sleep(Duration::from_millis(120));
        sys.refresh_cpu_usage();
        let cpus = sys.cpus();
        let pct = if cpus.is_empty() {
            0.0
        } else {
            cpus.iter().map(|c| c.cpu_usage() as f64).sum::<f64>() / cpus.len() as f64
        };
        json!(pct)
    });
    v.as_f64()
}

pub(crate) fn get_cpu_info() -> Value {
    let mut s = SYSTEM.lock().unwrap();
    if s.is_none() {
        *s = Some(System::new());
    }
    let sys = s.as_mut().unwrap();
    sys.refresh_cpu_usage();
    let cpus = sys.cpus();
    let cores = cpus.len().max(1);
    let mhz = cpus
        .first()
        .map(|c| format!("{:.0}", c.frequency()))
        .unwrap_or_default();
    json!({ "cores": cores, "mhz": mhz })
}

// ---------- memory ----------
pub(crate) fn get_memory() -> Value {
    let mut s = SYSTEM.lock().unwrap();
    if s.is_none() {
        *s = Some(System::new());
    }
    let sys = s.as_mut().unwrap();
    sys.refresh_memory();
    let total = sys.total_memory();
    let avail = sys.available_memory();
    let used = total.saturating_sub(avail);
    let percent = if total > 0 {
        (used as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };
    json!({
        "total_gb": format!("{:.1}", total as f64 / 1024.0 / 1024.0 / 1024.0),
        "used_gb": format!("{:.1}", used as f64 / 1024.0 / 1024.0 / 1024.0),
        "percent": percent,
    })
}

// ---------- disk ----------
pub(crate) fn get_disk() -> Value {
    let disks = Disks::new_with_refreshed_list();
    let mut total = 0u64;
    let mut used = 0u64;
    for d in disks.list() {
        total += d.total_space();
        used += d.total_space().saturating_sub(d.available_space());
    }
    if total == 0 {
        return json!({"error": "no disks"});
    }
    let percent = (used as f64 / total as f64 * 100.0).round();
    json!({
        "total_gb": format!("{:.1}", total as f64 / 1024.0 / 1024.0 / 1024.0),
        "used_gb": format!("{:.1}", used as f64 / 1024.0 / 1024.0 / 1024.0),
        "percent": percent,
        "fs_type": "ntfs",
    })
}

pub(crate) fn get_disks() -> Vec<Value> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|d| {
            let total = d.total_space();
            let avail = d.available_space();
            let used = total.saturating_sub(avail);
            let pct = if total > 0 {
                (used as f64 / total as f64 * 100.0).round()
            } else {
                0.0
            };
            json!({
                "mount": d.mount_point().to_string_lossy().to_string(),
                "fs_type": d.file_system().to_string_lossy().to_string(),
                "total_gb": format!("{:.1}", total as f64 / 1024.0 / 1024.0 / 1024.0),
                "used_gb": format!("{:.1}", used as f64 / 1024.0 / 1024.0 / 1024.0),
                "percent": pct,
            })
        })
        .collect()
}

// ---------- load / uptime / kernel ----------
pub(crate) fn get_loadavg() -> String {
    // Windows has no Unix load average; return empty so the panel shows "--" instead of fake 0.00
    String::new()
}

pub(crate) fn get_uptime() -> String {
    let secs = System::uptime();
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    format!("{days}d {hours}h {mins}m")
}

pub(crate) fn get_kernel_version() -> String {
    // Real OS version e.g. "Windows 11 (build 26100.9168)" via PowerShell
    let script = "$v=[System.Environment]::OSVersion.Version; \"Windows {0} (build {1})\" -f $v.Major,$v.Build";
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Windows".to_string())
}

pub(crate) fn get_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

// ---------- network ----------
pub(crate) fn get_traffic(iface_override: Option<&str>) -> Value {
    let iface = iface_override.unwrap_or("").to_string();
    let mut networks = Networks::new_with_refreshed_list();
    let target = if iface.is_empty() {
        networks.keys().next().cloned().unwrap_or_default()
    } else {
        iface.clone()
    };
    let rx1 = networks.get(&target).map(|n| n.received()).unwrap_or(0);
    let tx1 = networks.get(&target).map(|n| n.transmitted()).unwrap_or(0);
    std::thread::sleep(Duration::from_millis(200));
    networks.refresh(true);
    let rx2 = networks.get(&target).map(|n| n.received()).unwrap_or(0);
    let tx2 = networks.get(&target).map(|n| n.transmitted()).unwrap_or(0);
    let dt = 0.2;
    let rx = (rx2.saturating_sub(rx1)) as f64 / dt;
    let tx = (tx2.saturating_sub(tx1)) as f64 / dt;
    // Per-interface cumulative totals (aligned with linux)
    let ifaces: Vec<Value> = networks
        .iter()
        .map(|(name, n)| {
            json!({
                "name": name,
                "total_rx": n.received(),
                "total_tx": n.transmitted(),
            })
        })
        .collect();
    json!({
        "iface": target,
        "speed_rx": rx,
        "speed_tx": tx,
        "total_rx": rx2,
        "total_tx": tx2,
        "speed_rx_str": format_speed(rx),
        "speed_tx_str": format_speed(tx),
        "ifaces": ifaces,
        "rx_mbs": rx / 1048576.0,
        "tx_mbs": tx / 1048576.0,
    })
}

fn format_speed(bps: f64) -> String {
    if bps >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bps / 1024.0 / 1024.0)
    } else if bps >= 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.0} B/s", bps)
    }
}

// ---------- processes ----------
pub(crate) fn get_processes(sort: &str, limit: usize, name: &str) -> Vec<Value> {
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let mut procs: Vec<Value> = sys
        .processes()
        .iter()
        .map(|(pid, p)| {
            json!({
                "pid": pid.as_u32(),
                "name": p.name().to_string_lossy().to_string(),
                "cpu": p.cpu_usage(),
                "rss_mb": format!("{:.1}", p.memory() as f64 / 1024.0),
            })
        })
        .collect();
    match sort {
        "cpu" => procs.sort_by(|a, b| {
            b["cpu"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&a["cpu"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        _ => procs.sort_by(|a, b| {
            b["rss_mb"]
                .as_f64()
                .unwrap_or(0.0)
                .partial_cmp(&a["rss_mb"].as_f64().unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
    if !name.is_empty() {
        procs.retain(|p| {
            p["name"]
                .as_str()
                .unwrap_or("")
                .to_lowercase()
                .contains(&name.to_lowercase())
        });
    }
    procs.truncate(limit.max(1));
    procs
}

pub(crate) fn count_processes() -> u64 {
    let mut sys = System::new_all();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, false);
    sys.processes().len() as u64
}

// ---------- io (disk read/write + tcp) ----------
pub(crate) fn get_io() -> Value {
    // Cached 5s: panel polls /io every 5s; avoids spawning PowerShell each time
    cached("io", 5, || {
        let (read_bps, write_bps) = disk_io_counters();
        let tcp = tcp_connections();
        json!({
            "disk_read_mbs": (read_bps / 1048576.0 * 100.0).round() / 100.0,
            "disk_write_mbs": (write_bps / 1048576.0 * 100.0).round() / 100.0,
            "tcp_conns": tcp,
        })
    })
}

fn disk_io_counters() -> (f64, f64) {
    // Single PowerShell call returns both read and write counters (one process, not two)
    let script = "$r=(Get-Counter '\\PhysicalDisk(_Total)\\Disk Read Bytes/sec').CounterSamples[0].CookedValue; $w=(Get-Counter '\\PhysicalDisk(_Total)\\Disk Write Bytes/sec').CounterSamples[0].CookedValue; \"$r|$w\"";
    match std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()
    {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            let parts: Vec<&str> = s.split('|').collect();
            let read = parts
                .first()
                .and_then(|v| v.trim().parse::<f64>().ok())
                .unwrap_or(0.0);
            let write = parts
                .get(1)
                .and_then(|v| v.trim().parse::<f64>().ok())
                .unwrap_or(0.0);
            (read, write)
        }
        Err(_) => (0.0, 0.0),
    }
}

fn tcp_connections() -> f64 {
    let out = std::process::Command::new("netstat")
        .args(["-an"])
        .output()
        .ok();
    match out {
        Some(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            s.lines().filter(|l| l.contains("ESTABLISHED")).count() as f64
        }
        None => f64::NAN,
    }
}

// ---------- docker (via docker CLI when Docker Desktop is installed) ----------
fn docker_available() -> bool {
    std::process::Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub(crate) fn get_docker_containers() -> Value {
    if !docker_available() {
        return json!({"error": "docker not available"});
    }
    let out = std::process::Command::new("docker")
        .args([
            "ps",
            "-a",
            "--format",
            "{{.Names}}|{{.Image}}|{{.State}}|{{.Ports}}",
        ])
        .output();
    match out {
        Ok(o) => {
            let mut list = vec![];
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let p: Vec<&str> = line.split('|').collect();
                if p.len() >= 3 {
                    list.push(json!({
                        "name": p[0], "image": p[1], "state": p[2],
                        "ports": p.get(3).copied().unwrap_or(""),
                    }));
                }
            }
            json!({"containers": list})
        }
        Err(e) => json!({"error": format!("docker failed: {e}")}),
    }
}

pub(crate) fn docker_action(name: &str, action: &str) -> Value {
    if !docker_available() {
        return json!({"error": "docker not available"});
    }
    let out = std::process::Command::new("docker")
        .args([action, name])
        .output();
    match out {
        Ok(o) => {
            json!({"ok": o.status.success(), "output": String::from_utf8_lossy(&o.stdout).trim().to_string()})
        }
        Err(e) => json!({"ok": false, "error": e.to_string()}),
    }
}

// ---------- listening ports (netstat) ----------
pub(crate) fn get_ports() -> Vec<Value> {
    let out = std::process::Command::new("netstat")
        .args(["-ano"])
        .output();
    match out {
        Ok(o) => {
            let mut ports: Vec<Value> = vec![];
            let mut seen = std::collections::HashSet::new();
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4
                    && (parts[0] == "TCP" || parts[0] == "UDP")
                    && parts[1].contains(':')
                {
                    let state = if parts.len() >= 4 { parts[3] } else { "" };
                    if state == "LISTENING" || parts[0] == "UDP" {
                        let local = parts[1]
                            .rsplit_once(':')
                            .map(|(h, p)| (h.to_string(), p.to_string()))
                            .unwrap_or((parts[1].to_string(), String::new()));
                        let key = format!("{}:{}", parts[0], local.1);
                        if seen.insert(key) {
                            ports.push(json!({
                                "proto": parts[0],
                                "local_addr": local.0,
                                "port": local.1,
                                "pid": parts.last().copied().unwrap_or(""),
                            }));
                        }
                    }
                }
            }
            ports
        }
        Err(_) => vec![],
    }
}

// ---------- wifi signal (laptop) ----------
pub(crate) fn get_wifi_signal() -> Value {
    // Cached 10s: adapter status changes rarely
    cached("wifi", 10, || {
        // netsh works under SYSTEM/service context; parse real SSID + signal.
        // On failure return nulls (never fabricate an adapter name as SSID).
        match std::process::Command::new("netsh")
            .args(["wlan", "show", "interfaces"])
            .output()
        {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout);
                let mut ssid = String::new();
                let mut signal = String::new();
                for line in s.lines() {
                    let t = line.trim();
                    let lower = t.to_lowercase();
                    // SSID stays "SSID" in most locales; signal is localized (Signal / 信号)
                    if (lower.starts_with("ssid") || lower.starts_with("网络名称"))
                        && t.contains(':')
                        && ssid.is_empty()
                    {
                        ssid = t.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
                    }
                    if (lower.starts_with("signal") || lower.starts_with("信号"))
                        && t.contains(':')
                        && signal.is_empty()
                    {
                        signal = t.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();
                    }
                }
                if !ssid.is_empty() {
                    json!({"ssid": ssid, "signal": signal, "connected": true})
                } else {
                    json!({"ssid": null, "signal": null, "connected": false})
                }
            }
            Err(_) => json!({"ssid": null, "signal": null, "connected": false}),
        }
    })
}

// ---------- recent system events (Windows Event Log) ----------
pub(crate) fn get_event_log(limit: usize) -> Vec<Value> {
    let script = format!(
        "Get-WinEvent -FilterHashtable @{{LogName='System'; StartTime=(Get-Date).AddHours(-24)}} -MaxEvents {limit} | Select-Object @{{n='time';e={{$_.TimeCreated.ToString(\"yyyy-MM-dd HH:mm:ss\")}}}},Id,ProviderName,LevelDisplayName,Message | ConvertTo-Json -Compress"
    );
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output();
    match out {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            let v: Value = serde_json::from_str(s.trim()).unwrap_or(json!([]));
            match v {
                Value::Array(items) => items.into_iter().map(|it| json!({
                    "time": it["time"].as_str().unwrap_or(""),
                    "id": it["Id"].as_u64().unwrap_or(0),
                    "level": it["LevelDisplayName"].as_str().unwrap_or(""),
                    "source": it["ProviderName"].as_str().unwrap_or(""),
                    "message": it["Message"].as_str().unwrap_or("").chars().take(200).collect::<String>(),
                })).collect(),
                _ => vec![],
            }
        }
        Err(_) => vec![],
    }
}
