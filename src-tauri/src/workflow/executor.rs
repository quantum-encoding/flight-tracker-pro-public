use super::models::{
    Edge, Node, NodeExecutionResult, NodeType, Workflow,
};
use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Invalid workflow: {0}")]
    InvalidWorkflow(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Missing configuration: {0}")]
    MissingConfig(String),

    #[error("Shell command failed: {0}")]
    ShellError(String),

    #[error("AI API error: {0}")]
    AIError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}

pub type ExecutionResult<T> = Result<T, ExecutionError>;

/// Trait for executing different node types
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>>;
}

/// Executor for Shell nodes
pub struct ShellExecutor;

#[async_trait]
impl NodeExecutor for ShellExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let command = node
            .interpolate_config("cmd", context)
            .or_else(|| node.interpolate_config("command", context))
            .ok_or_else(|| ExecutionError::MissingConfig("cmd or command".to_string()))?;

        tracing::info!("Executing shell command: {}", command);

        let output = if cfg!(target_os = "windows") {
            tokio::process::Command::new("cmd")
                .args(["/C", &command])
                .output()
                .await?
        } else {
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output()
                .await?
        };

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            Ok(HashMap::from([
                ("stdout".to_string(), json!(stdout)),
                ("stderr".to_string(), json!(stderr)),
                ("exit_code".to_string(), json!(0)),
            ]))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(ExecutionError::ShellError(format!(
                "Command failed with exit code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr
            )))
        }
    }
}

/// Executor for AI Prompt nodes
pub struct AiPromptExecutor;

