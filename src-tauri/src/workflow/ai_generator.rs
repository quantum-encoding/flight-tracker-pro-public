use anyhow::{Result, Context};
use serde_json::json;
use crate::workflow::Workflow;
use std::collections::HashMap;

/// Generate a workflow from a natural language prompt using AI
pub async fn generate_workflow_from_prompt(
    prompt: &str,
    provider: &str,
    api_key: &str,
) -> Result<Workflow> {
    // Build the system prompt with node type specifications
    let system_prompt = build_system_prompt();

    // Build the user prompt
    let user_prompt = format!(
        "{}\n\nUser Request: {}\n\nGenerate the workflow JSON now:",
        system_prompt,
        prompt
    );

    // Call the appropriate AI provider
    let response_content = match provider {
        "gemini" => {
            // Use very high token limit for workflow generation (32k tokens)
            // Gemini 2.5 Flash Lite doesn't have thinking overhead, so this should be plenty
            let result = crate::gemini::chat_with_gemini_custom(
                &user_prompt,
                api_key,
                "gemini-2.5-flash-lite",
                32768  // 32k tokens for complete workflow JSON
            ).await?;
            result.content
        }
        "deepseek" => {
            // DeepSeek uses Anthropic-compatible API
            let client = reqwest::Client::new();
            let response = client
                .post("https://api.deepseek.com/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&json!({
                    "model": "deepseek-chat",
                    "messages": [
                        {"role": "user", "content": user_prompt}
                    ],
                    "temperature": 0.7
                }))
                .send()
                .await?;

            let response_json: serde_json::Value = response.json().await?;
            response_json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string()
        }
        "grok" | "xai" => {
            let client = reqwest::Client::new();
            let response = client
                .post("https://api.x.ai/v1/chat/completions")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&json!({
                    "model": "grok-4-fast-reasoning",
                    "messages": [
                        {"role": "user", "content": user_prompt}
                    ],
                    "temperature": 0.7
                }))
                .send()
                .await?;

            let response_json: serde_json::Value = response.json().await?;
            response_json["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_string()
        }
        _ => return Err(anyhow::anyhow!("Unsupported provider: {}", provider)),
    };

    // Clean and parse the JSON response
    let workflow_json = clean_json_response(&response_content)?;

    eprintln!("=== ATTEMPTING TO PARSE WORKFLOW ===");
    let mut workflow: Workflow = serde_json::from_str(&workflow_json)
        .map_err(|e| {
            eprintln!("=== PARSE ERROR ===");
            eprintln!("Error: {}", e);
            eprintln!("JSON was: {}", &workflow_json[..workflow_json.len().min(1000)]);
            anyhow::anyhow!("Failed to parse AI response as workflow JSON: {}", e)
        })?;

    eprintln!("=== PARSE SUCCESS ===");
    eprintln!("Workflow name: {}", workflow.name);
    eprintln!("Nodes: {}", workflow.nodes.len());
    eprintln!("Edges: {}", workflow.edges.len());

    // Generate UUIDs for all nodes and edges
    for node in &mut workflow.nodes {
        node.id = uuid::Uuid::new_v4().to_string();
    }

    // Update edge references to use new UUIDs
    let id_map: HashMap<String, String> = workflow.nodes.iter()
        .enumerate()
        .map(|(i, node)| ((i + 1).to_string(), node.id.clone()))
        .collect();

    for edge in &mut workflow.edges {
        edge.id = uuid::Uuid::new_v4().to_string();
        if let Some(new_source) = id_map.get(&edge.source) {
            edge.source = new_source.clone();
        }
        if let Some(new_target) = id_map.get(&edge.target) {
            edge.target = new_target.clone();
        }
    }

    // Set workflow ID
    workflow.id = uuid::Uuid::new_v4().to_string();

    // Validate the generated workflow
    workflow.validate_dag()
        .map_err(|e| anyhow::anyhow!("Generated workflow contains cycles or is invalid: {}", e))?;

    tracing::info!(
        "AI generated workflow: {} with {} nodes and {} edges",
        workflow.name,
        workflow.nodes.len(),
        workflow.edges.len()
    );

    Ok(workflow)
}

