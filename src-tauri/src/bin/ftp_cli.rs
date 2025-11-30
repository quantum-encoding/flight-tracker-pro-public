// ftp-cli: Command-line interface for Flight Tracker Pro
// Allows AI agents and scripts to interact with the database via the agent server
//
// Usage:
//   ftp-cli --help                          Show help
//   ftp-cli ping                            Test connection
//   ftp-cli schema                          Show database schema
//   ftp-cli list-flights <user_id>          List flights for user
//   ftp-cli get-flight <flight_id>          Get flight details
//   ftp-cli stats <user_id>                 Get user statistics
//   ftp-cli search-airports <query>         Search airports
//
// Environment:
//   FTP_API_URL    Override API endpoint (default: http://127.0.0.1:9529)
//   FTP_API_KEY    API key for authentication (required if server has auth enabled)

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::error::Error;

const DEFAULT_API_URL: &str = "http://127.0.0.1:9529";

#[derive(Parser)]
#[command(name = "ftp-cli")]
#[command(author = "Quantum Encoding LTD")]
#[command(version = "1.0.0")]
#[command(about = "Flight Tracker Pro CLI - Agent interface for database access")]
#[command(long_about = r#"
Flight Tracker Pro Command-Line Interface

A powerful CLI tool for AI agents and automation scripts to interact with
Flight Tracker Pro's database through its HTTP API server.

The application must be running with the agent server enabled for commands to work.
Default API endpoint: http://127.0.0.1:9529

Environment Variables:
  FTP_API_URL    Override the API endpoint URL
  FTP_API_KEY    API key for authentication (future feature)

Examples:
  ftp-cli ping                           Test server connection
  ftp-cli schema                         Display database schema
  ftp-cli list-flights user123 --limit 10
  ftp-cli get-flight abc-123-def
  ftp-cli search-airports "London"
  ftp-cli stats user123
"#)]
struct Cli {
    /// API server URL (overrides FTP_API_URL)
    #[arg(short, long, global = true)]
    url: Option<String>,

    /// API key for authentication (overrides FTP_API_KEY)
    #[arg(short = 'k', long, global = true)]
    api_key: Option<String>,

    /// Output format
    #[arg(short, long, global = true, default_value = "pretty")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    /// Pretty-printed JSON
    Pretty,
    /// Compact JSON (for piping)
    Json,
    /// Plain text (human readable)
    Text,
}

#[derive(Subcommand)]
enum Commands {
    /// Test connection to the server
    Ping,

    /// Show database schema and available commands
    Schema,

    /// Health check - detailed server status
    Health,

    /// List flights for a user
    ListFlights {
        /// User ID
        user_id: String,

        /// Maximum number of flights to return
        #[arg(short, long)]
        limit: Option<i32>,

        /// Offset for pagination
        #[arg(short, long)]
        offset: Option<i32>,
    },

    /// Get details of a specific flight
    GetFlight {
        /// Flight ID
        flight_id: String,
    },

    /// Create a new flight
    CreateFlight {
        /// User ID
        user_id: String,

        /// Departure airport code
        #[arg(long)]
        from: String,

        /// Arrival airport code
        #[arg(long)]
        to: String,

        /// Departure datetime (ISO 8601)
        #[arg(long)]
        departure: String,

        /// Arrival datetime (ISO 8601, optional)
        #[arg(long)]
        arrival: Option<String>,

        /// Flight number (optional)
        #[arg(long)]
        flight_number: Option<String>,

        /// Notes (optional)
        #[arg(long)]
        notes: Option<String>,
    },

    /// Delete a flight
    DeleteFlight {
        /// Flight ID
        flight_id: String,
    },

    /// List all airports
    ListAirports,

    /// Search airports by name, code, or city
    SearchAirports {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long)]
        limit: Option<i64>,
    },

    /// Get airport details
    GetAirport {
        /// Airport ID
        airport_id: String,
    },

    /// Create a new airport
    CreateAirport {
        /// Airport name
        name: String,

        /// ICAO code (optional)
        #[arg(long)]
        icao: Option<String>,

        /// IATA code (optional)
        #[arg(long)]
        iata: Option<String>,

        /// City (optional)
        #[arg(long)]
        city: Option<String>,

        /// Country (optional)
        #[arg(long)]
        country: Option<String>,

        /// Latitude (optional)
        #[arg(long)]
        lat: Option<f64>,

        /// Longitude (optional)
        #[arg(long)]
        lon: Option<f64>,
    },

    /// Get user details
    GetUser {
        /// User ID
        user_id: String,
    },

    /// Get the primary user
    GetPrimaryUser,

    /// Get statistics for a user
    Stats {
        /// User ID
        user_id: String,
    },

    /// Get statistics for an airport
    AirportStats {
        /// Airport code
        airport_code: String,

        /// User ID
        user_id: String,
    },

    /// List research reports
    ListReports {
        /// User ID
        user_id: String,

        /// Maximum results
        #[arg(short, long)]
        limit: Option<i64>,
    },

    /// Get a research report
    GetReport {
        /// Report ID
        report_id: String,
    },

    /// Search agent memories
    SearchMemories {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Get memory statistics
    MemoryStats,

    /// Send a raw JSON command
    Raw {
        /// JSON command body
        json: String,
    },
}

