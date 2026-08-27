use crate::{log_write, set_retention_days, tail_log, LOG_DIR};
pub(crate) fn cmd_log_show(args: &[String]) -> i32 {
    let lines = args
        .get(2)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(50);
    let log = tail_log(lines);
    if log.is_empty() {
        println!("no logs ({LOG_DIR})");
        return 0;
    }
    println!("=== hyper-panel log (last {lines} lines) ===");
    println!("{log}");
    0
}

pub(crate) fn cmd_log_system(args: &[String]) -> i32 {
    let lines = args
        .get(2)
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(50);
    // View host service logs via systemd journal
    let output = std::process::Command::new("journalctl")
        .args(["-u", "hyper-panel", "-n", &lines.to_string(), "--no-pager"])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            print!("{}", String::from_utf8_lossy(&o.stdout));
            0
        }
        Ok(o) => {
            eprintln!("journalctl failed: {}", String::from_utf8_lossy(&o.stderr));
            1
        }
        Err(e) => {
            eprintln!("cannot run journalctl: {e} (systemd only)");
            1
        }
    }
}

pub(crate) fn cmd_log_retention(args: &[String]) -> i32 {
    let days = match args.get(2).and_then(|s| s.parse::<u64>().ok()) {
        Some(d) if d > 0 => d,
        _ => {
            eprintln!("usage: hyper-panel log retention <days> (positive integer)");
            return 1;
        }
    };
    match set_retention_days(days) {
        Ok(()) => {
            println!("log retention set to {days} days");
            log_write("INFO", &format!("log retention set to {days} days"));
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
