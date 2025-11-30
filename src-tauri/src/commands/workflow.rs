use crate::workflow::{Workflow, WorkflowManager, Checkpoint, CheckpointManager, generate_workflow_from_prompt};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// Application state for workflow management
pub struct WorkflowState {
    pub manager: Arc<RwLock<WorkflowManager>>,
}

impl WorkflowState {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            manager: Arc::new(RwLock::new(WorkflowManager::new(Some(Arc::new(app))))),
        }
    }
}

/// Tauri command to validate a workflow DAG
#[tauri::command]
pub async fn validate_workflow(workflow: Workflow) -> Result<String, String> {
    workflow
        .validate_dag()
        .map(|_| "Workflow is a valid DAG".to_string())
        .map_err(|e| e.to_string())
}

/// Tauri command to get the execution order for a workflow
#[tauri::command]
pub async fn get_execution_order(workflow: Workflow) -> Result<Vec<String>, String> {
    workflow.get_execution_order().map_err(|e| e.to_string())
}

/// Tauri command to execute a workflow
#[tauri::command]
pub async fn execute_workflow(
    workflow: Workflow,
    state: State<'_, WorkflowState>,
) -> Result<String, String> {
    let manager = state.manager.read().await;
    manager
        .start_workflow(workflow)
        .await
        .map_err(|e| e.to_string())
}

/// Tauri command to check if a workflow is running
#[tauri::command]
pub async fn is_workflow_running(
    workflow_id: String,
    state: State<'_, WorkflowState>,
) -> Result<bool, String> {
    let manager = state.manager.read().await;
    Ok(manager.is_running(&workflow_id).await)
}

/// Tauri command to cancel a running workflow
#[tauri::command]
pub async fn cancel_workflow(
    workflow_id: String,
    state: State<'_, WorkflowState>,
) -> Result<bool, String> {
    let manager = state.manager.read().await;
    Ok(manager.cancel_workflow(&workflow_id).await)
}

/// Tauri command to export workflow to JSON file
#[tauri::command]
pub async fn export_workflow(workflow: Workflow, path: String) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&workflow)
        .map_err(|e| format!("Failed to serialize workflow: {}", e))?;

    tokio::fs::write(&path, json)
        .await
        .map_err(|e| format!("Failed to write file {}: {}", path, e))
}

/// Tauri command to import workflow from JSON file
#[tauri::command]
pub async fn import_workflow(path: String) -> Result<Workflow, String> {
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read file {}: {}", path, e))?;

    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse workflow: {}", e))
}

/// Tauri command to initialize git checkpoint for a workflow
#[tauri::command]
pub async fn init_workflow_checkpoint(workflow_id: String) -> Result<String, String> {
    let manager = CheckpointManager::init(&workflow_id)
        .map_err(|e| format!("Failed to initialize checkpoint: {}", e))?;

    Ok(manager.repo_path().display().to_string())
}

/// Tauri command to create a checkpoint
#[tauri::command]
pub async fn create_checkpoint(
    workflow_id: String,
    message: String,
    data: String,
) -> Result<Checkpoint, String> {
    let manager = CheckpointManager::init(&workflow_id)
        .map_err(|e| format!("Failed to initialize checkpoint manager: {}", e))?;

    manager
        .create_checkpoint(&message, &data)
        .map_err(|e| format!("Failed to create checkpoint: {}", e))
}

/// Tauri command to get checkpoint history
#[tauri::command]
pub async fn get_checkpoint_history(workflow_id: String) -> Result<Vec<Checkpoint>, String> {
    let manager = CheckpointManager::init(&workflow_id)
        .map_err(|e| format!("Failed to initialize checkpoint manager: {}", e))?;

    manager
        .get_history()
        .map_err(|e| format!("Failed to get checkpoint history: {}", e))
}

/// Tauri command to get state at a specific checkpoint
#[tauri::command]
pub async fn get_checkpoint_state(
    workflow_id: String,
    commit_hash: String,
) -> Result<String, String> {
    let manager = CheckpointManager::init(&workflow_id)
        .map_err(|e| format!("Failed to initialize checkpoint manager: {}", e))?;

    manager
        .get_state_at_checkpoint(&commit_hash)
        .map_err(|e| format!("Failed to get checkpoint state: {}", e))
}

/// Tauri command to generate a workflow from natural language using AI
#[tauri::command]
pub async fn generate_workflow_ai(
    prompt: String,
    provider: String,
) -> Result<Workflow, String> {
    // Get API key for the provider
    let api_key = match provider.as_str() {
        "gemini" | "google" => std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .or_else(|_| std::env::var("GENAI_API_KEY"))
            .map_err(|_| "GEMINI_API_KEY not set. Please set environment variable.".to_string())?,
        "deepseek" => std::env::var("DEEPSEEK_API_KEY")
            .map_err(|_| "DEEPSEEK_API_KEY not set. Please set environment variable.".to_string())?,
        "grok" | "xai" => std::env::var("XAI_API_KEY")
            .map_err(|_| "XAI_API_KEY not set. Please set environment variable.".to_string())?,
        _ => return Err(format!("Unsupported provider: {}. Use 'gemini', 'deepseek', or 'grok'", provider)),
    };

    // Generate the workflow
    generate_workflow_from_prompt(&prompt, &provider, &api_key)
        .await
        .map_err(|e| format!("AI workflow generation failed: {}", e))
}
