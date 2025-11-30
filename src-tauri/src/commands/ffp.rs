// Frequent Flyer Program commands
use tauri::State;

use super::AppState;

// ===== FREQUENT FLYER PROGRAM COMMANDS =====

#[tauri::command]
pub fn create_ffp(
    ffp: crate::models::FFPInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_ffp(
        &ffp.user_id,
        &ffp.program_name,
        ffp.airline.as_deref(),
        ffp.alliance.as_deref(),
        ffp.member_number.as_deref(),
        ffp.tier_status.as_deref(),
        ffp.current_miles,
        ffp.lifetime_miles,
        ffp.tier_miles,
        ffp.tier_expiry_date.as_deref(),
        ffp.notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_ffp(
    ffp_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::FrequentFlyerProgram>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_ffp(&ffp_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_user_ffps(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::FrequentFlyerProgram>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_user_ffps(&user_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_ffp(
    ffp_id: String,
    ffp: crate::models::FFPInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_ffp(
        &ffp_id,
        &ffp.program_name,
        ffp.airline.as_deref(),
        ffp.alliance.as_deref(),
        ffp.member_number.as_deref(),
        ffp.tier_status.as_deref(),
        ffp.current_miles,
        ffp.lifetime_miles,
        ffp.tier_miles,
        ffp.tier_expiry_date.as_deref(),
        ffp.notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_ffp(ffp_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_ffp(&ffp_id).map_err(|e| e.to_string())
}
