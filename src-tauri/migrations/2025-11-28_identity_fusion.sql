-- Identity Fusion Schema Migration
-- Establishes the Canonical Identity Architecture for passenger deduplication
-- Date: 2025-11-28

-- =============================================================================
-- 1. CANONICAL IDENTITIES TABLE (The "Soul")
-- =============================================================================
-- Represents the single, real-world entity (e.g., "Jeffrey Epstein")
-- This is the source of truth - one record per real person

CREATE TABLE IF NOT EXISTS passengers (
    id TEXT PRIMARY KEY,                              -- UUID
    canonical_name TEXT NOT NULL UNIQUE,              -- The "Correct" Name (e.g., "JEFFREY EPSTEIN")
    notes TEXT,                                       -- Optional notes about this person
    total_flights INTEGER DEFAULT 0,                  -- Aggregated count (denormalized for performance)
    first_seen_date TEXT,                             -- First appearance in flight logs
    last_seen_date TEXT,                              -- Last appearance in flight logs
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_passengers_canonical_name ON passengers(canonical_name);


-- =============================================================================
-- 2. PASSENGER ALIASES TABLE (The "Variations")
-- =============================================================================
-- Links raw text variations to the Canonical Identity
-- e.g., "JE" -> UUID_EPSTEIN, "JEFFREY" -> UUID_EPSTEIN
-- An alias can ONLY belong to ONE canonical identity (enforced by UNIQUE)

CREATE TABLE IF NOT EXISTS passenger_aliases (
    id TEXT PRIMARY KEY,                              -- UUID
    passenger_id TEXT NOT NULL,                       -- FK to passengers.id (Master Record)
    raw_name TEXT NOT NULL UNIQUE,                    -- The variation string (UNIQUE = one owner only)
    usage_count INTEGER DEFAULT 0,                    -- How many times this alias appears in flight logs
    source_document TEXT,                             -- Traceability (e.g., "E-flight-logs.pdf page 3")
    match_type TEXT,                                  -- 'exact', 'abbreviation', 'fuzzy', 'manual'
    confidence REAL DEFAULT 1.0,                      -- 0.0-1.0 confidence in this mapping
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (passenger_id) REFERENCES passengers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_passenger_aliases_raw ON passenger_aliases(raw_name);
CREATE INDEX IF NOT EXISTS idx_passenger_aliases_passenger ON passenger_aliases(passenger_id);


-- =============================================================================
-- 3. FLIGHT PASSENGERS JUNCTION TABLE
-- =============================================================================
-- Many-to-many: A flight can have multiple passengers, a passenger can be on multiple flights
-- This replaces storing passenger names in the flights.notes field

CREATE TABLE IF NOT EXISTS flight_passengers (
    flight_id TEXT NOT NULL,                          -- FK to flights.id
    passenger_id TEXT NOT NULL,                       -- FK to passengers.id (the canonical identity)
    seat_info TEXT,                                   -- Optional: seat assignment
    role TEXT DEFAULT 'passenger',                    -- 'passenger', 'pilot', 'crew'
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    PRIMARY KEY (flight_id, passenger_id),
    FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE CASCADE,
    FOREIGN KEY (passenger_id) REFERENCES passengers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_flight_passengers_flight ON flight_passengers(flight_id);
CREATE INDEX IF NOT EXISTS idx_flight_passengers_passenger ON flight_passengers(passenger_id);


-- =============================================================================
-- 4. MIGRATION VIEW (For backwards compatibility during transition)
-- =============================================================================
-- Provides a view that mimics the old passenger_mappings table structure
-- Allows existing code to continue working while we migrate

CREATE VIEW IF NOT EXISTS passenger_mappings_compat AS
SELECT
    pa.raw_name AS abbreviation,
    p.canonical_name AS full_name,
    p.notes AS notes,
    pa.created_at AS created_at,
    p.updated_at AS updated_at
FROM passenger_aliases pa
JOIN passengers p ON pa.passenger_id = p.id;


-- =============================================================================
-- 5. USEFUL AGGREGATION VIEWS
-- =============================================================================

-- View: All aliases grouped by canonical identity
CREATE VIEW IF NOT EXISTS passenger_identity_summary AS
SELECT
    p.id AS passenger_id,
    p.canonical_name,
    p.total_flights,
    COUNT(pa.id) AS alias_count,
    GROUP_CONCAT(pa.raw_name, ', ') AS aliases
FROM passengers p
LEFT JOIN passenger_aliases pa ON p.id = pa.passenger_id
GROUP BY p.id;

-- View: Unmerged aliases (aliases that point to themselves - candidates for merging)
CREATE VIEW IF NOT EXISTS unmerged_aliases AS
SELECT
    p.id AS passenger_id,
    p.canonical_name,
    pa.raw_name,
    pa.usage_count
FROM passengers p
JOIN passenger_aliases pa ON p.id = pa.passenger_id
WHERE p.canonical_name = pa.raw_name  -- Self-alias (not yet merged into a master)
ORDER BY pa.usage_count DESC;
