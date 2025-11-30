// Grok agent for flight context research and analysis
// Uses X.AI's Grok models with agentic web search and X search capabilities

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

const GROK_RESPONSES_URL: &str = "https://api.x.ai/v1/responses";
const GROK_CHAT_URL: &str = "https://api.x.ai/v1/chat/completions";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokSearchRequest {
    pub flight_id: String,
    pub query: String,
    pub enable_web_search: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokAnalysisResult {
    pub summary: String,
    pub key_findings: Vec<Finding>,
    pub sources: Vec<Source>,
    pub confidence_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub category: String,
    pub description: String,
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub title: String,
    pub url: Option<String>,
    pub snippet: String,
}

/// Analyze flight context using Grok with advanced search
pub async fn analyze_flight_with_grok(
    flight_route: &str,
    flight_date: &str,
    passenger_names: Vec<String>,
    research_topics: Vec<String>,
    web_search_results: Vec<String>,
    api_key: &str,
    model_name: &str, // "grok-4-fast-non-reasoning", "grok-4-fast-reasoning", or "grok-code-fast-1"
) -> Result<GrokAnalysisResult> {
    let client = reqwest::Client::new();

    // Use the specified model (validated by caller)
    let model = model_name;

    // Build comprehensive prompt
    let mut prompt_parts = vec![
        format!("Analyze the following flight and travel context:"),
        format!("Route: {}", flight_route),
        format!("Date: {}", flight_date),
    ];

    if !passenger_names.is_empty() {
        prompt_parts.push(format!("Passengers: {}", passenger_names.join(", ")));
    }

    if !research_topics.is_empty() {
        prompt_parts.push(format!("\nResearch topics: {}", research_topics.join(", ")));
    }

    if !web_search_results.is_empty() {
        prompt_parts.push("\nWeb search results:".to_string());
        for (idx, result) in web_search_results.iter().enumerate() {
            prompt_parts.push(format!("\n[Source {}]: {}", idx + 1, result));
        }
    }

    prompt_parts.push("\nProvide analysis in JSON format with this structure:".to_string());
    prompt_parts.push(
        r#"{
  "summary": "2-3 sentence overview of key findings",
  "key_findings": [
    {
      "category": "News|Events|Weather|Travel|Aviation|Other",
      "description": "Description of the finding",
      "relevance": 0.0-1.0
    }
  ],
  "sources": [
    {
      "title": "Source title",
      "url": "URL if available",
      "snippet": "Brief relevant excerpt"
    }
  ],
  "confidence_score": 0.0-1.0
}"#
        .to_string(),
    );

    prompt_parts
        .push("\nReturn ONLY valid JSON, no markdown formatting, no explanation.".to_string());

    let prompt = prompt_parts.join("\n");

    // Build API request
    let payload = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "You are an expert travel and aviation analyst. Analyze flight contexts, news, events, and travel conditions. Always respond with valid JSON only."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": 0.3,
        "max_tokens": 4000,
        "stream": false
    });

    // Call Grok API
    let response = client
        .post(GROK_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Grok API error: {}", error_text));
    }

    let response_json: serde_json::Value = response.json().await?;

    // Extract content
    let content = response_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract content from Grok response"))?;

    // Clean markdown formatting
    let json_text = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Parse JSON response
    let result: GrokAnalysisResult = serde_json::from_str(json_text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse Grok JSON response: {}. Raw text: {}",
            e,
            json_text
        )
    })?;

    Ok(result)
}

/// Research flight using Grok's built-in web search capabilities
/// This uses Grok's native search integration (if available)
pub async fn research_with_grok_search(
    query: &str,
    api_key: &str,
    model_name: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    let model = model_name;

    let payload = serde_json::json!({
        "model": model,
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful research assistant with access to real-time web information. Provide concise, accurate answers based on current information."
            },
            {
                "role": "user",
                "content": query
            }
        ],
        "temperature": 0.5,
        "max_tokens": 2000,
        "stream": false
    });

    let response = client
        .post(GROK_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Grok API error: {}", error_text));
    }

    let response_json: serde_json::Value = response.json().await?;

    let content = response_json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract content from Grok response"))?
        .to_string();

    Ok(content)
}

/// Compare multiple AI providers' analysis (Grok + DeepSeek)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiProviderAnalysis {
    pub grok_analysis: Option<GrokAnalysisResult>,
    pub deepseek_analysis: Option<crate::deepseek::ResearchResult>,
    pub consensus_summary: String,
    pub disagreements: Vec<String>,
}

