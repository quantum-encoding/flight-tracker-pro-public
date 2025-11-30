// Tauri commands for document ingestion and processing

use tauri::State;
use std::path::PathBuf;

use super::AppState;
use crate::doc_ingestion::{IngestionQueue, QueueStats};
use crate::doc_worker::WorkerPool;

#[tauri::command]
pub fn enqueue_pdf_for_processing(
    user_id: String,
    pdf_path: String,
    document_id: Option<String>,
    priority: Option<i32>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Get app work directory
    let work_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("work");

    let db_path = PathBuf::from(
        db.get_connection()
            .path()
            .ok_or("Failed to get database path")?
    );

    // Create ingestion queue
    let queue = IngestionQueue::new(db_path, work_dir)
        .map_err(|e| e.to_string())?;

    // Enqueue the document
    let job_id = queue
        .enqueue_document(
            &user_id,
            &PathBuf::from(pdf_path),
            document_id.as_deref(),
            priority.unwrap_or(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(job_id)
}

#[tauri::command]
pub fn get_ingestion_queue_stats(
    state: State<'_, AppState>,
) -> Result<QueueStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let db_path = PathBuf::from(
        db.get_connection()
            .path()
            .ok_or("Failed to get database path")?
    );

    let work_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("work");

    let queue = IngestionQueue::new(db_path, work_dir)
        .map_err(|e| e.to_string())?;

    let stats = queue.get_stats().map_err(|e| e.to_string())?;

    Ok(stats)
}

#[tauri::command]
pub fn recover_ingestion_queue(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let db_path = PathBuf::from(
        db.get_connection()
            .path()
            .ok_or("Failed to get database path")?
    );

    let work_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("work");

    let queue = IngestionQueue::new(db_path, work_dir)
        .map_err(|e| e.to_string())?;

    let recovered = queue.recover_crashed_jobs()
        .map_err(|e| e.to_string())?;

    Ok(recovered)
}

#[tauri::command]
pub async fn start_document_worker(
    state: State<'_, AppState>,
) -> Result<String, String> {
    // Get Gemini API key
    let gemini_api_key = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_setting("gemini_api_key")
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Gemini API key not configured".to_string())?
    };

    let db_path = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        PathBuf::from(
            db.get_connection()
                .path()
                .ok_or("Failed to get database path")?
        )
    };

    let work_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join("work");

    // Create worker pool
    let worker_pool = WorkerPool::new(db_path, work_dir, gemini_api_key)
        .map_err(|e| e.to_string())?;

    // Spawn worker in background
    tauri::async_runtime::spawn(async move {
        if let Err(e) = worker_pool.run_worker().await {
            eprintln!("❌ Document worker error: {}", e);
        }
    });

    Ok("Document worker started successfully".to_string())
}

#[tauri::command(rename_all = "camelCase")]
pub fn query_relationship_graph(
    source_type: String,
    source_id: String,
    max_depth: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<GraphRelationship>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let _depth = max_depth.unwrap_or(2);
    let mut relationships = Vec::new();

    // Query direct relationships
    let mut stmt = db
        .get_connection()
        .prepare(
            "SELECT id, source_type, source_id, target_type, target_id, relationship_type, weight, evidence_ids
             FROM relationship_graph
             WHERE source_type = ?1 AND source_id = ?2
             UNION
             SELECT id, source_type, source_id, target_type, target_id, relationship_type, weight, evidence_ids
             FROM relationship_graph
             WHERE target_type = ?1 AND target_id = ?2
             ORDER BY weight DESC
             LIMIT 100",
        )
        .map_err(|e| e.to_string())?;

    let results = stmt
        .query_map([&source_type, &source_id], |row| {
            Ok(GraphRelationship {
                id: row.get(0)?,
                source_type: row.get(1)?,
                source_id: row.get(2)?,
                target_type: row.get(3)?,
                target_id: row.get(4)?,
                relationship_type: row.get(5)?,
                weight: row.get(6)?,
                evidence: row.get::<_, Option<String>>(7)?,
            })
        })
        .map_err(|e| e.to_string())?;

    for result in results {
        relationships.push(result.map_err(|e| e.to_string())?);
    }

    Ok(relationships)
}

#[derive(Debug, serde::Serialize)]
pub struct GraphRelationship {
    pub id: String,
    pub source_type: String,
    pub source_id: String,
    pub target_type: String,
    pub target_id: String,
    pub relationship_type: String,
    pub weight: f64,
    pub evidence: Option<String>,
}

