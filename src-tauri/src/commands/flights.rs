// Flight CRUD commands
use tauri::State;
use super::AppState;
use crate::models::{Flight, FlightInput};

#[tauri::command]
pub fn create_flight(
    user_id: String,
    flight: FlightInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_flight(&user_id, &flight)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_flight(flight_id: String, state: State<'_, AppState>) -> Result<Option<Flight>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_flight(&flight_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_flights(
    user_id: String,
    limit: i32,
    offset: i32,
    state: State<'_, AppState>,
) -> Result<Vec<Flight>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_flights(&user_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_flight(flight_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_flight(&flight_id).map_err(|e| e.to_string())
}