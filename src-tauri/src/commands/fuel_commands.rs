// Fuel price tracking commands with AI-powered search
use super::AppState;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

// ===== DATA STRUCTURES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelPrice {
    pub id: String,
    pub airport_code: Option<String>,
    pub location_name: String,
    pub region: Option<String>,
    pub country: Option<String>,
    pub fuel_type: String,
    pub price_per_gallon: f64,
    pub price_per_liter: Option<f64>,
    pub currency: String,
    pub effective_date: String,
    pub source: Option<String>,
    pub source_url: Option<String>,
    pub confidence: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelEntry {
    pub id: String,
    pub user_id: String,
    pub flight_id: Option<String>,
    pub aircraft_id: Option<String>,
    pub airport_code: Option<String>,
    pub location_name: Option<String>,
    pub fuel_type: String,
    pub gallons: f64,
    pub price_per_gallon: f64,
    pub total_cost: f64,
    pub currency: String,
    pub purchase_date: String,
    pub fbo_name: Option<String>,
    pub receipt_number: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelSearchRequest {
    pub query: String,
    pub fuel_type: Option<String>,  // jet_a, avgas_100ll, mogas
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelSearchResult {
    pub prices: Vec<FuelPrice>,
    pub ai_summary: String,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFuelEntry {
    pub user_id: String,
    pub flight_id: Option<String>,
    pub aircraft_id: Option<String>,
    pub airport_code: Option<String>,
    pub location_name: Option<String>,
    pub fuel_type: String,
    pub gallons: f64,
    pub price_per_gallon: f64,
    pub currency: Option<String>,
    pub purchase_date: String,
    pub fbo_name: Option<String>,
    pub receipt_number: Option<String>,
    pub notes: Option<String>,
}

// ===== AI FUEL PRICE SEARCH =====

#[tauri::command]
pub async fn search_fuel_prices(
    request: FuelSearchRequest,
    state: State<'_, AppState>,
) -> Result<FuelSearchResult, String> {
    // Get API key
    let api_key = {
        let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;
        db.get_setting("gemini_api_key")
            .map_err(|e| format!("Failed to get API key: {}", e))?
            .ok_or_else(|| "Gemini API key not configured".to_string())?
    };

    let fuel_type = request.fuel_type.as_deref().unwrap_or("jet_a");
    let fuel_type_name = match fuel_type {
        "jet_a" => "Jet-A fuel",
        "avgas_100ll" => "100LL Avgas",
        "mogas" => "Mogas/automotive gasoline",
        _ => "aviation fuel",
    };

    // Build the AI prompt
    let prompt = format!(
        r#"You are a fuel price research assistant. Find current {} prices for: {}

{}

Provide a structured response with:
1. Current average price per gallon in USD
2. Price range (low to high) if available
3. Specific airport/FBO prices if the query mentions a location
4. Date of the price information
5. Sources for your information

Format your response as JSON with this structure:
{{
  "prices": [
    {{
      "location_name": "Location or region name",
      "airport_code": "ICAO code if applicable or null",
      "region": "State/Country/Region",
      "country": "Country code",
      "price_per_gallon": 6.50,
      "price_per_liter": 1.72,
      "currency": "USD",
      "effective_date": "2024-11",
      "source": "Source name",
      "source_url": "URL if available",
      "confidence": "high/medium/low"
    }}
  ],
  "summary": "Brief summary of findings",
  "sources": ["Source 1", "Source 2"]
}}

Be accurate. If you're uncertain about specific prices, indicate low confidence.
Include both specific location prices and regional averages when relevant."#,
        fuel_type_name,
        request.query,
        request.region.as_ref().map(|r| format!("Focus on region: {}", r)).unwrap_or_default()
    );

    // Call Gemini API
    let client = reqwest::Client::new();
    let response = client
        .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-lite:generateContent")
        .query(&[("key", &api_key)])
        .json(&serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }],
            "generationConfig": {
                "temperature": 0.3,
                "maxOutputTokens": 2048
            }
        }))
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    // Extract the text response
    let ai_text = response_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .ok_or_else(|| "No response from AI".to_string())?;

    // Try to parse the JSON from the response
    let parsed = parse_fuel_response(ai_text, fuel_type)?;

    // Cache the prices in the database
    {
        let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;
        for price in &parsed.prices {
            let id = Uuid::new_v4().to_string();
            let _ = db.get_connection().execute(
                "INSERT OR REPLACE INTO fuel_price_cache
                 (id, airport_code, location_name, region, country, fuel_type,
                  price_per_gallon, price_per_liter, currency, effective_date,
                  source, source_url, confidence, ai_response, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, datetime('now'))",
                params![
                    id,
                    price.airport_code,
                    price.location_name,
                    price.region,
                    price.country,
                    fuel_type,
                    price.price_per_gallon,
                    price.price_per_liter,
                    price.currency,
                    price.effective_date,
                    price.source,
                    price.source_url,
                    price.confidence,
                    ai_text
                ],
            );

            // Also save to permanent fuel_price_history for self-improvement
            let history_id = Uuid::new_v4().to_string();
            let _ = db.get_connection().execute(
                "INSERT INTO fuel_price_history
                 (id, airport_code, location_name, region, country, fuel_type,
                  price_per_gallon, price_per_liter, currency, effective_date, source, source_url)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    history_id,
                    price.airport_code,
                    price.location_name,
                    price.region,
                    price.country,
                    fuel_type,
                    price.price_per_gallon,
                    price.price_per_liter,
                    price.currency,
                    price.effective_date,
                    price.source.as_deref().unwrap_or("AI Search"),
                    price.source_url
                ],
            );
        }
    }

    Ok(parsed)
}

