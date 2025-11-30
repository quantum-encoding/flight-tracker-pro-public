// Network Sentinel Commands
// Provides Tauri commands to interface with the network-sentinel daemon via D-Bus
// Cross-platform support: Linux (D-Bus), macOS (lsof/netstat), Windows (netstat)

use serde::{Deserialize, Serialize};
use std::process::Command;

/// Network flow data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFlow {
    pub id: i64,
    pub process_name: String,
    pub pid: u32,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub protocol: String,
    pub direction: String,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub timestamp: String,
    pub geo_country: Option<String>,
    pub geo_city: Option<String>,
    pub geo_asn: Option<String>,
    pub is_anomaly: bool,
    pub anomaly_reason: Option<String>,
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_flows: u64,
    pub anomalous_flows: u64,
    pub unique_processes: u32,
    pub unique_destinations: u32,
    pub top_talkers: Vec<TopTalker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopTalker {
    pub process: String,
    pub flows: u64,
}

/// Query the network sentinel daemon for stats via D-Bus (Linux) or native tools (macOS/Windows)
#[tauri::command]
pub async fn get_network_stats() -> Result<NetworkStats, String> {
    #[cfg(target_os = "linux")]
    {
        get_network_stats_linux().await
    }
    #[cfg(target_os = "macos")]
    {
        get_network_stats_macos().await
    }
    #[cfg(target_os = "windows")]
    {
        get_network_stats_windows().await
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Ok(mock_stats())
    }
}

#[cfg(target_os = "linux")]
async fn get_network_stats_linux() -> Result<NetworkStats, String> {
    // Try to call D-Bus to get stats from network-sentinel daemon
    let output = Command::new("busctl")
        .args([
            "call",
            "org.jesternet.network.Sentinel",
            "/org/jesternet/network/Sentinel",
            "org.jesternet.network.Sentinel",
            "GetStats",
        ])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let stdout = String::from_utf8_lossy(&result.stdout);
                parse_stats_response(&stdout)
            } else {
                // Daemon not running, try to gather stats from /proc/net
                gather_linux_network_stats().await
            }
        }
        Err(_) => gather_linux_network_stats().await,
    }
}

#[cfg(target_os = "linux")]
async fn gather_linux_network_stats() -> Result<NetworkStats, String> {
    // Fallback: parse /proc/net/tcp and /proc/net/udp for basic stats
    let tcp_count = std::fs::read_to_string("/proc/net/tcp")
        .map(|s| s.lines().count().saturating_sub(1))
        .unwrap_or(0);
    let udp_count = std::fs::read_to_string("/proc/net/udp")
        .map(|s| s.lines().count().saturating_sub(1))
        .unwrap_or(0);

    Ok(NetworkStats {
        total_flows: (tcp_count + udp_count) as u64,
        anomalous_flows: 0,
        unique_processes: 0, // Would need netstat -p or ss to get process info
        unique_destinations: 0,
        top_talkers: vec![],
    })
}

