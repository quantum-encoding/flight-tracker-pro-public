// OCR module for boarding pass and travel document analysis using Google Gemini API

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrFlightResult {
    pub flight_number: Option<String>,
    pub departure_airport: Option<String>,
    pub arrival_airport: Option<String>,
    pub departure_datetime: Option<String>,
    pub arrival_datetime: Option<String>,
    pub passenger_name: Option<String>,
    pub booking_reference: Option<String>,
    pub ticket_number: Option<String>,
    pub seat_number: Option<String>,
    pub fare_class: Option<String>,
    pub gate: Option<String>,
    pub terminal: Option<String>,
    pub aircraft_type: Option<String>,
    pub airline: Option<String>,
    pub frequent_flyer_number: Option<String>,
}

/// Analyze a boarding pass or travel document using Gemini API
pub async fn analyze_with_gemini(
    image_bytes: Vec<u8>,
    api_key: &str,
    use_lite_model: bool,
) -> Result<OcrFlightResult> {
    // Convert image bytes to base64
    let image_base64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_bytes);

    // Choose model based on preference
    let model = if use_lite_model {
        "gemini-2.5-flash-lite"
    } else {
        "gemini-3-pro-preview"
    };

    // Construct the Gemini API request
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    // Create the request payload with detailed OCR prompt
    let payload = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "text": r#"You are an expert travel document analyzer. Analyze this boarding pass, ticket, or travel document image and extract the following flight information in valid JSON format only, with no other text:

{
  "flight_number": "string (e.g., 'AA1234', 'UA567')",
  "departure_airport": "string (IATA code like 'JFK', 'LAX', or full name)",
  "arrival_airport": "string (IATA code like 'JFK', 'LAX', or full name)",
  "departure_datetime": "string (ISO 8601 format YYYY-MM-DDTHH:MM:SS or best approximation)",
  "arrival_datetime": "string (ISO 8601 format YYYY-MM-DDTHH:MM:SS or best approximation)",
  "passenger_name": "string (passenger name)",
  "booking_reference": "string (PNR/booking reference)",
  "ticket_number": "string (ticket number if shown)",
  "seat_number": "string (e.g., '12A', '23F')",
  "fare_class": "string (Economy, Business, First, Premium Economy, etc.)",
  "gate": "string (departure gate)",
  "terminal": "string (terminal)",
  "aircraft_type": "string (e.g., 'Boeing 737', 'A320')",
  "airline": "string (airline name)",
  "frequent_flyer_number": "string (FF number if visible)"
}

IMPORTANT RULES:
1. If a value cannot be found in the document, use null
2. For dates, convert any format to ISO 8601 (YYYY-MM-DDTHH:MM:SS)
3. For airports, prefer IATA codes (3-letter) if available
4. Extract only information that is clearly visible
5. Return ONLY the JSON object, no markdown formatting, no explanation"#
                },
                {
                    "inlineData": {
                        "mimeType": "image/jpeg",
                        "data": image_base64
                    },
                    "mediaResolution": {
                        "level": "media_resolution_high"
                    }
                }
            ]
        }],
        "generationConfig": {
            "maxOutputTokens": 4096,
            "responseMimeType": "application/json"
        }
    });

    // Send request to Gemini API
    let response = client.post(&url).json(&payload).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Gemini API error: {}", error_text));
    }

    // Parse the response
    let response_json: serde_json::Value = response.json().await?;

    // Extract the text from Gemini's response
    let text = response_json
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract text from Gemini response"))?;

    // Parse the JSON from the response text
    // Gemini might wrap the JSON in markdown code blocks, so clean it
    let json_text = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse the JSON into OcrFlightResult
    let result: OcrFlightResult = serde_json::from_str(json_text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse Gemini JSON response: {}. Raw text: {}",
            e,
            json_text
        )
    })?;

    Ok(result)
}

/// Analyze with intelligent retry logic for rate limiting
pub async fn analyze_with_retry(
    image_bytes: Vec<u8>,
    api_key: &str,
    use_lite_model: bool,
    max_retries: u32,
) -> Result<OcrFlightResult> {
    let mut retry_count = 0;
    let mut base_delay = 2000; // Start with 2 seconds

    loop {
        match analyze_with_gemini(image_bytes.clone(), api_key, use_lite_model).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let error_msg = e.to_string();

                // Check if it's a rate limiting error (429)
                let is_rate_limit = error_msg.contains("429")
                    || error_msg.contains("rate limit")
                    || error_msg.contains("quota");

                if is_rate_limit && retry_count < max_retries {
                    retry_count += 1;

                    // Exponential backoff with jitter
                    let jitter = (rand::random::<f64>() * 1000.0) as u64;
                    let delay = base_delay + jitter;

                    eprintln!(
                        "Rate limit hit (429), retry {}/{} after {}ms",
                        retry_count, max_retries, delay
                    );

                    sleep(Duration::from_millis(delay)).await;

                    // Exponential backoff: double the delay each time
                    base_delay *= 2;

                    continue;
                } else {
                    // Not a rate limit error or max retries reached
                    return Err(e);
                }
            }
        }
    }
}

