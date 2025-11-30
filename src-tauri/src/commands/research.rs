// AI research commands (DeepSeek, Grok, Gemini)
use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;

// ===== RESEARCH TYPES =====

#[derive(Debug, Serialize, Deserialize)]
pub struct ResearchRequest {
    pub flight_id: String,
    pub research_departure: bool,
    pub research_destination: bool,
    pub research_news: bool,
    pub research_events: bool,
    pub research_weather: bool,
    pub research_passengers: bool,
}

// ===== HELPER FUNCTIONS =====

/// Get API key from environment or database
fn get_api_key(
    env_vars: &[&str],
    db_key: &str,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    // First try environment variables (in order of preference)
    for env_var in env_vars {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }

    // Fall back to database setting
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_setting(db_key)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "{} not configured. Set {} environment variable or add it in Settings.",
                db_key, env_vars[0]
            )
        })
}

/// Perform web search using DuckDuckGo
async fn perform_web_search(query: &str, max_results: usize) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let html = response.text().await.map_err(|e| e.to_string())?;

    // Parse search results (basic extraction)
    let mut results = Vec::new();

    // Compile regex for HTML tag removal once (used in both loops)
    let html_tag_regex = regex::Regex::new(r"<[^>]+>").unwrap();

    // Simple regex-based extraction of result snippets
    let re = regex::Regex::new(r#"<a class="result__snippet"[^>]*>(.*?)</a>"#)
        .map_err(|e| e.to_string())?;

    for cap in re.captures_iter(&html).take(max_results) {
        if let Some(snippet) = cap.get(1) {
            let text = snippet.as_str();
            // Unescape HTML entities
            let unescaped = html_escape::decode_html_entities(text).to_string();
            // Remove HTML tags
            let clean = html_tag_regex
                .replace_all(&unescaped, "")
                .to_string();

            if !clean.trim().is_empty() {
                results.push(clean.trim().to_string());
            }
        }
    }

    // Also try to extract titles
    let title_re =
        regex::Regex::new(r#"<a class="result__a"[^>]*>(.*?)</a>"#).map_err(|e| e.to_string())?;

    for cap in title_re.captures_iter(&html).take(max_results) {
        if let Some(title) = cap.get(1) {
            let text = title.as_str();
            let unescaped = html_escape::decode_html_entities(text).to_string();
            let clean = html_tag_regex
                .replace_all(&unescaped, "")
                .to_string();

            if !clean.trim().is_empty() {
                results.push(format!("Title: {}", clean.trim()));
            }
        }
    }

    Ok(results)
}

/// Sanitize a string to be used as a filename
fn sanitize_filename(s: &str) -> String {
    let mut sanitized = s.to_lowercase();

    // Replace spaces and special characters with hyphens
    sanitized = sanitized
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect();

    // Remove consecutive hyphens/underscores
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }

    // Trim hyphens and underscores from start/end
    sanitized = sanitized.trim_matches(|c| c == '-' || c == '_').to_string();

    // Limit length to 50 characters
    if sanitized.len() > 50 {
        sanitized.truncate(50);
        sanitized = sanitized.trim_matches(|c| c == '-' || c == '_').to_string();
    }

    // Fallback if empty
    if sanitized.is_empty() {
        sanitized = "report".to_string();
    }

    sanitized
}

// ===== DEEPSEEK RESEARCH COMMANDS =====

