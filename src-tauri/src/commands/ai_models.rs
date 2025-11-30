use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use tauri::Manager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub provider: String,
    pub model: String,
    pub input_cost_per_1m: f64,
    pub output_cost_per_1m: f64,
    pub cache_write_cost_per_1m: f64,
    pub cache_read_cost_per_1m: f64,
    pub websearch_cost_per_query: f64,
    pub requests_per_minute: i32,
    pub tokens_per_minute: i64,
    pub context_window: i64,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModels {
    pub provider: String,
    pub display_name: String,
    pub models: Vec<ModelInfo>,
}

/// Load all AI models from the CSV resource file
#[tauri::command]
pub fn get_ai_models(app_handle: tauri::AppHandle) -> Result<Vec<ProviderModels>, String> {
    // Try multiple paths for the CSV file
    let possible_paths = vec![
        // Production: bundled resources
        app_handle
            .path()
            .resource_dir()
            .ok()
            .map(|p| p.join("resources").join("model_costs.csv")),
        // Development: source directory
        Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("model_costs.csv")),
    ];

    let mut file_result = None;
    let mut last_path = String::new();

    for path_opt in possible_paths.into_iter().flatten() {
        last_path = path_opt.to_string_lossy().to_string();
        if let Ok(f) = File::open(&path_opt) {
            file_result = Some(f);
            break;
        }
    }

    let file = file_result.ok_or_else(|| {
        format!("Failed to open model_costs.csv. Last tried path: {}", last_path)
    })?;

    let reader = BufReader::new(file);
    let mut csv_reader = csv::Reader::from_reader(reader);

    let mut models: Vec<ModelInfo> = Vec::new();

    for result in csv_reader.deserialize() {
        let record: ModelInfo = result.map_err(|e| format!("Failed to parse CSV row: {}", e))?;
        models.push(record);
    }

    // Group by provider
    let mut provider_map: std::collections::HashMap<String, Vec<ModelInfo>> = std::collections::HashMap::new();

    for model in models {
        provider_map
            .entry(model.provider.clone())
            .or_insert_with(Vec::new)
            .push(model);
    }

    // Convert to ProviderModels format with display names
    let provider_display_names: std::collections::HashMap<&str, &str> = [
        ("anthropic", "Anthropic (Claude)"),
        ("google", "Google (Gemini)"),
        ("deepseek", "DeepSeek"),
        ("xai", "xAI (Grok)"),
    ].iter().cloned().collect();

    let mut result: Vec<ProviderModels> = provider_map
        .into_iter()
        .map(|(provider, models)| {
            let display_name = provider_display_names
                .get(provider.as_str())
                .unwrap_or(&provider.as_str())
                .to_string();
            ProviderModels {
                provider: provider.clone(),
                display_name,
                models,
            }
        })
        .collect();

    // Sort providers in a logical order
    let provider_order = ["anthropic", "google", "deepseek", "xai"];
    result.sort_by(|a, b| {
        let a_idx = provider_order.iter().position(|&p| p == a.provider).unwrap_or(99);
        let b_idx = provider_order.iter().position(|&p| p == b.provider).unwrap_or(99);
        a_idx.cmp(&b_idx)
    });

    Ok(result)
}

/// Get models for a specific provider
#[tauri::command]
pub fn get_models_by_provider(
    app_handle: tauri::AppHandle,
    provider: String,
) -> Result<Vec<ModelInfo>, String> {
    let all_models = get_ai_models(app_handle)?;

    let provider_models = all_models
        .into_iter()
        .find(|p| p.provider == provider)
        .map(|p| p.models)
        .unwrap_or_default();

    Ok(provider_models)
}
