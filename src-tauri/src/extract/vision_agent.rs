// Vision Agent Module
// Sends page images to Google Gemini for handwritten flight log extraction

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

/// Extracted flight log entry - CSV-ready format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightLogEntry {
    /// Date in YYYY-MM-DD format
    #[serde(default)]
    pub date: Option<String>,
    /// Departure airport code (3-4 letters)
    #[serde(default, alias = "departure", alias = "origin", alias = "departure_airport")]
    pub from: Option<String>,
    /// Arrival airport code (3-4 letters)
    #[serde(default, alias = "arrival", alias = "destination", alias = "arrival_airport")]
    pub to: Option<String>,
    /// Aircraft tail number (e.g., N12516)
    #[serde(default, alias = "tail_number", alias = "registration")]
    pub aircraft_registration: Option<String>,
    /// Passengers, semicolon-separated
    #[serde(default)]
    pub passengers: Option<String>,
    /// Flight number if present
    #[serde(default)]
    pub flight_number: Option<String>,
    /// Page number this entry came from (added during processing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_page: Option<u32>,
}

/// Result of processing a single page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageExtractionResult {
    pub page_number: u32,
    pub image_path: String,
    pub entries: Vec<FlightLogEntry>,
    pub raw_response: Option<String>,
    pub error: Option<String>,
}

/// Configuration for the vision agent
#[derive(Debug, Clone)]
pub struct VisionAgentConfig {
    /// Gemini API key
    pub api_key: String,
    /// Model to use (default: gemini-1.5-flash)
    pub model: String,
    /// Maximum tokens in response
    pub max_tokens: u32,
    /// Temperature for generation (0.0 - 1.0)
    pub temperature: f32,
}

impl Default for VisionAgentConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "gemini-2.5-flash-lite".to_string(),
            max_tokens: 8192,
            temperature: 0.1, // Low temperature for structured extraction
        }
    }
}

/// The extraction prompt for Gemini - outputs CSV-ready format
const EXTRACTION_PROMPT: &str = r#"Extract flight log entries from this scanned handwritten page.

OUTPUT FORMAT: Return ONLY a JSON array. Each entry must have these exact fields for CSV import:
{
  "date": "YYYY-MM-DD",
  "from": "ABC",
  "to": "XYZ",
  "aircraft_registration": "N12345",
  "passengers": "Name1; Name2",
  "flight_number": "123"
}

CRITICAL DATE HANDLING:
The DATE column has a SPECIAL FORMAT:
- The YEAR and MONTH appear at the TOP of the column (e.g., "1991 JUL" or "JUL 1998")
- Individual rows only show the DAY NUMBER (e.g., "25", "30", "3", "5")
- Dates DESCEND chronologically - when you see a smaller day number after a larger one, the MONTH has changed
- Example: If header says "JUN" and rows are: 25, 30, JUL 3, 5, 9 → dates are Jun 25, Jun 30, Jul 3, Jul 5, Jul 9
- Sometimes a new month marker appears inline (like "JUL" or "AUG" written in the date column)
- ALWAYS output full YYYY-MM-DD format by combining the context year/month with the day

FIELD RULES:
- date: MUST be YYYY-MM-DD format. Use the year/month from column header + day from each row
- from/to: 3-4 letter airport codes, UPPERCASE (e.g., PSP, CMH, BKL, ATW, ILG, TEB)
- aircraft_registration: US tail number starting with N (e.g., N12516, N404CB)
- passengers: Names from remarks column, semicolon-separated. Extract ALL names mentioned.
- flight_number: If present in FLT.NO. column

READING HANDWRITING:
- Tail numbers: N + digits + optional letters. Common confusions: 0/O, 1/I, 5/S, 8/B
- The "REMARKS" column contains passenger names and notes - extract passenger names
- If a row has FROM and TO airports, include it even if other fields are unclear
- Skip header rows and summary/total rows
- Process entries in ORDER from top to bottom of the page

Return ONLY the JSON array, no markdown or explanation:"#;

/// Vision agent for extracting flight logs using Gemini
pub struct VisionAgent {
    config: VisionAgentConfig,
    client: reqwest::Client,
}

