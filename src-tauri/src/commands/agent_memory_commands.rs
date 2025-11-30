use crate::agent_memory;
use crate::commands::AppState;
use tauri::State;

#[tauri::command]
pub fn search_agent_memories(
    search_query: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<agent_memory::MemorySearchResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    agent_memory::search_memories(conn, &search_query, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_flight_memories(
    flight_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<agent_memory::AgentMemory>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    agent_memory::get_flight_memories(conn, &flight_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recent_memories(
    limit: usize,
    agent_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<agent_memory::AgentMemory>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    agent_memory::get_recent_memories(conn, limit, agent_filter.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_memory_stats(
    state: State<'_, AppState>,
) -> Result<agent_memory::MemoryStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    agent_memory::get_memory_stats(conn)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cleanup_expired_memories(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    agent_memory::cleanup_expired_memories(conn)
        .map_err(|e| e.to_string())
}