#[derive(Serialize)]
#[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
enum AgentCommand {
    Ping { message: String },
    HealthCheck,
    GetSchema,
    ListFlights { user_id: String, limit: Option<i32>, offset: Option<i32> },
    GetFlight { flight_id: String },
    CreateFlight { user_id: String, flight: FlightInput },
    DeleteFlight { flight_id: String },
    ListAirports,
    GetAirport { airport_id: String },
    CreateAirport { airport: AirportInput },
    SearchAirports { query: String, limit: Option<i64> },
    GetUser { user_id: String },
    GetPrimaryUser,
    GetStatistics { user_id: String },
    GetAirportStats { airport_code: String, user_id: String },
    ListResearchReports { user_id: String, limit: Option<i64> },
    GetResearchReport { report_id: String },
    SearchMemories { query: String, limit: Option<usize> },
    GetMemoryStats,
}

#[derive(Serialize)]
struct FlightInput {
    departure_airport: String,
    arrival_airport: String,
    departure_datetime: String,
    arrival_datetime: Option<String>,
    flight_number: Option<String>,
    notes: Option<String>,
    // Include other optional fields as None
    aircraft_type_id: Option<String>,
    aircraft_registration: Option<String>,
    total_duration: Option<i32>,
    flight_duration: Option<i32>,
    distance_nm: Option<f64>,
    distance_km: Option<f64>,
    carbon_emissions_kg: Option<f64>,
    booking_reference: Option<String>,
    ticket_number: Option<String>,
    seat_number: Option<String>,
    fare_class: Option<String>,
    base_fare: Option<f64>,
    taxes: Option<f64>,
    total_cost: Option<f64>,
    currency: Option<String>,
    attachment_path: Option<String>,
}

#[derive(Serialize)]
struct AirportInput {
    name: String,
    icao_code: Option<String>,
    iata_code: Option<String>,
    city: Option<String>,
    country: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    timezone: Option<String>,
}

#[derive(Deserialize)]
struct ApiResponse {
    success: bool,
    data: Option<serde_json::Value>,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Determine API URL from args, env, or default
    let api_url = cli.url
        .or_else(|| std::env::var("FTP_API_URL").ok())
        .unwrap_or_else(|| DEFAULT_API_URL.to_string());

    // Get API key from args or environment
    let api_key = cli.api_key
        .or_else(|| std::env::var("FTP_API_KEY").ok());

    let command = match cli.command {
        Commands::Ping => AgentCommand::Ping {
            message: "CLI health check".to_string(),
        },
        Commands::Health => AgentCommand::HealthCheck,
        Commands::Schema => AgentCommand::GetSchema,
        Commands::ListFlights { user_id, limit, offset } => {
            AgentCommand::ListFlights { user_id, limit, offset }
        }
        Commands::GetFlight { flight_id } => {
            AgentCommand::GetFlight { flight_id }
        }
        Commands::CreateFlight { user_id, from, to, departure, arrival, flight_number, notes } => {
            AgentCommand::CreateFlight {
                user_id,
                flight: FlightInput {
                    departure_airport: from,
                    arrival_airport: to,
                    departure_datetime: departure,
                    arrival_datetime: arrival,
                    flight_number,
                    notes,
                    aircraft_type_id: None,
                    aircraft_registration: None,
                    total_duration: None,
                    flight_duration: None,
                    distance_nm: None,
                    distance_km: None,
                    carbon_emissions_kg: None,
                    booking_reference: None,
                    ticket_number: None,
                    seat_number: None,
                    fare_class: None,
                    base_fare: None,
                    taxes: None,
                    total_cost: None,
                    currency: None,
                    attachment_path: None,
                },
            }
        }
        Commands::DeleteFlight { flight_id } => {
            AgentCommand::DeleteFlight { flight_id }
        }
        Commands::ListAirports => AgentCommand::ListAirports,
        Commands::GetAirport { airport_id } => {
            AgentCommand::GetAirport { airport_id }
        }
        Commands::CreateAirport { name, icao, iata, city, country, lat, lon } => {
            AgentCommand::CreateAirport {
                airport: AirportInput {
                    name,
                    icao_code: icao,
                    iata_code: iata,
                    city,
                    country,
                    latitude: lat,
                    longitude: lon,
                    timezone: None,
                },
            }
        }
        Commands::SearchAirports { query, limit } => {
            AgentCommand::SearchAirports { query, limit }
        }
        Commands::GetUser { user_id } => {
            AgentCommand::GetUser { user_id }
        }
        Commands::GetPrimaryUser => AgentCommand::GetPrimaryUser,
        Commands::Stats { user_id } => {
            AgentCommand::GetStatistics { user_id }
        }
        Commands::AirportStats { airport_code, user_id } => {
            AgentCommand::GetAirportStats { airport_code, user_id }
        }
        Commands::ListReports { user_id, limit } => {
            AgentCommand::ListResearchReports { user_id, limit }
        }
        Commands::GetReport { report_id } => {
            AgentCommand::GetResearchReport { report_id }
        }
        Commands::SearchMemories { query, limit } => {
            AgentCommand::SearchMemories { query, limit }
        }
        Commands::MemoryStats => AgentCommand::GetMemoryStats,
        Commands::Raw { json } => {
            // Parse and send raw JSON
            let value: serde_json::Value = serde_json::from_str(&json)?;
            return send_raw_command(&api_url, api_key.as_deref(), value, cli.format).await;
        }
    };

