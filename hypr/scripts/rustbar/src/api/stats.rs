//! System stats API (CPU usage, RAM usage, Battery status)

use std::fs;
use std::sync::Mutex;

struct CpuSample {
    idle: u64,
    total: u64,
}

static PREV_CPU: Mutex<Option<CpuSample>> = Mutex::new(None);

/// Reads CPU usage percentage (0..100) using delta from /proc/stat
pub fn get_cpu_usage() -> u32 {
    let content = match fs::read_to_string("/proc/stat") {
        Ok(c) => c,
        Err(_) => return 0,
    };

    let first_line = match content.lines().next() {
        Some(l) => l,
        None => return 0,
    };

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 5 || parts[0] != "cpu" {
        return 0;
    }

    let user: u64 = parts[1].parse().unwrap_or(0);
    let nice: u64 = parts[2].parse().unwrap_or(0);
    let system: u64 = parts[3].parse().unwrap_or(0);
    let idle: u64 = parts[4].parse().unwrap_or(0);
    let iowait: u64 = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let irq: u64 = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
    let softirq: u64 = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
    let steal: u64 = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);

    let idle_total = idle + iowait;
    let non_idle = user + nice + system + irq + softirq + steal;
    let total = idle_total + non_idle;

    let mut guard = PREV_CPU.lock().unwrap();
    let result = if let Some(prev) = guard.as_ref() {
        let total_delta = total.saturating_sub(prev.total);
        let idle_delta = idle_total.saturating_sub(prev.idle);
        if total_delta > 0 {
            let used_delta = total_delta.saturating_sub(idle_delta);
            ((used_delta as f64 / total_delta as f64) * 100.0).round() as u32
        } else {
            0
        }
    } else {
        0
    };

    *guard = Some(CpuSample { idle: idle_total, total });
    result
}

/// Reads RAM usage matching waybar-ram.sh logic (GB with 1 decimal if >= 1024MB, or MB)
pub fn get_ram_display() -> String {
    let content = match fs::read_to_string("/proc/meminfo") {
        Ok(c) => c,
        Err(_) => return "RAM 0M".to_string(),
    };

    let mut mem_total_kb = 0u64;
    let mut mem_available_kb = 0u64;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                mem_total_kb = parts[1].parse().unwrap_or(0);
            }
        } else if line.starts_with("MemAvailable:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                mem_available_kb = parts[1].parse().unwrap_or(0);
            }
        }
    }

    let mem_used_kb = mem_total_kb.saturating_sub(mem_available_kb);
    let used_mb = mem_used_kb / 1024;

    if used_mb >= 1024 {
        let used_gb = used_mb as f64 / 1024.0;
        format!("RAM {:.1}G", used_gb)
    } else {
        format!("RAM {}M", used_mb)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryStatus {
    Charging,
    Discharging,
    Full,
    Unknown,
}

pub struct BatteryInfo {
    pub capacity: u32,
    pub status: BatteryStatus,
}

/// Reads Battery capacity and status from /sys/class/power_supply/BAT0/
pub fn get_battery_info() -> BatteryInfo {
    let capacity = fs::read_to_string("/sys/class/power_supply/BAT0/capacity")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(100);

    let status_str = fs::read_to_string("/sys/class/power_supply/BAT0/status")
        .ok()
        .unwrap_or_else(|| "Full".to_string());

    let status = match status_str.trim() {
        "Charging" => BatteryStatus::Charging,
        "Discharging" => BatteryStatus::Discharging,
        "Full" => BatteryStatus::Full,
        _ => BatteryStatus::Unknown,
    };

    BatteryInfo { capacity, status }
}
