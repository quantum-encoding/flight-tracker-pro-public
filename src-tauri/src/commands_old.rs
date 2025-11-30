// Tauri commands - Backend API for the frontend

use crate::database::Database;
use crate::models::*;
use crate::ocr;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db: Mutex<Database>,
}

// ===== SETTINGS COMMANDS =====

#[tauri::command]
pub fn get_setting(key: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_setting(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_setting(key: String, value: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_setting(&key, &value).map_err(|e| e.to_string())
}

// ===== USER COMMANDS =====

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

// ===== FLIGHT COMMANDS =====

#[tauri::command]
pub fn create_flight(
    user_id: String,
    flight: FlightInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_flight(&user_id, &flight)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_flight(flight_id: String, state: State<'_, AppState>) -> Result<Option<Flight>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_flight(&flight_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_flights(
    user_id: String,
    limit: i32,
    offset: i32,
    state: State<'_, AppState>,
) -> Result<Vec<Flight>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_flights(&user_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_flight(flight_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_flight(&flight_id).map_err(|e| e.to_string())
}

// ===== STATISTICS COMMANDS =====

#[tauri::command]
pub fn get_statistics(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<FlightStatistics, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_statistics(&user_id).map_err(|e| e.to_string())
}

// ===== OCR COMMANDS =====

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

    // Process batch with intelligent retry and configurable delay (3 seconds between requests)
    let results = ocr::batch_analyze_with_progress(file_paths, &api_key, use_lite_model, 3000)
        .await
        .into_iter()
        .map(|r| r.map_err(|e| e.to_string()))
        .collect();

    Ok(results)
}

// ===== CSV IMPORT COMMANDS =====

#[derive(Debug, Serialize, Deserialize)]
pub struct CsvImportResult {
    pub success_count: usize,
    pub error_count: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditedCsvRow {
    pub row_number: usize,
    pub date: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub passengers: Option<String>,
    pub flight_number: Option<String>,
    pub aircraft_registration: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvColumnMapping {
    pub date_column: usize,
    pub from_column: usize,
    pub to_column: usize,
    pub passengers_column: Option<usize>,
    pub flight_number_column: Option<usize>,
    pub aircraft_registration_column: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvPreviewRow {
    pub row_number: usize,
    pub date: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub passengers: String,
    pub flight_number: Option<String>,
    pub aircraft_registration: Option<String>,
    pub parsed_date: Option<String>,
    pub distance_nm: Option<f64>,
    pub validation_errors: Vec<String>,
    pub raw_values: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CsvPreviewResult {
    pub headers: Vec<String>,
    pub detected_mapping: CsvColumnMapping,
    pub preview_rows: Vec<CsvPreviewRow>,
    pub total_rows: usize,
    pub valid_rows: usize,
    pub invalid_rows: usize,
}

#[tauri::command]
pub fn preview_csv_import(
    csv_path: String,
    max_preview_rows: Option<usize>,
) -> Result<CsvPreviewResult, String> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(&csv_path).map_err(|e| format!("Failed to open CSV file: {}", e))?;

    let mut reader = ::csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(BufReader::new(file));

    // Get headers
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect();

    // Detect column mapping based on headers
    let detected_mapping = detect_column_mapping(&headers);

    let max_rows = max_preview_rows.unwrap_or(10);
    let mut preview_rows = Vec::new();
    let mut total_rows = 0;
    let mut valid_rows = 0;
    let mut invalid_rows = 0;

    for (idx, result) in reader.records().enumerate() {
        total_rows += 1;

        match result {
            Ok(record) => {
                let raw_values: Vec<String> = record.iter().map(|s| s.to_string()).collect();

                if preview_rows.len() < max_rows {
                    let preview_row = parse_csv_row_preview(
                        idx + 2, // +2 for header and 0-indexing
                        &record,
                        &detected_mapping,
                        &raw_values,
                    );

                    if preview_row.validation_errors.is_empty() {
                        valid_rows += 1;
                    } else {
                        invalid_rows += 1;
                    }

                    preview_rows.push(preview_row);
                } else {
                    // Just count remaining rows for validation
                    let has_errors = validate_csv_row(&record, &detected_mapping);
                    if has_errors {
                        invalid_rows += 1;
                    } else {
                        valid_rows += 1;
                    }
                }
            }
            Err(_) => {
                invalid_rows += 1;
            }
        }
    }

    Ok(CsvPreviewResult {
        headers,
        detected_mapping,
        preview_rows,
        total_rows,
        valid_rows,
        invalid_rows,
    })
}

fn detect_column_mapping(headers: &[String]) -> CsvColumnMapping {
    let mut date_column = 0;
    let mut from_column = 1;
    let mut to_column = 2;
    let mut passengers_column = None;
    let mut flight_number_column = None;
    let mut aircraft_registration_column = None;

    for (idx, header) in headers.iter().enumerate() {
        let lower = header.to_lowercase();

        if lower.contains("date") || lower.contains("departure_datetime") {
            date_column = idx;
        } else if lower.contains("from") || lower.contains("origin") || lower.contains("departure")
        {
            from_column = idx;
        } else if lower.contains("to") || lower.contains("destination") || lower.contains("arrival")
        {
            to_column = idx;
        } else if lower.contains("passenger") {
            passengers_column = Some(idx);
        } else if lower.contains("flight") && lower.contains("number") {
            flight_number_column = Some(idx);
        } else if lower.contains("tail")
            || lower.contains("registration")
            || lower.contains("aircraft")
        {
            aircraft_registration_column = Some(idx);
        }
    }

    CsvColumnMapping {
        date_column,
        from_column,
        to_column,
        passengers_column,
        flight_number_column,
        aircraft_registration_column,
    }
}

fn parse_csv_row_preview(
    row_number: usize,
    record: &csv::StringRecord,
    mapping: &CsvColumnMapping,
    raw_values: &[String],
) -> CsvPreviewRow {
    let mut validation_errors = Vec::new();

    // Extract values
    let date = record
        .get(mapping.date_column)
        .unwrap_or("")
        .trim()
        .to_string();
    let departure_airport = record
        .get(mapping.from_column)
        .unwrap_or("")
        .trim()
        .to_uppercase();
    let arrival_airport = record
        .get(mapping.to_column)
        .unwrap_or("")
        .trim()
        .to_uppercase();

    let passengers = mapping
        .passengers_column
        .and_then(|col| record.get(col))
        .unwrap_or("")
        .trim()
        .to_string();

    let flight_number = mapping
        .flight_number_column
        .and_then(|col| record.get(col))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let aircraft_registration = mapping
        .aircraft_registration_column
        .and_then(|col| record.get(col))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Validate date
    let parsed_date = if date.is_empty() {
        validation_errors.push("Date is empty".to_string());
        None
    } else if !is_valid_date_field(&date) {
        validation_errors.push("Date format not recognized".to_string());
        None
    } else {
        match parse_date(&date) {
            Some(d) => Some(d),
            None => {
                validation_errors.push("Failed to parse date".to_string());
                None
            }
        }
    };

    // Validate airports
    if departure_airport.is_empty() {
        validation_errors.push("Departure airport is empty".to_string());
    } else if departure_airport.len() < 2 || departure_airport.len() > 4 {
        validation_errors.push("Invalid departure airport code".to_string());
    }

    if arrival_airport.is_empty() {
        validation_errors.push("Arrival airport is empty".to_string());
    } else if arrival_airport.len() < 2 || arrival_airport.len() > 4 {
        validation_errors.push("Invalid arrival airport code".to_string());
    }

    // Calculate distance
    let distance_nm = if validation_errors.is_empty() {
        crate::geo::calculate_airport_distance(&departure_airport, &arrival_airport)
            .map(|(nm, _)| nm)
    } else {
        None
    };

    CsvPreviewRow {
        row_number,
        date,
        departure_airport,
        arrival_airport,
        passengers,
        flight_number,
        aircraft_registration,
        parsed_date,
        distance_nm,
        validation_errors,
        raw_values: raw_values.to_vec(),
    }
}

fn validate_csv_row(record: &csv::StringRecord, mapping: &CsvColumnMapping) -> bool {
    let date = record.get(mapping.date_column).unwrap_or("").trim();
    let departure = record.get(mapping.from_column).unwrap_or("").trim();
    let arrival = record.get(mapping.to_column).unwrap_or("").trim();

    // Has errors if any validation fails
    date.is_empty()
        || !is_valid_date_field(date)
        || parse_date(date).is_none()
        || departure.is_empty()
        || arrival.is_empty()
        || departure.len() < 2
        || departure.len() > 4
        || arrival.len() < 2
        || arrival.len() > 4
}

#[tauri::command]
pub fn import_flights_from_csv_with_mapping(
    user_id: String,
    csv_path: String,
    column_mapping: CsvColumnMapping,
    edited_rows: Option<Vec<EditedCsvRow>>,
    state: State<'_, AppState>,
) -> Result<CsvImportResult, String> {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufReader;

    // Build a map of edited rows for quick lookup
    let edited_map: HashMap<usize, EditedCsvRow> = edited_rows
        .unwrap_or_default()
        .into_iter()
        .map(|row| (row.row_number, row))
        .collect();

    let file = File::open(&csv_path).map_err(|e| format!("Failed to open CSV file: {}", e))?;

    let mut reader = ::csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(BufReader::new(file));

    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();

    let db = state.db.lock().map_err(|e| e.to_string())?;

    for (idx, result) in reader.records().enumerate() {
        let row_number = idx + 2; // +2 for header and 0-indexing

        match result {
            Ok(record) => {
                // Check if this row has been edited
                let flight_data = if let Some(edited) = edited_map.get(&row_number) {
                    // Use edited data
                    (
                        edited.date.clone(),
                        edited.departure_airport.clone(),
                        edited.arrival_airport.clone(),
                        edited.passengers.clone().unwrap_or_default(),
                        edited.flight_number.clone(),
                        edited.aircraft_registration.clone(),
                    )
                } else {
                    // Use data from CSV with column mapping
                    let date = record
                        .get(column_mapping.date_column)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let departure = record
                        .get(column_mapping.from_column)
                        .unwrap_or("")
                        .trim()
                        .to_uppercase();
                    let arrival = record
                        .get(column_mapping.to_column)
                        .unwrap_or("")
                        .trim()
                        .to_uppercase();

                    let passengers = column_mapping
                        .passengers_column
                        .and_then(|col| record.get(col))
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    let flight_number = column_mapping
                        .flight_number_column
                        .and_then(|col| record.get(col))
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    let aircraft_reg = column_mapping
                        .aircraft_registration_column
                        .and_then(|col| record.get(col))
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty());

                    (
                        date,
                        departure,
                        arrival,
                        passengers,
                        flight_number,
                        aircraft_reg,
                    )
                };

                let (date, departure, arrival, passengers, flight_number, aircraft_registration) =
                    flight_data;

                // Validate
                if date.is_empty() || !is_valid_date_field(&date) {
                    continue;
                }

                let departure_datetime = match parse_date(&date) {
                    Some(dt) => dt,
                    None => continue,
                };

                if departure.is_empty() || arrival.is_empty() {
                    continue;
                }

                if departure.len() < 2
                    || departure.len() > 4
                    || arrival.len() < 2
                    || arrival.len() > 4
                {
                    continue;
                }

                // Calculate distance
                let distance_nm =
                    crate::geo::calculate_airport_distance(&departure, &arrival).map(|(nm, _)| nm);

                let flight = FlightInput {
                    flight_number,
                    departure_airport: departure,
                    arrival_airport: arrival,
                    departure_datetime,
                    arrival_datetime: None,
                    aircraft_type_id: None,
                    aircraft_registration,
                    total_duration: None,
                    flight_duration: None,
                    distance_nm,
                    distance_km: None, // Will be calculated from distance_nm in database layer
                    carbon_emissions_kg: None, // Will be calculated from distance in database layer
                    booking_reference: None,
                    ticket_number: None,
                    seat_number: None,
                    fare_class: None,
                    base_fare: None,
                    taxes: None,
                    total_cost: None,
                    currency: Some("USD".to_string()),
                    notes: if !passengers.is_empty() {
                        Some(format!("Passengers: {}", passengers))
                    } else {
                        None
                    },
                    attachment_path: None,
                };

                match db.create_flight(&user_id, &flight) {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        errors.push(format!("Row {}: {}", row_number, e));
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Row {}: CSV parse error: {}", row_number, e));
                error_count += 1;
            }
        }
    }

    Ok(CsvImportResult {
        success_count,
        error_count,
        errors,
    })
}

#[tauri::command]
pub fn import_flights_from_csv(
    user_id: String,
    csv_path: String,
    state: State<'_, AppState>,
) -> Result<CsvImportResult, String> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(&csv_path).map_err(|e| format!("Failed to open CSV file: {}", e))?;

    let mut reader = ::csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(BufReader::new(file));

    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();

    let db = state.db.lock().map_err(|e| e.to_string())?;

    for (idx, result) in reader.records().enumerate() {
        let line_num = idx + 2; // +2 because line 1 is header and we're 0-indexed

        match result {
            Ok(record) => {
                // Skip rows with insufficient data
                if record.len() < 3 {
                    continue;
                }

                let date = record.get(0).unwrap_or("").trim();

                // Skip rows where date field is empty or doesn't look like a date
                if date.is_empty() || !is_valid_date_field(date) {
                    continue;
                }

                // Try to parse the date first - if it fails, skip this row as it's likely a header fragment
                let departure_datetime = match parse_date(date) {
                    Some(dt) => dt,
                    None => {
                        continue; // Skip invalid date rows silently (likely multiline headers)
                    }
                };

                // Smart column detection - try multiple formats
                // Format 1: Date, Tail Number, From, To, Passengers, Origin Airport, Destination Airport
                // Format 2: date, from, to, passenger_list, origin_code, destination_code

                let (departure, arrival, passengers) = if record.len() >= 7 {
                    // Long format - use Origin Airport (col 5) and Destination Airport (col 6)
                    let origin = record.get(5).unwrap_or("").trim();
                    let dest = record.get(6).unwrap_or("").trim();
                    let pass = record.get(4).unwrap_or("").trim();

                    // Fallback to columns 1-3 if 5-6 are empty
                    if !origin.is_empty() && !dest.is_empty() {
                        (origin, dest, pass)
                    } else {
                        let from = record.get(1).unwrap_or("").trim();
                        let to = record.get(2).unwrap_or("").trim();
                        let p = record.get(3).unwrap_or("").trim();
                        (from, to, p)
                    }
                } else if record.len() >= 4 {
                    // Short format
                    let from = record.get(1).unwrap_or("").trim();
                    let to = record.get(2).unwrap_or("").trim();
                    let pass = record.get(3).unwrap_or("").trim();
                    (from, to, pass)
                } else {
                    continue;
                };

                // Validate airport codes (must be 3-4 letter codes, not header text)
                if departure.is_empty() || arrival.is_empty() {
                    continue; // Skip silently - likely header fragments
                }

                // Airport codes should be 2-4 uppercase letters/numbers
                let is_valid_airport = |code: &str| -> bool {
                    code.len() >= 2
                        && code.len() <= 4
                        && code.chars().all(|c| c.is_ascii_alphanumeric())
                };

                if !is_valid_airport(departure) || !is_valid_airport(arrival) {
                    continue; // Skip invalid airport codes silently
                }

                // Calculate distance if we can find the airports
                let distance_nm =
                    crate::geo::calculate_airport_distance(departure, arrival).map(|(nm, _)| nm);

                let flight = FlightInput {
                    flight_number: None,
                    departure_airport: departure.to_uppercase(),
                    arrival_airport: arrival.to_uppercase(),
                    departure_datetime,
                    arrival_datetime: None,
                    aircraft_type_id: None,
                    aircraft_registration: None,
                    total_duration: None,
                    flight_duration: None,
                    distance_nm,
                    distance_km: None, // Will be calculated from distance_nm in database layer
                    carbon_emissions_kg: None, // Will be calculated from distance in database layer
                    booking_reference: None,
                    ticket_number: None,
                    seat_number: None,
                    fare_class: None,
                    base_fare: None,
                    taxes: None,
                    total_cost: None,
                    currency: Some("USD".to_string()),
                    notes: if !passengers.is_empty() {
                        Some(format!("Passengers: {}", passengers))
                    } else {
                        None
                    },
                    attachment_path: None,
                };

                match db.create_flight(&user_id, &flight) {
                    Ok(_) => success_count += 1,
                    Err(e) => {
                        errors.push(format!("Line {}: {}", line_num, e));
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Line {}: CSV parse error: {}", line_num, e));
                error_count += 1;
            }
        }
    }

    Ok(CsvImportResult {
        success_count,
        error_count,
        errors,
    })
}

fn is_valid_date_field(s: &str) -> bool {
    // Check if string looks like a date (contains digits and common separators)
    let has_digits = s.chars().any(|c| c.is_ascii_digit());
    let has_separators = s.contains('-') || s.contains('/');

    // Must have digits and either be all digits or have separators
    has_digits && (s.chars().all(|c| c.is_ascii_digit()) || has_separators)
}

fn parse_date(date_str: &str) -> Option<String> {
    use chrono::NaiveDate;

    // Try ISO format (YYYY-MM-DD)
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        return Some(format!("{}T00:00:00", date.format("%Y-%m-%d")));
    }

    // Try US format (MM/DD/YYYY)
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%m/%d/%Y") {
        return Some(format!("{}T00:00:00", date.format("%Y-%m-%d")));
    }

    // Try European format (DD/MM/YYYY)
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%d/%m/%Y") {
        return Some(format!("{}T00:00:00", date.format("%Y-%m-%d")));
    }

    // Try compact format (YYYYMMDD)
    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y%m%d") {
        return Some(format!("{}T00:00:00", date.format("%Y-%m-%d")));
    }

    None
}

// ===== DATA MANAGEMENT COMMANDS =====

#[tauri::command]
pub fn export_data_to_csv(
    user_id: String,
    export_path: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    use std::fs::File;

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all flights for the user
    let flights = db
        .list_flights(&user_id, i32::MAX, 0)
        .map_err(|e| e.to_string())?;

    let file =
        File::create(&export_path).map_err(|e| format!("Failed to create export file: {}", e))?;

    let mut writer = ::csv::Writer::from_writer(file);

    // Write header
    writer
        .write_record([
            "Date",
            "Flight Number",
            "Departure Airport",
            "Arrival Airport",
            "Departure Time",
            "Arrival Time",
            "Distance (NM)",
            "Distance (KM)",
            "Duration (minutes)",
            "Aircraft Registration",
            "Seat Number",
            "Fare Class",
            "Total Cost",
            "Currency",
            "Booking Reference",
            "Notes",
        ])
        .map_err(|e| e.to_string())?;

    // Write flight records
    for flight in &flights {
        writer
            .write_record([
                flight.departure_datetime.split('T').next().unwrap_or(""),
                flight.flight_number.as_deref().unwrap_or(""),
                &flight.departure_airport,
                &flight.arrival_airport,
                &flight.departure_datetime,
                flight.arrival_datetime.as_deref().unwrap_or(""),
                &flight
                    .distance_nm
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                &flight
                    .distance_km
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                &flight
                    .flight_duration
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                flight.aircraft_registration.as_deref().unwrap_or(""),
                flight.seat_number.as_deref().unwrap_or(""),
                flight.fare_class.as_deref().unwrap_or(""),
                &flight.total_cost.map(|c| c.to_string()).unwrap_or_default(),
                flight.currency.as_deref().unwrap_or(""),
                flight.booking_reference.as_deref().unwrap_or(""),
                flight.notes.as_deref().unwrap_or(""),
            ])
            .map_err(|e| e.to_string())?;
    }

    writer.flush().map_err(|e| e.to_string())?;

    Ok(flights.len())
}

#[tauri::command]
pub fn reset_database(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Delete all data from tables (in correct order due to foreign keys)
    db.conn
        .execute("DELETE FROM pilot_logbook", [])
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM journey_flights", [])
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM journeys", [])
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM ocr_queue", [])
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM flights", [])
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM users", [])
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM settings", [])
        .map_err(|e| e.to_string())?;
    db.conn
        .execute("DELETE FROM statistics_cache", [])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ===== INVESTIGATION COMMANDS =====

#[tauri::command]
pub async fn investigate_flight(
    flight_id: String,
    passenger_names: Vec<String>,
    state: State<'_, AppState>,
) -> Result<crate::models::InvestigationResult, String> {
    use crate::investigation;

    // Get the flight details
    let flight = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_flight(&flight_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Flight not found".to_string())?
    };

    // Get Gemini API key from environment or settings (try GENAI_API_KEY, GOOGLE_GENAI_API_KEY, or GEMINI_API_KEY)
    let api_key = get_api_key(
        &["GENAI_API_KEY", "GOOGLE_GENAI_API_KEY", "GEMINI_API_KEY"],
        "gemini_api_key",
        &state,
    )?;

    // Extract investigation parameters
    let location = format!("{} to {}", flight.departure_airport, flight.arrival_airport);
    let date = flight
        .departure_datetime
        .split('T')
        .next()
        .unwrap_or(&flight.departure_datetime)
        .to_string();

    // Run the investigation
    let result = investigation::run_investigation(passenger_names, location, date, api_key)
        .await
        .map_err(|e| format!("Investigation failed: {}", e))?;

    // Store in database cache (optional - for now just return)
    // TODO: Save to investigations table

    Ok(result)
}

#[tauri::command]
pub fn get_flight_investigation(
    flight_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::Investigation>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let investigation = db
        .conn
        .query_row(
            "SELECT id, flight_id, user_id, passenger_names, location, investigation_date,
                    generated_queries, status, ai_summary, sources_json, corroboration_score,
                    error_message, processing_time_ms, created_at, completed_at
             FROM investigations
             WHERE flight_id = ?1
             ORDER BY created_at DESC
             LIMIT 1",
            rusqlite::params![flight_id],
            |row| {
                Ok(crate::models::Investigation {
                    id: row.get(0)?,
                    flight_id: row.get(1)?,
                    user_id: row.get(2)?,
                    passenger_names: row.get(3)?,
                    location: row.get(4)?,
                    investigation_date: row.get(5)?,
                    generated_queries: row.get(6)?,
                    status: row.get(7)?,
                    ai_summary: row.get(8)?,
                    sources_json: row.get(9)?,
                    corroboration_score: row.get(10)?,
                    error_message: row.get(11)?,
                    processing_time_ms: row.get(12)?,
                    created_at: row.get(13)?,
                    completed_at: row.get(14)?,
                })
            },
        )
        .optional()
        .map_err(|e: rusqlite::Error| e.to_string())?;

    Ok(investigation)
}

// ===== AIRPORT LIST COMMAND =====

#[derive(Debug, Serialize, Deserialize)]
pub struct AirportVisit {
    pub airport_code: String,
    pub visit_count: i32,
    pub departure_count: i32,
    pub arrival_count: i32,
}

#[tauri::command]
pub fn get_airport_list(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AirportVisit>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT
            airport_code,
            SUM(CASE WHEN type = 'departure' THEN 1 ELSE 0 END) as departure_count,
            SUM(CASE WHEN type = 'arrival' THEN 1 ELSE 0 END) as arrival_count,
            COUNT(*) as total_visits
         FROM (
            SELECT departure_airport as airport_code, 'departure' as type FROM flights WHERE user_id = ?1
            UNION ALL
            SELECT arrival_airport as airport_code, 'arrival' as type FROM flights WHERE user_id = ?1
         )
         GROUP BY airport_code
         ORDER BY total_visits DESC, airport_code ASC"
    ).map_err(|e| e.to_string())?;

    let airports = stmt
        .query_map(rusqlite::params![user_id], |row| {
            Ok(AirportVisit {
                airport_code: row.get(0)?,
                departure_count: row.get(1)?,
                arrival_count: row.get(2)?,
                visit_count: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(airports)
}

// ===== DOCUMENTS/INVESTIGATIONS LIST =====

#[derive(Debug, Serialize, Deserialize)]
pub struct InvestigationSummary {
    pub flight_id: String,
    pub flight_number: Option<String>,
    pub route: String,
    pub date: String,
    pub corroboration_score: f64,
    pub passenger_names: String,
    pub created_at: String,
}

#[tauri::command]
pub fn list_all_investigations(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<InvestigationSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db
        .conn
        .prepare(
            "SELECT
            i.flight_id,
            f.flight_number,
            f.departure_airport,
            f.arrival_airport,
            f.departure_datetime,
            i.corroboration_score,
            i.passenger_names,
            i.created_at
         FROM flight_investigations i
         JOIN flights f ON i.flight_id = f.id
         WHERE f.user_id = ?1
         ORDER BY i.created_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let investigations = stmt
        .query_map(rusqlite::params![user_id], |row| {
            let departure: String = row.get(2)?;
            let arrival: String = row.get(3)?;
            let datetime: String = row.get(4)?;

            Ok(InvestigationSummary {
                flight_id: row.get(0)?,
                flight_number: row.get(1)?,
                route: format!("{} → {}", departure, arrival),
                date: datetime.split('T').next().unwrap_or(&datetime).to_string(),
                corroboration_score: row.get(5)?,
                passenger_names: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(investigations)
}

// ===== ANALYTICS COMMANDS =====

#[derive(Debug, Serialize, Deserialize)]
pub struct RouteStatistic {
    pub route: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub flight_count: i32,
    pub total_distance_km: f64,
    pub avg_duration_minutes: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AirportStatistic {
    pub airport_code: String,
    pub visit_count: i32,
    pub departure_count: i32,
    pub arrival_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Analytics {
    pub top_routes: Vec<RouteStatistic>,
    pub most_visited_airports: Vec<AirportStatistic>,
    pub total_unique_routes: i32,
    pub total_unique_airports: i32,
}

#[tauri::command]
pub fn get_analytics(
    user_id: String,
    limit: i32,
    state: State<'_, AppState>,
) -> Result<Analytics, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get top routes
    let mut route_stmt = db
        .conn
        .prepare(
            "SELECT
            departure_airport,
            arrival_airport,
            COUNT(*) as flight_count,
            COALESCE(SUM(distance_km), 0) as total_distance,
            COALESCE(AVG(flight_duration), 0) as avg_duration
         FROM flights
         WHERE user_id = ?1
         GROUP BY departure_airport, arrival_airport
         ORDER BY flight_count DESC
         LIMIT ?2",
        )
        .map_err(|e| e.to_string())?;

    let top_routes = route_stmt
        .query_map(rusqlite::params![user_id, limit], |row| {
            let departure: String = row.get(0)?;
            let arrival: String = row.get(1)?;

            Ok(RouteStatistic {
                departure_airport: departure.clone(),
                arrival_airport: arrival.clone(),
                route: format!("{} → {}", departure, arrival),
                flight_count: row.get(2)?,
                total_distance_km: row.get(3)?,
                avg_duration_minutes: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Get most visited airports
    let mut airport_stmt = db.conn.prepare(
        "SELECT
            airport_code,
            SUM(CASE WHEN type = 'departure' THEN 1 ELSE 0 END) as departure_count,
            SUM(CASE WHEN type = 'arrival' THEN 1 ELSE 0 END) as arrival_count,
            COUNT(*) as total_visits
         FROM (
            SELECT departure_airport as airport_code, 'departure' as type FROM flights WHERE user_id = ?1
            UNION ALL
            SELECT arrival_airport as airport_code, 'arrival' as type FROM flights WHERE user_id = ?1
         )
         GROUP BY airport_code
         ORDER BY total_visits DESC
         LIMIT ?2"
    ).map_err(|e| e.to_string())?;

    let most_visited_airports = airport_stmt
        .query_map(rusqlite::params![user_id, limit], |row| {
            Ok(AirportStatistic {
                airport_code: row.get(0)?,
                departure_count: row.get(1)?,
                arrival_count: row.get(2)?,
                visit_count: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Get total unique routes
    let total_unique_routes: i32 = db
        .conn
        .query_row(
            "SELECT COUNT(DISTINCT departure_airport || '-' || arrival_airport)
         FROM flights WHERE user_id = ?1",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Get total unique airports visited (both departures and arrivals)
    let total_unique_airports: i32 = db
        .conn
        .query_row(
            "SELECT COUNT(DISTINCT airport_code) FROM (
                SELECT departure_airport as airport_code FROM flights WHERE user_id = ?1
                UNION
                SELECT arrival_airport as airport_code FROM flights WHERE user_id = ?1
            )",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(Analytics {
        top_routes,
        most_visited_airports,
        total_unique_routes,
        total_unique_airports,
    })
}

// ===== DATASET MANAGEMENT (Multiple Users) =====

#[tauri::command]
pub fn list_all_users(state: State<'_, AppState>) -> Result<Vec<User>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT id, name, email, pilot_license_number, license_type, license_country, created_at, updated_at
         FROM users
         ORDER BY created_at ASC"
    ).map_err(|e| e.to_string())?;

    let users = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                pilot_license_number: row.get(3)?,
                license_type: row.get(4)?,
                license_country: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(users)
}

#[tauri::command]
pub fn update_user_name(
    user_id: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute(
            "UPDATE users SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![new_name, user_id],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ===== PASSENGER NAME MANAGEMENT =====

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerName {
    pub abbreviation: String,
    pub full_name: Option<String>,
    pub usage_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerMapping {
    pub abbreviation: String,
    pub full_name: String,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn get_all_passenger_names(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PassengerName>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Extract all passenger names from notes field
    let mut stmt = db
        .conn
        .prepare("SELECT notes FROM flights WHERE user_id = ?1 AND notes IS NOT NULL")
        .map_err(|e| e.to_string())?;

    let notes_list: Vec<String> = stmt
        .query_map(rusqlite::params![user_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Parse passenger names from notes
    let mut name_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

    for notes in notes_list {
        if notes.starts_with("Passengers: ") {
            let passenger_part = notes.trim_start_matches("Passengers: ");
            // Split by comma and trim
            for name in passenger_part.split(',') {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    *name_counts.entry(trimmed.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    // Get existing mappings
    let mut mapping_stmt = db
        .conn
        .prepare("SELECT abbreviation, full_name FROM passenger_mappings")
        .map_err(|e| e.to_string())?;

    let mappings: std::collections::HashMap<String, String> = mapping_stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<std::collections::HashMap<_, _>, _>>()
        .map_err(|e| e.to_string())?;

    // Combine into result
    let mut result: Vec<PassengerName> = name_counts
        .into_iter()
        .map(|(abbr, count)| PassengerName {
            abbreviation: abbr.clone(),
            full_name: mappings.get(&abbr).cloned(),
            usage_count: count,
        })
        .collect();

    // Sort by usage count descending
    result.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

    Ok(result)
}

#[tauri::command]
pub fn save_passenger_mapping(
    abbreviation: String,
    full_name: String,
    notes: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute(
            "INSERT INTO passenger_mappings (abbreviation, full_name, notes, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(abbreviation) DO UPDATE SET
            full_name = ?2,
            notes = ?3,
            updated_at = datetime('now')",
            rusqlite::params![abbreviation, full_name, notes],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_passenger_mapping(
    abbreviation: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn
        .execute(
            "DELETE FROM passenger_mappings WHERE abbreviation = ?1",
            rusqlite::params![abbreviation],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_all_passenger_mappings(
    state: State<'_, AppState>,
) -> Result<Vec<PassengerMapping>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT abbreviation, full_name, notes FROM passenger_mappings ORDER BY abbreviation ASC"
    ).map_err(|e| e.to_string())?;

    let mappings = stmt
        .query_map([], |row| {
            Ok(PassengerMapping {
                abbreviation: row.get(0)?,
                full_name: row.get(1)?,
                notes: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(mappings)
}

// ===== PASSENGER DETAILS =====

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerRoute {
    pub route: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub flight_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerCompanion {
    pub name: String,
    pub flight_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PassengerDetails {
    pub abbreviation: String,
    pub full_name: Option<String>,
    pub total_flights: i32,
    pub total_distance_km: f64,
    pub top_routes: Vec<PassengerRoute>,
    pub travel_companions: Vec<PassengerCompanion>,
    pub first_flight_date: Option<String>,
    pub last_flight_date: Option<String>,
    // CO2 emissions tracking
    pub total_co2_kg: f64,
    pub avg_co2_per_flight_kg: f64,
    pub carbon_offset_purchased: bool,
}

#[tauri::command]
pub fn get_passenger_details(
    user_id: String,
    abbreviation: String,
    state: State<'_, AppState>,
) -> Result<PassengerDetails, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get full name mapping if it exists
    let full_name: Option<String> = db
        .conn
        .query_row(
            "SELECT full_name FROM passenger_mappings WHERE abbreviation = ?1",
            rusqlite::params![abbreviation],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    // Get flights containing this passenger (including CO2 data)
    let mut flight_stmt = db
        .conn
        .prepare(
            "SELECT id, notes, departure_airport, arrival_airport, departure_datetime, distance_km, carbon_emissions_kg, carbon_offset_purchased
         FROM flights
         WHERE user_id = ?1 AND notes LIKE ?2
         ORDER BY departure_datetime DESC",
        )
        .map_err(|e| e.to_string())?;

    let search_pattern = format!("%{}%", abbreviation);
    let flights: Vec<(String, String, String, String, String, Option<f64>, Option<f64>, Option<i32>)> = flight_stmt
        .query_map(rusqlite::params![user_id, search_pattern], |row| {
            Ok((
                row.get(0)?,  // id
                row.get(1)?,  // notes
                row.get(2)?,  // departure_airport
                row.get(3)?,  // arrival_airport
                row.get(4)?,  // departure_datetime
                row.get(5)?,  // distance_km
                row.get(6)?,  // carbon_emissions_kg
                row.get(7)?,  // carbon_offset_purchased
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Filter to only flights where this passenger is actually listed
    let relevant_flights: Vec<_> = flights
        .into_iter()
        .filter(|(_, notes, _, _, _, _, _, _)| {
            if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
                passenger_part
                    .split(',')
                    .any(|name| name.trim() == abbreviation)
            } else {
                false
            }
        })
        .collect();

    let total_flights = relevant_flights.len() as i32;
    let total_distance_km: f64 = relevant_flights
        .iter()
        .filter_map(|(_, _, _, _, _, distance, _, _)| *distance)
        .sum();

    // Calculate CO2 emissions
    let total_co2_kg: f64 = relevant_flights
        .iter()
        .filter_map(|(_, _, _, _, _, _, co2, _)| *co2)
        .sum();

    let avg_co2_per_flight_kg = if total_flights > 0 {
        total_co2_kg / total_flights as f64
    } else {
        0.0
    };

    let carbon_offset_purchased = relevant_flights
        .iter()
        .any(|(_, _, _, _, _, _, _, offset)| offset.unwrap_or(0) > 0);

    // Get first and last flight dates
    let first_flight_date = relevant_flights
        .last()
        .map(|(_, _, _, _, date, _, _, _)| date.clone());
    let last_flight_date = relevant_flights
        .first()
        .map(|(_, _, _, _, date, _, _, _)| date.clone());

    // Calculate top routes
    let mut route_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for (_, _, dep, arr, _, _, _, _) in &relevant_flights {
        let route = format!("{} → {}", dep, arr);
        *route_counts.entry(route).or_insert(0) += 1;
    }

    let mut top_routes: Vec<PassengerRoute> = route_counts
        .into_iter()
        .map(|(route, count)| {
            let parts: Vec<&str> = route.split(" → ").collect();
            PassengerRoute {
                route: route.clone(),
                departure_airport: parts.first().unwrap_or(&"").to_string(),
                arrival_airport: parts.get(1).unwrap_or(&"").to_string(),
                flight_count: count,
            }
        })
        .collect();
    top_routes.sort_by(|a, b| b.flight_count.cmp(&a.flight_count));
    top_routes.truncate(5); // Top 5 routes

    // Calculate travel companions
    let mut companion_counts: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();
    for (_, notes, _, _, _, _, _, _) in &relevant_flights {
        if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
            let passengers: Vec<&str> = passenger_part.split(',').map(|s| s.trim()).collect();

            // Count other passengers on same flights
            for passenger in passengers {
                if passenger != abbreviation && !passenger.is_empty() {
                    *companion_counts.entry(passenger.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    let mut travel_companions: Vec<PassengerCompanion> = companion_counts
        .into_iter()
        .map(|(name, count)| PassengerCompanion {
            name,
            flight_count: count,
        })
        .collect();
    travel_companions.sort_by(|a, b| b.flight_count.cmp(&a.flight_count));
    travel_companions.truncate(10); // Top 10 companions

    Ok(PassengerDetails {
        abbreviation,
        full_name,
        total_flights,
        total_distance_km,
        top_routes,
        travel_companions,
        first_flight_date,
        last_flight_date,
        total_co2_kg,
        avg_co2_per_flight_kg,
        carbon_offset_purchased,
    })
}

// ===== DEEPSEEK RESEARCH COMMANDS =====

#[derive(Debug, Serialize, Deserialize)]
pub struct ResearchRequest {
    pub flight_id: String,
    pub research_departure: bool,
    pub research_destination: bool,
    pub research_news: bool,
    pub research_events: bool,
    pub research_weather: bool,
    pub research_passengers: bool,
}

#[tauri::command]
pub async fn research_flight_with_deepseek(
    request: ResearchRequest,
    state: State<'_, AppState>,
) -> Result<crate::deepseek::ResearchResult, String> {
    // Get flight details
    let flight = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_flight(&request.flight_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Flight not found".to_string())?
    };

    // Get DeepSeek API key from environment or settings
    let api_key = get_api_key(&["DEEPSEEK_API_KEY"], "deepseek_api_key", &state)?;

    // Extract passenger names from notes
    let passenger_names: Vec<String> = if request.research_passengers {
        if let Some(notes) = &flight.notes {
            if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
                passenger_part
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Determine location(s) to research
    let mut locations = Vec::new();
    if request.research_departure {
        locations.push(flight.departure_airport.clone());
    }
    if request.research_destination {
        locations.push(flight.arrival_airport.clone());
    }

    let location_str = if locations.is_empty() {
        format!("{} to {}", flight.departure_airport, flight.arrival_airport)
    } else {
        locations.join(" and ")
    };

    let date = flight
        .departure_datetime
        .split('T')
        .next()
        .unwrap_or(&flight.departure_datetime)
        .to_string();

    // Perform web searches using DuckDuckGo
    let mut search_results = Vec::new();

    if request.research_news {
        let news_query = format!("{} {} news", location_str, date);
        if let Ok(results) = perform_web_search(&news_query, 5).await {
            search_results.extend(results);
        }
    }

    if request.research_events {
        let events_query = format!("{} {} events conferences", location_str, date);
        if let Ok(results) = perform_web_search(&events_query, 5).await {
            search_results.extend(results);
        }
    }

    if request.research_weather {
        let weather_query = format!("{} {} weather conditions", location_str, date);
        if let Ok(results) = perform_web_search(&weather_query, 3).await {
            search_results.extend(results);
        }
    }

    if request.research_passengers && !passenger_names.is_empty() {
        for passenger in &passenger_names {
            let passenger_query = format!("{} {} {}", passenger, location_str, date);
            if let Ok(results) = perform_web_search(&passenger_query, 3).await {
                search_results.extend(results);
            }
        }
    }

    // If no search results, return empty result
    if search_results.is_empty() {
        return Ok(crate::deepseek::ResearchResult {
            summary: "No web search results found for the specified criteria.".to_string(),
            news_stories: Vec::new(),
            events: Vec::new(),
            weather: None,
            passenger_mentions: Vec::new(),
        });
    }

    // Build DeepSeek research request
    let deepseek_request = crate::deepseek::ResearchRequest {
        date: date.clone(),
        location: location_str,
        passenger_names,
        research_news: request.research_news,
        research_events: request.research_events,
        research_weather: request.research_weather,
        research_passengers: request.research_passengers,
    };

    // Call DeepSeek to analyze search results
    let result =
        crate::deepseek::research_flight_context(deepseek_request, search_results, &api_key)
            .await
            .map_err(|e| format!("DeepSeek research failed: {}", e))?;

    Ok(result)
}

/// Perform web search using DuckDuckGo
async fn perform_web_search(query: &str, max_results: usize) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

    let response = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let html = response.text().await.map_err(|e| e.to_string())?;

    // Parse search results (basic extraction)
    let mut results = Vec::new();

    // Compile regex for HTML tag removal once (used in both loops)
    let html_tag_regex = regex::Regex::new(r"<[^>]+>").unwrap();

    // Simple regex-based extraction of result snippets
    let re = regex::Regex::new(r#"<a class="result__snippet"[^>]*>(.*?)</a>"#)
        .map_err(|e| e.to_string())?;

    for cap in re.captures_iter(&html).take(max_results) {
        if let Some(snippet) = cap.get(1) {
            let text = snippet.as_str();
            // Unescape HTML entities
            let unescaped = html_escape::decode_html_entities(text).to_string();
            // Remove HTML tags
            let clean = html_tag_regex
                .replace_all(&unescaped, "")
                .to_string();

            if !clean.trim().is_empty() {
                results.push(clean.trim().to_string());
            }
        }
    }

    // Also try to extract titles
    let title_re =
        regex::Regex::new(r#"<a class="result__a"[^>]*>(.*?)</a>"#).map_err(|e| e.to_string())?;

    for cap in title_re.captures_iter(&html).take(max_results) {
        if let Some(title) = cap.get(1) {
            let text = title.as_str();
            let unescaped = html_escape::decode_html_entities(text).to_string();
            let clean = html_tag_regex
                .replace_all(&unescaped, "")
                .to_string();

            if !clean.trim().is_empty() {
                results.push(format!("Title: {}", clean.trim()));
            }
        }
    }

    Ok(results)
}

// ===== GROK AI RESEARCH =====

// Helper function to get API key from env or database
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
pub async fn research_flight_with_grok(
    flight_id: String,
    research_topics: Vec<String>,
    model_name: String, // "grok-4-fast-non-reasoning", "grok-4-fast-reasoning", or "grok-code-fast-1"
    custom_query: Option<String>, // Optional custom query for freeform chat
    state: State<'_, AppState>,
) -> Result<crate::grok::GrokAnalysisResult, String> {
    // Validate model name
    let valid_models = [
        "grok-4-fast-non-reasoning",
        "grok-4-fast-reasoning",
        "grok-code-fast-1",
    ];
    if !valid_models.contains(&model_name.as_str()) {
        return Err(format!(
            "Invalid model name. Must be one of: {:?}",
            valid_models
        ));
    }

    // Get Grok API key from environment or settings (XAI_API_KEY or GROK_API_KEY)
    let api_key = get_api_key(&["XAI_API_KEY", "GROK_API_KEY"], "grok_api_key", &state)?;

    // If custom query is provided, use direct chat mode
    if let Some(query) = custom_query {
        return crate::grok::chat_with_grok(&query, &api_key, &model_name)
            .await
            .map_err(|e| format!("Grok chat failed: {}", e));
    }

    // Get flight details
    let flight = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_flight(&flight_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Flight not found".to_string())?
    };

    let flight_route = format!("{} to {}", flight.departure_airport, flight.arrival_airport);
    let flight_date = flight
        .departure_datetime
        .split('T')
        .next()
        .unwrap_or(&flight.departure_datetime)
        .to_string();

    // Extract passenger names from notes
    let passenger_names: Vec<String> = if let Some(notes) = &flight.notes {
        if let Some(passenger_line) = notes.lines().find(|line| line.starts_with("Passengers:")) {
            passenger_line
                .trim_start_matches("Passengers:")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Perform web searches based on research topics
    let mut search_results = Vec::new();

    for topic in &research_topics {
        let query = match topic.as_str() {
            "news" => format!("{} {} news", flight_route, flight_date),
            "events" => format!("{} {} events", flight_route, flight_date),
            "weather" => format!("{} {} weather", flight_route, flight_date),
            "aviation" => format!("{} flight status aviation incidents", flight_route),
            _ => continue,
        };

        if let Ok(results) = perform_web_search(&query, 3).await {
            search_results.extend(results);
        }
    }

    // Call Grok for analysis
    crate::grok::analyze_flight_with_grok(
        &flight_route,
        &flight_date,
        passenger_names,
        research_topics,
        search_results,
        &api_key,
        &model_name,
    )
    .await
    .map_err(|e| format!("Grok analysis failed: {}", e))
}

#[tauri::command]
pub async fn multi_provider_flight_research(
    flight_id: String,
    research_topics: Vec<String>,
    state: State<'_, AppState>,
) -> Result<crate::grok::MultiProviderAnalysis, String> {
    // Get API keys from environment or settings
    let grok_api_key = get_api_key(&["XAI_API_KEY", "GROK_API_KEY"], "grok_api_key", &state).ok();
    let deepseek_api_key = get_api_key(&["DEEPSEEK_API_KEY"], "deepseek_api_key", &state).ok();

    if grok_api_key.is_none() && deepseek_api_key.is_none() {
        return Err(
            "At least one AI provider API key must be configured (Grok or DeepSeek)".to_string(),
        );
    }

    // Get flight details
    let flight = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_flight(&flight_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Flight not found".to_string())?
    };

    let flight_route = format!("{} to {}", flight.departure_airport, flight.arrival_airport);
    let flight_date = flight
        .departure_datetime
        .split('T')
        .next()
        .unwrap_or(&flight.departure_datetime)
        .to_string();

    // Extract passenger names
    let passenger_names: Vec<String> = if let Some(notes) = &flight.notes {
        if let Some(passenger_line) = notes.lines().find(|line| line.starts_with("Passengers:")) {
            passenger_line
                .trim_start_matches("Passengers:")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Perform comprehensive web searches
    let mut search_results = Vec::new();
    for topic in &research_topics {
        let query = match topic.as_str() {
            "news" => format!("{} {} news", flight_route, flight_date),
            "events" => format!("{} {} events", flight_route, flight_date),
            "weather" => format!("{} {} weather", flight_route, flight_date),
            "aviation" => format!("{} flight aviation", flight_route),
            _ => continue,
        };
        if let Ok(results) = perform_web_search(&query, 5).await {
            search_results.extend(results);
        }
    }

    // Call multi-provider analysis
    crate::grok::multi_provider_analysis(
        &flight_route,
        &flight_date,
        passenger_names,
        research_topics,
        search_results,
        grok_api_key.as_deref(),
        deepseek_api_key.as_deref(),
    )
    .await
    .map_err(|e| format!("Multi-provider analysis failed: {}", e))
}

// ===== GEMINI CHAT =====

#[tauri::command]
pub async fn chat_with_gemini(
    query: String,
    model: String, // "gemini-2.5-pro", "gemini-2.5-flash", or "gemini-2.5-flash-lite"
    state: State<'_, AppState>,
) -> Result<crate::gemini::GeminiChatResult, String> {
    // Get Gemini API key from environment or settings
    let api_key = get_api_key(
        &["GENAI_API_KEY", "GEMINI_API_KEY"],
        "gemini_api_key",
        &state,
    )?;

    // Call Gemini chat
    crate::gemini::chat_with_gemini(&query, &api_key, &model)
        .await
        .map_err(|e| format!("Gemini chat failed: {}", e))
}

// ===== DEEPSEEK CHAT =====

#[tauri::command]
pub async fn chat_with_deepseek(
    query: String,
    state: State<'_, AppState>,
) -> Result<crate::deepseek::DeepSeekChatResult, String> {
    // Get DeepSeek API key from environment or settings
    let api_key = get_api_key(&["DEEPSEEK_API_KEY"], "deepseek_api_key", &state)?;

    // Call DeepSeek chat
    crate::deepseek::chat_with_deepseek(&query, &api_key)
        .await
        .map_err(|e| format!("DeepSeek chat failed: {}", e))
}

// ===== RESEARCH REPORTS =====

#[tauri::command]
pub fn save_research_report(
    user_id: String,
    report: crate::models::ResearchReportInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_research_report(&user_id, &report)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_research_report(
    report_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::ResearchReport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_research_report(&report_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_research_reports(
    user_id: String,
    limit: i64,
    offset: i64,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::ResearchReport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_research_reports(&user_id, limit, offset)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_research_report(report_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_research_report(&report_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn count_research_reports(user_id: String, state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.count_research_reports(&user_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_research_report_to_markdown(
    report_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use std::fs;

    use chrono::{DateTime, Utc};

    // Get the report from database
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let report = db
        .get_research_report(&report_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Report not found".to_string())?;

    // Parse the created_at timestamp for directory naming
    let created_at_parsed = DateTime::parse_from_rfc3339(&report.created_at)
        .or_else(|_| {
            // Fallback: try parsing as SQLite datetime format
            chrono::NaiveDateTime::parse_from_str(&report.created_at, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc).fixed_offset())
        })
        .map_err(|e| format!("Failed to parse timestamp: {}", e))?;

    let timestamp = created_at_parsed.format("%Y%m%d-%H%M%S").to_string();

    // Create base directory path: ~/flight-tracker-pro/researcher/
    let home_dir = dirs::home_dir().ok_or_else(|| "Failed to get home directory".to_string())?;
    let base_path = home_dir.join("flight-tracker-pro").join("researcher");

    // Create agent-specific directory with timestamp
    let agent_dir = base_path.join(format!(
        "{}-{}",
        report.agent_name.to_lowercase(),
        timestamp
    ));

    // Create directories if they don't exist
    fs::create_dir_all(&agent_dir).map_err(|e| format!("Failed to create directory: {}", e))?;

    // Generate filename from search query (sanitized)
    let filename = sanitize_filename(&report.search_query);
    let markdown_filename = format!("{}.md", filename);
    let file_path = agent_dir.join(&markdown_filename);

    // Build markdown content
    let mut markdown = String::new();

    // Header
    markdown.push_str(&format!("# Research Report: {}\n\n", report.search_query));
    markdown.push_str(&format!("**Agent:** {}\n", report.agent_name));
    if let Some(model) = &report.agent_model {
        markdown.push_str(&format!("**Model:** {}\n", model));
    }
    markdown.push_str(&format!("**Date:** {}\n", report.created_at));
    markdown.push_str(&format!("**Report Type:** {}\n", report.report_type));
    if let Some(confidence) = report.confidence_score {
        markdown.push_str(&format!("**Confidence Score:** {:.2}\n", confidence));
    }
    if let Some(processing_time) = report.processing_time_ms {
        markdown.push_str(&format!("**Processing Time:** {}ms\n", processing_time));
    }
    if let Some(flight_id) = &report.flight_id {
        markdown.push_str(&format!("**Flight ID:** {}\n", flight_id));
    }
    markdown.push_str("\n---\n\n");

    // Summary
    markdown.push_str("## Summary\n\n");
    markdown.push_str(&report.report_summary);
    markdown.push_str("\n\n");

    // Details
    if let Some(details) = &report.report_details {
        markdown.push_str("## Detailed Analysis\n\n");
        markdown.push_str(details);
        markdown.push_str("\n\n");
    }

    // Research Topics
    if let Some(topics_json) = &report.research_topics {
        if let Ok(topics) = serde_json::from_str::<Vec<String>>(topics_json) {
            markdown.push_str("## Research Topics\n\n");
            for topic in topics {
                markdown.push_str(&format!("- {}\n", topic));
            }
            markdown.push('\n');
        }
    }

    // Sources
    if let Some(sources_json) = &report.sources {
        if let Ok(sources) =
            serde_json::from_str::<Vec<crate::models::ResearchSource>>(sources_json)
        {
            markdown.push_str("## Sources\n\n");
            for (idx, source) in sources.iter().enumerate() {
                markdown.push_str(&format!("### {}. {}\n\n", idx + 1, source.title));
                if let Some(url) = &source.url {
                    markdown.push_str(&format!("**URL:** {}\n\n", url));
                }
                markdown.push_str(&format!("{}\n\n", source.snippet));
            }
        }
    }

    // Footer
    markdown.push_str("---\n\n");
    markdown.push_str("*Generated by Flight Tracker Pro*\n");

    // Write to file
    fs::write(&file_path, markdown).map_err(|e| format!("Failed to write markdown file: {}", e))?;

    // Return the full path
    Ok(file_path.to_string_lossy().to_string())
}

/// Sanitize a string to be used as a filename
fn sanitize_filename(s: &str) -> String {
    let mut sanitized = s.to_lowercase();

    // Replace spaces and special characters with hyphens
    sanitized = sanitized
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect();

    // Remove consecutive hyphens/underscores
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }

    // Trim hyphens and underscores from start/end
    sanitized = sanitized.trim_matches(|c| c == '-' || c == '_').to_string();

    // Limit length to 50 characters
    if sanitized.len() > 50 {
        sanitized.truncate(50);
        sanitized = sanitized.trim_matches(|c| c == '-' || c == '_').to_string();
    }

    // Fallback if empty
    if sanitized.is_empty() {
        sanitized = "report".to_string();
    }

    sanitized
}

// ===== DISTANCE & CO2 CALCULATION COMMANDS =====

use crate::geo;

#[derive(Debug, Serialize, Deserialize)]
pub struct DistanceResult {
    pub distance_nm: f64,
    pub distance_km: f64,
}

/// Calculate distance between two airports using IATA codes
#[tauri::command]
pub fn calculate_distance(from_airport: String, to_airport: String) -> Result<DistanceResult, String> {
    let (nm, km) = geo::calculate_airport_distance(&from_airport, &to_airport)
        .ok_or_else(|| {
            format!(
                "Could not find coordinates for airports: {} or {}",
                from_airport, to_airport
            )
        })?;

    Ok(DistanceResult {
        distance_nm: nm,
        distance_km: km,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CO2Result {
    pub co2_kg: f64,
    pub co2_tonnes: f64,
}

/// Calculate CO2 emissions for a flight
#[tauri::command]
pub fn calculate_co2_emissions(
    distance_km: f64,
    aircraft_type: Option<String>,
) -> Result<CO2Result, String> {
    let co2_kg =
        crate::calculations::calculate_co2_emissions(distance_km, aircraft_type.as_deref());

    Ok(CO2Result {
        co2_kg,
        co2_tonnes: co2_kg / 1000.0,
    })
}

/// Calculate flight duration in minutes
#[tauri::command]
pub fn calculate_flight_duration(
    distance_km: f64,
    aircraft_type: Option<String>,
) -> Result<i32, String> {
    let duration =
        crate::calculations::calculate_flight_time(distance_km, aircraft_type.as_deref());
    Ok(duration)
}

/// Calculate per-passenger CO2 emissions
#[tauri::command]
pub fn calculate_per_passenger_co2(
    distance_km: f64,
    aircraft_type: Option<String>,
    passenger_count: Option<u32>,
) -> Result<f64, String> {
    let co2 = crate::calculations::calculate_per_passenger_co2(
        distance_km,
        aircraft_type.as_deref(),
        passenger_count,
    );
    Ok(co2)
}

/// Convert fuel from kilograms to liters
#[tauri::command]
pub fn fuel_kg_to_liters(fuel_kg: f64) -> Result<f64, String> {
    Ok(crate::calculations::fuel_kg_to_liters(fuel_kg))
}

/// Convert fuel from kilograms to US gallons
#[tauri::command]
pub fn fuel_kg_to_gallons(fuel_kg: f64) -> Result<f64, String> {
    Ok(crate::calculations::fuel_kg_to_gallons(fuel_kg))
}

// ===== JOURNEY COMMANDS =====

#[tauri::command]
pub fn create_journey(
    user_id: String,
    name: String,
    description: Option<String>,
    start_date: String,
    end_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_journey(&user_id, &name, description.as_deref(), &start_date, end_date.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_journey(
    journey_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::Journey>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_journey(&journey_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_user_journeys(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Journey>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_user_journeys(&user_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_journey(
    journey_id: String,
    name: String,
    description: Option<String>,
    start_date: String,
    end_date: Option<String>,
    is_favorite: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_journey(
        &journey_id,
        &name,
        description.as_deref(),
        &start_date,
        end_date.as_deref(),
        is_favorite,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_journey(journey_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_journey(&journey_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_flight_to_journey(
    journey_id: String,
    flight_id: String,
    sequence_order: i32,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.add_flight_to_journey(&journey_id, &flight_id, sequence_order)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_flight_from_journey(
    journey_id: String,
    flight_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.remove_flight_from_journey(&journey_id, &flight_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_journey_flights(
    journey_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Flight>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_journey_flights(&journey_id)
        .map_err(|e| e.to_string())
}

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

// ===== AIRPORT COMMANDS =====

#[tauri::command]
pub fn create_airport(
    airport: crate::models::AirportInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.create_airport(
        airport.icao_code.as_deref(),
        airport.iata_code.as_deref(),
        &airport.name,
        airport.city.as_deref(),
        airport.country.as_deref(),
        airport.latitude,
        airport.longitude,
        airport.timezone.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_airport(
    airport_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::Airport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_airport(&airport_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_all_airports(
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::Airport>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_all_airports().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_airport(
    airport_id: String,
    airport: crate::models::AirportInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_airport(
        &airport_id,
        airport.icao_code.as_deref(),
        airport.iata_code.as_deref(),
        &airport.name,
        airport.city.as_deref(),
        airport.country.as_deref(),
        airport.latitude,
        airport.longitude,
        airport.timezone.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_airport(airport_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_airport(&airport_id).map_err(|e| e.to_string())
}

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

// ===== INITIALIZATION COMMAND =====

#[tauri::command]
pub fn initialize_app(state: State<'_, AppState>) -> Result<bool, String> {
    // Check if a user exists
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = db.get_primary_user().map_err(|e| e.to_string())?;

    Ok(user.is_some())
}