#[async_trait]
impl NodeExecutor for AiPromptExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let prompt = node
            .interpolate_config("prompt", context)
            .ok_or_else(|| ExecutionError::MissingConfig("prompt".to_string()))?;

        let model = node
            .get_config("model")
            .cloned()
            .unwrap_or_else(|| "gemini-2.5-flash".to_string());

        let provider = node
            .get_config("provider")
            .cloned()
            .unwrap_or_else(|| "google".to_string());

        tracing::info!(
            "Executing AI prompt with provider: {}, model: {}",
            provider,
            model
        );

        // Emit agent start event
        if let Some(app_handle) = _app.as_ref() {
            crate::agent_tracking::emit_agent_start(
                app_handle,
                &format!("AI-{}", provider),
                &model,
                "AI Prompt Execution"
            );
        }

        // Get API key from environment (check multiple possible variable names)
        let api_key = match provider.as_str() {
            "google" | "gemini" => std::env::var("GEMINI_API_KEY")
                .or_else(|_| std::env::var("GOOGLE_API_KEY"))
                .or_else(|_| std::env::var("GENAI_API_KEY"))
                .unwrap_or_else(|_| String::new()),
            "deepseek" => std::env::var("DEEPSEEK_API_KEY")
                .unwrap_or_else(|_| String::new()),
            "grok" | "xai" => std::env::var("XAI_API_KEY")
                .unwrap_or_else(|_| String::new()),
            _ => String::new(),
        };

        if api_key.is_empty() {
            return Err(ExecutionError::ExecutionFailed(format!(
                "API key not found for provider: {}. Set environment variable.",
                provider
            )));
        }

        // Call the appropriate AI provider
        let result = match provider.as_str() {
            "google" | "gemini" => {
                let result = crate::gemini::chat_with_gemini(&prompt, &api_key, &model)
                    .await
                    .map_err(|e| {
                        if let Some(app_handle) = _app.as_ref() {
                            crate::agent_tracking::emit_agent_error(
                                app_handle,
                                &format!("AI-{}", provider),
                                &model,
                                &format!("Gemini API error: {}", e)
                            );
                        }
                        ExecutionError::ExecutionFailed(format!("Gemini API error: {}", e))
                    })?;

                let tokens_total = result.tokens_used.unwrap_or(0);
                let tokens_input = tokens_total / 2;
                let tokens_output = tokens_total - tokens_input;

                if let Some(app_handle) = _app.as_ref() {
                    crate::agent_tracking::emit_agent_complete(
                        app_handle,
                        &format!("AI-{}", provider),
                        &model,
                        tokens_input,
                        tokens_output
                    );
                }

                Ok(HashMap::from([
                    ("response".to_string(), json!(result.content)),
                    ("model".to_string(), json!(model)),
                    ("provider".to_string(), json!("gemini")),
                    ("tokens_used".to_string(), json!(result.tokens_used.unwrap_or(0))),
                ]))
            }
            "deepseek" => {
                let result = crate::deepseek::chat_with_deepseek(&prompt, &api_key)
                    .await
                    .map_err(|e| {
                        if let Some(app_handle) = _app.as_ref() {
                            crate::agent_tracking::emit_agent_error(
                                app_handle,
                                &format!("AI-{}", provider),
                                &model,
                                &format!("DeepSeek API error: {}", e)
                            );
                        }
                        ExecutionError::ExecutionFailed(format!("DeepSeek API error: {}", e))
                    })?;

                let tokens_total = result.tokens_used.unwrap_or(0);
                let tokens_input = tokens_total / 2;
                let tokens_output = tokens_total - tokens_input;

                if let Some(app_handle) = _app.as_ref() {
                    crate::agent_tracking::emit_agent_complete(
                        app_handle,
                        &format!("AI-{}", provider),
                        "deepseek-chat",
                        tokens_input,
                        tokens_output
                    );
                }

                Ok(HashMap::from([
                    ("response".to_string(), json!(result.content)),
                    ("model".to_string(), json!("deepseek-chat")),
                    ("provider".to_string(), json!("deepseek")),
                    ("tokens_used".to_string(), json!(result.tokens_used.unwrap_or(0))),
                ]))
            }
            "grok" | "xai" => {
                let grok_model = if model.is_empty() || model == "gemini-2.5-flash" {
                    "grok-3-fast".to_string()
                } else {
                    model.clone()
                };

                let result = crate::grok::chat_with_grok(&prompt, &api_key, &grok_model)
                    .await
                    .map_err(|e| {
                        if let Some(app_handle) = _app.as_ref() {
                            crate::agent_tracking::emit_agent_error(
                                app_handle,
                                &format!("AI-{}", provider),
                                &grok_model,
                                &format!("Grok API error: {}", e)
                            );
                        }
                        ExecutionError::ExecutionFailed(format!("Grok API error: {}", e))
                    })?;

                if let Some(app_handle) = _app.as_ref() {
                    crate::agent_tracking::emit_agent_complete(
                        app_handle,
                        &format!("AI-{}", provider),
                        &grok_model,
                        0,
                        0
                    );
                }

                Ok(HashMap::from([
                    ("response".to_string(), json!(result.summary)),
                    ("model".to_string(), json!(grok_model)),
                    ("provider".to_string(), json!("grok")),
                    ("tokens_used".to_string(), json!(0)),
                ]))
            }
            _ => {
                if let Some(app_handle) = _app.as_ref() {
                    crate::agent_tracking::emit_agent_error(
                        app_handle,
                        &format!("AI-{}", provider),
                        &model,
                        &format!("Unsupported provider: {}", provider)
                    );
                }

                Err(ExecutionError::ExecutionFailed(format!(
                    "Unsupported AI provider: {}. Use 'google', 'gemini', 'deepseek', or 'grok'",
                    provider
                )))
            }
        };

        result
    }
}

/// Executor for Database nodes
pub struct DatabaseExecutor;

#[async_trait]
impl NodeExecutor for DatabaseExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let query = node
            .interpolate_config("query", context)
            .ok_or_else(|| ExecutionError::MissingConfig("query".to_string()))?;

        let db_type = node
            .get_config("db_type")
            .cloned()
            .unwrap_or_else(|| "sqlite".to_string());

        tracing::info!("Executing database query on {}: {}", db_type, query);

        // Note: Actual database operations would be implemented here
        // For now, returning a mock response
        // In production, this would connect to actual databases

        Ok(HashMap::from([
            ("rows_affected".to_string(), json!(0)),
            ("result".to_string(), json!([])),
            ("db_type".to_string(), json!(db_type)),
        ]))
    }
}

/// Executor for Trade Agent nodes
pub struct TradeAgentExecutor;

