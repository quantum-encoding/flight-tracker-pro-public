// User management commands
use tauri::State;
use super::AppState;
use crate::models::User;

#[tauri::command]
pub fn create_user(user: User, state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_user(&user).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_user(user_id: String, state: State<'_, AppState>) -> Result<Option<User>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_user(&user_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_primary_user(state: State<'_, AppState>) -> Result<Option<User>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_primary_user().map_err(|e| e.to_string())
}