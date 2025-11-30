// Aviation Calculations - Fuel consumption and CO2 emissions
// Based on ICAO and aviation industry standards

/// Aircraft performance characteristics
/// Based on typical fuel consumption for common aircraft types
#[derive(Debug, Clone)]
pub struct AircraftPerformance {
    pub fuel_burn_rate_kg_per_km: f64, // Average fuel burn in kg per km
    pub co2_factor: f64,               // CO2 produced per kg of fuel (typically 3.15)
}

impl AircraftPerformance {
    /// Get default performance characteristics for unknown aircraft
    /// Uses average of narrow-body jet (like 737/A320)
    pub fn default() -> Self {
        Self {
            fuel_burn_rate_kg_per_km: 3.5, // ~3.5 kg/km for typical narrow-body
            co2_factor: 3.15,              // 1 kg Jet-A1 = 3.15 kg CO2
        }
    }

    /// Get performance characteristics by aircraft type
    pub fn from_aircraft_type(aircraft_type: &str) -> Self {
        let aircraft_upper = aircraft_type.to_uppercase();

        // Match common aircraft types with their fuel consumption profiles
        // Values are approximate averages based on cruise conditions

        if aircraft_upper.contains("747") || aircraft_upper.contains("A380") {
            // Large wide-body (4 engines)
            Self {
                fuel_burn_rate_kg_per_km: 12.0,
                co2_factor: 3.15,
            }
        } else if aircraft_upper.contains("777")
            || aircraft_upper.contains("787")
            || aircraft_upper.contains("A350")
            || aircraft_upper.contains("330")
        {
            // Modern wide-body (2 engines)
            Self {
                fuel_burn_rate_kg_per_km: 8.0,
                co2_factor: 3.15,
            }
        } else if aircraft_upper.contains("737")
            || aircraft_upper.contains("A320")
            || aircraft_upper.contains("A319")
            || aircraft_upper.contains("A321")
        {
            // Narrow-body jets
            Self {
                fuel_burn_rate_kg_per_km: 3.5,
                co2_factor: 3.15,
            }
        } else if aircraft_upper.contains("ERJ")
            || aircraft_upper.contains("CRJ")
            || aircraft_upper.contains("E170")
            || aircraft_upper.contains("E190")
        {
            // Regional jets
            Self {
                fuel_burn_rate_kg_per_km: 2.5,
                co2_factor: 3.15,
            }
        } else if aircraft_upper.contains("CESSNA")
            || aircraft_upper.contains("C172")
            || aircraft_upper.contains("C182")
            || aircraft_upper.contains("PA-")
            || aircraft_upper.contains("PIPER")
        {
            // General aviation piston
            Self {
                fuel_burn_rate_kg_per_km: 0.4,
                co2_factor: 3.15,
            }
        } else if aircraft_upper.contains("CITATION")
            || aircraft_upper.contains("GULFSTREAM")
            || aircraft_upper.contains("LEARJET")
            || aircraft_upper.contains("FALCON")
            || aircraft_upper.contains("CHALLENGER")
            || aircraft_upper.contains("HS125")
        {
            // Business jets
            Self {
                fuel_burn_rate_kg_per_km: 1.8,
                co2_factor: 3.15,
            }
        } else {
            // Default to narrow-body average
            Self::default()
        }
    }
}

/// Calculate fuel consumption for a flight
///
/// # Arguments
/// * `distance_km` - Flight distance in kilometers
/// * `aircraft_type` - Optional aircraft type string (e.g., "Boeing 737", "A320")
///
/// # Returns
/// Fuel consumption in kilograms
pub fn calculate_fuel_consumption(distance_km: f64, aircraft_type: Option<&str>) -> f64 {
    let performance = match aircraft_type {
        Some(ac) => AircraftPerformance::from_aircraft_type(ac),
        None => AircraftPerformance::default(),
    };

    // Base fuel calculation
    let mut fuel_kg = distance_km * performance.fuel_burn_rate_kg_per_km;

    // Add takeoff and landing overhead (approximately 10% for jet aircraft)
    fuel_kg *= 1.10;

    // Add taxi fuel (approximately 100 kg for jets, 20 kg for GA)
    let taxi_fuel = if performance.fuel_burn_rate_kg_per_km > 2.0 {
        100.0 // Jet aircraft
    } else {
        20.0 // General aviation
    };

    fuel_kg + taxi_fuel
}

