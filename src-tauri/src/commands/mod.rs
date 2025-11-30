// Modular command organization for Flight Tracker Pro
// This module structure improves maintainability by grouping related commands

use crate::database::Database;
use std::sync::Mutex;
use tauri::State;

// Shared application state
pub struct AppState {
    pub db: Mutex<Database>,
}

// Module declarations
pub mod calculations;
pub mod settings;
pub mod users;
pub mod flights;
pub mod csv_import;
pub mod statistics;
pub mod ocr;
pub mod data_management;
pub mod investigations;
pub mod passengers;
pub mod research;
pub mod journeys;
pub mod pilot_logbook;
pub mod ffp;
pub mod airports;
pub mod airport_enrichment;
pub mod aircraft;
pub mod documents;
pub mod fuel_prices;
pub mod fuel_commands;
pub mod analytics;
pub mod network_scanner;
pub mod network_sentinel;
pub mod agent_memory_commands;
pub mod doc_ingestion_commands;
pub mod custom_schema_commands;
pub mod self_improvement;
pub mod workflow;
pub mod identity_fusion;
pub mod data_editor;
pub mod media_gallery;
pub mod batch_calculations;
pub mod active_defense;
pub mod network_tools;
pub mod donation;
pub mod ai_models;

// Re-export all commands for easy registration
pub use calculations::*;
pub use settings::*;
pub use users::*;
pub use flights::*;
pub use csv_import::*;
pub use statistics::*;
pub use ocr::*;
pub use data_management::*;
pub use investigations::*;
pub use passengers::*;
pub use research::*;
pub use journeys::*;
pub use pilot_logbook::*;
pub use ffp::*;
pub use airports::*;
pub use airport_enrichment::*;
pub use aircraft::*;
pub use documents::*;
pub use fuel_prices::*;
pub use fuel_commands::*;
pub use analytics::*;
pub use network_scanner::*;
pub use network_sentinel::*;
pub use agent_memory_commands::*;
pub use doc_ingestion_commands::*;
pub use custom_schema_commands::*;
pub use self_improvement::*;
pub use workflow::*;
pub use identity_fusion::*;
pub use data_editor::*;
pub use media_gallery::*;
pub use batch_calculations::*;
pub use active_defense::*;
pub use network_tools::*;
pub use donation::*;
pub use ai_models::*;

// ===== INITIALIZATION COMMAND =====

#[tauri::command]
pub fn initialize_app(state: State<'_, AppState>) -> Result<bool, String> {
    // Check if a user exists
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let user = db.get_primary_user().map_err(|e| e.to_string())?;

    Ok(user.is_some())
}
