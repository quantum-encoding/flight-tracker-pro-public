// Batch calculation commands for distance and CO2 emissions
// Calculates missing distances using airport coordinates and recalculates per-passenger CO2

use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;

// ===== RESULT TYPES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistanceCalculationResult {
    pub flight_id: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub distance_nm: f64,
    pub distance_km: f64,
    pub source: String, // "database" or "hardcoded"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDistanceResult {
    pub updated_count: usize,
    pub skipped_count: usize,
    pub failed_count: usize,
    pub calculations: Vec<DistanceCalculationResult>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CO2CalculationResult {
    pub flight_id: String,
    pub total_co2_kg: f64,
    pub passenger_count: u32,
    pub per_passenger_co2_kg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCO2Result {
    pub updated_count: usize,
    pub skipped_count: usize,
    pub calculations: Vec<CO2CalculationResult>,
}

// ===== BATCH DISTANCE CALCULATION =====

/// Calculate and update distances for all flights missing distance_km
/// Uses airport coordinates from database first, falls back to hardcoded geo.rs coordinates
#[tauri::command]
pub fn batch_calculate_missing_distances(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<BatchDistanceResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all flights missing distance_km
    let mut stmt = db.conn.prepare(
        "SELECT id, departure_airport, arrival_airport
         FROM flights
         WHERE user_id = ?1 AND (distance_km IS NULL OR distance_km = 0)"
    ).map_err(|e| e.to_string())?;

    let flights_missing_distance: Vec<(String, String, String)> = stmt
        .query_map(rusqlite::params![user_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut failed_count = 0;
    let mut calculations: Vec<DistanceCalculationResult> = vec![];
    let mut errors: Vec<String> = vec![];

    for (flight_id, departure, arrival) in flights_missing_distance {
        // Try to get coordinates from database airports table first
        let dep_coords = get_airport_coords_from_db(&db, &departure);
        let arr_coords = get_airport_coords_from_db(&db, &arrival);

        let (distance_nm, distance_km, source) = match (dep_coords, arr_coords) {
            (Some((dep_lat, dep_lon)), Some((arr_lat, arr_lon))) => {
                let (nm, km) = crate::geo::calculate_distance(dep_lat, dep_lon, arr_lat, arr_lon);
                (nm, km, "database".to_string())
            }
            _ => {
                // Fall back to hardcoded coordinates in geo.rs
                match crate::geo::calculate_airport_distance(&departure, &arrival) {
                    Some((nm, km)) => (nm, km, "hardcoded".to_string()),
                    None => {
                        // Cannot calculate distance - missing coordinates for one or both airports
                        skipped_count += 1;
                        errors.push(format!(
                            "Missing coordinates for {} -> {} (flight {})",
                            departure, arrival, flight_id
                        ));
                        continue;
                    }
                }
            }
        };

        // Update the flight with calculated distances
        match db.conn.execute(
            "UPDATE flights SET distance_nm = ?1, distance_km = ?2, updated_at = datetime('now') WHERE id = ?3",
            rusqlite::params![distance_nm, distance_km, flight_id],
        ) {
            Ok(_) => {
                updated_count += 1;
                calculations.push(DistanceCalculationResult {
                    flight_id,
                    departure_airport: departure,
                    arrival_airport: arrival,
                    distance_nm,
                    distance_km,
                    source,
                });
            }
            Err(e) => {
                failed_count += 1;
                errors.push(format!("Failed to update flight {}: {}", flight_id, e));
            }
        }
    }

    Ok(BatchDistanceResult {
        updated_count,
        skipped_count,
        failed_count,
        calculations,
        errors,
    })
}

/// Get airport coordinates from the database airports table
fn get_airport_coords_from_db(
    db: &crate::database::Database,
    airport_code: &str,
) -> Option<(f64, f64)> {
    // Try to find by IATA code first (most common in flight logs like "PBI", "TEB")
    let result: Result<(f64, f64), _> = db.conn.query_row(
        "SELECT latitude, longitude FROM airports WHERE iata_code = ?1 AND latitude IS NOT NULL AND longitude IS NOT NULL",
        rusqlite::params![airport_code],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    if let Ok(coords) = result {
        return Some(coords);
    }

    // Fall back to ICAO code
    let result: Result<(f64, f64), _> = db.conn.query_row(
        "SELECT latitude, longitude FROM airports WHERE icao_code = ?1 AND latitude IS NOT NULL AND longitude IS NOT NULL",
        rusqlite::params![airport_code],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    result.ok()
}

// ===== BATCH CO2 RECALCULATION =====

/// Recalculate CO2 emissions and per-passenger CO2 for all flights
/// Uses the "Shared Responsibility" model: crew excluded from passenger count
/// Formula: per_passenger_co2 = total_co2 / (passenger_count - crew_count)
#[tauri::command]
pub fn batch_recalculate_co2(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<BatchCO2Result, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all flights with distance_km (required for CO2 calculation)
    let mut stmt = db.conn.prepare(
        "SELECT id, departure_airport, arrival_airport, distance_km, notes, aircraft_registration
         FROM flights
         WHERE user_id = ?1 AND distance_km IS NOT NULL AND distance_km > 0"
    ).map_err(|e| e.to_string())?;

    let flights: Vec<(String, String, String, f64, Option<String>, Option<String>)> = stmt
        .query_map(rusqlite::params![user_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut updated_count = 0;
    let mut skipped_count = 0;
    let mut calculations: Vec<CO2CalculationResult> = vec![];

    for (flight_id, _departure, _arrival, distance_km, notes, aircraft_reg) in flights {
        // Count passengers from notes field (format: "Passengers: Name1, Name2, Name3")
        let passenger_count = count_passengers_from_notes(notes.as_deref());

        if passenger_count == 0 {
            // No passengers recorded - skip per-passenger calculation but still calculate total CO2
            skipped_count += 1;
            continue;
        }

        // Get aircraft type for more accurate fuel burn calculation
        let aircraft_type = aircraft_reg.as_deref();

        // Calculate total CO2 using the standard formula:
        // Distance × Fuel Burn Rate × CO2 Factor (3.16 for Jet A1)
        let total_co2_kg = crate::calculations::calculate_co2_emissions(distance_km, aircraft_type);

        // Calculate per-passenger CO2 (excluding crew)
        // Crew (pilot, copilot) are considered part of the aircraft, not passengers
        // They fly regardless of passenger count, so CO2 responsibility is on passengers only
        let per_passenger_co2_kg = total_co2_kg / passenger_count as f64;

        // Update the flight with calculated CO2 values
        match db.conn.execute(
            "UPDATE flights SET carbon_emissions_kg = ?1, per_passenger_co2_kg = ?2, updated_at = datetime('now') WHERE id = ?3",
            rusqlite::params![total_co2_kg, per_passenger_co2_kg, flight_id],
        ) {
            Ok(_) => {
                updated_count += 1;
                calculations.push(CO2CalculationResult {
                    flight_id,
                    total_co2_kg,
                    passenger_count,
                    per_passenger_co2_kg,
                });
            }
            Err(_) => {
                skipped_count += 1;
            }
        }
    }

    Ok(BatchCO2Result {
        updated_count,
        skipped_count,
        calculations,
    })
}

/// Count passengers from the notes field
/// Format: "Passengers: Name1, Name2, Name3"
/// Returns 0 if no passengers found
fn count_passengers_from_notes(notes: Option<&str>) -> u32 {
    match notes {
        Some(n) if n.starts_with("Passengers: ") => {
            let passenger_part = n.strip_prefix("Passengers: ").unwrap_or("");
            passenger_part
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .count() as u32
        }
        _ => 0,
    }
}

// ===== COMBINED BATCH OPERATION =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCalculationSummary {
    pub distance_result: BatchDistanceResult,
    pub co2_result: BatchCO2Result,
}

/// Run both distance and CO2 calculations in sequence
/// First calculates missing distances, then recalculates CO2 for all flights
#[tauri::command]
pub fn batch_calculate_all(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<BatchCalculationSummary, String> {
    // First, calculate missing distances
    let distance_result = batch_calculate_missing_distances(user_id.clone(), state.clone())?;

    // Then, recalculate CO2 for all flights (now including newly calculated distances)
    let co2_result = batch_recalculate_co2(user_id, state)?;

    Ok(BatchCalculationSummary {
        distance_result,
        co2_result,
    })
}

// ===== STREAMING BATCH CALCULATION =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgressUpdate {
    pub phase: String,           // "distance" or "co2"
    pub current: usize,
    pub total: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingBatchResult {
    pub distance_result: BatchDistanceResult,
    pub co2_result: BatchCO2Result,
}

/// Run batch calculations with progress events emitted to frontend
/// This allows the UI to show a progress bar and remain responsive
#[tauri::command]
pub async fn batch_calculate_streaming(
    user_id: String,
    batch_size: usize,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<StreamingBatchResult, String> {
    use tauri::Emitter;

    let batch_size = if batch_size == 0 { 50 } else { batch_size };

    // ===== PHASE 1: DISTANCE CALCULATION =====
    let distance_result = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        // Get all flights missing distance_km
        let mut stmt = db.conn.prepare(
            "SELECT id, departure_airport, arrival_airport
             FROM flights
             WHERE user_id = ?1 AND (distance_km IS NULL OR distance_km = 0)"
        ).map_err(|e| e.to_string())?;

        let flights_missing_distance: Vec<(String, String, String)> = stmt
            .query_map(rusqlite::params![user_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let total = flights_missing_distance.len();
        let mut updated_count = 0;
        let mut skipped_count = 0;
        let mut failed_count = 0;
        let mut calculations: Vec<DistanceCalculationResult> = vec![];
        let mut errors: Vec<String> = vec![];

        for (i, (flight_id, departure, arrival)) in flights_missing_distance.into_iter().enumerate() {
            // Emit progress every batch_size items or on last item
            if i % batch_size == 0 || i == total - 1 {
                let _ = app_handle.emit("batch-progress", BatchProgressUpdate {
                    phase: "distance".to_string(),
                    current: i + 1,
                    total,
                    updated: updated_count,
                    skipped: skipped_count,
                    failed: failed_count,
                    complete: false,
                });
            }

            // Try to get coordinates from database airports table first
            let dep_coords = get_airport_coords_from_db(&db, &departure);
            let arr_coords = get_airport_coords_from_db(&db, &arrival);

            let (distance_nm, distance_km, source) = match (dep_coords, arr_coords) {
                (Some((dep_lat, dep_lon)), Some((arr_lat, arr_lon))) => {
                    let (nm, km) = crate::geo::calculate_distance(dep_lat, dep_lon, arr_lat, arr_lon);
                    (nm, km, "database".to_string())
                }
                _ => {
                    // Fall back to hardcoded coordinates in geo.rs
                    match crate::geo::calculate_airport_distance(&departure, &arrival) {
                        Some((nm, km)) => (nm, km, "hardcoded".to_string()),
                        None => {
                            skipped_count += 1;
                            errors.push(format!(
                                "Missing coordinates for {} -> {} (flight {})",
                                departure, arrival, flight_id
                            ));
                            continue;
                        }
                    }
                }
            };

            // Update the flight with calculated distances
            match db.conn.execute(
                "UPDATE flights SET distance_nm = ?1, distance_km = ?2, updated_at = datetime('now') WHERE id = ?3",
                rusqlite::params![distance_nm, distance_km, flight_id],
            ) {
                Ok(_) => {
                    updated_count += 1;
                    calculations.push(DistanceCalculationResult {
                        flight_id,
                        departure_airport: departure,
                        arrival_airport: arrival,
                        distance_nm,
                        distance_km,
                        source,
                    });
                }
                Err(e) => {
                    failed_count += 1;
                    errors.push(format!("Failed to update flight {}: {}", flight_id, e));
                }
            }
        }

        // Emit distance phase complete
        let _ = app_handle.emit("batch-progress", BatchProgressUpdate {
            phase: "distance".to_string(),
            current: total,
            total,
            updated: updated_count,
            skipped: skipped_count,
            failed: failed_count,
            complete: true,
        });

        BatchDistanceResult {
            updated_count,
            skipped_count,
            failed_count,
            calculations,
            errors,
        }
    };

    // ===== PHASE 2: CO2 CALCULATION =====
    let co2_result = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        // Get all flights with distance_km
        let mut stmt = db.conn.prepare(
            "SELECT id, departure_airport, arrival_airport, distance_km, notes, aircraft_registration
             FROM flights
             WHERE user_id = ?1 AND distance_km IS NOT NULL AND distance_km > 0"
        ).map_err(|e| e.to_string())?;

        let flights: Vec<(String, String, String, f64, Option<String>, Option<String>)> = stmt
            .query_map(rusqlite::params![user_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let total = flights.len();
        let mut updated_count = 0;
        let mut skipped_count = 0;
        let mut calculations: Vec<CO2CalculationResult> = vec![];

        for (i, (flight_id, _departure, _arrival, distance_km, notes, aircraft_reg)) in flights.into_iter().enumerate() {
            // Emit progress every batch_size items or on last item
            if i % batch_size == 0 || i == total - 1 {
                let _ = app_handle.emit("batch-progress", BatchProgressUpdate {
                    phase: "co2".to_string(),
                    current: i + 1,
                    total,
                    updated: updated_count,
                    skipped: skipped_count,
                    failed: 0,
                    complete: false,
                });
            }

            // Count passengers from notes field
            let passenger_count = count_passengers_from_notes(notes.as_deref());

            if passenger_count == 0 {
                skipped_count += 1;
                continue;
            }

            let aircraft_type = aircraft_reg.as_deref();
            let total_co2_kg = crate::calculations::calculate_co2_emissions(distance_km, aircraft_type);
            let per_passenger_co2_kg = total_co2_kg / passenger_count as f64;

            match db.conn.execute(
                "UPDATE flights SET carbon_emissions_kg = ?1, per_passenger_co2_kg = ?2, updated_at = datetime('now') WHERE id = ?3",
                rusqlite::params![total_co2_kg, per_passenger_co2_kg, flight_id],
            ) {
                Ok(_) => {
                    updated_count += 1;
                    calculations.push(CO2CalculationResult {
                        flight_id,
                        total_co2_kg,
                        passenger_count,
                        per_passenger_co2_kg,
                    });
                }
                Err(_) => {
                    skipped_count += 1;
                }
            }
        }

        // Emit CO2 phase complete
        let _ = app_handle.emit("batch-progress", BatchProgressUpdate {
            phase: "co2".to_string(),
            current: total,
            total,
            updated: updated_count,
            skipped: skipped_count,
            failed: 0,
            complete: true,
        });

        BatchCO2Result {
            updated_count,
            skipped_count,
            calculations,
        }
    };

    Ok(StreamingBatchResult {
        distance_result,
        co2_result,
    })
}

// ===== GET CALCULATION STATS =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationStats {
    pub total_flights: i64,
    pub flights_with_distance: i64,
    pub flights_missing_distance: i64,
    pub flights_with_co2: i64,
    pub flights_with_per_passenger_co2: i64,
    pub flights_with_passengers: i64,
}

/// Get statistics about which flights have/need calculations
#[tauri::command]
pub fn get_calculation_stats(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<CalculationStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let total_flights: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let flights_with_distance: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1 AND distance_km IS NOT NULL AND distance_km > 0",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let flights_missing_distance: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1 AND (distance_km IS NULL OR distance_km = 0)",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let flights_with_co2: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1 AND carbon_emissions_kg IS NOT NULL AND carbon_emissions_kg > 0",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let flights_with_per_passenger_co2: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1 AND per_passenger_co2_kg IS NOT NULL AND per_passenger_co2_kg > 0",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let flights_with_passengers: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1 AND notes IS NOT NULL AND notes LIKE 'Passengers:%'",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    Ok(CalculationStats {
        total_flights,
        flights_with_distance,
        flights_missing_distance,
        flights_with_co2,
        flights_with_per_passenger_co2,
        flights_with_passengers,
    })
}
