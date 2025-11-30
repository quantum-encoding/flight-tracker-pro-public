// Document Processing Worker Pool with Multi-Agent Pipeline
// Implements the "Conductor" pattern with specialized agents

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::doc_ingestion::{DocumentChunk, IngestionQueue, ProcessingStage, ChunkStatus};

/// Represents an extracted entity from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub entity_type: String,  // "person", "date", "location", "flight_number", "tail_number"
    pub value: String,
    pub confidence: f64,
    pub context: Option<String>,
}

/// Represents a match between a document chunk and a flight record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightMatch {
    pub flight_id: String,
    pub match_type: String,  // "tail_number", "date_location", "passenger_name"
    pub confidence: f64,
    pub evidence: Vec<String>,
}

/// Agent A: OCR Text Extractor with Hybrid Fallback
/// Implements "The Fallback Protocol" for scanned PDFs
pub struct OcrAgent {
    api_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtractionMethod {
    TextExtract,      // Fast PDF text extraction
    VisionRequired,   // Scanned image, needs Vision AI
}

impl OcrAgent {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    /// Hybrid extraction: Try fast text extraction first, fall back to Vision OCR
    pub async fn extract_text(&self, chunk_path: &str) -> Result<(String, ExtractionMethod)> {
        // Attempt 1: Fast Extract using pdf-extract
        match self.try_fast_extract(chunk_path) {
            Ok(text) if text.len() > 50 => {
                // Success! Text is substantial, use it
                eprintln!("📄 Fast extract successful: {} chars", text.len());
                return Ok((text, ExtractionMethod::TextExtract));
            }
            Ok(_) | Err(_) => {
                // Text is empty/short or extraction failed
                eprintln!("📸 Fast extract failed/insufficient, falling back to Vision OCR");
            }
        }

        // Attempt 2: Vision Fallback - This is a scanned document
        let text = self.vision_ocr_extract(chunk_path).await
            .context("Vision OCR fallback failed")?;

        Ok((text, ExtractionMethod::VisionRequired))
    }

    /// Attempt fast text extraction from PDF
    fn try_fast_extract(&self, chunk_path: &str) -> Result<String> {
        let bytes = std::fs::read(chunk_path)?;

        // Try pdf-extract for native text
        let text = pdf_extract::extract_text_from_mem(&bytes)
            .context("Failed to extract text from PDF")?;

        Ok(text.trim().to_string())
    }

    /// Vision OCR: Render PDF page as image and send to Gemini
    async fn vision_ocr_extract(&self, chunk_path: &str) -> Result<String> {
        // Read the PDF/image file
        let file_bytes = std::fs::read(chunk_path)
            .context("Failed to read file for Vision OCR")?;

        // If it's already an image format (jpg, png), use it directly
        // Otherwise, we'd need to render the PDF page - for now, try direct OCR
        let text = crate::ocr::extract_document_text_with_retry(
            file_bytes,
            &self.api_key,
            5, // max retries
        )
        .await
        .context("Vision OCR extraction failed")?;

        Ok(text)
    }
}

/// Agent B: Entity Extractor
/// Extracts structured entities (names, dates, locations) from text
pub struct EntityExtractorAgent {
    api_key: String,
}

impl EntityExtractorAgent {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    /// Extract entities from text using Gemini
    pub async fn extract_entities(&self, text: &str) -> Result<Vec<ExtractedEntity>> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-lite:generateContent?key={}",
            self.api_key
        );

        let prompt = format!(
            r#"Extract all relevant entities from this document text and return them as JSON.

Focus on:
- Person names (type: "person")
- Dates (type: "date") - ISO 8601 format YYYY-MM-DD
- Locations (type: "location") - airports, cities, countries
- Flight numbers (type: "flight_number") - e.g., AA1234, N908JE
- Aircraft tail numbers (type: "tail_number") - e.g., N908JE

Return ONLY a JSON array of entities in this exact format:
[
  {{"entity_type": "person", "value": "Jane Doe", "confidence": 0.95, "context": "passenger manifest"}},
  {{"entity_type": "date", "value": "2023-05-15", "confidence": 0.99, "context": "flight date"}},
  {{"entity_type": "tail_number", "value": "N908JE", "confidence": 0.95, "context": "aircraft registration"}}
]

Text to analyze:
{}
"#,
            text
        );

        let payload = serde_json::json!({
            "contents": [{
                "parts": [{"text": prompt}]
            }],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 2048,
                "responseMimeType": "application/json"
            }
        });

        let response = client.post(&url).json(&payload).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Gemini API error: {}", error_text));
        }

        let response_json: serde_json::Value = response.json().await?;

        // Extract the text from Gemini's response
        let response_text = response_json
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract text from Gemini response"))?;

        // Parse the JSON response
        let entities: Vec<ExtractedEntity> = serde_json::from_str(response_text.trim())
            .context("Failed to parse entity JSON")?;

        Ok(entities)
    }
}

