use super::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

// ===== DATA STRUCTURES =====

#[derive(Debug, Serialize, Deserialize)]
pub struct UserCorrection {
    pub id: String,
    pub field_name: String,
    pub original_value: Option<String>,
    pub corrected_value: String,
    pub occurrence_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPattern {
    pub pattern_type: String,
    pub pattern_key: String,
    pub pattern_value: String,
    pub frequency: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FlightAnomaly {
    pub id: String,
    pub flight_id: String,
    pub anomaly_type: String,
    pub severity: String,
    pub description: String,
    pub suggested_fix: Option<String>,
    pub is_resolved: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateCandidate {
    pub id: String,
    pub flight_id_1: String,
    pub flight_id_2: String,
    pub similarity_score: f64,
    pub match_reasons: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RouteStats {
    pub departure_airport: String,
    pub arrival_airport: String,
    pub avg_duration_minutes: Option<f64>,
    pub avg_distance_km: Option<f64>,
    pub common_aircraft: Option<String>,
    pub flight_count: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SmartDefaults {
    pub airports: Vec<String>,
    pub airlines: Vec<String>,
    pub aircraft: Vec<String>,
    pub routes: Vec<RouteStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FuelPriceRecord {
    pub id: String,
    pub airport_code: Option<String>,
    pub location_name: String,
    pub region: Option<String>,
    pub country: Option<String>,
    pub fuel_type: String,
    pub price_per_gallon: f64,
    pub currency: String,
    pub effective_date: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SelfImprovementStats {
    pub corrections_count: i32,
    pub patterns_count: i32,
    pub anomalies_pending: i32,
    pub duplicates_pending: i32,
    pub cached_responses: i32,
    pub fuel_prices_stored: i32,
    pub routes_learned: i32,
}

// ===== AUTO-FILL LEARNING =====

#[tauri::command]
pub fn record_correction(
    state: State<'_, AppState>,
    user_id: String,
    field_name: String,
    original_value: Option<String>,
    corrected_value: String,
    context: Option<String>,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    db.conn
        .execute(
            "INSERT INTO user_corrections (id, user_id, field_name, original_value, corrected_value, context)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(user_id, field_name, original_value, corrected_value)
             DO UPDATE SET occurrence_count = occurrence_count + 1, updated_at = datetime('now')",
            rusqlite::params![&id, &user_id, &field_name, &original_value, &corrected_value, &context],
        )
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub fn get_suggested_correction(
    state: State<'_, AppState>,
    user_id: String,
    field_name: String,
    original_value: String,
) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let result: Option<String> = db
        .conn
        .query_row(
            "SELECT corrected_value FROM user_corrections
             WHERE user_id = ?1 AND field_name = ?2 AND original_value = ?3
             ORDER BY occurrence_count DESC LIMIT 1",
            rusqlite::params![&user_id, &field_name, &original_value],
            |row| row.get(0),
        )
        .ok();

    Ok(result)
}

// ===== SMART DEFAULTS =====

#[tauri::command]
pub fn record_user_pattern(
    state: State<'_, AppState>,
    user_id: String,
    pattern_type: String,
    pattern_key: String,
    pattern_value: String,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    db.conn
        .execute(
            "INSERT INTO user_patterns (id, user_id, pattern_type, pattern_key, pattern_value)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(user_id, pattern_type, pattern_key)
             DO UPDATE SET frequency = frequency + 1, last_used = datetime('now'), pattern_value = ?5",
            rusqlite::params![&id, &user_id, &pattern_type, &pattern_key, &pattern_value],
        )
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub fn get_smart_defaults(
    state: State<'_, AppState>,
    user_id: String,
) -> Result<SmartDefaults, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get top airports
    let mut stmt = db.conn
        .prepare("SELECT pattern_value FROM user_patterns WHERE user_id = ?1 AND pattern_type = 'airport' ORDER BY frequency DESC LIMIT 10")
        .map_err(|e| e.to_string())?;
    let airports: Vec<String> = stmt
        .query_map([&user_id], |row: &rusqlite::Row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<String, _>| r.ok())
        .collect();

    // Get top airlines
    let mut stmt = db.conn
        .prepare("SELECT pattern_value FROM user_patterns WHERE user_id = ?1 AND pattern_type = 'airline' ORDER BY frequency DESC LIMIT 10")
        .map_err(|e| e.to_string())?;
    let airlines: Vec<String> = stmt
        .query_map([&user_id], |row: &rusqlite::Row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<String, _>| r.ok())
        .collect();

    // Get top aircraft
    let mut stmt = db.conn
        .prepare("SELECT pattern_value FROM user_patterns WHERE user_id = ?1 AND pattern_type = 'aircraft' ORDER BY frequency DESC LIMIT 10")
        .map_err(|e| e.to_string())?;
    let aircraft: Vec<String> = stmt
        .query_map([&user_id], |row: &rusqlite::Row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<String, _>| r.ok())
        .collect();

    // Get route statistics
    let mut stmt = db.conn
        .prepare("SELECT departure_airport, arrival_airport, avg_duration_minutes, avg_distance_km, common_aircraft, flight_count FROM route_statistics ORDER BY flight_count DESC LIMIT 20")
        .map_err(|e| e.to_string())?;
    let routes: Vec<RouteStats> = stmt
        .query_map([], |row: &rusqlite::Row| {
            Ok(RouteStats {
                departure_airport: row.get(0)?,
                arrival_airport: row.get(1)?,
                avg_duration_minutes: row.get(2)?,
                avg_distance_km: row.get(3)?,
                common_aircraft: row.get(4)?,
                flight_count: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<RouteStats, _>| r.ok())
        .collect();

    Ok(SmartDefaults { airports, airlines, aircraft, routes })
}

// ===== ANOMALY DETECTION =====

#[tauri::command]
pub fn detect_flight_anomalies(
    state: State<'_, AppState>,
    flight_id: String,
) -> Result<Vec<FlightAnomaly>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut anomalies = Vec::new();

    // Get flight data
    let flight: Option<(f64, f64, String, String)> = db.conn
        .query_row(
            "SELECT distance_km, flight_duration_hours, departure_airport, arrival_airport FROM flights WHERE id = ?1",
            [&flight_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok();

    if let Some((distance, duration, dep, arr)) = flight {
        // Check for impossible speed (> 1200 km/h for commercial, accounting for wind)
        if duration > 0.0 {
            let speed = distance / duration;
            if speed > 1200.0 {
                let id = Uuid::new_v4().to_string();
                anomalies.push(FlightAnomaly {
                    id: id.clone(),
                    flight_id: flight_id.clone(),
                    anomaly_type: "impossible_speed".to_string(),
                    severity: "error".to_string(),
                    description: format!("Calculated speed of {:.0} km/h exceeds maximum possible", speed),
                    suggested_fix: Some("Check flight duration or distance".to_string()),
                    is_resolved: false,
                });
                // Save to database
                let _ = db.conn.execute(
                    "INSERT OR IGNORE INTO flight_anomalies (id, flight_id, anomaly_type, severity, description, suggested_fix) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![&id, &flight_id, "impossible_speed", "error", &anomalies.last().unwrap().description, "Check flight duration or distance"],
                );
            }
        }

        // Check for zero/negative distance
        if distance <= 0.0 && dep != arr {
            let id = Uuid::new_v4().to_string();
            anomalies.push(FlightAnomaly {
                id: id.clone(),
                flight_id: flight_id.clone(),
                anomaly_type: "invalid_distance".to_string(),
                severity: "warning".to_string(),
                description: "Flight distance is zero or negative".to_string(),
                suggested_fix: Some("Recalculate distance from airport coordinates".to_string()),
                is_resolved: false,
            });
            let _ = db.conn.execute(
                "INSERT OR IGNORE INTO flight_anomalies (id, flight_id, anomaly_type, severity, description, suggested_fix) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![&id, &flight_id, "invalid_distance", "warning", "Flight distance is zero or negative", "Recalculate distance from airport coordinates"],
            );
        }

        // Check route statistics for outliers
        let route_stats: Option<(f64, f64)> = db.conn
            .query_row(
                "SELECT avg_duration_minutes, avg_distance_km FROM route_statistics WHERE departure_airport = ?1 AND arrival_airport = ?2",
                [&dep, &arr],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((avg_dur, _avg_dist)) = route_stats {
            let duration_mins = duration * 60.0;
            if duration_mins > avg_dur * 2.0 || duration_mins < avg_dur * 0.5 {
                let id = Uuid::new_v4().to_string();
                anomalies.push(FlightAnomaly {
                    id: id.clone(),
                    flight_id: flight_id.clone(),
                    anomaly_type: "duration_outlier".to_string(),
                    severity: "warning".to_string(),
                    description: format!("Duration ({:.0} min) differs significantly from average ({:.0} min) for this route", duration_mins, avg_dur),
                    suggested_fix: Some("Verify departure and arrival times".to_string()),
                    is_resolved: false,
                });
            }
        }
    }

    Ok(anomalies)
}

#[tauri::command]
pub fn get_pending_anomalies(
    state: State<'_, AppState>,
) -> Result<Vec<FlightAnomaly>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn
        .prepare("SELECT id, flight_id, anomaly_type, severity, description, suggested_fix, is_resolved FROM flight_anomalies WHERE is_resolved = 0 ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let anomalies = stmt
        .query_map([], |row: &rusqlite::Row| {
            Ok(FlightAnomaly {
                id: row.get(0)?,
                flight_id: row.get(1)?,
                anomaly_type: row.get(2)?,
                severity: row.get(3)?,
                description: row.get(4)?,
                suggested_fix: row.get(5)?,
                is_resolved: row.get::<_, i32>(6)? == 1,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<FlightAnomaly, _>| r.ok())
        .collect();

    Ok(anomalies)
}

#[tauri::command]
pub fn resolve_anomaly(
    state: State<'_, AppState>,
    anomaly_id: String,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.conn
        .execute(
            "UPDATE flight_anomalies SET is_resolved = 1, resolved_at = datetime('now') WHERE id = ?1",
            [&anomaly_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(true)
}

// ===== DUPLICATE DETECTION =====

#[tauri::command]
pub fn find_duplicates(
    state: State<'_, AppState>,
    flight_id: String,
) -> Result<Vec<DuplicateCandidate>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut duplicates = Vec::new();

    // Get the flight to compare
    let flight: Option<(String, String, String, Option<String>)> = db.conn
        .query_row(
            "SELECT departure_airport, arrival_airport, departure_datetime, flight_number FROM flights WHERE id = ?1",
            [&flight_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .ok();

    if let Some((dep, arr, datetime, flight_num)) = flight {
        // Find flights with same route within 24 hours
        let mut stmt = db.conn
            .prepare(
                "SELECT id, departure_airport, arrival_airport, departure_datetime, flight_number
                 FROM flights
                 WHERE id != ?1
                 AND departure_airport = ?2
                 AND arrival_airport = ?3
                 AND abs(julianday(departure_datetime) - julianday(?4)) < 1",
            )
            .map_err(|e| e.to_string())?;

        let candidates: Vec<(String, String, String, String, Option<String>)> = stmt
            .query_map(rusqlite::params![&flight_id, &dep, &arr, &datetime], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        for (cand_id, _, _, cand_datetime, cand_flight_num) in candidates {
            let mut score = 0.5; // Base score for same route same day
            let mut reasons = vec!["Same route".to_string(), "Same day".to_string()];

            // Check if flight numbers match
            if flight_num.is_some() && flight_num == cand_flight_num {
                score += 0.3;
                reasons.push("Same flight number".to_string());
            }

            // Check if times are very close
            if datetime == cand_datetime {
                score += 0.2;
                reasons.push("Exact same departure time".to_string());
            }

            if score >= 0.5 {
                let id = Uuid::new_v4().to_string();
                duplicates.push(DuplicateCandidate {
                    id: id.clone(),
                    flight_id_1: flight_id.clone(),
                    flight_id_2: cand_id.clone(),
                    similarity_score: score,
                    match_reasons: reasons.clone(),
                });

                // Save to database
                let _ = db.conn.execute(
                    "INSERT OR IGNORE INTO duplicate_candidates (id, flight_id_1, flight_id_2, similarity_score, match_reasons) VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![&id, &flight_id, &cand_id, score, serde_json::to_string(&reasons).unwrap_or_default()],
                );
            }
        }
    }

    Ok(duplicates)
}

#[tauri::command]
pub fn get_pending_duplicates(
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateCandidate>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn
        .prepare("SELECT id, flight_id_1, flight_id_2, similarity_score, match_reasons FROM duplicate_candidates WHERE status = 'pending' ORDER BY similarity_score DESC")
        .map_err(|e| e.to_string())?;

    let duplicates = stmt
        .query_map([], |row: &rusqlite::Row| {
            let reasons_str: String = row.get(4)?;
            let reasons: Vec<String> = serde_json::from_str(&reasons_str).unwrap_or_default();
            Ok(DuplicateCandidate {
                id: row.get(0)?,
                flight_id_1: row.get(1)?,
                flight_id_2: row.get(2)?,
                similarity_score: row.get(3)?,
                match_reasons: reasons,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<DuplicateCandidate, _>| r.ok())
        .collect();

    Ok(duplicates)
}

// ===== ROUTE LEARNING =====

#[tauri::command]
pub fn update_route_statistics(
    state: State<'_, AppState>,
    departure_airport: String,
    arrival_airport: String,
    duration_minutes: f64,
    distance_km: f64,
    aircraft: Option<String>,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();

    db.conn
        .execute(
            "INSERT INTO route_statistics (id, departure_airport, arrival_airport, avg_duration_minutes, avg_distance_km, common_aircraft, flight_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)
             ON CONFLICT(departure_airport, arrival_airport) DO UPDATE SET
                avg_duration_minutes = (avg_duration_minutes * flight_count + ?4) / (flight_count + 1),
                avg_distance_km = (avg_distance_km * flight_count + ?5) / (flight_count + 1),
                common_aircraft = COALESCE(?6, common_aircraft),
                flight_count = flight_count + 1,
                last_updated = datetime('now')",
            rusqlite::params![&id, &departure_airport, &arrival_airport, duration_minutes, distance_km, &aircraft],
        )
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub fn get_route_prediction(
    state: State<'_, AppState>,
    departure_airport: String,
    arrival_airport: String,
) -> Result<Option<RouteStats>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let result = db.conn
        .query_row(
            "SELECT departure_airport, arrival_airport, avg_duration_minutes, avg_distance_km, common_aircraft, flight_count FROM route_statistics WHERE departure_airport = ?1 AND arrival_airport = ?2",
            [&departure_airport, &arrival_airport],
            |row| Ok(RouteStats {
                departure_airport: row.get(0)?,
                arrival_airport: row.get(1)?,
                avg_duration_minutes: row.get(2)?,
                avg_distance_km: row.get(3)?,
                common_aircraft: row.get(4)?,
                flight_count: row.get(5)?,
            }),
        )
        .ok();

    Ok(result)
}

// ===== FUEL PRICE HISTORY =====

#[tauri::command]
pub fn save_fuel_price(
    state: State<'_, AppState>,
    airport_code: Option<String>,
    location_name: String,
    region: Option<String>,
    country: Option<String>,
    fuel_type: String,
    price_per_gallon: f64,
    currency: String,
    effective_date: String,
    source: String,
    source_url: Option<String>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let price_per_liter = price_per_gallon / 3.78541;

    db.conn
        .execute(
            "INSERT INTO fuel_price_history (id, airport_code, location_name, region, country, fuel_type, price_per_gallon, price_per_liter, currency, effective_date, source, source_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![&id, &airport_code, &location_name, &region, &country, &fuel_type, price_per_gallon, price_per_liter, &currency, &effective_date, &source, &source_url],
        )
        .map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub fn get_fuel_price_history(
    state: State<'_, AppState>,
    airport_code: Option<String>,
    fuel_type: Option<String>,
    limit: Option<i32>,
) -> Result<Vec<FuelPriceRecord>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(100);

    // Build dynamic query
    let mut conditions = Vec::new();
    if airport_code.is_some() {
        conditions.push("airport_code = ?");
    }
    if fuel_type.is_some() {
        conditions.push("fuel_type = ?");
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let query = format!(
        "SELECT id, airport_code, location_name, region, country, fuel_type, price_per_gallon, currency, effective_date, source FROM fuel_price_history {} ORDER BY effective_date DESC LIMIT {}",
        where_clause, lim
    );

    let mut stmt = db.conn.prepare(&query).map_err(|e| e.to_string())?;

    // Build params dynamically
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(ref ac) = airport_code {
        params.push(Box::new(ac.clone()));
    }
    if let Some(ref ft) = fuel_type {
        params.push(Box::new(ft.clone()));
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let records: Vec<FuelPriceRecord> = stmt
        .query_map(param_refs.as_slice(), |row: &rusqlite::Row| {
            Ok(FuelPriceRecord {
                id: row.get(0)?,
                airport_code: row.get(1)?,
                location_name: row.get(2)?,
                region: row.get(3)?,
                country: row.get(4)?,
                fuel_type: row.get(5)?,
                price_per_gallon: row.get(6)?,
                currency: row.get(7)?,
                effective_date: row.get(8)?,
                source: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r: Result<FuelPriceRecord, _>| r.ok())
        .collect();

    Ok(records)
}

// ===== AI CACHE =====

#[tauri::command]
pub fn cache_ai_response(
    state: State<'_, AppState>,
    query_text: String,
    provider: String,
    response_text: String,
    confidence: Option<f64>,
    ttl_hours: Option<i32>,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let query_hash = format!("{:x}", md5::compute(&query_text));
    let ttl = ttl_hours.unwrap_or(24 * 7); // Default 1 week
    let expires = format!("datetime('now', '+{} hours')", ttl);

    db.conn
        .execute(
            &format!("INSERT INTO ai_response_cache (id, query_hash, provider, query_text, response_text, confidence, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, {})
             ON CONFLICT(query_hash) DO UPDATE SET hit_count = hit_count + 1", expires),
            rusqlite::params![&id, &query_hash, &provider, &query_text, &response_text, confidence],
        )
        .map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub fn get_cached_ai_response(
    state: State<'_, AppState>,
    query_text: String,
) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let query_hash = format!("{:x}", md5::compute(&query_text));

    let result: Option<String> = db.conn
        .query_row(
            "SELECT response_text FROM ai_response_cache WHERE query_hash = ?1 AND (expires_at IS NULL OR expires_at > datetime('now'))",
            [&query_hash],
            |row| row.get(0),
        )
        .ok();

    // Increment hit count
    if result.is_some() {
        let _ = db.conn.execute(
            "UPDATE ai_response_cache SET hit_count = hit_count + 1 WHERE query_hash = ?1",
            [&query_hash],
        );
    }

    Ok(result)
}

// ===== STATS =====

#[tauri::command]
pub fn get_self_improvement_stats(
    state: State<'_, AppState>,
) -> Result<SelfImprovementStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let corrections_count: i32 = db.conn
        .query_row("SELECT COUNT(*) FROM user_corrections", [], |row| row.get(0))
        .unwrap_or(0);

    let patterns_count: i32 = db.conn
        .query_row("SELECT COUNT(*) FROM user_patterns", [], |row| row.get(0))
        .unwrap_or(0);

    let anomalies_pending: i32 = db.conn
        .query_row("SELECT COUNT(*) FROM flight_anomalies WHERE is_resolved = 0", [], |row| row.get(0))
        .unwrap_or(0);

    let duplicates_pending: i32 = db.conn
        .query_row("SELECT COUNT(*) FROM duplicate_candidates WHERE status = 'pending'", [], |row| row.get(0))
        .unwrap_or(0);

    let cached_responses: i32 = db.conn
        .query_row("SELECT COUNT(*) FROM ai_response_cache", [], |row| row.get(0))
        .unwrap_or(0);

    let fuel_prices_stored: i32 = db.conn
        .query_row("SELECT COUNT(*) FROM fuel_price_history", [], |row| row.get(0))
        .unwrap_or(0);

    let routes_learned: i32 = db.conn
        .query_row("SELECT COUNT(*) FROM route_statistics", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(SelfImprovementStats {
        corrections_count,
        patterns_count,
        anomalies_pending,
        duplicates_pending,
        cached_responses,
        fuel_prices_stored,
        routes_learned,
    })
}

/// Populate route statistics from existing flight data
#[tauri::command]
pub fn populate_route_statistics(
    state: State<'_, AppState>,
) -> Result<PopulateRouteStatsResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Aggregate route data from flights
    let mut stmt = db.conn
        .prepare(
            "SELECT
                departure_airport,
                arrival_airport,
                AVG(duration_minutes) as avg_duration,
                AVG(distance_km) as avg_distance,
                COUNT(*) as flight_count
             FROM flights
             WHERE departure_airport IS NOT NULL
               AND arrival_airport IS NOT NULL
               AND departure_airport != ''
               AND arrival_airport != ''
             GROUP BY departure_airport, arrival_airport
             HAVING flight_count >= 1"
        )
        .map_err(|e| e.to_string())?;

    let routes: Vec<(String, String, Option<f64>, Option<f64>, i32)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<f64>>(3)?,
                row.get::<_, i32>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    drop(stmt);

    let mut routes_added = 0;
    let mut routes_updated = 0;

    for (dep, arr, avg_duration, avg_distance, count) in routes {
        let id = Uuid::new_v4().to_string();

        let result = db.conn.execute(
            "INSERT INTO route_statistics (id, departure_airport, arrival_airport, avg_duration_minutes, avg_distance_km, flight_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(departure_airport, arrival_airport) DO UPDATE SET
                avg_duration_minutes = ?4,
                avg_distance_km = ?5,
                flight_count = ?6,
                updated_at = datetime('now')",
            rusqlite::params![id, dep, arr, avg_duration, avg_distance, count],
        );

        match result {
            Ok(1) => routes_added += 1,
            Ok(_) => routes_updated += 1,
            Err(e) => eprintln!("Failed to insert route {}->{}: {}", dep, arr, e),
        }
    }

    Ok(PopulateRouteStatsResult {
        routes_added,
        routes_updated,
        total_routes: routes_added + routes_updated,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct PopulateRouteStatsResult {
    pub routes_added: i32,
    pub routes_updated: i32,
    pub total_routes: i32,
}
