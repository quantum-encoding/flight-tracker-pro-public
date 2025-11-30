// Analytics chart data aggregation commands
use tauri::State;

use super::AppState;

// Helper function to sanitize floats (replace NaN/Infinity with 0.0)
fn sanitize_f64(value: f64) -> f64 {
    if value.is_nan() || value.is_infinite() {
        0.0
    } else {
        value
    }
}

// ===== ANALYTICS COMMANDS =====

#[tauri::command]
pub fn get_temporal_analysis(
    request: crate::models::TemporalAnalysisRequest,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::TemporalFlightData>, String> {
    eprintln!("[DEBUG] get_temporal_analysis called");
    let db = state.db.lock().map_err(|e| {
        eprintln!("[ERROR] get_temporal_analysis: DB lock failed: {}", e);
        e.to_string()
    })?;

    let mut result = db.get_temporal_flight_data(
        &request.user_id,
        &request.granularity,
        request.start_date.as_deref(),
        request.end_date.as_deref(),
    )
    .map_err(|e| {
        eprintln!("[ERROR] get_temporal_analysis: Query failed: {}", e);
        e.to_string()
    })?;

    // Sanitize float values
    for item in &mut result {
        item.total_distance_km = sanitize_f64(item.total_distance_km);
        item.total_co2_kg = sanitize_f64(item.total_co2_kg);
    }

    eprintln!("[DEBUG] get_temporal_analysis returning {} items", result.len());
    // Try to serialize to catch serialization errors
    match serde_json::to_string(&result) {
        Ok(_) => eprintln!("[DEBUG] get_temporal_analysis: Serialization test passed"),
        Err(e) => {
            eprintln!("[ERROR] get_temporal_analysis: Serialization failed: {}", e);
            return Err(format!("Serialization error: {}", e));
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn get_geospatial_analysis(
    request: crate::models::GeospatialAnalysisRequest,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::AirportVisitData>, String> {
    eprintln!("[DEBUG] get_geospatial_analysis called");
    let db = state.db.lock().map_err(|e| {
        eprintln!("[ERROR] get_geospatial_analysis: DB lock failed: {}", e);
        e.to_string()
    })?;

    let result = db.get_airport_visit_data(
        &request.user_id,
        request.limit,
        request.start_date.as_deref(),
        request.end_date.as_deref(),
    )
    .map_err(|e| {
        eprintln!("[ERROR] get_geospatial_analysis: Query failed: {}", e);
        e.to_string()
    })?;

    eprintln!("[DEBUG] get_geospatial_analysis returning {} items", result.len());

    // Try to serialize to catch serialization errors
    match serde_json::to_string(&result) {
        Ok(_) => eprintln!("[DEBUG] get_geospatial_analysis: Serialization test passed"),
        Err(e) => {
            eprintln!("[ERROR] get_geospatial_analysis: Serialization failed: {}", e);
            return Err(format!("Serialization error: {}", e));
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn get_passenger_network(
    request: crate::models::NetworkAnalysisRequest,
    state: State<'_, AppState>,
) -> Result<crate::models::PassengerNetworkData, String> {
    eprintln!("[DEBUG] get_passenger_network called");
    let db = state.db.lock().map_err(|e| {
        eprintln!("[ERROR] get_passenger_network: DB lock failed: {}", e);
        e.to_string()
    })?;
    let min_shared_flights = request.min_flights_together.unwrap_or(1);

    let mut result = db.get_passenger_network_data(
        &request.user_id,
        min_shared_flights,
        request.start_date.as_deref(),
        request.end_date.as_deref(),
    )
    .map_err(|e| {
        eprintln!("[ERROR] get_passenger_network: Query failed: {}", e);
        e.to_string()
    })?;

    // Sanitize float values in nodes
    for node in &mut result.nodes {
        node.total_distance_km = sanitize_f64(node.total_distance_km);
    }

    eprintln!("[DEBUG] get_passenger_network returning {} nodes, {} edges", result.nodes.len(), result.edges.len());
    match serde_json::to_string(&result) {
        Ok(_) => eprintln!("[DEBUG] get_passenger_network: Serialization test passed"),
        Err(e) => {
            eprintln!("[ERROR] get_passenger_network: Serialization failed: {}", e);
            return Err(format!("Serialization error: {}", e));
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn get_comparative_metrics(
    request: crate::models::ComparativeMetricsRequest,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::PassengerMetrics>, String> {
    eprintln!("[DEBUG] get_comparative_metrics called");
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut result = db.get_passenger_metrics(
        &request.user_id,
        &request.rank_by,
        request.limit,
        request.start_date.as_deref(),
        request.end_date.as_deref(),
    )
    .map_err(|e| {
        eprintln!("[ERROR] get_comparative_metrics: Query failed: {}", e);
        e.to_string()
    })?;

    // Sanitize float values
    for item in &mut result {
        item.total_distance_km = sanitize_f64(item.total_distance_km);
        item.total_co2_kg = sanitize_f64(item.total_co2_kg);
        item.total_flight_hours = sanitize_f64(item.total_flight_hours);
        item.avg_flight_distance_km = sanitize_f64(item.avg_flight_distance_km);
    }

    eprintln!("[DEBUG] get_comparative_metrics returning {} items", result.len());
    match serde_json::to_string(&result) {
        Ok(_) => eprintln!("[DEBUG] get_comparative_metrics: Serialization test passed"),
        Err(e) => {
            eprintln!("[ERROR] get_comparative_metrics: Serialization failed: {}", e);
            return Err(format!("Serialization error: {}", e));
        }
    }

    Ok(result)
}

// ===== ADDITIONAL ANALYTICS COMMANDS =====

#[tauri::command]
pub fn get_aircraft_utilization(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::AircraftUtilization>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut result = db.get_aircraft_utilization(&user_id)
        .map_err(|e| e.to_string())?;

    // Sanitize float values to prevent JSON serialization errors
    for item in &mut result {
        if item.total_hours.is_nan() || item.total_hours.is_infinite() {
            item.total_hours = 0.0;
        }
        if item.total_distance_km.is_nan() || item.total_distance_km.is_infinite() {
            item.total_distance_km = 0.0;
        }
        if item.avg_flight_hours.is_nan() || item.avg_flight_hours.is_infinite() {
            item.avg_flight_hours = 0.0;
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn get_cost_breakdown(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::CostBreakdown>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut result = db.get_cost_breakdown(&user_id)
        .map_err(|e| e.to_string())?;

    // Sanitize float values
    for item in &mut result {
        item.total_cost = sanitize_f64(item.total_cost);
    }

    Ok(result)
}

#[tauri::command]
pub fn get_day_night_stats(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<crate::models::DayNightStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut result = db.get_day_night_stats(&user_id)
        .map_err(|e| e.to_string())?;

    // Sanitize float values
    result.day_hours = sanitize_f64(result.day_hours);
    result.night_hours = sanitize_f64(result.night_hours);

    Ok(result)
}

#[tauri::command]
pub fn get_long_haul_flights(
    user_id: String,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::LongHaulFlight>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut result = db.get_long_haul_flights(&user_id, limit.unwrap_or(10))
        .map_err(|e| e.to_string())?;

    // Sanitize float values
    for item in &mut result {
        item.distance_km = sanitize_f64(item.distance_km);
        if let Some(duration) = item.flight_duration {
            item.flight_duration = Some(sanitize_f64(duration));
        }
    }

    Ok(result)
}

#[tauri::command]
pub fn get_pilot_currency(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::CurrencyItem>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_pilot_currency(&user_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_monthly_cost_trend(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::MonthlyCostData>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut result = db.get_monthly_cost_trend(&user_id)
        .map_err(|e| e.to_string())?;

    // Sanitize float values
    for item in &mut result {
        item.total_cost = sanitize_f64(item.total_cost);
        item.total_hours = sanitize_f64(item.total_hours);
        item.cost_per_hour = sanitize_f64(item.cost_per_hour);
        item.fuel_cost = sanitize_f64(item.fuel_cost);
        item.other_costs = sanitize_f64(item.other_costs);
    }

    Ok(result)
}

#[tauri::command]
pub fn get_runway_risk_data(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::RunwayRiskData>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_runway_risk_data(&user_id)
        .map_err(|e| e.to_string())
}