#[async_trait]
impl NodeExecutor for TradeAgentExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let strategy = node
            .get_config("strategy")
            .ok_or_else(|| ExecutionError::MissingConfig("strategy".to_string()))?;

        let symbol = node
            .interpolate_config("symbol", context)
            .or_else(|| node.interpolate_config("asset", context))
            .ok_or_else(|| ExecutionError::MissingConfig("symbol or asset".to_string()))?;

        tracing::info!(
            "Executing trade agent with strategy: {} for symbol: {}",
            strategy,
            symbol
        );

        // Note: Actual trading logic would be implemented here
        // For now, returning a mock response

        Ok(HashMap::from([
            ("strategy".to_string(), json!(strategy)),
            ("symbol".to_string(), json!(symbol)),
            ("action".to_string(), json!("hold")),
            ("confidence".to_string(), json!(0.5)),
        ]))
    }
}

/// Executor for HTTP Request nodes
pub struct HttpRequestExecutor;

#[async_trait]
impl NodeExecutor for HttpRequestExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let url = node
            .interpolate_config("url", context)
            .ok_or_else(|| ExecutionError::MissingConfig("url".to_string()))?;

        let method = node
            .get_config("method")
            .cloned()
            .unwrap_or_else(|| "GET".to_string());

        let body = node.interpolate_config("body", context);

        tracing::info!("Executing HTTP {} request to {}", method, url);

        let client = reqwest::Client::new();
        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "DELETE" => client.delete(&url),
            _ => return Err(ExecutionError::ExecutionFailed(format!("Unsupported HTTP method: {}", method))),
        };

        if let Some(body_content) = body {
            request = request.body(body_content);
        }

        let response = request.send().await?;
        let status = response.status().as_u16();
        let text = response.text().await?;

        Ok(HashMap::from([
            ("status".to_string(), json!(status)),
            ("body".to_string(), json!(text)),
            ("success".to_string(), json!((200..300).contains(&status))),
        ]))
    }
}

/// Executor for File Read nodes
pub struct FileReadExecutor;

#[async_trait]
impl NodeExecutor for FileReadExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let path = node
            .interpolate_config("path", context)
            .ok_or_else(|| ExecutionError::MissingConfig("path".to_string()))?;

        tracing::info!("Reading file: {}", path);

        let content = tokio::fs::read_to_string(&path).await?;

        Ok(HashMap::from([
            ("content".to_string(), json!(content)),
            ("path".to_string(), json!(path)),
            ("size".to_string(), json!(content.len())),
        ]))
    }
}

/// Executor for File Write nodes
pub struct FileWriteExecutor;

#[async_trait]
impl NodeExecutor for FileWriteExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let path = node
            .interpolate_config("path", context)
            .ok_or_else(|| ExecutionError::MissingConfig("path".to_string()))?;

        let content = node
            .interpolate_config("content", context)
            .ok_or_else(|| ExecutionError::MissingConfig("content".to_string()))?;

        tracing::info!("Writing file: {}", path);

        tokio::fs::write(&path, &content).await?;

        Ok(HashMap::from([
            ("path".to_string(), json!(path)),
            ("bytes_written".to_string(), json!(content.len())),
            ("success".to_string(), json!(true)),
        ]))
    }
}

/// Executor for Transform nodes (data transformation)
pub struct TransformExecutor;

#[async_trait]
impl NodeExecutor for TransformExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let operation = node
            .get_config("operation")
            .ok_or_else(|| ExecutionError::MissingConfig("operation".to_string()))?;

        let input = node.interpolate_config("input", context).unwrap_or_default();

        tracing::info!("Transforming data with operation: {}", operation);

        let result = match operation.as_str() {
            "uppercase" => input.to_uppercase(),
            "lowercase" => input.to_lowercase(),
            "trim" => input.trim().to_string(),
            "json_parse" => {
                // Try to parse as JSON
                match serde_json::from_str::<serde_json::Value>(&input) {
                    Ok(val) => val.to_string(),
                    Err(e) => return Err(ExecutionError::ExecutionFailed(format!("JSON parse error: {}", e))),
                }
            }
            _ => input,
        };

        Ok(HashMap::from([
            ("result".to_string(), json!(result)),
            ("operation".to_string(), json!(operation)),
        ]))
    }
}

/// Executor for Filter nodes
pub struct FilterExecutor;

