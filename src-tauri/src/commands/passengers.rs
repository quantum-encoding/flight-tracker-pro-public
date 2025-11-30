// Passenger management commands
use crate::models::User;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;

// ===== PASSENGER TYPES =====

/// Type alias for flight query results (id, notes, dep_airport, arr_airport, datetime, distance, emissions, offset)
type FlightQueryRow = (String, String, String, String, String, Option<f64>, Option<f64>, Option<i32>);

#[derive(Debug, Serialize, Deserialize)]
pub struct RouteStatistic {
    pub route: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub flight_count: i32,
    pub total_distance_km: f64,
    pub avg_duration_minutes: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AirportStatistic {
    pub airport_code: String,
    pub visit_count: i32,
    pub departure_count: i32,
    pub arrival_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Analytics {
    pub top_routes: Vec<RouteStatistic>,
    pub most_visited_airports: Vec<AirportStatistic>,
    pub total_unique_routes: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerName {
    pub abbreviation: String,
    pub full_name: Option<String>,
    pub usage_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerMapping {
    pub abbreviation: String,
    pub full_name: String,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerRoute {
    pub route: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub flight_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerCompanion {
    pub name: String,
    pub flight_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerDetails {
    pub abbreviation: String,
    pub full_name: Option<String>,
    pub total_flights: i32,
    pub total_distance_km: f64,
    pub top_routes: Vec<PassengerRoute>,
    pub travel_companions: Vec<PassengerCompanion>,
    pub first_flight_date: Option<String>,
    pub last_flight_date: Option<String>,
    pub total_co2_kg: f64,
    pub avg_co2_per_flight_kg: f64,
    pub carbon_offset_purchased: bool,
}

// ===== ANALYTICS COMMANDS =====

#[tauri::command]
pub fn get_analytics(
    user_id: String,
    limit: i32,
    state: State<'_, AppState>,
) -> Result<Analytics, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get top routes
    let mut route_stmt = db
        .conn
        .prepare(
            "SELECT
            departure_airport,
            arrival_airport,
            COUNT(*) as flight_count,
            COALESCE(SUM(distance_km), 0) as total_distance,
            COALESCE(AVG(flight_duration), 0) as avg_duration
         FROM flights
         WHERE user_id = ?1
         GROUP BY departure_airport, arrival_airport
         ORDER BY flight_count DESC
         LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;

    let top_routes = route_stmt
        .query_map(rusqlite::params![user_id, limit], |row| {
            let departure: String = row.get(0)?;
            let arrival: String = row.get(1)?;

            Ok(RouteStatistic {
                departure_airport: departure.clone(),
                arrival_airport: arrival.clone(),
                route: format!("{} → {}", departure, arrival),
                flight_count: row.get(2)?,
                total_distance_km: row.get(3)?,
                avg_duration_minutes: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Get most visited airports
    let mut airport_stmt = db.conn.prepare(
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
         ORDER BY total_visits DESC
         LIMIT ?2"
    ).map_err(|e| e.to_string())?;

    let most_visited_airports = airport_stmt
        .query_map(rusqlite::params![user_id, limit], |row| {
            Ok(AirportStatistic {
                airport_code: row.get(0)?,
                departure_count: row.get(1)?,
                arrival_count: row.get(2)?,
                visit_count: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Get total unique routes
    let total_unique_routes: i32 = db
        .conn
        .query_row(
            "SELECT COUNT(DISTINCT departure_airport || '-' || arrival_airport)
         FROM flights WHERE user_id = ?1",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(Analytics {
        top_routes,
        most_visited_airports,
        total_unique_routes,
    })
}

// ===== DATASET MANAGEMENT (Multiple Users) =====

#[tauri::command]
pub fn list_all_users(state: State<'_, AppState>) -> Result<Vec<User>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT id, name, email, pilot_license_number, license_type, license_country, created_at, updated_at
         FROM users
         ORDER BY created_at ASC"
    ).map_err(|e| e.to_string())?;

    let users = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                pilot_license_number: row.get(3)?,
                license_type: row.get(4)?,
                license_country: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(users)
}

#[tauri::command]
pub fn update_user_name(
    user_id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute(
            "UPDATE users SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![new_name, user_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ===== PASSENGER NAME MANAGEMENT =====

#[tauri::command]
pub fn get_all_passenger_names(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PassengerName>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Extract all passenger names from notes field
    let mut stmt = db
        .conn
        .prepare("SELECT notes FROM flights WHERE user_id = ?1 AND notes IS NOT NULL")
        .map_err(|e| e.to_string())?;

    let notes_list: Vec<String> = stmt
        .query_map(rusqlite::params![user_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Parse passenger names from notes
    let mut name_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

    for notes in notes_list {
        if notes.starts_with("Passengers: ") {
            let passenger_part = notes.trim_start_matches("Passengers: ");
            // Split by comma and trim
            for name in passenger_part.split(',') {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    *name_counts.entry(trimmed.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    // Get existing mappings
    let mut mapping_stmt = db
        .conn
        .prepare("SELECT abbreviation, full_name FROM passenger_mappings")
        .map_err(|e| e.to_string())?;

    let mappings: std::collections::HashMap<String, String> = mapping_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    // Combine into result
    let mut result: Vec<PassengerName> = name_counts
        .into_iter()
        .map(|(abbr, count)| PassengerName {
            abbreviation: abbr.clone(),
            full_name: mappings.get(&abbr).cloned(),
            usage_count: count,
        })
        .collect();

    // Sort by usage count descending
    result.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

    Ok(result)
}

#[tauri::command]
pub fn save_passenger_mapping(
    abbreviation: String,
    full_name: String,
    notes: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute(
            "INSERT INTO passenger_mappings (abbreviation, full_name, notes, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(abbreviation) DO UPDATE SET
            full_name = ?2,
            notes = ?3,
            updated_at = datetime('now')",
            rusqlite::params![abbreviation, full_name, notes],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_passenger_mapping(
    abbreviation: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute(
            "DELETE FROM passenger_mappings WHERE abbreviation = ?1",
            rusqlite::params![abbreviation],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_all_passenger_mappings(
    state: State<'_, AppState>,
) -> Result<Vec<PassengerMapping>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT abbreviation, full_name, notes FROM passenger_mappings ORDER BY abbreviation ASC"
    ).map_err(|e| e.to_string())?;

    let mappings = stmt
        .query_map([], |row| {
            Ok(PassengerMapping {
                abbreviation: row.get(0)?,
                full_name: row.get(1)?,
                notes: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(mappings)
}

// ===== PASSENGER DETAILS =====

#[tauri::command]
pub fn get_passenger_details(
    user_id: String,
    abbreviation: String,
    state: State<'_, AppState>,
) -> Result<PassengerDetails, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get full name mapping if it exists
    let full_name: Option<String> = db
        .conn
        .query_row(
            "SELECT full_name FROM passenger_mappings WHERE abbreviation = ?1",
            rusqlite::params![abbreviation],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    // Get flights containing this passenger (including CO2 data)
    let mut flight_stmt = db
        .conn
        .prepare(
            "SELECT id, notes, departure_airport, arrival_airport, departure_datetime, distance_km, carbon_emissions_kg, carbon_offset_purchased
         FROM flights
         WHERE user_id = ?1 AND notes LIKE ?2
         ORDER BY departure_datetime DESC",
        )
        .map_err(|e| e.to_string())?;

    let search_pattern = format!("%{}%", abbreviation);
    let flights: Vec<FlightQueryRow> = flight_stmt
        .query_map(rusqlite::params![user_id, search_pattern], |row| {
            Ok((
                row.get(0)?,  // id
                row.get(1)?,  // notes
                row.get(2)?,  // departure_airport
                row.get(3)?,  // arrival_airport
                row.get(4)?,  // departure_datetime
                row.get(5)?,  // distance_km
                row.get(6)?,  // carbon_emissions_kg
                row.get(7)?,  // carbon_offset_purchased
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Filter to only flights where this passenger is actually listed
    let relevant_flights: Vec<_> = flights
        .into_iter()
        .filter(|(_, notes, _, _, _, _, _, _)| {
            if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
                passenger_part
                    .split(',')
                    .any(|name| name.trim() == abbreviation)
            } else {
                false
            }
        })
        .collect();

    let total_flights = relevant_flights.len() as i32;
    let total_distance_km: f64 = relevant_flights
        .iter()
        .filter_map(|(_, _, _, _, _, distance, _, _)| *distance)
        .sum();

    // Calculate CO2 emissions
    let total_co2_kg: f64 = relevant_flights
        .iter()
        .filter_map(|(_, _, _, _, _, _, co2, _)| *co2)
        .sum();

    let avg_co2_per_flight_kg = if total_flights > 0 {
        total_co2_kg / total_flights as f64
    } else {
        0.0
    };

    let carbon_offset_purchased = relevant_flights
        .iter()
        .any(|(_, _, _, _, _, _, _, offset)| offset.unwrap_or(0) > 0);

    // Get first and last flight dates
    let first_flight_date = relevant_flights
        .last()
        .map(|(_, _, _, _, date, _, _, _)| date.clone());
    let last_flight_date = relevant_flights
        .first()
        .map(|(_, _, _, _, date, _, _, _)| date.clone());

    // Calculate top routes
    let mut route_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for (_, _, dep, arr, _, _, _, _) in &relevant_flights {
        let route = format!("{} → {}", dep, arr);
        *route_counts.entry(route).or_insert(0) += 1;
    }

    let mut top_routes: Vec<PassengerRoute> = route_counts
        .into_iter()
        .map(|(route, count)| {
            let parts: Vec<&str> = route.split(" → ").collect();
            PassengerRoute {
                route: route.clone(),
                departure_airport: parts.first().unwrap_or(&"").to_string(),
                arrival_airport: parts.get(1).unwrap_or(&"").to_string(),
                flight_count: count,
            }
        })
        .collect();
    top_routes.sort_by(|a, b| b.flight_count.cmp(&a.flight_count));
    top_routes.truncate(5); // Top 5 routes

    // Calculate travel companions
    let mut companion_counts: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();
    for (_, notes, _, _, _, _, _, _) in &relevant_flights {
        if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
            let passengers: Vec<&str> = passenger_part.split(',').map(|s| s.trim()).collect();

            // Count other passengers on same flights
            for passenger in passengers {
                if passenger != abbreviation && !passenger.is_empty() {
                    *companion_counts.entry(passenger.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    let mut travel_companions: Vec<PassengerCompanion> = companion_counts
        .into_iter()
        .map(|(name, count)| PassengerCompanion {
            name,
            flight_count: count,
        })
        .collect();
    travel_companions.sort_by(|a, b| b.flight_count.cmp(&a.flight_count));
    travel_companions.truncate(10); // Top 10 companions

    Ok(PassengerDetails {
        abbreviation,
        full_name,
        total_flights,
        total_distance_km,
        top_routes,
        travel_companions,
        first_flight_date,
        last_flight_date,
        total_co2_kg,
        avg_co2_per_flight_kg,
        carbon_offset_purchased,
    })
}

// ===== CANONICAL NAMES FOR DROPDOWN =====

#[tauri::command]
pub fn get_canonical_names(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get distinct full_name values from passenger_mappings
    let mut stmt = db.conn.prepare(
        "SELECT DISTINCT full_name FROM passenger_mappings ORDER BY full_name ASC"
    ).map_err(|e| e.to_string())?;

    let names = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(names)
}

/// Get all unmapped passenger names (for alias assignment UI)
#[tauri::command]
pub fn get_unmapped_passengers(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PassengerName>, String> {
    let all_passengers = get_all_passenger_names(user_id, state)?;
    Ok(all_passengers.into_iter().filter(|p| p.full_name.is_none()).collect())
}

/// Get all aliases that map to a specific canonical name
#[tauri::command]
pub fn get_aliases_for_canonical(
    canonical_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<PassengerMapping>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT abbreviation, full_name, notes FROM passenger_mappings WHERE full_name = ?1 ORDER BY abbreviation ASC"
    ).map_err(|e| e.to_string())?;

    let mappings = stmt
        .query_map(rusqlite::params![canonical_name], |row| {
            Ok(PassengerMapping {
                abbreviation: row.get(0)?,
                full_name: row.get(1)?,
                notes: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(mappings)
}

// ===== PDF DOSSIER EXPORT =====

#[tauri::command]
pub fn export_passenger_dossier(
    user_id: String,
    passenger_name: String,
    output_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use crate::pdf_dossier::PassengerDossier;
    use std::path::PathBuf;

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Generate dossier data
    let dossier = PassengerDossier::from_passenger(&db, &passenger_name, &user_id)
        .map_err(|e| format!("Failed to generate dossier data: {}", e))?;

    drop(db); // Release the lock before PDF generation

    // Generate PDF
    let path = PathBuf::from(&output_path);
    dossier
        .generate_pdf(&path)
        .map_err(|e| format!("Failed to generate PDF: {}", e))?;

    Ok(format!("Dossier exported successfully to {}", output_path))
}