/// Agent C: Flight Matcher
/// Queries the local flight database to find matches
pub struct FlightMatcherAgent {
    db_path: PathBuf,
}

impl FlightMatcherAgent {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Find flights matching the extracted entities
    pub fn find_matches(&self, entities: &[ExtractedEntity]) -> Result<Vec<FlightMatch>> {
        let conn = Connection::open(&self.db_path)?;
        let mut matches = Vec::new();

        // Extract specific entity types
        let dates: Vec<&str> = entities
            .iter()
            .filter(|e| e.entity_type == "date")
            .map(|e| e.value.as_str())
            .collect();

        let locations: Vec<&str> = entities
            .iter()
            .filter(|e| e.entity_type == "location")
            .map(|e| e.value.as_str())
            .collect();

        let tail_numbers: Vec<&str> = entities
            .iter()
            .filter(|e| e.entity_type == "tail_number")
            .map(|e| e.value.as_str())
            .collect();

        let flight_numbers: Vec<&str> = entities
            .iter()
            .filter(|e| e.entity_type == "flight_number")
            .map(|e| e.value.as_str())
            .collect();

        let person_names: Vec<&str> = entities
            .iter()
            .filter(|e| e.entity_type == "person")
            .map(|e| e.value.as_str())
            .collect();

        // Strategy 1: Match by tail number (highest confidence)
        for tail in &tail_numbers {
            let mut stmt = conn.prepare(
                "SELECT id FROM flights WHERE aircraft_registration = ?1"
            )?;

            let flight_ids: Vec<String> = stmt
                .query_map([tail], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;

            for flight_id in flight_ids {
                matches.push(FlightMatch {
                    flight_id,
                    match_type: "tail_number".to_string(),
                    confidence: 0.95,
                    evidence: vec![format!("Aircraft registration: {}", tail)],
                });
            }
        }

        // Strategy 2: Match by flight number
        for flight_num in &flight_numbers {
            let mut stmt = conn.prepare(
                "SELECT id FROM flights WHERE flight_number = ?1"
            )?;

            let flight_ids: Vec<String> = stmt
                .query_map([flight_num], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;

            for flight_id in flight_ids {
                matches.push(FlightMatch {
                    flight_id,
                    match_type: "flight_number".to_string(),
                    confidence: 0.90,
                    evidence: vec![format!("Flight number: {}", flight_num)],
                });
            }
        }

        // Strategy 3: Match by date + location
        if !dates.is_empty() && !locations.is_empty() {
            for date in &dates {
                for location in &locations {
                    let mut stmt = conn.prepare(
                        "SELECT id FROM flights
                         WHERE DATE(departure_datetime) = ?1
                         AND (departure_airport LIKE ?2 OR arrival_airport LIKE ?2)"
                    )?;

                    let location_pattern = format!("%{}%", location);
                    let flight_ids: Vec<String> = stmt
                        .query_map([*date, location_pattern.as_str()], |row| row.get(0))?
                        .collect::<Result<Vec<_>, _>>()?;

                    for flight_id in flight_ids {
                        matches.push(FlightMatch {
                            flight_id,
                            match_type: "date_location".to_string(),
                            confidence: 0.75,
                            evidence: vec![
                                format!("Date: {}", date),
                                format!("Location: {}", location),
                            ],
                        });
                    }
                }
            }
        }

        // Strategy 4: Match by passenger name (via passenger_mappings)
        for person in &person_names {
            let mut stmt = conn.prepare(
                "SELECT f.id FROM flights f
                 INNER JOIN passenger_mappings pm ON f.id = pm.flight_id
                 WHERE pm.passenger_name LIKE ?1 OR pm.full_name LIKE ?1"
            )?;

            let name_pattern = format!("%{}%", person);
            let flight_ids: Vec<String> = stmt
                .query_map([&name_pattern], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;

            for flight_id in flight_ids {
                matches.push(FlightMatch {
                    flight_id,
                    match_type: "passenger_name".to_string(),
                    confidence: 0.80,
                    evidence: vec![format!("Passenger: {}", person)],
                });
            }
        }

        Ok(matches)
    }
}

/// Agent D: Graph Builder
/// Builds relationship graph connecting people, flights, locations, and documents
pub struct GraphBuilderAgent {
    db_path: PathBuf,
}

impl GraphBuilderAgent {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Add a relationship edge to the graph
    pub fn add_relationship(
        &self,
        source_type: &str,
        source_id: &str,
        target_type: &str,
        target_id: &str,
        relationship_type: &str,
        weight: f64,
        evidence: Vec<String>,
    ) -> Result<String> {
        let conn = Connection::open(&self.db_path)?;
        let relationship_id = Uuid::new_v4().to_string();

        let evidence_json = serde_json::to_string(&evidence)?;

        conn.execute(
            "INSERT INTO relationship_graph
             (id, source_type, source_id, target_type, target_id, relationship_type, weight, evidence_ids, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL)
             ON CONFLICT(source_type, source_id, target_type, target_id, relationship_type)
             DO UPDATE SET weight = weight + ?7, evidence_ids = ?8, updated_at = datetime('now')",
            rusqlite::params![
                relationship_id,
                source_type,
                source_id,
                target_type,
                target_id,
                relationship_type,
                weight,
                evidence_json,
            ],
        )?;

        Ok(relationship_id)
    }

    /// Build graph connections from document matches
    pub fn build_connections(
        &self,
        chunk_id: &str,
        entities: &[ExtractedEntity],
        matches: &[FlightMatch],
    ) -> Result<()> {
        // Connect document chunk to matched flights
        for flight_match in matches {
            self.add_relationship(
                "document_chunk",
                chunk_id,
                "flight",
                &flight_match.flight_id,
                &flight_match.match_type,
                flight_match.confidence,
                flight_match.evidence.clone(),
            )?;
        }

        // Connect entities to the document chunk
        for entity in entities {
            let entity_id = format!("{}:{}", entity.entity_type, entity.value);
            self.add_relationship(
                "document_chunk",
                chunk_id,
                "entity",
                &entity_id,
                "mentions",
                entity.confidence,
                vec![entity.context.clone().unwrap_or_default()],
            )?;
        }

        // Connect persons to flights (cross-reference)
        for entity in entities.iter().filter(|e| e.entity_type == "person") {
            for flight_match in matches {
                if flight_match.match_type == "passenger_name" {
                    let person_id = format!("person:{}", entity.value);
                    self.add_relationship(
                        &person_id,
                        &entity.value,
                        "flight",
                        &flight_match.flight_id,
                        "traveled_on",
                        entity.confidence * flight_match.confidence,
                        vec![format!("Document evidence: {}", chunk_id)],
                    )?;
                }
            }
        }

        Ok(())
    }
}

/// Multi-Agent Worker Pool
/// Coordinates the processing pipeline across all agents
pub struct WorkerPool {
    queue: Arc<Mutex<IngestionQueue>>,
    ocr_agent: Arc<OcrAgent>,
    entity_agent: Arc<EntityExtractorAgent>,
    matcher_agent: Arc<FlightMatcherAgent>,
    graph_agent: Arc<GraphBuilderAgent>,
    db_path: PathBuf,
}

impl WorkerPool {
    pub fn new(
        db_path: PathBuf,
        work_dir: PathBuf,
        gemini_api_key: String,
    ) -> Result<Self> {
        let queue = IngestionQueue::new(db_path.clone(), work_dir)?;

        Ok(Self {
            queue: Arc::new(Mutex::new(queue)),
            ocr_agent: Arc::new(OcrAgent::new(gemini_api_key.clone())),
            entity_agent: Arc::new(EntityExtractorAgent::new(gemini_api_key)),
            matcher_agent: Arc::new(FlightMatcherAgent::new(db_path.clone())),
            graph_agent: Arc::new(GraphBuilderAgent::new(db_path.clone())),
            db_path,
        })
    }

    /// Process a single chunk through the entire pipeline
    pub async fn process_chunk(&self, mut chunk: DocumentChunk) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;

        // Stage 1: OCR Extraction with Hybrid Fallback
        if chunk.processing_stage == ProcessingStage::Pending {
            let (text, method) = self.ocr_agent.extract_text(&chunk.content_path).await?;

            let method_str = match method {
                ExtractionMethod::TextExtract => "text_extract",
                ExtractionMethod::VisionRequired => "vision_ocr",
            };

            // Store OCR text and processing method
            conn.execute(
                "UPDATE document_chunks SET ocr_text = ?1, processing_stage = 'ocr', processing_method = ?3 WHERE id = ?2",
                rusqlite::params![text, chunk.id, method_str],
            )?;

            chunk.processing_stage = ProcessingStage::OcrExtraction;
        }

        // Stage 2: Entity Extraction
        if chunk.processing_stage == ProcessingStage::OcrExtraction {
            let ocr_text: String = conn.query_row(
                "SELECT ocr_text FROM document_chunks WHERE id = ?1",
                [&chunk.id],
                |row| row.get(0),
            )?;

            let entities = self.entity_agent.extract_entities(&ocr_text).await?;

            // Store entities
            let entities_json = serde_json::to_string(&entities)?;
            conn.execute(
                "UPDATE document_chunks SET extracted_entities = ?1, processing_stage = 'entity_extraction' WHERE id = ?2",
                rusqlite::params![entities_json, chunk.id],
            )?;

            // Insert into entity_extractions table
            for entity in &entities {
                let entity_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO entity_extractions
                     (id, chunk_id, entity_type, entity_value, confidence, context)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        entity_id,
                        chunk.id,
                        entity.entity_type,
                        entity.value,
                        entity.confidence,
                        entity.context,
                    ],
                )?;
            }

            chunk.processing_stage = ProcessingStage::EntityExtraction;
        }

        // Stage 3: Flight Matching
        if chunk.processing_stage == ProcessingStage::EntityExtraction {
            let entities_json: String = conn.query_row(
                "SELECT extracted_entities FROM document_chunks WHERE id = ?1",
                [&chunk.id],
                |row| row.get(0),
            )?;

            let entities: Vec<ExtractedEntity> = serde_json::from_str(&entities_json)?;
            let matches = self.matcher_agent.find_matches(&entities)?;

            // Store matches
            for flight_match in &matches {
                let match_id = Uuid::new_v4().to_string();
                let evidence_json = serde_json::to_string(&flight_match.evidence)?;

                conn.execute(
                    "INSERT INTO document_matches
                     (id, chunk_id, flight_id, match_type, confidence, evidence)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        match_id,
                        chunk.id,
                        flight_match.flight_id,
                        flight_match.match_type,
                        flight_match.confidence,
                        evidence_json,
                    ],
                )?;
            }

            conn.execute(
                "UPDATE document_chunks SET processing_stage = 'flight_matching' WHERE id = ?1",
                [&chunk.id],
            )?;

            chunk.processing_stage = ProcessingStage::FlightMatching;
        }

