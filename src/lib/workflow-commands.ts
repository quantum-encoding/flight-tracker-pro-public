import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

// Type definitions matching Rust models

export type NodeType =
  // Execution nodes
  | 'Shell'
  | 'AiPrompt'
  | 'Database'
  | 'TradeAgent'
  // Data operations
  | 'HttpRequest'
  | 'FileRead'
  | 'FileWrite'
  | 'Transform'
  | 'Filter'
  // Control flow
  | 'Conditional'
  | 'Loop'
  | 'Aggregator'
  | 'Merge'
  // Output
  | 'Notify'
  | 'Log';

export interface Node {
  id: string;
  label: string;
  type: NodeType;
  x: number;
  y: number;
  config: Record<string, string>;
  comments?: string;
  required_inputs?: number;
  variables?: Record<string, string>;
}

export interface Edge {
  id: string;
  source: string;
  target: string;
}

export interface Workflow {
  id: string;
  name: string;
  description?: string;
  nodes: Node[];
  edges: Edge[];
  metadata?: Record<string, any>;
}

export type ExecutionStatus = 'idle' | 'running' | 'success' | 'error' | 'retrying';

export interface NodeExecutionResult {
  node_id: string;
  status: ExecutionStatus;
  output: Record<string, any>;
  error?: string;
  start_time: string;
  end_time?: string;
  duration_ms?: number;
}

export interface Checkpoint {
  commit_hash: string;
  message: string;
  timestamp: string;
  workflow_id: string;
}

// Tauri command wrappers

/**
 * Validate that a workflow is a valid DAG (no cycles)
 */
export async function validateWorkflow(workflow: Workflow): Promise<string> {
  return invoke<string>('validate_workflow', { workflow });
}

/**
 * Get the execution order for nodes in a workflow
 */
export async function getExecutionOrder(workflow: Workflow): Promise<string[]> {
  return invoke<string[]>('get_execution_order', { workflow });
}

/**
 * Execute a workflow asynchronously
 * Returns the workflow ID for tracking
 */
export async function executeWorkflow(workflow: Workflow): Promise<string> {
  return invoke<string>('execute_workflow', { workflow });
}

/**
 * Check if a workflow is currently running
 */
export async function isWorkflowRunning(workflowId: string): Promise<boolean> {
  return invoke<boolean>('is_workflow_running', { workflowId });
}

/**
 * Cancel a running workflow
 */
export async function cancelWorkflow(workflowId: string): Promise<boolean> {
  return invoke<boolean>('cancel_workflow', { workflowId });
}

/**
 * Export a workflow to a JSON file
 */
export async function exportWorkflow(workflow: Workflow, path: string): Promise<void> {
  return invoke<void>('export_workflow', { workflow, path });
}

/**
 * Import a workflow from a JSON file
 */
export async function importWorkflow(path: string): Promise<Workflow> {
  return invoke<Workflow>('import_workflow', { path });
}

// Event listeners

/**
 * Listen for workflow progress events
 */
export async function onWorkflowProgress(
  callback: (result: NodeExecutionResult) => void
): Promise<UnlistenFn> {
  return listen<NodeExecutionResult>('workflow-progress', (event) => {
    callback(event.payload);
  });
}

// Utility functions

/**
 * Create a new empty workflow
 */
export function createWorkflow(name: string, description?: string): Workflow {
  return {
    id: crypto.randomUUID(),
    name,
    description,
    nodes: [],
    edges: [],
    metadata: {},
  };
}

/**
 * Create a new node
 */
export function createNode(
  type: NodeType,
  label: string,
  x: number = 0,
  y: number = 0
): Node {
  return {
    id: crypto.randomUUID(),
    label,
    type,
    x,
    y,
    config: {},
  };
}

/**
 * Create a new edge between nodes
 */
export function createEdge(source: string, target: string): Edge {
  return {
    id: crypto.randomUUID(),
    source,
    target,
  };
}

// Checkpoint commands

/**
 * Initialize git checkpoint for a workflow
 */
export async function initWorkflowCheckpoint(workflowId: string): Promise<string> {
  return invoke<string>('init_workflow_checkpoint', { workflowId });
}

/**
 * Create a checkpoint with workflow state
 */
export async function createCheckpoint(
  workflowId: string,
  message: string,
  data: string
): Promise<Checkpoint> {
  return invoke<Checkpoint>('create_checkpoint', { workflowId, message, data });
}

/**
 * Get checkpoint history for a workflow
 */
export async function getCheckpointHistory(workflowId: string): Promise<Checkpoint[]> {
  return invoke<Checkpoint[]>('get_checkpoint_history', { workflowId });
}

/**
 * Get workflow state at a specific checkpoint
 */
export async function getCheckpointState(
  workflowId: string,
  commitHash: string
): Promise<string> {
  return invoke<string>('get_checkpoint_state', { workflowId, commitHash });
}

/**
 * Generate a workflow from natural language prompt using AI
 */
export async function generateWorkflowAI(
  prompt: string,
  provider: string
): Promise<Workflow> {
  return invoke<Workflow>('generate_workflow_ai', { prompt, provider });
}

/**
 * Check if Tauri is available (running in desktop app vs web)
 */
export function isTauriAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI__' in window;
}

/**
 * Safe invoke that checks if Tauri is available
 */
export async function safeInvoke<T>(
  command: string,
  args?: Record<string, any>
): Promise<T | null> {
  if (!isTauriAvailable()) {
    console.warn(`Tauri not available, cannot invoke: ${command}`);
    return null;
  }
  return invoke<T>(command, args);
}