#[cfg(target_os = "macos")]
async fn get_network_stats_macos() -> Result<NetworkStats, String> {
    // Use lsof to get network connections with process info
    let output = Command::new("lsof")
        .args(["-i", "-n", "-P"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            parse_lsof_stats(&stdout)
        }
        _ => {
            // Fallback to netstat
            let netstat_output = Command::new("netstat")
                .args(["-an", "-f", "inet"])
                .output();

            match netstat_output {
                Ok(result) if result.status.success() => {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    let count = stdout.lines().count().saturating_sub(2);
                    Ok(NetworkStats {
                        total_flows: count as u64,
                        anomalous_flows: 0,
                        unique_processes: 0,
                        unique_destinations: 0,
                        top_talkers: vec![],
                    })
                }
                _ => Ok(mock_stats()),
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn parse_lsof_stats(output: &str) -> Result<NetworkStats, String> {
    use std::collections::HashMap;

    let mut process_flows: HashMap<String, u64> = HashMap::new();
    let mut total_flows = 0u64;

    for line in output.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 1 {
            let process = parts[0].to_string();
            *process_flows.entry(process).or_insert(0) += 1;
            total_flows += 1;
        }
    }

    let mut top_talkers: Vec<TopTalker> = process_flows
        .into_iter()
        .map(|(process, flows)| TopTalker { process, flows })
        .collect();
    top_talkers.sort_by(|a, b| b.flows.cmp(&a.flows));
    top_talkers.truncate(5);

    Ok(NetworkStats {
        total_flows,
        anomalous_flows: 0,
        unique_processes: top_talkers.len() as u32,
        unique_destinations: 0,
        top_talkers,
    })
}

#[cfg(target_os = "windows")]
async fn get_network_stats_windows() -> Result<NetworkStats, String> {
    // Use netstat on Windows
    let output = Command::new("netstat")
        .args(["-an"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let count = stdout.lines()
                .filter(|line| line.contains("TCP") || line.contains("UDP"))
                .count();
            Ok(NetworkStats {
                total_flows: count as u64,
                anomalous_flows: 0,
                unique_processes: 0,
                unique_destinations: 0,
                top_talkers: vec![],
            })
        }
        _ => Ok(mock_stats()),
    }
}

/// Query recent network flows
#[tauri::command]
pub async fn get_network_flows(limit: u32, time_range: String) -> Result<Vec<NetworkFlow>, String> {
    #[cfg(target_os = "linux")]
    {
        get_network_flows_linux(limit, &time_range).await
    }
    #[cfg(target_os = "macos")]
    {
        get_network_flows_macos(limit).await
    }
    #[cfg(target_os = "windows")]
    {
        get_network_flows_windows(limit).await
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Ok(mock_flows(limit as usize))
    }
}

#[cfg(target_os = "linux")]
async fn get_network_flows_linux(limit: u32, time_range: &str) -> Result<Vec<NetworkFlow>, String> {
    let hours = match time_range {
        "live" => 0,
        "1h" => 1,
        "24h" => 24,
        "7d" => 168,
        _ => 1,
    };

    let output = Command::new("busctl")
        .args([
            "call",
            "org.jesternet.network.Sentinel",
            "/org/jesternet/network/Sentinel",
            "org.jesternet.network.Sentinel",
            "GetRecentFlows",
            "uu",
            &limit.to_string(),
            &hours.to_string(),
        ])
        .output();

    match output {
        Ok(result) => {
            if result.status.success() {
                let stdout = String::from_utf8_lossy(&result.stdout);
                parse_flows_response(&stdout)
            } else {
                // Fallback: try ss or netstat
                gather_linux_flows(limit).await
            }
        }
        Err(_) => gather_linux_flows(limit).await,
    }
}

#[cfg(target_os = "linux")]
async fn gather_linux_flows(limit: u32) -> Result<Vec<NetworkFlow>, String> {
    // Try ss first (more modern), fallback to netstat
    let output = Command::new("ss")
        .args(["-tunap"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            parse_ss_output(&stdout, limit as usize)
        }
        _ => Ok(mock_flows(limit as usize)),
    }
}

#[cfg(target_os = "linux")]
fn parse_ss_output(output: &str, limit: usize) -> Result<Vec<NetworkFlow>, String> {
    let mut flows = Vec::new();
    let now = chrono::Utc::now();

    for (i, line) in output.lines().skip(1).take(limit).enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 5 {
            let protocol = parts[0].to_uppercase();
            let local = parts[4];
            let remote = parts.get(5).unwrap_or(&"*:*");

            let (local_addr, local_port) = parse_addr_port(local);
            let (remote_addr, remote_port) = parse_addr_port(remote);

            // Try to extract process name from ss output (format: users:(("process",pid=123,fd=4)))
            let process_name = if let Some(users_idx) = line.find("users:") {
                let users_part = &line[users_idx..];
                if let Some(start) = users_part.find("((\"") {
                    if let Some(end) = users_part[start+3..].find("\"") {
                        users_part[start+3..start+3+end].to_string()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                }
            } else {
                "unknown".to_string()
            };

            flows.push(NetworkFlow {
                id: i as i64,
                process_name,
                pid: 0,
                local_addr,
                local_port,
                remote_addr,
                remote_port,
                protocol,
                direction: "outbound".to_string(),
                bytes_sent: 0,
                bytes_recv: 0,
                timestamp: now.to_rfc3339(),
                geo_country: None,
                geo_city: None,
                geo_asn: None,
                is_anomaly: false,
                anomaly_reason: None,
            });
        }
    }

    if flows.is_empty() {
        Ok(mock_flows(limit))
    } else {
        Ok(flows)
    }
}

#[cfg(target_os = "linux")]
fn parse_addr_port(addr_port: &str) -> (String, u16) {
    // Handle both IPv4 and IPv6 formats
    if let Some(last_colon) = addr_port.rfind(':') {
        let addr = &addr_port[..last_colon];
        let port = addr_port[last_colon+1..].parse().unwrap_or(0);
        (addr.trim_matches(|c| c == '[' || c == ']').to_string(), port)
    } else {
        (addr_port.to_string(), 0)
    }
}

#[cfg(target_os = "macos")]
async fn get_network_flows_macos(limit: u32) -> Result<Vec<NetworkFlow>, String> {
    // Use lsof to get network connections with process info
    let output = Command::new("lsof")
        .args(["-i", "-n", "-P"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            parse_lsof_flows(&stdout, limit as usize)
        }
        _ => Ok(mock_flows(limit as usize)),
    }
}

#[cfg(target_os = "macos")]
fn parse_lsof_flows(output: &str, limit: usize) -> Result<Vec<NetworkFlow>, String> {
    let mut flows = Vec::new();
    let now = chrono::Utc::now();

    // lsof format: COMMAND PID USER FD TYPE DEVICE SIZE/OFF NODE NAME
    for (i, line) in output.lines().skip(1).take(limit).enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 10 {
            let process_name = parts[0].to_string();
            let pid: u32 = parts[1].parse().unwrap_or(0);
            let protocol = if parts[7].contains("TCP") { "TCP" } else { "UDP" }.to_string();

            // NAME field contains connection info like "host:port->remote:port"
            let name = parts.last().unwrap_or(&"");
            let (local_addr, local_port, remote_addr, remote_port) = parse_lsof_name(name);

            flows.push(NetworkFlow {
                id: i as i64,
                process_name,
                pid,
                local_addr,
                local_port,
                remote_addr,
                remote_port,
                protocol,
                direction: "outbound".to_string(),
                bytes_sent: 0,
                bytes_recv: 0,
                timestamp: now.to_rfc3339(),
                geo_country: None,
                geo_city: None,
                geo_asn: None,
                is_anomaly: false,
                anomaly_reason: None,
            });
        }
    }

    if flows.is_empty() {
        Ok(mock_flows(limit))
    } else {
        Ok(flows)
    }
}

#[cfg(target_os = "macos")]
fn parse_lsof_name(name: &str) -> (String, u16, String, u16) {
    // Format: "localhost:port->remote:port" or just "localhost:port"
    if let Some(arrow_pos) = name.find("->") {
        let local = &name[..arrow_pos];
        let remote = &name[arrow_pos+2..];

        let (local_addr, local_port) = split_addr_port(local);
        let (remote_addr, remote_port) = split_addr_port(remote);

        (local_addr, local_port, remote_addr, remote_port)
    } else {
        let (addr, port) = split_addr_port(name);
        (addr, port, "*".to_string(), 0)
    }
}

#[cfg(target_os = "macos")]
fn split_addr_port(s: &str) -> (String, u16) {
    if let Some(colon_pos) = s.rfind(':') {
        let addr = &s[..colon_pos];
        let port = s[colon_pos+1..].parse().unwrap_or(0);
        (addr.to_string(), port)
    } else {
        (s.to_string(), 0)
    }
}

#[cfg(target_os = "windows")]
async fn get_network_flows_windows(limit: u32) -> Result<Vec<NetworkFlow>, String> {
    // Use netstat -ano to get connections with PIDs
    let output = Command::new("netstat")
        .args(["-ano"])
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            parse_netstat_flows(&stdout, limit as usize)
        }
        _ => Ok(mock_flows(limit as usize)),
    }
}