/// Calculate CO2 emissions for a flight
///
/// # Arguments
/// * `distance_km` - Flight distance in kilometers
/// * `aircraft_type` - Optional aircraft type string
///
/// # Returns
/// CO2 emissions in kilograms
pub fn calculate_co2_emissions(distance_km: f64, aircraft_type: Option<&str>) -> f64 {
    let fuel_kg = calculate_fuel_consumption(distance_km, aircraft_type);

    let performance = match aircraft_type {
        Some(ac) => AircraftPerformance::from_aircraft_type(ac),
        None => AircraftPerformance::default(),
    };

    fuel_kg * performance.co2_factor
}

/// Calculate per-passenger CO2 emissions
///
/// # Arguments
/// * `distance_km` - Flight distance in kilometers
/// * `aircraft_type` - Optional aircraft type string
/// * `passenger_count` - Number of passengers (for load factor calculation)
///
/// # Returns
/// CO2 emissions per passenger in kilograms
pub fn calculate_per_passenger_co2(
    distance_km: f64,
    aircraft_type: Option<&str>,
    passenger_count: Option<u32>,
) -> f64 {
    let total_co2 = calculate_co2_emissions(distance_km, aircraft_type);

    // Use typical load factors if passenger count not provided
    let passengers = passenger_count.unwrap_or_else(|| {
        let ac_type = aircraft_type.unwrap_or("");
        if ac_type.to_uppercase().contains("747") || ac_type.to_uppercase().contains("A380") {
            350 // Large wide-body
        } else if ac_type.to_uppercase().contains("777") || ac_type.to_uppercase().contains("787") {
            250 // Medium wide-body
        } else if ac_type.to_uppercase().contains("737") || ac_type.to_uppercase().contains("A320")
        {
            150 // Narrow-body
        } else {
            100 // Default/regional
        }
    });

    total_co2 / passengers as f64
}

/// Convert fuel from kg to liters (Jet-A1 density ~0.8 kg/L)
pub fn fuel_kg_to_liters(fuel_kg: f64) -> f64 {
    fuel_kg / 0.8
}

/// Convert fuel from kg to gallons (US gallons, Jet-A1 ~6.7 lbs/gal)
pub fn fuel_kg_to_gallons(fuel_kg: f64) -> f64 {
    fuel_kg * 2.20462 / 6.7 // kg to lbs, then lbs to gallons
}

/// Calculate estimated flight time based on distance and aircraft type
///
/// # Arguments
/// * `distance_km` - Flight distance in kilometers
/// * `aircraft_type` - Optional aircraft type string
///
/// # Returns
/// Flight duration in minutes (includes taxi, climb, cruise, descent)
pub fn calculate_flight_time(distance_km: f64, aircraft_type: Option<&str>) -> i32 {
    let aircraft_upper = aircraft_type.unwrap_or("").to_uppercase();

    // Determine cruise speed based on aircraft type (km/h)
    let cruise_speed_kmh = if aircraft_upper.contains("747") || aircraft_upper.contains("A380") {
        910.0 // Large wide-body
    } else if aircraft_upper.contains("777")
        || aircraft_upper.contains("787")
        || aircraft_upper.contains("A350")
        || aircraft_upper.contains("330")
    {
        905.0 // Modern wide-body
    } else if aircraft_upper.contains("737")
        || aircraft_upper.contains("A320")
        || aircraft_upper.contains("A319")
        || aircraft_upper.contains("A321")
    {
        850.0 // Narrow-body jets
    } else if aircraft_upper.contains("ERJ")
        || aircraft_upper.contains("CRJ")
        || aircraft_upper.contains("E170")
        || aircraft_upper.contains("E190")
    {
        820.0 // Regional jets
    } else if aircraft_upper.contains("CESSNA")
        || aircraft_upper.contains("C172")
        || aircraft_upper.contains("C182")
        || aircraft_upper.contains("PA-")
        || aircraft_upper.contains("PIPER")
    {
        200.0 // General aviation piston
    } else if aircraft_upper.contains("CITATION")
        || aircraft_upper.contains("GULFSTREAM")
        || aircraft_upper.contains("LEARJET")
        || aircraft_upper.contains("FALCON")
        || aircraft_upper.contains("CHALLENGER")
        || aircraft_upper.contains("HS125")
    {
        750.0 // Business jets
    } else {
        850.0 // Default to narrow-body average
    };

    // Calculate cruise time in hours
    let cruise_time_hours = distance_km / cruise_speed_kmh;

    // Add overhead for taxi, takeoff, climb, descent (varies by distance)
    let overhead_minutes = if distance_km < 500.0 {
        45.0 // Short flights: more overhead relative to cruise
    } else if distance_km < 1500.0 {
        35.0 // Medium flights
    } else if distance_km < 5000.0 {
        30.0 // Long flights
    } else {
        25.0 // Ultra-long flights: overhead is smaller percentage
    };

    // Total time in minutes
    let total_minutes = (cruise_time_hours * 60.0) + overhead_minutes;

    total_minutes.round() as i32
}

