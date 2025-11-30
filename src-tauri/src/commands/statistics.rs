// Statistics and analytics commands
use tauri::State;
use super::AppState;
use crate::models::FlightStatistics;

#[tauri::command]
pub fn get_statistics(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<FlightStatistics, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_statistics(&user_id).map_err(|e| e.to_string())
}