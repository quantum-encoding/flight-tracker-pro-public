// Investigation Engine - AI-Powered Cross-Referencing System
// Generates search queries, executes searches, and synthesizes evidence

use crate::models::{InvestigationResult, InvestigationSource};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const QUERY_GENERATION_PROMPT: &str = r#"You are a world-class investigative journalist and OSINT researcher. Your task is to generate highly specific, targeted search engine queries to find public records, news articles, event listings, social media posts, or official documents.

Given the following information:
- Person/People: {passenger_names}
- Location: {location}
- Date: {date}

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

Focus on queries that would return credible, verifiable sources. Avoid speculation."#;

const SYNTHESIS_PROMPT: &str = r#"You are an expert investigative analyst synthesizing search results into an evidence report.

Investigation Target:
- Person/People: {passenger_names}
- Location: {location}
- Date: {date}

Search Results:
{search_results}

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
{
  "summary": "Your detailed analysis here...",
  "key_findings": ["finding 1", "finding 2", ...],
  "corroboration_score": 0.75,
  "sources_analysis": [
    {
      "source_index": 0,
      "relevance": "high|medium|low",
      "credibility": "official|news|social|unverified",
      "key_quote": "Extracted quote or fact"
    }
  ]
}

Be objective and factual. Note when evidence is absent, weak, or contradictory."#;

#[derive(Debug, Serialize, Deserialize)]
struct QueryGenerationResponse {
    queries: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SynthesisResponse {
    summary: String,
    key_findings: Vec<String>,
    corroboration_score: f64,
    sources_analysis: Vec<SourceAnalysis>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SourceAnalysis {
    source_index: usize,
    relevance: String,
    credibility: String,
    key_quote: String,
}

// Simplified search result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Generate AI-powered search queries using Gemini
pub async fn generate_search_queries(
    passenger_names: &[String],
    location: &str,
    date: &str,
    api_key: &str,
) -> Result<Vec<String>> {
    let names_list = passenger_names.join(", ");

    let prompt = QUERY_GENERATION_PROMPT
        .replace("{passenger_names}", &names_list)
        .replace("{location}", location)
        .replace("{date}", date);

    let response = call_gemini_json(&prompt, api_key).await?;

    // Parse the response as a JSON array of strings
    let queries: Vec<String> =
        serde_json::from_str(&response).context("Failed to parse query generation response")?;

    Ok(queries)
}

/// Execute web searches using SearXNG or DuckDuckGo (privacy-focused, no API key needed)
pub async fn execute_searches(queries: &[String]) -> Result<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    // Use DuckDuckGo HTML scraping (respecting rate limits)
    for (idx, query) in queries.iter().enumerate() {
        // Rate limit: wait 2 seconds between searches to be respectful
        if idx > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }

        match search_duckduckgo(query).await {
            Ok(results) => {
                all_results.extend(results);
            }
            Err(e) => {
                eprintln!("Search failed for query '{}': {}", query, e);
                // Continue with other queries even if one fails
            }
        }

        // Limit total results to prevent overwhelming the AI
        if all_results.len() >= 30 {
            break;
        }
    }

    Ok(all_results)
}

/// Search using DuckDuckGo HTML API (no auth required)
async fn search_duckduckgo(query: &str) -> Result<Vec<SearchResult>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to execute DuckDuckGo search")?;

    let html = response.text().await?;

    // Parse HTML to extract results (simple regex-based parsing)
    parse_duckduckgo_results(&html)
}