#[async_trait]
impl NodeExecutor for FilterExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let condition = node
            .interpolate_config("condition", context)
            .ok_or_else(|| ExecutionError::MissingConfig("condition".to_string()))?;

        tracing::info!("Evaluating filter condition: {}", condition);

        // Simple condition evaluation (can be extended)
        let passes = !condition.is_empty() && condition != "false" && condition != "0";

        Ok(HashMap::from([
            ("passes".to_string(), json!(passes)),
            ("condition".to_string(), json!(condition)),
        ]))
    }
}

/// Executor for Conditional nodes (if/else branching)
pub struct ConditionalExecutor;

#[async_trait]
impl NodeExecutor for ConditionalExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let condition = node
            .interpolate_config("condition", context)
            .ok_or_else(|| ExecutionError::MissingConfig("condition".to_string()))?;

        tracing::info!("Evaluating condition: {}", condition);

        // Evaluate condition - supports simple comparisons
        let result = evaluate_condition(&condition, context);

        Ok(HashMap::from([
            ("result".to_string(), json!(result)),
            ("condition".to_string(), json!(condition)),
            ("branch".to_string(), json!(if result { "true" } else { "false" })),
        ]))
    }
}

/// Simple condition evaluator
fn evaluate_condition(condition: &str, context: &HashMap<String, serde_json::Value>) -> bool {
    let condition = condition.trim();

    // Handle boolean literals
    if condition == "true" || condition == "1" {
        return true;
    }
    if condition == "false" || condition == "0" || condition.is_empty() {
        return false;
    }

    // Handle comparisons (==, !=, >, <, >=, <=)
    if let Some((left, right)) = condition.split_once("==") {
        let left_val = resolve_value(left.trim(), context);
        let right_val = resolve_value(right.trim(), context);
        return left_val == right_val;
    }
    if let Some((left, right)) = condition.split_once("!=") {
        let left_val = resolve_value(left.trim(), context);
        let right_val = resolve_value(right.trim(), context);
        return left_val != right_val;
    }
    if let Some((left, right)) = condition.split_once(">=") {
        let left_val = resolve_value(left.trim(), context);
        let right_val = resolve_value(right.trim(), context);
        if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
            return l >= r;
        }
    }
    if let Some((left, right)) = condition.split_once("<=") {
        let left_val = resolve_value(left.trim(), context);
        let right_val = resolve_value(right.trim(), context);
        if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
            return l <= r;
        }
    }
    if let Some((left, right)) = condition.split_once('>') {
        let left_val = resolve_value(left.trim(), context);
        let right_val = resolve_value(right.trim(), context);
        if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
            return l > r;
        }
    }
    if let Some((left, right)) = condition.split_once('<') {
        let left_val = resolve_value(left.trim(), context);
        let right_val = resolve_value(right.trim(), context);
        if let (Ok(l), Ok(r)) = (left_val.parse::<f64>(), right_val.parse::<f64>()) {
            return l < r;
        }
    }

    // Handle contains check
    if condition.contains(".contains(") {
        // Pattern: variable.contains("value")
        if let Some(start) = condition.find(".contains(") {
            let var_name = &condition[..start];
            let rest = &condition[start + 10..];
            if let Some(end) = rest.find(')') {
                let search_val = rest[..end].trim_matches(|c| c == '"' || c == '\'');
                let var_val = resolve_value(var_name, context);
                return var_val.contains(search_val);
            }
        }
    }

    // Default: non-empty string is truthy
    !condition.is_empty()
}

/// Resolve a value from context or return as literal
fn resolve_value(val: &str, context: &HashMap<String, serde_json::Value>) -> String {
    let val = val.trim().trim_matches(|c| c == '"' || c == '\'');

    // Check if it's a context reference
    if let Some(ctx_val) = context.get(val) {
        match ctx_val {
            serde_json::Value::String(s) => s.clone(),
            v => v.to_string(),
        }
    } else {
        val.to_string()
    }
}

/// Executor for Loop nodes (iteration over data)
pub struct LoopExecutor;