    // Send the command
    let result = send_command(&api_url, api_key.as_deref(), &command, cli.format).await;

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1)
        }
    }
}

async fn send_command(
    api_url: &str,
    api_key: Option<&str>,
    command: &AgentCommand,
    format: OutputFormat,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/command", api_url);

    let mut request = client.post(&url).json(command);

    // Add API key header if provided
    if let Some(key) = api_key {
        request = request.header("X-API-Key", key);
    }

    let response = request
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                format!("Connection failed. Is Flight Tracker Pro running?\nEndpoint: {}", url)
            } else {
                format!("Request failed: {}", e)
            }
        })?;

    let status = response.status();
    let api_response: ApiResponse = response.json().await?;

    if api_response.success {
        output_data(api_response.data.unwrap_or(serde_json::Value::Null), format);
        Ok(())
    } else {
        Err(format!(
            "API Error ({}): {}",
            status,
            api_response.error.unwrap_or_else(|| "Unknown error".to_string())
        ).into())
    }
}

async fn send_raw_command(
    api_url: &str,
    api_key: Option<&str>,
    command: serde_json::Value,
    format: OutputFormat,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/command", api_url);

    let mut request = client.post(&url).json(&command);

    // Add API key header if provided
    if let Some(key) = api_key {
        request = request.header("X-API-Key", key);
    }

    let response = request
        .send()
        .await
        .map_err(|e| {
            if e.is_connect() {
                format!("Connection failed. Is Flight Tracker Pro running?\nEndpoint: {}", url)
            } else {
                format!("Request failed: {}", e)
            }
        })?;

    let status = response.status();
    let api_response: ApiResponse = response.json().await?;

    if api_response.success {
        output_data(api_response.data.unwrap_or(serde_json::Value::Null), format);
        Ok(())
    } else {
        Err(format!(
            "API Error ({}): {}",
            status,
            api_response.error.unwrap_or_else(|| "Unknown error".to_string())
        ).into())
    }
}

fn output_data(data: serde_json::Value, format: OutputFormat) {
    match format {
        OutputFormat::Pretty => {
            println!("{}", serde_json::to_string_pretty(&data).unwrap());
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string(&data).unwrap());
        }
        OutputFormat::Text => {
            output_as_text(&data, 0);
        }
    }
}

fn output_as_text(value: &serde_json::Value, indent: usize) {
    let prefix = "  ".repeat(indent);
    match value {
        serde_json::Value::Null => println!("{}null", prefix),
        serde_json::Value::Bool(b) => println!("{}{}", prefix, b),
        serde_json::Value::Number(n) => println!("{}{}", prefix, n),
        serde_json::Value::String(s) => println!("{}{}", prefix, s),
        serde_json::Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                println!("{}[{}]:", prefix, i);
                output_as_text(item, indent + 1);
            }
        }
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                if val.is_object() || val.is_array() {
                    println!("{}{}:", prefix, key);
                    output_as_text(val, indent + 1);
                } else {
                    println!("{}{}: {}", prefix, key, format_simple_value(val));
                }
            }
        }
    }
}

fn format_simple_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}
