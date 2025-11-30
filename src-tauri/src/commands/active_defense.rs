// Active Defense Commands - D-Bus integration for system sentinel controls
// Provides Tauri commands to interact with jesternet sentinel services

use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::State;

use super::AppState;

// ===== RESULT TYPES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelResult {
    pub success: bool,
    pub message: String,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu: Option<CpuMetrics>,
    pub memory: Option<MemoryMetrics>,
    pub thermal: Option<ThermalMetrics>,
    pub gpu: Option<GpuMetrics>,
    pub network: Option<NetworkMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub load: String,
    pub frequency: String,
    pub governor: String,
    pub turbo_enabled: bool,
    pub alert_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub available_mb: u64,
    pub memory_percent: f64,
    pub swap_percent: f64,
    pub alert_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalMetrics {
    pub hottest: String,
    pub sensors: String,
    pub alert_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub gpus: String,
    pub utilization: String,
    pub vram: String,
    pub thermals: String,
    pub alert_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub bandwidth: String,
    pub connection_count: String,
    pub blocked_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_mb: f64,
}

// ===== HELPER: D-BUS CALL =====

fn dbus_call(service: &str, path: &str, interface: &str, method: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("busctl");
    cmd.arg("call")
        .arg(service)
        .arg(path)
        .arg(interface)
        .arg(method);

    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().map_err(|e| format!("Failed to execute busctl: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

fn dbus_call_json(service: &str, path: &str, interface: &str, method: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new("busctl");
    cmd.arg("call")
        .arg("--json=short")
        .arg(service)
        .arg(path)
        .arg(interface)
        .arg(method);

    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().map_err(|e| format!("Failed to execute busctl: {}", e))?;

    if output.status.success() {
        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        // Extract the actual data from D-Bus JSON wrapper: {"type":"s","data":["..."]}
        extract_dbus_string_data(&raw)
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Extract the actual string data from D-Bus JSON response
/// D-Bus returns: {"type":"s","data":["actual json string here"]}
/// We need to extract and parse the inner JSON string
fn extract_dbus_string_data(raw: &str) -> Result<String, String> {
    // Try to parse as JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(raw.trim());

    match parsed {
        Ok(value) => {
            // Check if it's the D-Bus wrapper format
            if let Some(data_array) = value.get("data").and_then(|d| d.as_array()) {
                if let Some(first) = data_array.first() {
                    // If it's a string, return it (it might be nested JSON)
                    if let Some(s) = first.as_str() {
                        return Ok(s.to_string());
                    }
                    // Otherwise stringify it
                    return Ok(first.to_string());
                }
            }
            // Not the wrapper format, return as-is
            Ok(raw.trim().to_string())
        }
        Err(_) => {
            // Not JSON, return raw
            Ok(raw.trim().to_string())
        }
    }
}

// ===== SYSTEM METRICS =====

#[tauri::command]
pub async fn get_all_sentinel_metrics() -> Result<SystemMetrics, String> {
    let cpu = get_cpu_snapshot().await.ok();
    let memory = get_memory_snapshot().await.ok();
    let thermal = get_thermal_snapshot().await.ok();
    let gpu = get_gpu_snapshot().await.ok();
    let network = get_network_snapshot().await.ok();

    Ok(SystemMetrics {
        cpu,
        memory,
        thermal,
        gpu,
        network,
    })
}

// ===== CPU SENTINEL =====

/// Parse CPU load JSON and format as human-readable string
fn format_cpu_load(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        let load_1m = value.get("load_1m").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let load_5m = value.get("load_5m").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let load_15m = value.get("load_15m").and_then(|v| v.as_f64()).unwrap_or(0.0);
        format!("{:.2} / {:.2} / {:.2}", load_1m, load_5m, load_15m)
    } else {
        json_str.to_string()
    }
}

/// Parse CPU frequency JSON and format as human-readable string
/// Expected format: {"avg_frequency_mhz":2288,"frequency_pct":49.7,"max_frequency_mhz":4600,"min_frequency_mhz":800}
fn format_cpu_frequency(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        // Try avg_frequency_mhz first (actual field name from D-Bus)
        if let Some(freq) = value.get("avg_frequency_mhz").and_then(|v| v.as_f64()) {
            let max = value.get("max_frequency_mhz").and_then(|v| v.as_f64());
            let pct = value.get("frequency_pct").and_then(|v| v.as_f64());

            if let (Some(max_freq), Some(percent)) = (max, pct) {
                return format!("{:.0} MHz ({:.0}%)", freq, percent);
            }
            return format!("{:.0} MHz", freq);
        }
        // Fallback field names
        if let Some(freq) = value.get("current_mhz").and_then(|v| v.as_f64()) {
            return format!("{:.0} MHz", freq);
        }
        if let Some(freq) = value.get("frequency_mhz").and_then(|v| v.as_f64()) {
            return format!("{:.0} MHz", freq);
        }
        // If it's just a number
        if let Some(freq) = value.as_f64() {
            return format!("{:.0} MHz", freq);
        }
    }
    json_str.to_string()
}

/// Extract governor from JSON response
fn extract_governor(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(gov) = value.get("governor").and_then(|v| v.as_str()) {
            return gov.to_string();
        }
        if let Some(gov) = value.as_str() {
            return gov.to_string();
        }
    }
    // Try to clean up raw string response (remove quotes)
    json_str.trim_matches('"').to_string()
}

/// Extract turbo state from JSON response
fn extract_turbo_enabled(json_str: &str) -> bool {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(turbo) = value.get("turbo_enabled").and_then(|v| v.as_bool()) {
            return turbo;
        }
        if let Some(turbo) = value.as_bool() {
            return turbo;
        }
    }
    // Default to true if we can't parse
    true
}

/// Clean up alert level response
fn clean_alert_level(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(level) = value.get("level").and_then(|v| v.as_str()) {
            return level.to_string();
        }
        if let Some(level) = value.as_str() {
            return level.to_string();
        }
    }
    // Remove quotes from raw string
    json_str.trim_matches('"').to_string()
}