#[async_trait]
impl NodeExecutor for LoopExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let iterations = node
            .interpolate_config("iterations", context)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);

        let input_array = node
            .interpolate_config("input", context)
            .and_then(|s| serde_json::from_str::<Vec<serde_json::Value>>(&s).ok())
            .unwrap_or_default();

        tracing::info!("Loop node: {} iterations or {} array items", iterations, input_array.len());

        // If we have an input array, use its length
        let actual_iterations = if !input_array.is_empty() {
            input_array.len() as u32
        } else {
            iterations
        };

        Ok(HashMap::from([
            ("iterations".to_string(), json!(actual_iterations)),
            ("items".to_string(), json!(input_array)),
            ("current_index".to_string(), json!(0)),
            ("completed".to_string(), json!(true)),
        ]))
    }
}

/// Executor for Aggregator nodes (waits for multiple inputs and combines them)
pub struct AggregatorExecutor;

#[async_trait]
impl NodeExecutor for AggregatorExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        tracing::info!("Aggregating inputs for node: {}", node.id);

        // Collect all inputs from parent nodes
        let mut inputs = Vec::new();
        for (key, value) in context.iter() {
            if !key.starts_with(&node.id) {
                inputs.push(value.clone());
            }
        }

        Ok(HashMap::from([
            ("inputs".to_string(), json!(inputs)),
            ("count".to_string(), json!(inputs.len())),
            ("aggregated".to_string(), json!(true)),
        ]))
    }
}

/// Executor for Merge nodes (combines specific fields)
pub struct MergeExecutor;

#[async_trait]
impl NodeExecutor for MergeExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        tracing::info!("Merging data for node: {}", node.id);

        let mut merged = serde_json::Map::new();

        // Merge all context values into a single object
        for (key, value) in context.iter() {
            if !key.starts_with(&node.id) {
                merged.insert(key.clone(), value.clone());
            }
        }

        Ok(HashMap::from([
            ("merged".to_string(), json!(merged)),
            ("fields".to_string(), json!(merged.len())),
        ]))
    }
}

/// Executor for Log nodes
pub struct LogExecutor;

#[async_trait]
impl NodeExecutor for LogExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let message = node
            .interpolate_config("message", context)
            .unwrap_or_else(|| "No message".to_string());

        let level = node
            .get_config("level")
            .cloned()
            .unwrap_or_else(|| "info".to_string());

        match level.as_str() {
            "error" => tracing::error!("{}", message),
            "warn" => tracing::warn!("{}", message),
            "debug" => tracing::debug!("{}", message),
            _ => tracing::info!("{}", message),
        }

        Ok(HashMap::from([
            ("message".to_string(), json!(message)),
            ("level".to_string(), json!(level)),
            ("logged".to_string(), json!(true)),
        ]))
    }
}

/// Executor for Notify nodes
pub struct NotifyExecutor;

#[async_trait]
impl NodeExecutor for NotifyExecutor {
    async fn execute(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
        _app: Option<Arc<AppHandle>>,
    ) -> ExecutionResult<HashMap<String, serde_json::Value>> {
        let title = node
            .interpolate_config("title", context)
            .unwrap_or_else(|| "Notification".to_string());

        let message = node
            .interpolate_config("message", context)
            .unwrap_or_else(|| "No message".to_string());

        tracing::info!("Notification: {} - {}", title, message);

        // In a real implementation, this would send a system notification
        Ok(HashMap::from([
            ("title".to_string(), json!(title)),
            ("message".to_string(), json!(message)),
            ("sent".to_string(), json!(true)),
        ]))
    }
}

/// Main workflow executor that orchestrates node execution
pub struct WorkflowExecutor {
    executors: HashMap<NodeType, Arc<dyn NodeExecutor>>,
    app: Option<Arc<AppHandle>>,
}

