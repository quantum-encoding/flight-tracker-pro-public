// Flight investigation commands
use tauri::State;
use super::AppState;
use crate::models::{Investigation, InvestigationResult};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

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

#[tauri::command]
pub async fn investigate_flight(
    flight_id: String,
    passenger_names: Vec<String>,
    model: Option<String>, // Optional: "gemini", "deepseek", or "grok"
    state: State<'_, AppState>,
) -> Result<InvestigationResult, String> {
    use crate::investigation;

    // Get the flight details
    let flight = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_flight(&flight_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Flight not found".to_string())?
    };

    // Extract investigation parameters
    let location = format!("{} to {}", flight.departure_airport, flight.arrival_airport);
    let date = flight
        .departure_datetime
        .split('T')
        .next()
        .unwrap_or(&flight.departure_datetime)
        .to_string();

    // Select investigation provider based on model parameter
    let selected_model = model.as_deref().unwrap_or("gemini");

    let result = match selected_model {
        "deepseek" => {
            // Use DeepSeek for investigation
            let api_key = get_api_key(
                &["DEEPSEEK_API_KEY"],
                "deepseek_api_key",
                &state,
            )?;

            crate::deepseek::run_deepseek_investigation(
                passenger_names,
                location,
                date,
                api_key,
            )
            .await
            .map_err(|e| format!("DeepSeek investigation failed: {}", e))?
        }
        "grok" => {
            // Use Grok for investigation with agentic search
            let api_key = get_api_key(
                &["GROK_API_KEY", "XAI_API_KEY"],
                "grok_api_key",
                &state,
            )?;

            crate::grok::run_grok_investigation(
                passenger_names,
                location,
                date,
                api_key,
            )
            .await
            .map_err(|e| format!("Grok investigation failed: {}", e))?
        }
        _ => {
            // Default to Gemini
            let api_key = get_api_key(
                &["GENAI_API_KEY", "GOOGLE_GENAI_API_KEY", "GEMINI_API_KEY"],
                "gemini_api_key",
                &state,
            )?;

            investigation::run_investigation(passenger_names, location, date, api_key)
                .await
                .map_err(|e| format!("Gemini investigation failed: {}", e))?
        }
    };

    // Store in database cache (optional - for now just return)
    // TODO: Save to investigations table

    Ok(result)
}

#[tauri::command]
pub fn get_flight_investigation(
    flight_id: String,
    state: State<'_, AppState>,
) -> Result<Option<Investigation>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let investigation = db
        .conn
        .query_row(
            "SELECT id, flight_id, user_id, passenger_names, location, investigation_date,
                    generated_queries, status, ai_summary, sources_json, corroboration_score,
                    error_message, processing_time_ms, created_at, completed_at
             FROM investigations
             WHERE flight_id = ?1
             ORDER BY created_at DESC
             LIMIT 1",
            rusqlite::params![flight_id],
            |row| {
                Ok(Investigation {
                    id: row.get(0)?,
                    flight_id: row.get(1)?,
                    user_id: row.get(2)?,
                    passenger_names: row.get(3)?,
                    location: row.get(4)?,
                    investigation_date: row.get(5)?,
                    generated_queries: row.get(6)?,
                    status: row.get(7)?,
                    ai_summary: row.get(8)?,
                    sources_json: row.get(9)?,
                    corroboration_score: row.get(10)?,
                    error_message: row.get(11)?,
                    processing_time_ms: row.get(12)?,
                    created_at: row.get(13)?,
                    completed_at: row.get(14)?,
                })
            },
        )
        .optional()
        .map_err(|e: rusqlite::Error| e.to_string())?;

    Ok(investigation)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AirportVisit {
    pub airport_code: String,
    pub visit_count: i32,
    pub departure_count: i32,
    pub arrival_count: i32,
}

#[tauri::command]
pub fn get_airport_list(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AirportVisit>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT
            airport_code,
            SUM(CASE WHEN type = 'departure' THEN 1 ELSE 0 END) as departure_count,
            SUM(CASE WHEN type = 'arrival' THEN 1 ELSE 0 END) as arrival_count,
            COUNT(*) as total_visits
         FROM (
            SELECT departure_airport as airport_code, 'departure' as type FROM flights WHERE user_id = ?1
            UNION ALL
            SELECT arrival_airport as airport_code, 'arrival' as type FROM flights WHERE user_id = ?1
         )
         GROUP BY airport_code
         ORDER BY total_visits DESC, airport_code ASC"
    ).map_err(|e| e.to_string())?;

    let airports = stmt
        .query_map(rusqlite::params![user_id], |row| {
            Ok(AirportVisit {
                airport_code: row.get(0)?,
                departure_count: row.get(1)?,
                arrival_count: row.get(2)?,
                visit_count: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(airports)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InvestigationSummary {
    pub flight_id: String,
    pub flight_number: Option<String>,
    pub route: String,
    pub date: String,
    pub corroboration_score: f64,
    pub passenger_names: String,
    pub created_at: String,
}

#[tauri::command]
pub fn list_all_investigations(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<InvestigationSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT
            i.flight_id,
            f.flight_number,
            f.departure_airport,
            f.arrival_airport,
            f.departure_datetime,
            i.corroboration_score,
            i.passenger_names,
            i.created_at
         FROM flight_investigations i
         JOIN flights f ON i.flight_id = f.id
         WHERE f.user_id = ?1
         ORDER BY i.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let investigations = stmt
        .query_map(rusqlite::params![user_id], |row| {
            let departure: String = row.get(2)?;
            let arrival: String = row.get(3)?;
            let datetime: String = row.get(4)?;

            Ok(InvestigationSummary {
                flight_id: row.get(0)?,
                flight_number: row.get(1)?,
                route: format!("{} → {}", departure, arrival),
                date: datetime.split('T').next().unwrap_or(&datetime).to_string(),
                corroboration_score: row.get(5)?,
                passenger_names: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(investigations)
}
