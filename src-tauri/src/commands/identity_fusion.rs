// Identity Fusion Commands
// Manages canonical passenger identities and their aliases

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

use super::AppState;

// ===== DATA TYPES =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalPassenger {
    pub id: String,
    pub canonical_name: String,
    pub notes: Option<String>,
    pub total_flights: i32,
    pub alias_count: i32,
    pub first_seen_date: Option<String>,
    pub last_seen_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerAlias {
    pub id: String,
    pub passenger_id: String,
    pub raw_name: String,
    pub usage_count: i32,
    pub source_document: Option<String>,
    pub match_type: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    pub passengers_created: i32,
    pub aliases_created: i32,
    pub flight_links_created: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapBatchResult {
    pub batch_number: usize,
    pub total_batches: usize,
    pub passengers_created_this_batch: i32,
    pub aliases_created_this_batch: i32,
    pub flight_links_created_this_batch: i32,
    pub total_passengers_created: i32,
    pub total_aliases_created: i32,
    pub total_flight_links_created: i32,
    pub is_complete: bool,
    pub phase: String, // "extracting", "creating_passengers", "linking_flights", "complete"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub source_alias: String,
    pub target_passenger_id: String,
    pub target_canonical_name: String,
    pub flights_updated: i32,
}

// ===== BOOTSTRAP COMMAND =====
// Scans flight notes for passenger names and creates initial identity records
// Optimized with batch processing and transactions for better performance

#[tauri::command]
pub fn bootstrap_identities(
    user_id: String,
    state: State<'_, AppState>,
) -> Result<BootstrapResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Step 1: Extract all unique passenger names from flights.notes
    let mut stmt = db.conn.prepare(
        "SELECT id, notes FROM flights WHERE user_id = ?1 AND notes IS NOT NULL"
    ).map_err(|e| e.to_string())?;

    let flights: Vec<(String, String)> = stmt
        .query_map(params![user_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Count occurrences of each name and collect flight-passenger pairs
    let mut name_counts: HashMap<String, i32> = HashMap::new();
    let mut flight_passengers: Vec<(String, String)> = Vec::new();

    for (flight_id, notes) in &flights {
        // Parse passengers from notes - handle multiple formats
        let passenger_text = if let Some(part) = notes.strip_prefix("Passengers: ") {
            part
        } else {
            notes.as_str()
        };

        // Split by common delimiters
        for name in passenger_text.split(|c| c == ',' || c == ';' || c == '|') {
            let trimmed = name.trim().to_uppercase();
            if !trimmed.is_empty() && trimmed.len() > 1 {
                *name_counts.entry(trimmed.clone()).or_insert(0) += 1;
                flight_passengers.push((flight_id.clone(), trimmed));
            }
        }
    }

    // Step 2: Get existing aliases in one batch query
    let existing_aliases: std::collections::HashSet<String> = {
        let mut stmt = db.conn.prepare(
            "SELECT raw_name FROM passenger_aliases"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Filter to only new names
    let new_names: Vec<(&String, &i32)> = name_counts
        .iter()
        .filter(|(name, _)| !existing_aliases.contains(*name))
        .collect();

    let mut passengers_created = 0;
    let mut aliases_created = 0;
    let mut flight_links_created = 0;

    // Step 3: Batch insert passengers and aliases using a transaction
    // Process in chunks of 500 for better memory management
    const BATCH_SIZE: usize = 500;

    // Create a mapping of name -> passenger_id for flight links
    let mut name_to_passenger_id: HashMap<String, String> = HashMap::new();

    // First, populate with existing aliases
    {
        let mut stmt = db.conn.prepare(
            "SELECT raw_name, passenger_id FROM passenger_aliases"
        ).map_err(|e| e.to_string())?;

        for row in stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| e.to_string())? {
            if let Ok((name, pid)) = row {
                name_to_passenger_id.insert(name, pid);
            }
        }
    }

    // Insert new passengers and aliases in batches
    for chunk in new_names.chunks(BATCH_SIZE) {
        db.conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

        for (name, count) in chunk {
            let passenger_id = Uuid::new_v4().to_string();
            let alias_id = Uuid::new_v4().to_string();

            db.conn.execute(
                "INSERT INTO passengers (id, canonical_name, total_flights, created_at, updated_at)
                 VALUES (?1, ?2, ?3, datetime('now'), datetime('now'))",
                params![passenger_id, name, count]
            ).map_err(|e| e.to_string())?;
            passengers_created += 1;

            db.conn.execute(
                "INSERT INTO passenger_aliases (id, passenger_id, raw_name, usage_count, match_type, confidence, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'exact', 1.0, datetime('now'))",
                params![alias_id, passenger_id, name, count]
            ).map_err(|e| e.to_string())?;
            aliases_created += 1;

            name_to_passenger_id.insert((*name).clone(), passenger_id);
        }

        db.conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    }

    // Step 4: Batch insert flight_passengers links
    for chunk in flight_passengers.chunks(BATCH_SIZE) {
        db.conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

        for (flight_id, passenger_name) in chunk {
            if let Some(pid) = name_to_passenger_id.get(passenger_name) {
                let result = db.conn.execute(
                    "INSERT OR IGNORE INTO flight_passengers (flight_id, passenger_id, created_at)
                     VALUES (?1, ?2, datetime('now'))",
                    params![flight_id, pid]
                ).map_err(|e| e.to_string())?;

                if result > 0 {
                    flight_links_created += 1;
                }
            }
        }

        db.conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    }

    Ok(BootstrapResult {
        passengers_created,
        aliases_created,
        flight_links_created,
    })
}

// ===== BATCHED BOOTSTRAP COMMAND =====
// Processes identity bootstrap in batches to avoid UI freeze
// Call with batch_number starting at 0, increment until is_complete is true

#[tauri::command]
pub fn bootstrap_identities_batch(
    user_id: String,
    batch_number: usize,
    batch_size: usize,
    state: State<'_, AppState>,
) -> Result<BootstrapBatchResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let batch_size = if batch_size == 0 { 200 } else { batch_size };

    // Phase 1: Extract all data on first batch (batch 0)
    if batch_number == 0 {
        // Clear any previous bootstrap state by starting fresh
        // This is idempotent - we'll skip existing aliases
    }

    // Get all flights with notes
    let mut stmt = db.conn.prepare(
        "SELECT id, notes FROM flights WHERE user_id = ?1 AND notes IS NOT NULL"
    ).map_err(|e| e.to_string())?;

    let flights: Vec<(String, String)> = stmt
        .query_map(params![user_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Parse all passenger names
    let mut name_counts: HashMap<String, i32> = HashMap::new();
    let mut flight_passengers: Vec<(String, String)> = Vec::new();

    for (flight_id, notes) in &flights {
        let passenger_text = if let Some(part) = notes.strip_prefix("Passengers: ") {
            part
        } else {
            notes.as_str()
        };

        for name in passenger_text.split(|c| c == ',' || c == ';' || c == '|') {
            let trimmed = name.trim().to_uppercase();
            if !trimmed.is_empty() && trimmed.len() > 1 {
                *name_counts.entry(trimmed.clone()).or_insert(0) += 1;
                flight_passengers.push((flight_id.clone(), trimmed));
            }
        }
    }

    // Get existing aliases
    let existing_aliases: std::collections::HashSet<String> = {
        let mut stmt = db.conn.prepare("SELECT raw_name FROM passenger_aliases")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    // Filter to new names only
    let new_names: Vec<(String, i32)> = name_counts
        .iter()
        .filter(|(name, _)| !existing_aliases.contains(*name))
        .map(|(n, c)| (n.clone(), *c))
        .collect();

    let total_new_passengers = new_names.len();
    let total_flight_links = flight_passengers.len();

    // Calculate which phase we're in based on batch number
    let passenger_batches = (total_new_passengers + batch_size - 1) / batch_size.max(1);
    let link_batches = (total_flight_links + batch_size - 1) / batch_size.max(1);
    let total_batches = passenger_batches + link_batches;

    let mut passengers_created_this_batch = 0;
    let mut aliases_created_this_batch = 0;
    let mut flight_links_created_this_batch = 0;

    // Get name -> passenger_id mapping
    let mut name_to_passenger_id: HashMap<String, String> = {
        let mut stmt = db.conn.prepare("SELECT raw_name, passenger_id FROM passenger_aliases")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let phase: String;
    let is_complete: bool;

    if batch_number < passenger_batches {
        // Phase: Creating passengers
        phase = "creating_passengers".to_string();
        is_complete = false;

        let start = batch_number * batch_size;
        let end = (start + batch_size).min(total_new_passengers);

        if start < total_new_passengers {
            db.conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

            for (name, count) in new_names.iter().skip(start).take(end - start) {
                let passenger_id = Uuid::new_v4().to_string();
                let alias_id = Uuid::new_v4().to_string();

                db.conn.execute(
                    "INSERT INTO passengers (id, canonical_name, total_flights, created_at, updated_at)
                     VALUES (?1, ?2, ?3, datetime('now'), datetime('now'))",
                    params![passenger_id, name, count]
                ).map_err(|e| e.to_string())?;
                passengers_created_this_batch += 1;

                db.conn.execute(
                    "INSERT INTO passenger_aliases (id, passenger_id, raw_name, usage_count, match_type, confidence, created_at)
                     VALUES (?1, ?2, ?3, ?4, 'exact', 1.0, datetime('now'))",
                    params![alias_id, passenger_id, name, count]
                ).map_err(|e| e.to_string())?;
                aliases_created_this_batch += 1;

                name_to_passenger_id.insert(name.clone(), passenger_id);
            }

            db.conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
        }
    } else {
        // Phase: Linking flights
        let link_batch = batch_number - passenger_batches;
        let start = link_batch * batch_size;
        let end = (start + batch_size).min(total_flight_links);

        if start < total_flight_links {
            phase = "linking_flights".to_string();
            is_complete = link_batch + 1 >= link_batches;

            db.conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

            for (flight_id, passenger_name) in flight_passengers.iter().skip(start).take(end - start) {
                if let Some(pid) = name_to_passenger_id.get(passenger_name) {
                    let result = db.conn.execute(
                        "INSERT OR IGNORE INTO flight_passengers (flight_id, passenger_id, created_at)
                         VALUES (?1, ?2, datetime('now'))",
                        params![flight_id, pid]
                    ).map_err(|e| e.to_string())?;

                    if result > 0 {
                        flight_links_created_this_batch += 1;
                    }
                }
            }

            db.conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
        } else {
            phase = "complete".to_string();
            is_complete = true;
        }
    }

    // Get totals from database
    let total_passengers_created: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM passengers", [], |row| row.get(0)
    ).unwrap_or(0);

    let total_aliases_created: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM passenger_aliases", [], |row| row.get(0)
    ).unwrap_or(0);

    let total_flight_links_created: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM flight_passengers", [], |row| row.get(0)
    ).unwrap_or(0);

    Ok(BootstrapBatchResult {
        batch_number,
        total_batches,
        passengers_created_this_batch,
        aliases_created_this_batch,
        flight_links_created_this_batch,
        total_passengers_created,
        total_aliases_created,
        total_flight_links_created,
        is_complete,
        phase,
    })
}

// ===== MERGE ALIAS COMMAND =====
// Merges an alias (or standalone passenger) into a target canonical identity

#[tauri::command]
pub fn merge_alias(
    source_raw_name: String,
    target_passenger_id: String,
    state: State<'_, AppState>,
) -> Result<MergeResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get the source alias info
    let (source_alias_id, source_passenger_id): (String, String) = db.conn.query_row(
        "SELECT id, passenger_id FROM passenger_aliases WHERE raw_name = ?1",
        params![source_raw_name],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| format!("Source alias '{}' not found: {}", source_raw_name, e))?;

    // Get target canonical name
    let target_canonical_name: String = db.conn.query_row(
        "SELECT canonical_name FROM passengers WHERE id = ?1",
        params![target_passenger_id],
        |row| row.get(0)
    ).map_err(|e| format!("Target passenger not found: {}", e))?;

    // If source and target are the same, nothing to do
    if source_passenger_id == target_passenger_id {
        return Err("Source and target are the same passenger".to_string());
    }

    // Step 1: Update the alias to point to the new passenger
    db.conn.execute(
        "UPDATE passenger_aliases SET passenger_id = ?1, match_type = 'manual' WHERE id = ?2",
        params![target_passenger_id, source_alias_id]
    ).map_err(|e| e.to_string())?;

    // Step 2: Update flight_passengers to point to the new passenger
    let flights_updated = db.conn.execute(
        "UPDATE flight_passengers SET passenger_id = ?1 WHERE passenger_id = ?2",
        params![target_passenger_id, source_passenger_id]
    ).map_err(|e| e.to_string())? as i32;

    // Step 3: Check if the source passenger has any remaining aliases
    let remaining_aliases: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM passenger_aliases WHERE passenger_id = ?1",
        params![source_passenger_id],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    // Step 4: If no remaining aliases, delete the orphaned passenger record
    if remaining_aliases == 0 {
        db.conn.execute(
            "DELETE FROM passengers WHERE id = ?1",
            params![source_passenger_id]
        ).map_err(|e| e.to_string())?;
    }

    // Step 5: Update flight counts on target passenger
    db.conn.execute(
        "UPDATE passengers SET
            total_flights = (SELECT COUNT(DISTINCT flight_id) FROM flight_passengers WHERE passenger_id = ?1),
            updated_at = datetime('now')
         WHERE id = ?1",
        params![target_passenger_id]
    ).map_err(|e| e.to_string())?;

    Ok(MergeResult {
        source_alias: source_raw_name,
        target_passenger_id,
        target_canonical_name,
        flights_updated,
    })
}

// ===== LIST COMMANDS =====

#[tauri::command]
pub fn list_canonical_passengers(
    state: State<'_, AppState>,
) -> Result<Vec<CanonicalPassenger>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT p.id, p.canonical_name, p.notes, p.total_flights, p.first_seen_date, p.last_seen_date,
                (SELECT COUNT(*) FROM passenger_aliases WHERE passenger_id = p.id) as alias_count
         FROM passengers p
         ORDER BY p.total_flights DESC"
    ).map_err(|e| e.to_string())?;

    let passengers = stmt
        .query_map([], |row| {
            Ok(CanonicalPassenger {
                id: row.get(0)?,
                canonical_name: row.get(1)?,
                notes: row.get(2)?,
                total_flights: row.get(3)?,
                first_seen_date: row.get(4)?,
                last_seen_date: row.get(5)?,
                alias_count: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(passengers)
}

#[tauri::command]
pub fn get_passenger_aliases(
    passenger_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<PassengerAlias>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT id, passenger_id, raw_name, usage_count, source_document, match_type, confidence
         FROM passenger_aliases
         WHERE passenger_id = ?1
         ORDER BY usage_count DESC"
    ).map_err(|e| e.to_string())?;

    let aliases = stmt
        .query_map(params![passenger_id], |row| {
            Ok(PassengerAlias {
                id: row.get(0)?,
                passenger_id: row.get(1)?,
                raw_name: row.get(2)?,
                usage_count: row.get(3)?,
                source_document: row.get(4)?,
                match_type: row.get(5)?,
                confidence: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(aliases)
}

#[tauri::command]
pub fn list_unmerged_passengers(
    state: State<'_, AppState>,
) -> Result<Vec<CanonicalPassenger>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Unmerged = passengers where their only alias is a self-alias (canonical_name == raw_name)
    let mut stmt = db.conn.prepare(
        "SELECT p.id, p.canonical_name, p.notes, p.total_flights, p.first_seen_date, p.last_seen_date, 1 as alias_count
         FROM passengers p
         WHERE EXISTS (
             SELECT 1 FROM passenger_aliases pa
             WHERE pa.passenger_id = p.id AND pa.raw_name = p.canonical_name
         )
         AND (SELECT COUNT(*) FROM passenger_aliases WHERE passenger_id = p.id) = 1
         ORDER BY p.total_flights DESC"
    ).map_err(|e| e.to_string())?;

    let passengers = stmt
        .query_map([], |row| {
            Ok(CanonicalPassenger {
                id: row.get(0)?,
                canonical_name: row.get(1)?,
                notes: row.get(2)?,
                total_flights: row.get(3)?,
                first_seen_date: row.get(4)?,
                last_seen_date: row.get(5)?,
                alias_count: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(passengers)
}

// ===== RENAME CANONICAL =====

#[tauri::command]
pub fn rename_canonical_passenger(
    passenger_id: String,
    new_canonical_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn.execute(
        "UPDATE passengers SET canonical_name = ?1, updated_at = datetime('now') WHERE id = ?2",
        params![new_canonical_name.to_uppercase(), passenger_id]
    ).map_err(|e| e.to_string())?;

    Ok(())
}

// ===== STATS =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityStats {
    pub total_passengers: i32,
    pub total_aliases: i32,
    pub unmerged_count: i32,
    pub merged_count: i32,
}

#[tauri::command]
pub fn get_identity_stats(
    state: State<'_, AppState>,
) -> Result<IdentityStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let total_passengers: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM passengers",
        [],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    let total_aliases: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM passenger_aliases",
        [],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    // Unmerged = has exactly 1 self-alias
    let unmerged_count: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM passengers p
         WHERE EXISTS (
             SELECT 1 FROM passenger_aliases pa
             WHERE pa.passenger_id = p.id AND pa.raw_name = p.canonical_name
         )
         AND (SELECT COUNT(*) FROM passenger_aliases WHERE passenger_id = p.id) = 1",
        [],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    let merged_count = total_passengers - unmerged_count;

    Ok(IdentityStats {
        total_passengers,
        total_aliases,
        unmerged_count,
        merged_count,
    })
}

// ===== DO NOT DEDUPLICATE MANAGEMENT =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoDedupPassenger {
    pub passenger_id: String,
    pub canonical_name: String,
    pub reason: Option<String>,
    pub total_flights: i32,
    pub created_at: String,
}

/// Mark a passenger as "do not deduplicate" - they will not appear in merge suggestions
#[tauri::command]
pub fn mark_no_dedup(
    passenger_id: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn.execute(
        "INSERT OR REPLACE INTO passenger_no_dedup (passenger_id, reason, created_at)
         VALUES (?1, ?2, datetime('now'))",
        params![passenger_id, reason]
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Remove a passenger from the "do not deduplicate" list
#[tauri::command]
pub fn unmark_no_dedup(
    passenger_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    db.conn.execute(
        "DELETE FROM passenger_no_dedup WHERE passenger_id = ?1",
        params![passenger_id]
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Check if a passenger is marked as "do not deduplicate"
#[tauri::command]
pub fn is_no_dedup(
    passenger_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let count: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM passenger_no_dedup WHERE passenger_id = ?1",
        params![passenger_id],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    Ok(count > 0)
}

/// List all passengers marked as "do not deduplicate"
#[tauri::command]
pub fn list_no_dedup_passengers(
    state: State<'_, AppState>,
) -> Result<Vec<NoDedupPassenger>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut stmt = db.conn.prepare(
        "SELECT nd.passenger_id, p.canonical_name, nd.reason, p.total_flights, nd.created_at
         FROM passenger_no_dedup nd
         JOIN passengers p ON nd.passenger_id = p.id
         ORDER BY p.canonical_name"
    ).map_err(|e| e.to_string())?;

    let passengers = stmt
        .query_map([], |row| {
            Ok(NoDedupPassenger {
                passenger_id: row.get(0)?,
                canonical_name: row.get(1)?,
                reason: row.get(2)?,
                total_flights: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(passengers)
}

// ===== DELETE PASSENGER FUNCTIONALITY =====
// Delete malformed or unwanted passenger entries entirely

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletePassengerResult {
    pub passenger_deleted: bool,
    pub aliases_deleted: i32,
    pub flight_links_removed: i32,
}

/// Delete a passenger entirely (for malformed/unwanted entries)
/// This removes the passenger, all aliases, and all flight links
#[tauri::command]
pub fn delete_passenger(
    passenger_id: String,
    state: State<'_, AppState>,
) -> Result<DeletePassengerResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Count aliases before deletion
    let aliases_count: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM passenger_aliases WHERE passenger_id = ?1",
        params![&passenger_id],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    // Count flight links before deletion
    let flight_links_count: i32 = db.conn.query_row(
        "SELECT COUNT(*) FROM flight_passengers WHERE passenger_id = ?1",
        params![&passenger_id],
        |row| row.get(0)
    ).map_err(|e| e.to_string())?;

    // Delete from no_dedup list if present
    db.conn.execute(
        "DELETE FROM passenger_no_dedup WHERE passenger_id = ?1",
        params![&passenger_id]
    ).map_err(|e| e.to_string())?;

    // Delete flight links
    db.conn.execute(
        "DELETE FROM flight_passengers WHERE passenger_id = ?1",
        params![&passenger_id]
    ).map_err(|e| e.to_string())?;

    // Delete aliases
    db.conn.execute(
        "DELETE FROM passenger_aliases WHERE passenger_id = ?1",
        params![&passenger_id]
    ).map_err(|e| e.to_string())?;

    // Delete the passenger record
    let deleted = db.conn.execute(
        "DELETE FROM passengers WHERE id = ?1",
        params![&passenger_id]
    ).map_err(|e| e.to_string())?;

    Ok(DeletePassengerResult {
        passenger_deleted: deleted > 0,
        aliases_deleted: aliases_count,
        flight_links_removed: flight_links_count,
    })
}

// ===== SPLIT PASSENGER FUNCTIONALITY =====
// Allows splitting malformed entries like "JE GM ET" into separate identities

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitPreview {
    pub original_passenger_id: String,
    pub original_name: String,
    pub proposed_names: Vec<String>,
    pub flight_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplitResult {
    pub original_deleted: bool,
    pub new_passengers_created: Vec<String>,
    pub flights_reassigned: i32,
}

/// Preview what a split would produce - doesn't make changes
#[tauri::command]
pub fn preview_split_passenger(
    passenger_id: String,
    delimiter: Option<String>,
    state: State<'_, AppState>,
) -> Result<SplitPreview, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get the passenger info
    let (canonical_name, total_flights): (String, i32) = db.conn.query_row(
        "SELECT canonical_name, total_flights FROM passengers WHERE id = ?1",
        params![passenger_id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| format!("Passenger not found: {}", e))?;

    // Split the name using delimiter or auto-detect
    let delimiter = delimiter.unwrap_or_else(|| " ".to_string());
    let proposed_names: Vec<String> = canonical_name
        .split(&delimiter)
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .collect();

    Ok(SplitPreview {
        original_passenger_id: passenger_id,
        original_name: canonical_name,
        proposed_names,
        flight_count: total_flights,
    })
}

/// Execute the split - creates new passengers and reassigns flights
#[tauri::command]
pub fn split_passenger(
    passenger_id: String,
    new_names: Vec<String>,
    state: State<'_, AppState>,
) -> Result<SplitResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Validate we have names to split into
    if new_names.is_empty() {
        return Err("No names provided for split".to_string());
    }

    // Get the original passenger info
    let (_original_name, _total_flights): (String, i32) = db.conn.query_row(
        "SELECT canonical_name, total_flights FROM passengers WHERE id = ?1",
        params![passenger_id],
        |row| Ok((row.get(0)?, row.get(1)?))
    ).map_err(|e| format!("Passenger not found: {}", e))?;

    // Get all flights associated with this passenger
    let flight_ids: Vec<String> = {
        let mut stmt = db.conn.prepare(
            "SELECT flight_id FROM flight_passengers WHERE passenger_id = ?1"
        ).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(params![passenger_id], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    db.conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

    let mut new_passenger_ids: Vec<String> = Vec::new();
    let mut flights_reassigned = 0;

    // For each new name, either find existing passenger or create new one
    for name in &new_names {
        let normalized_name = name.trim().to_uppercase();
        if normalized_name.is_empty() || normalized_name.len() < 2 {
            continue;
        }

        // Check if this name already exists as an alias
        let existing_passenger_id: Option<String> = db.conn.query_row(
            "SELECT passenger_id FROM passenger_aliases WHERE raw_name = ?1",
            params![normalized_name],
            |row| row.get(0)
        ).optional().map_err(|e| e.to_string())?;

        let target_passenger_id = if let Some(existing_id) = existing_passenger_id {
            // Name already exists, use that passenger
            existing_id
        } else {
            // Create new passenger and alias
            let new_id = Uuid::new_v4().to_string();
            let alias_id = Uuid::new_v4().to_string();

            db.conn.execute(
                "INSERT INTO passengers (id, canonical_name, total_flights, created_at, updated_at)
                 VALUES (?1, ?2, 0, datetime('now'), datetime('now'))",
                params![new_id, normalized_name]
            ).map_err(|e| e.to_string())?;

            db.conn.execute(
                "INSERT INTO passenger_aliases (id, passenger_id, raw_name, usage_count, match_type, confidence, created_at)
                 VALUES (?1, ?2, ?3, 0, 'split', 1.0, datetime('now'))",
                params![alias_id, new_id, normalized_name]
            ).map_err(|e| e.to_string())?;

            new_passenger_ids.push(normalized_name.clone());
            new_id
        };

        // Link all the original flights to this new passenger
        for flight_id in &flight_ids {
            let result = db.conn.execute(
                "INSERT OR IGNORE INTO flight_passengers (flight_id, passenger_id, created_at)
                 VALUES (?1, ?2, datetime('now'))",
                params![flight_id, target_passenger_id]
            ).map_err(|e| e.to_string())?;

            if result > 0 {
                flights_reassigned += 1;
            }
        }

        // Update flight count on target passenger
        db.conn.execute(
            "UPDATE passengers SET
                total_flights = (SELECT COUNT(DISTINCT flight_id) FROM flight_passengers WHERE passenger_id = ?1),
                updated_at = datetime('now')
             WHERE id = ?1",
            params![target_passenger_id]
        ).map_err(|e| e.to_string())?;
    }

    // Remove flight links from original passenger
    db.conn.execute(
        "DELETE FROM flight_passengers WHERE passenger_id = ?1",
        params![passenger_id]
    ).map_err(|e| e.to_string())?;

    // Delete the original passenger's alias
    db.conn.execute(
        "DELETE FROM passenger_aliases WHERE passenger_id = ?1",
        params![passenger_id]
    ).map_err(|e| e.to_string())?;

    // Delete the original passenger
    db.conn.execute(
        "DELETE FROM passengers WHERE id = ?1",
        params![passenger_id]
    ).map_err(|e| e.to_string())?;

    db.conn.execute("COMMIT", []).map_err(|e| e.to_string())?;

    Ok(SplitResult {
        original_deleted: true,
        new_passengers_created: new_passenger_ids,
        flights_reassigned,
    })
}

/// Detect if a passenger name looks like it needs splitting
/// Returns suggested delimiter if compound name detected
#[tauri::command]
pub fn detect_compound_name(
    passenger_id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let canonical_name: String = db.conn.query_row(
        "SELECT canonical_name FROM passengers WHERE id = ?1",
        params![passenger_id],
        |row| row.get(0)
    ).map_err(|e| format!("Passenger not found: {}", e))?;

    Ok(detect_compound_pattern(&canonical_name))
}

/// Helper function to detect compound patterns in a name
fn detect_compound_pattern(name: &str) -> Option<String> {
    // Pattern 1: Multiple 2-3 letter uppercase tokens (initials) like "JE GM ET"
    let tokens: Vec<&str> = name.split_whitespace().collect();
    if tokens.len() > 1 && tokens.iter().all(|t| t.len() <= 3 && t.chars().all(|c| c.is_uppercase())) {
        return Some(" ".to_string());
    }

    // Pattern 2: Slash separated "JE/GM/ET"
    if name.contains('/') {
        let slash_parts: Vec<&str> = name.split('/').collect();
        if slash_parts.len() > 1 {
            return Some("/".to_string());
        }
    }

    // Pattern 3: Comma separated "JE, GM, ET"
    if name.contains(',') {
        let comma_parts: Vec<&str> = name.split(',').collect();
        if comma_parts.len() > 1 {
            return Some(",".to_string());
        }
    }

    // Pattern 4: Ampersand "JE & GM"
    if name.contains('&') {
        return Some("&".to_string());
    }

    // Pattern 5: "AND" separator "JE AND GM"
    if name.contains(" AND ") {
        return Some(" AND ".to_string());
    }

    None
}

// ===== BATCH SPLIT FUNCTIONALITY =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSplitCandidate {
    pub passenger_id: String,
    pub canonical_name: String,
    pub total_flights: i32,
    pub detected_delimiter: Option<String>,
    pub proposed_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSplitResult {
    pub total_processed: i32,
    pub total_new_passengers: i32,
    pub total_flights_reassigned: i32,
    pub errors: Vec<String>,
}

/// Find all passengers that look like they need splitting (compound names)
#[tauri::command]
pub fn find_splittable_passengers(
    search_query: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<BatchSplitCandidate>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get all passengers, optionally filtered by search
    let query = if let Some(ref q) = search_query {
        format!(
            "SELECT id, canonical_name, total_flights FROM passengers WHERE canonical_name LIKE '%{}%' ORDER BY total_flights DESC",
            q.replace("'", "''")
        )
    } else {
        "SELECT id, canonical_name, total_flights FROM passengers ORDER BY total_flights DESC".to_string()
    };

    let mut stmt = db.conn.prepare(&query).map_err(|e| e.to_string())?;

    let passengers: Vec<(String, String, i32)> = {
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?))
        }).map_err(|e| e.to_string())?;
        rows.filter_map(|r| r.ok()).collect()
    };

    let mut candidates = Vec::new();

    for (id, name, flights) in passengers {
        if let Some(delimiter) = detect_compound_pattern(&name) {
            let proposed_names: Vec<String> = name
                .split(&delimiter)
                .map(|s| s.trim().to_uppercase())
                .filter(|s| !s.is_empty() && s.len() >= 2)
                .collect();

            // Only include if we'd actually split into multiple names
            if proposed_names.len() > 1 {
                candidates.push(BatchSplitCandidate {
                    passenger_id: id,
                    canonical_name: name,
                    total_flights: flights,
                    detected_delimiter: Some(delimiter),
                    proposed_names,
                });
            }
        }
    }

    Ok(candidates)
}

// ===== SEARCH WITH ALIASES =====
// Search passengers by canonical name OR any of their aliases

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPassengerResult {
    pub id: String,
    pub canonical_name: String,
    pub notes: Option<String>,
    pub total_flights: i32,
    pub alias_count: i32,
    pub first_seen_date: Option<String>,
    pub last_seen_date: Option<String>,
    pub matched_alias: Option<String>, // The alias that matched the search (if not canonical name)
    pub aliases: Vec<String>,          // All aliases for display
}

/// Search passengers by canonical name OR any alias
/// Returns passengers sorted by relevance (exact matches first, then partial)
#[tauri::command]
pub fn search_passengers(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<SearchPassengerResult>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let query_upper = query.trim().to_uppercase();

    // If query is empty, return all passengers (limited)
    if query_upper.is_empty() {
        let mut stmt = db.conn.prepare(
            "SELECT p.id, p.canonical_name, p.notes, p.total_flights, p.first_seen_date, p.last_seen_date,
                    (SELECT COUNT(*) FROM passenger_aliases WHERE passenger_id = p.id) as alias_count
             FROM passengers p
             ORDER BY p.total_flights DESC
             LIMIT 500"
        ).map_err(|e| e.to_string())?;

        let results: Vec<(String, String, Option<String>, i32, Option<String>, Option<String>, i32)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i32>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let mut search_results = Vec::new();
        for (id, canonical_name, notes, total_flights, first_seen, last_seen, alias_count) in results {
            let aliases = get_aliases_list(&db.conn, &id)?;

            search_results.push(SearchPassengerResult {
                id,
                canonical_name,
                notes,
                total_flights,
                alias_count,
                first_seen_date: first_seen,
                last_seen_date: last_seen,
                matched_alias: None,
                aliases,
            });
        }

        return Ok(search_results);
    }

    // Search for matches in both canonical_name and raw_name (aliases)
    // Use DISTINCT to avoid duplicates when both canonical and alias match
    let mut stmt = db.conn.prepare(
        "SELECT DISTINCT p.id, p.canonical_name, p.notes, p.total_flights,
                p.first_seen_date, p.last_seen_date,
                (SELECT COUNT(*) FROM passenger_aliases WHERE passenger_id = p.id) as alias_count,
                (SELECT raw_name FROM passenger_aliases
                 WHERE passenger_id = p.id AND raw_name LIKE ?1
                 AND raw_name != p.canonical_name
                 LIMIT 1) as matched_alias,
                CASE
                    WHEN p.canonical_name = ?2 THEN 1
                    WHEN p.canonical_name LIKE ?1 THEN 2
                    WHEN EXISTS (SELECT 1 FROM passenger_aliases WHERE passenger_id = p.id AND raw_name = ?2) THEN 3
                    WHEN EXISTS (SELECT 1 FROM passenger_aliases WHERE passenger_id = p.id AND raw_name LIKE ?1) THEN 4
                    ELSE 5
                END as match_rank
         FROM passengers p
         WHERE p.canonical_name LIKE ?1
            OR EXISTS (
                SELECT 1 FROM passenger_aliases pa
                WHERE pa.passenger_id = p.id AND pa.raw_name LIKE ?1
            )
         ORDER BY match_rank, p.total_flights DESC
         LIMIT 100"
    ).map_err(|e| e.to_string())?;

    let like_pattern = format!("%{}%", query_upper);

    let results: Vec<(String, String, Option<String>, i32, Option<String>, Option<String>, i32, Option<String>)> = stmt
        .query_map(params![like_pattern, query_upper], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, i32>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Fetch aliases for each passenger
    let mut search_results = Vec::new();
    for (id, canonical_name, notes, total_flights, first_seen, last_seen, alias_count, matched_alias) in results {
        let aliases = get_aliases_list(&db.conn, &id)?;

        search_results.push(SearchPassengerResult {
            id,
            canonical_name,
            notes,
            total_flights,
            alias_count,
            first_seen_date: first_seen,
            last_seen_date: last_seen,
            matched_alias,
            aliases,
        });
    }

    Ok(search_results)
}

/// Helper to get list of alias names for a passenger
fn get_aliases_list(conn: &rusqlite::Connection, passenger_id: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn.prepare(
        "SELECT raw_name FROM passenger_aliases WHERE passenger_id = ?1 ORDER BY usage_count DESC"
    ).map_err(|e| e.to_string())?;

    let aliases: Vec<String> = stmt
        .query_map(params![passenger_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    Ok(aliases)
}

// ===== PASSENGER DETAILS WITH ALIAS AGGREGATION =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerRoute {
    pub route: String,
    pub departure_airport: String,
    pub arrival_airport: String,
    pub flight_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerCompanion {
    pub name: String,
    pub flight_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassengerDetailsAggregated {
    pub passenger_id: String,
    pub canonical_name: String,
    pub total_flights: i32,
    pub total_distance_km: f64,
    pub top_routes: Vec<PassengerRoute>,
    pub travel_companions: Vec<PassengerCompanion>,
    pub first_flight_date: Option<String>,
    pub last_flight_date: Option<String>,
    pub total_co2_kg: f64,
    pub avg_co2_per_flight_kg: f64,
    pub carbon_offset_purchased: bool,
    pub aliases: Vec<String>,
}

/// Get passenger details aggregated across ALL aliases
/// This is the identity-fusion-aware version that properly counts merged passengers
#[tauri::command]
pub fn get_passenger_details_aggregated(
    user_id: String,
    passenger_id: String,
    state: State<'_, AppState>,
) -> Result<PassengerDetailsAggregated, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get canonical name
    let canonical_name: String = db.conn.query_row(
        "SELECT canonical_name FROM passengers WHERE id = ?1",
        params![passenger_id],
        |row| row.get(0)
    ).map_err(|e| format!("Passenger not found: {}", e))?;

    // Get ALL aliases for this passenger
    let aliases = get_aliases_list(&db.conn, &passenger_id)?;

    if aliases.is_empty() {
        return Ok(PassengerDetailsAggregated {
            passenger_id,
            canonical_name,
            total_flights: 0,
            total_distance_km: 0.0,
            top_routes: vec![],
            travel_companions: vec![],
            first_flight_date: None,
            last_flight_date: None,
            total_co2_kg: 0.0,
            avg_co2_per_flight_kg: 0.0,
            carbon_offset_purchased: false,
            aliases: vec![],
        });
    }

    // Build a query that searches for ANY of the aliases in the notes field
    // We need to get all flights where ANY alias appears in the notes
    let mut all_flight_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Also collect flight data
    type FlightData = (String, String, String, String, String, Option<f64>, Option<f64>, Option<i32>);
    let mut flight_data_map: std::collections::HashMap<String, FlightData> = std::collections::HashMap::new();

    // Query flights for each alias
    for alias in &aliases {
        let search_pattern = format!("%{}%", alias);
        let mut stmt = db.conn.prepare(
            "SELECT id, notes, departure_airport, arrival_airport, departure_datetime, distance_km, carbon_emissions_kg, carbon_offset_purchased
             FROM flights
             WHERE user_id = ?1 AND notes LIKE ?2"
        ).map_err(|e| e.to_string())?;

        let flights: Vec<FlightData> = stmt
            .query_map(params![user_id, search_pattern], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        // Filter to only flights where this alias is actually in the passenger list
        for flight in flights {
            let (id, notes, dep, arr, datetime, dist, co2, offset) = flight;

            // Check if this alias is actually in the passenger list
            if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
                if passenger_part.split(',').any(|name| name.trim() == alias) {
                    all_flight_ids.insert(id.clone());
                    flight_data_map.entry(id).or_insert((
                        String::new(), notes, dep, arr, datetime, dist, co2, offset
                    ));
                }
            }
        }
    }

    // Now compute aggregated stats from unique flights
    let mut total_distance_km = 0.0;
    let mut total_co2_kg = 0.0;
    let mut carbon_offset_purchased = false;
    let mut flight_dates: Vec<String> = Vec::new();
    let mut route_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    let mut companion_counts: std::collections::HashMap<String, i32> = std::collections::HashMap::new();

    // Create a set of all aliases for companion filtering
    let alias_set: std::collections::HashSet<&str> = aliases.iter().map(|s| s.as_str()).collect();

    for (_id, (_, notes, dep, arr, datetime, dist, co2, offset)) in &flight_data_map {
        // Distance
        if let Some(d) = dist {
            total_distance_km += d;
        }

        // CO2
        if let Some(c) = co2 {
            total_co2_kg += c;
        }

        // Carbon offset
        if offset.unwrap_or(0) > 0 {
            carbon_offset_purchased = true;
        }

        // Date
        flight_dates.push(datetime.clone());

        // Route
        let route = format!("{} → {}", dep, arr);
        *route_counts.entry(route).or_insert(0) += 1;

        // Companions - exclude ANY of this passenger's aliases
        if let Some(passenger_part) = notes.strip_prefix("Passengers: ") {
            for companion in passenger_part.split(',').map(|s| s.trim()) {
                if !companion.is_empty() && !alias_set.contains(companion) {
                    *companion_counts.entry(companion.to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    // Sort dates to get first and last
    flight_dates.sort();
    let first_flight_date = flight_dates.first().cloned();
    let last_flight_date = flight_dates.last().cloned();

    // Calculate averages
    let total_flights = all_flight_ids.len() as i32;
    let avg_co2_per_flight_kg = if total_flights > 0 {
        total_co2_kg / total_flights as f64
    } else {
        0.0
    };

    // Build top routes
    let mut top_routes: Vec<PassengerRoute> = route_counts
        .into_iter()
        .map(|(route, count)| {
            let parts: Vec<&str> = route.split(" → ").collect();
            PassengerRoute {
                route: route.clone(),
                departure_airport: parts.first().unwrap_or(&"").to_string(),
                arrival_airport: parts.get(1).unwrap_or(&"").to_string(),
                flight_count: count,
            }
        })
        .collect();
    top_routes.sort_by(|a, b| b.flight_count.cmp(&a.flight_count));
    top_routes.truncate(5);

    // Build travel companions
    let mut travel_companions: Vec<PassengerCompanion> = companion_counts
        .into_iter()
        .map(|(name, count)| PassengerCompanion {
            name,
            flight_count: count,
        })
        .collect();
    travel_companions.sort_by(|a, b| b.flight_count.cmp(&a.flight_count));
    travel_companions.truncate(10);

    Ok(PassengerDetailsAggregated {
        passenger_id,
        canonical_name,
        total_flights,
        total_distance_km,
        top_routes,
        travel_companions,
        first_flight_date,
        last_flight_date,
        total_co2_kg,
        avg_co2_per_flight_kg,
        carbon_offset_purchased,
        aliases,
    })
}

/// Execute batch split on multiple passengers
#[tauri::command]
pub fn batch_split_passengers(
    passenger_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<BatchSplitResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut total_processed = 0;
    let mut total_new_passengers = 0;
    let mut total_flights_reassigned = 0;
    let mut errors: Vec<String> = Vec::new();

    for passenger_id in passenger_ids {
        // Get passenger info
        let passenger_info: Result<(String, i32), _> = db.conn.query_row(
            "SELECT canonical_name, total_flights FROM passengers WHERE id = ?1",
            params![passenger_id],
            |row| Ok((row.get(0)?, row.get(1)?))
        );

        let (canonical_name, _) = match passenger_info {
            Ok(info) => info,
            Err(e) => {
                errors.push(format!("Passenger {} not found: {}", passenger_id, e));
                continue;
            }
        };

        // Detect delimiter
        let delimiter = match detect_compound_pattern(&canonical_name) {
            Some(d) => d,
            None => {
                errors.push(format!("No compound pattern detected for: {}", canonical_name));
                continue;
            }
        };

        // Get proposed names
        let new_names: Vec<String> = canonical_name
            .split(&delimiter)
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty() && s.len() >= 2)
            .collect();

        if new_names.len() <= 1 {
            errors.push(format!("Cannot split '{}' - would result in single name", canonical_name));
            continue;
        }

        // Get flight IDs for this passenger
        let flight_ids: Vec<String> = {
            let mut stmt = db.conn.prepare(
                "SELECT flight_id FROM flight_passengers WHERE passenger_id = ?1"
            ).map_err(|e| e.to_string())?;
            let rows = stmt.query_map(params![passenger_id], |row| row.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            rows.filter_map(|r| r.ok()).collect()
        };

        // Process each new name
        db.conn.execute("SAVEPOINT batch_split", []).map_err(|e| e.to_string())?;

        let result: Result<(i32, i32), String> = (|| {
            let mut new_count = 0;
            let mut flights_count = 0;

            for name in &new_names {
                // Check if name already exists
                let existing_passenger_id: Option<String> = db.conn.query_row(
                    "SELECT passenger_id FROM passenger_aliases WHERE raw_name = ?1",
                    params![name],
                    |row| row.get(0)
                ).optional().map_err(|e| e.to_string())?;

                let target_id = if let Some(existing_id) = existing_passenger_id {
                    existing_id
                } else {
                    let new_id = Uuid::new_v4().to_string();
                    let alias_id = Uuid::new_v4().to_string();

                    db.conn.execute(
                        "INSERT INTO passengers (id, canonical_name, total_flights, created_at, updated_at)
                         VALUES (?1, ?2, 0, datetime('now'), datetime('now'))",
                        params![new_id, name]
                    ).map_err(|e| e.to_string())?;

                    db.conn.execute(
                        "INSERT INTO passenger_aliases (id, passenger_id, raw_name, usage_count, match_type, confidence, created_at)
                         VALUES (?1, ?2, ?3, 0, 'batch_split', 1.0, datetime('now'))",
                        params![alias_id, new_id, name]
                    ).map_err(|e| e.to_string())?;

                    new_count += 1;
                    new_id
                };

                // Link flights
                for flight_id in &flight_ids {
                    let result = db.conn.execute(
                        "INSERT OR IGNORE INTO flight_passengers (flight_id, passenger_id, created_at)
                         VALUES (?1, ?2, datetime('now'))",
                        params![flight_id, target_id]
                    ).map_err(|e| e.to_string())?;
                    if result > 0 {
                        flights_count += 1;
                    }
                }

                // Update flight count
                db.conn.execute(
                    "UPDATE passengers SET
                        total_flights = (SELECT COUNT(DISTINCT flight_id) FROM flight_passengers WHERE passenger_id = ?1),
                        updated_at = datetime('now')
                     WHERE id = ?1",
                    params![target_id]
                ).map_err(|e| e.to_string())?;
            }

            // Delete original
            db.conn.execute(
                "DELETE FROM flight_passengers WHERE passenger_id = ?1",
                params![passenger_id]
            ).map_err(|e| e.to_string())?;

            db.conn.execute(
                "DELETE FROM passenger_aliases WHERE passenger_id = ?1",
                params![passenger_id]
            ).map_err(|e| e.to_string())?;

            db.conn.execute(
                "DELETE FROM passengers WHERE id = ?1",
                params![passenger_id]
            ).map_err(|e| e.to_string())?;

            Ok((new_count, flights_count))
        })();

        match result {
            Ok((new_count, flights_count)) => {
                db.conn.execute("RELEASE SAVEPOINT batch_split", []).map_err(|e| e.to_string())?;
                total_processed += 1;
                total_new_passengers += new_count;
                total_flights_reassigned += flights_count;
            }
            Err(e) => {
                db.conn.execute("ROLLBACK TO SAVEPOINT batch_split", []).ok();
                errors.push(format!("Failed to split '{}': {}", canonical_name, e));
            }
        }
    }

    Ok(BatchSplitResult {
        total_processed,
        total_new_passengers,
        total_flights_reassigned,
        errors,
    })
}
