// Donation and support commands for Flight Tracker Pro
// Generates QR codes for crypto addresses and manages donation configuration

use qrcode::QrCode;
use qrcode::render::svg;

/// Donation configuration - addresses for various payment methods
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DonationConfig {
    pub btc_address: String,
    pub eth_address: String,
    pub sol_address: String,
    pub usdt_address: String,
    pub xrp_address: String,
    pub stripe_link: String,
    pub github_sponsors: String,
    pub message: String,
}

impl Default for DonationConfig {
    fn default() -> Self {
        Self {
            btc_address: "bc1q77yqen5tark5f8g0nujvttqmjfw0aty2fpg4p3".to_string(),
            eth_address: "0xc890685a6755e9D081Ea4F7E3C3beC2EC3582CfA".to_string(),
            sol_address: "2qVp5Z82ecQ4cbwgcdDT2aHHsAmi64FtvjGc4i2DvFdE".to_string(),
            usdt_address: "0xc890685a6755e9D081Ea4F7E3C3beC2EC3582CfA".to_string(), // ERC-20 (same as ETH)
            xrp_address: "r3aHuSTG3i6T3z5GeaiBNEN9yYan6svCgx".to_string(),
            stripe_link: "https://buy.stripe.com/8x25kD0dD8hP8wK63G4ko00".to_string(),
            github_sponsors: "https://github.com/sponsors/quantum-encoding".to_string(),
            message: "Flight Tracker Pro is free and open-source. Your support helps keep development active!".to_string(),
        }
    }
}

/// Generate a QR code SVG for a given address/URL
#[tauri::command]
pub fn generate_qr_code(data: String, size: Option<u32>) -> Result<String, String> {
    let code = QrCode::new(data.as_bytes())
        .map_err(|e| format!("Failed to generate QR code: {}", e))?;

    let pixel_size = size.unwrap_or(4);

    let svg_string = code.render()
        .min_dimensions(200, 200)
        .dark_color(svg::Color("#ffffff"))
        .light_color(svg::Color("#00000000")) // Transparent background
        .quiet_zone(true)
        .build();

    Ok(svg_string)
}

/// Generate a QR code with custom colors (for themed UI)
#[tauri::command]
pub fn generate_qr_code_themed(
    data: String,
    fg_color: String,
    bg_color: String,
) -> Result<String, String> {
    let code = QrCode::new(data.as_bytes())
        .map_err(|e| format!("Failed to generate QR code: {}", e))?;

    let svg_string = code.render()
        .min_dimensions(200, 200)
        .dark_color(svg::Color(&fg_color))
        .light_color(svg::Color(&bg_color))
        .quiet_zone(true)
        .build();

    Ok(svg_string)
}

/// Get donation configuration
/// In production, this could fetch from a remote endpoint for updatable addresses
#[tauri::command]
pub fn get_donation_config() -> Result<DonationConfig, String> {
    // For now, return default config
    // In future: could fetch from https://quantumencoding.io/donate-config.json
    Ok(DonationConfig::default())
}

/// Record a donation event (for analytics/thank you messages)
#[tauri::command]
pub fn record_donation_click(method: String) -> Result<(), String> {
    // Log the click for analytics (privacy-respecting, local only)
    println!("Donation click recorded: {}", method);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_generation() {
        let result = generate_qr_code("test_address".to_string(), None);
        assert!(result.is_ok());
        let svg = result.unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_donation_config() {
        let config = DonationConfig::default();
        assert!(!config.btc_address.is_empty());
        assert!(!config.eth_address.is_empty());
    }
}