        // Stage 4: Graph Building
        if chunk.processing_stage == ProcessingStage::FlightMatching {
            let entities_json: String = conn.query_row(
                "SELECT extracted_entities FROM document_chunks WHERE id = ?1",
                [&chunk.id],
                |row| row.get(0),
            )?;

            let entities: Vec<ExtractedEntity> = serde_json::from_str(&entities_json)?;

            // Get matches
            let mut stmt = conn.prepare(
                "SELECT flight_id, match_type, confidence, evidence FROM document_matches WHERE chunk_id = ?1"
            )?;

            let matches: Vec<FlightMatch> = stmt
                .query_map([&chunk.id], |row| {
                    let evidence_json: String = row.get(3)?;
                    let evidence: Vec<String> = serde_json::from_str(&evidence_json).unwrap_or_default();

                    Ok(FlightMatch {
                        flight_id: row.get(0)?,
                        match_type: row.get(1)?,
                        confidence: row.get(2)?,
                        evidence,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;

            // Build graph connections
            self.graph_agent.build_connections(&chunk.id, &entities, &matches)?;

            conn.execute(
                "UPDATE document_chunks SET processing_stage = 'completed', status = 'completed' WHERE id = ?1",
                [&chunk.id],
            )?;
        }

        Ok(())
    }

    /// Main worker loop - continuously processes chunks from the queue
    pub async fn run_worker(&self) -> Result<()> {
        loop {
            let chunk = {
                let queue = self.queue.lock().await;
                queue.get_next_chunk()?
            };

            if let Some(chunk) = chunk {
                eprintln!("🔧 Processing chunk {} (page {})", chunk.id, chunk.chunk_number);

                // Mark as processing
                {
                    let queue = self.queue.lock().await;
                    queue.update_chunk_progress(
                        &chunk.id,
                        ChunkStatus::Processing,
                        chunk.processing_stage.clone(),
                        None,
                    )?;
                }

                // Process the chunk
                match self.process_chunk(chunk.clone()).await {
                    Ok(_) => {
                        let queue = self.queue.lock().await;
                        queue.update_chunk_progress(
                            &chunk.id,
                            ChunkStatus::Completed,
                            ProcessingStage::Completed,
                            None,
                        )?;
                        eprintln!("✅ Chunk {} completed", chunk.id);
                    }
                    Err(e) => {
                        eprintln!("❌ Chunk {} failed: {}", chunk.id, e);
                        let queue = self.queue.lock().await;
                        queue.update_chunk_progress(
                            &chunk.id,
                            ChunkStatus::Failed,
                            chunk.processing_stage,
                            Some(&e.to_string()),
                        )?;
                    }
                }
            } else {
                // No chunks available, sleep for a bit
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    }
}
