// WebSocket Agent Server - Bridge endpoint for remote agent orchestration
// Listens on port 9528 for commands from quantum-local-bridge
// HTTP API Server - REST endpoint for Tailscale mobile access on port 9529
//
// Authentication:
// - Set FTP_API_KEY environment variable to enable API key authentication
// - Clients must include X-API-Key header with requests
// - Keys can also be stored in ~/.config/flight-tracker-pro/api_keys.json

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use axum::{
    extract::{State as AxumState, Request},
    http::{StatusCode, HeaderMap},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::database::Database;
use crate::models::{FlightInput, ResearchReportInput};

// ===== API KEY AUTHENTICATION =====

/// Stores valid API keys and their metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeyConfig {
    /// Whether authentication is enabled
    pub enabled: bool,
    /// Master API key (from environment variable FTP_API_KEY)
    pub master_key: Option<String>,
    /// Named API keys with permissions
    pub keys: Vec<ApiKeyEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    pub name: String,
    pub key: String,
    pub permissions: Vec<String>,
    pub created_at: String,
    pub last_used: Option<String>,
}

impl ApiKeyConfig {
    /// Load API key configuration
    pub fn load() -> Self {
        let mut config = Self::default();

        // Check environment variable for master key
        if let Ok(key) = std::env::var("FTP_API_KEY") {
            if !key.is_empty() {
                config.enabled = true;
                config.master_key = Some(key);
                println!("🔐 API key authentication enabled via FTP_API_KEY");
            }
        }

        // Try to load additional keys from config file
        if let Some(config_dir) = dirs::config_dir() {
            let keys_path = config_dir.join("flight-tracker-pro").join("api_keys.json");
            if keys_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&keys_path) {
                    if let Ok(file_config) = serde_json::from_str::<ApiKeyConfig>(&content) {
                        config.keys = file_config.keys;
                        if file_config.enabled {
                            config.enabled = true;
                        }
                        println!("🔑 Loaded {} API keys from config", config.keys.len());
                    }
                }
            }
        }

        if !config.enabled {
            println!("⚠️  API authentication disabled (set FTP_API_KEY to enable)");
        }

        config
    }

    /// Validate an API key
    pub fn validate_key(&self, key: &str) -> bool {
        if !self.enabled {
            return true; // Auth disabled, allow all
        }

        // Check master key
        if let Some(ref master) = self.master_key {
            if key == master {
                return true;
            }
        }

        // Check named keys
        self.keys.iter().any(|k| k.key == key)
    }
}

// ===== MESSAGE PROTOCOL =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentCommand {
    Ping {
        message: String,
    },
    ResearchFlight {
        agent: String,        // "grok", "deepseek", "gemini"
        flight_id: String,
        topics: Vec<String>,  // ["news", "weather", "aviation"]
        model: Option<String>,
        custom_query: Option<String>,
    },
    ChatWithAgent {
        agent: String,
        query: String,
        model: Option<String>,
    },
    AnalyzeBoardingPass {
        image_path: String,
    },
    SearchMemories {
        query: String,
        limit: Option<usize>,
    },
    GetMemoryStats,
    ExportData {
        format: String, // "csv", "json"
    },

    // ===== DATABASE CRUD COMMANDS =====

    // Flight CRUD
    ListFlights {
        user_id: String,
        limit: Option<i32>,
        offset: Option<i32>,
    },
    GetFlight {
        flight_id: String,
    },
    CreateFlight {
        user_id: String,
        flight: FlightInput,
    },
    UpdateFlight {
        flight_id: String,
        flight: FlightInput,
    },
    DeleteFlight {
        flight_id: String,
    },

    // Airport CRUD
    ListAirports,
    GetAirport {
        airport_id: String,
    },
    CreateAirport {
        airport: AirportInput,
    },
    SearchAirports {
        query: String,
        limit: Option<i64>,
    },

    // User Management
    GetUser {
        user_id: String,
    },
    GetPrimaryUser,

    // Statistics
    GetStatistics {
        user_id: String,
    },
    GetAirportStats {
        airport_code: String,
        user_id: String,
    },

    // Research Reports
    ListResearchReports {
        user_id: String,
        limit: Option<i64>,
    },
    GetResearchReport {
        report_id: String,
    },
    SaveResearchReport {
        user_id: String,
        report: ResearchReportInput,
    },

    // Schema & Health
    GetSchema,
    HealthCheck,
}

