// Document Ingestion Pipeline with PDF Chunking and Multi-Agent Processing
// Implements "The Shredder" - splits PDFs into manageable chunks for AI processing

use anyhow::{Context, Result};
use lopdf::Document as PdfDocument;
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;

/// Represents a single chunk from a document (typically one PDF page)
#[derive(Debug, Clone)]
pub struct DocumentChunk {
    pub id: String,
    pub queue_id: String,
    pub chunk_hash: String,
    pub chunk_number: i32,
    pub content_path: String,
    pub status: ChunkStatus,
    pub processing_stage: ProcessingStage,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChunkStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

impl ChunkStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ChunkStatus::Pending => "pending",
            ChunkStatus::Processing => "processing",
            ChunkStatus::Completed => "completed",
            ChunkStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => ChunkStatus::Pending,
            "processing" => ChunkStatus::Processing,
            "completed" => ChunkStatus::Completed,
            "failed" => ChunkStatus::Failed,
            _ => ChunkStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessingStage {
    Pending,
    OcrExtraction,
    EntityExtraction,
    FlightMatching,
    GraphBuilding,
    Completed,
}

impl ProcessingStage {
    pub fn as_str(&self) -> &str {
        match self {
            ProcessingStage::Pending => "pending",
            ProcessingStage::OcrExtraction => "ocr",
            ProcessingStage::EntityExtraction => "entity_extraction",
            ProcessingStage::FlightMatching => "flight_matching",
            ProcessingStage::GraphBuilding => "graph_building",
            ProcessingStage::Completed => "completed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => ProcessingStage::Pending,
            "ocr" => ProcessingStage::OcrExtraction,
            "entity_extraction" => ProcessingStage::EntityExtraction,
            "flight_matching" => ProcessingStage::FlightMatching,
            "graph_building" => ProcessingStage::GraphBuilding,
            "completed" => ProcessingStage::Completed,
            _ => ProcessingStage::Pending,
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            ProcessingStage::Pending => Some(ProcessingStage::OcrExtraction),
            ProcessingStage::OcrExtraction => Some(ProcessingStage::EntityExtraction),
            ProcessingStage::EntityExtraction => Some(ProcessingStage::FlightMatching),
            ProcessingStage::FlightMatching => Some(ProcessingStage::GraphBuilding),
            ProcessingStage::GraphBuilding => Some(ProcessingStage::Completed),
            ProcessingStage::Completed => None,
        }
    }
}

/// Represents a document ingestion job in the queue
#[derive(Debug, Clone)]
pub struct IngestionJob {
    pub id: String,
    pub user_id: String,
    pub source_file_path: String,
    pub source_file_name: String,
    pub source_file_hash: String,
    pub total_pages: i32,
    pub pages_processed: i32,
    pub status: ChunkStatus,
    pub priority: i32,
    pub retry_count: i32,
    pub max_retries: i32,
}

/// The Shredder - Splits PDFs into single-page chunks for processing
pub struct DocumentShredder {
    work_dir: PathBuf,
}

impl DocumentShredder {
    pub fn new(work_dir: PathBuf) -> Result<Self> {
        // Create work directory if it doesn't exist
        fs::create_dir_all(&work_dir).context("Failed to create work directory")?;

        let chunks_dir = work_dir.join("chunks");
        fs::create_dir_all(&chunks_dir).context("Failed to create chunks directory")?;

        Ok(Self { work_dir })
    }

