/**
 * Node Specifications for all 12 workflow node types
 */

import type { NodeSpec, NodeSpecMap } from '$lib/types/workflow';

export const NODE_SPECS: NodeSpecMap = {
  // Core Nodes
  Shell: {
    inputs: [
      { id: 'stdin', label: 'Standard Input', type: 'string' },
      { id: 'env', label: 'Environment Vars', type: 'json' }
    ],
    outputs: [
      { id: 'stdout', label: 'Standard Output', type: 'string' },
      { id: 'exit_code', label: 'Exit Code', type: 'number' }
    ],
    configFields: [
      { key: 'cmd', label: 'Command', type: 'textarea', required: true },
      { key: 'cwd', label: 'Working Directory', type: 'text' },
      { key: 'timeout', label: 'Timeout (ms)', type: 'number' }
    ]
  },

  AiPrompt: {
    inputs: [{ id: 'context', label: 'Context Data', type: 'string' }],
    outputs: [
      { id: 'response', label: 'AI Response', type: 'string' },
      { id: 'tokens', label: 'Token Usage', type: 'number' }
    ],
    configFields: [
      { key: 'provider', label: 'Provider', type: 'select', options: ['gemini', 'deepseek', 'grok'], required: true },
      { key: 'model', label: 'Model', type: 'text', required: true },
      { key: 'prompt', label: 'System Prompt', type: 'textarea', required: true },
      { key: 'temperature', label: 'Temperature', type: 'number' }
    ]
  },

  Database: {
    inputs: [{ id: 'params', label: 'Parameters', type: 'json' }],
    outputs: [{ id: 'rows', label: 'Result Rows', type: 'json' }],
    configFields: [
      { key: 'connection', label: 'Connection String', type: 'text' },
      { key: 'query', label: 'SQL Query', type: 'textarea', required: true }
    ]
  },

  TradeAgent: {
    inputs: [{ id: 'trigger_signal', label: 'Trigger Signal', type: 'number' }],
    outputs: [
      { id: 'tx_hash', label: 'TX Hash', type: 'string' },
      { id: 'status', label: 'Order Status', type: 'string' }
    ],
    configFields: [
      { key: 'asset', label: 'Asset Symbol', type: 'text', required: true },
      { key: 'action', label: 'Action', type: 'select', options: ['BUY', 'SELL', 'HOLD'] },
      { key: 'risk_score', label: 'Max Risk Score', type: 'number' },
      { key: 'amount', label: 'Amount', type: 'number', required: true }
    ]
  },

  // Data Processing
  Aggregator: {
    inputs: [
      { id: 'input1', label: 'Input 1', type: 'json' },
      { id: 'input2', label: 'Input 2', type: 'json' },
      { id: 'input3', label: 'Input 3', type: 'json' }
    ],
    outputs: [
      { id: 'merged', label: 'Merged Data', type: 'json' },
      { id: 'count', label: 'Input Count', type: 'number' }
    ],
    configFields: [
      { key: 'strategy', label: 'Merge Strategy', type: 'select', options: ['merge', 'concat', 'first', 'last'], required: true },
      { key: 'wait_for_all', label: 'Wait for All Inputs', type: 'select', options: ['true', 'false'] },
      { key: 'timeout', label: 'Wait Timeout (ms)', type: 'number' }
    ]
  },

  Transform: {
    inputs: [{ id: 'data', label: 'Input Data', type: 'json' }],
    outputs: [{ id: 'result', label: 'Transformed Data', type: 'json' }],
    configFields: [
      { key: 'operation', label: 'Operation', type: 'select', options: ['map', 'filter', 'reduce', 'custom'], required: true },
      { key: 'function', label: 'Transform Function', type: 'textarea', required: true }
    ]
  },

  Filter: {
    inputs: [{ id: 'data', label: 'Input Data', type: 'json' }],
    outputs: [
      { id: 'passed', label: 'Passed Filter', type: 'json' },
      { id: 'failed', label: 'Failed Filter', type: 'json' }
    ],
    configFields: [
      { key: 'condition', label: 'Filter Condition', type: 'textarea', required: true },
      { key: 'mode', label: 'Mode', type: 'select', options: ['javascript', 'jsonpath'] }
    ]
  },

  // Integration
  HTTPRequest: {
    inputs: [
      { id: 'url', label: 'URL', type: 'string' },
      { id: 'body', label: 'Request Body', type: 'json' }
    ],
    outputs: [
      { id: 'response', label: 'Response Data', type: 'json' },
      { id: 'status', label: 'Status Code', type: 'number' }
    ],
    configFields: [
      { key: 'method', label: 'Method', type: 'select', options: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'], required: true },
      { key: 'url', label: 'URL', type: 'text', required: true },
      { key: 'headers', label: 'Headers (JSON)', type: 'textarea' }
    ]
  },

  Email: {
    inputs: [
      { id: 'to', label: 'Recipients', type: 'string' },
      { id: 'body', label: 'Email Body', type: 'string' }
    ],
    outputs: [{ id: 'status', label: 'Send Status', type: 'string' }],
    configFields: [
      { key: 'from', label: 'From Address', type: 'text', required: true },
      { key: 'subject', label: 'Subject', type: 'text', required: true },
      { key: 'smtp_server', label: 'SMTP Server', type: 'text' }
    ]
  },

  Webhook: {
    inputs: [],
    outputs: [
      { id: 'payload', label: 'Webhook Payload', type: 'json' },
      { id: 'headers', label: 'Request Headers', type: 'json' }
    ],
    configFields: [
      { key: 'path', label: 'Webhook Path', type: 'text', required: true },
      { key: 'method', label: 'Allowed Methods', type: 'select', options: ['POST', 'GET', 'ANY'] }
    ]
  },

  FileOperation: {
    inputs: [
      { id: 'content', label: 'File Content', type: 'string' }
    ],
    outputs: [
      { id: 'data', label: 'File Data', type: 'string' },
      { id: 'success', label: 'Operation Success', type: 'boolean' }
    ],
    configFields: [
      { key: 'operation', label: 'Operation', type: 'select', options: ['read', 'write', 'append', 'delete'], required: true },
      { key: 'path', label: 'File Path', type: 'text', required: true },
      { key: 'encoding', label: 'Encoding', type: 'select', options: ['utf8', 'base64', 'binary'] }
    ]
  },

  Scheduler: {
    inputs: [],
    outputs: [
      { id: 'trigger', label: 'Trigger Signal', type: 'boolean' },
      { id: 'timestamp', label: 'Execution Time', type: 'number' }
    ],
    configFields: [
      { key: 'schedule', label: 'Cron Expression', type: 'text', required: true },
      { key: 'timezone', label: 'Timezone', type: 'text' },
      { key: 'enabled', label: 'Enabled', type: 'select', options: ['true', 'false'] }
    ]
  }
};

export const NODE_CATEGORIES = {
  'Core Nodes': ['Shell', 'AiPrompt', 'Database', 'TradeAgent'],
  'Data Processing': ['Aggregator', 'Transform', 'Filter'],
  'Integration': ['HTTPRequest', 'Email', 'Webhook', 'FileOperation', 'Scheduler']
} as const;