impl VisionAgent {
    pub fn new(config: VisionAgentConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Create with just an API key using default settings
    pub fn with_api_key(api_key: String) -> Self {
        Self::new(VisionAgentConfig {
            api_key,
            ..Default::default()
        })
    }

    /// Extract flight log entries from an image
    pub async fn extract_from_image(&self, image_path: &Path, page_number: u32) -> Result<PageExtractionResult> {
        // Read and encode image as base64
        let image_data = fs::read(image_path)
            .await
            .context("Failed to read image file")?;

        let base64_image = BASE64.encode(&image_data);

        // Determine MIME type from extension
        let mime_type = match image_path.extension().and_then(|e| e.to_str()) {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("tiff") | Some("tif") => "image/tiff",
            Some("webp") => "image/webp",
            _ => "image/png",
        };

        // Build Gemini API request
        let request_body = serde_json::json!({
            "contents": [{
                "parts": [
                    {
                        "text": EXTRACTION_PROMPT
                    },
                    {
                        "inline_data": {
                            "mime_type": mime_type,
                            "data": base64_image
                        }
                    }
                ]
            }],
            "generationConfig": {
                "temperature": self.config.temperature,
                "maxOutputTokens": self.config.max_tokens,
                "responseMimeType": "application/json"
            }
        });

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.config.model, self.config.api_key
        );

        // Send request
        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to send request to Gemini API")?;

        let status = response.status();
        let response_text = response.text().await?;

        if !status.is_success() {
            return Ok(PageExtractionResult {
                page_number,
                image_path: image_path.to_string_lossy().to_string(),
                entries: vec![],
                raw_response: Some(response_text.clone()),
                error: Some(format!("API error ({}): {}", status, response_text)),
            });
        }

        // Parse response
        let response_json: serde_json::Value = serde_json::from_str(&response_text)
            .context("Failed to parse Gemini response")?;

        // Extract text content from Gemini response
        let text_content = response_json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("");

        // Parse the JSON array from the response
        let entries = self.parse_entries(text_content, page_number);

        Ok(PageExtractionResult {
            page_number,
            image_path: image_path.to_string_lossy().to_string(),
            entries,
            raw_response: Some(text_content.to_string()),
            error: None,
        })
    }

    /// Parse flight entries from JSON text
    fn parse_entries(&self, text: &str, page_number: u32) -> Vec<FlightLogEntry> {
        // Try to extract JSON array from the response
        let json_text = self.extract_json_array(text);

        match serde_json::from_str::<Vec<FlightLogEntry>>(&json_text) {
            Ok(mut entries) => {
                // Add source page to each entry
                for entry in &mut entries {
                    entry.source_page = Some(page_number);
                }
                entries
            }
            Err(e) => {
                eprintln!("Failed to parse entries from page {}: {}", page_number, e);
                eprintln!("Raw text: {}", &json_text[..json_text.len().min(500)]);
                vec![]
            }
        }
    }

    /// Extract JSON array from potentially wrapped response
    fn extract_json_array(&self, text: &str) -> String {
        let text = text.trim();

        // If it starts with [, assume it's already a JSON array
        if text.starts_with('[') {
            return text.to_string();
        }

        // Try to find JSON array in markdown code blocks
        if let Some(start) = text.find("```json") {
            if let Some(end) = text[start..].find("```\n").or_else(|| text[start..].rfind("```")) {
                let json_start = start + 7; // Skip "```json"
                let json_end = start + end;
                if json_start < json_end {
                    return text[json_start..json_end].trim().to_string();
                }
            }
        }

        // Try to find raw JSON array
        if let Some(start) = text.find('[') {
            if let Some(end) = text.rfind(']') {
                if start < end {
                    return text[start..=end].to_string();
                }
            }
        }

        // Return original if no array found
        text.to_string()
    }
}

/// Process multiple images concurrently with rate limiting
pub async fn process_images_concurrent(
    agent: &VisionAgent,
    image_paths: Vec<(std::path::PathBuf, u32)>, // (path, page_number)
    concurrency: usize,
) -> Vec<PageExtractionResult> {
    use tokio::sync::Semaphore;
    use std::sync::Arc;

    let semaphore = Arc::new(Semaphore::new(concurrency));
    let mut handles = Vec::new();

    for (path, page_num) in image_paths {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let agent_config = agent.config.clone();
        let path_owned = path.clone();

        let handle = tokio::spawn(async move {
            let agent = VisionAgent::new(agent_config);
            let result = agent.extract_from_image(&path_owned, page_num).await;
            drop(permit); // Release semaphore

            match result {
                Ok(r) => r,
                Err(e) => PageExtractionResult {
                    page_number: page_num,
                    image_path: path_owned.to_string_lossy().to_string(),
                    entries: vec![],
                    raw_response: None,
                    error: Some(e.to_string()),
                },
            }
        });

        handles.push(handle);
    }

    // Collect results
    let mut results = Vec::new();
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }

    // Sort by page number
    results.sort_by_key(|r| r.page_number);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_array() {
        let agent = VisionAgent::with_api_key("test".to_string());

        // Test raw array
        let input = r#"[{"date": "25"}]"#;
        assert_eq!(agent.extract_json_array(input), input);

        // Test with markdown
        let input = "```json\n[{\"date\": \"25\"}]\n```";
        assert!(agent.extract_json_array(input).contains("["));

        // Test with surrounding text
        let input = "Here are the entries: [{\"date\": \"25\"}] end";
        assert!(agent.extract_json_array(input).starts_with('['));
    }
}
