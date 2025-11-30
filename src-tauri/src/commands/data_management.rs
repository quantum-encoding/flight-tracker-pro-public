// Data export and database management commands
use tauri::State;
use super::AppState;
use std::fs::File;

#[tauri::command]
pub fn export_data_to_csv(
    user_id: String,
    export_path: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all flights for the user
    let flights = db
        .list_flights(&user_id, i32::MAX, 0)
        .map_err(|e| e.to_string())?;

    let file =
        File::create(&export_path).map_err(|e| format!("Failed to create export file: {}", e))?;

    let mut writer = ::csv::Writer::from_writer(file);

    // Write header
    writer
        .write_record([
            "Date",
            "Flight Number",
            "Departure Airport",
            "Arrival Airport",
            "Departure Time",
            "Arrival Time",
            "Distance (NM)",
            "Distance (KM)",
            "Duration (minutes)",
            "Aircraft Registration",
            "Seat Number",
            "Fare Class",
            "Total Cost",
            "Currency",
            "Booking Reference",
            "Notes",
        ])
        .map_err(|e| e.to_string())?;

    // Write flight records
    for flight in &flights {
        writer
            .write_record([
                flight.departure_datetime.split('T').next().unwrap_or(""),
                flight.flight_number.as_deref().unwrap_or(""),
                &flight.departure_airport,
                &flight.arrival_airport,
                &flight.departure_datetime,
                flight.arrival_datetime.as_deref().unwrap_or(""),
                &flight
                    .distance_nm
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                &flight
                    .distance_km
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                &flight
                    .flight_duration
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                flight.aircraft_registration.as_deref().unwrap_or(""),
                flight.seat_number.as_deref().unwrap_or(""),
                flight.fare_class.as_deref().unwrap_or(""),
                &flight.total_cost.map(|c| c.to_string()).unwrap_or_default(),
                flight.currency.as_deref().unwrap_or(""),
                flight.booking_reference.as_deref().unwrap_or(""),
                flight.notes.as_deref().unwrap_or(""),
            ])
            .map_err(|e| e.to_string())?;
    }

    writer.flush().map_err(|e| e.to_string())?;

    Ok(flights.len())
}

#[tauri::command]
pub fn reset_database(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Disable foreign key checks to allow deletion in any order
    db.conn
        .execute("PRAGMA foreign_keys = OFF", [])
        .map_err(|e| e.to_string())?;

    // Get all table names from the database
    let mut stmt = db.conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|e| e.to_string())?;

    let table_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    drop(stmt);

    // Delete all data from all tables
    for table in &table_names {
        db.conn
            .execute(&format!("DELETE FROM \"{}\"", table), [])
            .map_err(|e| format!("Failed to clear table {}: {}", table, e))?;
    }

    // Re-enable foreign key checks
    db.conn
        .execute("PRAGMA foreign_keys = ON", [])
        .map_err(|e| e.to_string())?;

    Ok(())
}