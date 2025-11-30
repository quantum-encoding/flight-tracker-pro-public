// Airport data enrichment from OurAirports dataset
// Provides search and lookup from bundled CSV, plus fuzzy matching

use tauri::{State, Manager};
use super::AppState;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use rusqlite::OptionalExtension;
use std::path::PathBuf;
use std::sync::{OnceLock, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportData {
    pub ident: String,
    pub iata_code: Option<String>,
    pub name: String,
    pub latitude_deg: Option<f64>,
    pub longitude_deg: Option<f64>,
    pub municipality: Option<String>,
    pub iso_country: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnrichmentResult {
    pub total_processed: usize,
    pub enriched_count: usize,
    pub new_airports: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportSearchResult {
    pub airport: AirportData,
    pub match_type: String,        // "exact_icao", "exact_iata", "fuzzy", "name_match"
    pub similarity_score: f32,     // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportLookupResponse {
    pub exact_match: Option<AirportData>,
    pub suggestions: Vec<AirportSearchResult>,
}

// Global cache for loaded airport data (lazy-loaded once)
static AIRPORT_CACHE: OnceLock<Mutex<Option<AirportCache>>> = OnceLock::new();

struct AirportCache {
    airports: Vec<AirportData>,
    icao_index: HashMap<String, usize>,  // ICAO -> index
    iata_index: HashMap<String, usize>,  // IATA -> index
}

impl AirportCache {
    fn new(airports: Vec<AirportData>) -> Self {
        let mut icao_index = HashMap::new();
        let mut iata_index = HashMap::new();

        for (idx, airport) in airports.iter().enumerate() {
            // Index by ICAO (ident)
            if !airport.ident.is_empty() {
                icao_index.insert(airport.ident.to_uppercase(), idx);
            }
            // Index by IATA
            if let Some(ref iata) = airport.iata_code {
                if !iata.is_empty() {
                    iata_index.insert(iata.to_uppercase(), idx);
                }
            }
        }

        AirportCache { airports, icao_index, iata_index }
    }

    fn lookup_exact(&self, code: &str) -> Option<&AirportData> {
        let code_upper = code.to_uppercase();

        // Try ICAO first (4 chars)
        if let Some(&idx) = self.icao_index.get(&code_upper) {
            return Some(&self.airports[idx]);
        }

        // Try IATA (3 chars)
        if let Some(&idx) = self.iata_index.get(&code_upper) {
            return Some(&self.airports[idx]);
        }

        None
    }

    fn search_fuzzy(&self, query: &str, limit: usize) -> Vec<AirportSearchResult> {
        let query_upper = query.to_uppercase();
        let query_lower = query.to_lowercase();
        let mut results: Vec<AirportSearchResult> = Vec::new();

        for airport in &self.airports {
            let mut best_score: f32 = 0.0;
            let mut match_type = "fuzzy";

            // Check ICAO similarity
            let icao_sim = Self::levenshtein_similarity(&airport.ident.to_uppercase(), &query_upper);
            if icao_sim > best_score {
                best_score = icao_sim;
                match_type = if icao_sim == 1.0 { "exact_icao" } else { "fuzzy_icao" };
            }

            // Check IATA similarity
            if let Some(ref iata) = airport.iata_code {
                let iata_sim = Self::levenshtein_similarity(&iata.to_uppercase(), &query_upper);
                if iata_sim > best_score {
                    best_score = iata_sim;
                    match_type = if iata_sim == 1.0 { "exact_iata" } else { "fuzzy_iata" };
                }
            }

            // Check name contains (boost if starts with query)
            let name_lower = airport.name.to_lowercase();
            if name_lower.contains(&query_lower) {
                let name_score = if name_lower.starts_with(&query_lower) { 0.9 } else { 0.7 };
                if name_score > best_score {
                    best_score = name_score;
                    match_type = "name_match";
                }
            }

            // Check city/municipality
            if let Some(ref city) = airport.municipality {
                let city_lower = city.to_lowercase();
                if city_lower.contains(&query_lower) {
                    let city_score = if city_lower.starts_with(&query_lower) { 0.85 } else { 0.65 };
                    if city_score > best_score {
                        best_score = city_score;
                        match_type = "city_match";
                    }
                }
            }

            // Only include if reasonable match
            if best_score >= 0.4 {
                results.push(AirportSearchResult {
                    airport: airport.clone(),
                    match_type: match_type.to_string(),
                    similarity_score: best_score,
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(limit);

        results
    }

    // Levenshtein distance based similarity (0.0 - 1.0)
    fn levenshtein_similarity(a: &str, b: &str) -> f32 {
        if a == b { return 1.0; }
        if a.is_empty() || b.is_empty() { return 0.0; }

        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let len_a = a_chars.len();
        let len_b = b_chars.len();

        // Quick rejection for very different lengths
        let max_len = len_a.max(len_b);
        let len_diff = (len_a as i32 - len_b as i32).abs() as usize;
        if len_diff > max_len / 2 { return 0.0; }

        let mut prev_row: Vec<usize> = (0..=len_b).collect();
        let mut curr_row: Vec<usize> = vec![0; len_b + 1];

        for i in 1..=len_a {
            curr_row[0] = i;
            for j in 1..=len_b {
                let cost = if a_chars[i-1] == b_chars[j-1] { 0 } else { 1 };
                curr_row[j] = (prev_row[j] + 1)
                    .min(curr_row[j-1] + 1)
                    .min(prev_row[j-1] + cost);
            }
            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        let distance = prev_row[len_b];
        1.0 - (distance as f32 / max_len as f32)
    }
}

/// Load airport data from local CSV file (OurAirports format)
fn load_airports_from_csv(csv_path: &PathBuf) -> Result<Vec<AirportData>> {
    let mut airports = Vec::new();
    let mut reader = csv::Reader::from_path(csv_path)?;

    for result in reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        // CSV format: id,ident,type,name,latitude_deg,longitude_deg,elevation_ft,continent,
        // iso_country,iso_region,municipality,scheduled_service,icao_code,iata_code,gps_code,local_code,home_link,wikipedia_link,keywords
        let airport_type = record.get(2).unwrap_or("");

        // Include all airport types that have runways (exclude heliports, seaplane bases, balloonports, closed)
        let valid_types = ["large_airport", "medium_airport", "small_airport"];
        if !valid_types.contains(&airport_type) {
            continue;
        }

        let ident = record.get(1).unwrap_or("").trim().to_string();

        // iata_code is at index 13
        let iata_code = record.get(13).and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
        });

        // Skip if no identifier at all
        if ident.is_empty() && iata_code.is_none() {
            continue;
        }

        let latitude_deg = record.get(4)
            .and_then(|s| s.trim().parse::<f64>().ok());

        let longitude_deg = record.get(5)
            .and_then(|s| s.trim().parse::<f64>().ok());

        // Skip if no coordinates
        if latitude_deg.is_none() || longitude_deg.is_none() {
            continue;
        }

        airports.push(AirportData {
            ident,
            iata_code,
            name: record.get(3).unwrap_or("").to_string(),
            latitude_deg,
            longitude_deg,
            municipality: record.get(10).map(|s| s.to_string()),
            iso_country: record.get(8).map(|s| s.to_string()),
        });
    }

    Ok(airports)
}

/// Fetch airport data from OurAirports CSV (fallback for web fetch)
#[allow(dead_code)]
async fn fetch_ourairports_data() -> Result<Vec<AirportData>> {
    const OURAIRPORTS_CSV_URL: &str = "https://davidmegginson.github.io/ourairports-data/airports.csv";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;

    let response = client.get(OURAIRPORTS_CSV_URL).send().await?;
    let text = response.text().await?;

    let mut airports = Vec::new();
    let mut reader = csv::Reader::from_reader(text.as_bytes());

    for result in reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        let airport_type = record.get(2).unwrap_or("");
        let valid_types = ["large_airport", "medium_airport", "small_airport"];
        if !valid_types.contains(&airport_type) {
            continue;
        }

        let ident = record.get(1).unwrap_or("").trim().to_string();
        let iata_code = record.get(13).and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
        });

        if ident.is_empty() && iata_code.is_none() {
            continue;
        }

        let latitude_deg = record.get(4).and_then(|s| s.trim().parse::<f64>().ok());
        let longitude_deg = record.get(5).and_then(|s| s.trim().parse::<f64>().ok());

        if latitude_deg.is_none() || longitude_deg.is_none() {
            continue;
        }

        airports.push(AirportData {
            ident,
            iata_code,
            name: record.get(3).unwrap_or("").to_string(),
            latitude_deg,
            longitude_deg,
            municipality: record.get(10).map(|s| s.to_string()),
            iso_country: record.get(8).map(|s| s.to_string()),
        });
    }

    Ok(airports)
}

#[tauri::command]
pub async fn enrich_airport_data(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<EnrichmentResult, String> {
    let mut result = EnrichmentResult {
        total_processed: 0,
        enriched_count: 0,
        new_airports: 0,
        errors: Vec::new(),
    };

    // Try to load from local CSV file first using fallback paths
    let airports = match find_airports_csv(&app_handle) {
        Ok(resource_path) => {
            load_airports_from_csv(&resource_path)
                .map_err(|e| format!("Failed to load local airports.csv: {}", e))?
        }
        Err(_) => {
            // Fallback to web fetch if local file doesn't exist
            fetch_ourairports_data()
                .await
                .map_err(|e| format!("Failed to fetch OurAirports data: {}", e))?
        }
    };

    result.total_processed = airports.len();

    let db = state.db.lock().map_err(|e| e.to_string())?;

    for airport in airports {
        let icao_code = &airport.ident;
        let iata_code = airport.iata_code.as_deref();

        // Check if airport already exists by IATA or ICAO code
        let existing = db.conn
            .query_row(
                "SELECT id, latitude, longitude FROM airports WHERE iata_code = ?1 OR icao_code = ?2",
                rusqlite::params![iata_code.unwrap_or(""), icao_code],
                |row| {
                    let id: String = row.get(0)?;
                    let lat: Option<f64> = row.get(1)?;
                    let lon: Option<f64> = row.get(2)?;
                    Ok((id, lat, lon))
                }
            )
            .optional()
            .map_err(|e| format!("Database error: {}", e))?;

        match existing {
            Some((id, existing_lat, existing_lon)) => {
                // Update if coordinates are missing
                if existing_lat.is_none() || existing_lon.is_none() {
                    if let (Some(lat), Some(lon)) = (airport.latitude_deg, airport.longitude_deg) {
                        match db.conn.execute(
                            "UPDATE airports SET latitude = ?1, longitude = ?2, name = ?3, city = ?4, country = ?5 WHERE id = ?6",
                            rusqlite::params![lat, lon, airport.name, airport.municipality, airport.iso_country, id],
                        ) {
                            Ok(_) => result.enriched_count += 1,
                            Err(e) => result.errors.push(format!("Failed to update {}: {}", icao_code, e)),
                        }
                    }
                }
            }
            None => {
                // Insert new airport (needs valid coordinates)
                if let (Some(lat), Some(lon)) = (airport.latitude_deg, airport.longitude_deg) {
                    match db.conn.execute(
                        "INSERT INTO airports (id, icao_code, iata_code, name, city, country, latitude, longitude)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        rusqlite::params![
                            uuid::Uuid::new_v4().to_string(),
                            icao_code,
                            iata_code,
                            airport.name,
                            airport.municipality,
                            airport.iso_country,
                            lat,
                            lon
                        ],
                    ) {
                        Ok(_) => result.new_airports += 1,
                        Err(e) => result.errors.push(format!("Failed to insert {}: {}", icao_code, e)),
                    }
                }
            }
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn get_missing_coordinates_count(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: usize = db.conn
        .query_row(
            "SELECT COUNT(*) FROM airports WHERE latitude IS NULL OR longitude IS NULL",
            [],
            |row| row.get(0)
        )
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(count)
}

#[tauri::command]
pub fn get_total_airports_count(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: usize = db.conn
        .query_row(
            "SELECT COUNT(*) FROM airports",
            [],
            |row| row.get(0)
        )
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(count)
}

/// Find the airports.csv file, trying multiple paths for dev and production
fn find_airports_csv(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let possible_paths = vec![
        // Production: bundled resources via Tauri
        app_handle.path()
            .resolve("resources/airports.csv", tauri::path::BaseDirectory::Resource)
            .ok(),
        // Development: source directory using compile-time path
        Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("airports.csv")),
        // Development: relative to working directory
        Some(std::path::PathBuf::from("resources/airports.csv")),
        Some(std::path::PathBuf::from("src-tauri/resources/airports.csv")),
    ];

    for path_opt in possible_paths {
        if let Some(path) = path_opt {
            if path.exists() {
                eprintln!("[AirportCache] Found airports.csv at: {:?}", path);
                return Ok(path);
            }
        }
    }

    Err("airports.csv not found in any known location".to_string())
}

// Helper to ensure the airport cache is initialized
fn get_or_init_cache(app_handle: &tauri::AppHandle) -> Result<std::sync::MutexGuard<'static, Option<AirportCache>>, String> {
    let mutex = AIRPORT_CACHE.get_or_init(|| Mutex::new(None));
    let mut guard = mutex.lock().map_err(|e| format!("Cache lock poisoned: {}", e))?;

    if guard.is_none() {
        let resource_path = find_airports_csv(app_handle)?;

        let airports = load_airports_from_csv(&resource_path)
            .map_err(|e| format!("Failed to load airports.csv: {}", e))?;

        eprintln!("[AirportCache] Loaded {} airports from CSV", airports.len());
        *guard = Some(AirportCache::new(airports));
    }

    Ok(guard)
}

/// Lookup an airport by ICAO or IATA code (exact match with fuzzy suggestions)
#[tauri::command]
pub fn lookup_airport(
    app_handle: tauri::AppHandle,
    code: String,
) -> Result<AirportLookupResponse, String> {
    let guard = get_or_init_cache(&app_handle)?;
    let cache = guard.as_ref().ok_or("Airport cache not initialized")?;

    let code = code.trim();
    if code.is_empty() {
        return Ok(AirportLookupResponse {
            exact_match: None,
            suggestions: vec![],
        });
    }

    // Try exact match first
    let exact_match = cache.lookup_exact(code).cloned();

    // If no exact match, provide fuzzy suggestions
    let suggestions = if exact_match.is_none() {
        cache.search_fuzzy(code, 5)
    } else {
        vec![]
    };

    Ok(AirportLookupResponse {
        exact_match,
        suggestions,
    })
}

/// Search airports by any term (name, code, city, country)
#[tauri::command]
pub fn search_airports_csv(
    app_handle: tauri::AppHandle,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<AirportSearchResult>, String> {
    let guard = get_or_init_cache(&app_handle)?;
    let cache = guard.as_ref().ok_or("Airport cache not initialized")?;

    let query = query.trim();
    if query.is_empty() {
        return Ok(vec![]);
    }

    let limit = limit.unwrap_or(10);
    let results = cache.search_fuzzy(query, limit);

    Ok(results)
}

/// Get airport count in CSV (for stats display)
#[tauri::command]
pub fn get_csv_airport_count(
    app_handle: tauri::AppHandle,
) -> Result<usize, String> {
    let guard = get_or_init_cache(&app_handle)?;
    let cache = guard.as_ref().ok_or("Airport cache not initialized")?;
    Ok(cache.airports.len())
}

/// Get all airports from CSV for map display
/// Returns all airports with coordinates for 2D/3D map visualization
#[tauri::command]
pub fn get_all_csv_airports(
    app_handle: tauri::AppHandle,
) -> Result<Vec<AirportData>, String> {
    let guard = get_or_init_cache(&app_handle)?;
    let cache = guard.as_ref().ok_or("Airport cache not initialized")?;
    Ok(cache.airports.clone())
}

/// Get airports filtered by type for map display (large, medium, small)
#[tauri::command]
pub fn get_csv_airports_by_codes(
    app_handle: tauri::AppHandle,
    codes: Vec<String>,
) -> Result<Vec<AirportData>, String> {
    let guard = get_or_init_cache(&app_handle)?;
    let cache = guard.as_ref().ok_or("Airport cache not initialized")?;

    let codes_upper: Vec<String> = codes.iter().map(|c| c.to_uppercase()).collect();

    let results: Vec<AirportData> = cache.airports.iter()
        .filter(|a| {
            codes_upper.contains(&a.ident.to_uppercase()) ||
            a.iata_code.as_ref().map(|c| codes_upper.contains(&c.to_uppercase())).unwrap_or(false)
        })
        .cloned()
        .collect();

    Ok(results)
}

/// Get all unique airport codes from flight logs
#[tauri::command]
pub fn get_visited_airport_codes(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut codes: Vec<String> = Vec::new();

    // Get departure airports
    let mut stmt = db.conn
        .prepare("SELECT DISTINCT departure_airport FROM flights WHERE departure_airport IS NOT NULL AND departure_airport != ''")
        .map_err(|e| e.to_string())?;

    let dep_codes: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    codes.extend(dep_codes);

    drop(stmt);

    // Get arrival airports
    let mut stmt = db.conn
        .prepare("SELECT DISTINCT arrival_airport FROM flights WHERE arrival_airport IS NOT NULL AND arrival_airport != ''")
        .map_err(|e| e.to_string())?;

    let arr_codes: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    codes.extend(arr_codes);

    // Deduplicate and sort
    codes.sort();
    codes.dedup();

    Ok(codes)
}

#[derive(Debug, Serialize)]
pub struct SmartImportResult {
    pub codes_found: usize,
    pub already_in_db: usize,
    pub imported: usize,
    pub not_found_in_csv: Vec<String>,
}

/// Import only airports that appear in flight logs (smart import)
#[tauri::command]
pub fn import_visited_airports(
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<SmartImportResult, String> {
    // Get visited airport codes from flights
    let visited_codes = {
        let db = state.db.lock().map_err(|e| e.to_string())?;

        let mut codes: Vec<String> = Vec::new();

        let mut stmt = db.conn
            .prepare("SELECT DISTINCT departure_airport FROM flights WHERE departure_airport IS NOT NULL AND departure_airport != ''")
            .map_err(|e| e.to_string())?;

        codes.extend(
            stmt.query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
        );

        drop(stmt);

        let mut stmt = db.conn
            .prepare("SELECT DISTINCT arrival_airport FROM flights WHERE arrival_airport IS NOT NULL AND arrival_airport != ''")
            .map_err(|e| e.to_string())?;

        codes.extend(
            stmt.query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
        );

        codes.sort();
        codes.dedup();
        codes
    };

    let codes_found = visited_codes.len();

    // Get CSV cache
    let guard = get_or_init_cache(&app_handle)?;
    let cache = guard.as_ref().ok_or("Airport cache not initialized")?;

    let mut already_in_db = 0;
    let mut imported = 0;
    let mut not_found_in_csv: Vec<String> = Vec::new();

    let db = state.db.lock().map_err(|e| e.to_string())?;

    for code in &visited_codes {
        let code_upper = code.to_uppercase();

        // Check if already in database
        let exists: bool = db.conn
            .query_row(
                "SELECT 1 FROM airports WHERE UPPER(iata_code) = ?1 OR UPPER(icao_code) = ?1",
                [&code_upper],
                |_| Ok(true)
            )
            .unwrap_or(false);

        if exists {
            already_in_db += 1;
            continue;
        }

        // Find in CSV cache
        let airport = cache.lookup_exact(&code_upper);

        match airport {
            Some(a) => {
                if let (Some(lat), Some(lon)) = (a.latitude_deg, a.longitude_deg) {
                    let result = db.conn.execute(
                        "INSERT INTO airports (id, icao_code, iata_code, name, city, country, latitude, longitude)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        rusqlite::params![
                            uuid::Uuid::new_v4().to_string(),
                            &a.ident,
                            &a.iata_code,
                            &a.name,
                            &a.municipality,
                            &a.iso_country,
                            lat,
                            lon
                        ],
                    );

                    if result.is_ok() {
                        imported += 1;
                    }
                }
            }
            None => {
                not_found_in_csv.push(code.clone());
            }
        }
    }

    Ok(SmartImportResult {
        codes_found,
        already_in_db,
        imported,
        not_found_in_csv,
    })
}
