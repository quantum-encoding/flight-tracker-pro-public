use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Node types supported by the workflow engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "PascalCase")]
pub enum NodeType {
    // Execution nodes
    Shell,
    AiPrompt,
    Database,
    TradeAgent,

    // Data operations
    HttpRequest,
    FileRead,
    FileWrite,
    Transform,
    Filter,

    // Control flow
    Conditional,
    Loop,
    Aggregator,
    Merge,

    // Output
    Notify,
    Log,
}

/// Represents a node in the workflow DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub x: f64,
    pub y: f64,
    #[serde(deserialize_with = "deserialize_flexible_config")]
    pub config: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_inputs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", deserialize_with = "deserialize_optional_flexible_config", default)]
    pub variables: Option<HashMap<String, String>>,
}

// Custom deserializer that converts any JSON value to String
fn deserialize_flexible_config<'de, D>(deserializer: D) -> Result<HashMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let map: HashMap<String, serde_json::Value> = HashMap::deserialize(deserializer)?;
    Ok(map.into_iter().map(|(k, v)| {
        let v_str = match v {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Array(arr) => serde_json::to_string(&arr).unwrap_or_default(),
            serde_json::Value::Object(obj) => serde_json::to_string(&obj).unwrap_or_default(),
            serde_json::Value::Null => String::new(),
        };
        (k, v_str)
    }).collect())
}

fn deserialize_optional_flexible_config<'de, D>(deserializer: D) -> Result<Option<HashMap<String, String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<HashMap<String, serde_json::Value>> = Option::deserialize(deserializer)?;
    Ok(opt.map(|map| {
        map.into_iter().map(|(k, v)| {
            let v_str = match v {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Array(arr) => serde_json::to_string(&arr).unwrap_or_default(),
                serde_json::Value::Object(obj) => serde_json::to_string(&obj).unwrap_or_default(),
                serde_json::Value::Null => String::new(),
            };
            (k, v_str)
        }).collect()
    }))
}

impl Node {
    /// Create a new node with the given type and label
    pub fn new(node_type: NodeType, label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.into(),
            node_type,
            x: 0.0,
            y: 0.0,
            config: HashMap::new(),
            comments: None,
            required_inputs: None,
            variables: None,
        }
    }

    /// Get a configuration value by key
    pub fn get_config(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }

    /// Set a configuration value
    pub fn set_config(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.config.insert(key.into(), value.into());
    }

    /// Interpolate variables in the format {{variable_name}} with actual values
    pub fn interpolate_config(&self, key: &str, context: &HashMap<String, serde_json::Value>) -> Option<String> {
        let value = self.get_config(key)?;
        let mut result = value.clone();

        // Replace {{variable}} patterns with values from context
        let var_pattern = regex::Regex::new(r"\{\{([^}]+)\}\}").ok()?;
        for cap in var_pattern.captures_iter(value) {
            if let Some(var_name) = cap.get(1) {
                let var_key = var_name.as_str().trim();
                if let Some(var_value) = context.get(var_key) {
                    let replacement = match var_value {
                        serde_json::Value::String(s) => s.clone(),
                        v => v.to_string(),
                    };
                    result = result.replace(&format!("{{{{{}}}}}", var_key), &replacement);
                }
            }
        }

        Some(result)
    }
}

/// Represents an edge (connection) between two nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
}

impl Edge {
    /// Create a new edge from source to target
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source: source.into(),
            target: target.into(),
        }
    }
}

/// Represents a complete workflow with nodes and edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    #[serde(default = "default_workflow_id")]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

fn default_workflow_id() -> String {
    Uuid::new_v4().to_string()
}