/// Calculate block time (gate-to-gate) which includes ground operations
///
/// # Arguments
/// * `flight_duration` - Flight duration in minutes
///
/// # Returns
/// Block duration in minutes (adds taxi time)
pub fn calculate_block_time(flight_duration: i32) -> i32 {
    // Add typical taxi time (10-15 minutes each end)
    flight_duration + 25
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuel_calculation_737() {
        // JFK to LAX is approximately 3983 km
        let distance_km = 3983.0;
        let fuel = calculate_fuel_consumption(distance_km, Some("Boeing 737"));

        // Expected: ~3983 * 3.5 * 1.1 + 100 = ~15,400 kg
        assert!(
            fuel > 14000.0 && fuel < 17000.0,
            "Fuel should be around 15,400 kg, got {}",
            fuel
        );
    }

    #[test]
    fn test_co2_calculation() {
        let distance_km = 1000.0;
        let co2 = calculate_co2_emissions(distance_km, Some("A320"));

        // CO2 should be roughly fuel * 3.15
        assert!(co2 > 10000.0, "CO2 for 1000km should be significant");
    }

    #[test]
    fn test_fuel_conversions() {
        let fuel_kg = 1000.0;
        let liters = fuel_kg_to_liters(fuel_kg);
        let gallons = fuel_kg_to_gallons(fuel_kg);

        assert!((liters - 1250.0).abs() < 1.0, "1000 kg should be ~1250 L");
        assert!((gallons - 330.0).abs() < 10.0, "1000 kg should be ~330 gal");
    }

    #[test]
    fn test_flight_time_calculation() {
        // JFK to LAX: ~3983 km
        // Expected: ~3983/850 * 60 + 30 = ~311 minutes (5h 11m)
        let time = calculate_flight_time(3983.0, Some("Boeing 737"));
        assert!(
            time >= 300 && time <= 330,
            "JFK-LAX should be ~5-5.5 hours, got {} minutes",
            time
        );

        // Short regional flight: 500 km
        // Expected: ~500/850 * 60 + 45 = ~80 minutes
        let short_time = calculate_flight_time(500.0, Some("ERJ"));
        assert!(
            short_time >= 70 && short_time <= 90,
            "500km should be ~1-1.5 hours, got {} minutes",
            short_time
        );

        // Ultra-long: 12000 km
        // Expected: ~12000/905 * 60 + 25 = ~820 minutes (13h 40m)
        let long_time = calculate_flight_time(12000.0, Some("777"));
        assert!(
            long_time >= 800 && long_time <= 850,
            "12000km should be ~13-14 hours, got {} minutes",
            long_time
        );
    }

    #[test]
    fn test_block_time() {
        let flight_duration = 300; // 5 hours
        let block = calculate_block_time(flight_duration);
        assert_eq!(block, 325, "Block time should add 25 minutes for taxi");
    }
}