/// Build the system prompt with all node type specifications
fn build_system_prompt() -> String {
    r#"You are a workflow generation expert. Convert natural language descriptions into executable workflow definitions.

CRITICAL RULES:
1. Return ONLY valid JSON - no markdown, no code blocks, no explanations
2. Use realistic, executable configurations
3. Create proper dependency chains with edges
4. Position nodes left-to-right (x = node_index * 350, y varies by layer)

AVAILABLE NODE TYPES:

**Execution Nodes:**
- Shell: Execute shell commands
  Config: { "cmd": "shell command here", "cwd": "/optional/path" }

- AiPrompt: Call AI for processing
  Config: { "provider": "gemini|deepseek|grok", "model": "gemini-2.5-flash-lite", "prompt": "..." }

- Database: Database operations
  Config: { "query": "SELECT * FROM ...", "connection": "..." }

**Data Operations:**
- HttpRequest: HTTP API calls
  Config: { "url": "https://...", "method": "GET|POST", "body": "optional" }

- FileRead: Read files
  Config: { "path": "/path/to/file.txt" }

- FileWrite: Write files
  Config: { "path": "/path/to/output.txt", "content": "{{variable}}" }

- Transform: Transform data
  Config: { "operation": "uppercase|lowercase|trim|json_parse", "input": "{{node.output}}" }

- Filter: Conditional filtering
  Config: { "condition": "equals|contains|greater_than", "value": "..." }

**Control Flow:**
- Aggregator: Wait for multiple inputs
  Config: { "required_inputs": "2" }
  RequiredInputs: 2

- Merge: Merge data streams
  Config: { "strategy": "concat|merge|latest" }

**Output:**
- Log: Write to logs
  Config: { "level": "info|warn|error", "message": "{{node.output}}" }

- Notify: System notification
  Config: { "title": "...", "message": "..." }

VARIABLE SYSTEM:
Reference previous node outputs using: {{node_id.output_key}}
Example: "variables": { "data": "1.stdout", "result": "2.response" }

OUTPUT FORMAT:
{
  "name": "Workflow Name",
  "description": "Brief description",
  "nodes": [
    {
      "id": "1",
      "label": "Descriptive Name",
      "type": "Shell",
      "x": 100,
      "y": 100,
      "config": { "cmd": "echo 'Hello'" }
    },
    {
      "id": "2",
      "label": "Process Data",
      "type": "AiPrompt",
      "x": 450,
      "y": 100,
      "config": {
        "provider": "gemini",
        "model": "gemini-2.5-flash-lite",
        "prompt": "Analyze this: {{1.stdout}}"
      },
      "variables": { "input": "1.stdout" }
    }
  ],
  "edges": [
    { "id": "e1", "source": "1", "target": "2" }
  ]
}

EXAMPLES:

Request: "scrape a website and analyze it"
Response:
{
  "name": "Web Scraper & Analyzer",
  "description": "Fetch webpage content and analyze with AI",
  "nodes": [
    {"id": "1", "label": "Fetch Webpage", "type": "HttpRequest", "x": 100, "y": 100, "config": {"url": "https://example.com", "method": "GET"}},
    {"id": "2", "label": "Analyze Content", "type": "AiPrompt", "x": 450, "y": 100, "config": {"provider": "gemini", "model": "gemini-2.5-flash-lite", "prompt": "Summarize this webpage: {{1.body}}"}, "variables": {"content": "1.body"}},
    {"id": "3", "label": "Save Analysis", "type": "FileWrite", "x": 800, "y": 100, "config": {"path": "analysis.txt", "content": "{{2.response}}"}},
    {"id": "4", "label": "Notify Done", "type": "Notify", "x": 1150, "y": 100, "config": {"title": "Analysis Complete", "message": "Results saved"}}
  ],
  "edges": [
    {"id": "e1", "source": "1", "target": "2"},
    {"id": "e2", "source": "2", "target": "3"},
    {"id": "e3", "source": "3", "target": "4"}
  ]
}

Now generate a workflow based on the user's request."#.to_string()
}

/// Clean JSON response from AI (remove markdown code blocks, etc.)
fn clean_json_response(response: &str) -> Result<String> {
    let trimmed = response.trim();

    // Log raw response for debugging
    eprintln!("=== RAW AI RESPONSE ===");
    eprintln!("{}", &trimmed[..trimmed.len().min(500)]);
    eprintln!("=== END RAW RESPONSE ===");

    // Remove markdown code blocks if present
    let json_str = if trimmed.starts_with("```") {
        // Extract content between ```json and ```
        let lines: Vec<&str> = trimmed.lines().collect();
        let mut in_code_block = false;
        let mut json_lines = Vec::new();

        for line in lines {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
                continue;
            }
            if in_code_block {
                json_lines.push(line);
            }
        }

        json_lines.join("\n")
    } else {
        trimmed.to_string()
    };

    eprintln!("=== CLEANED JSON ===");
    eprintln!("{}", &json_str[..json_str.len().min(500)]);
    eprintln!("=== END CLEANED ===");

    // Find the first '{' and last '}'
    if let (Some(start), Some(end)) = (json_str.find('{'), json_str.rfind('}')) {
        let extracted = json_str[start..=end].to_string();
        eprintln!("=== EXTRACTED JSON ({}..{}) ===", start, end);
        eprintln!("{}", &extracted[..extracted.len().min(500)]);
        Ok(extracted)
    } else {
        Err(anyhow::anyhow!("No valid JSON object found in response. Response was: {}", &trimmed[..trimmed.len().min(200)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_json_response() {
        let markdown_response = r#"```json
{
  "name": "Test",
  "nodes": []
}
```"#;

        let cleaned = clean_json_response(markdown_response).unwrap();
        assert!(cleaned.starts_with('{'));
        assert!(cleaned.ends_with('}'));
        assert!(serde_json::from_str::<serde_json::Value>(&cleaned).is_ok());
    }

    #[test]
    fn test_clean_json_with_text() {
        let response = r#"Here's the workflow:
{
  "name": "Test"
}
That should work!"#;

        let cleaned = clean_json_response(response).unwrap();
        assert_eq!(cleaned, r#"{"name": "Test"}"#.trim());
    }
}