fn parse_fuel_response(ai_text: &str, fuel_type: &str) -> Result<FuelSearchResult, String> {
    // Try to extract JSON from the response (might be wrapped in markdown code blocks)
    let json_str = if ai_text.contains("```json") {
        ai_text
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(ai_text)
    } else if ai_text.contains("```") {
        ai_text
            .split("```")
            .nth(1)
            .unwrap_or(ai_text)
    } else {
        ai_text
    };

    // Try to parse as JSON
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str.trim()) {
        let mut prices = Vec::new();

        if let Some(price_array) = parsed["prices"].as_array() {
            for p in price_array {
                prices.push(FuelPrice {
                    id: Uuid::new_v4().to_string(),
                    airport_code: p["airport_code"].as_str().map(|s| s.to_string()),
                    location_name: p["location_name"].as_str().unwrap_or("Unknown").to_string(),
                    region: p["region"].as_str().map(|s| s.to_string()),
                    country: p["country"].as_str().map(|s| s.to_string()),
                    fuel_type: fuel_type.to_string(),
                    price_per_gallon: p["price_per_gallon"].as_f64().unwrap_or(0.0),
                    price_per_liter: p["price_per_liter"].as_f64(),
                    currency: p["currency"].as_str().unwrap_or("USD").to_string(),
                    effective_date: p["effective_date"].as_str().unwrap_or("Unknown").to_string(),
                    source: p["source"].as_str().map(|s| s.to_string()),
                    source_url: p["source_url"].as_str().map(|s| s.to_string()),
                    confidence: p["confidence"].as_str().unwrap_or("medium").to_string(),
                    created_at: Utc::now().to_rfc3339(),
                });
            }
        }

        let summary = parsed["summary"].as_str().unwrap_or("").to_string();
        let sources = parsed["sources"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        Ok(FuelSearchResult {
            prices,
            ai_summary: summary,
            sources,
        })
    } else {
        // Fallback: return the raw text as summary
        Ok(FuelSearchResult {
            prices: Vec::new(),
            ai_summary: ai_text.to_string(),
            sources: Vec::new(),
        })
    }
}

// ===== CACHED PRICES =====

