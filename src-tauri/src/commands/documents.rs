// Custom documents management commands
use tauri::State;
use std::fs;
use std::path::Path;

use super::AppState;

// ===== CUSTOM DOCUMENTS COMMANDS =====

#[tauri::command]
pub fn create_custom_document(
    user_id: String,
    document: crate::models::CustomDocumentInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Convert tags vec to JSON string
    let tags_json = if let Some(tags) = &document.tags {
        Some(serde_json::to_string(tags).map_err(|e| e.to_string())?)
    } else {
        None
    };

    db.create_custom_document(
        &user_id,
        &document.title,
        &document.content,
        document.category.as_deref(),
        tags_json.as_deref(),
        document.flight_id.as_deref(),
        document.journey_id.as_deref(),
        document.passenger_name.as_deref(),
        document.fuel_entry_id.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_custom_document(
    document_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::CustomDocument>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_custom_document(&document_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_user_documents(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::CustomDocument>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_user_documents(&user_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_custom_document(
    document_id: String,
    document: crate::models::CustomDocumentInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Convert tags vec to JSON string
    let tags_json = if let Some(tags) = &document.tags {
        Some(serde_json::to_string(tags).map_err(|e| e.to_string())?)
    } else {
        None
    };

    db.update_custom_document(
        &document_id,
        &document.title,
        &document.content,
        document.category.as_deref(),
        tags_json.as_deref(),
        document.flight_id.as_deref(),
        document.journey_id.as_deref(),
        document.passenger_name.as_deref(),
        document.fuel_entry_id.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_custom_document(
    document_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_custom_document(&document_id)
        .map_err(|e| e.to_string())
}

/// Import a document from an image file using OCR (Gemini 2.5 Flash Lite)
/// Extracts text from the image and creates a new document with the content
#[tauri::command]
pub async fn import_document_with_ocr(
    user_id: String,
    image_path: String,
    title: String,
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get Gemini API key from settings (clone to avoid holding lock across await)
    let gemini_api_key = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_setting("gemini_api_key")
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Gemini API key not configured".to_string())?
    }; // Lock released here

    // Read the image file
    let image_bytes = fs::read(&image_path)
        .map_err(|e| format!("Failed to read image file: {}", e))?;

    // Get filename for default title
    let filename = Path::new(&image_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Imported Document")
        .to_string();

    // Extract text using Gemini OCR with retry logic
    let extracted_text = crate::ocr::extract_document_text_with_retry(
        image_bytes,
        &gemini_api_key,
        5, // max retries
    )
    .await
    .map_err(|e| format!("OCR extraction failed: {}", e))?;

    // Prepare document title
    let document_title = if title.is_empty() {
        format!("OCR Import: {}", filename)
    } else {
        title
    };

    // Create document with extracted text
    let document_id = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.create_custom_document(
            &user_id,
            &document_title,
            &extracted_text,
            category.as_deref(),
            None, // tags
            None, // flight_id
            None, // journey_id
            None, // passenger_name
            None, // fuel_entry_id
        )
        .map_err(|e| e.to_string())?
    }; // Lock released here

    Ok(document_id)
}
