use crate::{json, read_file, Value, HOSTNAME_FILE, NET_SAMPLE_MS};
// Linux platform implementation (reads /proc and /sys)
use crate::metrics::PROC_SAMPLE;
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};

pub(crate) fn get_cpu_temp() -> Option<f64> {
    read_file("/sys/class/thermal/thermal_zone0/temp")
        .ok()
        .and_then(|s| s.trim().parse::<f64>().ok().map(|v| v / 1000.0))
}
pub(crate) fn get_gpu_temp() -> Option<f64> {
    // Prefer NVIDIA: nvidia-smi reports GPU temperature directly
    if let Ok(out) = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=temperature.gpu", "--format=csv,noheader"])
        .output()
    {
        if out.status.success() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                if let Some(v) = s
                    .split_whitespace()
                    .next()
                    .and_then(|x| x.parse::<f64>().ok())
                {
                    return Some(v);
                }
            }
        }
    }
    // AMD/Intel: probe thermal_zone or hwmon (type contains gpu/video/vc)
    let dir = "/sys/class/thermal";
    let entries = fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let base = e.path();
        let typ = fs::read_to_string(base.join("type")).unwrap_or_default();
        let t = typ.to_lowercase();
        if t.contains("gpu") || t.contains("video") || t.contains("vc") {
            let temp = fs::read_to_string(base.join("temp")).ok()?;
            return temp.trim().parse::<f64>().ok().map(|v| v / 1000.0);
        }
    }
    // AMD: /sys/class/drm/card*/device/hwmon/hwmon*/temp1_input
    if let Ok(cards) = fs::read_dir("/sys/class/drm") {
        for c in cards.flatten() {
            let hw = c.path().join("device/hwmon");
            if let Ok(hwmons) = fs::read_dir(hw) {
                for h in hwmons.flatten() {
                    let temp = fs::read_to_string(h.path().join("temp1_input")).ok()?;
                    let v = temp.trim().parse::<f64>().ok()?;
                    if v > 0.0 {
                        return Some(v / 1000.0);
                    }
                }
            }
        }
    }
    None
}
pub(crate) fn read_cpu_stat() -> Option<(u64, u64)> {
    let content = read_file("/proc/stat").ok()?;
    let line = content.lines().find(|l| l.starts_with("cpu "))?;
    let nums: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|x| x.parse().ok())
        .collect();
    if nums.len() < 4 {
        return None;
    }
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total: u64 = nums.iter().sum();
    Some((total, idle))
}
pub(crate) fn get_cpu_usage() -> Option<f64> {
    let (t1, i1) = read_cpu_stat()?;
    std::thread::sleep(Duration::from_millis(200));
    let (t2, i2) = read_cpu_stat()?;
    let d_total = t2.saturating_sub(t1);
    let d_idle = i2.saturating_sub(i1);
    if d_total == 0 {
        return Some(0.0);
    }
    Some(100.0 * (d_total - d_idle) as f64 / d_total as f64)
}
pub(crate) fn get_loadavg() -> String {
    read_file("/proc/loadavg")
        .map(|s| s.split_whitespace().take(3).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}
pub(crate) fn get_uptime() -> String {
    let s = read_file("/proc/uptime").unwrap_or_default();
    // /proc/uptime first field is float seconds (machine uptime, e.g. "457747.14 ...")
    // parse as f64 then truncate to u64 (u64 parse fails on the decimal point)
    let secs = s
        .split_whitespace()
        .next()
        .and_then(|x| x.parse::<f64>().ok())
        .map(|x| x as u64)
        .unwrap_or(0);
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}
pub(crate) fn get_kernel_version() -> String {
    read_file("/proc/version")
        .map(|s| {
            // "Linux version 6.8.0-1060-raspi (buildd@...) (gcc ...) #1 SMP ..."
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() >= 3 {
                format!("{} {}", parts[0], parts[2])
            } else {
                s.trim().to_string()
            }
        })
        .unwrap_or_default()
}
pub(crate) fn get_cpu_info() -> Value {
    let content = read_file("/proc/cpuinfo").unwrap_or_default();
    let cores = content
        .lines()
        .filter(|l| l.starts_with("processor"))
        .count();
    let mut mhz = String::new();
    for l in content.lines() {
        if l.starts_with("cpu MHz") || l.starts_with("BogoMIPS") {
            mhz = l.split(':').nth(1).unwrap_or("").trim().to_string();
            break;
        }
    }
    json!({ "cores": cores, "mhz": mhz })
}
pub(crate) fn get_memory() -> Value {
    let content = read_file("/proc/meminfo").unwrap_or_default();
    let get = |key: &str| -> u64 {
        content
            .lines()
            .find(|l| l.starts_with(key))
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|x| x.parse().ok())
            .unwrap_or(0)
    };
    let total = get("MemTotal:");
    let avail = get("MemAvailable:");
    let used = total.saturating_sub(avail);
    let percent = if total > 0 {
        (used as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };
    json!({
        "total": total,
        "used": used,
        "available": avail,
        "percent": percent,
        "total_gb": format!("{:.1}", total as f64 / 1024.0 / 1024.0),
        "used_gb": format!("{:.1}", used as f64 / 1024.0 / 1024.0),
    })
}
pub(crate) fn statvfs_at(path: &str) -> Option<(u64, u64, f64, u64, u64, u64)> {
    let mut vfs: libc::statvfs = unsafe { std::mem::zeroed() };
    let c_path = std::ffi::CString::new(path).ok()?;
    let ok = unsafe { libc::statvfs(c_path.as_ptr(), &mut vfs) };
    if ok != 0 {
        return None;
    }
    let total = vfs.f_blocks as u64 * vfs.f_frsize as u64;
    let avail = vfs.f_bavail as u64 * vfs.f_frsize as u64;
    let used = total.saturating_sub(avail);
    let percent = if total > 0 {
        (used as f64 / total as f64 * 100.0).round()
    } else {
        0.0
    };
    // inode: total/free/used
    let inodes_total = vfs.f_files as u64;
    let inodes_free = vfs.f_ffree as u64;
    let inodes_used = inodes_total.saturating_sub(inodes_free);
    Some((total, used, percent, inodes_total, inodes_used, inodes_free))
}
pub(crate) fn get_disk() -> Value {
    let Some((total, used, percent, _, _, _)) = statvfs_at("/") else {
        return json!({"error": "statvfs failed"});
    };
    json!({
        "total": total,
        "used": used,
        "avail": total.saturating_sub(used),
        "percent": percent,
        "total_gb": format!("{:.1}", total as f64 / 1024.0 / 1024.0 / 1024.0),
        "used_gb": format!("{:.1}", used as f64 / 1024.0 / 1024.0 / 1024.0),
    })
}
pub(crate) fn get_disks() -> Vec<Value> {
    let mut mounts: Vec<(String, String)> = Vec::new();
    if let Ok(content) = read_file("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 {
                continue;
            }
            let fs_type = parts[2];
            if fs_type == "ext4"
                || fs_type == "ext3"
                || fs_type == "xfs"
                || fs_type == "btrfs"
                || fs_type == "vfat"
            {
                let mp = parts[1];
                if mp.starts_with('/') && mp != "/proc" && mp != "/sys" && mp != "/dev" {
                    mounts.push((mp.to_string(), fs_type.to_string()));
                }
            }
        }
    }
    mounts.sort();
    mounts.dedup();

    let mut result = Vec::new();
    for (mp, fs_type) in &mounts {
        if let Some((total, used, percent, inodes_total, inodes_used, inodes_free)) = statvfs_at(mp)
        {
            let name = if mp == "/" {
                "root".to_string()
            } else {
                mp.trim_start_matches('/').replace('/', "-")
            };
            let free = total.saturating_sub(used);
            let inode_pct = if inodes_total > 0 {
                (inodes_used as f64 / inodes_total as f64 * 100.0).round()
            } else {
                0.0
            };
            result.push(json!({
                "mount": mp,
                "name": name,
                "fs_type": fs_type,
                "percent": percent,
                "total_gb": format!("{:.1}", total as f64 / 1024.0 / 1024.0 / 1024.0),
                "used_gb": format!("{:.1}", used as f64 / 1024.0 / 1024.0 / 1024.0),
                "free_gb": format!("{:.1}", free as f64 / 1024.0 / 1024.0 / 1024.0),
                "inodes_total": inodes_total,
                "inodes_used": inodes_used,
                "inodes_free": inodes_free,
                "inode_pct": inode_pct,
            }));
        }
    }
    result
}
pub(crate) fn get_default_iface() -> String {
    if let Ok(content) = read_file("/proc/net/route") {
        for line in content.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[1] == "00000000" && parts[3] != "00000000" {
                return parts[0].to_string();
            }
        }
    }
    "eth0".to_string()
}
pub(crate) fn read_iface_bytes(iface: &str) -> Option<(u64, u64)> {
    let content = read_file("/proc/net/dev").ok()?;
    for line in content.lines().skip(2) {
        if line.trim_start().starts_with(&format!("{iface}:")) {
            let values = line
                .split(':')
                .nth(1)
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>();
            let rx = values
                .first()
                .and_then(|x| x.parse::<u64>().ok())
                .unwrap_or(0);
            let tx = values
                .get(8)
                .and_then(|x| x.parse::<u64>().ok())
                .unwrap_or(0);
            return Some((rx, tx));
        }
    }
    None
}
pub(crate) fn list_ifaces() -> Vec<String> {
    let mut result = Vec::new();
    if let Ok(content) = read_file("/proc/net/dev") {
        for line in content.lines().skip(2) {
            let name = line.split(':').next().unwrap_or("").trim().to_string();
            if name.is_empty() || name == "lo" {
                continue;
            }
            if name.starts_with("docker")
                || name.starts_with("veth")
                || name.starts_with("br-")
                || name.starts_with("tailscale")
                || name.starts_with("virbr")
            {
                continue;
            }
            result.push(name);
        }
    }
    result
}
pub(crate) fn format_speed(bps: f64) -> String {
    if bps >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bps / 1024.0 / 1024.0)
    } else if bps >= 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else {
        format!("{:.0} B/s", bps)
    }
}
pub(crate) fn get_traffic(iface_override: Option<&str>) -> Value {
    let iface = iface_override
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(get_default_iface);
    let Some((rx1, tx1)) = read_iface_bytes(&iface) else {
        return json!({"error": format!("cannot read network interface {iface}")});
    };
    std::thread::sleep(Duration::from_millis(NET_SAMPLE_MS));
    let Some((rx2, tx2)) = read_iface_bytes(&iface) else {
        return json!({"error": format!("cannot read network interface {iface}")});
    };
    let dt = NET_SAMPLE_MS as f64 / 1000.0;
    let speed_rx = (rx2.saturating_sub(rx1)) as f64 / dt;
    let speed_tx = (tx2.saturating_sub(tx1)) as f64 / dt;

    let mut ifaces = Vec::new();
    for name in list_ifaces() {
        if let Some((r, t)) = read_iface_bytes(&name) {
            ifaces.push(json!({ "name": name, "total_rx": r, "total_tx": t }));
        }
    }

    json!({
        "iface": iface,
        "speed_rx": speed_rx,
        "speed_tx": speed_tx,
        "total_rx": rx2,
        "total_tx": tx2,
        "speed_rx_str": format_speed(speed_rx),
        "speed_tx_str": format_speed(speed_tx),
        "ifaces": ifaces,
    })
}
pub(crate) fn get_processes(sort: &str, limit: usize, name_filter: &str) -> Vec<Value> {
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64;

    let mut procs = Vec::new();
    let Ok(dir) = fs::read_dir("/proc") else {
        return vec![];
    };
    for e in dir.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        let Ok(stat) = read_file(&format!("/proc/{pid}/stat")) else {
            continue;
        };
        let Some(close) = stat.rfind(')') else {
            continue;
        };
        let after = &stat[close + 1..];
        let fields: Vec<&str> = after.split_whitespace().collect();
        let state = fields.first().copied().unwrap_or("?");
        let utime: u64 = fields.get(11).and_then(|x| x.parse().ok()).unwrap_or(0);
        let stime: u64 = fields.get(12).and_then(|x| x.parse().ok()).unwrap_or(0);
        let rss_pages: u64 = fields.get(21).and_then(|x| x.parse().ok()).unwrap_or(0);
        let rss_kb = rss_pages * page_size / 1024;
        let comm = stat[..close]
            .rsplit_once('(')
            .map(|(_, c)| c.to_string())
            .unwrap_or_default();
        procs.push((pid, comm, state.to_string(), utime + stime, rss_kb));
    }

    let total_cpu = read_cpu_stat().map(|(t, _)| t).unwrap_or(0);
    let mut sample = PROC_SAMPLE.lock().unwrap();
    let cpu_map: HashMap<u32, f64> = match &*sample {
        Some((last_time, last_total, last_map)) => {
            let dt = last_time.elapsed().as_secs_f64();
            let d_total = total_cpu.saturating_sub(*last_total) as f64;
            if dt > 0.3 && d_total > 0.0 {
                procs
                    .iter()
                    .map(|(pid, _, _, ticks, _)| {
                        let d_proc =
                            ticks.saturating_sub(last_map.get(pid).copied().unwrap_or(0)) as f64;
                        (*pid, d_proc / d_total * 100.0)
                    })
                    .collect()
            } else {
                HashMap::new()
            }
        }
        None => HashMap::new(),
    };
    let new_map: HashMap<u32, u64> = procs
        .iter()
        .map(|(pid, _, _, ticks, _)| (*pid, *ticks))
        .collect();
    *sample = Some((Instant::now(), total_cpu, new_map));

    let ncpu = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let cpu_pct_of = |pid: u32| -> f64 { cpu_map.get(&pid).copied().unwrap_or(0.0) * ncpu as f64 };

    let name_filter = name_filter.to_lowercase();
    let filtered: Vec<_> = if name_filter.is_empty() {
        procs.into_iter().collect()
    } else {
        procs
            .into_iter()
            .filter(|(_, comm, _, _, _)| comm.to_lowercase().contains(&name_filter))
            .collect()
    };

    let mut sorted = filtered;
    if sort == "cpu" {
        sorted.sort_by(|a, b| {
            cpu_pct_of(b.0)
                .partial_cmp(&cpu_pct_of(a.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        sorted.sort_by_key(|a| std::cmp::Reverse(a.4));
    }

    sorted
        .iter()
        .take(limit)
        .filter(|(_, _, _, _, rss)| *rss > 0)
        .map(|(pid, comm, state, _, rss_kb)| {
            json!({
                "pid": pid,
                "name": comm,
                "state": state,
                "cpu": (cpu_pct_of(*pid) * 10.0).round() / 10.0,
                "rss": rss_kb,
                "rss_mb": format!("{:.1}", *rss_kb as f64 / 1024.0),
            })
        })
        .collect()
}

pub(crate) fn count_processes() -> u64 {
    match fs::read_dir("/proc") {
        Ok(d) => d
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().parse::<u32>().is_ok())
            .count() as u64,
        Err(_) => 0,
    }
}

pub(crate) fn get_hostname() -> String {
    read_file(HOSTNAME_FILE)
        .unwrap_or_default()
        .trim()
        .to_string()
}

// Windows-only features: Linux no-ops (keeps platform API surface uniform)
pub(crate) fn get_ports() -> Vec<Value> {
    vec![]
}
pub(crate) fn get_wifi_signal() -> Value {
    json!({"ssid": "", "signal": "", "connected": false})
}
pub(crate) fn get_event_log(_limit: usize) -> Vec<Value> {
    vec![]
}

// Linux reads /proc per call; no shared snapshot needed
pub(crate) fn refresh_system() {}
