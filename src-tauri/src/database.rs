// Database Module - SQLite operations for Flight Tracker Pro

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use uuid::Uuid;

use crate::models::*;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    /// Initialize the database with schema
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(&db_path).context("Failed to open database connection")?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .context("Failed to enable foreign keys")?;

        // Initialize schema
        Self::init_schema(&conn)?;

        Ok(Self { conn })
    }

    /// Get a reference to the database connection for advanced operations
    pub fn get_connection(&self) -> &Connection {
        &self.conn
    }

    /// Create the database schema
    fn init_schema(conn: &Connection) -> Result<()> {
        let schema_sql = include_str!("schema.sql");
        conn.execute_batch(schema_sql)
            .context("Failed to initialize database schema")?;

        // Run migrations for new tables (idempotent - safe to run multiple times)
        Self::run_migrations(conn)?;

        Ok(())
    }

    /// Run database migrations for new features
    fn run_migrations(conn: &Connection) -> Result<()> {
        // Migration: Add trusted device tables for radar feature
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS trusted_wifi_devices (
                bssid TEXT PRIMARY KEY,
                ssid TEXT,
                notes TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_seen TEXT
            );

            CREATE TABLE IF NOT EXISTS trusted_bluetooth_devices (
                address TEXT PRIMARY KEY,
                name TEXT,
                device_type TEXT,
                notes TEXT,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_seen TEXT
            );

            CREATE TABLE IF NOT EXISTS agent_memory (
                id TEXT PRIMARY KEY,
                agent_name TEXT NOT NULL,
                memory_type TEXT NOT NULL,
                flight_id TEXT,
                user_id TEXT,
                query TEXT,
                content TEXT NOT NULL,
                summary TEXT,
                tokens_used INTEGER DEFAULT 0,
                cost_usd REAL DEFAULT 0.0,
                model TEXT,
                embedding TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT,
                last_accessed TEXT,
                access_count INTEGER DEFAULT 0,
                confidence_score REAL,
                verified INTEGER DEFAULT 0,
                FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE SET NULL,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS agent_memory_fts USING fts5(
                query, content, summary,
                content='agent_memory',
                content_rowid='rowid'
            );

            CREATE TRIGGER IF NOT EXISTS agent_memory_fts_insert AFTER INSERT ON agent_memory BEGIN
                INSERT INTO agent_memory_fts(rowid, query, content, summary)
                VALUES (new.rowid, new.query, new.content, new.summary);
            END;

            CREATE TRIGGER IF NOT EXISTS agent_memory_fts_delete AFTER DELETE ON agent_memory BEGIN
                DELETE FROM agent_memory_fts WHERE rowid = old.rowid;
            END;

            CREATE TRIGGER IF NOT EXISTS agent_memory_fts_update AFTER UPDATE ON agent_memory BEGIN
                DELETE FROM agent_memory_fts WHERE rowid = old.rowid;
                INSERT INTO agent_memory_fts(rowid, query, content, summary)
                VALUES (new.rowid, new.query, new.content, new.summary);
            END;

            CREATE INDEX IF NOT EXISTS idx_agent_memory_flight ON agent_memory(flight_id);
            CREATE INDEX IF NOT EXISTS idx_agent_memory_user ON agent_memory(user_id);
            CREATE INDEX IF NOT EXISTS idx_agent_memory_type ON agent_memory(memory_type);
            CREATE INDEX IF NOT EXISTS idx_agent_memory_agent ON agent_memory(agent_name);
            CREATE INDEX IF NOT EXISTS idx_agent_memory_created ON agent_memory(created_at);
            CREATE INDEX IF NOT EXISTS idx_agent_memory_expires ON agent_memory(expires_at);

            CREATE TABLE IF NOT EXISTS document_ingestion_queue (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                source_document_id TEXT,
                source_file_path TEXT NOT NULL,
                source_file_name TEXT NOT NULL,
                source_file_hash TEXT NOT NULL,
                total_pages INTEGER NOT NULL,
                pages_processed INTEGER DEFAULT 0,
                status TEXT DEFAULT 'pending',
                error_message TEXT,
                priority INTEGER DEFAULT 0,
                retry_count INTEGER DEFAULT 0,
                max_retries INTEGER DEFAULT 3,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                started_at TEXT,
                completed_at TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY (source_document_id) REFERENCES custom_documents(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_doc_queue_status ON document_ingestion_queue(status);
            CREATE INDEX IF NOT EXISTS idx_doc_queue_user ON document_ingestion_queue(user_id);
            CREATE INDEX IF NOT EXISTS idx_doc_queue_priority ON document_ingestion_queue(priority DESC, created_at ASC);

            CREATE TABLE IF NOT EXISTS document_chunks (
                id TEXT PRIMARY KEY,
                queue_id TEXT NOT NULL,
                chunk_hash TEXT UNIQUE NOT NULL,
                chunk_number INTEGER NOT NULL,
                chunk_type TEXT DEFAULT 'page',
                content_path TEXT NOT NULL,
                status TEXT DEFAULT 'pending',
                processing_method TEXT DEFAULT 'text_extract',
                ocr_text TEXT,
                extracted_entities TEXT,
                processing_stage TEXT DEFAULT 'pending',
                error_message TEXT,
                retry_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                processed_at TEXT,
                FOREIGN KEY (queue_id) REFERENCES document_ingestion_queue(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_chunks_queue ON document_chunks(queue_id);
            CREATE INDEX IF NOT EXISTS idx_chunks_status ON document_chunks(status);
            CREATE INDEX IF NOT EXISTS idx_chunks_hash ON document_chunks(chunk_hash);
            CREATE INDEX IF NOT EXISTS idx_chunks_stage ON document_chunks(processing_stage);

            CREATE TABLE IF NOT EXISTS entity_extractions (
                id TEXT PRIMARY KEY,
                chunk_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_value TEXT NOT NULL,
                confidence REAL DEFAULT 0.0,
                context TEXT,
                start_position INTEGER,
                end_position INTEGER,
                metadata TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (chunk_id) REFERENCES document_chunks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_entities_chunk ON entity_extractions(chunk_id);
            CREATE INDEX IF NOT EXISTS idx_entities_type ON entity_extractions(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entities_value ON entity_extractions(entity_value);

            CREATE TABLE IF NOT EXISTS document_matches (
                id TEXT PRIMARY KEY,
                chunk_id TEXT NOT NULL,
                flight_id TEXT,
                match_type TEXT NOT NULL,
                confidence REAL NOT NULL,
                evidence TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (chunk_id) REFERENCES document_chunks(id) ON DELETE CASCADE,
                FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_matches_chunk ON document_matches(chunk_id);
            CREATE INDEX IF NOT EXISTS idx_matches_flight ON document_matches(flight_id);
            CREATE INDEX IF NOT EXISTS idx_matches_confidence ON document_matches(confidence);

            CREATE TABLE IF NOT EXISTS relationship_graph (
                id TEXT PRIMARY KEY,
                source_type TEXT NOT NULL,
                source_id TEXT NOT NULL,
                target_type TEXT NOT NULL,
                target_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                weight REAL DEFAULT 1.0,
                evidence_ids TEXT,
                metadata TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(source_type, source_id, target_type, target_id, relationship_type)
            );

            CREATE INDEX IF NOT EXISTS idx_graph_source ON relationship_graph(source_type, source_id);
            CREATE INDEX IF NOT EXISTS idx_graph_target ON relationship_graph(target_type, target_id);
            CREATE INDEX IF NOT EXISTS idx_graph_type ON relationship_graph(relationship_type);

            -- Fuel Price Cache: AI-searched prices
            CREATE TABLE IF NOT EXISTS fuel_price_cache (
                id TEXT PRIMARY KEY,
                airport_code TEXT,
                location_name TEXT NOT NULL,
                region TEXT,
                country TEXT,
                fuel_type TEXT NOT NULL DEFAULT 'jet_a',
                price_per_gallon REAL NOT NULL,
                price_per_liter REAL,
                currency TEXT DEFAULT 'USD',
                effective_date TEXT NOT NULL,
                source TEXT,
                source_url TEXT,
                confidence TEXT DEFAULT 'medium',
                ai_response TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT,
                UNIQUE(airport_code, fuel_type, effective_date)
            );

            CREATE INDEX IF NOT EXISTS idx_fuel_cache_airport ON fuel_price_cache(airport_code);
            CREATE INDEX IF NOT EXISTS idx_fuel_cache_type ON fuel_price_cache(fuel_type);
            CREATE INDEX IF NOT EXISTS idx_fuel_cache_date ON fuel_price_cache(effective_date);
            CREATE INDEX IF NOT EXISTS idx_fuel_cache_region ON fuel_price_cache(region);

            -- Fuel Entries: User's actual fuel purchases
            CREATE TABLE IF NOT EXISTS fuel_entries (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                flight_id TEXT,
                aircraft_id TEXT,
                airport_code TEXT,
                location_name TEXT,
                fuel_type TEXT NOT NULL DEFAULT 'jet_a',
                gallons REAL NOT NULL,
                price_per_gallon REAL NOT NULL,
                total_cost REAL NOT NULL,
                currency TEXT DEFAULT 'USD',
                purchase_date TEXT NOT NULL,
                fbo_name TEXT,
                receipt_number TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_fuel_entries_user ON fuel_entries(user_id);
            CREATE INDEX IF NOT EXISTS idx_fuel_entries_flight ON fuel_entries(flight_id);
            CREATE INDEX IF NOT EXISTS idx_fuel_entries_airport ON fuel_entries(airport_code);
            CREATE INDEX IF NOT EXISTS idx_fuel_entries_date ON fuel_entries(purchase_date);

            -- Fuel Types: Custom fuel types defined by users
            CREATE TABLE IF NOT EXISTS fuel_types (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                code TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                category TEXT DEFAULT 'aviation',
                is_default INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE(user_id, code)
            );

            CREATE INDEX IF NOT EXISTS idx_fuel_types_user ON fuel_types(user_id);
            CREATE INDEX IF NOT EXISTS idx_fuel_types_code ON fuel_types(code);

            -- Custom Schemas: User-defined entity types
            CREATE TABLE IF NOT EXISTS custom_schemas (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                description TEXT,
                icon TEXT DEFAULT 'database',
                color TEXT DEFAULT '#6366f1',
                is_active INTEGER DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE(user_id, name)
            );

            -- Custom Schema Fields: Field definitions for each schema
            CREATE TABLE IF NOT EXISTS custom_schema_fields (
                id TEXT PRIMARY KEY,
                schema_id TEXT NOT NULL,
                name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                field_type TEXT NOT NULL,
                is_required INTEGER DEFAULT 0,
                default_value TEXT,
                options TEXT,
                validation_rules TEXT,
                sort_order INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (schema_id) REFERENCES custom_schemas(id) ON DELETE CASCADE,
                UNIQUE(schema_id, name)
            );

            -- Custom Entity Records: Actual data for custom schemas
            CREATE TABLE IF NOT EXISTS custom_records (
                id TEXT PRIMARY KEY,
                schema_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                data TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (schema_id) REFERENCES custom_schemas(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            -- Flight Custom Fields: Extra fields on flight records
            CREATE TABLE IF NOT EXISTS flight_custom_fields (
                id TEXT PRIMARY KEY,
                flight_id TEXT NOT NULL,
                field_name TEXT NOT NULL,
                field_value TEXT,
                field_type TEXT NOT NULL DEFAULT 'text',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE CASCADE,
                UNIQUE(flight_id, field_name)
            );

            CREATE INDEX IF NOT EXISTS idx_custom_schemas_user ON custom_schemas(user_id);
            CREATE INDEX IF NOT EXISTS idx_custom_fields_schema ON custom_schema_fields(schema_id);
            CREATE INDEX IF NOT EXISTS idx_custom_records_schema ON custom_records(schema_id);
            CREATE INDEX IF NOT EXISTS idx_custom_records_user ON custom_records(user_id);
            CREATE INDEX IF NOT EXISTS idx_flight_custom_fields ON flight_custom_fields(flight_id);

            -- ===== SELF-IMPROVEMENT SYSTEM =====

            -- User Corrections: Track corrections for auto-fill learning
            CREATE TABLE IF NOT EXISTS user_corrections (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                field_name TEXT NOT NULL,
                original_value TEXT,
                corrected_value TEXT NOT NULL,
                context TEXT,
                occurrence_count INTEGER DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(user_id, field_name, original_value, corrected_value)
            );

            -- User Patterns: Track behavioral patterns for smart defaults
            CREATE TABLE IF NOT EXISTS user_patterns (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                pattern_type TEXT NOT NULL,
                pattern_key TEXT NOT NULL,
                pattern_value TEXT NOT NULL,
                frequency INTEGER DEFAULT 1,
                last_used TEXT NOT NULL DEFAULT (datetime('now')),
                metadata TEXT,
                UNIQUE(user_id, pattern_type, pattern_key)
            );

            -- Query Performance: Track slow queries for optimization
            CREATE TABLE IF NOT EXISTS query_performance (
                id TEXT PRIMARY KEY,
                query_hash TEXT NOT NULL,
                query_type TEXT NOT NULL,
                table_name TEXT,
                filters TEXT,
                execution_time_ms INTEGER NOT NULL,
                result_count INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- AI Response Cache: Cache AI responses for cost optimization
            CREATE TABLE IF NOT EXISTS ai_response_cache (
                id TEXT PRIMARY KEY,
                query_hash TEXT NOT NULL UNIQUE,
                provider TEXT NOT NULL,
                query_text TEXT NOT NULL,
                response_text TEXT NOT NULL,
                confidence REAL,
                hit_count INTEGER DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT,
                metadata TEXT
            );

            -- Flight Anomalies: Detected anomalies for data integrity
            CREATE TABLE IF NOT EXISTS flight_anomalies (
                id TEXT PRIMARY KEY,
                flight_id TEXT NOT NULL,
                anomaly_type TEXT NOT NULL,
                severity TEXT DEFAULT 'warning',
                description TEXT NOT NULL,
                suggested_fix TEXT,
                is_resolved INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                resolved_at TEXT,
                FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE CASCADE
            );

            -- Duplicate Candidates: Potential duplicate flights
            CREATE TABLE IF NOT EXISTS duplicate_candidates (
                id TEXT PRIMARY KEY,
                flight_id_1 TEXT NOT NULL,
                flight_id_2 TEXT NOT NULL,
                similarity_score REAL NOT NULL,
                match_reasons TEXT,
                status TEXT DEFAULT 'pending',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                resolved_at TEXT,
                FOREIGN KEY (flight_id_1) REFERENCES flights(id) ON DELETE CASCADE,
                FOREIGN KEY (flight_id_2) REFERENCES flights(id) ON DELETE CASCADE
            );

            -- Provider Accuracy: Track AI provider accuracy for triangulation
            CREATE TABLE IF NOT EXISTS provider_accuracy (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                query_type TEXT NOT NULL,
                correct_count INTEGER DEFAULT 0,
                total_count INTEGER DEFAULT 0,
                avg_confidence REAL,
                last_updated TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(provider, query_type)
            );

            -- Route Statistics: Learned route patterns
            CREATE TABLE IF NOT EXISTS route_statistics (
                id TEXT PRIMARY KEY,
                departure_airport TEXT NOT NULL,
                arrival_airport TEXT NOT NULL,
                avg_duration_minutes REAL,
                avg_distance_km REAL,
                common_aircraft TEXT,
                flight_count INTEGER DEFAULT 1,
                last_updated TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(departure_airport, arrival_airport)
            );

            -- Fuel Price History: Persistent fuel price database
            CREATE TABLE IF NOT EXISTS fuel_price_history (
                id TEXT PRIMARY KEY,
                airport_code TEXT,
                location_name TEXT NOT NULL,
                region TEXT,
                country TEXT,
                fuel_type TEXT NOT NULL DEFAULT 'jet_a',
                price_per_gallon REAL NOT NULL,
                price_per_liter REAL,
                currency TEXT DEFAULT 'USD',
                effective_date TEXT NOT NULL,
                source TEXT NOT NULL,
                source_url TEXT,
                verified INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_corrections_user ON user_corrections(user_id);
            CREATE INDEX IF NOT EXISTS idx_patterns_user ON user_patterns(user_id, pattern_type);
            CREATE INDEX IF NOT EXISTS idx_query_perf_hash ON query_performance(query_hash);
            CREATE INDEX IF NOT EXISTS idx_ai_cache_hash ON ai_response_cache(query_hash);
            CREATE INDEX IF NOT EXISTS idx_anomalies_flight ON flight_anomalies(flight_id);
            CREATE INDEX IF NOT EXISTS idx_duplicates_flights ON duplicate_candidates(flight_id_1, flight_id_2);
            CREATE INDEX IF NOT EXISTS idx_route_stats ON route_statistics(departure_airport, arrival_airport);
            CREATE INDEX IF NOT EXISTS idx_fuel_history_location ON fuel_price_history(airport_code, fuel_type);
            CREATE INDEX IF NOT EXISTS idx_fuel_history_date ON fuel_price_history(effective_date);

            CREATE TABLE IF NOT EXISTS ocr_corrections (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                field_name TEXT NOT NULL,
                original_value TEXT,
                corrected_value TEXT NOT NULL,
                image_hash TEXT,
                confidence_score REAL,
                pattern_type TEXT,
                applied_count INTEGER DEFAULT 0,
                verified INTEGER DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_applied TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_ocr_corrections_user ON ocr_corrections(user_id);
            CREATE INDEX IF NOT EXISTS idx_ocr_corrections_field ON ocr_corrections(field_name);
            CREATE INDEX IF NOT EXISTS idx_ocr_corrections_original ON ocr_corrections(original_value);
            CREATE INDEX IF NOT EXISTS idx_ocr_corrections_created ON ocr_corrections(created_at);

            CREATE TABLE IF NOT EXISTS ocr_learning_patterns (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                pattern_type TEXT NOT NULL,
                field_name TEXT NOT NULL,
                match_pattern TEXT NOT NULL,
                replacement_value TEXT NOT NULL,
                confidence REAL DEFAULT 0.5,
                occurrence_count INTEGER DEFAULT 1,
                success_count INTEGER DEFAULT 0,
                rejection_count INTEGER DEFAULT 0,
                is_active INTEGER DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_used TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_learning_patterns_user ON ocr_learning_patterns(user_id);
            CREATE INDEX IF NOT EXISTS idx_learning_patterns_field ON ocr_learning_patterns(field_name);
            CREATE INDEX IF NOT EXISTS idx_learning_patterns_type ON ocr_learning_patterns(pattern_type);
            CREATE INDEX IF NOT EXISTS idx_learning_patterns_active ON ocr_learning_patterns(is_active);

            -- ===== IDENTITY FUSION ARCHITECTURE =====
            -- Canonical Identity Table (The Master Records)
            CREATE TABLE IF NOT EXISTS passengers (
                id TEXT PRIMARY KEY,
                canonical_name TEXT NOT NULL UNIQUE,
                notes TEXT,
                total_flights INTEGER DEFAULT 0,
                first_seen_date TEXT,
                last_seen_date TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_passengers_canonical_name ON passengers(canonical_name);

            -- Passenger Aliases Table (Variations linked to Master)
            CREATE TABLE IF NOT EXISTS passenger_aliases (
                id TEXT PRIMARY KEY,
                passenger_id TEXT NOT NULL,
                raw_name TEXT NOT NULL UNIQUE,
                usage_count INTEGER DEFAULT 0,
                source_document TEXT,
                match_type TEXT,
                confidence REAL DEFAULT 1.0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (passenger_id) REFERENCES passengers(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_passenger_aliases_raw ON passenger_aliases(raw_name);
            CREATE INDEX IF NOT EXISTS idx_passenger_aliases_passenger ON passenger_aliases(passenger_id);

            -- Flight-Passenger Junction Table (Many-to-Many)
            CREATE TABLE IF NOT EXISTS flight_passengers (
                flight_id TEXT NOT NULL,
                passenger_id TEXT NOT NULL,
                seat_info TEXT,
                role TEXT DEFAULT 'passenger',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (flight_id, passenger_id),
                FOREIGN KEY (flight_id) REFERENCES flights(id) ON DELETE CASCADE,
                FOREIGN KEY (passenger_id) REFERENCES passengers(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_flight_passengers_flight ON flight_passengers(flight_id);
            CREATE INDEX IF NOT EXISTS idx_flight_passengers_passenger ON flight_passengers(passenger_id);

            -- Do Not Deduplicate List: Passengers that should remain separate
            CREATE TABLE IF NOT EXISTS passenger_no_dedup (
                passenger_id TEXT PRIMARY KEY,
                reason TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (passenger_id) REFERENCES passengers(id) ON DELETE CASCADE
            );"
        ).context("Failed to run database migrations")?;

        // Migration: Add entity linking columns to custom_documents
        // These columns allow documents to be linked to journeys, passengers, and fuel entries
        let _ = conn.execute(
            "ALTER TABLE custom_documents ADD COLUMN journey_id TEXT REFERENCES journeys(id) ON DELETE SET NULL",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE custom_documents ADD COLUMN passenger_name TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE custom_documents ADD COLUMN fuel_entry_id TEXT REFERENCES fuel_entries(id) ON DELETE SET NULL",
            [],
        );
        // Create indexes (safe to run multiple times)
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_custom_documents_journey ON custom_documents(journey_id)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_custom_documents_passenger ON custom_documents(passenger_name)",
            [],
        );
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_custom_documents_fuel ON custom_documents(fuel_entry_id)",
            [],
        );

        // Migration: Add per_passenger_co2_kg column to flights
        // This stores the CO2 share per passenger (total CO2 / passenger count, excluding crew)
        let _ = conn.execute(
            "ALTER TABLE flights ADD COLUMN per_passenger_co2_kg REAL",
            [],
        );

        Ok(())
    }

    // ===== SETTINGS OPERATIONS =====

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .context("Failed to get setting")?;

        Ok(value)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = datetime('now')",
                params![key, value],
            )
            .context("Failed to set setting")?;

        Ok(())
    }

    // ===== USER OPERATIONS =====

    pub fn create_user(&self, user_input: &User) -> Result<String> {
        let id = Uuid::new_v4().to_string();

        self.conn.execute(
            "INSERT INTO users (id, name, email, pilot_license_number, license_type, license_country)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                user_input.name,
                user_input.email,
                user_input.pilot_license_number,
                user_input.license_type,
                user_input.license_country
            ],
        ).context("Failed to create user")?;

        Ok(id)
    }

    pub fn get_user(&self, user_id: &str) -> Result<Option<User>> {
        let user = self
            .conn
            .query_row(
                "SELECT id, name, email, pilot_license_number, license_type, license_country,
                        created_at, updated_at
                 FROM users WHERE id = ?1",
                params![user_id],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        email: row.get(2)?,
                        pilot_license_number: row.get(3)?,
                        license_type: row.get(4)?,
                        license_country: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .optional()
            .context("Failed to get user")?;

        Ok(user)
    }

    pub fn get_primary_user(&self) -> Result<Option<User>> {
        let user = self
            .conn
            .query_row(
                "SELECT id, name, email, pilot_license_number, license_type, license_country,
                        created_at, updated_at
                 FROM users LIMIT 1",
                [],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        email: row.get(2)?,
                        pilot_license_number: row.get(3)?,
                        license_type: row.get(4)?,
                        license_country: row.get(5)?,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                    })
                },
            )
            .optional()
            .context("Failed to get primary user")?;

        Ok(user)
    }

    // ===== FLIGHT OPERATIONS =====

    pub fn create_flight(&self, user_id: &str, flight: &FlightInput) -> Result<String> {
        let id = Uuid::new_v4().to_string();

        // Use distance_km if provided, otherwise convert from nautical miles
        let distance_km = flight
            .distance_km
            .or_else(|| flight.distance_nm.map(|nm| nm * 1.852));

        // Calculate flight time if we have distance and no existing duration
        let flight_duration = flight.flight_duration.or_else(|| {
            distance_km.map(|km| {
                crate::calculations::calculate_flight_time(
                    km,
                    flight.aircraft_registration.as_deref(),
                )
            })
        });

        // Calculate total duration (block time) if we have flight duration
        let total_duration = flight
            .total_duration
            .or_else(|| flight_duration.map(crate::calculations::calculate_block_time));

        // Use provided CO2 emissions, or calculate from distance if available
        let carbon_emissions_kg = flight.carbon_emissions_kg.or_else(|| {
            distance_km.map(|km| {
                crate::calculations::calculate_co2_emissions(
                    km,
                    flight.aircraft_registration.as_deref(),
                )
            })
        });

        self.conn
            .execute(
                "INSERT INTO flights (
                id, user_id, flight_number, departure_airport, arrival_airport,
                departure_datetime, arrival_datetime, aircraft_type_id, aircraft_registration,
                total_duration, flight_duration, distance_nm, distance_km, booking_reference,
                ticket_number, seat_number, fare_class, base_fare, taxes, total_cost,
                currency, carbon_emissions_kg, notes, attachment_path, data_source
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24, 'manual'
            )",
                params![
                    id,
                    user_id,
                    flight.flight_number,
                    flight.departure_airport,
                    flight.arrival_airport,
                    flight.departure_datetime,
                    flight.arrival_datetime,
                    flight.aircraft_type_id,
                    flight.aircraft_registration,
                    total_duration,  // Use calculated value
                    flight_duration, // Use calculated value
                    flight.distance_nm,
                    distance_km,
                    flight.booking_reference,
                    flight.ticket_number,
                    flight.seat_number,
                    flight.fare_class,
                    flight.base_fare,
                    flight.taxes,
                    flight.total_cost,
                    flight.currency,
                    carbon_emissions_kg,
                    flight.notes,
                    flight.attachment_path
                ],
            )
            .context("Failed to create flight")?;

        Ok(id)
    }

    pub fn get_flight(&self, flight_id: &str) -> Result<Option<Flight>> {
        let flight = self.conn
            .query_row(
                "SELECT id, user_id, flight_number, departure_airport, arrival_airport,
                        departure_datetime, arrival_datetime, aircraft_type_id, aircraft_registration,
                        total_duration, flight_duration, block_duration, distance_nm, distance_km,
                        booking_reference, ticket_number, seat_number, fare_class,
                        base_fare, taxes, total_cost, currency, carbon_emissions_kg, per_passenger_co2_kg,
                        carbon_offset_purchased, frequent_flyer_program, miles_earned,
                        notes, attachment_path, data_source, verified, created_at, updated_at
                 FROM flights WHERE id = ?1",
                params![flight_id],
                |row| {
                    Ok(Flight {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        flight_number: row.get(2)?,
                        departure_airport: row.get(3)?,
                        arrival_airport: row.get(4)?,
                        departure_datetime: row.get(5)?,
                        arrival_datetime: row.get(6)?,
                        aircraft_type_id: row.get(7)?,
                        aircraft_registration: row.get(8)?,
                        total_duration: row.get(9)?,
                        flight_duration: row.get(10)?,
                        block_duration: row.get(11)?,
                        distance_nm: row.get(12)?,
                        distance_km: row.get(13)?,
                        booking_reference: row.get(14)?,
                        ticket_number: row.get(15)?,
                        seat_number: row.get(16)?,
                        fare_class: row.get(17)?,
                        base_fare: row.get(18)?,
                        taxes: row.get(19)?,
                        total_cost: row.get(20)?,
                        currency: row.get(21)?,
                        carbon_emissions_kg: row.get(22)?,
                        per_passenger_co2_kg: row.get(23)?,
                        carbon_offset_purchased: row.get(24)?,
                        frequent_flyer_program: row.get(25)?,
                        miles_earned: row.get(26)?,
                        notes: row.get(27)?,
                        attachment_path: row.get(28)?,
                        data_source: row.get(29)?,
                        verified: row.get(30)?,
                        created_at: row.get(31)?,
                        updated_at: row.get(32)?,
                    })
                }
            )
            .optional()
            .context("Failed to get flight")?;

        Ok(flight)
    }

    pub fn list_flights(&self, user_id: &str, limit: i32, offset: i32) -> Result<Vec<Flight>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, flight_number, departure_airport, arrival_airport,
                    departure_datetime, arrival_datetime, aircraft_type_id, aircraft_registration,
                    total_duration, flight_duration, block_duration, distance_nm, distance_km,
                    booking_reference, ticket_number, seat_number, fare_class,
                    base_fare, taxes, total_cost, currency, carbon_emissions_kg, per_passenger_co2_kg,
                    carbon_offset_purchased, frequent_flyer_program, miles_earned,
                    notes, attachment_path, data_source, verified, created_at, updated_at
             FROM flights
             WHERE user_id = ?1
             ORDER BY departure_datetime DESC
             LIMIT ?2 OFFSET ?3",
            )
            .context("Failed to prepare list flights query")?;

        let flights = stmt
            .query_map(params![user_id, limit, offset], |row| {
                Ok(Flight {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    flight_number: row.get(2)?,
                    departure_airport: row.get(3)?,
                    arrival_airport: row.get(4)?,
                    departure_datetime: row.get(5)?,
                    arrival_datetime: row.get(6)?,
                    aircraft_type_id: row.get(7)?,
                    aircraft_registration: row.get(8)?,
                    total_duration: row.get(9)?,
                    flight_duration: row.get(10)?,
                    block_duration: row.get(11)?,
                    distance_nm: row.get(12)?,
                    distance_km: row.get(13)?,
                    booking_reference: row.get(14)?,
                    ticket_number: row.get(15)?,
                    seat_number: row.get(16)?,
                    fare_class: row.get(17)?,
                    base_fare: row.get(18)?,
                    taxes: row.get(19)?,
                    total_cost: row.get(20)?,
                    currency: row.get(21)?,
                    carbon_emissions_kg: row.get(22)?,
                    per_passenger_co2_kg: row.get(23)?,
                    carbon_offset_purchased: row.get(24)?,
                    frequent_flyer_program: row.get(25)?,
                    miles_earned: row.get(26)?,
                    notes: row.get(27)?,
                    attachment_path: row.get(28)?,
                    data_source: row.get(29)?,
                    verified: row.get(30)?,
                    created_at: row.get(31)?,
                    updated_at: row.get(32)?,
                })
            })
            .context("Failed to query flights")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect flights")?;

        Ok(flights)
    }

    pub fn delete_flight(&self, flight_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM flights WHERE id = ?1", params![flight_id])
            .context("Failed to delete flight")?;

        Ok(())
    }

    // ===== STATISTICS OPERATIONS =====

    pub fn get_statistics(&self, user_id: &str) -> Result<FlightStatistics> {
        // Get basic flight stats
        let (total_flights, total_distance_nm, total_distance_km, total_flight_time_hours, total_carbon_kg): (i32, f64, f64, f64, f64) = self
            .conn
            .query_row(
                "SELECT
                COUNT(*) as total_flights,
                COALESCE(SUM(distance_nm), 0.0) as total_distance_nm,
                COALESCE(SUM(distance_km), 0.0) as total_distance_km,
                COALESCE(SUM(flight_duration), 0.0) / 60.0 as total_flight_time_hours,
                COALESCE(SUM(carbon_emissions_kg), 0.0) as total_carbon_kg
             FROM flights
             WHERE user_id = ?1",
                params![user_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .context("Failed to get basic statistics")?;

        // Count unique airports from BOTH departures AND arrivals
        let airports_visited: i32 = self
            .conn
            .query_row(
                "SELECT COUNT(DISTINCT airport_code) FROM (
                    SELECT departure_airport as airport_code FROM flights WHERE user_id = ?1
                    UNION
                    SELECT arrival_airport as airport_code FROM flights WHERE user_id = ?1
                )",
                params![user_id],
                |row| row.get(0),
            )
            .context("Failed to count airports")?;

        Ok(FlightStatistics {
            total_flights,
            total_distance_nm,
            total_distance_km,
            total_flight_time_hours,
            airports_visited,
            total_carbon_kg,
            countries_visited: 0,    // TODO: Calculate from airports
            favorite_aircraft: None, // TODO: Calculate
            favorite_route: None,    // TODO: Calculate
        })
    }

    // ===== RESEARCH REPORTS OPERATIONS =====

    pub fn save_research_report(
        &self,
        user_id: &str,
        input: &ResearchReportInput,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();

        // Convert research topics to JSON string
        let research_topics_json = if let Some(topics) = &input.research_topics {
            Some(serde_json::to_string(topics).context("Failed to serialize research topics")?)
        } else {
            None
        };

        // Convert sources to JSON string
        let sources_json = if let Some(sources) = &input.sources {
            Some(serde_json::to_string(sources).context("Failed to serialize sources")?)
        } else {
            None
        };

        let report_type = input.report_type.as_deref().unwrap_or("general");

        self.conn
            .execute(
                "INSERT INTO research_reports
             (id, user_id, agent_name, agent_model, search_query, research_topics,
              report_summary, report_details, sources, confidence_score, flight_id,
              report_type, processing_time_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    id,
                    user_id,
                    input.agent_name,
                    input.agent_model,
                    input.search_query,
                    research_topics_json,
                    input.report_summary,
                    input.report_details,
                    sources_json,
                    input.confidence_score,
                    input.flight_id,
                    report_type,
                    input.processing_time_ms,
                ],
            )
            .context("Failed to save research report")?;

        Ok(id)
    }

    pub fn get_research_report(&self, report_id: &str) -> Result<Option<ResearchReport>> {
        let report = self
            .conn
            .query_row(
                "SELECT id, user_id, agent_name, agent_model, search_query, research_topics,
                        report_summary, report_details, sources, confidence_score, flight_id,
                        report_type, processing_time_ms, created_at
                 FROM research_reports WHERE id = ?1",
                params![report_id],
                |row| {
                    Ok(ResearchReport {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        agent_name: row.get(2)?,
                        agent_model: row.get(3)?,
                        search_query: row.get(4)?,
                        research_topics: row.get(5)?,
                        report_summary: row.get(6)?,
                        report_details: row.get(7)?,
                        sources: row.get(8)?,
                        confidence_score: row.get(9)?,
                        flight_id: row.get(10)?,
                        report_type: row.get(11)?,
                        processing_time_ms: row.get(12)?,
                        created_at: row.get(13)?,
                    })
                },
            )
            .optional()
            .context("Failed to get research report")?;

        Ok(report)
    }

    pub fn list_research_reports(
        &self,
        user_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ResearchReport>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, agent_name, agent_model, search_query, research_topics,
                    report_summary, report_details, sources, confidence_score, flight_id,
                    report_type, processing_time_ms, created_at
             FROM research_reports
             WHERE user_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2 OFFSET ?3",
            )
            .context("Failed to prepare statement")?;

        let reports = stmt
            .query_map(params![user_id, limit, offset], |row| {
                Ok(ResearchReport {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    agent_name: row.get(2)?,
                    agent_model: row.get(3)?,
                    search_query: row.get(4)?,
                    research_topics: row.get(5)?,
                    report_summary: row.get(6)?,
                    report_details: row.get(7)?,
                    sources: row.get(8)?,
                    confidence_score: row.get(9)?,
                    flight_id: row.get(10)?,
                    report_type: row.get(11)?,
                    processing_time_ms: row.get(12)?,
                    created_at: row.get(13)?,
                })
            })
            .context("Failed to query research reports")?;

        let mut result = Vec::new();
        for report in reports {
            result.push(report.context("Failed to map research report")?);
        }

        Ok(result)
    }

    pub fn delete_research_report(&self, report_id: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM research_reports WHERE id = ?1",
                params![report_id],
            )
            .context("Failed to delete research report")?;

        Ok(())
    }

    pub fn count_research_reports(&self, user_id: &str) -> Result<i64> {
        let count: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM research_reports WHERE user_id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .context("Failed to count research reports")?;

        Ok(count)
    }

    // ===== JOURNEY OPERATIONS =====

    pub fn create_journey(
        &self,
        user_id: &str,
        name: &str,
        description: Option<&str>,
        start_date: &str,
        end_date: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();

        self.conn
            .execute(
                "INSERT INTO journeys (id, user_id, name, description, start_date, end_date, is_favorite)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                params![id, user_id, name, description, start_date, end_date],
            )
            .context("Failed to create journey")?;

        Ok(id)
    }

    pub fn get_journey(&self, journey_id: &str) -> Result<Option<Journey>> {
        let journey = self
            .conn
            .query_row(
                "SELECT id, user_id, name, description, start_date, end_date,
                        is_favorite, thumbnail_path, created_at, updated_at
                 FROM journeys WHERE id = ?1",
                params![journey_id],
                |row| {
                    Ok(Journey {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        name: row.get(2)?,
                        description: row.get(3)?,
                        start_date: row.get(4)?,
                        end_date: row.get(5)?,
                        is_favorite: row.get(6)?,
                        thumbnail_path: row.get(7)?,
                        created_at: row.get(8)?,
                        updated_at: row.get(9)?,
                    })
                },
            )
            .optional()
            .context("Failed to get journey")?;

        Ok(journey)
    }

    pub fn list_user_journeys(&self, user_id: &str) -> Result<Vec<Journey>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, name, description, start_date, end_date,
                        is_favorite, thumbnail_path, created_at, updated_at
                 FROM journeys
                 WHERE user_id = ?1
                 ORDER BY start_date DESC",
            )
            .context("Failed to prepare journey list query")?;

        let journeys = stmt
            .query_map(params![user_id], |row| {
                Ok(Journey {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    start_date: row.get(4)?,
                    end_date: row.get(5)?,
                    is_favorite: row.get(6)?,
                    thumbnail_path: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })
            .context("Failed to query journeys")?;

        let mut result = Vec::new();
        for journey in journeys {
            result.push(journey.context("Failed to map journey")?);
        }

        Ok(result)
    }

    pub fn update_journey(
        &self,
        journey_id: &str,
        name: &str,
        description: Option<&str>,
        start_date: &str,
        end_date: Option<&str>,
        is_favorite: i32,
    ) -> Result<()> {
        self.conn
            .execute(
                "UPDATE journeys
                 SET name = ?1, description = ?2, start_date = ?3, end_date = ?4,
                     is_favorite = ?5, updated_at = datetime('now')
                 WHERE id = ?6",
                params![name, description, start_date, end_date, is_favorite, journey_id],
            )
            .context("Failed to update journey")?;

        Ok(())
    }

    pub fn delete_journey(&self, journey_id: &str) -> Result<()> {
        // Delete journey (cascade will handle journey_flights)
        self.conn
            .execute("DELETE FROM journeys WHERE id = ?1", params![journey_id])
            .context("Failed to delete journey")?;

        Ok(())
    }

    pub fn add_flight_to_journey(
        &self,
        journey_id: &str,
        flight_id: &str,
        sequence_order: i32,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO journey_flights (journey_id, flight_id, sequence_order)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(journey_id, flight_id) DO UPDATE SET sequence_order = excluded.sequence_order",
                params![journey_id, flight_id, sequence_order],
            )
            .context("Failed to add flight to journey")?;

        Ok(())
    }

    pub fn remove_flight_from_journey(&self, journey_id: &str, flight_id: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM journey_flights WHERE journey_id = ?1 AND flight_id = ?2",
                params![journey_id, flight_id],
            )
            .context("Failed to remove flight from journey")?;

        Ok(())
    }

    pub fn get_journey_flights(&self, journey_id: &str) -> Result<Vec<Flight>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT f.id, f.user_id, f.flight_number, f.departure_airport, f.arrival_airport,
                        f.departure_datetime, f.arrival_datetime, f.aircraft_type_id, f.aircraft_registration,
                        f.total_duration, f.flight_duration, f.block_duration, f.distance_nm, f.distance_km,
                        f.booking_reference, f.ticket_number, f.seat_number, f.fare_class,
                        f.base_fare, f.taxes, f.total_cost, f.currency, f.carbon_emissions_kg, f.per_passenger_co2_kg,
                        f.carbon_offset_purchased, f.frequent_flyer_program, f.miles_earned,
                        f.notes, f.attachment_path, f.data_source, f.verified, f.created_at, f.updated_at
                 FROM flights f
                 INNER JOIN journey_flights jf ON f.id = jf.flight_id
                 WHERE jf.journey_id = ?1
                 ORDER BY jf.sequence_order ASC, f.departure_datetime ASC",
            )
            .context("Failed to prepare journey flights query")?;

        let flights = stmt
            .query_map(params![journey_id], |row| {
                Ok(Flight {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    flight_number: row.get(2)?,
                    departure_airport: row.get(3)?,
                    arrival_airport: row.get(4)?,
                    departure_datetime: row.get(5)?,
                    arrival_datetime: row.get(6)?,
                    aircraft_type_id: row.get(7)?,
                    aircraft_registration: row.get(8)?,
                    total_duration: row.get(9)?,
                    flight_duration: row.get(10)?,
                    block_duration: row.get(11)?,
                    distance_nm: row.get(12)?,
                    distance_km: row.get(13)?,
                    booking_reference: row.get(14)?,
                    ticket_number: row.get(15)?,
                    seat_number: row.get(16)?,
                    fare_class: row.get(17)?,
                    base_fare: row.get(18)?,
                    taxes: row.get(19)?,
                    total_cost: row.get(20)?,
                    currency: row.get(21)?,
                    carbon_emissions_kg: row.get(22)?,
                    per_passenger_co2_kg: row.get(23)?,
                    carbon_offset_purchased: row.get(24)?,
                    frequent_flyer_program: row.get(25)?,
                    miles_earned: row.get(26)?,
                    notes: row.get(27)?,
                    attachment_path: row.get(28)?,
                    data_source: row.get(29)?,
                    verified: row.get(30)?,
                    created_at: row.get(31)?,
                    updated_at: row.get(32)?,
                })
            })
            .context("Failed to query journey flights")?;

        let mut result = Vec::new();
        for flight in flights {
            result.push(flight.context("Failed to map flight")?);
        }

        Ok(result)
    }

    // ===== PILOT LOGBOOK OPERATIONS =====

    pub fn create_pilot_logbook_entry(
        &self,
        flight_id: &str,
        pic_time: f64,
        sic_time: f64,
        dual_time: f64,
        instructor_time: f64,
        solo_time: f64,
        cross_country_time: f64,
        day_time: f64,
        night_time: f64,
        actual_instrument_time: f64,
        simulated_instrument_time: f64,
        ground_trainer_time: f64,
        day_takeoffs: i32,
        day_landings: i32,
        night_takeoffs: i32,
        night_landings: i32,
        ils_approaches: i32,
        vor_approaches: i32,
        ndb_approaches: i32,
        gps_approaches: i32,
        visual_approaches: i32,
        ifr_time: f64,
        vfr_time: f64,
        pilot_name: Option<&str>,
        copilot_name: Option<&str>,
        instructor_name: Option<&str>,
        route: Option<&str>,
        remarks: Option<&str>,
        endorsements: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO pilot_logbook (
                id, flight_id, pic_time, sic_time, dual_time, instructor_time, solo_time,
                cross_country_time, day_time, night_time, actual_instrument_time,
                simulated_instrument_time, ground_trainer_time, day_takeoffs, day_landings,
                night_takeoffs, night_landings, ils_approaches, vor_approaches, ndb_approaches,
                gps_approaches, visual_approaches, ifr_time, vfr_time, pilot_name, copilot_name,
                instructor_name, route, remarks, endorsements
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30
            )",
            params![
                id, flight_id, pic_time, sic_time, dual_time, instructor_time, solo_time,
                cross_country_time, day_time, night_time, actual_instrument_time,
                simulated_instrument_time, ground_trainer_time, day_takeoffs, day_landings,
                night_takeoffs, night_landings, ils_approaches, vor_approaches, ndb_approaches,
                gps_approaches, visual_approaches, ifr_time, vfr_time, pilot_name, copilot_name,
                instructor_name, route, remarks, endorsements
            ],
        )
        .context("Failed to create pilot logbook entry")?;
        Ok(id)
    }

    pub fn get_pilot_logbook_entry(&self, entry_id: &str) -> Result<Option<PilotLogbook>> {
        let result = self.conn.query_row(
            "SELECT id, flight_id, pic_time, sic_time, dual_time, instructor_time, solo_time,
                    cross_country_time, day_time, night_time, actual_instrument_time,
                    simulated_instrument_time, ground_trainer_time, day_takeoffs, day_landings,
                    night_takeoffs, night_landings, ils_approaches, vor_approaches, ndb_approaches,
                    gps_approaches, visual_approaches, ifr_time, vfr_time, pilot_name, copilot_name,
                    instructor_name, route, remarks, endorsements
             FROM pilot_logbook
             WHERE id = ?1",
            params![entry_id],
            |row| {
                Ok(PilotLogbook {
                    id: row.get(0)?,
                    flight_id: row.get(1)?,
                    pic_time: row.get(2)?,
                    sic_time: row.get(3)?,
                    dual_time: row.get(4)?,
                    instructor_time: row.get(5)?,
                    solo_time: row.get(6)?,
                    cross_country_time: row.get(7)?,
                    day_time: row.get(8)?,
                    night_time: row.get(9)?,
                    actual_instrument_time: row.get(10)?,
                    simulated_instrument_time: row.get(11)?,
                    ground_trainer_time: row.get(12)?,
                    day_takeoffs: row.get(13)?,
                    day_landings: row.get(14)?,
                    night_takeoffs: row.get(15)?,
                    night_landings: row.get(16)?,
                    ils_approaches: row.get(17)?,
                    vor_approaches: row.get(18)?,
                    ndb_approaches: row.get(19)?,
                    gps_approaches: row.get(20)?,
                    visual_approaches: row.get(21)?,
                    ifr_time: row.get(22)?,
                    vfr_time: row.get(23)?,
                    pilot_name: row.get(24)?,
                    copilot_name: row.get(25)?,
                    instructor_name: row.get(26)?,
                    route: row.get(27)?,
                    remarks: row.get(28)?,
                    endorsements: row.get(29)?,
                })
            },
        );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_pilot_logbook_by_flight(&self, flight_id: &str) -> Result<Option<PilotLogbook>> {
        let result = self.conn.query_row(
            "SELECT id, flight_id, pic_time, sic_time, dual_time, instructor_time, solo_time,
                    cross_country_time, day_time, night_time, actual_instrument_time,
                    simulated_instrument_time, ground_trainer_time, day_takeoffs, day_landings,
                    night_takeoffs, night_landings, ils_approaches, vor_approaches, ndb_approaches,
                    gps_approaches, visual_approaches, ifr_time, vfr_time, pilot_name, copilot_name,
                    instructor_name, route, remarks, endorsements
             FROM pilot_logbook
             WHERE flight_id = ?1",
            params![flight_id],
            |row| {
                Ok(PilotLogbook {
                    id: row.get(0)?,
                    flight_id: row.get(1)?,
                    pic_time: row.get(2)?,
                    sic_time: row.get(3)?,
                    dual_time: row.get(4)?,
                    instructor_time: row.get(5)?,
                    solo_time: row.get(6)?,
                    cross_country_time: row.get(7)?,
                    day_time: row.get(8)?,
                    night_time: row.get(9)?,
                    actual_instrument_time: row.get(10)?,
                    simulated_instrument_time: row.get(11)?,
                    ground_trainer_time: row.get(12)?,
                    day_takeoffs: row.get(13)?,
                    day_landings: row.get(14)?,
                    night_takeoffs: row.get(15)?,
                    night_landings: row.get(16)?,
                    ils_approaches: row.get(17)?,
                    vor_approaches: row.get(18)?,
                    ndb_approaches: row.get(19)?,
                    gps_approaches: row.get(20)?,
                    visual_approaches: row.get(21)?,
                    ifr_time: row.get(22)?,
                    vfr_time: row.get(23)?,
                    pilot_name: row.get(24)?,
                    copilot_name: row.get(25)?,
                    instructor_name: row.get(26)?,
                    route: row.get(27)?,
                    remarks: row.get(28)?,
                    endorsements: row.get(29)?,
                })
            },
        );

        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_all_pilot_logbook_entries(&self) -> Result<Vec<PilotLogbook>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, flight_id, pic_time, sic_time, dual_time, instructor_time, solo_time,
                    cross_country_time, day_time, night_time, actual_instrument_time,
                    simulated_instrument_time, ground_trainer_time, day_takeoffs, day_landings,
                    night_takeoffs, night_landings, ils_approaches, vor_approaches, ndb_approaches,
                    gps_approaches, visual_approaches, ifr_time, vfr_time, pilot_name, copilot_name,
                    instructor_name, route, remarks, endorsements
             FROM pilot_logbook
             ORDER BY id DESC",
        )?;

        let entries = stmt
            .query_map([], |row| {
                Ok(PilotLogbook {
                    id: row.get(0)?,
                    flight_id: row.get(1)?,
                    pic_time: row.get(2)?,
                    sic_time: row.get(3)?,
                    dual_time: row.get(4)?,
                    instructor_time: row.get(5)?,
                    solo_time: row.get(6)?,
                    cross_country_time: row.get(7)?,
                    day_time: row.get(8)?,
                    night_time: row.get(9)?,
                    actual_instrument_time: row.get(10)?,
                    simulated_instrument_time: row.get(11)?,
                    ground_trainer_time: row.get(12)?,
                    day_takeoffs: row.get(13)?,
                    day_landings: row.get(14)?,
                    night_takeoffs: row.get(15)?,
                    night_landings: row.get(16)?,
                    ils_approaches: row.get(17)?,
                    vor_approaches: row.get(18)?,
                    ndb_approaches: row.get(19)?,
                    gps_approaches: row.get(20)?,
                    visual_approaches: row.get(21)?,
                    ifr_time: row.get(22)?,
                    vfr_time: row.get(23)?,
                    pilot_name: row.get(24)?,
                    copilot_name: row.get(25)?,
                    instructor_name: row.get(26)?,
                    route: row.get(27)?,
                    remarks: row.get(28)?,
                    endorsements: row.get(29)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    pub fn update_pilot_logbook_entry(
        &self,
        entry_id: &str,
        pic_time: f64,
        sic_time: f64,
        dual_time: f64,
        instructor_time: f64,
        solo_time: f64,
        cross_country_time: f64,
        day_time: f64,
        night_time: f64,
        actual_instrument_time: f64,
        simulated_instrument_time: f64,
        ground_trainer_time: f64,
        day_takeoffs: i32,
        day_landings: i32,
        night_takeoffs: i32,
        night_landings: i32,
        ils_approaches: i32,
        vor_approaches: i32,
        ndb_approaches: i32,
        gps_approaches: i32,
        visual_approaches: i32,
        ifr_time: f64,
        vfr_time: f64,
        pilot_name: Option<&str>,
        copilot_name: Option<&str>,
        instructor_name: Option<&str>,
        route: Option<&str>,
        remarks: Option<&str>,
        endorsements: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE pilot_logbook SET
                pic_time = ?2, sic_time = ?3, dual_time = ?4, instructor_time = ?5, solo_time = ?6,
                cross_country_time = ?7, day_time = ?8, night_time = ?9, actual_instrument_time = ?10,
                simulated_instrument_time = ?11, ground_trainer_time = ?12, day_takeoffs = ?13,
                day_landings = ?14, night_takeoffs = ?15, night_landings = ?16, ils_approaches = ?17,
                vor_approaches = ?18, ndb_approaches = ?19, gps_approaches = ?20, visual_approaches = ?21,
                ifr_time = ?22, vfr_time = ?23, pilot_name = ?24, copilot_name = ?25,
                instructor_name = ?26, route = ?27, remarks = ?28, endorsements = ?29
             WHERE id = ?1",
            params![
                entry_id, pic_time, sic_time, dual_time, instructor_time, solo_time,
                cross_country_time, day_time, night_time, actual_instrument_time,
                simulated_instrument_time, ground_trainer_time, day_takeoffs, day_landings,
                night_takeoffs, night_landings, ils_approaches, vor_approaches, ndb_approaches,
                gps_approaches, visual_approaches, ifr_time, vfr_time, pilot_name, copilot_name,
                instructor_name, route, remarks, endorsements
            ],
        )
        .context("Failed to update pilot logbook entry")?;
        Ok(())
    }

    pub fn delete_pilot_logbook_entry(&self, entry_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM pilot_logbook WHERE id = ?1", params![entry_id])
            .context("Failed to delete pilot logbook entry")?;
        Ok(())
    }

    pub fn get_pilot_logbook_totals(&self) -> Result<PilotLogbookTotals> {
        let result = self.conn.query_row(
            "SELECT
                COALESCE(SUM(pic_time), 0.0),
                COALESCE(SUM(sic_time), 0.0),
                COALESCE(SUM(dual_time), 0.0),
                COALESCE(SUM(instructor_time), 0.0),
                COALESCE(SUM(solo_time), 0.0),
                COALESCE(SUM(cross_country_time), 0.0),
                COALESCE(SUM(day_time), 0.0),
                COALESCE(SUM(night_time), 0.0),
                COALESCE(SUM(actual_instrument_time), 0.0),
                COALESCE(SUM(simulated_instrument_time), 0.0),
                COALESCE(SUM(ground_trainer_time), 0.0),
                COALESCE(SUM(day_takeoffs), 0),
                COALESCE(SUM(day_landings), 0),
                COALESCE(SUM(night_takeoffs), 0),
                COALESCE(SUM(night_landings), 0),
                COALESCE(SUM(ils_approaches), 0),
                COALESCE(SUM(vor_approaches), 0),
                COALESCE(SUM(ndb_approaches), 0),
                COALESCE(SUM(gps_approaches), 0),
                COALESCE(SUM(visual_approaches), 0),
                COALESCE(SUM(ifr_time), 0.0),
                COALESCE(SUM(vfr_time), 0.0)
             FROM pilot_logbook",
            [],
            |row| {
                Ok(PilotLogbookTotals {
                    total_pic_time: row.get(0)?,
                    total_sic_time: row.get(1)?,
                    total_dual_time: row.get(2)?,
                    total_instructor_time: row.get(3)?,
                    total_solo_time: row.get(4)?,
                    total_cross_country_time: row.get(5)?,
                    total_day_time: row.get(6)?,
                    total_night_time: row.get(7)?,
                    total_actual_instrument_time: row.get(8)?,
                    total_simulated_instrument_time: row.get(9)?,
                    total_ground_trainer_time: row.get(10)?,
                    total_day_takeoffs: row.get(11)?,
                    total_day_landings: row.get(12)?,
                    total_night_takeoffs: row.get(13)?,
                    total_night_landings: row.get(14)?,
                    total_ils_approaches: row.get(15)?,
                    total_vor_approaches: row.get(16)?,
                    total_ndb_approaches: row.get(17)?,
                    total_gps_approaches: row.get(18)?,
                    total_visual_approaches: row.get(19)?,
                    total_ifr_time: row.get(20)?,
                    total_vfr_time: row.get(21)?,
                })
            },
        )?;

        Ok(result)
    }

    // ===== FREQUENT FLYER PROGRAM OPERATIONS =====

    pub fn create_ffp(
        &self,
        user_id: &str,
        program_name: &str,
        airline: Option<&str>,
        alliance: Option<&str>,
        member_number: Option<&str>,
        tier_status: Option<&str>,
        current_miles: f64,
        lifetime_miles: f64,
        tier_miles: f64,
        tier_expiry_date: Option<&str>,
        notes: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO frequent_flyer_programs (
                id, user_id, program_name, airline, alliance, member_number, tier_status,
                current_miles, lifetime_miles, tier_miles, tier_expiry_date, notes
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                id, user_id, program_name, airline, alliance, member_number, tier_status,
                current_miles, lifetime_miles, tier_miles, tier_expiry_date, notes
            ],
        )
        .context("Failed to create FFP")?;
        Ok(id)
    }

    pub fn get_ffp(&self, ffp_id: &str) -> Result<Option<FrequentFlyerProgram>> {
        let result = self.conn.query_row(
            "SELECT id, user_id, program_name, airline, alliance, member_number, tier_status,
                    current_miles, lifetime_miles, tier_miles, tier_expiry_date, notes
             FROM frequent_flyer_programs
             WHERE id = ?1",
            params![ffp_id],
            |row| {
                Ok(FrequentFlyerProgram {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    program_name: row.get(2)?,
                    airline: row.get(3)?,
                    alliance: row.get(4)?,
                    member_number: row.get(5)?,
                    tier_status: row.get(6)?,
                    current_miles: row.get(7)?,
                    lifetime_miles: row.get(8)?,
                    tier_miles: row.get(9)?,
                    tier_expiry_date: row.get(10)?,
                    notes: row.get(11)?,
                })
            },
        );

        match result {
            Ok(ffp) => Ok(Some(ffp)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_user_ffps(&self, user_id: &str) -> Result<Vec<FrequentFlyerProgram>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_id, program_name, airline, alliance, member_number, tier_status,
                    current_miles, lifetime_miles, tier_miles, tier_expiry_date, notes
             FROM frequent_flyer_programs
             WHERE user_id = ?1
             ORDER BY program_name ASC",
        )?;

        let ffps = stmt
            .query_map(params![user_id], |row| {
                Ok(FrequentFlyerProgram {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    program_name: row.get(2)?,
                    airline: row.get(3)?,
                    alliance: row.get(4)?,
                    member_number: row.get(5)?,
                    tier_status: row.get(6)?,
                    current_miles: row.get(7)?,
                    lifetime_miles: row.get(8)?,
                    tier_miles: row.get(9)?,
                    tier_expiry_date: row.get(10)?,
                    notes: row.get(11)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(ffps)
    }

    pub fn update_ffp(
        &self,
        ffp_id: &str,
        program_name: &str,
        airline: Option<&str>,
        alliance: Option<&str>,
        member_number: Option<&str>,
        tier_status: Option<&str>,
        current_miles: f64,
        lifetime_miles: f64,
        tier_miles: f64,
        tier_expiry_date: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE frequent_flyer_programs SET
                program_name = ?2, airline = ?3, alliance = ?4, member_number = ?5,
                tier_status = ?6, current_miles = ?7, lifetime_miles = ?8, tier_miles = ?9,
                tier_expiry_date = ?10, notes = ?11
             WHERE id = ?1",
            params![
                ffp_id, program_name, airline, alliance, member_number, tier_status,
                current_miles, lifetime_miles, tier_miles, tier_expiry_date, notes
            ],
        )
        .context("Failed to update FFP")?;
        Ok(())
    }

    pub fn delete_ffp(&self, ffp_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM frequent_flyer_programs WHERE id = ?1", params![ffp_id])
            .context("Failed to delete FFP")?;
        Ok(())
    }

    // ===== AIRPORT OPERATIONS =====

    pub fn create_airport(
        &self,
        icao_code: Option<&str>,
        iata_code: Option<&str>,
        name: &str,
        city: Option<&str>,
        country: Option<&str>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        timezone: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO airports (id, icao_code, iata_code, name, city, country, latitude, longitude, timezone)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, icao_code, iata_code, name, city, country, latitude, longitude, timezone],
        )
        .context("Failed to create airport")?;
        Ok(id)
    }

    pub fn get_airport(&self, airport_id: &str) -> Result<Option<Airport>> {
        let result = self.conn.query_row(
            "SELECT id, icao_code, iata_code, name, city, country, latitude, longitude, timezone
             FROM airports WHERE id = ?1",
            params![airport_id],
            |row| {
                Ok(Airport {
                    id: row.get(0)?,
                    icao_code: row.get(1)?,
                    iata_code: row.get(2)?,
                    name: row.get(3)?,
                    city: row.get(4)?,
                    country: row.get(5)?,
                    latitude: row.get(6)?,
                    longitude: row.get(7)?,
                    timezone: row.get(8)?,
                })
            },
        );

        match result {
            Ok(airport) => Ok(Some(airport)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_all_airports(&self) -> Result<Vec<Airport>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, icao_code, iata_code, name, city, country, latitude, longitude, timezone
             FROM airports ORDER BY name ASC",
        )?;

        let airports = stmt
            .query_map([], |row| {
                Ok(Airport {
                    id: row.get(0)?,
                    icao_code: row.get(1)?,
                    iata_code: row.get(2)?,
                    name: row.get(3)?,
                    city: row.get(4)?,
                    country: row.get(5)?,
                    latitude: row.get(6)?,
                    longitude: row.get(7)?,
                    timezone: row.get(8)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(airports)
    }

    pub fn update_airport(
        &self,
        airport_id: &str,
        icao_code: Option<&str>,
        iata_code: Option<&str>,
        name: &str,
        city: Option<&str>,
        country: Option<&str>,
        latitude: Option<f64>,
        longitude: Option<f64>,
        timezone: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE airports SET
                icao_code = ?2, iata_code = ?3, name = ?4, city = ?5, country = ?6,
                latitude = ?7, longitude = ?8, timezone = ?9
             WHERE id = ?1",
            params![airport_id, icao_code, iata_code, name, city, country, latitude, longitude, timezone],
        )
        .context("Failed to update airport")?;
        Ok(())
    }

    pub fn delete_airport(&self, airport_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM airports WHERE id = ?1", params![airport_id])
            .context("Failed to delete airport")?;
        Ok(())
    }

    // ===== AIRCRAFT TYPE OPERATIONS =====

    pub fn create_aircraft_type(
        &self,
        manufacturer: &str,
        model: &str,
        type_designator: Option<&str>,
        category: Option<&str>,
        class: Option<&str>,
        notes: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO aircraft_types (id, manufacturer, model, type_designator, category, class, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, manufacturer, model, type_designator, category, class, notes],
        )
        .context("Failed to create aircraft type")?;
        Ok(id)
    }

    pub fn get_aircraft_type(&self, aircraft_type_id: &str) -> Result<Option<AircraftType>> {
        let result = self.conn.query_row(
            "SELECT id, manufacturer, model, type_designator, category, class, notes
             FROM aircraft_types WHERE id = ?1",
            params![aircraft_type_id],
            |row| {
                Ok(AircraftType {
                    id: row.get(0)?,
                    manufacturer: row.get(1)?,
                    model: row.get(2)?,
                    type_designator: row.get(3)?,
                    category: row.get(4)?,
                    class: row.get(5)?,
                    notes: row.get(6)?,
                })
            },
        );

        match result {
            Ok(aircraft) => Ok(Some(aircraft)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_all_aircraft_types(&self) -> Result<Vec<AircraftType>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, manufacturer, model, type_designator, category, class, notes
             FROM aircraft_types ORDER BY manufacturer, model ASC",
        )?;

        let aircraft_types = stmt
            .query_map([], |row| {
                Ok(AircraftType {
                    id: row.get(0)?,
                    manufacturer: row.get(1)?,
                    model: row.get(2)?,
                    type_designator: row.get(3)?,
                    category: row.get(4)?,
                    class: row.get(5)?,
                    notes: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(aircraft_types)
    }

    pub fn update_aircraft_type(
        &self,
        aircraft_type_id: &str,
        manufacturer: &str,
        model: &str,
        type_designator: Option<&str>,
        category: Option<&str>,
        class: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE aircraft_types SET
                manufacturer = ?2, model = ?3, type_designator = ?4, category = ?5, class = ?6, notes = ?7
             WHERE id = ?1",
            params![aircraft_type_id, manufacturer, model, type_designator, category, class, notes],
        )
        .context("Failed to update aircraft type")?;
        Ok(())
    }

    pub fn delete_aircraft_type(&self, aircraft_type_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM aircraft_types WHERE id = ?1", params![aircraft_type_id])
            .context("Failed to delete aircraft type")?;
        Ok(())
    }

    // ===== CUSTOM DOCUMENTS OPERATIONS =====

    pub fn create_custom_document(
        &self,
        user_id: &str,
        title: &str,
        content: &str,
        category: Option<&str>,
        tags: Option<&str>,
        flight_id: Option<&str>,
        journey_id: Option<&str>,
        passenger_name: Option<&str>,
        fuel_entry_id: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();

        self.conn.execute(
            "INSERT INTO custom_documents (id, user_id, title, content, category, tags, flight_id, journey_id, passenger_name, fuel_entry_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, user_id, title, content, category, tags, flight_id, journey_id, passenger_name, fuel_entry_id],
        ).context("Failed to create custom document")?;

        Ok(id)
    }

    pub fn get_custom_document(&self, document_id: &str) -> Result<Option<CustomDocument>> {
        let document = self
            .conn
            .query_row(
                "SELECT id, user_id, title, content, category, tags, flight_id, journey_id, passenger_name, fuel_entry_id, created_at, updated_at
                 FROM custom_documents WHERE id = ?1",
                params![document_id],
                |row| {
                    Ok(CustomDocument {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        title: row.get(2)?,
                        content: row.get(3)?,
                        category: row.get(4)?,
                        tags: row.get(5)?,
                        flight_id: row.get(6)?,
                        journey_id: row.get(7)?,
                        passenger_name: row.get(8)?,
                        fuel_entry_id: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                },
            )
            .optional()
            .context("Failed to get custom document")?;

        Ok(document)
    }

    pub fn list_user_documents(&self, user_id: &str) -> Result<Vec<CustomDocument>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, title, content, category, tags, flight_id, journey_id, passenger_name, fuel_entry_id, created_at, updated_at
                 FROM custom_documents WHERE user_id = ?1
                 ORDER BY created_at DESC",
            )
            .context("Failed to prepare list user documents query")?;

        let documents = stmt
            .query_map(params![user_id], |row| {
                Ok(CustomDocument {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    category: row.get(4)?,
                    tags: row.get(5)?,
                    flight_id: row.get(6)?,
                    journey_id: row.get(7)?,
                    passenger_name: row.get(8)?,
                    fuel_entry_id: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(documents)
    }

    pub fn update_custom_document(
        &self,
        document_id: &str,
        title: &str,
        content: &str,
        category: Option<&str>,
        tags: Option<&str>,
        flight_id: Option<&str>,
        journey_id: Option<&str>,
        passenger_name: Option<&str>,
        fuel_entry_id: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE custom_documents SET
                title = ?2, content = ?3, category = ?4, tags = ?5, flight_id = ?6, journey_id = ?7, passenger_name = ?8, fuel_entry_id = ?9, updated_at = datetime('now')
             WHERE id = ?1",
            params![document_id, title, content, category, tags, flight_id, journey_id, passenger_name, fuel_entry_id],
        )
        .context("Failed to update custom document")?;
        Ok(())
    }

    pub fn delete_custom_document(&self, document_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM custom_documents WHERE id = ?1", params![document_id])
            .context("Failed to delete custom document")?;
        Ok(())
    }

    // ===== FUEL PRICE OPERATIONS =====

    pub fn create_fuel_price(
        &self,
        user_id: &str,
        fuel_type: &str,
        price_per_unit: f64,
        unit: &str,
        currency: &str,
        start_date: &str,
        end_date: Option<&str>,
        location: Option<&str>,
        supplier: Option<&str>,
        notes: Option<&str>,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();

        self.conn.execute(
            "INSERT INTO fuel_prices (id, user_id, fuel_type, price_per_unit, unit, currency, start_date, end_date, location, supplier, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![id, user_id, fuel_type, price_per_unit, unit, currency, start_date, end_date, location, supplier, notes],
        ).context("Failed to create fuel price")?;

        Ok(id)
    }

    pub fn get_fuel_price(&self, fuel_price_id: &str) -> Result<Option<FuelPrice>> {
        let fuel_price = self
            .conn
            .query_row(
                "SELECT id, user_id, fuel_type, price_per_unit, unit, currency, start_date, end_date, location, supplier, notes, created_at, updated_at
                 FROM fuel_prices WHERE id = ?1",
                params![fuel_price_id],
                |row| {
                    Ok(FuelPrice {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        fuel_type: row.get(2)?,
                        price_per_unit: row.get(3)?,
                        unit: row.get(4)?,
                        currency: row.get(5)?,
                        start_date: row.get(6)?,
                        end_date: row.get(7)?,
                        location: row.get(8)?,
                        supplier: row.get(9)?,
                        notes: row.get(10)?,
                        created_at: row.get(11)?,
                        updated_at: row.get(12)?,
                    })
                },
            )
            .optional()
            .context("Failed to get fuel price")?;

        Ok(fuel_price)
    }

    pub fn list_fuel_prices(&self, user_id: &str) -> Result<Vec<FuelPrice>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, user_id, fuel_type, price_per_unit, unit, currency, start_date, end_date, location, supplier, notes, created_at, updated_at
                 FROM fuel_prices WHERE user_id = ?1
                 ORDER BY start_date DESC",
            )
            .context("Failed to prepare list fuel prices query")?;

        let fuel_prices = stmt
            .query_map(params![user_id], |row| {
                Ok(FuelPrice {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    fuel_type: row.get(2)?,
                    price_per_unit: row.get(3)?,
                    unit: row.get(4)?,
                    currency: row.get(5)?,
                    start_date: row.get(6)?,
                    end_date: row.get(7)?,
                    location: row.get(8)?,
                    supplier: row.get(9)?,
                    notes: row.get(10)?,
                    created_at: row.get(11)?,
                    updated_at: row.get(12)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(fuel_prices)
    }

    pub fn get_fuel_price_for_date(
        &self,
        user_id: &str,
        date: &str,
        fuel_type: Option<&str>,
        location: Option<&str>,
    ) -> Result<Option<FuelPrice>> {
        let mut query = String::from(
            "SELECT id, user_id, fuel_type, price_per_unit, unit, currency, start_date, end_date, location, supplier, notes, created_at, updated_at
             FROM fuel_prices
             WHERE user_id = ?1
             AND start_date <= ?2
             AND (end_date IS NULL OR end_date >= ?2)"
        );

        let mut param_count = 2;
        if fuel_type.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND fuel_type = ?{}", param_count));
        }
        if location.is_some() {
            param_count += 1;
            query.push_str(&format!(" AND location = ?{}", param_count));
        }

        query.push_str(" ORDER BY start_date DESC LIMIT 1");

        let fuel_price = match (fuel_type, location) {
            (Some(ft), Some(loc)) => {
                self.conn.query_row(
                    &query,
                    params![user_id, date, ft, loc],
                    |row| {
                        Ok(FuelPrice {
                            id: row.get(0)?,
                            user_id: row.get(1)?,
                            fuel_type: row.get(2)?,
                            price_per_unit: row.get(3)?,
                            unit: row.get(4)?,
                            currency: row.get(5)?,
                            start_date: row.get(6)?,
                            end_date: row.get(7)?,
                            location: row.get(8)?,
                            supplier: row.get(9)?,
                            notes: row.get(10)?,
                            created_at: row.get(11)?,
                            updated_at: row.get(12)?,
                        })
                    },
                )
            }
            (Some(ft), None) => {
                self.conn.query_row(
                    &query,
                    params![user_id, date, ft],
                    |row| {
                        Ok(FuelPrice {
                            id: row.get(0)?,
                            user_id: row.get(1)?,
                            fuel_type: row.get(2)?,
                            price_per_unit: row.get(3)?,
                            unit: row.get(4)?,
                            currency: row.get(5)?,
                            start_date: row.get(6)?,
                            end_date: row.get(7)?,
                            location: row.get(8)?,
                            supplier: row.get(9)?,
                            notes: row.get(10)?,
                            created_at: row.get(11)?,
                            updated_at: row.get(12)?,
                        })
                    },
                )
            }
            (None, Some(loc)) => {
                self.conn.query_row(
                    &query,
                    params![user_id, date, loc],
                    |row| {
                        Ok(FuelPrice {
                            id: row.get(0)?,
                            user_id: row.get(1)?,
                            fuel_type: row.get(2)?,
                            price_per_unit: row.get(3)?,
                            unit: row.get(4)?,
                            currency: row.get(5)?,
                            start_date: row.get(6)?,
                            end_date: row.get(7)?,
                            location: row.get(8)?,
                            supplier: row.get(9)?,
                            notes: row.get(10)?,
                            created_at: row.get(11)?,
                            updated_at: row.get(12)?,
                        })
                    },
                )
            }
            (None, None) => {
                self.conn.query_row(
                    &query,
                    params![user_id, date],
                    |row| {
                        Ok(FuelPrice {
                            id: row.get(0)?,
                            user_id: row.get(1)?,
                            fuel_type: row.get(2)?,
                            price_per_unit: row.get(3)?,
                            unit: row.get(4)?,
                            currency: row.get(5)?,
                            start_date: row.get(6)?,
                            end_date: row.get(7)?,
                            location: row.get(8)?,
                            supplier: row.get(9)?,
                            notes: row.get(10)?,
                            created_at: row.get(11)?,
                            updated_at: row.get(12)?,
                        })
                    },
                )
            }
        }
        .optional()
        .context("Failed to get fuel price for date")?;

        Ok(fuel_price)
    }

    pub fn update_fuel_price(
        &self,
        fuel_price_id: &str,
        fuel_type: &str,
        price_per_unit: f64,
        unit: &str,
        currency: &str,
        start_date: &str,
        end_date: Option<&str>,
        location: Option<&str>,
        supplier: Option<&str>,
        notes: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE fuel_prices SET
                fuel_type = ?2, price_per_unit = ?3, unit = ?4, currency = ?5,
                start_date = ?6, end_date = ?7, location = ?8, supplier = ?9, notes = ?10,
                updated_at = datetime('now')
             WHERE id = ?1",
            params![fuel_price_id, fuel_type, price_per_unit, unit, currency, start_date, end_date, location, supplier, notes],
        )
        .context("Failed to update fuel price")?;
        Ok(())
    }

    pub fn delete_fuel_price(&self, fuel_price_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM fuel_prices WHERE id = ?1", params![fuel_price_id])
            .context("Failed to delete fuel price")?;
        Ok(())
    }

    // ===== ANALYTICS OPERATIONS =====

    /// Get temporal flight data grouped by time period
    pub fn get_temporal_flight_data(
        &self,
        user_id: &str,
        granularity: &str,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<TemporalFlightData>> {
        let mut query = match granularity {
            // Note: We filter out dates before 1903 (first powered flight) to exclude placeholder dates like 1900-01-01
            "quarter" => String::from(
                "SELECT
                    strftime('%Y', departure_datetime) || '-Q' ||
                    CAST((CAST(strftime('%m', departure_datetime) AS INTEGER) + 2) / 3 AS TEXT) as period,
                    COUNT(*) as flight_count,
                    COALESCE(SUM(distance_km), 0.0) as total_distance_km,
                    COALESCE(SUM(carbon_emissions_kg), 0.0) as total_co2_kg,
                    strftime('%Y', departure_datetime) || '-' ||
                    printf('%02d', ((CAST(strftime('%m', departure_datetime) AS INTEGER) - 1) / 3) * 3 + 1) || '-01' as period_start
                FROM flights
                WHERE user_id = ?1 AND departure_datetime >= '1903-01-01'"
            ),
            "year" => String::from(
                "SELECT
                    strftime('%Y', departure_datetime) as period,
                    COUNT(*) as flight_count,
                    COALESCE(SUM(distance_km), 0.0) as total_distance_km,
                    COALESCE(SUM(carbon_emissions_kg), 0.0) as total_co2_kg,
                    strftime('%Y', departure_datetime) || '-01-01' as period_start
                FROM flights
                WHERE user_id = ?1 AND departure_datetime >= '1903-01-01'"
            ),
            _ => String::from(
                "SELECT
                    strftime('%Y-%m', departure_datetime) as period,
                    COUNT(*) as flight_count,
                    COALESCE(SUM(distance_km), 0.0) as total_distance_km,
                    COALESCE(SUM(carbon_emissions_kg), 0.0) as total_co2_kg,
                    strftime('%Y-%m-01', departure_datetime) as period_start
                FROM flights
                WHERE user_id = ?1 AND departure_datetime >= '1903-01-01'"
            ),
        };

        if start_date.is_some() {
            query.push_str(" AND departure_datetime >= ?2");
        }
        if end_date.is_some() {
            let param_num = if start_date.is_some() { 3 } else { 2 };
            query.push_str(&format!(" AND departure_datetime <= ?{}", param_num));
        }

        query.push_str(" GROUP BY period ORDER BY period ASC");

        let mut stmt = self.conn.prepare(&query)
            .context("Failed to prepare temporal analysis query")?;

        let data = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                stmt.query_map(params![user_id, start, end], |row| {
                    Ok(TemporalFlightData {
                        period: row.get(0)?,
                        flight_count: row.get(1)?,
                        total_distance_km: row.get(2)?,
                        total_co2_kg: row.get(3)?,
                        period_start: row.get(4)?,
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (Some(start), None) => {
                stmt.query_map(params![user_id, start], |row| {
                    Ok(TemporalFlightData {
                        period: row.get(0)?,
                        flight_count: row.get(1)?,
                        total_distance_km: row.get(2)?,
                        total_co2_kg: row.get(3)?,
                        period_start: row.get(4)?,
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (None, Some(end)) => {
                stmt.query_map(params![user_id, end], |row| {
                    Ok(TemporalFlightData {
                        period: row.get(0)?,
                        flight_count: row.get(1)?,
                        total_distance_km: row.get(2)?,
                        total_co2_kg: row.get(3)?,
                        period_start: row.get(4)?,
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (None, None) => {
                stmt.query_map(params![user_id], |row| {
                    Ok(TemporalFlightData {
                        period: row.get(0)?,
                        flight_count: row.get(1)?,
                        total_distance_km: row.get(2)?,
                        total_co2_kg: row.get(3)?,
                        period_start: row.get(4)?,
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
        };

        Ok(data)
    }

    /// Get top visited airports with counts
    pub fn get_airport_visit_data(
        &self,
        user_id: &str,
        limit: i64,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<AirportVisitData>> {
        let mut query = String::from(
            "WITH departures AS (
                SELECT departure_airport as airport_code, 1 as is_departure, 0 as is_arrival
                FROM flights WHERE user_id = ? AND departure_airport IS NOT NULL"
        );

        if start_date.is_some() {
            query.push_str(" AND departure_datetime >= ?");
        }
        if end_date.is_some() {
            query.push_str(" AND departure_datetime <= ?");
        }

        query.push_str(
            "), arrivals AS (
                SELECT arrival_airport as airport_code, 0 as is_departure, 1 as is_arrival
                FROM flights WHERE user_id = ? AND arrival_airport IS NOT NULL"
        );

        if start_date.is_some() {
            query.push_str(" AND departure_datetime >= ?");
        }
        if end_date.is_some() {
            query.push_str(" AND departure_datetime <= ?");
        }

        query.push_str(
            "), combined AS (
                SELECT * FROM departures
                UNION ALL
                SELECT * FROM arrivals
            )
            SELECT
                c.airport_code,
                COALESCE(a.name, c.airport_code) as airport_name,
                COUNT(*) as total_visits,
                SUM(c.is_departure) as departure_count,
                SUM(c.is_arrival) as arrival_count,
                COALESCE(a.city || ', ' || a.country, 'Unknown') as location
            FROM combined c
            LEFT JOIN airports a ON c.airport_code = a.id OR c.airport_code = a.icao_code OR c.airport_code = a.iata_code
            GROUP BY c.airport_code
            ORDER BY total_visits DESC
            LIMIT ?"
        );

        let mut stmt = self.conn.prepare(&query)
            .context("Failed to prepare geospatial analysis query")?;

        let data = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                stmt.query_map(params![user_id, start, end, user_id, start, end, limit], |row| {
                    Ok(AirportVisitData {
                        airport_code: row.get(0)?,
                        airport_name: row.get(1)?,
                        total_visits: row.get(2)?,
                        departure_count: row.get(3)?,
                        arrival_count: row.get(4)?,
                        location: row.get(5)?,
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (Some(start), None) => {
                stmt.query_map(params![user_id, start, user_id, start, limit], |row| {
                    Ok(AirportVisitData {
                        airport_code: row.get(0)?,
                        airport_name: row.get(1)?,
                        total_visits: row.get(2)?,
                        departure_count: row.get(3)?,
                        arrival_count: row.get(4)?,
                        location: row.get(5)?,
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (None, Some(end)) => {
                stmt.query_map(params![user_id, end, user_id, end, limit], |row| {
                    Ok(AirportVisitData {
                        airport_code: row.get(0)?,
                        airport_name: row.get(1)?,
                        total_visits: row.get(2)?,
                        departure_count: row.get(3)?,
                        arrival_count: row.get(4)?,
                        location: row.get(5)?,
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (None, None) => {
                stmt.query_map(params![user_id, user_id, limit], |row| {
                    Ok(AirportVisitData {
                        airport_code: row.get(0)?,
                        airport_name: row.get(1)?,
                        total_visits: row.get(2)?,
                        departure_count: row.get(3)?,
                        arrival_count: row.get(4)?,
                        location: row.get(5)?,
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
        };

        Ok(data)
    }

    /// Get passenger network graph data
    /// Extracts passengers directly from notes field (not passenger_mappings table)
    pub fn get_passenger_network_data(
        &self,
        user_id: &str,
        min_shared_flights: i64,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<PassengerNetworkData> {
        use std::collections::{HashMap, HashSet};

        // Build date filter clause
        let date_filter = match (start_date, end_date) {
            (Some(_), Some(_)) => " AND departure_datetime >= ?2 AND departure_datetime <= ?3",
            (Some(_), None) => " AND departure_datetime >= ?2",
            (None, Some(_)) => " AND departure_datetime <= ?2",
            (None, None) => "",
        };

        // Get all flights with notes for this user
        let query = format!(
            "SELECT id, notes, distance_km, departure_airport, arrival_airport
             FROM flights
             WHERE user_id = ?1 AND notes IS NOT NULL AND notes != ''{}",
            date_filter
        );

        let mut stmt = self.conn.prepare(&query)
            .context("Failed to prepare flights query for passenger network")?;

        // Struct to hold flight data
        struct FlightData {
            id: String,
            notes: String,
            distance_km: f64,
            route: String,
        }

        let flights: Vec<FlightData> = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                stmt.query_map(params![user_id, start, end], |row| {
                    let dep: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
                    let arr: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
                    Ok(FlightData {
                        id: row.get(0)?,
                        notes: row.get(1)?,
                        distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        route: format!("{}-{}", dep, arr),
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (Some(start), None) => {
                stmt.query_map(params![user_id, start], |row| {
                    let dep: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
                    let arr: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
                    Ok(FlightData {
                        id: row.get(0)?,
                        notes: row.get(1)?,
                        distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        route: format!("{}-{}", dep, arr),
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (None, Some(end)) => {
                stmt.query_map(params![user_id, end], |row| {
                    let dep: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
                    let arr: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
                    Ok(FlightData {
                        id: row.get(0)?,
                        notes: row.get(1)?,
                        distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        route: format!("{}-{}", dep, arr),
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (None, None) => {
                stmt.query_map(params![user_id], |row| {
                    let dep: String = row.get::<_, Option<String>>(3)?.unwrap_or_default();
                    let arr: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
                    Ok(FlightData {
                        id: row.get(0)?,
                        notes: row.get(1)?,
                        distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        route: format!("{}-{}", dep, arr),
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
        };

        // Parse passengers from notes and build aggregations
        // passenger -> (flight_count, total_distance)
        let mut passenger_stats: HashMap<String, (i64, f64)> = HashMap::new();
        // (passenger1, passenger2) -> (flight_count, routes)
        let mut co_travel: HashMap<(String, String), (i64, HashSet<String>)> = HashMap::new();

        for flight in &flights {
            let passengers = Self::parse_passengers_from_notes(&flight.notes);

            // Update passenger stats
            for p in &passengers {
                let entry = passenger_stats.entry(p.clone()).or_insert((0, 0.0));
                entry.0 += 1;
                entry.1 += flight.distance_km;
            }

            // Update co-travel relationships (passengers on same flight)
            let passengers_vec: Vec<_> = passengers.into_iter().collect();
            for i in 0..passengers_vec.len() {
                for j in (i + 1)..passengers_vec.len() {
                    let (p1, p2) = if passengers_vec[i] < passengers_vec[j] {
                        (passengers_vec[i].clone(), passengers_vec[j].clone())
                    } else {
                        (passengers_vec[j].clone(), passengers_vec[i].clone())
                    };
                    let entry = co_travel.entry((p1, p2)).or_insert((0, HashSet::new()));
                    entry.0 += 1;
                    entry.1.insert(flight.route.clone());
                }
            }
        }

        // Build nodes
        let nodes: Vec<PassengerNode> = passenger_stats
            .into_iter()
            .map(|(name, (flights, distance))| PassengerNode {
                id: name.clone(),
                label: name,
                total_flights: flights,
                total_distance_km: distance,
            })
            .collect();

        // Build edges (filter by min_shared_flights)
        let edges: Vec<PassengerEdge> = co_travel
            .into_iter()
            .filter(|(_, (count, _))| *count >= min_shared_flights)
            .map(|((p1, p2), (count, routes))| PassengerEdge {
                source: p1,
                target: p2,
                flight_count: count,
                routes: routes.into_iter().collect(),
            })
            .collect();

        Ok(PassengerNetworkData { nodes, edges })
    }

    /// Helper function to parse passenger names from notes field
    fn parse_passengers_from_notes(notes: &str) -> std::collections::HashSet<String> {
        use std::collections::HashSet;
        let mut passengers = HashSet::new();

        // Try to parse as JSON array first
        if let Ok(parsed) = serde_json::from_str::<Vec<String>>(notes) {
            for name in parsed {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    passengers.insert(trimmed.to_uppercase());
                }
            }
            return passengers;
        }

        // Fall back to parsing as comma/newline separated text
        // Remove common prefixes like "Passengers:", "PAX:", etc.
        let cleaned = notes
            .replace("Passengers:", "")
            .replace("passengers:", "")
            .replace("PAX:", "")
            .replace("pax:", "");

        // Split by common delimiters
        for part in cleaned.split(|c| c == ',' || c == '\n' || c == ';' || c == '|') {
            let trimmed = part.trim();
            // Skip empty strings and common non-passenger entries
            if !trimmed.is_empty()
                && trimmed.len() > 1
                && !trimmed.to_lowercase().starts_with("note")
                && !trimmed.to_lowercase().starts_with("comment")
            {
                passengers.insert(trimmed.to_uppercase());
            }
        }

        passengers
    }

    /// Get comparative passenger metrics
    /// Extracts passengers directly from notes field (not passenger_mappings table)
    pub fn get_passenger_metrics(
        &self,
        user_id: &str,
        metric: &str,
        limit: i64,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<PassengerMetrics>> {
        use std::collections::{HashMap, HashSet};

        // Build date filter clause
        let date_filter = match (start_date, end_date) {
            (Some(_), Some(_)) => " AND departure_datetime >= ?2 AND departure_datetime <= ?3",
            (Some(_), None) => " AND departure_datetime >= ?2",
            (None, Some(_)) => " AND departure_datetime <= ?2",
            (None, None) => "",
        };

        // Get all flights with notes for this user
        let query = format!(
            "SELECT id, notes, distance_km, carbon_emissions_kg, total_duration, departure_airport, arrival_airport
             FROM flights
             WHERE user_id = ?1 AND notes IS NOT NULL AND notes != ''{}",
            date_filter
        );

        let mut stmt = self.conn.prepare(&query)
            .context("Failed to prepare flights query for passenger metrics")?;

        // Struct to hold flight data
        struct FlightData {
            notes: String,
            distance_km: f64,
            co2_kg: f64,
            duration_minutes: f64,
            airports: String,
        }

        let flights: Vec<FlightData> = match (start_date, end_date) {
            (Some(start), Some(end)) => {
                stmt.query_map(params![user_id, start, end], |row| {
                    let dep: String = row.get::<_, Option<String>>(5)?.unwrap_or_default();
                    let arr: String = row.get::<_, Option<String>>(6)?.unwrap_or_default();
                    Ok(FlightData {
                        notes: row.get(1)?,
                        distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        co2_kg: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                        duration_minutes: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                        airports: format!("{}{}", dep, arr),
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (Some(start), None) => {
                stmt.query_map(params![user_id, start], |row| {
                    let dep: String = row.get::<_, Option<String>>(5)?.unwrap_or_default();
                    let arr: String = row.get::<_, Option<String>>(6)?.unwrap_or_default();
                    Ok(FlightData {
                        notes: row.get(1)?,
                        distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        co2_kg: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                        duration_minutes: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                        airports: format!("{}{}", dep, arr),
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (None, Some(end)) => {
                stmt.query_map(params![user_id, end], |row| {
                    let dep: String = row.get::<_, Option<String>>(5)?.unwrap_or_default();
                    let arr: String = row.get::<_, Option<String>>(6)?.unwrap_or_default();
                    Ok(FlightData {
                        notes: row.get(1)?,
                        distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        co2_kg: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                        duration_minutes: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                        airports: format!("{}{}", dep, arr),
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
            (None, None) => {
                stmt.query_map(params![user_id], |row| {
                    let dep: String = row.get::<_, Option<String>>(5)?.unwrap_or_default();
                    let arr: String = row.get::<_, Option<String>>(6)?.unwrap_or_default();
                    Ok(FlightData {
                        notes: row.get(1)?,
                        distance_km: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                        co2_kg: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                        duration_minutes: row.get::<_, Option<f64>>(4)?.unwrap_or(0.0),
                        airports: format!("{}{}", dep, arr),
                    })
                })?.collect::<std::result::Result<Vec<_>, _>>()?
            }
        };

        // Build passenger stats
        // passenger -> (flights, distance, co2, hours, unique_airports_set)
        let mut passenger_stats: HashMap<String, (i64, f64, f64, f64, HashSet<String>)> = HashMap::new();

        for flight in &flights {
            let passengers = Self::parse_passengers_from_notes(&flight.notes);

            for p in passengers {
                let entry = passenger_stats.entry(p).or_insert((0, 0.0, 0.0, 0.0, HashSet::new()));
                entry.0 += 1; // flights
                entry.1 += flight.distance_km;
                entry.2 += flight.co2_kg;
                entry.3 += flight.duration_minutes / 60.0; // hours
                if !flight.airports.is_empty() {
                    entry.4.insert(flight.airports.clone());
                }
            }
        }

        // Convert to PassengerMetrics and sort
        let mut metrics: Vec<PassengerMetrics> = passenger_stats
            .into_iter()
            .map(|(name, (flights, distance, co2, hours, airports))| {
                let avg_distance = if flights > 0 { distance / flights as f64 } else { 0.0 };
                PassengerMetrics {
                    abbreviation: name.clone(),
                    full_name: Some(name),
                    total_flights: flights,
                    total_distance_km: distance,
                    total_co2_kg: co2,
                    total_flight_hours: hours,
                    unique_airports: airports.len() as i64,
                    avg_flight_distance_km: avg_distance,
                }
            })
            .collect();

        // Sort based on metric
        match metric {
            "distance" => metrics.sort_by(|a, b| b.total_distance_km.partial_cmp(&a.total_distance_km).unwrap_or(std::cmp::Ordering::Equal)),
            "co2" => metrics.sort_by(|a, b| b.total_co2_kg.partial_cmp(&a.total_co2_kg).unwrap_or(std::cmp::Ordering::Equal)),
            "hours" => metrics.sort_by(|a, b| b.total_flight_hours.partial_cmp(&a.total_flight_hours).unwrap_or(std::cmp::Ordering::Equal)),
            _ => metrics.sort_by(|a, b| b.total_flights.cmp(&a.total_flights)), // Default: flights
        }

        // Apply limit
        metrics.truncate(limit as usize);

        Ok(metrics)
    }

    // ===== ADDITIONAL ANALYTICS OPERATIONS =====

    /// Get aircraft utilization statistics
    pub fn get_aircraft_utilization(&self, user_id: &str) -> Result<Vec<AircraftUtilization>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                COALESCE(f.aircraft_registration, 'Unknown') as tail_number,
                COALESCE(at.manufacturer || ' ' || at.model, 'Unknown Type') as aircraft_type,
                COUNT(*) as total_flights,
                COALESCE(SUM(f.total_duration), 0.0) / 60.0 as total_hours,
                COALESCE(SUM(f.distance_km), 0.0) as total_distance_km,
                CASE WHEN COUNT(*) > 0
                     THEN COALESCE(SUM(f.total_duration), 0.0) / 60.0 / COUNT(*)
                     ELSE 0.0
                END as avg_flight_hours,
                MAX(f.departure_datetime) as last_flown,
                CAST((julianday('now') - julianday(MAX(f.departure_datetime))) AS INTEGER) as days_since_last_flight
            FROM flights f
            LEFT JOIN aircraft_types at ON f.aircraft_type_id = at.id
            WHERE f.user_id = ?1
            GROUP BY tail_number, aircraft_type
            ORDER BY total_flights DESC"
        ).context("Failed to prepare aircraft utilization query")?;

        let data = stmt
            .query_map(params![user_id], |row| {
                Ok(AircraftUtilization {
                    tail_number: row.get(0)?,
                    aircraft_type: row.get(1)?,
                    total_flights: row.get(2)?,
                    total_hours: row.get(3)?,
                    total_distance_km: row.get(4)?,
                    avg_flight_hours: row.get(5)?,
                    last_flown: row.get(6)?,
                    days_since_last_flight: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(data)
    }

    /// Get cost breakdown by category
    pub fn get_cost_breakdown(&self, user_id: &str) -> Result<Vec<CostBreakdown>> {
        // Calculate total for percentage
        let total: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(total_cost), 0.0) FROM flights WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;

        if total == 0.0 {
            return Ok(Vec::new());
        }

        // For now, we'll use a basic breakdown from existing flight costs
        // In a full implementation, you'd have separate cost categories
        let mut stmt = self.conn.prepare(
            "SELECT
                'flight_costs' as category,
                COALESCE(SUM(total_cost), 0.0) as total_cost,
                currency,
                COUNT(*) as item_count
            FROM flights
            WHERE user_id = ?1 AND total_cost IS NOT NULL
            GROUP BY currency"
        ).context("Failed to prepare cost breakdown query")?;

        let data = stmt
            .query_map(params![user_id], |row| {
                Ok(CostBreakdown {
                    category: row.get(0)?,
                    total_cost: row.get(1)?,
                    currency: row.get(2)?,
                    item_count: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(data)
    }

    /// Get day/night flight statistics
    pub fn get_day_night_stats(&self, user_id: &str) -> Result<DayNightStats> {
        let result = self.conn.query_row(
            "SELECT
                COALESCE(SUM(pl.day_takeoffs + pl.day_landings), 0) as total_day_flights,
                COALESCE(SUM(pl.night_takeoffs + pl.night_landings), 0) as total_night_flights,
                COALESCE(SUM(pl.day_time), 0.0) as day_hours,
                COALESCE(SUM(pl.night_time), 0.0) as night_hours,
                COALESCE(SUM(pl.night_landings), 0) as night_landings,
                COALESCE(SUM(pl.day_landings), 0) as day_landings
            FROM flights f
            JOIN pilot_logbook pl ON f.id = pl.flight_id
            WHERE f.user_id = ?1",
            params![user_id],
            |row| {
                Ok(DayNightStats {
                    total_day_flights: row.get(0)?,
                    total_night_flights: row.get(1)?,
                    day_hours: row.get(2)?,
                    night_hours: row.get(3)?,
                    night_landings: row.get(4)?,
                    day_landings: row.get(5)?,
                })
            },
        ).optional()
        .context("Failed to get day/night stats")?;

        Ok(result.unwrap_or(DayNightStats {
            total_day_flights: 0,
            total_night_flights: 0,
            day_hours: 0.0,
            night_hours: 0.0,
            night_landings: 0,
            day_landings: 0,
        }))
    }

    /// Get longest flights
    pub fn get_long_haul_flights(&self, user_id: &str, limit: i64) -> Result<Vec<LongHaulFlight>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                COALESCE(f.id, '') as id,
                COALESCE(f.departure_airport, '') as departure_airport,
                COALESCE(f.arrival_airport, '') as arrival_airport,
                COALESCE(f.distance_km, 0.0) as distance_km,
                f.total_duration as flight_duration,
                COALESCE(f.departure_datetime, '') as departure_datetime,
                f.aircraft_type_id as aircraft_type
            FROM flights f
            WHERE f.user_id = ?1 AND f.id IS NOT NULL
            ORDER BY distance_km DESC
            LIMIT ?2"
        ).context("Failed to prepare long haul flights query")?;

        let data = stmt
            .query_map(params![user_id, limit], |row| {
                Ok(LongHaulFlight {
                    flight_id: row.get(0)?,
                    departure_airport: row.get(1)?,
                    arrival_airport: row.get(2)?,
                    distance_km: row.get(3)?,
                    flight_duration: row.get(4)?,
                    departure_datetime: row.get(5)?,
                    aircraft_type: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(data)
    }

    /// Get pilot currency requirements
    pub fn get_pilot_currency(&self, user_id: &str) -> Result<Vec<CurrencyItem>> {
        // This is a simplified example - real implementation would track specific requirements
        let mut items = Vec::new();

        // Get recent takeoffs/landings for currency
        let (day_landings, night_landings, last_flight): (i64, i64, Option<String>) = self.conn.query_row(
            "SELECT
                COALESCE(SUM(pl.day_landings), 0),
                COALESCE(SUM(pl.night_landings), 0),
                MAX(f.departure_datetime)
            FROM flights f
            JOIN pilot_logbook pl ON f.id = pl.flight_id
            WHERE f.user_id = ?1
            AND f.departure_datetime >= date('now', '-90 days')",
            params![user_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap_or((0, 0, None));

        let days_since = last_flight.as_ref().map(|date| {
            self.conn.query_row(
                "SELECT CAST((julianday('now') - julianday(?1)) AS INTEGER)",
                params![date],
                |row| row.get(0),
            ).unwrap_or(0)
        });

        // Day currency (3 takeoffs/landings in 90 days)
        items.push(CurrencyItem {
            requirement: "Day Currency (3 landings/90 days)".to_string(),
            current_count: day_landings,
            required: 3,
            expires_in_days: days_since,
            status: if day_landings >= 3 { "good" } else { "warning" }.to_string(),
            last_completed: last_flight.clone(),
        });

        // Night currency (3 night landings in 90 days)
        items.push(CurrencyItem {
            requirement: "Night Currency (3 landings/90 days)".to_string(),
            current_count: night_landings,
            required: 3,
            expires_in_days: days_since,
            status: if night_landings >= 3 { "good" } else { "warning" }.to_string(),
            last_completed: last_flight,
        });

        Ok(items)
    }

    /// Get monthly cost trend
    pub fn get_monthly_cost_trend(&self, user_id: &str) -> Result<Vec<MonthlyCostData>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                strftime('%Y-%m', departure_datetime) as period,
                COALESCE(SUM(total_cost), 0.0) as total_cost,
                COALESCE(SUM(total_duration), 0.0) / 60.0 as total_hours,
                CASE WHEN SUM(total_duration) > 0
                     THEN (SUM(total_cost) / (SUM(total_duration) / 60.0))
                     ELSE 0.0
                END as cost_per_hour,
                0.0 as fuel_cost,
                COALESCE(SUM(total_cost), 0.0) as other_costs,
                COALESCE(currency, 'USD') as currency
            FROM flights
            WHERE user_id = ?1 AND total_cost IS NOT NULL
            GROUP BY period, currency
            ORDER BY period ASC"
        ).context("Failed to prepare monthly cost trend query")?;

        let data = stmt
            .query_map(params![user_id], |row| {
                Ok(MonthlyCostData {
                    period: row.get(0)?,
                    total_cost: row.get(1)?,
                    total_hours: row.get(2)?,
                    cost_per_hour: row.get(3)?,
                    fuel_cost: row.get(4)?,
                    other_costs: row.get(5)?,
                    currency: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(data)
    }

    /// Get runway risk assessment
    pub fn get_runway_risk_data(&self, user_id: &str) -> Result<Vec<RunwayRiskData>> {
        let mut stmt = self.conn.prepare(
            "WITH airport_visits AS (
                SELECT departure_airport as airport_code
                FROM flights WHERE user_id = ?1 AND departure_airport IS NOT NULL
                UNION ALL
                SELECT arrival_airport as airport_code
                FROM flights WHERE user_id = ?1 AND arrival_airport IS NOT NULL
            )
            SELECT
                av.airport_code,
                COALESCE(a.name, av.airport_code) as airport_name,
                5000 as runway_length_ft,
                COUNT(*) as visits,
                CASE
                    WHEN 5000 >= 5000 THEN 'safe'
                    WHEN 5000 >= 3000 THEN 'marginal'
                    WHEN 5000 >= 2000 THEN 'watch_out'
                    ELSE 'nope'
                END as risk_level
            FROM airport_visits av
            LEFT JOIN airports a ON av.airport_code = a.id OR av.airport_code = a.icao_code OR av.airport_code = a.iata_code
            GROUP BY av.airport_code
            ORDER BY visits DESC
            LIMIT 20"
        ).context("Failed to prepare runway risk query")?;

        let data = stmt
            .query_map(params![user_id], |row| {
                Ok(RunwayRiskData {
                    airport_code: row.get(0)?,
                    airport_name: row.get(1)?,
                    runway_length_ft: row.get(2)?,
                    visits: row.get(3)?,
                    risk_level: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(data)
    }
}