impl WorkflowExecutor {
    /// Create a new workflow executor
    pub fn new(app: Option<Arc<AppHandle>>) -> Self {
        let mut executors: HashMap<NodeType, Arc<dyn NodeExecutor>> = HashMap::new();

        // Execution nodes
        executors.insert(NodeType::Shell, Arc::new(ShellExecutor));
        executors.insert(NodeType::AiPrompt, Arc::new(AiPromptExecutor));
        executors.insert(NodeType::Database, Arc::new(DatabaseExecutor));
        executors.insert(NodeType::TradeAgent, Arc::new(TradeAgentExecutor));

        // Data operations
        executors.insert(NodeType::HttpRequest, Arc::new(HttpRequestExecutor));
        executors.insert(NodeType::FileRead, Arc::new(FileReadExecutor));
        executors.insert(NodeType::FileWrite, Arc::new(FileWriteExecutor));
        executors.insert(NodeType::Transform, Arc::new(TransformExecutor));
        executors.insert(NodeType::Filter, Arc::new(FilterExecutor));

        // Control flow
        executors.insert(NodeType::Conditional, Arc::new(ConditionalExecutor));
        executors.insert(NodeType::Loop, Arc::new(LoopExecutor));
        executors.insert(NodeType::Aggregator, Arc::new(AggregatorExecutor));
        executors.insert(NodeType::Merge, Arc::new(MergeExecutor));

        // Output
        executors.insert(NodeType::Notify, Arc::new(NotifyExecutor));
        executors.insert(NodeType::Log, Arc::new(LogExecutor));

        Self { executors, app }
    }

    /// Execute a single node with the given context
    async fn execute_node(
        &self,
        node: &Node,
        context: &HashMap<String, serde_json::Value>,
    ) -> NodeExecutionResult {
        let result = NodeExecutionResult::new(&node.id);

        let executor = match self.executors.get(&node.node_type) {
            Some(exec) => exec,
            None => {
                return result.error(format!("No executor found for node type: {:?}", node.node_type));
            }
        };

        match executor.execute(node, context, self.app.clone()).await {
            Ok(output) => {
                tracing::info!("Node {} executed successfully", node.id);
                result.success(output)
            }
            Err(e) => {
                tracing::error!("Node {} execution failed: {}", node.id, e);
                result.error(e.to_string())
            }
        }
    }

    /// Execute an entire workflow
    pub async fn execute_workflow(
        &self,
        workflow: &Workflow,
    ) -> ExecutionResult<Vec<NodeExecutionResult>> {
        use crate::workflow::CheckpointManager;

        // Validate that the workflow is a valid DAG
        workflow
            .validate_dag()
            .map_err(ExecutionError::InvalidWorkflow)?;

        // Get execution order
        let execution_order = workflow
            .get_execution_order()
            .map_err(ExecutionError::InvalidWorkflow)?;

        tracing::info!(
            "Executing workflow '{}' with {} nodes",
            workflow.name,
            execution_order.len()
        );

        // Initialize git checkpoint
        let checkpoint_manager = CheckpointManager::init(&workflow.id)
            .map_err(|e| ExecutionError::ExecutionFailed(format!("Failed to init checkpoint: {}", e)))?;

        // Create initial checkpoint
        let workflow_json = serde_json::to_string_pretty(&workflow)
            .map_err(|e| ExecutionError::ExecutionFailed(format!("Failed to serialize workflow: {}", e)))?;

        checkpoint_manager
            .create_checkpoint(
                &format!("Workflow started: {}", workflow.name),
                &workflow_json,
            )
            .map_err(|e| ExecutionError::ExecutionFailed(format!("Failed to create checkpoint: {}", e)))?;

        let mut results = Vec::new();
        let mut context: HashMap<String, serde_json::Value> = HashMap::new();

        // Execute nodes in topological order
        for node_id in execution_order {
            let node = workflow
                .get_node(&node_id)
                .ok_or_else(|| ExecutionError::NodeNotFound(node_id.clone()))?;

            // Emit "running" status before execution starts
            if let Some(ref app) = self.app {
                let running_status = NodeExecutionResult::new(&node_id);
                let _ = app.emit("workflow-progress", &running_status);
            }

            // Collect inputs from parent nodes
            let parent_outputs = self.collect_parent_outputs(&workflow.edges, &node_id, &results);
            context.extend(parent_outputs);

            // Execute the node
            let result = self.execute_node(node, &context).await;

            // Emit final progress event with result
            if let Some(ref app) = self.app {
                let _ = app.emit("workflow-progress", &result);
            }

            // Add node outputs to context for downstream nodes
            for (key, value) in &result.output {
                context.insert(format!("{}.{}", node_id, key), value.clone());
            }

            results.push(result.clone());

            // Create checkpoint after each node execution
            let checkpoint_data = serde_json::json!({
                "workflow_id": workflow.id,
                "current_node": node_id,
                "results": results,
                "context": context,
            });

            let checkpoint_msg = format!(
                "Node completed: {} ({:?}) - Status: {:?}",
                node.label,
                node.node_type,
                result.status
            );

            let _ = checkpoint_manager.create_checkpoint(
                &checkpoint_msg,
                &serde_json::to_string_pretty(&checkpoint_data).unwrap_or_default(),
            );
        }

        // Create final checkpoint
        let final_data = serde_json::json!({
            "workflow_id": workflow.id,
            "status": "completed",
            "results": results,
            "context": context,
        });

        let _ = checkpoint_manager.create_checkpoint(
            &format!("Workflow completed: {}", workflow.name),
            &serde_json::to_string_pretty(&final_data).unwrap_or_default(),
        );

        tracing::info!(
            "Workflow '{}' completed with checkpoints in: {}",
            workflow.name,
            checkpoint_manager.repo_path().display()
        );

        Ok(results)
    }