/// Build relationship graph from existing flight data
/// This populates the graph with:
/// - passenger <-> flight relationships
/// - flight <-> airport relationships (departure/arrival)
/// - passenger <-> passenger co-travel relationships
#[tauri::command]
pub fn build_flight_relationships(
    state: State<'_, AppState>,
) -> Result<BuildRelationshipsResult, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    let mut relationships_created = 0;
    let mut passengers_processed = 0;
    let mut flights_processed = 0;

    // Use a transaction for much faster batch inserts
    conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

    // Clear existing flight-based relationships to rebuild fresh
    conn.execute(
        "DELETE FROM relationship_graph WHERE source_type IN ('person', 'flight', 'airport') OR target_type IN ('person', 'flight', 'airport')",
        [],
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    })?;

    // 1. Build passenger <-> flight relationships
    // Passengers are stored in the 'notes' field with format "Passengers: Name1, Name2, ..."
    let mut stmt = conn.prepare(
        "SELECT f.id, f.flight_number, f.notes, f.departure_airport, f.arrival_airport, f.departure_datetime
         FROM flights f
         WHERE f.notes IS NOT NULL AND f.notes LIKE 'Passengers:%'"
    ).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    })?;

    let flights: Vec<(String, String, String, String, String, String)> = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1).unwrap_or_default(),
            row.get::<_, String>(2).unwrap_or_default(),
            row.get::<_, String>(3).unwrap_or_default(),
            row.get::<_, String>(4).unwrap_or_default(),
            row.get::<_, String>(5).unwrap_or_default(),
        ))
    }).map_err(|e| {
        let _ = conn.execute("ROLLBACK", []);
        e.to_string()
    })?
    .filter_map(|r| r.ok())
    .collect();

    drop(stmt); // Release the prepared statement before more operations

    // Collect all relationships to insert in batches
    let mut passenger_flight_rels: Vec<(String, String, String, String)> = Vec::new();
    let mut flight_airport_rels: Vec<(String, String, String, String, String)> = Vec::new();
    let mut flight_passengers: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

    for (flight_id, flight_number, notes_str, dep_airport, arr_airport, dep_time) in &flights {
        flights_processed += 1;

        // Parse passengers from notes field (format: "Passengers: Name1, Name2, ...")
        let passengers: Vec<String> = if notes_str.starts_with("Passengers:") {
            notes_str
                .trim_start_matches("Passengers:")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            vec![]
        };

        flight_passengers.insert(flight_id.clone(), passengers.clone());

        // Collect passenger -> flight relationships
        for passenger in &passengers {
            passengers_processed += 1;
            let evidence = format!("Flight {} on {}", flight_number, dep_time);
            passenger_flight_rels.push((passenger.clone(), flight_id.clone(), evidence, uuid::Uuid::new_v4().to_string()));
        }

        // Collect flight -> airport relationships
        if !dep_airport.is_empty() {
            flight_airport_rels.push((flight_id.clone(), dep_airport.clone(), "departed_from".to_string(), dep_time.clone(), uuid::Uuid::new_v4().to_string()));
        }

        if !arr_airport.is_empty() {
            flight_airport_rels.push((flight_id.clone(), arr_airport.clone(), "arrived_at".to_string(), dep_time.clone(), uuid::Uuid::new_v4().to_string()));
        }
    }

    // Batch insert passenger-flight relationships
    for (passenger, flight_id, evidence, rel_id) in &passenger_flight_rels {
        conn.execute(
            "INSERT OR REPLACE INTO relationship_graph
             (id, source_type, source_id, target_type, target_id, relationship_type, weight, evidence_ids, created_at, updated_at)
             VALUES (?1, 'person', ?2, 'flight', ?3, 'passenger_on', 1.0, ?4, datetime('now'), datetime('now'))",
            rusqlite::params![rel_id, passenger, flight_id, evidence],
        ).map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            e.to_string()
        })?;
        relationships_created += 1;
    }

    // Batch insert flight-airport relationships
    for (flight_id, airport, rel_type, evidence, rel_id) in &flight_airport_rels {
        conn.execute(
            "INSERT OR REPLACE INTO relationship_graph
             (id, source_type, source_id, target_type, target_id, relationship_type, weight, evidence_ids, created_at, updated_at)
             VALUES (?1, 'flight', ?2, 'airport', ?3, ?4, 1.0, ?5, datetime('now'), datetime('now'))",
            rusqlite::params![rel_id, flight_id, airport, rel_type, evidence],
        ).map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            e.to_string()
        })?;
        relationships_created += 1;
    }

    // 2. Build passenger <-> passenger co-travel relationships
    let mut co_travel_counts: std::collections::HashMap<(String, String), i32> = std::collections::HashMap::new();

    for passengers in flight_passengers.values() {
        for i in 0..passengers.len() {
            for j in (i + 1)..passengers.len() {
                let p1 = &passengers[i];
                let p2 = &passengers[j];
                let key = if p1 < p2 {
                    (p1.clone(), p2.clone())
                } else {
                    (p2.clone(), p1.clone())
                };
                *co_travel_counts.entry(key).or_insert(0) += 1;
            }
        }
    }

    for ((p1, p2), count) in co_travel_counts {
        let rel_id = uuid::Uuid::new_v4().to_string();
        let weight = (count as f64).min(10.0) / 10.0;
        let evidence = format!("Traveled together on {} flight(s)", count);

        conn.execute(
            "INSERT OR REPLACE INTO relationship_graph
             (id, source_type, source_id, target_type, target_id, relationship_type, weight, evidence_ids, created_at, updated_at)
             VALUES (?1, 'person', ?2, 'person', ?3, 'traveled_with', ?4, ?5, datetime('now'), datetime('now'))",
            rusqlite::params![rel_id, p1, p2, weight, evidence],
        ).map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            e.to_string()
        })?;
        relationships_created += 1;
    }

    // 3. Build airport <-> airport route relationships
    let mut route_counts: std::collections::HashMap<(String, String), i32> = std::collections::HashMap::new();

    for (_, _, _, dep_airport, arr_airport, _) in &flights {
        if !dep_airport.is_empty() && !arr_airport.is_empty() && dep_airport != arr_airport {
            let key = (dep_airport.clone(), arr_airport.clone());
            *route_counts.entry(key).or_insert(0) += 1;
        }
    }

    for ((from_airport, to_airport), count) in route_counts {
        let rel_id = uuid::Uuid::new_v4().to_string();
        let weight = (count as f64).min(20.0) / 20.0;
        let evidence = format!("{} flight(s) on this route", count);

        conn.execute(
            "INSERT OR REPLACE INTO relationship_graph
             (id, source_type, source_id, target_type, target_id, relationship_type, weight, evidence_ids, created_at, updated_at)
             VALUES (?1, 'airport', ?2, 'airport', ?3, 'route_to', ?4, ?5, datetime('now'), datetime('now'))",
            rusqlite::params![rel_id, from_airport, to_airport, weight, evidence],
        ).map_err(|e| {
            let _ = conn.execute("ROLLBACK", []);
            e.to_string()
        })?;
        relationships_created += 1;
    }

    // Commit the transaction
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;

    Ok(BuildRelationshipsResult {
        relationships_created,
        passengers_processed,
        flights_processed,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct BuildRelationshipsResult {
    pub relationships_created: i32,
    pub passengers_processed: i32,
    pub flights_processed: i32,
}

/// Get statistics about the relationship graph
#[tauri::command]
pub fn get_relationship_stats(
    state: State<'_, AppState>,
) -> Result<RelationshipStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.get_connection();

    let total_relationships: i32 = conn.query_row(
        "SELECT COUNT(*) FROM relationship_graph",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    let person_count: i32 = conn.query_row(
        "SELECT COUNT(DISTINCT source_id) FROM relationship_graph WHERE source_type = 'person'
         UNION SELECT COUNT(DISTINCT target_id) FROM relationship_graph WHERE target_type = 'person'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    let flight_count: i32 = conn.query_row(
        "SELECT COUNT(DISTINCT source_id) FROM relationship_graph WHERE source_type = 'flight'
         UNION SELECT COUNT(DISTINCT target_id) FROM relationship_graph WHERE target_type = 'flight'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    let airport_count: i32 = conn.query_row(
        "SELECT COUNT(DISTINCT source_id) FROM relationship_graph WHERE source_type = 'airport'
         UNION SELECT COUNT(DISTINCT target_id) FROM relationship_graph WHERE target_type = 'airport'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    // Get relationship type breakdown
    let mut stmt = conn.prepare(
        "SELECT relationship_type, COUNT(*) FROM relationship_graph GROUP BY relationship_type"
    ).map_err(|e| e.to_string())?;

    let type_counts: Vec<(String, i32)> = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
    }).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(RelationshipStats {
        total_relationships,
        person_count,
        flight_count,
        airport_count,
        relationship_types: type_counts,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct RelationshipStats {
    pub total_relationships: i32,
    pub person_count: i32,
    pub flight_count: i32,
    pub airport_count: i32,
    pub relationship_types: Vec<(String, i32)>,
}