    /// Calculate SHA-256 hash of a file
    fn calculate_file_hash(path: &Path) -> Result<String> {
        let bytes = fs::read(path).context("Failed to read file for hashing")?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    /// Split a PDF into single-page PDFs and return their paths
    pub fn split_pdf(&self, pdf_path: &Path, job_id: &str) -> Result<Vec<(i32, PathBuf, String)>> {
        // Load the PDF
        let doc = PdfDocument::load(pdf_path)
            .context("Failed to load PDF document")?;

        let _page_count = doc.get_pages().len();
        let mut chunks = Vec::new();

        // Create job-specific directory
        let job_chunks_dir = self.work_dir.join("chunks").join(job_id);
        fs::create_dir_all(&job_chunks_dir)?;

        // Split into individual pages
        for (page_num, _page_id) in doc.get_pages().iter().enumerate() {
            let page_number = (page_num + 1) as i32;

            // Create a new PDF with just this page
            let _page_doc = PdfDocument::with_version("1.5");

            // Extract the page from the original document
            // Note: lopdf's API for page extraction can be complex
            // For now, we'll use a simpler approach: convert each page to an image
            // and then OCR it directly, or use pdf-extract for text

            // For production, you might want to use pdf-extract or pdfium-render
            // This is a placeholder for the actual page extraction logic

            let chunk_filename = format!("page_{:04}.pdf", page_number);
            let chunk_path = job_chunks_dir.join(&chunk_filename);

            // Generate hash for this chunk
            let chunk_content = format!("{}-page-{}", job_id, page_number);
            let mut hasher = Sha256::new();
            hasher.update(chunk_content.as_bytes());
            let chunk_hash = format!("{:x}", hasher.finalize());

            // Save the chunk
            // NOTE: This is simplified - in production, you'd need to properly
            // extract and save individual pages from the PDF

            chunks.push((page_number, chunk_path, chunk_hash));
        }

        Ok(chunks)
    }

    /// Extract text from a PDF page using pdf-extract
    pub fn extract_text_from_page(&self, page_path: &Path) -> Result<String> {
        let bytes = fs::read(page_path)?;
        let text = pdf_extract::extract_text_from_mem(&bytes)
            .context("Failed to extract text from PDF page")?;
        Ok(text)
    }

    /// Extract text from entire PDF (fallback method)
    pub fn extract_text_from_pdf(&self, pdf_path: &Path) -> Result<String> {
        let bytes = fs::read(pdf_path)?;
        let text = pdf_extract::extract_text_from_mem(&bytes)
            .context("Failed to extract text from PDF")?;
        Ok(text)
    }
}

/// Persistent ingestion queue with crash recovery
pub struct IngestionQueue {
    conn: Connection,
    shredder: DocumentShredder,
}

impl IngestionQueue {
    pub fn new(db_path: PathBuf, work_dir: PathBuf) -> Result<Self> {
        let conn = Connection::open(&db_path)?;
        let shredder = DocumentShredder::new(work_dir)?;

        Ok(Self { conn, shredder })
    }

    /// Add a new PDF document to the ingestion queue
    pub fn enqueue_document(
        &self,
        user_id: &str,
        pdf_path: &Path,
        document_id: Option<&str>,
        priority: i32,
    ) -> Result<String> {
        let job_id = Uuid::new_v4().to_string();
        let file_name = pdf_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.pdf")
            .to_string();

        // Calculate file hash
        let file_hash = DocumentShredder::calculate_file_hash(pdf_path)?;

        // Check if this file was already processed
        let existing: Option<String> = self.conn
            .query_row(
                "SELECT id FROM document_ingestion_queue WHERE source_file_hash = ?1 AND status = 'completed'",
                params![file_hash],
                |row| row.get(0),
            )
            .optional()?;

        if let Some(existing_id) = existing {
            return Ok(existing_id); // Already processed
        }

        // Count pages in PDF
        let doc = PdfDocument::load(pdf_path)?;
        let total_pages = doc.get_pages().len() as i32;

        // Insert into queue
        self.conn.execute(
            "INSERT INTO document_ingestion_queue
             (id, user_id, source_document_id, source_file_path, source_file_name, source_file_hash, total_pages, status, priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)",
            params![
                job_id,
                user_id,
                document_id,
                pdf_path.to_string_lossy().to_string(),
                file_name,
                file_hash,
                total_pages,
                priority,
            ],
        )?;

        // Split PDF into chunks
        let chunks = self.shredder.split_pdf(pdf_path, &job_id)?;

        // Insert chunks into database
        for (page_num, chunk_path, chunk_hash) in chunks {
            let chunk_id = Uuid::new_v4().to_string();

            self.conn.execute(
                "INSERT INTO document_chunks
                 (id, queue_id, chunk_hash, chunk_number, chunk_type, content_path, status, processing_stage)
                 VALUES (?1, ?2, ?3, ?4, 'page', ?5, 'pending', 'pending')",
                params![
                    chunk_id,
                    job_id,
                    chunk_hash,
                    page_num,
                    chunk_path.to_string_lossy().to_string(),
                ],
            )?;
        }

        Ok(job_id)
    }

