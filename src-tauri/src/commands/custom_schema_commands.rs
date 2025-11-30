use super::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomSchema {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub icon: String,
    pub color: String,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaField {
    pub id: String,
    pub schema_id: String,
    pub name: String,
    pub display_name: String,
    pub field_type: String, // text, number, date, boolean, select, relation
    pub is_required: bool,
    pub default_value: Option<String>,
    pub options: Option<String>, // JSON for select options or relation config
    pub validation_rules: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CustomRecord {
    pub id: String,
    pub schema_id: String,
    pub user_id: String,
    pub data: String, // JSON object with field values
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSchemaInput {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub fields: Vec<CreateFieldInput>,
}

#[derive(Debug, Deserialize)]
pub struct CreateFieldInput {
    pub name: String,
    pub display_name: String,
    pub field_type: String,
    pub is_required: Option<bool>,
    pub default_value: Option<String>,
    pub options: Option<String>,
    pub validation_rules: Option<String>,
}

// ===== SCHEMA CRUD =====

#[tauri::command]
pub fn create_custom_schema(
    state: State<'_, AppState>,
    user_id: String,
    input: CreateSchemaInput,
) -> Result<CustomSchema, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let schema_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    db.conn
        .execute(
            "INSERT INTO custom_schemas (id, user_id, name, display_name, description, icon, color, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &schema_id,
                &user_id,
                &input.name,
                &input.display_name,
                &input.description,
                input.icon.as_deref().unwrap_or("database"),
                input.color.as_deref().unwrap_or("#6366f1"),
                &now,
                &now
            ],
        )
        .map_err(|e| e.to_string())?;

    // Insert fields
    for (i, field) in input.fields.iter().enumerate() {
        let field_id = Uuid::new_v4().to_string();
        db.conn
            .execute(
                "INSERT INTO custom_schema_fields (id, schema_id, name, display_name, field_type, is_required, default_value, options, validation_rules, sort_order, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    &field_id,
                    &schema_id,
                    &field.name,
                    &field.display_name,
                    &field.field_type,
                    field.is_required.unwrap_or(false),
                    &field.default_value,
                    &field.options,
                    &field.validation_rules,
                    i as i32,
                    &now
                ],
            )
            .map_err(|e| e.to_string())?;
    }

    Ok(CustomSchema {
        id: schema_id,
        user_id,
        name: input.name,
        display_name: input.display_name,
        description: input.description,
        icon: input.icon.unwrap_or_else(|| "database".to_string()),
        color: input.color.unwrap_or_else(|| "#6366f1".to_string()),
        is_active: true,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub fn list_custom_schemas(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<Vec<CustomSchema>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT id, user_id, name, display_name, description, icon, color, is_active, created_at, updated_at
             FROM custom_schemas WHERE user_id = ?1 AND is_active = 1 ORDER BY display_name",
        )
        .map_err(|e| e.to_string())?;

    let schemas = stmt
        .query_map([&user_id], |row: &rusqlite::Row| {
            Ok(CustomSchema {
                id: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                display_name: row.get(3)?,
                description: row.get(4)?,
                icon: row.get(5)?,
                color: row.get(6)?,
                is_active: row.get::<_, i32>(7)? == 1,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<CustomSchema, _>| r.ok())
        .collect();

    Ok(schemas)
}

#[tauri::command]
pub fn get_schema_fields(
    state: State<'_, AppState>,
    schema_id: String,
) -> Result<Vec<SchemaField>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT id, schema_id, name, display_name, field_type, is_required, default_value, options, validation_rules, sort_order
             FROM custom_schema_fields WHERE schema_id = ?1 ORDER BY sort_order",
        )
        .map_err(|e| e.to_string())?;

    let fields = stmt
        .query_map([&schema_id], |row: &rusqlite::Row| {
            Ok(SchemaField {
                id: row.get(0)?,
                schema_id: row.get(1)?,
                name: row.get(2)?,
                display_name: row.get(3)?,
                field_type: row.get(4)?,
                is_required: row.get::<_, i32>(5)? == 1,
                default_value: row.get(6)?,
                options: row.get(7)?,
                validation_rules: row.get(8)?,
                sort_order: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<SchemaField, _>| r.ok())
        .collect();

    Ok(fields)
}

#[tauri::command]
pub fn delete_custom_schema(
    state: State<'_, AppState>,
    schema_id: String,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Soft delete - set is_active = 0
    db.conn
        .execute(
            "UPDATE custom_schemas SET is_active = 0, updated_at = datetime('now') WHERE id = ?1",
            [&schema_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(true)
}

// ===== RECORD CRUD =====

#[tauri::command]
pub fn create_custom_record(
    state: State<'_, AppState>,
    schema_id: String,
    user_id: String,
    data: String,
) -> Result<CustomRecord, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let record_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    db.conn
        .execute(
            "INSERT INTO custom_records (id, schema_id, user_id, data, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![&record_id, &schema_id, &user_id, &data, &now, &now],
        )
        .map_err(|e| e.to_string())?;

    Ok(CustomRecord {
        id: record_id,
        schema_id,
        user_id,
        data,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub fn list_custom_records(
    state: State<'_, AppState>,
    schema_id: String,
) -> Result<Vec<CustomRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT id, schema_id, user_id, data, created_at, updated_at
             FROM custom_records WHERE schema_id = ?1 ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let records = stmt
        .query_map([&schema_id], |row: &rusqlite::Row| {
            Ok(CustomRecord {
                id: row.get(0)?,
                schema_id: row.get(1)?,
                user_id: row.get(2)?,
                data: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<CustomRecord, _>| r.ok())
        .collect();

    Ok(records)
}

#[tauri::command]
pub fn update_custom_record(
    state: State<'_, AppState>,
    record_id: String,
    data: String,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute(
            "UPDATE custom_records SET data = ?1, updated_at = datetime('now') WHERE id = ?2",
            [&data, &record_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub fn delete_custom_record(
    state: State<'_, AppState>,
    record_id: String,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute("DELETE FROM custom_records WHERE id = ?1", [&record_id])
        .map_err(|e| e.to_string())?;

    Ok(true)
}

// ===== FLIGHT CUSTOM FIELDS =====

#[tauri::command]
pub fn set_flight_custom_field(
    state: State<'_, AppState>,
    flight_id: String,
    field_name: String,
    field_value: String,
    field_type: String,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    db.conn
        .execute(
            "INSERT INTO flight_custom_fields (id, flight_id, field_name, field_value, field_type, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(flight_id, field_name) DO UPDATE SET field_value = ?4, field_type = ?5",
            rusqlite::params![&id, &flight_id, &field_name, &field_value, &field_type],
        )
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub fn get_flight_custom_fields(
    state: State<'_, AppState>,
    flight_id: String,
) -> Result<Vec<(String, String, String)>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT field_name, field_value, field_type FROM flight_custom_fields WHERE flight_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let fields = stmt
        .query_map([&flight_id], |row: &rusqlite::Row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<(String, String, String), _>| r.ok())
        .collect();

    Ok(fields)
}