#[tauri::command]
pub async fn research_flight_with_deepseek(
    request: ResearchRequest,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::deepseek::ResearchResult, String> {
    // Get flight details
    let flight = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_flight(&request.flight_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Flight not found".to_string())?
    };

    // Get DeepSeek API key from environment or settings
    let api_key = get_api_key(&["DEEPSEEK_API_KEY"], "deepseek_api_key", &state)?;

    // Build memory query string
    let memory_query = format!(
        "flight:{} news:{} events:{} weather:{} passengers:{}",
        request.flight_id,
        request.research_news,
        request.research_events,
        request.research_weather,
        request.research_passengers
    );

    // Check memory cache for recent research (within 24 hours)
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();

        if let Ok(Some(memory)) = crate::agent_memory::find_similar_memory(
            conn,
            &memory_query,
            "deepseek_research",
            Some(24), // 24 hour cache
        ) {
            // Cache hit - deserialize and return stored result
            if let Ok(cached_result) = serde_json::from_str::<crate::deepseek::ResearchResult>(&memory.content) {
                return Ok(cached_result);
            }
        }
    }

    // Emit agent start event
    let operation = format!("Flight research: {}", request.flight_id);
    crate::agent_tracking::emit_agent_start(&app_handle, "DeepSeek", "deepseek-chat", &operation);

    // Extract passenger names from notes
    let passenger_names: Vec<String> = if request.research_passengers {
        if let Some(notes) = &flight.notes {
            if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
                passenger_part
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Determine location(s) to research
    let mut locations = Vec::new();
    if request.research_departure {
        locations.push(flight.departure_airport.clone());
    }
    if request.research_destination {
        locations.push(flight.arrival_airport.clone());
    }

    let location_str = if locations.is_empty() {
        format!("{} to {}", flight.departure_airport, flight.arrival_airport)
    } else {
        locations.join(" and ")
    };

    let date = flight
        .departure_datetime
        .split('T')
        .next()
        .unwrap_or(&flight.departure_datetime)
        .to_string();

    // Perform web searches using DuckDuckGo
    let mut search_results = Vec::new();

    if request.research_news {
        let news_query = format!("{} {} news", location_str, date);
        if let Ok(results) = perform_web_search(&news_query, 5).await {
            search_results.extend(results);
        }
    }

    if request.research_events {
        let events_query = format!("{} {} events conferences", location_str, date);
        if let Ok(results) = perform_web_search(&events_query, 5).await {
            search_results.extend(results);
        }
    }

    if request.research_weather {
        let weather_query = format!("{} {} weather conditions", location_str, date);
        if let Ok(results) = perform_web_search(&weather_query, 3).await {
            search_results.extend(results);
        }
    }

    if request.research_passengers && !passenger_names.is_empty() {
        for passenger in &passenger_names {
            let passenger_query = format!("{} {} {}", passenger, location_str, date);
            if let Ok(results) = perform_web_search(&passenger_query, 3).await {
                search_results.extend(results);
            }
        }
    }

    // If no search results, return empty result
    if search_results.is_empty() {
        return Ok(crate::deepseek::ResearchResult {
            summary: "No web search results found for the specified criteria.".to_string(),
            news_stories: Vec::new(),
            events: Vec::new(),
            weather: None,
            passenger_mentions: Vec::new(),
        });
    }

    // Build DeepSeek research request
    let deepseek_request = crate::deepseek::ResearchRequest {
        date: date.clone(),
        location: location_str,
        passenger_names,
        research_news: request.research_news,
        research_events: request.research_events,
        research_weather: request.research_weather,
        research_passengers: request.research_passengers,
    };

    // Emit thinking event
    crate::agent_tracking::emit_agent_thinking(&app_handle, "DeepSeek", "deepseek-chat");

    // Call DeepSeek to analyze search results
    let result = crate::deepseek::research_flight_context(deepseek_request, search_results, &api_key)
        .await;

    // Emit completion or error event and store in memory
    match &result {
        Ok(research_result) => {
            crate::agent_tracking::emit_agent_complete(&app_handle, "DeepSeek", "deepseek-chat", 0, 0);

            // Store result in agent memory
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let conn = db.get_connection();

            // Serialize result for storage
            if let Ok(content_json) = serde_json::to_string(research_result) {
                let summary = format!(
                    "DeepSeek research for flight {} - {} stories, {} events",
                    request.flight_id,
                    research_result.news_stories.len(),
                    research_result.events.len()
                );

                let _ = crate::agent_memory::store_memory(
                    conn,
                    "DeepSeek",
                    "deepseek_research",
                    &content_json,
                    Some(&memory_query),
                    Some(&summary),
                    Some(&request.flight_id),
                    None, // user_id
                    Some("deepseek-chat"),
                    0, // tokens_used (not available from API)
                    0.0, // cost_usd
                    None, // confidence_score
                    Some(24), // 24 hour TTL
                );
            }
        },
        Err(e) => crate::agent_tracking::emit_agent_error(&app_handle, "DeepSeek", "deepseek-chat", &e.to_string()),
    }

    result.map_err(|e| format!("DeepSeek research failed: {}", e))
}

// ===== GROK AI RESEARCH =====

#[tauri::command]
pub async fn research_flight_with_grok(
    flight_id: String,
    research_topics: Vec<String>,
    model_name: String, // "grok-4-fast-non-reasoning", "grok-4-fast-reasoning", or "grok-code-fast-1"
    custom_query: Option<String>, // Optional custom query for freeform chat
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<crate::grok::GrokAnalysisResult, String> {
    // Validate model name
    let valid_models = [
        "grok-4-fast-non-reasoning",
        "grok-4-fast-reasoning",
        "grok-code-fast-1",
    ];
    if !valid_models.contains(&model_name.as_str()) {
        return Err(format!(
            "Invalid model name. Must be one of: {:?}",
            valid_models
        ));
    }

    // Get Grok API key from environment or settings (XAI_API_KEY or GROK_API_KEY)
    let api_key = get_api_key(&["XAI_API_KEY", "GROK_API_KEY"], "grok_api_key", &state)?;

    // Build memory query string for standard research (not custom queries)
    let memory_query = if custom_query.is_none() {
        format!(
            "flight:{} topics:{:?} model:{}",
            flight_id,
            research_topics,
            model_name
        )
    } else {
        custom_query.clone().unwrap_or_default()
    };

    // Check memory cache for recent research (within 24 hours) - skip for custom queries
    if custom_query.is_none() {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let conn = db.get_connection();

        if let Ok(Some(memory)) = crate::agent_memory::find_similar_memory(
            conn,
            &memory_query,
            "grok_research",
            Some(24), // 24 hour cache
        ) {
            // Cache hit - deserialize and return stored result
            if let Ok(cached_result) = serde_json::from_str::<crate::grok::GrokAnalysisResult>(&memory.content) {
                return Ok(cached_result);
            }
        }
    }

    // Emit agent start event
    let operation = custom_query.as_ref()
        .map(|q| format!("Chat: {}", q.chars().take(50).collect::<String>()))
        .unwrap_or_else(|| format!("Flight research: {}", flight_id));
    crate::agent_tracking::emit_agent_start(&app_handle, "Grok", &model_name, &operation);

    // If custom query is provided, use direct chat mode
    if let Some(query) = custom_query {
        crate::agent_tracking::emit_agent_thinking(&app_handle, "Grok", &model_name);

        let result = crate::grok::chat_with_grok(&query, &api_key, &model_name)
            .await;

        match &result {
            Ok(_) => crate::agent_tracking::emit_agent_complete(&app_handle, "Grok", &model_name, 0, 0),
            Err(e) => crate::agent_tracking::emit_agent_error(&app_handle, "Grok", &model_name, &e.to_string()),
        }

        return result.map_err(|e| format!("Grok chat failed: {}", e));
    }

    // Get flight details
    let flight = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_flight(&flight_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Flight not found".to_string())?
    };

    let flight_route = format!("{} to {}", flight.departure_airport, flight.arrival_airport);
    let flight_date = flight
        .departure_datetime
        .split('T')
        .next()
        .unwrap_or(&flight.departure_datetime)
        .to_string();

    // Extract passenger names from notes
    let passenger_names: Vec<String> = if let Some(notes) = &flight.notes {
        if let Some(passenger_line) = notes.lines().find(|line| line.starts_with("Passengers:")) {
            passenger_line
                .trim_start_matches("Passengers:")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Perform web searches based on research topics
    let mut search_results = Vec::new();

    for topic in &research_topics {
        let query = match topic.as_str() {
            "news" => format!("{} {} news", flight_route, flight_date),
            "events" => format!("{} {} events", flight_route, flight_date),
            "weather" => format!("{} {} weather", flight_route, flight_date),
            "aviation" => format!("{} flight status aviation incidents", flight_route),
            _ => continue,
        };

        if let Ok(results) = perform_web_search(&query, 3).await {
            search_results.extend(results);
        }
    }

    // Emit thinking event
    crate::agent_tracking::emit_agent_thinking(&app_handle, "Grok", &model_name);

    // Call Grok for analysis
    let result = crate::grok::analyze_flight_with_grok(
        &flight_route,
        &flight_date,
        passenger_names,
        research_topics.clone(),
        search_results,
        &api_key,
        &model_name,
    )
    .await;

    // Emit completion or error event and store in memory
    match &result {
        Ok(analysis_result) => {
            crate::agent_tracking::emit_agent_complete(&app_handle, "Grok", &model_name, 0, 0);

            // Store result in agent memory
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let conn = db.get_connection();

            // Serialize result for storage
            if let Ok(content_json) = serde_json::to_string(analysis_result) {
                let summary = format!(
                    "Grok {} analysis for {} - Topics: {:?}",
                    model_name,
                    flight_route,
                    research_topics
                );

                let _ = crate::agent_memory::store_memory(
                    conn,
                    "Grok",
                    "grok_research",
                    &content_json,
                    Some(&memory_query),
                    Some(&summary),
                    Some(&flight_id),
                    None, // user_id
                    Some(&model_name),
                    0, // tokens_used (not available from API)
                    0.0, // cost_usd
                    None, // confidence_score
                    Some(24), // 24 hour TTL
                );
            }
        },
        Err(e) => crate::agent_tracking::emit_agent_error(&app_handle, "Grok", &model_name, &e.to_string()),
    }

    result.map_err(|e| format!("Grok analysis failed: {}", e))
}

#[tauri::command]
pub async fn multi_provider_flight_research(
    flight_id: String,
    research_topics: Vec<String>,
    state: State<'_, AppState>,
) -> Result<crate::grok::MultiProviderAnalysis, String> {
    // Get API keys from environment or settings
    let grok_api_key = get_api_key(&["XAI_API_KEY", "GROK_API_KEY"], "grok_api_key", &state).ok();
    let deepseek_api_key = get_api_key(&["DEEPSEEK_API_KEY"], "deepseek_api_key", &state).ok();

    if grok_api_key.is_none() && deepseek_api_key.is_none() {
        return Err(
            "At least one AI provider API key must be configured (Grok or DeepSeek)".to_string(),
        );
    }

    // Get flight details
    let flight = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_flight(&flight_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Flight not found".to_string())?
    };

    let flight_route = format!("{} to {}", flight.departure_airport, flight.arrival_airport);
    let flight_date = flight
        .departure_datetime
        .split('T')
        .next()
        .unwrap_or(&flight.departure_datetime)
        .to_string();

    // Extract passenger names
    let passenger_names: Vec<String> = if let Some(notes) = &flight.notes {
        if let Some(passenger_line) = notes.lines().find(|line| line.starts_with("Passengers:")) {
            passenger_line
                .trim_start_matches("Passengers:")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Perform comprehensive web searches
    let mut search_results = Vec::new();
    for topic in &research_topics {
        let query = match topic.as_str() {
            "news" => format!("{} {} news", flight_route, flight_date),
            "events" => format!("{} {} events", flight_route, flight_date),
            "weather" => format!("{} {} weather", flight_route, flight_date),
            "aviation" => format!("{} flight aviation", flight_route),
            _ => continue,
        };
        if let Ok(results) = perform_web_search(&query, 5).await {
            search_results.extend(results);
        }
    }

    // Call multi-provider analysis
    crate::grok::multi_provider_analysis(
        &flight_route,
        &flight_date,
        passenger_names,
        research_topics,
        search_results,
        grok_api_key.as_deref(),
        deepseek_api_key.as_deref(),
    )
    .await
    .map_err(|e| format!("Multi-provider analysis failed: {}", e))
}

// ===== GEMINI CHAT =====

#[tauri::command]
pub async fn chat_with_gemini(
    query: String,
    model: String, // "gemini-2.5-pro", "gemini-2.5-flash", or "gemini-2.5-flash-lite"
    state: State<'_, AppState>,
) -> Result<crate::gemini::GeminiChatResult, String> {
    // Get Gemini API key from environment or settings
    let api_key = get_api_key(
        &["GENAI_API_KEY", "GEMINI_API_KEY"],
        "gemini_api_key",
        &state,
    )?;

    // Call Gemini chat
    crate::gemini::chat_with_gemini(&query, &api_key, &model)
        .await
        .map_err(|e| format!("Gemini chat failed: {}", e))
}

// ===== DEEPSEEK CHAT =====

#[tauri::command]
pub async fn chat_with_deepseek(
    query: String,
    state: State<'_, AppState>,
) -> Result<crate::deepseek::DeepSeekChatResult, String> {
    // Get DeepSeek API key from environment or settings
    let api_key = get_api_key(&["DEEPSEEK_API_KEY"], "deepseek_api_key", &state)?;

    // Call DeepSeek chat
    crate::deepseek::chat_with_deepseek(&query, &api_key)
        .await
        .map_err(|e| format!("DeepSeek chat failed: {}", e))
}

// ===== RESEARCH REPORTS =====

#[tauri::command]
pub fn save_research_report(
    user_id: String,
    report: crate::models::ResearchReportInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_research_report(&user_id, &report)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_research_report(
    report_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::ResearchReport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_research_report(&report_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_research_reports(
    user_id: String,
    limit: i64,
    offset: i64,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::ResearchReport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_research_reports(&user_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_research_report(report_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_research_report(&report_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn count_research_reports(user_id: String, state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.count_research_reports(&user_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_research_report_to_markdown(
    report_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use std::fs;

    use chrono::{DateTime, Utc};

    // Get the report from database
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let report = db
        .get_research_report(&report_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Report not found".to_string())?;

    // Parse the created_at timestamp for directory naming
    let created_at_parsed = DateTime::parse_from_rfc3339(&report.created_at)
        .or_else(|_| {
            // Fallback: try parsing as SQLite datetime format
            chrono::NaiveDateTime::parse_from_str(&report.created_at, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc).fixed_offset())
        })
        .map_err(|e| format!("Failed to parse timestamp: {}", e))?;

    let timestamp = created_at_parsed.format("%Y%m%d-%H%M%S").to_string();

    // Create base directory path: ~/flight-tracker-pro/researcher/
    let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory".to_string())?;
    let base_path = home_dir.join("flight-tracker-pro").join("researcher");

    // Create agent-specific directory with timestamp
    let agent_dir = base_path.join(format!(
        "{}-{}",
        report.agent_name.to_lowercase(),
        timestamp
    ));

    // Create directories if they don't exist
    fs::create_dir_all(&agent_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    // Generate filename from search query (sanitized)
    let filename = sanitize_filename(&report.search_query);
    let markdown_filename = format!("{}.md", filename);
    let file_path = agent_dir.join(&markdown_filename);

    // Build markdown content
    let mut markdown = String::new();

    // Header
    markdown.push_str(&format!("# Research Report: {}\n\n", report.search_query));
    markdown.push_str(&format!("**Agent:** {}\n", report.agent_name));
    if let Some(model) = &report.agent_model {
        markdown.push_str(&format!("**Model:** {}\n", model));
    }
    markdown.push_str(&format!("**Date:** {}\n", report.created_at));
    markdown.push_str(&format!("**Report Type:** {}\n", report.report_type));
    if let Some(confidence) = report.confidence_score {
        markdown.push_str(&format!("**Confidence Score:** {:.2}\n", confidence));
    }
    if let Some(processing_time) = report.processing_time_ms {
        markdown.push_str(&format!("**Processing Time:** {}ms\n", processing_time));
    }
    if let Some(flight_id) = &report.flight_id {
        markdown.push_str(&format!("**Flight ID:** {}\n", flight_id));
    }
    markdown.push_str("\n---\n\n");

    // Summary
    markdown.push_str("## Summary\n\n");
    markdown.push_str(&report.report_summary);
    markdown.push_str("\n\n");

    // Details
    if let Some(details) = &report.report_details {
        markdown.push_str("## Detailed Analysis\n\n");
        markdown.push_str(details);
        markdown.push_str("\n\n");
    }

    // Research Topics
    if let Some(topics_json) = &report.research_topics {
        if let Ok(topics) = serde_json::from_str::<Vec<String>>(topics_json) {
            markdown.push_str("## Research Topics\n\n");
            for topic in topics {
                markdown.push_str(&format!("- {}\n", topic));
            }
            markdown.push('\n');
        }
    }

    // Sources
    if let Some(sources_json) = &report.sources {
        if let Ok(sources) =
            serde_json::from_str::<Vec<crate::models::ResearchSource>>(sources_json)
        {
            markdown.push_str("## Sources\n\n");
            for (idx, source) in sources.iter().enumerate() {
                markdown.push_str(&format!("### {}. {}\n\n", idx + 1, source.title));
                if let Some(url) = &source.url {
                    markdown.push_str(&format!("**URL:** {}\n\n", url));
                }
                markdown.push_str(&format!("{}\n\n", source.snippet));
            }
        }
    }

    // Footer
    markdown.push_str("---\n\n");
    markdown.push_str("*Generated by Flight Tracker Pro*\n");

    // Write to file
    fs::write(&file_path, markdown).map_err(|e| format!("Failed to write markdown file: {}", e))?;

    // Return the full path
    Ok(file_path.to_string_lossy().to_string())
}