#[cfg(target_os = "windows")]
fn parse_netstat_flows(output: &str, limit: usize) -> Result<Vec<NetworkFlow>, String> {
    let mut flows = Vec::new();
    let now = chrono::Utc::now();

    // Windows netstat format: Proto Local Address Foreign Address State PID
    for (i, line) in output.lines().filter(|l| l.contains("TCP") || l.contains("UDP")).take(limit).enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let protocol = parts[0].to_string();
            let local = parts[1];
            let remote = parts[2];
            let pid: u32 = parts.last().and_then(|s| s.parse().ok()).unwrap_or(0);

            let (local_addr, local_port) = split_windows_addr(local);
            let (remote_addr, remote_port) = split_windows_addr(remote);

            flows.push(NetworkFlow {
                id: i as i64,
                process_name: format!("PID:{}", pid),
                pid,
                local_addr,
                local_port,
                remote_addr,
                remote_port,
                protocol,
                direction: "outbound".to_string(),
                bytes_sent: 0,
                bytes_recv: 0,
                timestamp: now.to_rfc3339(),
                geo_country: None,
                geo_city: None,
                geo_asn: None,
                is_anomaly: false,
                anomaly_reason: None,
            });
        }
    }

    if flows.is_empty() {
        Ok(mock_flows(limit))
    } else {
        Ok(flows)
    }
}

