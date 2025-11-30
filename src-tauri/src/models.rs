// Data models for Flight Tracker Pro

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub pilot_license_number: Option<String>,
    pub license_type: Option<String>,
    pub license_country: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Airport {
    pub id: String,
    pub icao_code: Option<String>,
    pub iata_code: Option<String>,
    pub name: String,
    pub city: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportInput {
    pub icao_code: Option<String>,
    pub iata_code: Option<String>,
    pub name: String,
    pub city: Option<String>,
    pub country: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftType {
    pub id: String,
    pub manufacturer: String,
    pub model: String,
    pub type_designator: Option<String>,
    pub category: Option<String>,
    pub class: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftTypeInput {
    pub manufacturer: String,
    pub model: String,
    pub type_designator: Option<String>,
    pub category: Option<String>,
    pub class: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    pub id: String,
    pub user_id: String,
    pub flight_number: Option<String>,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub departure_datetime: String,
    pub arrival_datetime: Option<String>,
    pub aircraft_type_id: Option<String>,
    pub aircraft_registration: Option<String>,
    pub total_duration: Option<i32>,
    pub flight_duration: Option<i32>,
    pub block_duration: Option<i32>,
    pub distance_nm: Option<f64>,
    pub distance_km: Option<f64>,
    pub booking_reference: Option<String>,
    pub ticket_number: Option<String>,
    pub seat_number: Option<String>,
    pub fare_class: Option<String>,
    pub base_fare: Option<f64>,
    pub taxes: Option<f64>,
    pub total_cost: Option<f64>,
    pub currency: Option<String>,
    pub carbon_emissions_kg: Option<f64>,
    pub per_passenger_co2_kg: Option<f64>,
    pub carbon_offset_purchased: i32,
    pub frequent_flyer_program: Option<String>,
    pub miles_earned: Option<f64>,
    pub notes: Option<String>,
    pub attachment_path: Option<String>,
    pub data_source: String,
    pub verified: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightInput {
    pub flight_number: Option<String>,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub departure_datetime: String,
    pub arrival_datetime: Option<String>,
    pub aircraft_type_id: Option<String>,
    pub aircraft_registration: Option<String>,
    pub total_duration: Option<i32>,
    pub flight_duration: Option<i32>,
    pub distance_nm: Option<f64>,
    pub distance_km: Option<f64>,
    pub carbon_emissions_kg: Option<f64>,
    pub booking_reference: Option<String>,
    pub ticket_number: Option<String>,
    pub seat_number: Option<String>,
    pub fare_class: Option<String>,
    pub base_fare: Option<f64>,
    pub taxes: Option<f64>,
    pub total_cost: Option<f64>,
    pub currency: Option<String>,
    pub notes: Option<String>,
    pub attachment_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PilotLogbook {
    pub id: String,
    pub flight_id: String,
    pub pic_time: f64,
    pub sic_time: f64,
    pub dual_time: f64,
    pub instructor_time: f64,
    pub solo_time: f64,
    pub cross_country_time: f64,
    pub day_time: f64,
    pub night_time: f64,
    pub actual_instrument_time: f64,
    pub simulated_instrument_time: f64,
    pub ground_trainer_time: f64,
    pub day_takeoffs: i32,
    pub day_landings: i32,
    pub night_takeoffs: i32,
    pub night_landings: i32,
    pub ils_approaches: i32,
    pub vor_approaches: i32,
    pub ndb_approaches: i32,
    pub gps_approaches: i32,
    pub visual_approaches: i32,
    pub ifr_time: f64,
    pub vfr_time: f64,
    pub pilot_name: Option<String>,
    pub copilot_name: Option<String>,
    pub instructor_name: Option<String>,
    pub route: Option<String>,
    pub remarks: Option<String>,
    pub endorsements: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Journey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub is_favorite: i32,
    pub thumbnail_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrequentFlyerProgram {
    pub id: String,
    pub user_id: String,
    pub program_name: String,
    pub airline: Option<String>,
    pub alliance: Option<String>,
    pub member_number: Option<String>,
    pub tier_status: Option<String>,
    pub current_miles: f64,
    pub lifetime_miles: f64,
    pub tier_miles: f64,
    pub tier_expiry_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FFPInput {
    pub user_id: String,
    pub program_name: String,
    pub airline: Option<String>,
    pub alliance: Option<String>,
    pub member_number: Option<String>,
    pub tier_status: Option<String>,
    pub current_miles: f64,
    pub lifetime_miles: f64,
    pub tier_miles: f64,
    pub tier_expiry_date: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightStatistics {
    pub total_flights: i32,
    pub total_distance_nm: f64,
    pub total_distance_km: f64,
    pub total_flight_time_hours: f64,
    pub countries_visited: i32,
    pub airports_visited: i32,
    pub total_carbon_kg: f64,
    pub favorite_aircraft: Option<String>,
    pub favorite_route: Option<String>,
}

// ===== INVESTIGATION MODELS =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Investigation {
    pub id: String,
    pub flight_id: String,
    pub user_id: String,
    pub passenger_names: String,
    pub location: String,
    pub investigation_date: String,
    pub generated_queries: Option<String>,
    pub status: String,
    pub ai_summary: Option<String>,
    pub sources_json: Option<String>,
    pub corroboration_score: Option<f64>,
    pub error_message: Option<String>,
    pub processing_time_ms: Option<i32>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationSource {
    pub title: String,
    pub url: String,
    pub excerpt: String,
    pub relevance_score: f64,
    pub publication_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationResult {
    pub investigation_id: String,
    pub status: String,
    pub ai_summary: String,
    pub sources: Vec<InvestigationSource>,
    pub corroboration_score: f64,
    pub generated_queries: Vec<String>,
    pub processing_time_ms: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationRequest {
    pub flight_id: String,
    pub passenger_names: Vec<String>,
}

// ===== RESEARCH REPORTS MODELS =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReport {
    pub id: String,
    pub user_id: String,
    pub agent_name: String,
    pub agent_model: Option<String>,
    pub search_query: String,
    pub research_topics: Option<String>,
    pub report_summary: String,
    pub report_details: Option<String>,
    pub sources: Option<String>,
    pub confidence_score: Option<f64>,
    pub flight_id: Option<String>,
    pub report_type: String,
    pub processing_time_ms: Option<i32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReportInput {
    pub agent_name: String,
    pub agent_model: Option<String>,
    pub search_query: String,
    pub research_topics: Option<Vec<String>>,
    pub report_summary: String,
    pub report_details: Option<String>,
    pub sources: Option<Vec<ResearchSource>>,
    pub confidence_score: Option<f64>,
    pub flight_id: Option<String>,
    pub report_type: Option<String>,
    pub processing_time_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSource {
    pub title: String,
    pub url: Option<String>,
    pub snippet: String,
}

// ===== PILOT LOGBOOK TOTALS AND INPUT =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PilotLogbookInput {
    pub flight_id: String,
    pub pic_time: f64,
    pub sic_time: f64,
    pub dual_time: f64,
    pub instructor_time: f64,
    pub solo_time: f64,
    pub cross_country_time: f64,
    pub day_time: f64,
    pub night_time: f64,
    pub actual_instrument_time: f64,
    pub simulated_instrument_time: f64,
    pub ground_trainer_time: f64,
    pub day_takeoffs: i32,
    pub day_landings: i32,
    pub night_takeoffs: i32,
    pub night_landings: i32,
    pub ils_approaches: i32,
    pub vor_approaches: i32,
    pub ndb_approaches: i32,
    pub gps_approaches: i32,
    pub visual_approaches: i32,
    pub ifr_time: f64,
    pub vfr_time: f64,
    pub pilot_name: Option<String>,
    pub copilot_name: Option<String>,
    pub instructor_name: Option<String>,
    pub route: Option<String>,
    pub remarks: Option<String>,
    pub endorsements: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PilotLogbookTotals {
    pub total_pic_time: f64,
    pub total_sic_time: f64,
    pub total_dual_time: f64,
    pub total_instructor_time: f64,
    pub total_solo_time: f64,
    pub total_cross_country_time: f64,
    pub total_day_time: f64,
    pub total_night_time: f64,
    pub total_actual_instrument_time: f64,
    pub total_simulated_instrument_time: f64,
    pub total_ground_trainer_time: f64,
    pub total_day_takeoffs: i32,
    pub total_day_landings: i32,
    pub total_night_takeoffs: i32,
    pub total_night_landings: i32,
    pub total_ils_approaches: i32,
    pub total_vor_approaches: i32,
    pub total_ndb_approaches: i32,
    pub total_gps_approaches: i32,
    pub total_visual_approaches: i32,
    pub total_ifr_time: f64,
    pub total_vfr_time: f64,
}

// ===== CUSTOM DOCUMENTS MODELS =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDocument {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub flight_id: Option<String>,
    pub journey_id: Option<String>,
    pub passenger_name: Option<String>,
    pub fuel_entry_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDocumentInput {
    pub title: String,
    pub content: String,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub flight_id: Option<String>,
    pub journey_id: Option<String>,
    pub passenger_name: Option<String>,
    pub fuel_entry_id: Option<String>,
}

// ===== FUEL PRICE MODELS =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelPrice {
    pub id: String,
    pub user_id: String,
    pub fuel_type: String,
    pub price_per_unit: f64,
    pub unit: String,
    pub currency: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub location: Option<String>,
    pub supplier: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelPriceInput {
    pub fuel_type: String,
    pub price_per_unit: f64,
    pub unit: String,
    pub currency: String,
    pub start_date: String,
    pub end_date: Option<String>,
    pub location: Option<String>,
    pub supplier: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelCostCalculation {
    pub flight_id: String,
    pub estimated_fuel_liters: f64,
    pub fuel_price: FuelPrice,
    pub total_cost: f64,
    pub calculation_method: String,
}

// ===== ANALYTICS CHART MODELS (EXACT CONTRACT MATCH) =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalFlightData {
    pub period: String,
    pub flight_count: i64,
    pub total_distance_km: f64,
    pub total_co2_kg: f64,
    pub period_start: String, // ISO 8601 start date for sorting
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAnalysisRequest {
    pub user_id: String,
    pub granularity: String, // "month", "quarter", "year"
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AirportVisitData {
    pub airport_code: String,
    pub airport_name: String,
    pub total_visits: i64,
    pub departure_count: i64,
    pub arrival_count: i64,
    pub location: String, // "City, Country"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeospatialAnalysisRequest {
    pub user_id: String,
    pub limit: i64,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerNode {
    pub id: String,        // Passenger abbreviation
    pub label: String,     // Display name (full_name or abbreviation)
    pub total_flights: i64,
    pub total_distance_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerEdge {
    pub source: String,
    pub target: String,
    pub flight_count: i64,
    pub routes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerNetworkData {
    pub nodes: Vec<PassengerNode>,
    pub edges: Vec<PassengerEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAnalysisRequest {
    pub user_id: String,
    #[serde(default)]
    pub min_flights_together: Option<i64>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerMetrics {
    pub abbreviation: String,
    pub full_name: Option<String>,
    pub total_flights: i64,
    pub total_distance_km: f64,
    pub total_co2_kg: f64,
    pub total_flight_hours: f64,
    pub unique_airports: i64,
    pub avg_flight_distance_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeMetricsRequest {
    pub user_id: String,
    pub rank_by: String, // "flights", "distance", "co2", "hours"
    pub limit: i64,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
}

// ===== ADDITIONAL ANALYTICS MODELS =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AircraftUtilization {
    pub tail_number: String,
    pub aircraft_type: String,
    pub total_flights: i64,
    pub total_hours: f64,
    pub total_distance_km: f64,
    pub avg_flight_hours: f64,
    pub last_flown: Option<String>,   // ISO date, can be NULL
    pub days_since_last_flight: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub category: String,             // fuel | maintenance | landing_fees | handling | crew | other
    pub total_cost: f64,
    pub currency: String,
    pub item_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayNightStats {
    pub total_day_flights: i64,
    pub total_night_flights: i64,
    pub day_hours: f64,
    pub night_hours: f64,
    pub night_landings: i64,
    pub day_landings: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongHaulFlight {
    pub flight_id: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub distance_km: f64,
    pub flight_duration: Option<f64>,
    pub departure_datetime: String,
    pub aircraft_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyItem {
    pub requirement: String,
    pub current_count: i64,
    pub required: i64,
    pub expires_in_days: Option<i64>,
    pub status: String,               // good | warning | expired
    pub last_completed: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyCostData {
    pub period: String,               // 2025-01
    pub total_cost: f64,
    pub total_hours: f64,
    pub cost_per_hour: f64,
    pub fuel_cost: f64,
    pub other_costs: f64,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunwayRiskData {
    pub airport_code: String,
    pub airport_name: String,
    pub runway_length_ft: i32,
    pub visits: i64,
    pub risk_level: String,           // safe | marginal | watch_out | nope
}
