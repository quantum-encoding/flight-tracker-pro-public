// OCR and boarding pass analysis commands
use tauri::{State, AppHandle, Manager};
use super::AppState;
use crate::ocr;
use crate::ocr_learning;

fn get_api_key(
    env_vars: &[&str],
    db_key: &str,
    state: &State<'_, AppState>,
) -> Result<String, String> {
    // First try environment variables (in order of preference)
    for env_var in env_vars {
        if let Ok(key) = std::env::var(env_var) {
            if !key.is_empty() {
                return Ok(key);
            }
        }
    }

    // Fall back to database setting
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_setting(db_key)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            format!(
                "{} not configured. Set {} environment variable or add it in Settings.",
                db_key, env_vars[0]
            )
        })
}

#[tauri::command]
pub async fn analyze_boarding_pass(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<ocr::OcrFlightResult, String> {
    // Validate file path
    if file_path.is_empty() {
        return Err("No file path provided".to_string());
    }

    // Read the image file
    let image_bytes =
        std::fs::read(&file_path).map_err(|e| format!("Failed to read image file: {}", e))?;

    // Get Gemini API key from environment or settings (try GENAI_API_KEY, GOOGLE_GENAI_API_KEY, or GEMINI_API_KEY)
    let api_key = get_api_key(
        &["GENAI_API_KEY", "GOOGLE_GENAI_API_KEY", "GEMINI_API_KEY"],
        "gemini_api_key",
        &state,
    )?;

    // Get model preference (lite vs standard)
    let use_lite_model = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_setting("use_gemini_lite")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "true".to_string())
            == "true"
    };

    // Call the Gemini OCR function
    let result = ocr::analyze_with_gemini(image_bytes, &api_key, use_lite_model)
        .await
        .map_err(|e| format!("OCR analysis failed: {}", e))?;

    Ok(result)
}

#[tauri::command]
pub async fn batch_analyze_boarding_passes(
    file_paths: Vec<String>,
    state: State<'_, AppState>,
    app_handle: AppHandle,
) -> Result<Vec<Result<ocr::OcrFlightResult, String>>, String> {
    if file_paths.is_empty() {
        return Err("No file paths provided".to_string());
    }

    // Get Gemini API key from environment or settings (try GENAI_API_KEY, GOOGLE_GENAI_API_KEY, or GEMINI_API_KEY)
    let api_key = get_api_key(
        &["GENAI_API_KEY", "GOOGLE_GENAI_API_KEY", "GEMINI_API_KEY"],
        "gemini_api_key",
        &state,
    )?;

    // Get model preference
    let use_lite_model = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_setting("use_gemini_lite")
            .map_err(|e| e.to_string())?
            .unwrap_or_else(|| "true".to_string())
            == "true"
    };

    // Process batch with parallel execution and progress tracking
    let results = ocr::batch_analyze_with_progress(file_paths, &api_key, use_lite_model, &app_handle)
        .await
        .into_iter()
        .map(|r| r.map_err(|e| e.to_string()))
        .collect();

    Ok(results)
}

// ===== OCR LEARNING COMMANDS =====

#[tauri::command]
pub fn record_ocr_correction(
    user_id: String,
    field_name: String,
    original_value: Option<String>,
    corrected_value: String,
    image_hash: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    ocr_learning::record_correction(
        conn,
        &user_id,
        &field_name,
        original_value.as_deref(),
        &corrected_value,
        image_hash.as_deref(),
    )
    .map_err(|e| format!("Failed to record correction: {}", e))
}

#[tauri::command]
pub fn get_ocr_suggestions(
    user_id: String,
    field_values: std::collections::HashMap<String, Option<String>>,
    state: State<'_, AppState>,
) -> Result<Vec<ocr_learning::CorrectionSuggestion>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    ocr_learning::apply_patterns_to_ocr_result(conn, &user_id, &field_values)
        .map_err(|e| format!("Failed to get suggestions: {}", e))
}

#[tauri::command]
pub fn accept_ocr_suggestion(
    user_id: String,
    field_name: String,
    match_pattern: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    ocr_learning::accept_suggestion(conn, &user_id, &field_name, &match_pattern)
        .map_err(|e| format!("Failed to accept suggestion: {}", e))
}

#[tauri::command]
pub fn reject_ocr_suggestion(
    user_id: String,
    field_name: String,
    match_pattern: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    ocr_learning::reject_suggestion(conn, &user_id, &field_name, &match_pattern)
        .map_err(|e| format!("Failed to reject suggestion: {}", e))
}

#[tauri::command]
pub fn get_ocr_correction_history(
    user_id: String,
    limit: i32,
    state: State<'_, AppState>,
) -> Result<Vec<ocr_learning::OcrCorrection>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    ocr_learning::get_correction_history(conn, &user_id, limit)
        .map_err(|e| format!("Failed to get correction history: {}", e))
}

#[tauri::command]
pub fn get_active_learning_patterns(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ocr_learning::OcrLearningPattern>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    ocr_learning::get_active_patterns(conn, &user_id)
        .map_err(|e| format!("Failed to get learning patterns: {}", e))
}