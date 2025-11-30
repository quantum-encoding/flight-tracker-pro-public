// DeepSeek API client using Anthropic API compatibility
// Provides AI-powered research and analysis capabilities

use anyhow::Result;
use serde::{Deserialize, Serialize};

const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/anthropic";
const DEEPSEEK_MODEL: &str = "deepseek-chat";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRequest {
    pub date: String,
    pub location: String,
    pub passenger_names: Vec<String>,
    pub research_news: bool,
    pub research_events: bool,
    pub research_weather: bool,
    pub research_passengers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchResult {
    pub summary: String,
    pub news_stories: Vec<NewsStory>,
    pub events: Vec<Event>,
    pub weather: Option<WeatherInfo>,
    pub passenger_mentions: Vec<PassengerMention>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsStory {
    pub title: String,
    pub summary: String,
    pub source: String,
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub description: String,
    pub venue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherInfo {
    pub condition: String,
    pub temperature: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerMention {
    pub passenger_name: String,
    pub context: String,
    pub source: String,
}

/// Research context around a flight using DeepSeek AI
pub async fn research_flight_context(
    request: ResearchRequest,
    web_search_results: Vec<String>,
    api_key: &str,
) -> Result<ResearchResult> {
    // Build the research prompt
    let mut prompt_parts = vec![
        format!("You are an expert research analyst. Analyze the following information about travel on {} to/from {}.", request.date, request.location),
        String::new(),
        "Web search results:".to_string(),
    ];

    for (idx, result) in web_search_results.iter().enumerate() {
        prompt_parts.push(format!("\n[Source {}]:\n{}", idx + 1, result));
    }

    prompt_parts.push(String::new());
    prompt_parts.push("Based on these search results, provide a comprehensive analysis in JSON format with the following structure:".to_string());
    prompt_parts.push(
        r#"{
  "summary": "A concise 2-3 sentence overview of the most significant events/context",
  "news_stories": [
    {
      "title": "Story headline",
      "summary": "Brief summary",
      "source": "Source name",
      "relevance": 0.0-1.0
    }
  ],
  "events": [
    {
      "name": "Event name",
      "description": "Event details",
      "venue": "Location/venue"
    }
  ],
  "weather": {
    "condition": "Condition",
    "temperature": "Temperature",
    "description": "Weather description"
  },
  "passenger_mentions": [
    {
      "passenger_name": "Name",
      "context": "Context where mentioned",
      "source": "Source"
    }
  ]
}"#
        .to_string(),
    );

    if !request.passenger_names.is_empty() {
        prompt_parts.push(format!(
            "\nPassenger names to research: {}",
            request.passenger_names.join(", ")
        ));
    }

    prompt_parts.push(String::new());
    prompt_parts.push("Focus on:".to_string());
    if request.research_news {
        prompt_parts.push("- Top news stories from that date and location".to_string());
    }
    if request.research_events {
        prompt_parts.push("- Major events, conferences, or gatherings".to_string());
    }
    if request.research_weather {
        prompt_parts.push("- Weather conditions".to_string());
    }
    if request.research_passengers {
        prompt_parts.push("- Any mentions of the passenger names".to_string());
    }

    prompt_parts.push(String::new());
    prompt_parts
        .push("Return ONLY valid JSON, no markdown formatting, no explanation.".to_string());

    let prompt = prompt_parts.join("\n");

    // Call DeepSeek API using Anthropic format
    let client = reqwest::Client::new();
    let url = format!("{}/v1/messages", DEEPSEEK_BASE_URL);

    let payload = serde_json::json!({
        "model": DEEPSEEK_MODEL,
        "max_tokens": 4000,
        "temperature": 0.3,
        "system": "You are an expert research analyst specializing in contextual travel research. Always respond with valid JSON only.",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("DeepSeek API error: {}", error_text));
    }

    let response_json: serde_json::Value = response.json().await?;

    // Extract the content from DeepSeek's response (Anthropic format)
    let content = response_json
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract content from DeepSeek response"))?;

    // Clean potential markdown formatting
    let json_text = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse the JSON response
    let result: ResearchResult = serde_json::from_str(json_text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse DeepSeek JSON response: {}. Raw text: {}",
            e,
            json_text
        )
    })?;

    Ok(result)
}

/// Use DeepSeek for OCR analysis as an alternative to Gemini
pub async fn analyze_boarding_pass(
    image_bytes: Vec<u8>,
    api_key: &str,
) -> Result<crate::ocr::OcrFlightResult> {
    // Convert image bytes to base64
    let _image_base64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_bytes);

    let client = reqwest::Client::new();
    let url = format!("{}/v1/messages", DEEPSEEK_BASE_URL);

    // Note: DeepSeek's Anthropic API doesn't support image type in the compatibility docs
    // This is here for future compatibility if they add it
    // For now, this will return an error and we'll fallback to Gemini

    let payload = serde_json::json!({
        "model": DEEPSEEK_MODEL,
        "max_tokens": 1024,
        "temperature": 0.1,
        "system": "You are an expert travel document analyzer.",
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": r#"Analyze this boarding pass image and extract flight information in valid JSON format only, with no other text:

{
  "flight_number": "string (e.g., 'AA1234')",
  "departure_airport": "string (IATA code)",
  "arrival_airport": "string (IATA code)",
  "departure_datetime": "string (ISO 8601 format)",
  "arrival_datetime": "string (ISO 8601 format)",
  "passenger_name": "string",
  "booking_reference": "string",
  "ticket_number": "string",
  "seat_number": "string",
  "fare_class": "string",
  "gate": "string",
  "terminal": "string",
  "aircraft_type": "string",
  "airline": "string",
  "frequent_flyer_number": "string"
}

Use null for missing values. Return ONLY the JSON object."#
                    }
                    // Image support would go here when DeepSeek adds it to Anthropic API
                    // Currently not supported according to compatibility table
                ]
            }
        ]
    });

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("DeepSeek API error: {}", error_text));
    }

    let response_json: serde_json::Value = response.json().await?;

    let content = response_json
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract content from DeepSeek response"))?;

    let json_text = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let result: crate::ocr::OcrFlightResult = serde_json::from_str(json_text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse DeepSeek JSON response: {}. Raw text: {}",
            e,
            json_text
        )
    })?;

    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekChatResult {
    pub content: String,
    pub tokens_used: Option<u32>,
}