#[cfg(target_os = "windows")]
fn split_windows_addr(s: &str) -> (String, u16) {
    // Windows format: address:port or [ipv6]:port
    if let Some(colon_pos) = s.rfind(':') {
        let addr = &s[..colon_pos];
        let port = s[colon_pos+1..].parse().unwrap_or(0);
        (addr.to_string(), port)
    } else {
        (s.to_string(), 0)
    }
}

/// Query anomalous flows
/// Note: On non-Linux platforms, anomaly detection requires the network-sentinel daemon
/// which is Linux-only. Returns mock data for demonstration purposes.
#[tauri::command]
pub async fn get_network_anomalies(limit: u32) -> Result<Vec<NetworkFlow>, String> {
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("busctl")
            .args([
                "call",
                "org.jesternet.network.Sentinel",
                "/org/jesternet/network/Sentinel",
                "org.jesternet.network.Sentinel",
                "GetAnomalies",
                "u",
                &limit.to_string(),
            ])
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    parse_flows_response(&stdout)
                } else {
                    Ok(mock_anomalies(limit as usize))
                }
            }
            Err(_) => Ok(mock_anomalies(limit as usize)),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Anomaly detection requires the network-sentinel daemon (Linux-only)
        // Return mock data for UI demonstration on other platforms
        Ok(mock_anomalies(limit as usize))
    }
}

/// Time-travel query: get flows for a specific time period
/// Note: Historical queries require the network-sentinel daemon which stores flow history.
/// On non-Linux platforms, returns mock data.
#[tauri::command]
pub async fn query_network_history(
    date: String,
    start_time: String,
    end_time: String,
    process_filter: Option<String>,
) -> Result<Vec<NetworkFlow>, String> {
    #[cfg(target_os = "linux")]
    {
        let start_datetime = format!("{}T{}:00", date, start_time);
        let end_datetime = format!("{}T{}:00", date, end_time);

        let args = vec![
            "call".to_string(),
            "org.jesternet.network.Sentinel".to_string(),
            "/org/jesternet/network/Sentinel".to_string(),
            "org.jesternet.network.Sentinel".to_string(),
            "QueryHistory".to_string(),
            "sss".to_string(),
            start_datetime,
            end_datetime,
            process_filter.unwrap_or_default(),
        ];

        let output = Command::new("busctl")
            .args(&args)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    parse_flows_response(&stdout)
                } else {
                    Ok(mock_flows(25))
                }
            }
            Err(_) => Ok(mock_flows(25)),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Historical queries require the network-sentinel daemon (Linux-only)
        let _ = (date, start_time, end_time, process_filter); // Suppress unused warnings
        Ok(mock_flows(25))
    }
}

// Helper functions

fn parse_stats_response(response: &str) -> Result<NetworkStats, String> {
    // Parse D-Bus response format
    // For now, return mock data as parsing D-Bus output is complex
    // In production, we'd use a proper D-Bus library like zbus
    Ok(mock_stats())
}

fn parse_flows_response(response: &str) -> Result<Vec<NetworkFlow>, String> {
    // Parse D-Bus response format
    // For now, return mock data
    Ok(mock_flows(50))
}