    /// Collect outputs from parent nodes
    fn collect_parent_outputs(
        &self,
        edges: &[Edge],
        node_id: &str,
        results: &[NodeExecutionResult],
    ) -> HashMap<String, serde_json::Value> {
        let parent_ids: Vec<&str> = edges
            .iter()
            .filter(|e| e.target == node_id)
            .map(|e| e.source.as_str())
            .collect();

        let mut outputs = HashMap::new();

        for result in results {
            if parent_ids.contains(&result.node_id.as_str()) {
                for (key, value) in &result.output {
                    outputs.insert(
                        format!("{}.{}", result.node_id, key),
                        value.clone(),
                    );
                }
            }
        }

        outputs
    }
}

/// Manages multiple workflow executions
pub struct WorkflowManager {
    executor: Arc<WorkflowExecutor>,
    running_workflows: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl WorkflowManager {
    pub fn new(app: Option<Arc<AppHandle>>) -> Self {
        Self {
            executor: Arc::new(WorkflowExecutor::new(app)),
            running_workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a workflow execution asynchronously
    pub async fn start_workflow(&self, workflow: Workflow) -> ExecutionResult<String> {
        let workflow_id = workflow.id.clone();
        let executor = Arc::clone(&self.executor);
        let running_workflows = Arc::clone(&self.running_workflows);
        let wf_id = workflow_id.clone();

        let handle = tokio::spawn(async move {
            match executor.execute_workflow(&workflow).await {
                Ok(results) => {
                    tracing::info!(
                        "Workflow {} completed with {} results",
                        workflow.id,
                        results.len()
                    );
                }
                Err(e) => {
                    tracing::error!("Workflow {} failed: {}", workflow.id, e);
                }
            }

            // Remove from running workflows when done
            running_workflows.write().await.remove(&wf_id);
        });

        self.running_workflows
            .write()
            .await
            .insert(workflow_id.clone(), handle);

        Ok(workflow_id)
    }

    /// Check if a workflow is currently running
    pub async fn is_running(&self, workflow_id: &str) -> bool {
        self.running_workflows
            .read()
            .await
            .contains_key(workflow_id)
    }

    /// Cancel a running workflow
    pub async fn cancel_workflow(&self, workflow_id: &str) -> bool {
        if let Some(handle) = self.running_workflows.write().await.remove(workflow_id) {
            handle.abort();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_executor() {
        let executor = ShellExecutor;
        let mut node = Node::new(NodeType::Shell, "Test Shell");
        node.set_config("cmd", "echo 'Hello World'");

        let context = HashMap::new();
        let result = executor.execute(&node, &context, None).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains_key("stdout"));
    }

    #[tokio::test]
    async fn test_workflow_execution() {
        let executor = WorkflowExecutor::new(None);
        let mut workflow = Workflow::new("Test Workflow");

        let mut node1 = Node::new(NodeType::Shell, "Node 1");
        node1.set_config("cmd", "echo 'test'");
        let node1_id = node1.id.clone();

        workflow.add_node(node1);

        let results = executor.execute_workflow(&workflow).await;
        assert!(results.is_ok());

        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].node_id, node1_id);
        assert_eq!(results[0].status, ExecutionStatus::Success);
    }
}