/// Analyze flight with multiple AI providers for validation
pub async fn multi_provider_analysis(
    flight_route: &str,
    flight_date: &str,
    passenger_names: Vec<String>,
    research_topics: Vec<String>,
    web_search_results: Vec<String>,
    grok_api_key: Option<&str>,
    deepseek_api_key: Option<&str>,
) -> Result<MultiProviderAnalysis> {
    let mut grok_result = None;
    let mut deepseek_result = None;

    // Run Grok analysis if API key provided
    if let Some(api_key) = grok_api_key {
        match analyze_flight_with_grok(
            flight_route,
            flight_date,
            passenger_names.clone(),
            research_topics.clone(),
            web_search_results.clone(),
            api_key,
            "grok-4-fast-reasoning", // Use advanced model for comparison
        )
        .await
        {
            Ok(result) => grok_result = Some(result),
            Err(e) => eprintln!("Grok analysis failed: {}", e),
        }
    }

    // Run DeepSeek analysis if API key provided
    if let Some(api_key) = deepseek_api_key {
        let deepseek_request = crate::deepseek::ResearchRequest {
            date: flight_date.to_string(),
            location: flight_route.to_string(),
            passenger_names: passenger_names.clone(),
            research_news: research_topics.contains(&"news".to_string()),
            research_events: research_topics.contains(&"events".to_string()),
            research_weather: research_topics.contains(&"weather".to_string()),
            research_passengers: !passenger_names.is_empty(),
        };

        match crate::deepseek::research_flight_context(
            deepseek_request,
            web_search_results.clone(),
            api_key,
        )
        .await
        {
            Ok(result) => deepseek_result = Some(result),
            Err(e) => eprintln!("DeepSeek analysis failed: {}", e),
        }
    }

    // Build consensus summary
    let consensus_summary = if grok_result.is_some() && deepseek_result.is_some() {
        "Multi-provider analysis complete. Both Grok and DeepSeek have provided insights."
            .to_string()
    } else if grok_result.is_some() {
        "Analysis provided by Grok.".to_string()
    } else if deepseek_result.is_some() {
        "Analysis provided by DeepSeek.".to_string()
    } else {
        "No AI analysis available - missing API keys.".to_string()
    };

    Ok(MultiProviderAnalysis {
        grok_analysis: grok_result,
        deepseek_analysis: deepseek_result,
        consensus_summary,
        disagreements: Vec::new(), // TODO: Implement disagreement detection
    })
}

/// Direct chat with Grok for freeform queries
pub async fn chat_with_grok(
    query: &str,
    api_key: &str,
    model_name: &str,
) -> Result<GrokAnalysisResult> {
    let client = reqwest::Client::new();

    // Create message payload
    let payload = serde_json::json!({
        "model": model_name,
        "messages": [
            {
                "role": "system",
                "content": "You are a helpful AI assistant with web search capabilities. Provide detailed, accurate responses in a structured format."
            },
            {
                "role": "user",
                "content": query
            }
        ],
        "temperature": 0.7,
        "max_tokens": 2000,
    });

    let response = client
        .post(GROK_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "Grok API error ({}): {}",
            status,
            error_text
        ));
    }

    let response_data: serde_json::Value = response.json().await?;

    // Extract response content
    let content = response_data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Return as simplified analysis result
    Ok(GrokAnalysisResult {
        summary: content,
        key_findings: Vec::new(),
        sources: Vec::new(),
        confidence_score: 0.85,
    })
}