/// Parse DuckDuckGo HTML results
fn parse_duckduckgo_results(html: &str) -> Result<Vec<SearchResult>> {
    let mut results = Vec::new();

    // This is a simplified parser - in production, use a proper HTML parser like scraper
    // For now, we'll extract basic information using regex patterns

    // DuckDuckGo result pattern: <a rel="nofollow" class="result__a" href="...">Title</a>
    let title_pattern =
        regex::Regex::new(r#"<a rel="nofollow" class="result__a" href="([^"]+)">([^<]+)</a>"#)?;
    let snippet_pattern = regex::Regex::new(r#"<a class="result__snippet"[^>]*>([^<]+)</a>"#)?;

    for (idx, title_cap) in title_pattern.captures_iter(html).enumerate() {
        if idx >= 5 {
            break; // Limit to 5 results per query
        }

        let url = title_cap
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        let title = title_cap
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        // Try to find corresponding snippet (approximate)
        let snippet = snippet_pattern
            .captures_iter(html)
            .nth(idx)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();

        if !url.is_empty() && !title.is_empty() {
            results.push(SearchResult {
                title: html_escape::decode_html_entities(&title).to_string(),
                url: html_escape::decode_html_entities(&url).to_string(),
                snippet: html_escape::decode_html_entities(&snippet).to_string(),
            });
        }
    }

    Ok(results)
}

/// Synthesize investigation results using Gemini AI
pub async fn synthesize_investigation(
    passenger_names: &[String],
    location: &str,
    date: &str,
    search_results: &[SearchResult],
    api_key: &str,
) -> Result<SynthesisResponse> {
    let names_list = passenger_names.join(", ");

    // Format search results for the prompt
    let results_text = search_results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "[{}] Title: {}\nURL: {}\nSnippet: {}\n",
                i, r.title, r.url, r.snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = SYNTHESIS_PROMPT
        .replace("{passenger_names}", &names_list)
        .replace("{location}", location)
        .replace("{date}", date)
        .replace("{search_results}", &results_text);

    let response = call_gemini_json(&prompt, api_key).await?;

    let synthesis: SynthesisResponse =
        serde_json::from_str(&response).context("Failed to parse synthesis response")?;

    Ok(synthesis)
}

/// Call Gemini API with JSON response mode
async fn call_gemini_json(prompt: &str, api_key: &str) -> Result<String> {
    let client = reqwest::Client::new();

    let model = "gemini-2.0-flash-exp"; // Use the most capable model for investigations
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let request_body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": prompt
            }]
        }],
        "generationConfig": {
            "temperature": 0.2,
            "topK": 40,
            "topP": 0.95,
            "maxOutputTokens": 8192,
            "responseMimeType": "application/json"
        }
    });

    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .context("Failed to call Gemini API")?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(anyhow::anyhow!("Gemini API error: {}", error_text));
    }

    let response_json: serde_json::Value = response.json().await?;

    let text = response_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No text in Gemini response"))?;

    Ok(text.to_string())
}

/// Main investigation orchestrator
pub async fn run_investigation(
    passenger_names: Vec<String>,
    location: String,
    date: String,
    api_key: String,
) -> Result<InvestigationResult> {
    let start_time = std::time::Instant::now();

    // Step 1: Generate search queries
    let queries = generate_search_queries(&passenger_names, &location, &date, &api_key).await?;

    // Step 2: Execute searches
    let search_results = execute_searches(&queries).await?;

    // Step 3: Synthesize results
    let synthesis = synthesize_investigation(
        &passenger_names,
        &location,
        &date,
        &search_results,
        &api_key,
    )
    .await?;

    // Step 4: Convert to investigation sources
    let sources: Vec<InvestigationSource> = search_results
        .iter()
        .enumerate()
        .filter_map(|(i, result)| {
            // Find relevance from synthesis
            let analysis = synthesis
                .sources_analysis
                .iter()
                .find(|a| a.source_index == i)?;

            let relevance_score = match analysis.relevance.as_str() {
                "high" => 0.9,
                "medium" => 0.6,
                _ => 0.3,
            };

            Some(InvestigationSource {
                title: result.title.clone(),
                url: result.url.clone(),
                excerpt: result.snippet.clone(),
                relevance_score,
                publication_date: None, // TODO: Extract from search results
            })
        })
        .collect();

    let elapsed = start_time.elapsed().as_millis() as i32;

    Ok(InvestigationResult {
        investigation_id: uuid::Uuid::new_v4().to_string(),
        status: "completed".to_string(),
        ai_summary: synthesis.summary,
        sources,
        corroboration_score: synthesis.corroboration_score,
        generated_queries: queries,
        processing_time_ms: elapsed,
    })
}
