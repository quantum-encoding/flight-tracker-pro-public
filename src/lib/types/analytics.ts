// src/lib/types/analytics.ts
// Exact copy from the contract (slightly cleaned up for real usage)

export type Granularity = 'month' | 'quarter' | 'year';
export type RankBy = 'flights' | 'distance' | 'co2' | 'hours';
export type FuelUnit = 'liter' | 'gallon' | 'imperial_gallon';

export interface TemporalFlightData {
  period: string;              // e.g. "2024-01" or "2024-Q1"
  flight_count: number;
  total_distance_km: number;
  total_co2_kg: number;
  period_start: string;        // ISO date for correct sorting
}

export interface AirportVisitData {
  airport_code: string;
  airport_name: string;
  total_visits: number;
  departure_count: number;
  arrival_count: number;
  location: string;
}

export interface PassengerNode {
  id: string;
  label: string;
  total_flights: number;
  total_distance_km: number;
  // d3 simulation properties (added dynamically)
  x?: number;
  y?: number;
  fx?: number | null;
  fy?: number | null;
}

export interface PassengerEdge {
  source: string;
  target: string;
  flight_count: number;
  routes: string[];
}

export interface PassengerNetworkData {
  nodes: PassengerNode[];
  edges: PassengerEdge[];
}

export interface PassengerMetrics {
  abbreviation: string;
  full_name: string | null;
  total_flights: number;
  total_distance_km: number;
  total_co2_kg: number;
  total_flight_hours: number;
  unique_airports: number;
  avg_flight_distance_km: number;
}

export interface FuelPrice {
  id: string;
  user_id: string;
  fuel_type: string;
  price_per_unit: number;
  unit: FuelUnit;
  currency: string;
  start_date: string;
  end_date: string | null;
  location: string | null;
  supplier: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface FuelCostCalculation {
  flight_id: string;
  estimated_fuel_liters: number;
  fuel_price: FuelPrice;
  total_cost: number;
  calculation_method: string;
}

// ===== ADDITIONAL ANALYTICS TYPES =====

export interface AircraftUtilization {
  tail_number: string;
  aircraft_type: string;
  total_flights: number;
  total_hours: number;
  total_distance_km: number;
  avg_flight_hours: number;
  last_flown: string | null;
  days_since_last_flight: number | null;
}

export interface CostBreakdown {
  category: string;
  total_cost: number;
  currency: string;
  item_count: number;
}

export interface DayNightStats {
  total_day_flights: number;
  total_night_flights: number;
  day_hours: number;
  night_hours: number;
  night_landings: number;
  day_landings: number;
}

export interface LongHaulFlight {
  flight_id: string;
  departure_airport: string;
  arrival_airport: string;
  distance_km: number;
  flight_duration: number | null;
  departure_datetime: string;
  aircraft_type: string | null;
}

export interface CurrencyItem {
  requirement: string;
  current_count: number;
  required: number;
  expires_in_days: number | null;
  status: string;
  last_completed: string | null;
}

export interface MonthlyCostData {
  period: string;
  total_cost: number;
  total_hours: number;
  cost_per_hour: number;
  fuel_cost: number;
  other_costs: number;
  currency: string;
}

export interface RunwayRiskData {
  airport_code: string;
  airport_name: string;
  runway_length_ft: number;
  visits: number;
  risk_level: string;
}
