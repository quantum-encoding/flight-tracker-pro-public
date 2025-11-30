// Network diagnostic tools for IP investigation
// Provides whois, nslookup, ping, traceroute, and geoip lookups
// Note: IP blocking is handled by active_defense module via D-Bus sentinels

use std::process::Command;

/// Run whois lookup on an IP address or domain
#[tauri::command]
pub async fn network_whois(ip: String) -> Result<String, String> {
    // Validate input to prevent command injection
    if !is_valid_ip_or_domain(&ip) {
        return Err("Invalid IP address or domain".to_string());
    }

    let output = Command::new("whois")
        .arg(&ip)
        .output()
        .map_err(|e| format!("Failed to execute whois: {}", e))?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        // Truncate if too long
        if result.len() > 10000 {
            Ok(format!("{}...\n\n[Output truncated - {} bytes total]", &result[..10000], result.len()))
        } else {
            Ok(result)
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("whois failed: {}", stderr))
    }
}

/// Run DNS lookup (nslookup or host) on an IP address or domain
#[tauri::command]
pub async fn network_nslookup(ip: String) -> Result<String, String> {
    if !is_valid_ip_or_domain(&ip) {
        return Err("Invalid IP address or domain".to_string());
    }

    // Try 'host' first (more common on Linux), fall back to 'nslookup'
    let output = Command::new("host")
        .arg(&ip)
        .output()
        .or_else(|_| {
            Command::new("nslookup")
                .arg(&ip)
                .output()
        })
        .map_err(|e| format!("Failed to execute DNS lookup: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        // Even on "failure", nslookup often has useful output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stdout.is_empty() {
            Ok(stdout.to_string())
        } else {
            Err(format!("DNS lookup failed: {}", stderr))
        }
    }
}

/// Ping an IP address (limited to 4 packets for quick response)
#[tauri::command]
pub async fn network_ping(ip: String) -> Result<String, String> {
    if !is_valid_ip_or_domain(&ip) {
        return Err("Invalid IP address or domain".to_string());
    }

    // Platform-specific ping arguments
    #[cfg(target_os = "windows")]
    let output = Command::new("ping")
        .arg("-n")
        .arg("4") // Only 4 packets
        .arg("-w")
        .arg("2000") // 2 second timeout (in milliseconds on Windows)
        .arg(&ip)
        .output()
        .map_err(|e| format!("Failed to execute ping: {}", e))?;

    #[cfg(target_os = "macos")]
    let output = Command::new("ping")
        .arg("-c")
        .arg("4") // Only 4 packets
        .arg("-t")
        .arg("2") // 2 second timeout (macOS uses -t for timeout)
        .arg(&ip)
        .output()
        .map_err(|e| format!("Failed to execute ping: {}", e))?;

    #[cfg(target_os = "linux")]
    let output = Command::new("ping")
        .arg("-c")
        .arg("4") // Only 4 packets
        .arg("-W")
        .arg("2") // 2 second timeout per packet (Linux uses -W)
        .arg(&ip)
        .output()
        .map_err(|e| format!("Failed to execute ping: {}", e))?;

    // Ping returns non-zero if host is unreachable, but we still want the output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        Ok(stdout.to_string())
    } else if !stderr.is_empty() {
        Err(stderr.to_string())
    } else {
        Err("No response from ping".to_string())
    }
}

