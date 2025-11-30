-- SQLite Database Schema for Flight Tracker Pro
-- Advanced flight management and tracking application

-- Application Settings Table
-- Stores user preferences and API keys
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Users Table
-- Stores pilot/traveler information
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT,
    pilot_license_number TEXT,
    license_type TEXT, -- PPL, CPL, ATPL, etc.
    license_country TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Airports Table
-- Cache of airport information
CREATE TABLE IF NOT EXISTS airports (
    id TEXT PRIMARY KEY, -- ICAO or IATA code
    icao_code TEXT,
    iata_code TEXT,
    name TEXT NOT NULL,
    city TEXT,
    country TEXT,
    latitude REAL,
    longitude REAL,
    timezone TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_airports_icao ON airports(icao_code);
CREATE INDEX IF NOT EXISTS idx_airports_iata ON airports(iata_code);

-- Aircraft Types Table
-- Catalog of aircraft models
CREATE TABLE IF NOT EXISTS aircraft_types (
    id TEXT PRIMARY KEY,
    manufacturer TEXT NOT NULL,
    model TEXT NOT NULL,
    type_designator TEXT, -- e.g., B738, A320
    category TEXT, -- SEL, MEL, SET, MET, etc.
    class TEXT, -- Airplane, Helicopter, Glider, etc.
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_aircraft_type_designator ON aircraft_types(type_designator);

-- Flights Table
-- Core flight records
CREATE TABLE IF NOT EXISTS flights (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,

    -- Flight Details
    flight_number TEXT, -- e.g., AA1234
    departure_airport TEXT NOT NULL,
    arrival_airport TEXT NOT NULL,
    departure_datetime TEXT NOT NULL,
    arrival_datetime TEXT,

    -- Aircraft Information
    aircraft_type_id TEXT,
    aircraft_registration TEXT, -- e.g., N12345

    -- Flight Duration (in minutes)
    total_duration INTEGER,
    flight_duration INTEGER, -- Actual flight time
    block_duration INTEGER, -- Block to block time

    -- Distance
    distance_nm REAL, -- Nautical miles
    distance_km REAL, -- Kilometers

    -- Booking Information
    booking_reference TEXT,
    ticket_number TEXT,
    seat_number TEXT,
    fare_class TEXT, -- Economy, Business, First, etc.

    -- Cost Information
    base_fare REAL,
    taxes REAL,
    total_cost REAL,
    currency TEXT DEFAULT 'USD',

    -- Environmental
    carbon_emissions_kg REAL,
    carbon_offset_purchased INTEGER DEFAULT 0,

    -- Frequent Flyer
    frequent_flyer_program TEXT,
    miles_earned REAL,

    -- Notes and Attachments
    notes TEXT,
    attachment_path TEXT, -- Path to boarding pass image, etc.

    -- Metadata
    data_source TEXT DEFAULT 'manual', -- manual, ocr, api
    verified INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (aircraft_type_id) REFERENCES aircraft_types(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_flights_user ON flights(user_id);
CREATE INDEX IF NOT EXISTS idx_flights_departure_date ON flights(departure_datetime);
CREATE INDEX IF NOT EXISTS idx_flights_departure_airport ON flights(departure_airport);
CREATE INDEX IF NOT EXISTS idx_flights_arrival_airport ON flights(arrival_airport);

-- Pilot Logbook Table
-- Detailed pilot-specific flight logging (FAA/EASA compliant)
CREATE TABLE IF NOT EXISTS pilot_logbook (
    id TEXT PRIMARY KEY,
    flight_id TEXT NOT NULL UNIQUE,

    -- Flight Times (in decimal hours)
    pic_time REAL DEFAULT 0.0, -- Pilot in Command
    sic_time REAL DEFAULT 0.0, -- Second in Command
    dual_time REAL DEFAULT 0.0, -- Dual instruction received
    instructor_time REAL DEFAULT 0.0, -- Flight instruction given
    solo_time REAL DEFAULT 0.0, -- Solo flight time
    cross_country_time REAL DEFAULT 0.0, -- Cross-country

    -- Conditions
    day_time REAL DEFAULT 0.0,
    night_time REAL DEFAULT 0.0,
    actual_instrument_time REAL DEFAULT 0.0,
    simulated_instrument_time REAL DEFAULT 0.0,
    ground_trainer_time REAL DEFAULT 0.0,

    -- Takeoffs and Landings
    day_takeoffs INTEGER DEFAULT 0,
    day_landings INTEGER DEFAULT 0,
    night_takeoffs INTEGER DEFAULT 0,
    night_landings INTEGER DEFAULT 0,

    -- Approach Types
    ils_approaches INTEGER DEFAULT 0,
    vor_approaches INTEGER DEFAULT 0,
    ndb_approaches INTEGER DEFAULT 0,
    gps_approaches INTEGER DEFAULT 0,
    visual_approaches INTEGER DEFAULT 0,

    -- Flight Rules
    ifr_time REAL DEFAULT 0.0,
    vfr_time REAL DEFAULT 0.0,

    -- Personnel
    pilot_name TEXT,
    copilot_name TEXT,
    instructor_name TEXT,

    -- Route and Remarks
    route TEXT, -- e.g., KJFK-KLAX via J230
    remarks TEXT,
    endorsements TEXT, -- Instructor endorsements

    -- Digitally signed for compliance
    digital_signature TEXT,
    signature_timestamp TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_logbook_flight ON pilot_logbook(flight_id);

-- Journeys Table
-- Groups of flights forming a trip
CREATE TABLE IF NOT EXISTS journeys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    start_date TEXT NOT NULL,
    end_date TEXT,
    is_favorite INTEGER DEFAULT 0,
    thumbnail_path TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_journeys_user ON journeys(user_id);
CREATE INDEX IF NOT EXISTS idx_journeys_start_date ON journeys(start_date);

-- Journey Flights Junction Table
-- Many-to-many relationship between journeys and flights
CREATE TABLE IF NOT EXISTS journey_flights (
    journey_id TEXT NOT NULL,
    flight_id TEXT NOT NULL,
    sequence_order INTEGER, -- Order of flight in journey
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    PRIMARY KEY (journey_id, flight_id),
    FOREIGN KEY (journey_id) REFERENCES journeys(id) ON DELETE CASCADE,
    FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_journey_flights_journey ON journey_flights(journey_id);
CREATE INDEX IF NOT EXISTS idx_journey_flights_flight ON journey_flights(flight_id);

-- Frequent Flyer Programs Table
-- Track loyalty programs and miles
CREATE TABLE IF NOT EXISTS frequent_flyer_programs (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    program_name TEXT NOT NULL, -- e.g., "United MileagePlus"
    airline TEXT, -- e.g., "United Airlines"
    alliance TEXT, -- Star Alliance, oneworld, SkyTeam
    member_number TEXT,
    tier_status TEXT, -- e.g., Silver, Gold, Platinum
    current_miles REAL DEFAULT 0.0,
    lifetime_miles REAL DEFAULT 0.0,
    tier_miles REAL DEFAULT 0.0,
    tier_expiry_date TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ffp_user ON frequent_flyer_programs(user_id);

-- OCR Processing Queue Table
-- Tracks bulk OCR operations
CREATE TABLE IF NOT EXISTS ocr_queue (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    file_name TEXT NOT NULL,
    status TEXT DEFAULT 'pending', -- pending, processing, completed, failed
    result_json TEXT, -- Extracted data as JSON
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    processed_at TEXT,

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ocr_queue_status ON ocr_queue(status);
CREATE INDEX IF NOT EXISTS idx_ocr_queue_user ON ocr_queue(user_id);

-- Statistics Cache Table
-- Pre-computed statistics for dashboard
CREATE TABLE IF NOT EXISTS statistics_cache (
    user_id TEXT PRIMARY KEY,
    total_flights INTEGER DEFAULT 0,
    total_distance_nm REAL DEFAULT 0.0,
    total_distance_km REAL DEFAULT 0.0,
    total_flight_time_hours REAL DEFAULT 0.0,
    countries_visited INTEGER DEFAULT 0,
    airports_visited INTEGER DEFAULT 0,
    total_carbon_kg REAL DEFAULT 0.0,
    favorite_aircraft TEXT,
    favorite_route TEXT,
    last_updated TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Investigations Table
-- AI-powered cross-referencing investigation cache
CREATE TABLE IF NOT EXISTS investigations (
    id TEXT PRIMARY KEY,
    flight_id TEXT NOT NULL,
    user_id TEXT NOT NULL,

    -- Investigation targets
    passenger_names TEXT NOT NULL, -- Comma-separated or JSON array
    location TEXT NOT NULL, -- Airport code or city name
    investigation_date TEXT NOT NULL, -- Date to investigate around

    -- AI-generated search queries
    generated_queries TEXT, -- JSON array of search queries

    -- Investigation results
    status TEXT DEFAULT 'pending', -- pending, processing, completed, failed
    ai_summary TEXT, -- AI-synthesized summary of findings
    sources_json TEXT, -- JSON array of sources with URLs, titles, excerpts
    corroboration_score REAL, -- 0.0-1.0 confidence score

    -- Metadata
    error_message TEXT,
    processing_time_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,

    FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_investigations_flight ON investigations(flight_id);
CREATE INDEX IF NOT EXISTS idx_investigations_user ON investigations(user_id);
CREATE INDEX IF NOT EXISTS idx_investigations_status ON investigations(status);

-- Create an alias view for backwards compatibility
CREATE VIEW IF NOT EXISTS flight_investigations AS SELECT * FROM investigations;

-- Passenger Name Mappings Table
-- Maps abbreviations to full names for data cleanup
CREATE TABLE IF NOT EXISTS passenger_mappings (
    abbreviation TEXT PRIMARY KEY,
    full_name TEXT NOT NULL,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_passenger_mappings_full_name ON passenger_mappings(full_name);

-- Research Reports Table
-- Stores AI agent research reports for documentation
CREATE TABLE IF NOT EXISTS research_reports (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,

    -- Agent Information
    agent_name TEXT NOT NULL, -- Gemini, Grok, DeepSeek, Local
    agent_model TEXT, -- Specific model used (e.g., grok-2-1212, deepseek-chat)

    -- Research Details
    search_query TEXT NOT NULL,
    research_topics TEXT, -- JSON array of topics researched

    -- Results
    report_summary TEXT NOT NULL, -- Main findings/summary
    report_details TEXT, -- Full detailed report (JSON or markdown)
    sources TEXT, -- JSON array of sources with URLs, titles, snippets
    confidence_score REAL, -- 0.0-1.0 confidence rating

    -- Metadata
    flight_id TEXT, -- Optional: if report is associated with a flight
    report_type TEXT DEFAULT 'general', -- general, flight, passenger, location
    processing_time_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_research_reports_user ON research_reports(user_id);
CREATE INDEX IF NOT EXISTS idx_research_reports_agent ON research_reports(agent_name);
CREATE INDEX IF NOT EXISTS idx_research_reports_flight ON research_reports(flight_id);
CREATE INDEX IF NOT EXISTS idx_research_reports_created ON research_reports(created_at);

-- Custom Documents Table
-- User-created notes and documents
-- Note: journey_id, passenger_name, fuel_entry_id columns are added via migration
-- for backwards compatibility with existing databases
CREATE TABLE IF NOT EXISTS custom_documents (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,

    -- Document Details
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    category TEXT, -- Notes, Planning, Reports, etc.
    tags TEXT, -- JSON array of tags

    -- Associations
    flight_id TEXT, -- Optional: link to specific flight

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_custom_documents_user ON custom_documents(user_id);
CREATE INDEX IF NOT EXISTS idx_custom_documents_flight ON custom_documents(flight_id);
CREATE INDEX IF NOT EXISTS idx_custom_documents_category ON custom_documents(category);
CREATE INDEX IF NOT EXISTS idx_custom_documents_created ON custom_documents(created_at);

-- Fuel Prices Table
-- Track fuel prices over time for cost analysis
CREATE TABLE IF NOT EXISTS fuel_prices (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,

    -- Fuel Details
    fuel_type TEXT NOT NULL DEFAULT 'Jet A', -- Jet A, Jet A-1, Avgas 100LL, Mogas, etc.
    price_per_unit REAL NOT NULL, -- Price per liter or gallon
    unit TEXT NOT NULL DEFAULT 'liter', -- liter, gallon, imperial_gallon
    currency TEXT NOT NULL DEFAULT 'USD', -- USD, EUR, GBP, etc.

    -- Time Period
    start_date TEXT NOT NULL, -- When this price period starts
    end_date TEXT, -- When this price period ends (NULL for current/ongoing)

    -- Optional Context
    location TEXT, -- Airport code, city, or region (e.g., "KJFK", "New York", "Northeast US")
    supplier TEXT, -- Fuel supplier/vendor name
    notes TEXT, -- Additional notes about this price

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_fuel_prices_user ON fuel_prices(user_id);
CREATE INDEX IF NOT EXISTS idx_fuel_prices_dates ON fuel_prices(start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_fuel_prices_location ON fuel_prices(location);
CREATE INDEX IF NOT EXISTS idx_fuel_prices_fuel_type ON fuel_prices(fuel_type);

-- Trusted WiFi Devices Table
-- Physical security monitoring: track known/safe WiFi networks
CREATE TABLE IF NOT EXISTS trusted_wifi_devices (
    bssid TEXT PRIMARY KEY, -- MAC address (BSSID) of the access point
    ssid TEXT, -- Network name (optional, can change)
    notes TEXT, -- User notes about this device
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen TEXT
);

CREATE INDEX IF NOT EXISTS idx_trusted_wifi_last_seen ON trusted_wifi_devices(last_seen);

-- Trusted Bluetooth Devices Table
-- Physical security monitoring: track known/safe Bluetooth devices
CREATE TABLE IF NOT EXISTS trusted_bluetooth_devices (
    address TEXT PRIMARY KEY, -- Bluetooth MAC address
    name TEXT, -- Device name (optional, can change)
    device_type TEXT, -- BLE, Classic, etc.
    notes TEXT, -- User notes about this device
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen TEXT
);

CREATE INDEX IF NOT EXISTS idx_trusted_bluetooth_last_seen ON trusted_bluetooth_devices(last_seen);

-- Agent Memory Table
-- Stores all agent thoughts, searches, and results for knowledge accumulation
-- Enables semantic search across agent history
CREATE TABLE IF NOT EXISTS agent_memory (
    id TEXT PRIMARY KEY,
    agent_name TEXT NOT NULL, -- Grok, DeepSeek, Gemini, etc.
    memory_type TEXT NOT NULL, -- 'search_query', 'research_result', 'thought', 'tool_call', 'conversation'

    -- Context
    flight_id TEXT, -- Optional: associated flight
    user_id TEXT, -- Optional: associated user

    -- Content
    query TEXT, -- The original query or prompt
    content TEXT NOT NULL, -- The actual memory content (JSON or text)
    summary TEXT, -- Short summary for quick scanning

    -- Metadata
    tokens_used INTEGER DEFAULT 0,
    cost_usd REAL DEFAULT 0.0,
    model TEXT, -- Model used to generate this memory

    -- Vector embedding for semantic search (stored as JSON array)
    embedding TEXT, -- JSON array of floats for vector similarity

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT, -- Optional TTL
    last_accessed TEXT, -- Track usage for relevance ranking
    access_count INTEGER DEFAULT 0,

    -- Quality metrics
    confidence_score REAL, -- 0.0-1.0 confidence in this memory
    verified INTEGER DEFAULT 0, -- Has this been verified by another agent or human?

    FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE SET NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_agent_memory_agent ON agent_memory(agent_name);
CREATE INDEX IF NOT EXISTS idx_agent_memory_type ON agent_memory(memory_type);
CREATE INDEX IF NOT EXISTS idx_agent_memory_flight ON agent_memory(flight_id);
CREATE INDEX IF NOT EXISTS idx_agent_memory_created ON agent_memory(created_at);
CREATE INDEX IF NOT EXISTS idx_agent_memory_expires ON agent_memory(expires_at);

-- Full-text search index for memory content
CREATE VIRTUAL TABLE IF NOT EXISTS agent_memory_fts USING fts5(
    query,
    content,
    summary,
    content='agent_memory',
    content_rowid='rowid'
);

-- Triggers to keep FTS index in sync
CREATE TRIGGER IF NOT EXISTS agent_memory_fts_insert AFTER INSERT ON agent_memory BEGIN
    INSERT INTO agent_memory_fts(rowid, query, content, summary)
    VALUES (new.rowid, new.query, new.content, new.summary);
END;

CREATE TRIGGER IF NOT EXISTS agent_memory_fts_delete AFTER DELETE ON agent_memory BEGIN
    DELETE FROM agent_memory_fts WHERE rowid = old.rowid;
END;

CREATE TRIGGER IF NOT EXISTS agent_memory_fts_update AFTER UPDATE ON agent_memory BEGIN
    UPDATE agent_memory_fts SET query = new.query, content = new.content, summary = new.summary
    WHERE rowid = new.rowid;
END;

-- OCR Corrections Table
-- Stores user corrections to OCR results for learning and improvement
CREATE TABLE IF NOT EXISTS ocr_corrections (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,

    -- Field that was corrected
    field_name TEXT NOT NULL, -- flight_number, departure_airport, etc.

    -- Values
    original_value TEXT, -- What OCR extracted
    corrected_value TEXT NOT NULL, -- What user corrected it to

    -- Context for pattern recognition
    image_hash TEXT, -- Hash of the source image for deduplication
    confidence_score REAL, -- Original OCR confidence if available

    -- Learning metadata
    pattern_type TEXT, -- 'abbreviation', 'ocr_misread', 'format_fix', etc.
    applied_count INTEGER DEFAULT 0, -- How many times this correction was auto-applied
    verified INTEGER DEFAULT 1, -- User-verified correction

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_applied TEXT, -- Last time this correction was auto-applied

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ocr_corrections_user ON ocr_corrections(user_id);
CREATE INDEX IF NOT EXISTS idx_ocr_corrections_field ON ocr_corrections(field_name);
CREATE INDEX IF NOT EXISTS idx_ocr_corrections_original ON ocr_corrections(original_value);
CREATE INDEX IF NOT EXISTS idx_ocr_corrections_created ON ocr_corrections(created_at);

-- OCR Learning Patterns Table
-- Aggregated patterns learned from corrections
CREATE TABLE IF NOT EXISTS ocr_learning_patterns (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,

    -- Pattern details
    pattern_type TEXT NOT NULL, -- 'substitution', 'abbreviation', 'format', 'airport_code'
    field_name TEXT NOT NULL, -- Which field this pattern applies to

    -- Pattern matching
    match_pattern TEXT NOT NULL, -- Regex or exact match pattern
    replacement_value TEXT NOT NULL, -- What to replace with

    -- Statistics
    confidence REAL DEFAULT 0.5, -- 0.0-1.0 confidence in this pattern
    occurrence_count INTEGER DEFAULT 1, -- How many times seen
    success_count INTEGER DEFAULT 0, -- How many times user accepted auto-suggestion
    rejection_count INTEGER DEFAULT 0, -- How many times user rejected

    -- Metadata
    is_active INTEGER DEFAULT 1, -- Can be disabled
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used TEXT,

    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_learning_patterns_user ON ocr_learning_patterns(user_id);
CREATE INDEX IF NOT EXISTS idx_learning_patterns_field ON ocr_learning_patterns(field_name);
CREATE INDEX IF NOT EXISTS idx_learning_patterns_type ON ocr_learning_patterns(pattern_type);
CREATE INDEX IF NOT EXISTS idx_learning_patterns_active ON ocr_learning_patterns(is_active);