// ============================================================================
// INVESTIGATION ENGINE - Grok Agentic Search Implementation
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct GrokResponsesRequest {
    model: String,
    input: Vec<GrokMessage>,
    tools: Vec<GrokTool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GrokTool {
    #[serde(rename = "type")]
    tool_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<GrokToolFilters>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GrokToolFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    excluded_domains: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GrokResponsesResponse {
    output: String,
    #[serde(default)]
    citations: Vec<String>,
}

/// Run investigation using Grok's agentic search with web and X search
pub async fn run_grok_investigation(
    passenger_names: Vec<String>,
    location: String,
    date: String,
    api_key: String,
) -> Result<crate::models::InvestigationResult> {
    let start_time = std::time::Instant::now();

    let names_list = passenger_names.join(", ");

    // Build investigation prompt
    let prompt = format!(
        r#"You are a world-class investigative journalist and OSINT researcher. Your task is to find verifiable evidence of the following person(s) being at or near the specified location on the specified date.

Investigation Target:
- Person/People: {}
- Location: {}
- Date: {} (search within 48 hours of this date)

Your mission:
1. Use web search to find public records, news articles, event listings, social media posts, or official documents
2. Use X (Twitter) search to find posts, mentions, or threads related to the person(s) and location
3. Cross-reference multiple sources to establish credibility
4. Focus on verifiable, timestamped evidence

Please provide your findings in a detailed investigation report with:
- A comprehensive summary (2-3 paragraphs) analyzing all evidence found
- Key findings with supporting evidence from your searches
- Assessment of the strength of evidence (corroboration score 0.0-1.0)
- List of all sources you examined

Be thorough, objective, and note when evidence is absent, weak, or contradictory."#,
        names_list, location, date
    );

    // Call Grok with agentic search tools enabled
    let client = reqwest::Client::new();

    let request_body = json!({
        "model": "grok-2-1212",
        "input": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "tools": [
            {
                "type": "web_search"
            },
            {
                "type": "x_search",
                "filters": {
                    "from_date": date.clone(),
                    // Set to_date to 2 days after from_date
                    "to_date": date.clone()
                }
            }
        ]
    });

    let response = client
        .post(GROK_RESPONSES_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!(
            "Grok API error ({}): {}",
            status,
            error_text
        ));
    }

    let grok_response: GrokResponsesResponse = response.json().await?;

    // Parse the investigation result from Grok's response
    let summary = grok_response.output.clone();

    // Estimate corroboration score based on response content
    let corroboration_score = estimate_corroboration_score(&summary, &grok_response.citations);

    // Build sources from citations
    let sources: Vec<crate::models::InvestigationSource> = grok_response
        .citations
        .into_iter()
        .enumerate()
        .map(|(idx, url)| crate::models::InvestigationSource {
            title: format!("Source {}", idx + 1),
            url: url.clone(),
            excerpt: "Found via Grok agentic search".to_string(),
            relevance_score: 0.8, // Default high relevance for Grok-cited sources
            publication_date: None,
        })
        .collect();

    let elapsed_ms = start_time.elapsed().as_millis() as i32;

    Ok(crate::models::InvestigationResult {
        investigation_id: uuid::Uuid::new_v4().to_string(),
        status: "completed".to_string(),
        ai_summary: summary,
        sources,
        corroboration_score,
        generated_queries: vec!["Grok agentic search (automated query generation)".to_string()],
        processing_time_ms: elapsed_ms,
    })
}

/// Extract key findings from Grok's summary using simple heuristics
fn extract_key_findings(summary: &str) -> Vec<String> {
    let mut findings = Vec::new();

    // Look for numbered lists or bullet points
    for line in summary.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- ") || trimmed.starts_with("• ") {
            findings.push(trimmed[2..].to_string());
        } else if trimmed.len() > 2 {
            // Check for "1.", "2.", etc.
            if let Some(first_char) = trimmed.chars().next() {
                if first_char.is_numeric() && trimmed.contains('.') {
                    if let Some(dot_pos) = trimmed.find('.') {
                        if dot_pos < 3 {
                            findings.push(trimmed[dot_pos + 1..].trim().to_string());
                        }
                    }
                }
            }
        }
    }

    // If no structured findings found, extract first few sentences
    if findings.is_empty() {
        let sentences: Vec<&str> = summary.split(". ").take(3).collect();
        findings = sentences.iter().map(|s| s.to_string()).collect();
    }

    findings
}

/// Estimate corroboration score based on response content and citations
fn estimate_corroboration_score(summary: &str, citations: &[String]) -> f64 {
    let summary_lower = summary.to_lowercase();

    // Check for strong evidence indicators
    let strong_indicators = [
        "confirmed",
        "verified",
        "documented",
        "official record",
        "according to",
        "published",
    ];
    let weak_indicators = [
        "no evidence",
        "could not find",
        "unconfirmed",
        "possibly",
        "might",
        "unclear",
    ];

    let strong_count = strong_indicators
        .iter()
        .filter(|&indicator| summary_lower.contains(indicator))
        .count();

    let weak_count = weak_indicators
        .iter()
        .filter(|&indicator| summary_lower.contains(indicator))
        .count();

    // Base score on citations count
    let citation_score = (citations.len() as f64 * 0.1).min(0.5);

    // Adjust based on indicators
    let indicator_adjustment = (strong_count as f64 * 0.1) - (weak_count as f64 * 0.15);

    // Final score between 0.0 and 1.0
    (citation_score + indicator_adjustment + 0.3).clamp(0.0, 1.0)
}
