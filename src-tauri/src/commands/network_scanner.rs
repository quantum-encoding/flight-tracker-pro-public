use serde::{Deserialize, Serialize};
use std::process::Command;
use anyhow::{Result, Context};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub bssid: String,
    pub signal_strength: i32,
    pub frequency: String,
    pub security: String,
    pub trusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BluetoothDevice {
    pub name: String,
    pub address: String,
    pub rssi: i16,
    pub device_type: String,
    pub trusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkScanResult {
    pub wifi_networks: Vec<WifiNetwork>,
    pub bluetooth_devices: Vec<BluetoothDevice>,
    pub scan_timestamp: String,
}

/// Scan for WiFi networks using system commands (cross-platform)
#[tauri::command]
pub async fn scan_wifi_networks(db_path: String) -> Result<Vec<WifiNetwork>, String> {
    scan_wifi_internal(&db_path)
        .await
        .map_err(|e| format!("WiFi scan failed: {}", e))
}

async fn scan_wifi_internal(db_path: &str) -> Result<Vec<WifiNetwork>> {
    #[cfg(target_os = "macos")]
    {
        scan_wifi_macos(db_path).await
    }

    #[cfg(target_os = "windows")]
    {
        scan_wifi_windows(db_path).await
    }

    #[cfg(target_os = "linux")]
    {
        scan_wifi_linux(db_path).await
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Ok(Vec::new())
    }
}

/// Linux WiFi scanning using nmcli or iwlist
#[cfg(target_os = "linux")]
async fn scan_wifi_linux(db_path: &str) -> Result<Vec<WifiNetwork>> {
    let mut networks = Vec::new();

    // Use 'nmcli' for WiFi scanning on Linux systems with NetworkManager
    let output = Command::new("nmcli")
        .args(["-t", "-f", "SSID,BSSID,SIGNAL,FREQ,SECURITY", "device", "wifi", "list"])
        .output()
        .context("Failed to execute nmcli command. Ensure NetworkManager is installed.")?;

    if !output.status.success() {
        // Fallback to iwlist if nmcli is not available
        return scan_wifi_iwlist(db_path).await;
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let trusted_devices = load_trusted_wifi_devices(db_path)?;

    for line in output_str.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 5 {
            let ssid = parts[0].trim().to_string();
            let bssid = parts[1].trim().to_uppercase();
            let signal = parts[2].trim().parse::<i32>().unwrap_or(0);
            let frequency = parts[3].trim().to_string();
            let security = parts[4].trim().to_string();

            if !ssid.is_empty() {
                networks.push(WifiNetwork {
                    ssid: ssid.clone(),
                    bssid: bssid.clone(),
                    signal_strength: signal,
                    frequency,
                    security,
                    trusted: trusted_devices.contains(&bssid),
                });
            }
        }
    }

    Ok(networks)
}

/// macOS WiFi scanning using airport utility
#[cfg(target_os = "macos")]
async fn scan_wifi_macos(db_path: &str) -> Result<Vec<WifiNetwork>> {
    let mut networks = Vec::new();

    // Use airport utility (available on all macOS systems)
    let output = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
        .args(["-s"])
        .output()
        .context("Failed to execute airport command")?;

    if !output.status.success() {
        return Ok(networks);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let trusted_devices = load_trusted_wifi_devices(db_path)?;

    // Skip header line
    for line in output_str.lines().skip(1) {
        // airport output format: SSID BSSID RSSI CHANNEL HT CC SECURITY
        // Fields are space-separated with varying widths
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 7 {
            // SSID might contain spaces, so we need to handle it carefully
            // BSSID is always in format XX:XX:XX:XX:XX:XX
            let bssid_idx = parts.iter().position(|p| p.contains(':') && p.len() == 17);
            if let Some(idx) = bssid_idx {
                let ssid = parts[..idx].join(" ");
                let bssid = parts[idx].to_uppercase();
                let rssi = parts.get(idx + 1).and_then(|s| s.parse::<i32>().ok()).unwrap_or(-100);
                let channel = parts.get(idx + 2).map(|s| s.to_string()).unwrap_or_default();
                let security = parts.get(idx + 6..).map(|s| s.join(" ")).unwrap_or_else(|| "Open".to_string());

                if !ssid.is_empty() {
                    networks.push(WifiNetwork {
                        ssid,
                        bssid: bssid.clone(),
                        signal_strength: rssi,
                        frequency: format!("Channel {}", channel),
                        security,
                        trusted: trusted_devices.contains(&bssid),
                    });
                }
            }
        }
    }

    Ok(networks)
}

/// Windows WiFi scanning using netsh
#[cfg(target_os = "windows")]
async fn scan_wifi_windows(db_path: &str) -> Result<Vec<WifiNetwork>> {
    let mut networks = Vec::new();

    // Use netsh to list available networks
    let output = Command::new("netsh")
        .args(["wlan", "show", "networks", "mode=bssid"])
        .output()
        .context("Failed to execute netsh command")?;

    if !output.status.success() {
        return Ok(networks);
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let trusted_devices = load_trusted_wifi_devices(db_path)?;

    let mut current_ssid = String::new();
    let mut current_bssid = String::new();
    let mut current_signal = 0i32;
    let mut current_channel = String::new();
    let mut current_security = String::new();

    for line in output_str.lines() {
        let line = line.trim();

        if line.starts_with("SSID") && !line.starts_with("BSSID") {
            // Save previous network if exists
            if !current_bssid.is_empty() {
                networks.push(WifiNetwork {
                    ssid: current_ssid.clone(),
                    bssid: current_bssid.clone(),
                    signal_strength: current_signal,
                    frequency: format!("Channel {}", current_channel),
                    security: current_security.clone(),
                    trusted: trusted_devices.contains(&current_bssid),
                });
            }

            // Parse new SSID
            if let Some(idx) = line.find(':') {
                current_ssid = line[idx + 1..].trim().to_string();
            }
            current_bssid = String::new();
            current_signal = 0;
            current_channel = String::new();
            current_security = String::new();
        } else if line.starts_with("BSSID") {
            if let Some(idx) = line.find(':') {
                current_bssid = line[idx + 1..].trim().to_uppercase();
            }
        } else if line.starts_with("Signal") {
            if let Some(idx) = line.find(':') {
                let signal_str = line[idx + 1..].trim().replace('%', "");
                // Convert percentage to approximate dBm (rough conversion)
                if let Ok(pct) = signal_str.parse::<i32>() {
                    current_signal = (pct as f32 / 2.0 - 100.0) as i32;
                }
            }
        } else if line.starts_with("Channel") {
            if let Some(idx) = line.find(':') {
                current_channel = line[idx + 1..].trim().to_string();
            }
        } else if line.starts_with("Authentication") || line.starts_with("Encryption") {
            if let Some(idx) = line.find(':') {
                let value = line[idx + 1..].trim();
                if current_security.is_empty() {
                    current_security = value.to_string();
                } else {
                    current_security = format!("{}/{}", current_security, value);
                }
            }
        }
    }

    // Add last network
    if !current_bssid.is_empty() {
        networks.push(WifiNetwork {
            ssid: current_ssid,
            bssid: current_bssid.clone(),
            signal_strength: current_signal,
            frequency: format!("Channel {}", current_channel),
            security: current_security,
            trusted: trusted_devices.contains(&current_bssid),
        });
    }

    Ok(networks)
}

/// Fallback WiFi scanning using iwlist (Linux only)
#[cfg(target_os = "linux")]
async fn scan_wifi_iwlist(db_path: &str) -> Result<Vec<WifiNetwork>> {
    let mut networks = Vec::new();

    // Get the wireless interface name
    let iface_output = Command::new("sh")
        .arg("-c")
        .arg("iw dev | grep Interface | awk '{print $2}' | head -1")
        .output()
        .context("Failed to get wireless interface")?;

    let interface = String::from_utf8_lossy(&iface_output.stdout).trim().to_string();
    if interface.is_empty() {
        return Ok(networks); // No wireless interface found
    }

    // Run iwlist scan
    let output = Command::new("iwlist")
        .arg(&interface)
        .arg("scan")
        .output()
        .context("Failed to execute iwlist. May require root privileges.")?;

    if !output.status.success() {
        return Ok(networks); // Return empty list if scan fails
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let trusted_devices = load_trusted_wifi_devices(db_path)?;

    // Parse iwlist output (simplified parser)
    let mut current_network: Option<WifiNetwork> = None;

    for line in output_str.lines() {
        let line = line.trim();

        if line.starts_with("Cell") {
            // Save previous network and start new one
            if let Some(network) = current_network.take() {
                networks.push(network);
            }

            // Extract BSSID from Cell line
            if let Some(addr_pos) = line.find("Address: ") {
                let bssid = line[addr_pos + 9..].trim().to_uppercase();
                current_network = Some(WifiNetwork {
                    ssid: String::new(),
                    bssid: bssid.clone(),
                    signal_strength: -100,
                    frequency: String::new(),
                    security: String::new(),
                    trusted: trusted_devices.contains(&bssid),
                });
            }
        } else if line.starts_with("ESSID:") {
            if let Some(ref mut network) = current_network {
                network.ssid = line.replace("ESSID:", "").replace("\"", "").trim().to_string();
            }
        } else if line.starts_with("Quality=") || line.contains("Signal level=") {
            if let Some(ref mut network) = current_network {
                // Extract signal strength
                if let Some(level_pos) = line.find("Signal level=") {
                    let level_str = &line[level_pos + 13..];
                    if let Some(dbm_str) = level_str.split_whitespace().next() {
                        network.signal_strength = dbm_str.parse::<i32>().unwrap_or(-100);
                    }
                }
            }
        } else if line.starts_with("Frequency:") {
            if let Some(ref mut network) = current_network {
                network.frequency = line.replace("Frequency:", "").trim().to_string();
            }
        } else if line.contains("Encryption key:on") || line.contains("WPA") || line.contains("WEP") {
            if let Some(ref mut network) = current_network {
                if network.security.is_empty() {
                    network.security = "Encrypted".to_string();
                }
            }
        }
    }

    // Add the last network
    if let Some(network) = current_network {
        networks.push(network);
    }

    Ok(networks)
}

/// Scan for Bluetooth devices
#[tauri::command]
pub async fn scan_bluetooth_devices(db_path: String) -> Result<Vec<BluetoothDevice>, String> {
    scan_bluetooth_internal(&db_path)
        .await
        .map_err(|e| format!("Bluetooth scan failed: {}", e))
}

async fn scan_bluetooth_internal(db_path: &str) -> Result<Vec<BluetoothDevice>> {
    let mut devices = Vec::new();

    // Initialize the Bluetooth manager
    let manager = Manager::new().await.context("Failed to initialize Bluetooth manager")?;

    // Get the first Bluetooth adapter
    let adapters = manager.adapters().await.context("Failed to get Bluetooth adapters")?;
    let adapter = adapters.into_iter().next().context("No Bluetooth adapters found")?;

    // Start scanning for devices
    adapter.start_scan(ScanFilter::default()).await.context("Failed to start BLE scan")?;

    // Scan for 5 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Get discovered devices
    let peripherals = adapter.peripherals().await.context("Failed to get peripherals")?;
    let trusted_devices = load_trusted_bluetooth_devices(db_path)?;

    for peripheral in peripherals {
        if let Ok(Some(props)) = peripheral.properties().await {
            let name = props.local_name.unwrap_or_else(|| "Unknown Device".to_string());
            let address = props.address.to_string();
            let rssi = props.rssi.unwrap_or(-100);

            devices.push(BluetoothDevice {
                name,
                address: address.clone(),
                rssi,
                device_type: "BLE".to_string(),
                trusted: trusted_devices.contains(&address),
            });
        }
    }

    // Stop scanning
    adapter.stop_scan().await.context("Failed to stop BLE scan")?;

    Ok(devices)
}

/// Scan both WiFi and Bluetooth
#[tauri::command]
pub async fn scan_local_network(db_path: String) -> Result<NetworkScanResult, String> {
    let wifi_networks = scan_wifi_internal(&db_path)
        .await
        .unwrap_or_else(|_| Vec::new());

    let bluetooth_devices = scan_bluetooth_internal(&db_path)
        .await
        .unwrap_or_else(|_| Vec::new());

    let scan_timestamp = chrono::Utc::now().to_rfc3339();

    Ok(NetworkScanResult {
        wifi_networks,
        bluetooth_devices,
        scan_timestamp,
    })
}

/// Mark a WiFi network as trusted
#[tauri::command]
pub fn trust_wifi_device(db_path: String, bssid: String) -> Result<(), String> {
    use rusqlite::Connection;

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO trusted_wifi_devices (bssid, added_at) VALUES (?1, ?2)",
        rusqlite::params![bssid, chrono::Utc::now().to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Mark a Bluetooth device as trusted
#[tauri::command]
pub fn trust_bluetooth_device(db_path: String, address: String) -> Result<(), String> {
    use rusqlite::Connection;

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO trusted_bluetooth_devices (address, added_at) VALUES (?1, ?2)",
        rusqlite::params![address, chrono::Utc::now().to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove trust from a WiFi network
#[tauri::command]
pub fn untrust_wifi_device(db_path: String, bssid: String) -> Result<(), String> {
    use rusqlite::Connection;

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM trusted_wifi_devices WHERE bssid = ?1",
        rusqlite::params![bssid],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove trust from a Bluetooth device
#[tauri::command]
pub fn untrust_bluetooth_device(db_path: String, address: String) -> Result<(), String> {
    use rusqlite::Connection;

    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM trusted_bluetooth_devices WHERE address = ?1",
        rusqlite::params![address],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// Helper functions

fn load_trusted_wifi_devices(db_path: &str) -> Result<Vec<String>> {
    use rusqlite::Connection;

    let conn = Connection::open(db_path)?;

    // Ensure table exists (idempotent migration)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS trusted_wifi_devices (
            bssid TEXT PRIMARY KEY,
            ssid TEXT,
            notes TEXT,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_seen TEXT
        )",
        [],
    )?;

    let mut stmt = conn.prepare("SELECT bssid FROM trusted_wifi_devices")?;
    let devices = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(devices)
}

fn load_trusted_bluetooth_devices(db_path: &str) -> Result<Vec<String>> {
    use rusqlite::Connection;

    let conn = Connection::open(db_path)?;

    // Ensure table exists (idempotent migration)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS trusted_bluetooth_devices (
            address TEXT PRIMARY KEY,
            name TEXT,
            device_type TEXT,
            notes TEXT,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_seen TEXT
        )",
        [],
    )?;

    let mut stmt = conn.prepare("SELECT address FROM trusted_bluetooth_devices")?;
    let devices = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(devices)
}