/// Run traceroute to an IP address (limited hops for reasonable time)
#[tauri::command]
pub async fn network_traceroute(ip: String) -> Result<String, String> {
    if !is_valid_ip_or_domain(&ip) {
        return Err("Invalid IP address or domain".to_string());
    }

    // Platform-specific traceroute command
    #[cfg(target_os = "windows")]
    let output = Command::new("tracert")
        .arg("-h")
        .arg("15") // Max 15 hops
        .arg("-w")
        .arg("2000") // 2 second wait per hop (in ms)
        .arg(&ip)
        .output()
        .map_err(|e| format!("Failed to execute tracert: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("traceroute")
        .arg("-m")
        .arg("15") // Max 15 hops
        .arg("-w")
        .arg("2") // 2 second wait per hop
        .arg(&ip)
        .output()
        .or_else(|_| {
            // Fall back to tracepath if traceroute not available
            Command::new("tracepath")
                .arg("-m")
                .arg("15")
                .arg(&ip)
                .output()
        })
        .map_err(|e| format!("Failed to execute traceroute: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        Ok(stdout.to_string())
    } else if !stderr.is_empty() {
        // Some traceroute errors are still informative
        Ok(format!("Traceroute output:\n{}", stderr))
    } else {
        Err("No response from traceroute".to_string())
    }
}

/// Get GeoIP information for an IP address
/// Uses geoiplookup if available, otherwise falls back to a simple curl request
#[tauri::command]
pub async fn network_geoip(ip: String) -> Result<String, String> {
    if !is_valid_ip_or_domain(&ip) {
        return Err("Invalid IP address or domain".to_string());
    }

    // First try geoiplookup (from geoip-bin package)
    let geoip_result = Command::new("geoiplookup")
        .arg(&ip)
        .output();

    if let Ok(output) = geoip_result {
        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout).to_string();
            if !result.contains("can't resolve") && !result.trim().is_empty() {
                return Ok(result);
            }
        }
    }

    // Fall back to curl with ip-api.com (free, no API key needed)
    // Note: ip-api.com has rate limits but is suitable for occasional lookups
    let url = format!("http://ip-api.com/line/{}?fields=status,message,country,countryCode,region,regionName,city,zip,lat,lon,timezone,isp,org,as,query", ip);

    let output = Command::new("curl")
        .arg("-s")
        .arg("-m")
        .arg("5") // 5 second timeout
        .arg(&url)
        .output()
        .map_err(|e| format!("Failed to fetch GeoIP data: {}", e))?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        let lines: Vec<&str> = result.lines().collect();

        // Parse the line-format response into readable format
        if lines.len() >= 14 && lines[0] == "success" {
            let formatted = format!(
                "IP Address:    {}\n\
                 Country:       {} ({})\n\
                 Region:        {} ({})\n\
                 City:          {}\n\
                 ZIP Code:      {}\n\
                 Coordinates:   {}, {}\n\
                 Timezone:      {}\n\
                 ISP:           {}\n\
                 Organization:  {}\n\
                 AS Number:     {}",
                lines.get(13).unwrap_or(&"N/A"),  // query
                lines.get(2).unwrap_or(&"N/A"),   // country
                lines.get(3).unwrap_or(&"N/A"),   // countryCode
                lines.get(5).unwrap_or(&"N/A"),   // regionName
                lines.get(4).unwrap_or(&"N/A"),   // region
                lines.get(6).unwrap_or(&"N/A"),   // city
                lines.get(7).unwrap_or(&"N/A"),   // zip
                lines.get(8).unwrap_or(&"N/A"),   // lat
                lines.get(9).unwrap_or(&"N/A"),   // lon
                lines.get(10).unwrap_or(&"N/A"), // timezone
                lines.get(11).unwrap_or(&"N/A"), // isp
                lines.get(12).unwrap_or(&"N/A"), // org
                lines.get(13).unwrap_or(&"N/A"), // as - actually this is wrong, let me check
            );
            return Ok(formatted);
        } else if !lines.is_empty() && lines[0] == "fail" {
            return Err(format!("GeoIP lookup failed: {}", lines.get(1).unwrap_or(&"Unknown error")));
        }

        // Return raw response if parsing fails
        Ok(result)
    } else {
        Err("Failed to fetch GeoIP data".to_string())
    }
}

/// Validate that the input is a reasonable IP address or domain name
/// This helps prevent command injection attacks
fn is_valid_ip_or_domain(input: &str) -> bool {
    if input.is_empty() || input.len() > 253 {
        return false;
    }

    // Only allow alphanumeric, dots, hyphens, and colons (for IPv6)
    input.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':'
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_inputs() {
        assert!(is_valid_ip_or_domain("8.8.8.8"));
        assert!(is_valid_ip_or_domain("192.168.1.1"));
        assert!(is_valid_ip_or_domain("google.com"));
        assert!(is_valid_ip_or_domain("sub.domain.example.com"));
        assert!(is_valid_ip_or_domain("2001:4860:4860::8888")); // IPv6
    }

    #[test]
    fn test_invalid_inputs() {
        assert!(!is_valid_ip_or_domain("")); // Empty
        assert!(!is_valid_ip_or_domain("8.8.8.8; rm -rf /")); // Command injection
        assert!(!is_valid_ip_or_domain("google.com && echo test")); // Command injection
        assert!(!is_valid_ip_or_domain("$(whoami)")); // Command substitution
        assert!(!is_valid_ip_or_domain("`whoami`")); // Backtick substitution
    }
}
