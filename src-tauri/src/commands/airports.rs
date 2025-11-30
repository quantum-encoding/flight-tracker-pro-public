// Airport management commands
use tauri::State;

use super::AppState;

// ===== AIRPORT COMMANDS =====

#[tauri::command]
pub fn create_airport(
    airport: crate::models::AirportInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_airport(
        airport.icao_code.as_deref(),
        airport.iata_code.as_deref(),
        &airport.name,
        airport.city.as_deref(),
        airport.country.as_deref(),
        airport.latitude,
        airport.longitude,
        airport.timezone.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_airport(
    airport_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::Airport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_airport(&airport_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_all_airports(
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Airport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_all_airports().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_airport(
    airport_id: String,
    airport: crate::models::AirportInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_airport(
        &airport_id,
        airport.icao_code.as_deref(),
        airport.iata_code.as_deref(),
        &airport.name,
        airport.city.as_deref(),
        airport.country.as_deref(),
        airport.latitude,
        airport.longitude,
        airport.timezone.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_airport(airport_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_airport(&airport_id).map_err(|e| e.to_string())
}
