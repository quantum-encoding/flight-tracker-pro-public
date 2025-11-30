/**
 * Core workflow types for the DAG engine - Svelte 5 Port
 */

export type NodeType =
  | 'Shell'
  | 'AiPrompt'
  | 'Database'
  | 'TradeAgent'
  | 'Aggregator'
  | 'Transform'
  | 'Filter'
  | 'HTTPRequest'
  | 'Email'
  | 'Scheduler'
  | 'FileOperation'
  | 'Webhook';

export type ExecutionStatus = 'idle' | 'running' | 'success' | 'error' | 'retrying';

export interface NodeData {
  id: string;
  label: string;
  type: NodeType;
  x: number;
  y: number;
  config: Record<string, string>;
  comments?: string;
  requiredInputs?: number;
  variables?: Record<string, string>;
  waitForAll?: boolean;
  timeout?: number;
  retryPolicy?: {
    maxAttempts: number;
    backoffMultiplier: number;
    initialDelay: number;
  };
}

export interface EdgeData {
  id: string;
  source: string;
  target: string;
}

export interface ViewState {
  x: number;
  y: number;
  zoom: number;
}

export interface HistoryState {
  nodes: NodeData[];
  edges: EdgeData[];
}

export interface WorkflowState {
  nodes: NodeData[];
  edges: EdgeData[];
  viewState: ViewState;
  history: HistoryState[];
  historyIndex: number;
}

export type PortType = 'string' | 'number' | 'json' | 'boolean';
export type ConfigFieldType = 'text' | 'textarea' | 'number' | 'select';

export interface PortDef {
  id: string;
  label: string;
  type: PortType;
}

export interface ConfigField {
  key: string;
  label: string;
  type: ConfigFieldType;
  options?: string[];
  required?: boolean;
}

export interface NodeSpec {
  inputs: PortDef[];
  outputs: PortDef[];
  configFields: ConfigField[];
}

export interface NodeExecutionData {
  input: Record<string, any>;
  output: Record<string, any>;
  timestamp: number;
  duration: number;
}

export type NodeStatusMap = Record<string, ExecutionStatus>;
export type NodeSpecMap = Record<NodeType, NodeSpec>;

export interface LogEntry {
  timestamp: number;
  level: 'info' | 'warn' | 'error' | 'success';
  message: string;
  nodeId?: string;
}