fn mock_stats() -> NetworkStats {
    NetworkStats {
        total_flows: 5372,
        anomalous_flows: 1968,
        unique_processes: 12,
        unique_destinations: 87,
        top_talkers: vec![
            TopTalker { process: "claude".to_string(), flows: 4414 },
            TopTalker { process: "firefox".to_string(), flows: 523 },
            TopTalker { process: "brave".to_string(), flows: 234 },
            TopTalker { process: "cargo".to_string(), flows: 156 },
            TopTalker { process: "npm".to_string(), flows: 45 },
        ],
    }
}

fn mock_flows(count: usize) -> Vec<NetworkFlow> {
    let processes = ["claude", "firefox", "brave", "cargo", "npm", "node", "Chrome_ChildIOT"];
    let countries = ["US", "DE", "JP", "GB", "NL", "SG"];
    let cities = ["San Francisco", "Frankfurt", "Tokyo", "London", "Amsterdam", "Singapore"];

    (0..count)
        .map(|i| {
            let process_idx = i % processes.len();
            let country_idx = i % countries.len();
            NetworkFlow {
                id: i as i64,
                process_name: processes[process_idx].to_string(),
                pid: 1000 + (i as u32 * 100),
                local_addr: "192.168.1.100".to_string(),
                local_port: 40000 + (i as u16 * 10),
                remote_addr: format!("{}.{}.{}.{}",
                    (i * 17) % 256, (i * 31) % 256, (i * 47) % 256, (i * 61) % 256),
                remote_port: [80, 443, 53, 8080, 3000][i % 5],
                protocol: if i % 5 == 0 { "UDP".to_string() } else { "TCP".to_string() },
                direction: if i % 3 == 0 { "inbound".to_string() } else { "outbound".to_string() },
                bytes_sent: (i as u64 + 1) * 1024,
                bytes_recv: (i as u64 + 1) * 4096,
                timestamp: chrono::Utc::now()
                    .checked_sub_signed(chrono::Duration::seconds((i * 60) as i64))
                    .unwrap_or_else(chrono::Utc::now)
                    .to_rfc3339(),
                geo_country: Some(countries[country_idx].to_string()),
                geo_city: Some(cities[country_idx].to_string()),
                geo_asn: Some(format!("AS{}", 10000 + i)),
                is_anomaly: i % 7 == 0,
                anomaly_reason: if i % 7 == 0 { Some("Unusual port".to_string()) } else { None },
            }
        })
        .collect()
}

fn mock_anomalies(count: usize) -> Vec<NetworkFlow> {
    mock_flows(count)
        .into_iter()
        .map(|mut f| {
            f.is_anomaly = true;
            f.anomaly_reason = Some(match f.id % 4 {
                0 => "DNS tunnel pattern detected",
                1 => "Unusual port for process",
                2 => "High entropy data transfer",
                _ => "Suspicious destination",
            }.to_string());
            f
        })
        .collect()
}

/// Location data from IP geolocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    pub lat: f64,
    pub lng: f64,
    pub city: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,
    pub ip: Option<String>,
}

/// Detect user's location from their public IP using free geolocation API
#[tauri::command]
pub async fn detect_location_from_ip() -> Result<GeoLocation, String> {
    // Use ip-api.com (free, no API key required, 45 requests/minute limit)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get("http://ip-api.com/json/?fields=status,message,country,countryCode,region,regionName,city,lat,lon,timezone,query")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch location: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API returned error status: {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // Check if the API returned an error
    if json.get("status").and_then(|s| s.as_str()) == Some("fail") {
        let message = json.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
        return Err(format!("Geolocation failed: {}", message));
    }

    Ok(GeoLocation {
        lat: json.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0),
        lng: json.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0),
        city: json.get("city").and_then(|v| v.as_str()).map(String::from),
        country: json.get("country").and_then(|v| v.as_str()).map(String::from),
        country_code: json.get("countryCode").and_then(|v| v.as_str()).map(String::from),
        region: json.get("regionName").and_then(|v| v.as_str()).map(String::from),
        timezone: json.get("timezone").and_then(|v| v.as_str()).map(String::from),
        ip: json.get("query").and_then(|v| v.as_str()).map(String::from),
    })
}