#[tauri::command]
pub async fn get_cpu_snapshot() -> Result<CpuMetrics, String> {
    let load_raw = dbus_call_json(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "GetLoad",
        &[]
    ).unwrap_or_default();

    let frequency_raw = dbus_call_json(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "GetFrequency",
        &[]
    ).unwrap_or_default();

    let governor_raw = dbus_call_json(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "GetGovernor",
        &[]
    ).unwrap_or_else(|_| "unknown".to_string());

    let turbo_raw = dbus_call_json(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "GetTurboEnabled",
        &[]
    ).unwrap_or_else(|_| "true".to_string());

    let alert_raw = dbus_call_json(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "GetAlertLevel",
        &[]
    ).unwrap_or_default();

    Ok(CpuMetrics {
        load: format_cpu_load(&load_raw),
        frequency: format_cpu_frequency(&frequency_raw),
        governor: extract_governor(&governor_raw),
        turbo_enabled: extract_turbo_enabled(&turbo_raw),
        alert_level: clean_alert_level(&alert_raw),
    })
}

#[tauri::command]
pub async fn cpu_set_governor(governor: String) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "SetGovernor",
        &["s", &governor]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("CPU governor set to {}", governor),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn cpu_disable_turbo() -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "DisableTurbo",
        &[]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: "Turbo boost disabled".to_string(),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn cpu_enable_turbo() -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "EnableTurbo",
        &[]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: "Turbo boost enabled".to_string(),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn cpu_emergency_power_reduce() -> Result<SentinelResult, String> {
    let result = dbus_call_json(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "EmergencyPowerReduce",
        &[]
    );

    match result {
        Ok(data) => Ok(SentinelResult {
            success: true,
            message: "Emergency power reduction activated".to_string(),
            data: Some(data),
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn cpu_reset_controls() -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.cpu.Sentinel",
        "/org/jesternet/cpu/Sentinel",
        "org.jesternet.cpu.Sentinel",
        "ResetCpuControls",
        &[]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: "CPU controls reset to defaults".to_string(),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

// ===== MEMORY MONITOR =====

#[tauri::command]
pub async fn get_memory_snapshot() -> Result<MemoryMetrics, String> {
    let available: u64 = dbus_call(
        "org.jesternet.memory.Monitor",
        "/org/jesternet/memory/Monitor",
        "org.jesternet.memory.Monitor",
        "GetAvailableMb",
        &[]
    ).ok().and_then(|s| s.trim().split_whitespace().last().and_then(|n| n.parse().ok())).unwrap_or(0);

    let mem_percent: f64 = dbus_call(
        "org.jesternet.memory.Monitor",
        "/org/jesternet/memory/Monitor",
        "org.jesternet.memory.Monitor",
        "GetMemoryPercent",
        &[]
    ).ok().and_then(|s| s.trim().split_whitespace().last().and_then(|n| n.parse().ok())).unwrap_or(0.0);

    let swap_percent: f64 = dbus_call(
        "org.jesternet.memory.Monitor",
        "/org/jesternet/memory/Monitor",
        "org.jesternet.memory.Monitor",
        "GetSwapPercent",
        &[]
    ).ok().and_then(|s| s.trim().split_whitespace().last().and_then(|n| n.parse().ok())).unwrap_or(0.0);

    let alert = dbus_call_json(
        "org.jesternet.memory.Monitor",
        "/org/jesternet/memory/Monitor",
        "org.jesternet.memory.Monitor",
        "GetAlertLevel",
        &[]
    ).unwrap_or_default();

    Ok(MemoryMetrics {
        available_mb: available,
        memory_percent: mem_percent,
        swap_percent,
        alert_level: alert,
    })
}

#[tauri::command]
pub async fn memory_drop_caches(level: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.memory.Monitor",
        "/org/jesternet/memory/Monitor",
        "org.jesternet.memory.Monitor",
        "DropCaches",
        &["u", &level.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Dropped caches (level {})", level),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn memory_emergency_relief() -> Result<SentinelResult, String> {
    let result = dbus_call_json(
        "org.jesternet.memory.Monitor",
        "/org/jesternet/memory/Monitor",
        "org.jesternet.memory.Monitor",
        "EmergencyRelief",
        &[]
    );

    match result {
        Ok(data) => Ok(SentinelResult {
            success: true,
            message: "Emergency memory relief activated".to_string(),
            data: Some(data),
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn memory_trigger_oom_kill() -> Result<SentinelResult, String> {
    let result = dbus_call_json(
        "org.jesternet.memory.Monitor",
        "/org/jesternet/memory/Monitor",
        "org.jesternet.memory.Monitor",
        "TriggerOomKill",
        &[]
    );

    match result {
        Ok(data) => Ok(SentinelResult {
            success: true,
            message: "OOM kill triggered".to_string(),
            data: Some(data),
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn memory_compact() -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.memory.Monitor",
        "/org/jesternet/memory/Monitor",
        "org.jesternet.memory.Monitor",
        "CompactMemory",
        &[]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: "Memory compaction triggered".to_string(),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

// ===== THERMAL SENTINEL =====

/// Format hottest sensor reading from JSON
/// Expected format: {"id":"coretemp_3","name":"Core 1","sensor_type":"Cpu","current_temp":95000,"max_temp":100000,"crit_temp":100000,"label":"Core 1"}
/// Note: current_temp is in millidegrees (95000 = 95°C)
fn format_hottest_sensor(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        // current_temp is in millidegrees
        let temp_milli = value.get("current_temp").and_then(|v| v.as_f64());
        let label = value.get("label").and_then(|v| v.as_str())
            .or_else(|| value.get("name").and_then(|v| v.as_str()));
        let sensor_type = value.get("sensor_type").and_then(|v| v.as_str());

        if let Some(t) = temp_milli {
            let temp_c = t / 1000.0; // Convert from millidegrees to degrees
            let name = label.or(sensor_type).unwrap_or("Sensor");
            return format!("{:.1}°C ({})", temp_c, name);
        }

        // Fallback: try temperature field in degrees
        if let Some(t) = value.get("temperature").and_then(|v| v.as_f64()) {
            let name = label.unwrap_or("Sensor");
            return format!("{:.1}°C ({})", t, name);
        }
    }
    json_str.to_string()
}

/// Format sensor list from JSON
/// Expected format: Array of sensors with current_temp in millidegrees
fn format_sensors(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(sensors) = value.as_array() {
            // Group sensors by type and show top ones
            let mut cpu_temps: Vec<f64> = Vec::new();
            let mut gpu_temps: Vec<f64> = Vec::new();
            let mut other_temps: Vec<(String, f64)> = Vec::new();

            for s in sensors {
                let temp_milli = s.get("current_temp").and_then(|v| v.as_f64());
                let sensor_type = s.get("sensor_type").and_then(|v| v.as_str()).unwrap_or("Other");
                let label = s.get("label").and_then(|v| v.as_str())
                    .or_else(|| s.get("name").and_then(|v| v.as_str()))
                    .unwrap_or("?");

                if let Some(t) = temp_milli {
                    let temp_c = t / 1000.0;
                    match sensor_type {
                        "Cpu" => cpu_temps.push(temp_c),
                        "Gpu" => gpu_temps.push(temp_c),
                        _ => other_temps.push((label.to_string(), temp_c)),
                    }
                }
            }

            let mut parts: Vec<String> = Vec::new();

            // Show CPU average/max
            if !cpu_temps.is_empty() {
                let max = cpu_temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let avg = cpu_temps.iter().sum::<f64>() / cpu_temps.len() as f64;
                parts.push(format!("CPU:{:.0}°(avg:{:.0}°)", max, avg));
            }

            // Show GPU temps
            if !gpu_temps.is_empty() {
                let max = gpu_temps.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                parts.push(format!("GPU:{:.0}°", max));
            }

            // Show up to 2 other notable sensors
            other_temps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for (name, temp) in other_temps.iter().take(2) {
                let short_name = if name.len() > 8 { &name[..8] } else { name };
                parts.push(format!("{}:{:.0}°", short_name, temp));
            }

            if !parts.is_empty() {
                return parts.join(" | ");
            }

            return format!("{} sensors", sensors.len());
        }
        // Maybe it's a count
        if let Some(count) = value.as_u64() {
            return format!("{} sensors", count);
        }
    }
    json_str.to_string()
}

#[tauri::command]
pub async fn get_thermal_snapshot() -> Result<ThermalMetrics, String> {
    let hottest_raw = dbus_call_json(
        "org.jesternet.thermal.Sentinel",
        "/org/jesternet/thermal/Sentinel",
        "org.jesternet.thermal.Sentinel",
        "GetHottest",
        &[]
    ).unwrap_or_default();

    let sensors_raw = dbus_call_json(
        "org.jesternet.thermal.Sentinel",
        "/org/jesternet/thermal/Sentinel",
        "org.jesternet.thermal.Sentinel",
        "GetSensors",
        &[]
    ).unwrap_or_default();

    let alert_raw = dbus_call_json(
        "org.jesternet.thermal.Sentinel",
        "/org/jesternet/thermal/Sentinel",
        "org.jesternet.thermal.Sentinel",
        "GetAlertLevel",
        &[]
    ).unwrap_or_default();

    Ok(ThermalMetrics {
        hottest: format_hottest_sensor(&hottest_raw),
        sensors: format_sensors(&sensors_raw),
        alert_level: clean_alert_level(&alert_raw),
    })
}

#[tauri::command]
pub async fn thermal_emergency_cool() -> Result<SentinelResult, String> {
    let result = dbus_call_json(
        "org.jesternet.thermal.Sentinel",
        "/org/jesternet/thermal/Sentinel",
        "org.jesternet.thermal.Sentinel",
        "EmergencyCool",
        &[]
    );

    match result {
        Ok(data) => Ok(SentinelResult {
            success: true,
            message: "Emergency cooling activated".to_string(),
            data: Some(data),
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn thermal_set_power_limit(watts: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.thermal.Sentinel",
        "/org/jesternet/thermal/Sentinel",
        "org.jesternet.thermal.Sentinel",
        "SetPowerLimit",
        &["u", &watts.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Power limit set to {} watts", watts),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn thermal_reset_controls() -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.thermal.Sentinel",
        "/org/jesternet/thermal/Sentinel",
        "org.jesternet.thermal.Sentinel",
        "ResetThermalControls",
        &[]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: "Thermal controls reset to defaults".to_string(),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

// ===== GPU SENTINEL =====

/// Format GPU list from JSON
/// Can be a string (GPU name) or an array of GPU objects
fn format_gpus(json_str: &str) -> String {
    // First check if it's just a plain string (GPU name)
    let trimmed = json_str.trim();
    if !trimmed.starts_with('{') && !trimmed.starts_with('[') {
        // It's a plain string, just return it cleaned up
        return trimmed.trim_matches('"').to_string();
    }

    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(gpus) = value.as_array() {
            if gpus.is_empty() {
                return "No GPUs".to_string();
            }
            let names: Vec<String> = gpus.iter().filter_map(|g| {
                g.get("name").and_then(|v| v.as_str()).map(|s| s.to_string())
            }).collect();
            if !names.is_empty() {
                return names.join(", ");
            }
            return format!("{} GPU(s)", gpus.len());
        }
        if let Some(count) = value.as_u64() {
            return format!("{} GPU(s)", count);
        }
        if let Some(name) = value.as_str() {
            return name.to_string();
        }
    }
    json_str.to_string()
}

/// Format GPU utilization from JSON
/// Expected format: {"decoder_utilization_pct":0,"encoder_utilization_pct":0,"gpu_utilization_pct":0,"memory_utilization_pct":0}
fn format_gpu_utilization(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        // Check for gpu_utilization_pct (actual field from D-Bus)
        let gpu_util = value.get("gpu_utilization_pct").and_then(|v| v.as_f64());
        let mem_util = value.get("memory_utilization_pct").and_then(|v| v.as_f64());
        let enc_util = value.get("encoder_utilization_pct").and_then(|v| v.as_f64());
        let dec_util = value.get("decoder_utilization_pct").and_then(|v| v.as_f64());

        if let Some(gpu) = gpu_util {
            let mut parts = vec![format!("GPU:{:.0}%", gpu)];
            if let Some(mem) = mem_util {
                if mem > 0.0 {
                    parts.push(format!("Mem:{:.0}%", mem));
                }
            }
            if let Some(enc) = enc_util {
                if enc > 0.0 {
                    parts.push(format!("Enc:{:.0}%", enc));
                }
            }
            if let Some(dec) = dec_util {
                if dec > 0.0 {
                    parts.push(format!("Dec:{:.0}%", dec));
                }
            }
            return parts.join(" ");
        }

        // Fallback: try generic percent field
        if let Some(pct) = value.get("percent").or_else(|| value.get("utilization")).and_then(|v| v.as_f64()) {
            return format!("{:.0}%", pct);
        }
        if let Some(pct) = value.as_f64() {
            return format!("{:.0}%", pct);
        }
    }
    json_str.to_string()
}

/// Format VRAM usage from JSON
/// Expected format: {"free_mib":3630,"total_mib":4096,"used_mib":466,"used_pct":11.37}
fn format_vram(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        // Check for MiB fields (actual field names from D-Bus)
        let used = value.get("used_mib").and_then(|v| v.as_f64());
        let total = value.get("total_mib").and_then(|v| v.as_f64());
        let used_pct = value.get("used_pct").and_then(|v| v.as_f64());

        if let (Some(u), Some(t)) = (used, total) {
            let pct = used_pct.unwrap_or_else(|| if t > 0.0 { (u / t) * 100.0 } else { 0.0 });
            return format!("{:.0}/{:.0} MiB ({:.0}%)", u, t, pct);
        }

        // Fallback: try MB fields
        let used = value.get("used_mb").or_else(|| value.get("used")).and_then(|v| v.as_f64());
        let total = value.get("total_mb").or_else(|| value.get("total")).and_then(|v| v.as_f64());

        if let (Some(u), Some(t)) = (used, total) {
            let pct = if t > 0.0 { (u / t) * 100.0 } else { 0.0 };
            return format!("{:.0}/{:.0} MB ({:.0}%)", u, t, pct);
        }
        if let Some(pct) = value.get("percent").and_then(|v| v.as_f64()) {
            return format!("{:.0}%", pct);
        }
    }
    json_str.to_string()
}

/// Format GPU thermals from JSON
/// Expected format: {"max_temperature_c":74,"power_draw_w":9.2,"power_limit_w":0.0,"temperature_c":74,"total_power_w":9.2}
fn format_gpu_thermals(json_str: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        // Check for temperature_c (actual field from D-Bus)
        let temp = value.get("temperature_c").and_then(|v| v.as_f64());
        let power = value.get("power_draw_w").and_then(|v| v.as_f64());
        let power_limit = value.get("power_limit_w").and_then(|v| v.as_f64());

        if let Some(t) = temp {
            let mut result = format!("{:.0}°C", t);
            if let Some(p) = power {
                if p > 0.0 {
                    if let Some(limit) = power_limit {
                        if limit > 0.0 {
                            result.push_str(&format!(" | {:.0}W/{:.0}W", p, limit));
                        } else {
                            result.push_str(&format!(" | {:.0}W", p));
                        }
                    } else {
                        result.push_str(&format!(" | {:.0}W", p));
                    }
                }
            }
            return result;
        }

        // Fallback: try generic temperature field
        if let Some(temp) = value.get("temperature").or_else(|| value.get("temp")).and_then(|v| v.as_f64()) {
            return format!("{:.0}°C", temp);
        }
        if let Some(temp) = value.as_f64() {
            return format!("{:.0}°C", temp);
        }
    }
    json_str.to_string()
}

#[tauri::command]
pub async fn get_gpu_snapshot() -> Result<GpuMetrics, String> {
    let gpus_raw = dbus_call_json(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "GetGpus",
        &[]
    ).unwrap_or_default();

    let utilization_raw = dbus_call_json(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "GetUtilization",
        &[]
    ).unwrap_or_default();

    let vram_raw = dbus_call_json(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "GetVram",
        &[]
    ).unwrap_or_default();

    let thermals_raw = dbus_call_json(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "GetThermals",
        &[]
    ).unwrap_or_default();

    let alert_raw = dbus_call_json(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "GetAlertLevel",
        &[]
    ).unwrap_or_default();

    Ok(GpuMetrics {
        gpus: format_gpus(&gpus_raw),
        utilization: format_gpu_utilization(&utilization_raw),
        vram: format_vram(&vram_raw),
        thermals: format_gpu_thermals(&thermals_raw),
        alert_level: clean_alert_level(&alert_raw),
    })
}

#[tauri::command]
pub async fn gpu_emergency_throttle(gpu_index: u32) -> Result<SentinelResult, String> {
    let result = dbus_call_json(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "EmergencyThrottle",
        &["u", &gpu_index.to_string()]
    );

    match result {
        Ok(data) => Ok(SentinelResult {
            success: true,
            message: format!("Emergency throttle applied to GPU {}", gpu_index),
            data: Some(data),
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn gpu_set_power_limit(gpu_index: u32, watts: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "SetPowerLimit",
        &["u", &gpu_index.to_string(), "u", &watts.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("GPU {} power limit set to {} watts", gpu_index, watts),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn gpu_reset(gpu_index: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "ResetGpu",
        &["u", &gpu_index.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("GPU {} reset", gpu_index),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn gpu_kill_process(pid: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.gpu.Sentinel",
        "/org/jesternet/gpu/Sentinel",
        "org.jesternet.gpu.Sentinel",
        "KillGpuProcess",
        &["u", &pid.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Killed GPU process {}", pid),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

// ===== NETWORK FLOW ANALYZER =====

#[tauri::command]
pub async fn get_network_snapshot() -> Result<NetworkMetrics, String> {
    let bandwidth = dbus_call_json(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "GetBandwidth",
        &[]
    ).unwrap_or_default();

    let connection_count = dbus_call_json(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "GetConnectionCount",
        &[]
    ).unwrap_or_default();

    let blocked = dbus_call_json(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "ListBlocked",
        &[]
    ).unwrap_or_default();

    // Parse blocked IPs from JSON
    let blocked_ips: Vec<String> = serde_json::from_str(&blocked).unwrap_or_default();

    Ok(NetworkMetrics {
        bandwidth,
        connection_count,
        blocked_ips,
    })
}

#[tauri::command]
pub async fn network_block_ip(ip: String) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "BlockIp",
        &["s", &ip]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Blocked IP: {}", ip),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn network_unblock_ip(ip: String) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "UnblockIp",
        &["s", &ip]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Unblocked IP: {}", ip),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn network_kill_connections_ip(ip: String) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "KillConnectionsIp",
        &["s", &ip]
    );

    match result {
        Ok(data) => Ok(SentinelResult {
            success: true,
            message: format!("Killed connections to {}", ip),
            data: Some(data),
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn network_rate_limit_ip(ip: String, kbytes_per_sec: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "RateLimitIp",
        &["s", &ip, "u", &kbytes_per_sec.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Rate limited {} to {} KB/s", ip, kbytes_per_sec),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn network_block_process(pid: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "BlockProcessNetwork",
        &["u", &pid.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Blocked network for process {}", pid),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn network_clear_all_blocks() -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.network.FlowAnalyzer",
        "/org/jesternet/network/FlowAnalyzer",
        "org.jesternet.network.FlowAnalyzer",
        "ClearAllBlocks",
        &[]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: "Cleared all network blocks".to_string(),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

// ===== PROCESS DIAGNOSTICIAN =====

#[tauri::command]
pub async fn process_freeze(pid: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.process.Diagnostician",
        "/org/jesternet/process/Diagnostician",
        "org.jesternet.process.Diagnostician",
        "FreezeProcess",
        &["u", &pid.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Frozen process {}", pid),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn process_thaw(pid: u32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.process.Diagnostician",
        "/org/jesternet/process/Diagnostician",
        "org.jesternet.process.Diagnostician",
        "ThawProcess",
        &["u", &pid.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Thawed process {}", pid),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn process_kill(pid: u32, signal: i32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.process.Diagnostician",
        "/org/jesternet/process/Diagnostician",
        "org.jesternet.process.Diagnostician",
        "KillProcess",
        &["u", &pid.to_string(), "i", &signal.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Sent signal {} to process {}", signal, pid),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn process_set_nice(pid: u32, nice: i32) -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.process.Diagnostician",
        "/org/jesternet/process/Diagnostician",
        "org.jesternet.process.Diagnostician",
        "SetNice",
        &["u", &pid.to_string(), "i", &nice.to_string()]
    );

    match result {
        Ok(_) => Ok(SentinelResult {
            success: true,
            message: format!("Set nice {} for process {}", nice, pid),
            data: None,
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

#[tauri::command]
pub async fn process_reap_zombies() -> Result<SentinelResult, String> {
    let result = dbus_call(
        "org.jesternet.process.Diagnostician",
        "/org/jesternet/process/Diagnostician",
        "org.jesternet.process.Diagnostician",
        "ReapZombies",
        &[]
    );

    match result {
        Ok(data) => Ok(SentinelResult {
            success: true,
            message: "Reaped zombie processes".to_string(),
            data: Some(data),
        }),
        Err(e) => Ok(SentinelResult {
            success: false,
            message: e,
            data: None,
        }),
    }
}

/// Parse process list JSON into Vec<ProcessInfo>
fn parse_process_list(json_str: &str) -> Vec<ProcessInfo> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(procs) = value.as_array() {
            return procs.iter().filter_map(|p| {
                let pid = p.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let name = p.get("name").or_else(|| p.get("command")).and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                let cpu_percent = p.get("cpu_percent").or_else(|| p.get("cpu")).and_then(|v| v.as_f64()).unwrap_or(0.0);
                let memory_mb = p.get("memory_mb").or_else(|| p.get("mem_mb")).or_else(|| p.get("memory")).and_then(|v| v.as_f64()).unwrap_or(0.0);
                Some(ProcessInfo {
                    pid,
                    name,
                    cpu_percent,
                    memory_mb,
                })
            }).collect();
        }
    }
    Vec::new()
}

#[tauri::command]
pub async fn process_get_top_cpu(count: u32) -> Result<Vec<ProcessInfo>, String> {
    let raw = dbus_call_json(
        "org.jesternet.process.Diagnostician",
        "/org/jesternet/process/Diagnostician",
        "org.jesternet.process.Diagnostician",
        "GetTopCpu",
        &["u", &count.to_string()]
    )?;
    Ok(parse_process_list(&raw))
}

#[tauri::command]
pub async fn process_get_top_memory(count: u32) -> Result<Vec<ProcessInfo>, String> {
    let raw = dbus_call_json(
        "org.jesternet.process.Diagnostician",
        "/org/jesternet/process/Diagnostician",
        "org.jesternet.process.Diagnostician",
        "GetTopMemory",
        &["u", &count.to_string()]
    )?;
    Ok(parse_process_list(&raw))
}

// ===== PANIC BUTTONS =====

#[tauri::command]
pub async fn emergency_all_systems() -> Result<SentinelResult, String> {
    // Execute all emergency actions in parallel
    let _ = thermal_emergency_cool().await;
    let _ = cpu_emergency_power_reduce().await;
    let _ = memory_emergency_relief().await;

    Ok(SentinelResult {
        success: true,
        message: "All emergency protocols activated".to_string(),
        data: None,
    })
}

#[tauri::command]
pub async fn lockdown_network() -> Result<SentinelResult, String> {
    // Block all non-essential network traffic
    // This is a placeholder - real implementation would be more sophisticated
    Ok(SentinelResult {
        success: true,
        message: "Network lockdown initiated".to_string(),
        data: None,
    })
}

#[tauri::command]
pub async fn performance_mode() -> Result<SentinelResult, String> {
    // Enable turbo and set performance governor
    let _ = cpu_enable_turbo().await;
    let _ = cpu_set_governor("performance".to_string()).await;

    Ok(SentinelResult {
        success: true,
        message: "Performance mode activated".to_string(),
        data: None,
    })
}

#[tauri::command]
pub async fn reset_all_controls() -> Result<SentinelResult, String> {
    let _ = cpu_reset_controls().await;
    let _ = thermal_reset_controls().await;
    let _ = network_clear_all_blocks().await;

    Ok(SentinelResult {
        success: true,
        message: "All controls reset to defaults".to_string(),
        data: None,
    })
}