// ===== INPUT TYPES FOR CRUD =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportInput {
    pub icao_code: Option<String>,
    pub iata_code: Option<String>,
    pub name: String,
    pub city: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum AgentResponse {
    Success {
        data: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    Error {
        error: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        details: Option<String>,
    },
    Streaming {
        event: StreamingEvent,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingEvent {
    AgentStart {
        agent: String,
        operation: String,
    },
    AgentThinking {
        agent: String,
    },
    AgentProgress {
        agent: String,
        progress: String,
    },
    AgentComplete {
        agent: String,
        tokens_used: u32,
        cost_usd: f64,
    },
    AgentError {
        agent: String,
        error: String,
    },
    Log {
        level: String, // "info", "warn", "error"
        message: String,
    },
}

// ===== SERVER STATE =====

pub struct AgentServer {
    port: u16,
    db_path: std::path::PathBuf,
}

impl AgentServer {
    pub fn new(port: u16, db_path: std::path::PathBuf) -> Self {
        Self { port, db_path }
    }

    pub async fn start(self) -> Result<()> {
        let ws_addr: SocketAddr = format!("127.0.0.1:{}", self.port)
            .parse()
            .context("Failed to parse WebSocket address")?;

        let http_port = self.port + 1; // HTTP on port 9529
        let http_addr: SocketAddr = format!("127.0.0.1:{}", http_port)
            .parse()
            .context("Failed to parse HTTP address")?;

        // Spawn HTTP server
        let db_path_http = self.db_path.clone();
        let _http_handle = tokio::spawn(async move {
            if let Err(e) = start_http_server(http_addr, db_path_http).await {
                eprintln!("❌ HTTP server error: {}", e);
            }
        });

        println!("🚀 Flight Tracker Agent Server listening on:");
        println!("   WebSocket: ws://{}", ws_addr);
        println!("   HTTP API:  http://{}", http_addr);
        println!("📡 Ready to receive commands from quantum-local-bridge and mobile clients");

        // Start WebSocket server (blocking)
        let listener = TcpListener::bind(&ws_addr)
            .await
            .context("Failed to bind WebSocket listener")?;

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    println!("✅ New WebSocket connection from {}", peer_addr);
                    let db_path = self.db_path.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, db_path).await {
                            eprintln!("❌ Connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Failed to accept connection: {}", e);
                }
            }
        }
    }
}

async fn handle_connection(stream: TcpStream, db_path: std::path::PathBuf) -> Result<()> {
    let ws_stream = accept_async(stream)
        .await
        .context("WebSocket handshake failed")?;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    println!("🔗 WebSocket connection established");

    while let Some(msg) = ws_receiver.next().await {
        let msg = msg.context("Failed to receive message")?;

        if msg.is_text() || msg.is_binary() {
            let text = msg.to_text().context("Failed to convert message to text")?;

            // Parse command
            let command: AgentCommand = match serde_json::from_str(text) {
                Ok(cmd) => cmd,
                Err(e) => {
                    let error_response = AgentResponse::Error {
                        error: format!("Invalid command format: {}", e),
                        details: Some(text.to_string()),
                    };
                    let error_json = serde_json::to_string(&error_response)?;
                    ws_sender.send(Message::Text(error_json)).await?;
                    continue;
                }
            };

            println!("📨 Received command: {:?}", command);

            // Execute command with streaming support
            if let Err(e) = execute_command(command, &db_path, &mut ws_sender).await {
                let error_response = AgentResponse::Error {
                    error: "Command execution failed".to_string(),
                    details: Some(e.to_string()),
                };
                let error_json = serde_json::to_string(&error_response)?;
                ws_sender.send(Message::Text(error_json)).await?;
            }
        } else if msg.is_close() {
            println!("👋 Client closed connection");
            break;
        }
    }

    Ok(())
}

