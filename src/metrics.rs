use crate::platform;
use crate::{json, Value, CACHE_TTL, VERSION};
// System metrics: CPU/memory/disk/network/process/temperature/logs/ports
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

pub(crate) static CACHE: Mutex<Option<(Instant, Value)>> = Mutex::new(None);

// Process CPU sample: (sample time, total cpu ticks, pid -> proc ticks)
pub(crate) type ProcSample = (Instant, u64, HashMap<u32, u64>);
pub(crate) static PROC_SAMPLE: Mutex<Option<ProcSample>> = Mutex::new(None);

pub(crate) fn cache_get() -> Option<Value> {
    let c = CACHE.lock().unwrap();
    if let Some((t, v)) = &*c {
        if t.elapsed() < CACHE_TTL {
            return Some(v.clone());
        }
    }
    None
}
pub(crate) fn cache_put(v: Value) {
    *CACHE.lock().unwrap() = Some((Instant::now(), v));
}
pub(crate) fn get_system() -> Value {
    platform::refresh_system();
    let cpu = platform::get_cpu_usage().unwrap_or(0.0);
    let mem = platform::get_memory();
    let disk = platform::get_disk();
    let cpu_temp = platform::get_cpu_temp();
    let gpu_temp = platform::get_gpu_temp();
    let cpu_info = platform::get_cpu_info();
    let disks = platform::get_disks();
    let nprocs = platform::count_processes();

    json!({
        "node_name": platform::get_hostname(),
        "version": VERSION,
        "kernel": platform::get_kernel_version(),
        "cpu": (cpu * 10.0).round() / 10.0,
        "cpu_temp": cpu_temp.map(|t| format!("{t:.0}°C")).unwrap_or_else(|| "N/A".to_string()),
        "cpu_temp_raw": cpu_temp,
        "gpu_temp": gpu_temp.map(|t| format!("{t:.0}°C")).unwrap_or_else(|| "N/A".to_string()),
        "gpu_temp_raw": gpu_temp,
        "cpu_cores": cpu_info["cores"],
        "cpu_mhz": cpu_info["mhz"],
        "mem_total": mem["total_gb"].as_str().unwrap_or(""),
        "mem_used": mem["used_gb"].as_str().unwrap_or(""),
        "mem_percent": mem["percent"].as_f64().unwrap_or(0.0),
        "disk_total": disk["total_gb"].as_str().unwrap_or(""),
        "disk_used": disk["used_gb"].as_str().unwrap_or(""),
        "disk_percent": disk["percent"].as_f64().unwrap_or(0.0),
        "disks": disks,
        "loadavg": platform::get_loadavg(),
        "uptime": platform::get_uptime(),
        "processes": nprocs,
    })
}
pub(crate) fn get_system_cached() -> Value {
    if let Some(v) = cache_get() {
        return v;
    }
    let v = get_system();
    cache_put(v.clone());
    v
}