impl Workflow {
    /// Create a new empty workflow
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            metadata: None,
        }
    }

    /// Add a node to the workflow
    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    /// Add an edge to the workflow
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get a mutable reference to a node by ID
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Remove a node and all connected edges
    pub fn remove_node(&mut self, id: &str) -> bool {
        if let Some(idx) = self.nodes.iter().position(|n| n.id == id) {
            self.nodes.remove(idx);
            self.edges.retain(|e| e.source != id && e.target != id);
            true
        } else {
            false
        }
    }

    /// Remove an edge by ID
    pub fn remove_edge(&mut self, id: &str) -> bool {
        if let Some(idx) = self.edges.iter().position(|e| e.id == id) {
            self.edges.remove(idx);
            true
        } else {
            false
        }
    }

    /// Validate that the workflow forms a valid DAG (no cycles)
    pub fn validate_dag(&self) -> Result<(), String> {
        use petgraph::graph::DiGraph;
        use petgraph::algo::toposort;

        let mut graph = DiGraph::<&str, ()>::new();
        let mut node_indices = HashMap::new();

        // Add all nodes to the graph
        for node in &self.nodes {
            let idx = graph.add_node(node.id.as_str());
            node_indices.insert(&node.id, idx);
        }

        // Add all edges
        for edge in &self.edges {
            if let (Some(&source_idx), Some(&target_idx)) =
                (node_indices.get(&edge.source), node_indices.get(&edge.target))
            {
                graph.add_edge(source_idx, target_idx, ());
            }
        }

        // Perform topological sort to detect cycles
        toposort(&graph, None)
            .map(|_| ())
            .map_err(|_| "Workflow contains cycles - must be a valid DAG".to_string())
    }

    /// Get nodes in topological order for execution
    pub fn get_execution_order(&self) -> Result<Vec<String>, String> {
        use petgraph::graph::DiGraph;
        use petgraph::algo::toposort;

        let mut graph = DiGraph::<&str, ()>::new();
        let mut node_indices = HashMap::new();

        for node in &self.nodes {
            let idx = graph.add_node(node.id.as_str());
            node_indices.insert(&node.id, idx);
        }

        for edge in &self.edges {
            if let (Some(&source_idx), Some(&target_idx)) =
                (node_indices.get(&edge.source), node_indices.get(&edge.target))
            {
                graph.add_edge(source_idx, target_idx, ());
            }
        }

        toposort(&graph, None)
            .map(|order| {
                order.iter()
                    .map(|&idx| graph[idx].to_string())
                    .collect()
            })
            .map_err(|_| "Cannot determine execution order - workflow contains cycles".to_string())
    }
}

/// Execution status for a node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Idle,
    Running,
    Success,
    Error,
    Retrying,
}

/// Result of node execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionResult {
    pub node_id: String,
    pub status: ExecutionStatus,
    pub output: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
}

impl NodeExecutionResult {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            status: ExecutionStatus::Running,
            output: HashMap::new(),
            error: None,
            start_time: chrono::Utc::now(),
            end_time: None,
            duration_ms: None,
        }
    }

    pub fn success(mut self, output: HashMap<String, serde_json::Value>) -> Self {
        let end_time = chrono::Utc::now();
        let duration = end_time.signed_duration_since(self.start_time);

        self.status = ExecutionStatus::Success;
        self.output = output;
        self.end_time = Some(end_time);
        self.duration_ms = Some(duration.num_milliseconds() as u64);
        self
    }

    pub fn error(mut self, error: impl Into<String>) -> Self {
        let end_time = chrono::Utc::now();
        let duration = end_time.signed_duration_since(self.start_time);

        self.status = ExecutionStatus::Error;
        self.error = Some(error.into());
        self.end_time = Some(end_time);
        self.duration_ms = Some(duration.num_milliseconds() as u64);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new(NodeType::Shell, "Test Node");
        assert_eq!(node.label, "Test Node");
        assert_eq!(node.node_type, NodeType::Shell);
        assert!(!node.id.is_empty());
    }

    #[test]
    fn test_workflow_dag_validation() {
        let mut workflow = Workflow::new("Test Workflow");

        let node1 = Node::new(NodeType::Shell, "Node 1");
        let node2 = Node::new(NodeType::Shell, "Node 2");
        let node1_id = node1.id.clone();
        let node2_id = node2.id.clone();

        workflow.add_node(node1);
        workflow.add_node(node2);
        workflow.add_edge(Edge::new(&node1_id, &node2_id));

        assert!(workflow.validate_dag().is_ok());
    }

    #[test]
    fn test_workflow_cycle_detection() {
        let mut workflow = Workflow::new("Cyclic Workflow");

        let node1 = Node::new(NodeType::Shell, "Node 1");
        let node2 = Node::new(NodeType::Shell, "Node 2");
        let node1_id = node1.id.clone();
        let node2_id = node2.id.clone();

        workflow.add_node(node1);
        workflow.add_node(node2);
        workflow.add_edge(Edge::new(&node1_id, &node2_id));
        workflow.add_edge(Edge::new(&node2_id, &node1_id));

        assert!(workflow.validate_dag().is_err());
    }
}