// ============================================================================
// HTTP REST API Server (for Tailscale mobile clients)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpApiResponse {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct AppState {
    db_path: std::path::PathBuf,
    api_keys: ApiKeyConfig,
}

/// Authentication middleware - checks for valid API key in X-API-Key header
async fn auth_middleware(
    AxumState(state): AxumState<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    // Skip auth check if authentication is disabled
    if !state.api_keys.enabled {
        return next.run(request).await;
    }

    // Get API key from header
    let api_key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Validate the key
    if state.api_keys.validate_key(api_key) {
        next.run(request).await
    } else {
        let response = HttpApiResponse {
            success: false,
            data: None,
            error: Some("Unauthorized: Invalid or missing API key".to_string()),
        };
        (StatusCode::UNAUTHORIZED, Json(response)).into_response()
    }
}

async fn http_health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "Flight Tracker Pro Agent Server",
        "version": "1.0.0",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn http_execute_command(
    AxumState(state): AxumState<Arc<AppState>>,
    Json(command): Json<AgentCommand>,
) -> impl IntoResponse {
    println!("📱 HTTP API: Received command: {:?}", command);

    // Execute command without streaming (HTTP doesn't support WebSocket streaming)
    match execute_command_sync(command, &state.db_path).await {
        Ok(response) => {
            (StatusCode::OK, Json(HttpApiResponse {
                success: true,
                data: Some(response),
                error: None,
            }))
        }
        Err(e) => {
            (StatusCode::BAD_REQUEST, Json(HttpApiResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            }))
        }
    }
}

