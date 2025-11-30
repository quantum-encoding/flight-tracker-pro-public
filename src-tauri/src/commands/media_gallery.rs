// Media Gallery - File storage for photos, documents, receipts
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use super::AppState;

// ===== DATA TYPES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    pub id: String,
    pub user_id: String,
    pub filename: String,
    pub original_filename: String,
    pub file_type: String,        // "photo", "document", "receipt", "boarding_pass", "other"
    pub mime_type: String,
    pub file_size: i64,
    pub file_path: String,        // Relative path within app storage
    pub thumbnail_path: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<String>,     // JSON array
    pub flight_id: Option<String>,
    pub journey_id: Option<String>,
    pub captured_date: Option<String>,
    pub location: Option<String>,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaUploadInput {
    pub file_type: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub flight_id: Option<String>,
    pub journey_id: Option<String>,
    pub captured_date: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaStats {
    pub total_files: i64,
    pub total_photos: i64,
    pub total_documents: i64,
    pub total_receipts: i64,
    pub total_boarding_passes: i64,
    pub total_size_bytes: i64,
    pub favorites_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFilter {
    pub file_type: Option<String>,
    pub flight_id: Option<String>,
    pub journey_id: Option<String>,
    pub favorites_only: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ===== HELPER FUNCTIONS =====

fn get_media_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let media_dir = app_dir.join("media");
    fs::create_dir_all(&media_dir).map_err(|e| format!("Failed to create media directory: {}", e))?;
    Ok(media_dir)
}

fn get_thumbnails_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;
    let thumb_dir = app_dir.join("thumbnails");
    fs::create_dir_all(&thumb_dir).map_err(|e| format!("Failed to create thumbnails directory: {}", e))?;
    Ok(thumb_dir)
}

fn get_mime_type(filename: &str) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "heic" | "heif" => "image/heic",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "txt" => "text/plain",
        "csv" => "text/csv",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn is_image(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

// ===== DATABASE SCHEMA INITIALIZATION =====

pub fn init_media_table(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS media_files (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            filename TEXT NOT NULL,
            original_filename TEXT NOT NULL,
            file_type TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            file_path TEXT NOT NULL,
            thumbnail_path TEXT,
            title TEXT,
            description TEXT,
            tags TEXT,
            flight_id TEXT,
            journey_id TEXT,
            captured_date TEXT,
            location TEXT,
            is_favorite INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (flight_id) REFERENCES flights(id),
            FOREIGN KEY (journey_id) REFERENCES journeys(id)
        )",
        [],
    )?;

    // Create indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_media_user ON media_files(user_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_media_type ON media_files(file_type)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_media_flight ON media_files(flight_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_media_journey ON media_files(journey_id)",
        [],
    )?;

    Ok(())
}

// ===== COMMANDS =====