/// Batch process multiple images with parallel execution
/// Returns results in the same order as input paths
pub async fn batch_analyze(
    image_paths: Vec<String>,
    api_key: &str,
    use_lite_model: bool,
) -> Vec<Result<OcrFlightResult>> {
    use futures::stream::{FuturesOrdered, StreamExt};
    use std::sync::Arc;

    let api_key = Arc::new(api_key.to_string());

    // Create ordered futures for parallel processing
    let mut futures = FuturesOrdered::new();

    for path in image_paths {
        let api_key = Arc::clone(&api_key);
        let future = tokio::spawn(async move {
            let image_result = std::fs::read(&path);

            match image_result {
                Ok(image_bytes) => {
                    analyze_with_gemini(image_bytes, &api_key, use_lite_model).await
                }
                Err(e) => Err(anyhow::anyhow!("Failed to read image {}: {}", path, e)),
            }
        });

        futures.push_back(future);
    }

    // Collect results in order
    let mut results = Vec::new();
    while let Some(result) = futures.next().await {
        match result {
            Ok(ocr_result) => results.push(ocr_result),
            Err(e) => results.push(Err(anyhow::anyhow!("Task panicked: {}", e))),
        }
    }

    results
}

/// Batch process with progress callback
/// Emits progress events via Tauri
pub async fn batch_analyze_with_progress<R: tauri::Runtime>(
    image_paths: Vec<String>,
    api_key: &str,
    use_lite_model: bool,
    app_handle: &tauri::AppHandle<R>,
) -> Vec<Result<OcrFlightResult>> {
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let api_key = Arc::new(api_key.to_string());
    let total = image_paths.len();
    let semaphore = Arc::new(Semaphore::new(3)); // Max 3 concurrent requests

    let mut handles = Vec::new();

    for (index, path) in image_paths.into_iter().enumerate() {
        let api_key = Arc::clone(&api_key);
        let app_handle = app_handle.clone();
        let semaphore = Arc::clone(&semaphore);

        let handle = tokio::spawn(async move {
            // Acquire semaphore permit
            let _permit = semaphore.acquire().await.unwrap();

            // Emit progress: starting
            let _ = app_handle.emit("batch-ocr:progress", serde_json::json!({
                "current": index + 1,
                "total": total,
                "status": "processing",
                "path": path.clone()
            }));

            let image_result = std::fs::read(&path);

            let result = match image_result {
                Ok(image_bytes) => {
                    analyze_with_gemini(image_bytes, &api_key, use_lite_model).await
                }
                Err(e) => Err(anyhow::anyhow!("Failed to read image {}: {}", path, e)),
            };

            // Emit progress: complete
            let status = if result.is_ok() { "success" } else { "error" };
            let _ = app_handle.emit("batch-ocr:progress", serde_json::json!({
                "current": index + 1,
                "total": total,
                "status": status,
                "path": path
            }));

            (index, result)
        });

        handles.push(handle);
    }

    // Wait for all tasks and collect results in order
    let mut indexed_results = Vec::new();
    for handle in handles {
        if let Ok((index, result)) = handle.await {
            indexed_results.push((index, result));
        }
    }

    // Sort by index to maintain order
    indexed_results.sort_by_key(|(index, _)| *index);
    indexed_results.into_iter().map(|(_, result)| result).collect()
}

/// Extract all text from a document/image using Gemini 2.5 Flash Lite (cheap model)
/// Returns the extracted text as a plain string
pub async fn extract_document_text(
    image_bytes: Vec<u8>,
    api_key: &str,
) -> Result<String> {
    // Convert image bytes to base64
    let image_base64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_bytes);

    // Use flash-lite for fast, cost-effective document OCR
    let model = "gemini-2.5-flash-lite";

    // Construct the Gemini API request
    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    // Create the request payload with OCR prompt
    let payload = serde_json::json!({
        "contents": [{
            "parts": [
                {
                    "text": "Extract all text from this document/image. Return only the extracted text, preserving the structure and formatting as much as possible. Do not add any commentary or explanation."
                },
                {
                    "inlineData": {
                        "mimeType": "image/jpeg",
                        "data": image_base64
                    },
                    "mediaResolution": {
                        "level": "media_resolution_medium"
                    }
                }
            ]
        }],
        "generationConfig": {
            "maxOutputTokens": 8192
        }
    });

    // Send request to Gemini API
    let response = client.post(&url).json(&payload).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Gemini API error: {}", error_text));
    }

    // Parse the response
    let response_json: serde_json::Value = response.json().await?;

    // Extract the text from Gemini's response
    let text = response_json
        .get("candidates")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract text from Gemini response"))?;

    Ok(text.to_string())
}

/// Extract text from document with retry logic for rate limiting
pub async fn extract_document_text_with_retry(
    image_bytes: Vec<u8>,
    api_key: &str,
    max_retries: u32,
) -> Result<String> {
    let mut retry_count = 0;
    let mut base_delay = 2000; // Start with 2 seconds

    loop {
        match extract_document_text(image_bytes.clone(), api_key).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let error_msg = e.to_string();

                // Check if it's a rate limiting error (429)
                let is_rate_limit = error_msg.contains("429")
                    || error_msg.contains("rate limit")
                    || error_msg.contains("quota");

                if is_rate_limit && retry_count < max_retries {
                    retry_count += 1;

                    // Exponential backoff with jitter
                    let jitter = (rand::random::<f64>() * 1000.0) as u64;
                    let delay = base_delay + jitter;

                    eprintln!(
                        "Rate limit hit (429), retry {}/{} after {}ms",
                        retry_count, max_retries, delay
                    );

                    sleep(Duration::from_millis(delay)).await;

                    // Exponential backoff: double the delay each time
                    base_delay *= 2;

                    continue;
                } else {
                    // Not a rate limit error or max retries reached
                    return Err(e);
                }
            }
        }
    }
}
