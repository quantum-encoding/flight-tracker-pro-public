// Flight Log Extraction Pipeline
// Converts scanned PDF flight logs into structured data using AI vision

pub mod splitter;
pub mod vision_agent;
pub mod aggregator;
pub mod identity_fusion;

pub use splitter::{split_pdf, get_page_count, SplitConfig, SplitResult, ImageFormat};
pub use vision_agent::{VisionAgent, VisionAgentConfig, FlightLogEntry, PageExtractionResult, process_images_concurrent};
pub use aggregator::{aggregate_results, MasterFlightLog, export_to_csv, save_master_log, save_csv_export};
pub use identity_fusion::{IdentityFusion, FusionConfig, PersonEntity, MergeCandidate, MatchType, FusionResult, jaro_winkler_similarity};
