// Fuel price management commands
use tauri::State;

use super::AppState;

// ===== FUEL PRICE COMMANDS =====

#[tauri::command]
pub fn create_fuel_price(
    user_id: String,
    fuel_price: crate::models::FuelPriceInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.create_fuel_price(
        &user_id,
        &fuel_price.fuel_type,
        fuel_price.price_per_unit,
        &fuel_price.unit,
        &fuel_price.currency,
        &fuel_price.start_date,
        fuel_price.end_date.as_deref(),
        fuel_price.location.as_deref(),
        fuel_price.supplier.as_deref(),
        fuel_price.notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_fuel_price(
    fuel_price_id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::FuelPrice>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_fuel_price(&fuel_price_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_fuel_prices(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::models::FuelPrice>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_fuel_prices(&user_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_fuel_price_for_date(
    user_id: String,
    date: String,
    fuel_type: Option<String>,
    location: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::FuelPrice>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_fuel_price_for_date(
        &user_id,
        &date,
        fuel_type.as_deref(),
        location.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_fuel_price(
    fuel_price_id: String,
    fuel_price: crate::models::FuelPriceInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.update_fuel_price(
        &fuel_price_id,
        &fuel_price.fuel_type,
        fuel_price.price_per_unit,
        &fuel_price.unit,
        &fuel_price.currency,
        &fuel_price.start_date,
        fuel_price.end_date.as_deref(),
        fuel_price.location.as_deref(),
        fuel_price.supplier.as_deref(),
        fuel_price.notes.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_fuel_price(
    fuel_price_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_fuel_price(&fuel_price_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn calculate_fuel_cost(
    user_id: String,
    flight_id: String,
    fuel_consumed_kg: f64,
    date: String,
    fuel_type: Option<String>,
    location: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<crate::models::FuelCostCalculation>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get fuel price for the date
    let fuel_price = db
        .get_fuel_price_for_date(
            &user_id,
            &date,
            fuel_type.as_deref(),
            location.as_deref(),
        )
        .map_err(|e| e.to_string())?;

    if let Some(price) = fuel_price {
        // Convert kg to liters based on fuel density
        let density = if price.fuel_type.contains("Avgas") {
            0.72 // Avgas density: ~0.72 kg/L
        } else {
            0.8 // Jet A density: ~0.8 kg/L
        };
        let estimated_fuel_liters = fuel_consumed_kg / density;

        // Calculate total cost based on price unit
        let total_cost = match price.unit.as_str() {
            "liter" => estimated_fuel_liters * price.price_per_unit,
            "gallon" => {
                let gallons = estimated_fuel_liters / 3.78541; // Convert to US gallons
                gallons * price.price_per_unit
            }
            "imperial_gallon" => {
                let imp_gallons = estimated_fuel_liters / 4.54609; // Convert to imperial gallons
                imp_gallons * price.price_per_unit
            }
            _ => estimated_fuel_liters * price.price_per_unit, // Default: treat as liters
        };

        let calculation_method = format!(
            "Estimated {} kg fuel @ {} {}/{} = {:.2} {}",
            fuel_consumed_kg,
            price.price_per_unit,
            price.currency,
            price.unit,
            total_cost,
            price.currency
        );

        Ok(Some(crate::models::FuelCostCalculation {
            flight_id,
            estimated_fuel_liters,
            fuel_price: price,
            total_cost,
            calculation_method,
        }))
    } else {
        Ok(None)
    }
}