/// Simple chat with DeepSeek (no tool calling, just conversation)
pub async fn chat_with_deepseek(query: &str, api_key: &str) -> Result<DeepSeekChatResult> {
    let client = reqwest::Client::new();

    // Build request payload (Anthropic-compatible format)
    let payload = serde_json::json!({
        "model": DEEPSEEK_MODEL,
        "max_tokens": 8192,
        "messages": [{
            "role": "user",
            "content": query
        }]
    });

    // Call DeepSeek API
    let response = client
        .post(format!("{}/v1/messages", DEEPSEEK_BASE_URL))
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    // Check HTTP status
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "DeepSeek API error ({}): {}",
            status,
            error_text
        ));
    }

    // Parse response
    let response_json: serde_json::Value = response.json().await?;

    // Extract content (Anthropic format)
    let content = response_json
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract content from DeepSeek response"))?
        .to_string();

    // Extract token usage if available
    let tokens_used = response_json
        .get("usage")
        .and_then(|u| u.get("total_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    Ok(DeepSeekChatResult {
        content,
        tokens_used,
    })
}

// ============================================================================
// INVESTIGATION ENGINE - DeepSeek Implementation
// ============================================================================

/// Generate search queries using DeepSeek for investigation purposes
pub async fn generate_investigation_queries(
    passenger_names: &[String],
    location: &str,
    date: &str,
    api_key: &str,
) -> Result<Vec<String>> {
    let names_list = passenger_names.join(", ");

    let prompt = format!(
        r#"You are a world-class investigative journalist and OSINT researcher. Your task is to generate highly specific, targeted search engine queries to find public records, news articles, event listings, social media posts, or official documents.

Given the following information:
- Person/People: {}
- Location: {}
- Date: {}

Generate 8-12 diverse search queries that would help find verifiable evidence of this person's presence at or near this location on or around this date (within 48 hours).

Prioritize queries that would find:
1. Local news articles and press releases
2. Event listings and charity galas
3. Business registrations and legal filings
4. Social media posts (archived or public)
5. Flight manifests or travel records
6. Property records and transactions
7. Court records and legal documents
8. Academic or professional conference attendee lists

Format your response as a JSON array of query strings, each optimized for Google Search. Make queries specific, use quotation marks for exact phrases, and include location and date constraints.

Example format:
["query 1", "query 2", "query 3", ...]

Focus on queries that would return credible, verifiable sources. Avoid speculation.

Return ONLY the JSON array, no other text."#,
        names_list, location, date
    );

    let client = reqwest::Client::new();
    let url = format!("{}/v1/messages", DEEPSEEK_BASE_URL);

    let payload = serde_json::json!({
        "model": DEEPSEEK_MODEL,
        "max_tokens": 2048,
        "temperature": 0.7,
        "system": "You are an expert OSINT investigator. Always respond with valid JSON only, no markdown formatting.",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "DeepSeek API error ({}): {}",
            status,
            error_text
        ));
    }

    let response_json: serde_json::Value = response.json().await?;

    // Extract content from Anthropic format
    let content = response_json
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract content from DeepSeek response"))?;

    // Clean potential markdown formatting
    let json_text = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse the JSON array of queries
    let queries: Vec<String> = serde_json::from_str(json_text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse DeepSeek query generation response: {}. Raw text: {}",
            e,
            json_text
        )
    })?;

    Ok(queries)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvestigationSynthesis {
    pub summary: String,
    pub key_findings: Vec<String>,
    pub corroboration_score: f64,
    pub sources_analysis: Vec<SourceAnalysisItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceAnalysisItem {
    pub source_index: usize,
    pub relevance: String,
    pub credibility: String,
    pub key_quote: String,
}

/// Synthesize investigation results using DeepSeek's advanced reasoning
pub async fn synthesize_investigation(
    passenger_names: &[String],
    location: &str,
    date: &str,
    search_results: &[crate::investigation::SearchResult],
    api_key: &str,
) -> Result<InvestigationSynthesis> {
    let names_list = passenger_names.join(", ");

    // Format search results for the prompt
    let mut search_results_text = String::new();
    for (idx, result) in search_results.iter().enumerate() {
        search_results_text.push_str(&format!(
            "\n[Source {}]\nTitle: {}\nURL: {}\nSnippet: {}\n",
            idx, result.title, result.url, result.snippet
        ));
    }

    let prompt = format!(
        r#"You are an expert investigative analyst synthesizing search results into an evidence report.

Investigation Target:
- Person/People: {}
- Location: {}
- Date: {}

Search Results:
{}

Analyze these search results and provide:

1. A concise summary (2-3 paragraphs) of any corroborating evidence, contradictions, or connections found
2. Extract key quotes or facts from the sources
3. Assess the credibility and relevance of each source
4. Provide an overall corroboration score from 0.0 to 1.0, where:
   - 0.0-0.3: No credible evidence found or contradictory evidence
   - 0.4-0.6: Circumstantial or indirect evidence
   - 0.7-0.9: Strong corroborating evidence from multiple sources
   - 1.0: Direct, verified evidence from official records

Format your response as JSON:
{{
  "summary": "Your detailed analysis here...",
  "key_findings": ["finding 1", "finding 2", ...],
  "corroboration_score": 0.75,
  "sources_analysis": [
    {{
      "source_index": 0,
      "relevance": "high|medium|low",
      "credibility": "official|news|social|unverified",
      "key_quote": "Extracted quote or fact"
    }}
  ]
}}

Be objective and factual. Note when evidence is absent, weak, or contradictory.

Return ONLY valid JSON, no markdown formatting."#,
        names_list, location, date, search_results_text
    );

    let client = reqwest::Client::new();
    let url = format!("{}/v1/messages", DEEPSEEK_BASE_URL);

    let payload = serde_json::json!({
        "model": DEEPSEEK_MODEL,
        "max_tokens": 4096,
        "temperature": 0.3,
        "system": "You are an expert investigative analyst specializing in evidence synthesis and corroboration. Always respond with valid JSON only.",
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "DeepSeek API error ({}): {}",
            status,
            error_text
        ));
    }

    let response_json: serde_json::Value = response.json().await?;

    // Extract content from Anthropic format
    let content = response_json
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract content from DeepSeek response"))?;

    // Clean potential markdown formatting
    let json_text = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse the JSON response
    let synthesis: InvestigationSynthesis = serde_json::from_str(json_text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse DeepSeek synthesis response: {}. Raw text: {}",
            e,
            json_text
        )
    })?;

    Ok(synthesis)
}