async fn execute_command_sync(
    command: AgentCommand,
    db_path: &std::path::PathBuf,
) -> Result<serde_json::Value> {
    match command {
        AgentCommand::Ping { message } => {
            Ok(serde_json::json!({
                "pong": message,
                "server": "Flight Tracker Pro Agent Server",
                "version": "1.0.0"
            }))
        }

        AgentCommand::ResearchFlight {
            agent,
            flight_id,
            topics,
            model: _,
            custom_query: _,
        } => {
            // Call the actual research function based on agent type
            match agent.to_lowercase().as_str() {
                "grok" => {
                    Ok(serde_json::json!({
                        "agent": "grok",
                        "flight_id": flight_id,
                        "topics": topics,
                        "findings": "Grok research integration pending"
                    }))
                }
                "deepseek" => {
                    Ok(serde_json::json!({
                        "agent": "deepseek",
                        "flight_id": flight_id,
                        "topics": topics,
                        "findings": "DeepSeek research integration pending"
                    }))
                }
                "gemini" => {
                    Ok(serde_json::json!({
                        "agent": "gemini",
                        "flight_id": flight_id,
                        "topics": topics,
                        "findings": "Gemini research integration pending"
                    }))
                }
                _ => Err(anyhow::anyhow!("Unknown agent: {}", agent)),
            }
        }

        AgentCommand::ChatWithAgent { agent, query, model: _ } => {
            Ok(serde_json::json!({
                "agent": agent,
                "query": query,
                "response": format!("Chat response from {} (integration pending)", agent)
            }))
        }

        AgentCommand::SearchMemories { query, limit } => {
            let db = Database::new(db_path.clone())?;
            let conn = db.get_connection();

            match crate::agent_memory::search_memories(conn, &query, limit.unwrap_or(10)) {
                Ok(results) => {
                    Ok(serde_json::json!({
                        "query": query,
                        "count": results.len(),
                        "results": results
                    }))
                }
                Err(e) => Err(anyhow::anyhow!("Memory search failed: {}", e)),
            }
        }

        AgentCommand::GetMemoryStats => {
            let db = Database::new(db_path.clone())?;
            let conn = db.get_connection();

            match crate::agent_memory::get_memory_stats(conn) {
                Ok(stats) => Ok(serde_json::to_value(stats)?),
                Err(e) => Err(anyhow::anyhow!("Failed to get stats: {}", e)),
            }
        }

        // ===== DATABASE CRUD HANDLERS =====

        AgentCommand::HealthCheck => {
            Ok(serde_json::json!({
                "status": "healthy",
                "server": "Flight Tracker Pro Agent Server",
                "version": "1.0.0",
                "database": db_path.display().to_string(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }

        AgentCommand::GetSchema => {
            Ok(serde_json::json!({
                "tables": [
                    {"name": "users", "description": "User accounts"},
                    {"name": "flights", "description": "Flight records"},
                    {"name": "airports", "description": "Airport information"},
                    {"name": "aircraft_types", "description": "Aircraft models"},
                    {"name": "journeys", "description": "Trip/journey groupings"},
                    {"name": "frequent_flyer_programs", "description": "FFP accounts"},
                    {"name": "pilot_logbook_entries", "description": "Pilot flight logs"},
                    {"name": "research_reports", "description": "AI research reports"},
                    {"name": "agent_memory", "description": "AI agent memory storage"},
                    {"name": "custom_documents", "description": "User documents"},
                    {"name": "passenger_mappings", "description": "Passenger name mappings"}
                ],
                "commands": [
                    "PING", "HEALTH_CHECK", "GET_SCHEMA",
                    "LIST_FLIGHTS", "GET_FLIGHT", "CREATE_FLIGHT", "UPDATE_FLIGHT", "DELETE_FLIGHT",
                    "LIST_AIRPORTS", "GET_AIRPORT", "CREATE_AIRPORT", "SEARCH_AIRPORTS",
                    "GET_USER", "GET_PRIMARY_USER",
                    "GET_STATISTICS", "GET_AIRPORT_STATS",
                    "LIST_RESEARCH_REPORTS", "GET_RESEARCH_REPORT", "SAVE_RESEARCH_REPORT",
                    "SEARCH_MEMORIES", "GET_MEMORY_STATS",
                    "RESEARCH_FLIGHT", "CHAT_WITH_AGENT", "EXPORT_DATA"
                ]
            }))
        }

        AgentCommand::ListFlights { user_id, limit, offset } => {
            let db = Database::new(db_path.clone())?;
            let flights = db.list_flights(&user_id, limit.unwrap_or(1000), offset.unwrap_or(0))
                .map_err(|e| anyhow::anyhow!("Failed to list flights: {}", e))?;

            let count = flights.len();
            Ok(serde_json::json!({
                "flights": flights,
                "count": count
            }))
        }

        AgentCommand::GetFlight { flight_id } => {
            let db = Database::new(db_path.clone())?;
            match db.get_flight(&flight_id) {
                Ok(Some(flight)) => Ok(serde_json::to_value(flight)?),
                Ok(None) => Err(anyhow::anyhow!("Flight not found: {}", flight_id)),
                Err(e) => Err(anyhow::anyhow!("Failed to get flight: {}", e)),
            }
        }

        AgentCommand::CreateFlight { user_id, flight } => {
            let db = Database::new(db_path.clone())?;
            let flight_id = db.create_flight(&user_id, &flight)
                .map_err(|e| anyhow::anyhow!("Failed to create flight: {}", e))?;

            Ok(serde_json::json!({
                "success": true,
                "flight_id": flight_id,
                "message": "Flight created successfully"
            }))
        }

        AgentCommand::DeleteFlight { flight_id } => {
            let db = Database::new(db_path.clone())?;
            db.delete_flight(&flight_id)
                .map_err(|e| anyhow::anyhow!("Failed to delete flight: {}", e))?;

            Ok(serde_json::json!({
                "success": true,
                "message": format!("Flight {} deleted", flight_id)
            }))
        }

        AgentCommand::UpdateFlight { flight_id, flight } => {
            // For now, we implement update as a note that the full update would require more params
            let db = Database::new(db_path.clone())?;
            // Check flight exists
            match db.get_flight(&flight_id) {
                Ok(Some(_)) => {
                    // Note: Full update would need individual field updates
                    // For now, return the flight data that would be updated
                    Ok(serde_json::json!({
                        "success": true,
                        "flight_id": flight_id,
                        "updates": flight,
                        "message": "Update received (full implementation requires database schema changes)"
                    }))
                }
                Ok(None) => Err(anyhow::anyhow!("Flight not found: {}", flight_id)),
                Err(e) => Err(anyhow::anyhow!("Failed to get flight: {}", e)),
            }
        }

        AgentCommand::ListAirports => {
            let db = Database::new(db_path.clone())?;
            let airports = db.list_all_airports()
                .map_err(|e| anyhow::anyhow!("Failed to list airports: {}", e))?;

            Ok(serde_json::json!({
                "airports": airports,
                "count": airports.len()
            }))
        }

        AgentCommand::GetAirport { airport_id } => {
            let db = Database::new(db_path.clone())?;
            match db.get_airport(&airport_id) {
                Ok(Some(airport)) => Ok(serde_json::to_value(airport)?),
                Ok(None) => Err(anyhow::anyhow!("Airport not found: {}", airport_id)),
                Err(e) => Err(anyhow::anyhow!("Failed to get airport: {}", e)),
            }
        }

        AgentCommand::CreateAirport { airport } => {
            let db = Database::new(db_path.clone())?;
            let airport_id = db.create_airport(
                airport.icao_code.as_deref(),
                airport.iata_code.as_deref(),
                &airport.name,
                airport.city.as_deref(),
                airport.country.as_deref(),
                airport.latitude,
                airport.longitude,
                airport.timezone.as_deref(),
            ).map_err(|e| anyhow::anyhow!("Failed to create airport: {}", e))?;

            Ok(serde_json::json!({
                "success": true,
                "airport_id": airport_id,
                "message": "Airport created successfully"
            }))
        }

        AgentCommand::SearchAirports { query, limit } => {
            let db = Database::new(db_path.clone())?;
            let all_airports = db.list_all_airports()
                .map_err(|e| anyhow::anyhow!("Failed to list airports: {}", e))?;

            // Simple search by code or name
            let query_lower = query.to_lowercase();
            let results: Vec<_> = all_airports.into_iter()
                .filter(|a| {
                    a.iata_code.as_ref().map(|c| c.to_lowercase().contains(&query_lower)).unwrap_or(false) ||
                    a.icao_code.as_ref().map(|c| c.to_lowercase().contains(&query_lower)).unwrap_or(false) ||
                    a.name.to_lowercase().contains(&query_lower) ||
                    a.city.as_ref().map(|c| c.to_lowercase().contains(&query_lower)).unwrap_or(false)
                })
                .take(limit.unwrap_or(20) as usize)
                .collect();

            Ok(serde_json::json!({
                "query": query,
                "airports": results,
                "count": results.len()
            }))
        }

        AgentCommand::GetUser { user_id } => {
            let db = Database::new(db_path.clone())?;
            match db.get_user(&user_id) {
                Ok(Some(user)) => Ok(serde_json::to_value(user)?),
                Ok(None) => Err(anyhow::anyhow!("User not found: {}", user_id)),
                Err(e) => Err(anyhow::anyhow!("Failed to get user: {}", e)),
            }
        }

        AgentCommand::GetPrimaryUser => {
            let db = Database::new(db_path.clone())?;
            match db.get_primary_user() {
                Ok(Some(user)) => Ok(serde_json::to_value(user)?),
                Ok(None) => Err(anyhow::anyhow!("No primary user found")),
                Err(e) => Err(anyhow::anyhow!("Failed to get primary user: {}", e)),
            }
        }

        AgentCommand::GetStatistics { user_id } => {
            let db = Database::new(db_path.clone())?;
            let stats = db.get_statistics(&user_id)
                .map_err(|e| anyhow::anyhow!("Failed to get statistics: {}", e))?;
            Ok(serde_json::to_value(stats)?)
        }

        AgentCommand::GetAirportStats { airport_code, user_id } => {
            let db = Database::new(db_path.clone())?;
            let flights = db.list_flights(&user_id, 10000, 0)
                .map_err(|e| anyhow::anyhow!("Failed to list flights: {}", e))?;

            let departures: Vec<_> = flights.iter()
                .filter(|f| f.departure_airport == airport_code)
                .collect();
            let arrivals: Vec<_> = flights.iter()
                .filter(|f| f.arrival_airport == airport_code)
                .collect();

            Ok(serde_json::json!({
                "airport_code": airport_code,
                "total_departures": departures.len(),
                "total_arrivals": arrivals.len(),
                "total_flights": departures.len() + arrivals.len()
            }))
        }

        AgentCommand::ListResearchReports { user_id, limit } => {
            let db = Database::new(db_path.clone())?;
            let reports = db.list_research_reports(&user_id, limit.unwrap_or(50), 0)
                .map_err(|e| anyhow::anyhow!("Failed to list reports: {}", e))?;

            Ok(serde_json::json!({
                "reports": reports,
                "count": reports.len()
            }))
        }

        AgentCommand::GetResearchReport { report_id } => {
            let db = Database::new(db_path.clone())?;
            match db.get_research_report(&report_id) {
                Ok(Some(report)) => Ok(serde_json::to_value(report)?),
                Ok(None) => Err(anyhow::anyhow!("Report not found: {}", report_id)),
                Err(e) => Err(anyhow::anyhow!("Failed to get report: {}", e)),
            }
        }

        AgentCommand::SaveResearchReport { user_id, report } => {
            let db = Database::new(db_path.clone())?;
            let report_id = db.save_research_report(&user_id, &report)
                .map_err(|e| anyhow::anyhow!("Failed to save report: {}", e))?;

            Ok(serde_json::json!({
                "success": true,
                "report_id": report_id,
                "message": "Research report saved"
            }))
        }

        _ => Err(anyhow::anyhow!("Command not supported via HTTP API")),
    }
}

async fn start_http_server(addr: SocketAddr, db_path: std::path::PathBuf) -> Result<()> {
    // Load API key configuration
    let api_keys = ApiKeyConfig::load();
    let state = Arc::new(AppState { db_path, api_keys });

    // Health endpoint is public (no auth required)
    // Command endpoint requires authentication if enabled
    let app = Router::new()
        .route("/api/health", get(http_health))
        .route("/api/command", post(http_execute_command))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("🌐 HTTP API server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

// ============================================================================
// WebSocket Command Execution (with streaming support)
// ============================================================================

async fn execute_command(
    command: AgentCommand,
    db_path: &std::path::PathBuf,
    ws_sender: &mut futures::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
) -> Result<()> {
    match command {
        AgentCommand::Ping { message } => {
            let response = AgentResponse::Success {
                data: serde_json::json!({
                    "pong": message,
                    "server": "Flight Tracker Pro Agent Server",
                    "version": "1.0.0"
                }),
                message: Some("PONG - Flight Tracker Agent Server is alive!".to_string()),
            };
            send_response(ws_sender, response).await?;
        }

        AgentCommand::ResearchFlight {
            agent,
            flight_id,
            topics,
            model,
            custom_query,
        } => {
            // Send streaming start event
            send_streaming_event(
                ws_sender,
                StreamingEvent::AgentStart {
                    agent: agent.clone(),
                    operation: format!("Research flight {} on topics: {:?}", flight_id, topics),
                },
            )
            .await?;

            // Send thinking event
            send_streaming_event(
                ws_sender,
                StreamingEvent::AgentThinking {
                    agent: agent.clone(),
                },
            )
            .await?;

            // Send progress event
            send_streaming_event(
                ws_sender,
                StreamingEvent::AgentProgress {
                    agent: agent.clone(),
                    progress: format!("Researching flight {} with {}", flight_id, agent),
                },
            )
            .await?;

            // Call the actual research function based on agent type
            let result = match agent.to_lowercase().as_str() {
                "grok" => {
                    // TODO: Call actual Grok research function
                    // For now, return placeholder
                    Ok(serde_json::json!({
                        "agent": "grok",
                        "flight_id": flight_id,
                        "topics": topics,
                        "findings": "Grok research integration pending"
                    }))
                }
                "deepseek" => {
                    // TODO: Call actual DeepSeek research function
                    Ok(serde_json::json!({
                        "agent": "deepseek",
                        "flight_id": flight_id,
                        "topics": topics,
                        "findings": "DeepSeek research integration pending"
                    }))
                }
                "gemini" => {
                    // TODO: Call actual Gemini research function
                    Ok(serde_json::json!({
                        "agent": "gemini",
                        "flight_id": flight_id,
                        "topics": topics,
                        "findings": "Gemini research integration pending"
                    }))
                }
                _ => Err(anyhow::anyhow!("Unknown agent: {}", agent)),
            };

            match result {
                Ok(data) => {
                    // Store research results in agent memory for future recall
                    if let Ok(db) = Database::new(db_path.clone()) {
                        let conn = db.get_connection();
                        let content = serde_json::to_string_pretty(&data).unwrap_or_default();
                        let summary = format!(
                            "Research on flight {} covering topics: {}",
                            flight_id,
                            topics.join(", ")
                        );

                        let _ = crate::agent_memory::store_memory(
                            conn,
                            &agent,
                            "research",
                            &content,
                            custom_query.as_deref(),
                            Some(&summary),
                            Some(&flight_id),
                            None, // user_id
                            model.as_deref(),
                            0,   // TODO: Track actual token usage
                            0.0, // TODO: Calculate actual cost
                            Some(0.95), // High confidence for successful research
                            Some(24), // Cache for 24 hours
                        );

                        println!("💾 Stored research results in agent memory");
                    }

                    // Send completion event
                    send_streaming_event(
                        ws_sender,
                        StreamingEvent::AgentComplete {
                            agent: agent.clone(),
                            tokens_used: 0, // TODO: Track actual token usage
                            cost_usd: 0.0,  // TODO: Calculate actual cost
                        },
                    )
                    .await?;

                    // Send final success response
                    send_response(
                        ws_sender,
                        AgentResponse::Success {
                            data,
                            message: Some(format!("Research completed by {}", agent)),
                        },
                    )
                    .await?;
                }
                Err(e) => {
                    // Send error event
                    send_streaming_event(
                        ws_sender,
                        StreamingEvent::AgentError {
                            agent: agent.clone(),
                            error: e.to_string(),
                        },
                    )
                    .await?;

                    // Send final error response
                    send_response(
                        ws_sender,
                        AgentResponse::Error {
                            error: format!("Research failed: {}", e),
                            details: None,
                        },
                    )
                    .await?;
                }
            }
        }

        AgentCommand::ChatWithAgent { agent, query, model } => {
            // Send streaming start event
            send_streaming_event(
                ws_sender,
                StreamingEvent::AgentStart {
                    agent: agent.clone(),
                    operation: format!("Chat query: {}", query),
                },
            )
            .await?;

            // TODO: Implement actual chat with agent
            let response_data = serde_json::json!({
                "agent": agent,
                "query": query,
                "response": "Chat functionality coming soon"
            });

            // Store chat interaction in agent memory
            if let Ok(db) = Database::new(db_path.clone()) {
                let conn = db.get_connection();
                let content = format!("Query: {}\nResponse: Chat functionality coming soon", query);

                let _ = crate::agent_memory::store_memory(
                    conn,
                    &agent,
                    "chat",
                    &content,
                    Some(&query),
                    Some("Chat interaction"),
                    None, // flight_id
                    None, // user_id
                    model.as_deref(),
                    0,   // TODO: Track actual token usage
                    0.0, // TODO: Calculate actual cost
                    Some(0.8), // Medium confidence for placeholder response
                    Some(24), // Cache for 24 hours
                );

                println!("💾 Stored chat interaction in agent memory");
            }

            send_response(
                ws_sender,
                AgentResponse::Success {
                    data: response_data,
                    message: Some(format!("Chat with {} queued", agent)),
                },
            )
            .await?;
        }

        AgentCommand::AnalyzeBoardingPass { image_path } => {
            send_response(
                ws_sender,
                AgentResponse::Success {
                    data: serde_json::json!({
                        "image_path": image_path,
                        "status": "OCR analysis queued"
                    }),
                    message: Some("Boarding pass analysis queued".to_string()),
                },
            )
            .await?;
        }

        AgentCommand::SearchMemories { query, limit } => {
            // Use the memory search functions
            let response = match Database::new(db_path.clone()) {
                Ok(db) => {
                    let conn = db.get_connection();
                    match crate::agent_memory::search_memories(conn, &query, limit.unwrap_or(20)) {
                        Ok(results) => AgentResponse::Success {
                            data: serde_json::json!({
                                "results": results,
                                "count": results.len()
                            }),
                            message: Some(format!("Found {} memories", results.len())),
                        },
                        Err(e) => AgentResponse::Error {
                            error: "Memory search failed".to_string(),
                            details: Some(e.to_string()),
                        },
                    }
                }
                Err(e) => AgentResponse::Error {
                    error: "Database connection failed".to_string(),
                    details: Some(e.to_string()),
                },
            };
            send_response(ws_sender, response).await?;
        }

        AgentCommand::GetMemoryStats => {
            let response = match Database::new(db_path.clone()) {
                Ok(db) => {
                    let conn = db.get_connection();
                    match crate::agent_memory::get_memory_stats(conn) {
                        Ok(stats) => AgentResponse::Success {
                            data: serde_json::json!(stats),
                            message: Some("Memory statistics retrieved".to_string()),
                        },
                        Err(e) => AgentResponse::Error {
                            error: "Failed to get memory stats".to_string(),
                            details: Some(e.to_string()),
                        },
                    }
                }
                Err(e) => AgentResponse::Error {
                    error: "Database connection failed".to_string(),
                    details: Some(e.to_string()),
                },
            };
            send_response(ws_sender, response).await?;
        }

        AgentCommand::ExportData { format } => {
            send_response(
                ws_sender,
                AgentResponse::Success {
                    data: serde_json::json!({
                        "format": format,
                        "status": "Export queued"
                    }),
                    message: Some(format!("Data export ({}) queued", format)),
                },
            )
            .await?;
        }

        // All database CRUD commands use the sync handler
        cmd => {
            match execute_command_sync(cmd, db_path).await {
                Ok(data) => {
                    send_response(
                        ws_sender,
                        AgentResponse::Success {
                            data,
                            message: Some("Command executed successfully".to_string()),
                        },
                    )
                    .await?;
                }
                Err(e) => {
                    send_response(
                        ws_sender,
                        AgentResponse::Error {
                            error: e.to_string(),
                            details: None,
                        },
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}

// ===== HELPER FUNCTIONS =====

/// Send a standard response message
async fn send_response(
    ws_sender: &mut futures::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
    response: AgentResponse,
) -> Result<()> {
    let json = serde_json::to_string(&response)?;
    ws_sender.send(Message::Text(json)).await?;
    Ok(())
}

/// Send a streaming event
async fn send_streaming_event(
    ws_sender: &mut futures::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, Message>,
    event: StreamingEvent,
) -> Result<()> {
    let response = AgentResponse::Streaming { event };
    send_response(ws_sender, response).await
}

// ===== BACKGROUND SERVER TASK =====

pub fn spawn_agent_server(db_path: std::path::PathBuf) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let server = AgentServer::new(9528, db_path);
        if let Err(e) = server.start().await {
            eprintln!("❌ Agent server failed: {}", e);
        }
    })
}
