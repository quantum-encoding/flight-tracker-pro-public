// Data Editor commands - deduplication, bulk operations, and data management
use serde::{Deserialize, Serialize};
use tauri::State;

use super::AppState;

// ===== DATA TYPES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub group_id: String,
    pub flight_ids: Vec<String>,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub departure_datetime: String,
    pub similarity_reason: String,
    pub flight_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightEditInput {
    pub flight_number: Option<String>,
    pub departure_airport: Option<String>,
    pub arrival_airport: Option<String>,
    pub departure_datetime: Option<String>,
    pub arrival_datetime: Option<String>,
    pub aircraft_registration: Option<String>,
    pub seat_number: Option<String>,
    pub fare_class: Option<String>,
    pub total_cost: Option<f64>,
    pub currency: Option<String>,
    pub booking_reference: Option<String>,
    pub notes: Option<String>,
    pub distance_km: Option<f64>,
    pub flight_duration: Option<i32>,
    pub carbon_emissions_kg: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkDeleteResult {
    pub deleted_count: usize,
    pub failed_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEditorStats {
    pub total_flights: i64,
    pub total_passengers: i64,
    pub potential_duplicates: i64,
    pub flights_without_notes: i64,
    pub flights_without_distance: i64,
}

// ===== DUPLICATE DETECTION =====

/// Find potential duplicate flights based on same route + similar datetime
#[tauri::command]
pub fn find_duplicate_flights(
    user_id: String,
    time_threshold_minutes: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateGroup>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let threshold = time_threshold_minutes.unwrap_or(60); // Default 60 minutes

    // Find flights with same route within threshold minutes of each other
    let mut stmt = db.conn.prepare(
        "WITH flight_pairs AS (
            SELECT
                f1.id as id1,
                f2.id as id2,
                f1.departure_airport,
                f1.arrival_airport,
                f1.departure_datetime as dt1,
                f2.departure_datetime as dt2,
                ABS(
                    (julianday(f1.departure_datetime) - julianday(f2.departure_datetime)) * 24 * 60
                ) as minute_diff
            FROM flights f1
            JOIN flights f2 ON f1.id < f2.id
                AND f1.user_id = f2.user_id
                AND f1.departure_airport = f2.departure_airport
                AND f1.arrival_airport = f2.arrival_airport
            WHERE f1.user_id = ?1
            HAVING minute_diff <= ?2
        )
        SELECT
            id1, id2, departure_airport, arrival_airport, dt1, minute_diff
        FROM flight_pairs
        ORDER BY departure_airport, arrival_airport, dt1"
    ).map_err(|e| e.to_string())?;

    let pairs: Vec<(String, String, String, String, String, f64)> = stmt
        .query_map(rusqlite::params![user_id, threshold], |row| {
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

    // Group pairs into duplicate groups
    let mut groups: std::collections::HashMap<String, DuplicateGroup> = std::collections::HashMap::new();
    let mut flight_to_group: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    for (id1, id2, dep, arr, dt, diff) in pairs {
        // Check if either flight is already in a group
        let existing_group_id = flight_to_group.get(&id1).cloned()
            .or_else(|| flight_to_group.get(&id2).cloned());

        if let Some(group_id) = existing_group_id {
            // Add to existing group
            let group = groups.get_mut(&group_id).unwrap();
            if !group.flight_ids.contains(&id1) {
                group.flight_ids.push(id1.clone());
                flight_to_group.insert(id1, group_id.clone());
            }
            if !group.flight_ids.contains(&id2) {
                group.flight_ids.push(id2.clone());
                flight_to_group.insert(id2, group_id.clone());
            }
            group.flight_count = group.flight_ids.len() as i32;
        } else {
            // Create new group
            let group_id = uuid::Uuid::new_v4().to_string();
            let reason = format!("Same route, within {} minutes", diff.round() as i32);

            flight_to_group.insert(id1.clone(), group_id.clone());
            flight_to_group.insert(id2.clone(), group_id.clone());

            groups.insert(group_id.clone(), DuplicateGroup {
                group_id,
                flight_ids: vec![id1, id2],
                departure_airport: dep,
                arrival_airport: arr,
                departure_datetime: dt,
                similarity_reason: reason,
                flight_count: 2,
            });
        }
    }

    let mut result: Vec<DuplicateGroup> = groups.into_values().collect();
    result.sort_by(|a, b| b.flight_count.cmp(&a.flight_count));

    Ok(result)
}

/// Merge duplicate flights - keeps the first one, deletes the rest
#[tauri::command]
pub fn merge_duplicate_flights(
    keep_flight_id: String,
    delete_flight_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut deleted = 0;
    for flight_id in &delete_flight_ids {
        if flight_id == &keep_flight_id {
            continue; // Don't delete the one we're keeping
        }

        // Delete related records first
        db.conn.execute(
            "DELETE FROM pilot_logbook WHERE flight_id = ?1",
            rusqlite::params![flight_id],
        ).map_err(|e| e.to_string())?;

        db.conn.execute(
            "DELETE FROM journey_flights WHERE flight_id = ?1",
            rusqlite::params![flight_id],
        ).map_err(|e| e.to_string())?;

        // Delete the flight
        let rows = db.conn.execute(
            "DELETE FROM flights WHERE id = ?1",
            rusqlite::params![flight_id],
        ).map_err(|e| e.to_string())?;

        deleted += rows;
    }

    Ok(deleted)
}

// ===== FLIGHT EDITING =====

/// Update a flight's fields
#[tauri::command]
pub fn update_flight(
    flight_id: String,
    updates: FlightEditInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Build dynamic update query
    let mut set_clauses: Vec<String> = vec![];
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

    if let Some(v) = updates.flight_number {
        set_clauses.push("flight_number = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.departure_airport {
        set_clauses.push("departure_airport = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.arrival_airport {
        set_clauses.push("arrival_airport = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.departure_datetime {
        set_clauses.push("departure_datetime = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.arrival_datetime {
        set_clauses.push("arrival_datetime = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.aircraft_registration {
        set_clauses.push("aircraft_registration = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.seat_number {
        set_clauses.push("seat_number = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.fare_class {
        set_clauses.push("fare_class = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.total_cost {
        set_clauses.push("total_cost = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.currency {
        set_clauses.push("currency = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.booking_reference {
        set_clauses.push("booking_reference = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.notes {
        set_clauses.push("notes = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.distance_km {
        set_clauses.push("distance_km = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.flight_duration {
        set_clauses.push("flight_duration = ?".to_string());
        params.push(Box::new(v));
    }
    if let Some(v) = updates.carbon_emissions_kg {
        set_clauses.push("carbon_emissions_kg = ?".to_string());
        params.push(Box::new(v));
    }

    if set_clauses.is_empty() {
        return Ok(()); // Nothing to update
    }

    set_clauses.push("updated_at = datetime('now')".to_string());

    let query = format!(
        "UPDATE flights SET {} WHERE id = ?",
        set_clauses.join(", ")
    );

    // Add flight_id as last parameter
    params.push(Box::new(flight_id));

    // Convert params to references
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    db.conn.execute(&query, param_refs.as_slice())
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ===== BULK OPERATIONS =====

/// Delete multiple flights at once
#[tauri::command]
pub fn bulk_delete_flights(
    flight_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<BulkDeleteResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut deleted_count = 0;
    let mut failed_ids: Vec<String> = vec![];

    for flight_id in &flight_ids {
        // Delete related records first
        let _ = db.conn.execute(
            "DELETE FROM pilot_logbook WHERE flight_id = ?1",
            rusqlite::params![flight_id],
        );

        let _ = db.conn.execute(
            "DELETE FROM journey_flights WHERE flight_id = ?1",
            rusqlite::params![flight_id],
        );

        // Delete the flight
        match db.conn.execute(
            "DELETE FROM flights WHERE id = ?1",
            rusqlite::params![flight_id],
        ) {
            Ok(rows) if rows > 0 => deleted_count += 1,
            _ => failed_ids.push(flight_id.clone()),
        }
    }

    Ok(BulkDeleteResult {
        deleted_count,
        failed_ids,
    })
}

/// Get all flights for editing (with pagination and optional filters)
#[tauri::command]
pub fn get_flights_for_editor(
    user_id: String,
    limit: i32,
    offset: i32,
    search: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Flight>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let sort_column = match sort_by.as_deref() {
        Some("departure_airport") => "departure_airport",
        Some("arrival_airport") => "arrival_airport",
        Some("flight_number") => "flight_number",
        Some("distance_km") => "distance_km",
        Some("notes") => "notes",
        _ => "departure_datetime",
    };

    let sort_direction = match sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let query = if let Some(ref s) = search {
        format!(
            "SELECT id, user_id, flight_number, departure_airport, arrival_airport,
                    departure_datetime, arrival_datetime, aircraft_type_id, aircraft_registration,
                    total_duration, flight_duration, block_duration, distance_nm, distance_km,
                    booking_reference, ticket_number, seat_number, fare_class, base_fare, taxes,
                    total_cost, currency, carbon_emissions_kg, per_passenger_co2_kg, carbon_offset_purchased,
                    frequent_flyer_program, miles_earned, notes, attachment_path, data_source,
                    verified, created_at, updated_at
             FROM flights
             WHERE user_id = ?1 AND (
                departure_airport LIKE ?4 OR
                arrival_airport LIKE ?4 OR
                flight_number LIKE ?4 OR
                notes LIKE ?4 OR
                booking_reference LIKE ?4
             )
             ORDER BY {} {}
             LIMIT ?2 OFFSET ?3",
            sort_column, sort_direction
        )
    } else {
        format!(
            "SELECT id, user_id, flight_number, departure_airport, arrival_airport,
                    departure_datetime, arrival_datetime, aircraft_type_id, aircraft_registration,
                    total_duration, flight_duration, block_duration, distance_nm, distance_km,
                    booking_reference, ticket_number, seat_number, fare_class, base_fare, taxes,
                    total_cost, currency, carbon_emissions_kg, per_passenger_co2_kg, carbon_offset_purchased,
                    frequent_flyer_program, miles_earned, notes, attachment_path, data_source,
                    verified, created_at, updated_at
             FROM flights
             WHERE user_id = ?1
             ORDER BY {} {}
             LIMIT ?2 OFFSET ?3",
            sort_column, sort_direction
        )
    };

    let mut stmt = db.conn.prepare(&query).map_err(|e| e.to_string())?;

    let flights = if let Some(s) = search {
        let search_pattern = format!("%{}%", s);
        stmt.query_map(
            rusqlite::params![user_id, limit, offset, search_pattern],
            map_flight_row,
        )
    } else {
        stmt.query_map(
            rusqlite::params![user_id, limit, offset],
            map_flight_row,
        )
    }
    .map_err(|e| e.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| e.to_string())?;

    Ok(flights)
}

/// Get total flight count for pagination
#[tauri::command]
pub fn get_flight_count(
    user_id: String,
    search: Option<String>,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: i64 = if let Some(s) = search {
        let search_pattern = format!("%{}%", s);
        db.conn.query_row(
            "SELECT COUNT(*) FROM flights WHERE user_id = ?1 AND (
                departure_airport LIKE ?2 OR
                arrival_airport LIKE ?2 OR
                flight_number LIKE ?2 OR
                notes LIKE ?2 OR
                booking_reference LIKE ?2
            )",
            rusqlite::params![user_id, search_pattern],
            |row| row.get(0),
        )
    } else {
        db.conn.query_row(
            "SELECT COUNT(*) FROM flights WHERE user_id = ?1",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
    }
    .map_err(|e| e.to_string())?;

    Ok(count)
}

// ===== DATA QUALITY STATS =====

/// Get data editor statistics
#[tauri::command]
pub fn get_data_editor_stats(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<DataEditorStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let total_flights: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    // Count unique passengers from notes
    let mut stmt = db.conn.prepare(
        "SELECT notes FROM flights WHERE user_id = ?1 AND notes IS NOT NULL AND notes LIKE 'Passengers:%'"
    ).map_err(|e| e.to_string())?;

    let notes_list: Vec<String> = stmt
        .query_map(rusqlite::params![user_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut unique_passengers: std::collections::HashSet<String> = std::collections::HashSet::new();
    for notes in notes_list {
        if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
            for name in passenger_part.split(',') {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    unique_passengers.insert(trimmed.to_string());
                }
            }
        }
    }

    // Count potential duplicates (same route within 60 min)
    let potential_duplicates: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM (
            SELECT f1.id
            FROM flights f1
            JOIN flights f2 ON f1.id < f2.id
                AND f1.user_id = f2.user_id
                AND f1.departure_airport = f2.departure_airport
                AND f1.arrival_airport = f2.arrival_airport
            WHERE f1.user_id = ?1
            AND ABS((julianday(f1.departure_datetime) - julianday(f2.departure_datetime)) * 24 * 60) <= 60
        )",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).unwrap_or(0);

    let flights_without_notes: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1 AND (notes IS NULL OR notes = '')",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    let flights_without_distance: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM flights WHERE user_id = ?1 AND distance_km IS NULL",
        rusqlite::params![user_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    Ok(DataEditorStats {
        total_flights,
        total_passengers: unique_passengers.len() as i64,
        potential_duplicates,
        flights_without_notes,
        flights_without_distance,
    })
}

// ===== PASSENGER MANAGEMENT =====

/// Remove a passenger from all flights (updates notes field)
#[tauri::command]
pub fn remove_passenger_from_flights(
    user_id: String,
    passenger_name: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all flights with this passenger
    let mut stmt = db.conn.prepare(
        "SELECT id, notes FROM flights WHERE user_id = ?1 AND notes LIKE ?2"
    ).map_err(|e| e.to_string())?;

    let search_pattern = format!("%{}%", passenger_name);
    let flights: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![user_id, search_pattern], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut updated_count = 0;

    for (flight_id, notes) in flights {
        if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
            let passengers: Vec<&str> = passenger_part
                .split(',')
                .map(|s| s.trim())
                .filter(|s| *s != passenger_name)
                .collect();

            let new_notes = if passengers.is_empty() {
                String::new()
            } else {
                format!("Passengers: {}", passengers.join(", "))
            };

            db.conn.execute(
                "UPDATE flights SET notes = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![new_notes, flight_id],
            ).map_err(|e| e.to_string())?;

            updated_count += 1;
        }
    }

    Ok(updated_count)
}

/// Rename a passenger across all flights
#[tauri::command]
pub fn rename_passenger_in_flights(
    user_id: String,
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all flights with this passenger
    let mut stmt = db.conn.prepare(
        "SELECT id, notes FROM flights WHERE user_id = ?1 AND notes LIKE ?2"
    ).map_err(|e| e.to_string())?;

    let search_pattern = format!("%{}%", old_name);
    let flights: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![user_id, search_pattern], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut updated_count = 0;

    for (flight_id, notes) in flights {
        if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
            let passengers: Vec<String> = passenger_part
                .split(',')
                .map(|s| {
                    let trimmed = s.trim();
                    if trimmed == old_name {
                        new_name.clone()
                    } else {
                        trimmed.to_string()
                    }
                })
                .collect();

            let new_notes = format!("Passengers: {}", passengers.join(", "));

            db.conn.execute(
                "UPDATE flights SET notes = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![new_notes, flight_id],
            ).map_err(|e| e.to_string())?;

            updated_count += 1;
        }
    }

    Ok(updated_count)
}

// Helper function to map a row to a Flight
fn map_flight_row(row: &rusqlite::Row) -> Result<crate::models::Flight, rusqlite::Error> {
    Ok(crate::models::Flight {
        id: row.get(0)?,
        user_id: row.get(1)?,
        flight_number: row.get(2)?,
        departure_airport: row.get(3)?,
        arrival_airport: row.get(4)?,
        departure_datetime: row.get(5)?,
        arrival_datetime: row.get(6)?,
        aircraft_type_id: row.get(7)?,
        aircraft_registration: row.get(8)?,
        total_duration: row.get(9)?,
        flight_duration: row.get(10)?,
        block_duration: row.get(11)?,
        distance_nm: row.get(12)?,
        distance_km: row.get(13)?,
        booking_reference: row.get(14)?,
        ticket_number: row.get(15)?,
        seat_number: row.get(16)?,
        fare_class: row.get(17)?,
        base_fare: row.get(18)?,
        taxes: row.get(19)?,
        total_cost: row.get(20)?,
        currency: row.get(21)?,
        carbon_emissions_kg: row.get(22)?,
        per_passenger_co2_kg: row.get(23)?,
        carbon_offset_purchased: row.get(24)?,
        frequent_flyer_program: row.get(25)?,
        miles_earned: row.get(26)?,
        notes: row.get(27)?,
        attachment_path: row.get(28)?,
        data_source: row.get(29)?,
        verified: row.get(30)?,
        created_at: row.get(31)?,
        updated_at: row.get(32)?,
    })
}