/// Complete investigation flow using DeepSeek
pub async fn run_deepseek_investigation(
    passenger_names: Vec<String>,
    location: String,
    date: String,
    api_key: String,
) -> Result<crate::models::InvestigationResult> {
    let start_time = std::time::Instant::now();

    // Step 1: Generate search queries using DeepSeek
    let queries = generate_investigation_queries(&passenger_names, &location, &date, &api_key).await?;

    // Step 2: Execute searches (reuse existing search infrastructure)
    let search_results = crate::investigation::execute_searches(&queries).await?;

    // Step 3: Synthesize results using DeepSeek
    let synthesis = synthesize_investigation(
        &passenger_names,
        &location,
        &date,
        &search_results,
        &api_key,
    )
    .await?;

    let elapsed_ms = start_time.elapsed().as_millis() as i64;

    // Convert to InvestigationResult format matching models.rs structure
    let sources: Vec<crate::models::InvestigationSource> = search_results
        .into_iter()
        .enumerate()
        .filter_map(|(idx, result)| {
            let analysis = synthesis
                .sources_analysis
                .iter()
                .find(|s| s.source_index == idx)?;

            let relevance_score = match analysis.relevance.as_str() {
                "high" => 0.9,
                "medium" => 0.6,
                _ => 0.3,
            };

            Some(crate::models::InvestigationSource {
                title: result.title.clone(),
                url: result.url.clone(),
                excerpt: result.snippet.clone(),
                relevance_score,
                publication_date: None, // TODO: Extract from search results
            })
        })
        .collect();

    Ok(crate::models::InvestigationResult {
        investigation_id: uuid::Uuid::new_v4().to_string(),
        status: "completed".to_string(),
        ai_summary: synthesis.summary,
        sources,
        corroboration_score: synthesis.corroboration_score,
        generated_queries: queries,
        processing_time_ms: elapsed_ms as i32,
    })
}
