// Calculation-related commands

#[tauri::command]
pub fn calculate_distance(
    lat1: f64,
    lon1: f64,
    lat2: f64,
    lon2: f64,
) -> Result<serde_json::Value, String> {
    let (distance_nm, distance_km) = crate::geo::calculate_distance(lat1, lon1, lat2, lon2);

    Ok(serde_json::json!({
        "distance_nm": distance_nm,
        "distance_km": distance_km
    }))
}

#[tauri::command]
pub fn calculate_co2_emissions(distance_km: f64, aircraft_type: Option<String>) -> f64 {
    crate::calculations::calculate_co2_emissions(
        distance_km,
        aircraft_type.as_deref(),
    )
}

#[tauri::command]
pub fn calculate_per_passenger_co2(
    distance_km: f64,
    aircraft_type: Option<String>,
    passenger_count: Option<u32>,
) -> f64 {
    crate::calculations::calculate_per_passenger_co2(
        distance_km,
        aircraft_type.as_deref(),
        passenger_count,
    )
}

#[tauri::command]
pub fn calculate_flight_duration(distance_km: f64, aircraft_type: Option<String>) -> i32 {
    crate::calculations::calculate_flight_time(
        distance_km,
        aircraft_type.as_deref(),
    )
}

#[tauri::command]
pub fn fuel_kg_to_liters(fuel_kg: f64) -> f64 {
    crate::calculations::fuel_kg_to_liters(fuel_kg)
}

#[tauri::command]
pub fn fuel_kg_to_gallons(fuel_kg: f64) -> f64 {
    crate::calculations::fuel_kg_to_gallons(fuel_kg)
}