/// Upload a file to the media gallery
#[tauri::command]
pub async fn upload_media_file(
    app_handle: AppHandle,
    user_id: String,
    source_path: String,
    input: MediaUploadInput,
    state: State<'_, AppState>,
) -> Result<MediaFile, String> {
    // Get original filename
    let source = std::path::Path::new(&source_path);
    let original_filename = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Generate unique filename
    let file_id = Uuid::new_v4().to_string();
    let ext = original_filename.rsplit('.').next().unwrap_or("bin");
    let new_filename = format!("{}.{}", file_id, ext);

    // Get mime type
    let mime_type = get_mime_type(&original_filename);

    // Get media directory and copy file
    let media_dir = get_media_dir(&app_handle)?;
    let dest_path = media_dir.join(&new_filename);

    fs::copy(&source_path, &dest_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    // Get file size
    let metadata = fs::metadata(&dest_path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;
    let file_size = metadata.len() as i64;

    // Generate thumbnail for images
    let thumbnail_path = if is_image(&mime_type) {
        // For now, just use the original - could add image resizing later
        // This keeps it simple without adding image processing dependencies
        None
    } else {
        None
    };

    // Convert tags to JSON
    let tags_json = input.tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    // Insert into database
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Ensure table exists
    init_media_table(&db.conn).map_err(|e| e.to_string())?;

    let now = chrono::Utc::now().to_rfc3339();

    db.conn.execute(
        "INSERT INTO media_files (
            id, user_id, filename, original_filename, file_type, mime_type, file_size,
            file_path, thumbnail_path, title, description, tags, flight_id, journey_id,
            captured_date, location, is_favorite, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 0, ?17, ?17)",
        rusqlite::params![
            file_id,
            user_id,
            new_filename,
            original_filename,
            input.file_type,
            mime_type,
            file_size,
            new_filename, // Store relative path
            thumbnail_path,
            input.title,
            input.description,
            tags_json,
            input.flight_id,
            input.journey_id,
            input.captured_date,
            input.location,
            now,
        ],
    ).map_err(|e| e.to_string())?;

    // Return the created media file
    Ok(MediaFile {
        id: file_id,
        user_id,
        filename: new_filename.clone(),
        original_filename,
        file_type: input.file_type,
        mime_type,
        file_size,
        file_path: new_filename,
        thumbnail_path,
        title: input.title,
        description: input.description,
        tags: tags_json,
        flight_id: input.flight_id,
        journey_id: input.journey_id,
        captured_date: input.captured_date,
        location: input.location,
        is_favorite: false,
        created_at: now.clone(),
        updated_at: now,
    })
}

/// List media files with optional filtering
#[tauri::command]
pub fn list_media_files(
    user_id: String,
    filter: Option<MediaFilter>,
    state: State<'_, AppState>,
) -> Result<Vec<MediaFile>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Ensure table exists
    let _ = init_media_table(&db.conn);

    let filter = filter.unwrap_or(MediaFilter {
        file_type: None,
        flight_id: None,
        journey_id: None,
        favorites_only: None,
        search: None,
        limit: Some(100),
        offset: Some(0),
    });

    let mut conditions = vec!["user_id = ?1".to_string()];
    let mut param_idx = 2;

    if filter.file_type.is_some() {
        conditions.push(format!("file_type = ?{}", param_idx));
        param_idx += 1;
    }
    if filter.flight_id.is_some() {
        conditions.push(format!("flight_id = ?{}", param_idx));
        param_idx += 1;
    }
    if filter.journey_id.is_some() {
        conditions.push(format!("journey_id = ?{}", param_idx));
        param_idx += 1;
    }
    if filter.favorites_only == Some(true) {
        conditions.push("is_favorite = 1".to_string());
    }
    if filter.search.is_some() {
        conditions.push(format!(
            "(title LIKE ?{} OR description LIKE ?{} OR original_filename LIKE ?{})",
            param_idx, param_idx, param_idx
        ));
    }

    let where_clause = conditions.join(" AND ");
    let limit = filter.limit.unwrap_or(100);
    let offset = filter.offset.unwrap_or(0);

    let query = format!(
        "SELECT id, user_id, filename, original_filename, file_type, mime_type, file_size,
                file_path, thumbnail_path, title, description, tags, flight_id, journey_id,
                captured_date, location, is_favorite, created_at, updated_at
         FROM media_files
         WHERE {}
         ORDER BY created_at DESC
         LIMIT {} OFFSET {}",
        where_clause, limit, offset
    );

    let mut stmt = db.conn.prepare(&query).map_err(|e| e.to_string())?;

    // Build params dynamically
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(user_id)];

    if let Some(ft) = &filter.file_type {
        params.push(Box::new(ft.clone()));
    }
    if let Some(fid) = &filter.flight_id {
        params.push(Box::new(fid.clone()));
    }
    if let Some(jid) = &filter.journey_id {
        params.push(Box::new(jid.clone()));
    }
    if let Some(s) = &filter.search {
        let pattern = format!("%{}%", s);
        params.push(Box::new(pattern));
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let files = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(MediaFile {
                id: row.get(0)?,
                user_id: row.get(1)?,
                filename: row.get(2)?,
                original_filename: row.get(3)?,
                file_type: row.get(4)?,
                mime_type: row.get(5)?,
                file_size: row.get(6)?,
                file_path: row.get(7)?,
                thumbnail_path: row.get(8)?,
                title: row.get(9)?,
                description: row.get(10)?,
                tags: row.get(11)?,
                flight_id: row.get(12)?,
                journey_id: row.get(13)?,
                captured_date: row.get(14)?,
                location: row.get(15)?,
                is_favorite: row.get::<_, i32>(16)? == 1,
                created_at: row.get(17)?,
                updated_at: row.get(18)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(files)
}

/// Get a single media file by ID
#[tauri::command]
pub fn get_media_file(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<Option<MediaFile>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let result = db.conn.query_row(
        "SELECT id, user_id, filename, original_filename, file_type, mime_type, file_size,
                file_path, thumbnail_path, title, description, tags, flight_id, journey_id,
                captured_date, location, is_favorite, created_at, updated_at
         FROM media_files WHERE id = ?1",
        [&file_id],
        |row| {
            Ok(MediaFile {
                id: row.get(0)?,
                user_id: row.get(1)?,
                filename: row.get(2)?,
                original_filename: row.get(3)?,
                file_type: row.get(4)?,
                mime_type: row.get(5)?,
                file_size: row.get(6)?,
                file_path: row.get(7)?,
                thumbnail_path: row.get(8)?,
                title: row.get(9)?,
                description: row.get(10)?,
                tags: row.get(11)?,
                flight_id: row.get(12)?,
                journey_id: row.get(13)?,
                captured_date: row.get(14)?,
                location: row.get(15)?,
                is_favorite: row.get::<_, i32>(16)? == 1,
                created_at: row.get(17)?,
                updated_at: row.get(18)?,
            })
        },
    );

    match result {
        Ok(file) => Ok(Some(file)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Get the full file path for a media file
#[tauri::command]
pub fn get_media_file_path(
    app_handle: AppHandle,
    filename: String,
) -> Result<String, String> {
    let media_dir = get_media_dir(&app_handle)?;
    let file_path = media_dir.join(&filename);
    Ok(file_path.to_string_lossy().to_string())
}

/// Update media file metadata
#[tauri::command]
pub fn update_media_file(
    file_id: String,
    title: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>,
    file_type: Option<String>,
    flight_id: Option<String>,
    journey_id: Option<String>,
    captured_date: Option<String>,
    location: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let tags_json = tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    db.conn.execute(
        "UPDATE media_files SET
            title = COALESCE(?2, title),
            description = COALESCE(?3, description),
            tags = COALESCE(?4, tags),
            file_type = COALESCE(?5, file_type),
            flight_id = ?6,
            journey_id = ?7,
            captured_date = COALESCE(?8, captured_date),
            location = COALESCE(?9, location),
            updated_at = datetime('now')
         WHERE id = ?1",
        rusqlite::params![
            file_id,
            title,
            description,
            tags_json,
            file_type,
            flight_id,
            journey_id,
            captured_date,
            location,
        ],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Toggle favorite status
#[tauri::command]
pub fn toggle_media_favorite(
    file_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn.execute(
        "UPDATE media_files SET is_favorite = NOT is_favorite, updated_at = datetime('now') WHERE id = ?1",
        [&file_id],
    ).map_err(|e| e.to_string())?;

    // Return new state
    let is_fav: i32 = db.conn.query_row(
        "SELECT is_favorite FROM media_files WHERE id = ?1",
        [&file_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    Ok(is_fav == 1)
}

/// Delete a media file
#[tauri::command]
pub fn delete_media_file(
    app_handle: AppHandle,
    file_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get filename before deletion
    let filename: String = db.conn.query_row(
        "SELECT filename FROM media_files WHERE id = ?1",
        [&file_id],
        |row| row.get(0),
    ).map_err(|e| e.to_string())?;

    // Delete from database
    db.conn.execute(
        "DELETE FROM media_files WHERE id = ?1",
        [&file_id],
    ).map_err(|e| e.to_string())?;

    // Delete file from disk
    let media_dir = get_media_dir(&app_handle)?;
    let file_path = media_dir.join(&filename);
    if file_path.exists() {
        let _ = fs::remove_file(&file_path);
    }

    Ok(())
}

/// Get media gallery statistics
#[tauri::command]
pub fn get_media_stats(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<MediaStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Ensure table exists
    let _ = init_media_table(&db.conn);

    let total_files: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM media_files WHERE user_id = ?1",
        [&user_id],
        |row| row.get(0),
    ).unwrap_or(0);

    let total_photos: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM media_files WHERE user_id = ?1 AND file_type = 'photo'",
        [&user_id],
        |row| row.get(0),
    ).unwrap_or(0);

    let total_documents: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM media_files WHERE user_id = ?1 AND file_type = 'document'",
        [&user_id],
        |row| row.get(0),
    ).unwrap_or(0);

    let total_receipts: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM media_files WHERE user_id = ?1 AND file_type = 'receipt'",
        [&user_id],
        |row| row.get(0),
    ).unwrap_or(0);

    let total_boarding_passes: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM media_files WHERE user_id = ?1 AND file_type = 'boarding_pass'",
        [&user_id],
        |row| row.get(0),
    ).unwrap_or(0);

    let total_size_bytes: i64 = db.conn.query_row(
        "SELECT COALESCE(SUM(file_size), 0) FROM media_files WHERE user_id = ?1",
        [&user_id],
        |row| row.get(0),
    ).unwrap_or(0);

    let favorites_count: i64 = db.conn.query_row(
        "SELECT COUNT(*) FROM media_files WHERE user_id = ?1 AND is_favorite = 1",
        [&user_id],
        |row| row.get(0),
    ).unwrap_or(0);

    Ok(MediaStats {
        total_files,
        total_photos,
        total_documents,
        total_receipts,
        total_boarding_passes,
        total_size_bytes,
        favorites_count,
    })
}

/// Get media files linked to a specific flight
#[tauri::command]
pub fn get_flight_media(
    flight_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MediaFile>, String> {
    list_media_files(
        String::new(), // Will be overridden by filter
        Some(MediaFilter {
            file_type: None,
            flight_id: Some(flight_id),
            journey_id: None,
            favorites_only: None,
            search: None,
            limit: Some(100),
            offset: Some(0),
        }),
        state,
    )
}

/// Get media files linked to a specific journey
#[tauri::command]
pub fn get_journey_media(
    journey_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<MediaFile>, String> {
    list_media_files(
        String::new(), // Will be overridden by filter
        Some(MediaFilter {
            file_type: None,
            flight_id: None,
            journey_id: Some(journey_id),
            favorites_only: None,
            search: None,
            limit: Some(100),
            offset: Some(0),
        }),
        state,
    )
}