    /// Get the next chunk to process
    pub fn get_next_chunk(&self) -> Result<Option<DocumentChunk>> {
        let chunk = self.conn
            .query_row(
                "SELECT id, queue_id, chunk_hash, chunk_number, content_path, status, processing_stage
                 FROM document_chunks
                 WHERE status IN ('pending', 'failed') AND retry_count < 3
                 ORDER BY
                   (SELECT priority FROM document_ingestion_queue WHERE id = queue_id) DESC,
                   created_at ASC
                 LIMIT 1",
                [],
                |row| {
                    Ok(DocumentChunk {
                        id: row.get(0)?,
                        queue_id: row.get(1)?,
                        chunk_hash: row.get(2)?,
                        chunk_number: row.get(3)?,
                        content_path: row.get(4)?,
                        status: ChunkStatus::from_str(&row.get::<_, String>(5)?),
                        processing_stage: ProcessingStage::from_str(&row.get::<_, String>(6)?),
                    })
                },
            )
            .optional()?;

        Ok(chunk)
    }

    /// Update chunk status and stage
    pub fn update_chunk_progress(
        &self,
        chunk_id: &str,
        status: ChunkStatus,
        stage: ProcessingStage,
        error: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE document_chunks
             SET status = ?1, processing_stage = ?2, error_message = ?3, processed_at = datetime('now')
             WHERE id = ?4",
            params![status.as_str(), stage.as_str(), error, chunk_id],
        )?;

        // Update job progress
        self.conn.execute(
            "UPDATE document_ingestion_queue
             SET pages_processed = (
               SELECT COUNT(*) FROM document_chunks
               WHERE queue_id = document_ingestion_queue.id AND status = 'completed'
             )
             WHERE id = (SELECT queue_id FROM document_chunks WHERE id = ?1)",
            params![chunk_id],
        )?;

        Ok(())
    }

    /// Get ingestion job by ID
    pub fn get_job(&self, job_id: &str) -> Result<Option<IngestionJob>> {
        let job = self.conn
            .query_row(
                "SELECT id, user_id, source_file_path, source_file_name, source_file_hash,
                        total_pages, pages_processed, status, priority, retry_count, max_retries
                 FROM document_ingestion_queue
                 WHERE id = ?1",
                params![job_id],
                |row| {
                    Ok(IngestionJob {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        source_file_path: row.get(2)?,
                        source_file_name: row.get(3)?,
                        source_file_hash: row.get(4)?,
                        total_pages: row.get(5)?,
                        pages_processed: row.get(6)?,
                        status: ChunkStatus::from_str(&row.get::<_, String>(7)?),
                        priority: row.get(8)?,
                        retry_count: row.get(9)?,
                        max_retries: row.get(10)?,
                    })
                },
            )
            .optional()?;

        Ok(job)
    }

    /// Recover incomplete jobs on startup (crash recovery)
    pub fn recover_crashed_jobs(&self) -> Result<Vec<String>> {
        let mut recovered = Vec::new();

        // Find chunks that were "processing" but never completed
        let mut stmt = self.conn.prepare(
            "SELECT id FROM document_chunks WHERE status = 'processing'"
        )?;

        let chunk_ids: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        // Reset them to pending for retry
        for chunk_id in chunk_ids {
            self.conn.execute(
                "UPDATE document_chunks
                 SET status = 'pending', retry_count = retry_count + 1
                 WHERE id = ?1",
                params![chunk_id],
            )?;
            recovered.push(chunk_id);
        }

        Ok(recovered)
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> Result<QueueStats> {
        let pending: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM document_chunks WHERE status = 'pending'",
            [],
            |row| row.get(0),
        )?;

        let processing: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM document_chunks WHERE status = 'processing'",
            [],
            |row| row.get(0),
        )?;

        let completed: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM document_chunks WHERE status = 'completed'",
            [],
            |row| row.get(0),
        )?;

        let failed: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM document_chunks WHERE status = 'failed'",
            [],
            |row| row.get(0),
        )?;

        Ok(QueueStats {
            pending,
            processing,
            completed,
            failed,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueueStats {
    pub pending: i32,
    pub processing: i32,
    pub completed: i32,
    pub failed: i32,
}