#[tauri::command]
pub fn get_cached_fuel_prices(
    fuel_type: Option<String>,
    region: Option<String>,
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<FuelPrice>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let mut sql = String::from(
        "SELECT id, airport_code, location_name, region, country, fuel_type,
                price_per_gallon, price_per_liter, currency, effective_date,
                source, source_url, confidence, created_at
         FROM fuel_price_cache WHERE 1=1"
    );

    let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref ft) = fuel_type {
        sql.push_str(" AND fuel_type = ?");
        params_vec.push(Box::new(ft.clone()));
    }

    if let Some(ref r) = region {
        sql.push_str(" AND (region LIKE ? OR country LIKE ? OR location_name LIKE ?)");
        let pattern = format!("%{}%", r);
        params_vec.push(Box::new(pattern.clone()));
        params_vec.push(Box::new(pattern.clone()));
        params_vec.push(Box::new(pattern));
    }

    sql.push_str(" ORDER BY created_at DESC");

    if let Some(l) = limit {
        sql.push_str(&format!(" LIMIT {}", l));
    }

    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

    let mut stmt = db.get_connection()
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let prices = stmt
        .query_map(params_refs.as_slice(), |row: &rusqlite::Row| {
            Ok(FuelPrice {
                id: row.get(0)?,
                airport_code: row.get(1)?,
                location_name: row.get(2)?,
                region: row.get(3)?,
                country: row.get(4)?,
                fuel_type: row.get(5)?,
                price_per_gallon: row.get(6)?,
                price_per_liter: row.get(7)?,
                currency: row.get(8)?,
                effective_date: row.get(9)?,
                source: row.get(10)?,
                source_url: row.get(11)?,
                confidence: row.get(12)?,
                created_at: row.get(13)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r: Result<FuelPrice, _>| r.ok())
        .collect();

    Ok(prices)
}

// ===== FUEL ENTRIES (User's purchases) =====

#[tauri::command]
pub fn add_fuel_entry(
    entry: NewFuelEntry,
    state: State<'_, AppState>,
) -> Result<FuelEntry, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let id = Uuid::new_v4().to_string();
    let total_cost = entry.gallons * entry.price_per_gallon;
    let currency = entry.currency.unwrap_or_else(|| "USD".to_string());
    let now = Utc::now().to_rfc3339();

    db.get_connection()
        .execute(
            "INSERT INTO fuel_entries
             (id, user_id, flight_id, aircraft_id, airport_code, location_name,
              fuel_type, gallons, price_per_gallon, total_cost, currency,
              purchase_date, fbo_name, receipt_number, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
            params![
                id,
                entry.user_id,
                entry.flight_id,
                entry.aircraft_id,
                entry.airport_code,
                entry.location_name,
                entry.fuel_type,
                entry.gallons,
                entry.price_per_gallon,
                total_cost,
                currency,
                entry.purchase_date,
                entry.fbo_name,
                entry.receipt_number,
                entry.notes,
                now
            ],
        )
        .map_err(|e| format!("Failed to add fuel entry: {}", e))?;

    Ok(FuelEntry {
        id,
        user_id: entry.user_id,
        flight_id: entry.flight_id,
        aircraft_id: entry.aircraft_id,
        airport_code: entry.airport_code,
        location_name: entry.location_name,
        fuel_type: entry.fuel_type,
        gallons: entry.gallons,
        price_per_gallon: entry.price_per_gallon,
        total_cost,
        currency,
        purchase_date: entry.purchase_date,
        fbo_name: entry.fbo_name,
        receipt_number: entry.receipt_number,
        notes: entry.notes,
        created_at: now,
    })
}

#[tauri::command]
pub fn get_fuel_entries(
    user_id: String,
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<FuelEntry>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let limit_val = limit.unwrap_or(100);

    let mut stmt = db.get_connection()
        .prepare(
            "SELECT id, user_id, flight_id, aircraft_id, airport_code, location_name,
                    fuel_type, gallons, price_per_gallon, total_cost, currency,
                    purchase_date, fbo_name, receipt_number, notes, created_at
             FROM fuel_entries
             WHERE user_id = ?1
             ORDER BY purchase_date DESC
             LIMIT ?2"
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let entries = stmt
        .query_map(params![user_id, limit_val], |row: &rusqlite::Row| {
            Ok(FuelEntry {
                id: row.get(0)?,
                user_id: row.get(1)?,
                flight_id: row.get(2)?,
                aircraft_id: row.get(3)?,
                airport_code: row.get(4)?,
                location_name: row.get(5)?,
                fuel_type: row.get(6)?,
                gallons: row.get(7)?,
                price_per_gallon: row.get(8)?,
                total_cost: row.get(9)?,
                currency: row.get(10)?,
                purchase_date: row.get(11)?,
                fbo_name: row.get(12)?,
                receipt_number: row.get(13)?,
                notes: row.get(14)?,
                created_at: row.get(15)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r: Result<FuelEntry, _>| r.ok())
        .collect();

    Ok(entries)
}

#[tauri::command]
pub fn get_fuel_stats(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let stats: Result<(f64, f64, f64, i32), rusqlite::Error> = db.get_connection().query_row(
        "SELECT
            COALESCE(SUM(total_cost), 0) as total_spent,
            COALESCE(SUM(gallons), 0) as total_gallons,
            COALESCE(AVG(price_per_gallon), 0) as avg_price,
            COUNT(*) as entry_count
         FROM fuel_entries WHERE user_id = ?1",
        params![user_id],
        |row: &rusqlite::Row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    );

    match stats {
        Ok((total_spent, total_gallons, avg_price, entry_count)) => {
            Ok(serde_json::json!({
                "total_spent": total_spent,
                "total_gallons": total_gallons,
                "avg_price_per_gallon": avg_price,
                "entry_count": entry_count
            }))
        }
        Err(e) => Err(format!("Failed to get fuel stats: {}", e)),
    }
}

#[tauri::command]
pub fn delete_fuel_entry(
    entry_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    db.get_connection()
        .execute("DELETE FROM fuel_entries WHERE id = ?1", params![entry_id])
        .map_err(|e| format!("Failed to delete fuel entry: {}", e))?;

    Ok(())
}

// ===== CUSTOM FUEL TYPES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelType {
    pub id: String,
    pub user_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub is_default: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFuelType {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[tauri::command]
pub fn get_fuel_types(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<FuelType>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    // Return both default types and user custom types
    let mut fuel_types = vec![
        FuelType {
            id: "default_jet_a".to_string(),
            user_id: user_id.clone(),
            code: "jet_a".to_string(),
            name: "Jet-A".to_string(),
            description: Some("Standard jet fuel for turbine engines".to_string()),
            category: "aviation".to_string(),
            is_default: true,
            created_at: "".to_string(),
        },
        FuelType {
            id: "default_avgas_100ll".to_string(),
            user_id: user_id.clone(),
            code: "avgas_100ll".to_string(),
            name: "100LL Avgas".to_string(),
            description: Some("Aviation gasoline for piston engines".to_string()),
            category: "aviation".to_string(),
            is_default: true,
            created_at: "".to_string(),
        },
        FuelType {
            id: "default_mogas".to_string(),
            user_id: user_id.clone(),
            code: "mogas".to_string(),
            name: "Mogas".to_string(),
            description: Some("Automotive gasoline approved for aviation use".to_string()),
            category: "aviation".to_string(),
            is_default: true,
            created_at: "".to_string(),
        },
    ];

    // Get custom fuel types from database
    let mut stmt = db.get_connection()
        .prepare(
            "SELECT id, user_id, code, name, description, category, is_default, created_at
             FROM fuel_types
             WHERE user_id = ?1
             ORDER BY name ASC"
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let custom_types: Vec<FuelType> = stmt
        .query_map(params![user_id], |row: &rusqlite::Row| {
            Ok(FuelType {
                id: row.get(0)?,
                user_id: row.get(1)?,
                code: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                category: row.get::<_, Option<String>>(5)?.unwrap_or_else(|| "aviation".to_string()),
                is_default: row.get::<_, i32>(6)? != 0,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r: Result<FuelType, _>| r.ok())
        .collect();

    fuel_types.extend(custom_types);

    Ok(fuel_types)
}

#[tauri::command]
pub fn add_fuel_type(
    user_id: String,
    fuel_type: NewFuelType,
    state: State<'_, AppState>,
) -> Result<FuelType, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    // Validate code format (lowercase, alphanumeric with underscores)
    let code = fuel_type.code.to_lowercase().replace(' ', "_");
    if code.is_empty() || code.len() > 50 {
        return Err("Fuel type code must be between 1 and 50 characters".to_string());
    }

    // Check if code already exists for this user or is a default
    if code == "jet_a" || code == "avgas_100ll" || code == "mogas" {
        return Err("Cannot override default fuel types".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let category = fuel_type.category.unwrap_or_else(|| "aviation".to_string());
    let now = Utc::now().to_rfc3339();

    db.get_connection()
        .execute(
            "INSERT INTO fuel_types (id, user_id, code, name, description, category, is_default, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
            params![
                id,
                user_id,
                code,
                fuel_type.name,
                fuel_type.description,
                category,
                now
            ],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE constraint") {
                "A fuel type with this code already exists".to_string()
            } else {
                format!("Failed to add fuel type: {}", e)
            }
        })?;

    Ok(FuelType {
        id,
        user_id,
        code,
        name: fuel_type.name,
        description: fuel_type.description,
        category,
        is_default: false,
        created_at: now,
    })
}

#[tauri::command]
pub fn delete_fuel_type(
    fuel_type_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    // Prevent deleting default types
    if fuel_type_id.starts_with("default_") {
        return Err("Cannot delete default fuel types".to_string());
    }

    db.get_connection()
        .execute("DELETE FROM fuel_types WHERE id = ?1 AND is_default = 0", params![fuel_type_id])
        .map_err(|e| format!("Failed to delete fuel type: {}", e))?;

    Ok(())
}
