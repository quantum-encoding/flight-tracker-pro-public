// Pilot logbook commands
use tauri::State;

use super::AppState;

// ===== PILOT LOGBOOK COMMANDS =====

#[tauri::command]
pub fn create_pilot_logbook_entry(
    entry: crate::models::PilotLogbookInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_pilot_logbook_entry(
        &entry.flight_id,
        entry.pic_time,
        entry.sic_time,
        entry.dual_time,
        entry.instructor_time,
        entry.solo_time,
        entry.cross_country_time,
        entry.day_time,
        entry.night_time,
        entry.actual_instrument_time,
        entry.simulated_instrument_time,
        entry.ground_trainer_time,
        entry.day_takeoffs,
        entry.day_landings,
        entry.night_takeoffs,
        entry.night_landings,
        entry.ils_approaches,
        entry.vor_approaches,
        entry.ndb_approaches,
        entry.gps_approaches,
        entry.visual_approaches,
        entry.ifr_time,
        entry.vfr_time,
        entry.pilot_name.as_deref(),
        entry.copilot_name.as_deref(),
        entry.instructor_name.as_deref(),
        entry.route.as_deref(),
        entry.remarks.as_deref(),
        entry.endorsements.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pilot_logbook_entry(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::PilotLogbook>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_pilot_logbook_entry(&entry_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pilot_logbook_by_flight(
    flight_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::PilotLogbook>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_pilot_logbook_by_flight(&flight_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_all_pilot_logbook_entries(
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::PilotLogbook>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_all_pilot_logbook_entries()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_pilot_logbook_entry(
    entry_id: String,
    entry: crate::models::PilotLogbookInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_pilot_logbook_entry(
        &entry_id,
        entry.pic_time,
        entry.sic_time,
        entry.dual_time,
        entry.instructor_time,
        entry.solo_time,
        entry.cross_country_time,
        entry.day_time,
        entry.night_time,
        entry.actual_instrument_time,
        entry.simulated_instrument_time,
        entry.ground_trainer_time,
        entry.day_takeoffs,
        entry.day_landings,
        entry.night_takeoffs,
        entry.night_landings,
        entry.ils_approaches,
        entry.vor_approaches,
        entry.ndb_approaches,
        entry.gps_approaches,
        entry.visual_approaches,
        entry.ifr_time,
        entry.vfr_time,
        entry.pilot_name.as_deref(),
        entry.copilot_name.as_deref(),
        entry.instructor_name.as_deref(),
        entry.route.as_deref(),
        entry.remarks.as_deref(),
        entry.endorsements.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_pilot_logbook_entry(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_pilot_logbook_entry(&entry_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_pilot_logbook_totals(
    state: State<'_, AppState>,
) -> Result<crate::models::PilotLogbookTotals, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_pilot_logbook_totals().map_err(|e| e.to_string())
}
