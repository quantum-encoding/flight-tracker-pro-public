use serde::{Deserialize, Serialize};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Thinking,
    Executing,
    Complete,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_name: String,
    pub model: String,
    pub status: AgentStatus,
    pub current_operation: Option<String>,
    pub tokens_input: u32,
    pub tokens_output: u32,
    pub cost_usd: f64,
    pub started_at: String,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub agent_name: String,
    pub model: String,
    pub event_type: String, // "start", "thinking", "executing", "token_update", "complete", "error"
    pub operation: Option<String>,
    pub tokens_input: Option<u32>,
    pub tokens_output: Option<u32>,
    pub cost_usd: Option<f64>,
    pub timestamp: String,
}

/// Model pricing in USD (as of 2024)
pub struct ModelPricing {
    pub input_cost_per_1m: f64,
    pub output_cost_per_1m: f64,
}

impl ModelPricing {
    pub fn get_pricing(model: &str) -> Self {
        match model {
            // Grok models
            "grok-2-1212" => ModelPricing {
                input_cost_per_1m: 2.00,
                output_cost_per_1m: 10.00,
            },
            "grok-beta" => ModelPricing {
                input_cost_per_1m: 5.00,
                output_cost_per_1m: 15.00,
            },
            // DeepSeek models
            "deepseek-chat" => ModelPricing {
                input_cost_per_1m: 0.14,
                output_cost_per_1m: 0.28,
            },
            "deepseek-reasoner" => ModelPricing {
                input_cost_per_1m: 0.55,
                output_cost_per_1m: 2.19,
            },
            // Gemini models
            "gemini-3-pro-preview" => ModelPricing {
                input_cost_per_1m: 2.00, // <200k tokens tier
                output_cost_per_1m: 12.00,
            },
            "gemini-3-pro-image-preview" => ModelPricing {
                input_cost_per_1m: 2.00,
                output_cost_per_1m: 0.134, // Image output varies by resolution
            },
            "gemini-2.5-flash-lite" => ModelPricing {
                input_cost_per_1m: 0.10,
                output_cost_per_1m: 0.40,
            },
            "gemini-2.0-flash-exp" | "gemini-exp-1206" => ModelPricing {
                input_cost_per_1m: 0.00, // Free tier
                output_cost_per_1m: 0.00,
            },
            "gemini-1.5-pro" => ModelPricing {
                input_cost_per_1m: 1.25,
                output_cost_per_1m: 5.00,
            },
            "gemini-1.5-flash" => ModelPricing {
                input_cost_per_1m: 0.075,
                output_cost_per_1m: 0.30,
            },
            // Claude models (via Anthropic API)
            "claude-3-opus-20240229" => ModelPricing {
                input_cost_per_1m: 15.00,
                output_cost_per_1m: 75.00,
            },
            "claude-3-sonnet-20240229" => ModelPricing {
                input_cost_per_1m: 3.00,
                output_cost_per_1m: 15.00,
            },
            "claude-3-haiku-20240307" => ModelPricing {
                input_cost_per_1m: 0.25,
                output_cost_per_1m: 1.25,
            },
            // Default fallback
            _ => ModelPricing {
                input_cost_per_1m: 1.00,
                output_cost_per_1m: 3.00,
            },
        }
    }

    pub fn calculate_cost(&self, tokens_input: u32, tokens_output: u32) -> f64 {
        let input_cost = (tokens_input as f64 / 1_000_000.0) * self.input_cost_per_1m;
        let output_cost = (tokens_output as f64 / 1_000_000.0) * self.output_cost_per_1m;
        input_cost + output_cost
    }
}

/// Emit agent lifecycle event
pub fn emit_agent_event<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    event: AgentEvent,
) {
    if let Err(e) = app_handle.emit("agent:status", event) {
        eprintln!("Failed to emit agent event: {}", e);
    }
}

/// Helper to emit agent start event
pub fn emit_agent_start<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    agent_name: &str,
    model: &str,
    operation: &str,
) {
    let event = AgentEvent {
        agent_name: agent_name.to_string(),
        model: model.to_string(),
        event_type: "start".to_string(),
        operation: Some(operation.to_string()),
        tokens_input: None,
        tokens_output: None,
        cost_usd: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    emit_agent_event(app_handle, event);
}

/// Helper to emit agent thinking event
pub fn emit_agent_thinking<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    agent_name: &str,
    model: &str,
) {
    let event = AgentEvent {
        agent_name: agent_name.to_string(),
        model: model.to_string(),
        event_type: "thinking".to_string(),
        operation: None,
        tokens_input: None,
        tokens_output: None,
        cost_usd: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    emit_agent_event(app_handle, event);
}

/// Helper to emit agent executing event
pub fn emit_agent_executing<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    agent_name: &str,
    model: &str,
    tool_name: &str,
) {
    let event = AgentEvent {
        agent_name: agent_name.to_string(),
        model: model.to_string(),
        event_type: "executing".to_string(),
        operation: Some(format!("Executing: {}", tool_name)),
        tokens_input: None,
        tokens_output: None,
        cost_usd: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    emit_agent_event(app_handle, event);
}

/// Helper to emit token update event
pub fn emit_agent_token_update<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    agent_name: &str,
    model: &str,
    tokens_input: u32,
    tokens_output: u32,
) {
    let pricing = ModelPricing::get_pricing(model);
    let cost = pricing.calculate_cost(tokens_input, tokens_output);

    let event = AgentEvent {
        agent_name: agent_name.to_string(),
        model: model.to_string(),
        event_type: "token_update".to_string(),
        operation: None,
        tokens_input: Some(tokens_input),
        tokens_output: Some(tokens_output),
        cost_usd: Some(cost),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    emit_agent_event(app_handle, event);
}

/// Helper to emit agent complete event
pub fn emit_agent_complete<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    agent_name: &str,
    model: &str,
    tokens_input: u32,
    tokens_output: u32,
) {
    let pricing = ModelPricing::get_pricing(model);
    let cost = pricing.calculate_cost(tokens_input, tokens_output);

    let event = AgentEvent {
        agent_name: agent_name.to_string(),
        model: model.to_string(),
        event_type: "complete".to_string(),
        operation: None,
        tokens_input: Some(tokens_input),
        tokens_output: Some(tokens_output),
        cost_usd: Some(cost),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    emit_agent_event(app_handle, event);
}

/// Helper to emit agent error event
pub fn emit_agent_error<R: tauri::Runtime>(
    app_handle: &tauri::AppHandle<R>,
    agent_name: &str,
    model: &str,
    error_msg: &str,
) {
    let event = AgentEvent {
        agent_name: agent_name.to_string(),
        model: model.to_string(),
        event_type: "error".to_string(),
        operation: Some(format!("Error: {}", error_msg)),
        tokens_input: None,
        tokens_output: None,
        cost_usd: None,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    emit_agent_event(app_handle, event);
}
