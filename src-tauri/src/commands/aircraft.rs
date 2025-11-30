// Aircraft type management commands
use tauri::State;

use super::AppState;

// ===== AIRCRAFT TYPE COMMANDS =====

#[tauri::command]
pub fn create_aircraft_type(
    aircraft: crate::models::AircraftTypeInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_aircraft_type(
        &aircraft.manufacturer,
        &aircraft.model,
        aircraft.type_designator.as_deref(),
        aircraft.category.as_deref(),
        aircraft.class.as_deref(),
        aircraft.notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_aircraft_type(
    aircraft_type_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::AircraftType>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_aircraft_type(&aircraft_type_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_all_aircraft_types(
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::AircraftType>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_all_aircraft_types().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_aircraft_type(
    aircraft_type_id: String,
    aircraft: crate::models::AircraftTypeInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_aircraft_type(
        &aircraft_type_id,
        &aircraft.manufacturer,
        &aircraft.model,
        aircraft.type_designator.as_deref(),
        aircraft.category.as_deref(),
        aircraft.class.as_deref(),
        aircraft.notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_aircraft_type(
    aircraft_type_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_aircraft_type(&aircraft_type_id)
        .map_err(|e| e.to_string())
}
