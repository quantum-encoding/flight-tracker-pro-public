// CSV import commands
use tauri::{State, Manager};
use super::AppState;
use crate::models::FlightInput;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

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
    /// Warnings that might need user attention (not errors - all rows import)
    pub validation_warnings: Vec<String>,
    pub raw_values: Vec<String>,
    /// Source page number for manual verification (extracted from notes or inferred)
    pub source_page: Option<u32>,
    /// Flag indicating this row needs user review (has warnings)
    pub needs_review: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CsvPreviewResult {
    pub headers: Vec<String>,
    pub detected_mapping: CsvColumnMapping,
    pub preview_rows: Vec<CsvPreviewRow>,
    /// All rows with validation info (for filtering in UI)
    pub all_rows: Vec<CsvPreviewRow>,
    pub total_rows: usize,
    /// Rows that will import cleanly
    pub clean_rows: usize,
    /// Rows that need user review (have warnings but will still import)
    pub review_rows: usize,
}

#[tauri::command]
pub fn preview_csv_import(
    csv_path: String,
    max_preview_rows: Option<usize>,
) -> Result<CsvPreviewResult, String> {
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

    let max_rows = max_preview_rows.unwrap_or(20);
    let mut all_rows = Vec::new();
    let mut total_rows = 0;
    let mut clean_rows = 0;
    let mut review_rows = 0;

    for (idx, result) in reader.records().enumerate() {
        total_rows += 1;

        match result {
            Ok(record) => {
                let raw_values: Vec<String> = record.iter().map(|s| s.to_string()).collect();

                let preview_row = parse_csv_row_preview(
                    idx + 2, // +2 for header and 0-indexing
                    &record,
                    &detected_mapping,
                    &raw_values,
                );

                if preview_row.needs_review {
                    review_rows += 1;
                } else {
                    clean_rows += 1;
                }

                all_rows.push(preview_row);
            }
            Err(e) => {
                review_rows += 1;
                // Add row with warning for tracking - still imports
                all_rows.push(CsvPreviewRow {
                    row_number: idx + 2,
                    date: String::new(),
                    departure_airport: String::new(),
                    arrival_airport: String::new(),
                    passengers: String::new(),
                    flight_number: None,
                    aircraft_registration: None,
                    parsed_date: None,
                    distance_nm: None,
                    validation_warnings: vec![format!("CSV parse issue: {}", e)],
                    raw_values: vec![],
                    source_page: None,
                    needs_review: true,
                });
            }
        }
    }

    // Take first N rows for preview display
    let preview_rows: Vec<CsvPreviewRow> = all_rows.iter().take(max_rows).cloned().collect();

    Ok(CsvPreviewResult {
        headers,
        detected_mapping,
        preview_rows,
        all_rows,
        total_rows,
        clean_rows,
        review_rows,
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
    let mut validation_warnings = Vec::new();
    let mut needs_review = false;

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

    // Parse date - warn but don't reject
    let parsed_date = if date.is_empty() {
        validation_warnings.push("Date is empty".to_string());
        needs_review = true;
        None
    } else if !is_valid_date_field(&date) {
        validation_warnings.push("Date format unusual".to_string());
        needs_review = true;
        None
    } else {
        match parse_date(&date) {
            Some(d) => Some(d),
            None => {
                validation_warnings.push("Date could not be parsed".to_string());
                needs_review = true;
                None
            }
        }
    };

    // Check airports - warn but accept anything
    if departure_airport.is_empty() {
        validation_warnings.push("From field is empty".to_string());
        needs_review = true;
    } else if departure_airport.len() > 8 {
        // Unusually long - might be misplaced data
        validation_warnings.push("From field looks like it may contain other data".to_string());
        needs_review = true;
    } else if !departure_airport.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        validation_warnings.push("From field contains unusual characters".to_string());
        needs_review = true;
    }

    if arrival_airport.is_empty() {
        validation_warnings.push("To field is empty".to_string());
        needs_review = true;
    } else if arrival_airport.len() > 8 {
        validation_warnings.push("To field looks like it may contain other data".to_string());
        needs_review = true;
    } else if !arrival_airport.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        validation_warnings.push("To field contains unusual characters".to_string());
        needs_review = true;
    }

    // Calculate distance - will work for recognized airports
    let distance_nm = crate::geo::calculate_airport_distance(&departure_airport, &arrival_airport)
        .map(|(nm, _)| nm);

    // Try to infer source page from row number (roughly 20 rows per page)
    let source_page = Some(((row_number - 2) / 20 + 1) as u32);

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
        validation_warnings,
        raw_values: raw_values.to_vec(),
        source_page,
        needs_review,
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

                // Build notes with all raw data for user review
                let mut notes_parts = Vec::new();
                if !passengers.is_empty() {
                    notes_parts.push(format!("Passengers: {}", passengers));
                }

                // Try to parse date - use placeholder if fails
                let departure_datetime = if date.is_empty() || !is_valid_date_field(&date) {
                    notes_parts.push(format!("Original date: {}", date));
                    "1900-01-01T00:00:00".to_string() // Placeholder for unparseable dates
                } else {
                    match parse_date(&date) {
                        Some(dt) => dt,
                        None => {
                            notes_parts.push(format!("Original date: {}", date));
                            "1900-01-01T00:00:00".to_string()
                        }
                    }
                };

                // Use empty string fallback for missing airports - import anyway
                let departure_airport = if departure.is_empty() { "???".to_string() } else { departure };
                let arrival_airport = if arrival.is_empty() { "???".to_string() } else { arrival };

                // Add warning if airport codes look unusual
                if departure_airport.len() > 4 || arrival_airport.len() > 4 {
                    notes_parts.push(format!("Review: From={}, To={}", departure_airport, arrival_airport));
                }

                // Calculate distance - will work for recognized airports
                let distance_nm =
                    crate::geo::calculate_airport_distance(&departure_airport, &arrival_airport).map(|(nm, _)| nm);

                let notes = if notes_parts.is_empty() { None } else { Some(notes_parts.join(" | ")) };

                let flight = FlightInput {
                    flight_number,
                    departure_airport,
                    arrival_airport,
                    departure_datetime,
                    arrival_datetime: None,
                    aircraft_type_id: None,
                    aircraft_registration,
                    total_duration: None,
                    flight_duration: None,
                    distance_nm,
                    distance_km: None,
                    carbon_emissions_kg: None,
                    booking_reference: None,
                    ticket_number: None,
                    seat_number: None,
                    fare_class: None,
                    base_fare: None,
                    taxes: None,
                    total_cost: None,
                    currency: Some("USD".to_string()),
                    notes,
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

/// Result for batched preload operations
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchPreloadResult {
    pub batch_number: usize,
    pub total_batches: usize,
    pub imported_this_batch: usize,
    pub total_imported: usize,
    pub total_rows: usize,
    pub is_complete: bool,
    pub errors: Vec<String>,
}

/// Preload test data in batches to avoid freezing the UI
/// Call repeatedly with increasing batch_number until is_complete is true
#[tauri::command]
pub fn preload_test_data_batch(
    user_id: String,
    batch_number: usize,
    batch_size: usize,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<BatchPreloadResult, String> {
    // Resolve the resource path for the bundled CSV
    let resource_path = app_handle
        .path()
        .resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?
        .join("resources")
        .join("flight_log.csv");

    let csv_path = resource_path.to_string_lossy().to_string();
    let file = File::open(&csv_path).map_err(|e| format!("Failed to open CSV file at {}: {}", csv_path, e))?;

    let mut reader = ::csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(BufReader::new(file));

    // Count total rows first (need to reopen file)
    let total_rows = {
        let count_file = File::open(&csv_path).map_err(|e| format!("Failed to open CSV: {}", e))?;
        let count_reader = ::csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_reader(BufReader::new(count_file));
        count_reader.into_records().count()
    };

    let total_batches = (total_rows + batch_size - 1) / batch_size;
    let start_idx = batch_number * batch_size;
    let end_idx = std::cmp::min(start_idx + batch_size, total_rows);

    let mut imported_this_batch = 0;
    let mut errors = Vec::new();

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Detect column mapping from headers
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("Failed to read headers: {}", e))?
        .iter()
        .map(|h| h.to_string())
        .collect();
    let mapping = detect_column_mapping(&headers);

    for (idx, result) in reader.records().enumerate() {
        // Skip rows before our batch
        if idx < start_idx {
            continue;
        }
        // Stop after our batch
        if idx >= end_idx {
            break;
        }

        let row_number = idx + 2;

        match result {
            Ok(record) => {
                let date = record
                    .get(mapping.date_column)
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let departure = record
                    .get(mapping.from_column)
                    .unwrap_or("")
                    .trim()
                    .to_uppercase();
                let arrival = record
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

                // Build notes with all raw data for user review
                let mut notes_parts = Vec::new();
                if !passengers.is_empty() {
                    notes_parts.push(format!("Passengers: {}", passengers));
                }

                // Try to parse date - use placeholder if fails
                let departure_datetime = if date.is_empty() || !is_valid_date_field(&date) {
                    notes_parts.push(format!("Original date: {}", date));
                    "1900-01-01T00:00:00".to_string()
                } else {
                    match parse_date(&date) {
                        Some(dt) => dt,
                        None => {
                            notes_parts.push(format!("Original date: {}", date));
                            "1900-01-01T00:00:00".to_string()
                        }
                    }
                };

                // Use fallback for missing airports - import anyway
                let departure_airport = if departure.is_empty() { "???".to_string() } else { departure };
                let arrival_airport = if arrival.is_empty() { "???".to_string() } else { arrival };

                // Add warning if airport codes look unusual
                if departure_airport.len() > 4 || arrival_airport.len() > 4 {
                    notes_parts.push(format!("Review: From={}, To={}", departure_airport, arrival_airport));
                }

                // Calculate distance - will work for recognized airports
                let distance_nm =
                    crate::geo::calculate_airport_distance(&departure_airport, &arrival_airport).map(|(nm, _)| nm);

                let notes = if notes_parts.is_empty() { None } else { Some(notes_parts.join(" | ")) };

                let flight = FlightInput {
                    flight_number,
                    departure_airport,
                    arrival_airport,
                    departure_datetime,
                    arrival_datetime: None,
                    aircraft_type_id: None,
                    aircraft_registration,
                    total_duration: None,
                    flight_duration: None,
                    distance_nm,
                    distance_km: None,
                    carbon_emissions_kg: None,
                    booking_reference: None,
                    ticket_number: None,
                    seat_number: None,
                    fare_class: None,
                    base_fare: None,
                    taxes: None,
                    total_cost: None,
                    currency: Some("USD".to_string()),
                    notes,
                    attachment_path: None,
                };

                match db.create_flight(&user_id, &flight) {
                    Ok(_) => imported_this_batch += 1,
                    Err(e) => {
                        errors.push(format!("Row {}: {}", row_number, e));
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Row {}: CSV parse error: {}", row_number, e));
            }
        }
    }

    let total_imported = batch_number * batch_size + imported_this_batch;
    let is_complete = batch_number + 1 >= total_batches;

    Ok(BatchPreloadResult {
        batch_number,
        total_batches,
        imported_this_batch,
        total_imported,
        total_rows,
        is_complete,
        errors,
    })
}
