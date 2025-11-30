// Journey/trip management commands
use tauri::State;

use super::AppState;

// ===== JOURNEY COMMANDS =====

#[tauri::command]
pub fn create_journey(
    user_id: String,
    name: String,
    description: Option<String>,
    start_date: String,
    end_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_journey(&user_id, &name, description.as_deref(), &start_date, end_date.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_journey(
    journey_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::Journey>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_journey(&journey_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_user_journeys(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Journey>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_user_journeys(&user_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_journey(
    journey_id: String,
    name: String,
    description: Option<String>,
    start_date: String,
    end_date: Option<String>,
    is_favorite: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_journey(
        &journey_id,
        &name,
        description.as_deref(),
        &start_date,
        end_date.as_deref(),
        is_favorite,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_journey(journey_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_journey(&journey_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_flight_to_journey(
    journey_id: String,
    flight_id: String,
    sequence_order: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.add_flight_to_journey(&journey_id, &flight_id, sequence_order)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_flight_from_journey(
    journey_id: String,
    flight_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.remove_flight_from_journey(&journey_id, &flight_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_journey_flights(
    journey_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Flight>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_journey_flights(&journey_id)
        .map_err(|e| e.to_string())
}
