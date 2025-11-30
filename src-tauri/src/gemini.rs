// Gemini chat integration for Flight Tracker Pro
// Simple chat interface without tool calling

use anyhow::Result;
use serde::{Deserialize, Serialize};

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiChatResult {
    pub content: String,
    pub tokens_used: Option<u32>,
}

/// Simple chat with Gemini (no tool calling, just conversation)
pub async fn chat_with_gemini(
    query: &str,
    api_key: &str,
    model: &str, // "gemini-3-pro-preview", "gemini-2.5-flash-lite", etc.
) -> Result<GeminiChatResult> {
    chat_with_gemini_custom(query, api_key, model, 16384).await
}

/// Chat with Gemini with custom token limit
pub async fn chat_with_gemini_custom(
    query: &str,
    api_key: &str,
    model: &str,
    max_output_tokens: u32,
) -> Result<GeminiChatResult> {
    let client = reqwest::Client::new();

    // Build API endpoint URL
    let api_url = format!(
        "{}/models/{}:generateContent?key={}",
        GEMINI_API_BASE, model, api_key
    );

    // Build request payload
    let payload = serde_json::json!({
        "contents": [{
            "role": "user",
            "parts": [{"text": query}]
        }],
        "generationConfig": {
            "temperature": 1.0,
            "maxOutputTokens": max_output_tokens,
            "topP": 0.95
        }
    });

    // Call Gemini API
    let response = client
        .post(&api_url)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    // Check HTTP status
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "Gemini API error ({}): {}",
            status,
            error_text
        ));
    }

    // Parse response
    let response_json: serde_json::Value = response.json().await?;

    // Track tokens
    let tokens_used = response_json
        .get("usageMetadata")
        .and_then(|u| u.get("totalTokenCount"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    // Get candidate response
    let candidate = response_json["candidates"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| anyhow::anyhow!("No candidates in response"))?;

    let parts = candidate["content"]["parts"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Invalid response structure"))?;

    // Extract text content
    let content = parts
        .iter()
        .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(GeminiChatResult {
        content,
        tokens_used,
    })
}
