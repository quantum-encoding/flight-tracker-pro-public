<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { fade, fly, slide } from 'svelte/transition';
  import {
    executeWorkflow,
    isWorkflowRunning,
    onWorkflowProgress,
    getCheckpointHistory,
    generateWorkflowAI,
    exportWorkflow,
    type Workflow as WorkflowType,
    type NodeExecutionResult,
    type Checkpoint
  } from '$lib/workflow-commands';
  import { save } from '@tauri-apps/plugin-dialog';
  import AgentTracker from '$lib/components/AgentTracker.svelte';
  import { theme } from '$lib/theme';

  // --- Types ---
  interface NodeData {
    id: string;
    label: string;
    type: 'Shell' | 'AiPrompt' | 'Database' | 'TradeAgent' | 'HttpRequest' | 'FileRead' | 'FileWrite' | 'Transform' | 'Filter' | 'Conditional' | 'Loop' | 'Aggregator' | 'Merge' | 'Notify' | 'Log';
    x: number;
    y: number;
    config: Record<string, string>;
    comments?: string;
    requiredInputs?: number;
    variables?: Record<string, string>;
  }

  interface EdgeData {
    id: string;
    source: string;
    target: string;
  }

  interface ViewState {
    x: number;
    y: number;
    zoom: number;
  }

  interface LogEntry {
    id: string;
    timestamp: string;
    level: 'info' | 'warn' | 'error' | 'success';
    source: string;
    message: string;
  }

  interface NodeExecutionData {
    input: Record<string, any>;
    output: Record<string, any>;
    timestamp: number;
    duration: number;
  }

  // --- Constants ---
  const NODE_SPECS = {
    // Execution Nodes
    'Shell': {
      color: 'indigo',
      category: 'Execution',
      icon: 'M8 9l3 3-3 3m5 0h3',
      config: [
        { key: 'cmd', label: 'Command', type: 'textarea' },
        { key: 'cwd', label: 'Working Dir', type: 'text' }
      ]
    },
    'AiPrompt': {
      color: 'violet',
      category: 'Execution',
      icon: 'M13 10V3L4 14h7v7l9-11h-7z',
      config: [
        { key: 'provider', label: 'Provider', type: 'select', options: ['gemini', 'deepseek', 'grok'] },
        { key: 'model', label: 'Model', type: 'select', options: [
          'gemini-2.5-flash',
          'gemini-2.5-pro',
          'gemini-2.5-flash-lite',
          'deepseek-chat',
          'grok-4-fast-non-reasoning',
          'grok-4-fast-reasoning',
          'grok-code-fast-1'
        ] },
        { key: 'prompt', label: 'Prompt', type: 'textarea' }
      ]
    },
    'Database': {
      color: 'cyan',
      category: 'Execution',
      icon: 'M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4',
      config: [
        { key: 'query', label: 'SQL Query', type: 'textarea' },
        { key: 'db_type', label: 'Database Type', type: 'select', options: ['sqlite', 'postgres', 'mysql'] }
      ]
    },
    'TradeAgent': {
      color: 'emerald',
      category: 'Execution',
      icon: 'M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z',
      config: [
        { key: 'symbol', label: 'Symbol', type: 'text' },
        { key: 'strategy', label: 'Strategy', type: 'text' },
        { key: 'action', label: 'Action', type: 'select', options: ['BUY', 'SELL', 'HOLD'] }
      ]
    },

    // Data Operations
    'HttpRequest': {
      color: 'blue',
      category: 'Data',
      icon: 'M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9',
      config: [
        { key: 'url', label: 'URL', type: 'text' },
        { key: 'method', label: 'Method', type: 'select', options: ['GET', 'POST', 'PUT', 'DELETE'] },
        { key: 'body', label: 'Body', type: 'textarea' }
      ]
    },
    'FileRead': {
      color: 'sky',
      category: 'Data',
      icon: 'M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z',
      config: [
        { key: 'path', label: 'File Path', type: 'text' }
      ]
    },
    'FileWrite': {
      color: 'sky',
      category: 'Data',
      icon: 'M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z',
      config: [
        { key: 'path', label: 'File Path', type: 'text' },
        { key: 'content', label: 'Content', type: 'textarea' }
      ]
    },
    'Transform': {
      color: 'purple',
      category: 'Data',
      icon: 'M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4',
      config: [
        { key: 'operation', label: 'Operation', type: 'select', options: ['uppercase', 'lowercase', 'trim', 'json_parse'] },
        { key: 'input', label: 'Input', type: 'textarea' }
      ]
    },
    'Filter': {
      color: 'pink',
      category: 'Data',
      icon: 'M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z',
      config: [
        { key: 'condition', label: 'Condition', type: 'text' }
      ]
    },

    // Control Flow
    'Aggregator': {
      color: 'amber',
      category: 'Control Flow',
      icon: 'M17 14v6m-3-3h6M6 10h2a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v2a2 2 0 002 2zm10 0h2a2 2 0 002-2V6a2 2 0 00-2-2h-2a2 2 0 00-2 2v2a2 2 0 002 2zM6 20h2a2 2 0 002-2v-2a2 2 0 00-2-2H6a2 2 0 00-2 2v2a2 2 0 002 2z',
      config: [
        { key: 'required_inputs', label: 'Required Inputs', type: 'number' }
      ]
    },
    'Merge': {
      color: 'orange',
      category: 'Control Flow',
      icon: 'M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4',
      config: []
    },
    'Conditional': {
      color: 'yellow',
      category: 'Control Flow',
      icon: 'M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z',
      config: [
        { key: 'condition', label: 'Condition', type: 'text' },
        { key: 'true_path', label: 'True Path', type: 'text' },
        { key: 'false_path', label: 'False Path', type: 'text' }
      ]
    },
    'Loop': {
      color: 'lime',
      category: 'Control Flow',
      icon: 'M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15',
      config: [
        { key: 'iterations', label: 'Iterations', type: 'number' },
        { key: 'array', label: 'Array Input', type: 'text' }
      ]
    },

    // Output
    'Log': {
      color: 'slate',
      category: 'Output',
      icon: 'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z',
      config: [
        { key: 'message', label: 'Message', type: 'textarea' },
        { key: 'level', label: 'Level', type: 'select', options: ['info', 'warn', 'error', 'debug'] }
      ]
    },
    'Notify': {
      color: 'rose',
      category: 'Output',
      icon: 'M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9',
      config: [
        { key: 'title', label: 'Title', type: 'text' },
        { key: 'message', label: 'Message', type: 'textarea' }
      ]
    }
  };

  // --- State (Runes) ---
  let nodes = $state<NodeData[]>([
    { id: '1', label: 'Get System Info', type: 'Shell', x: 100, y: 100, config: { cmd: 'echo "System: $(uname -s), Date: $(date +%Y-%m-%d)"' } },
    { id: '2', label: 'Analyze System', type: 'AiPrompt', x: 450, y: 150, config: { prompt: 'Summarize this system information in one sentence', model: 'gemini-2.5-flash', provider: 'gemini' }, variables: { 'context': '1.stdout' } },
    { id: '3', label: 'Log Result', type: 'Log', x: 800, y: 100, config: { level: 'info' }, variables: { 'message': '2.response' } },
  ]);
  
  let edges = $state<EdgeData[]>([
    { id: 'e1-2', source: '1', target: '2' },
    { id: 'e2-3', source: '2', target: '3' },
  ]);

  let view = $state<ViewState>({ x: 0, y: 0, zoom: 1 });
  let selectedNodeId = $state<string | null>(null);
  let connectingNodeId = $state<string | null>(null);
  let connectionPreviewPos = $state<{x: number, y: number}>({ x: 0, y: 0 });
  let hoverTargetNodeId = $state<string | null>(null);
  
  // Execution State
  let isExecuting = $state(false);
  let logs = $state<LogEntry[]>([]);
  let nodeStatus = $state<Record<string, 'idle' | 'running' | 'success' | 'error' | 'retrying'>>({});
  let executionData = $state<Record<string, NodeExecutionData>>({});
  let activeTab = $state<'logs' | 'data' | 'checkpoints'>('logs');
  let isConsoleOpen = $state(true);

  // Checkpoint State
  let checkpoints = $state<Checkpoint[]>([]);
  let currentWorkflowId = $state<string | null>(null);
  let isLoadingCheckpoints = $state(false);

  // AI Panel State (isGenerating and aiPromptText declared later)
  let isAiPanelOpen = $state(false);
  let selectedAiProvider = $state('gemini');
  let isDefaultProvider = $state(true);

  // Interaction State
  let isPanning = false;
  let draggingNodeId: string | null = null;
  let lastMousePos = { x: 0, y: 0 };
  let canvasRef: HTMLDivElement;

  // Templates
  interface Template {
    id: string;
    name: string;
    description: string;
    nodes: NodeData[];
    edges: EdgeData[];
  }

  let templates = $state<Template[]>([
    {
      id: 'research-simple',
      name: 'Simple Research',
      description: 'Query → AI Analysis → Report',
      nodes: [
        { id: '1', label: 'Search Query', type: 'Shell', x: 100, y: 100, config: { cmd: 'curl "https://api.example.com/search?q=topic"' } },
        { id: '2', label: 'AI Analysis', type: 'AiPrompt', x: 450, y: 100, config: { prompt: 'Analyze the research data', model: 'gemini-2.0-flash' }, variables: { 'data': '1.stdout' } },
        { id: '3', label: 'Save Report', type: 'Database', x: 800, y: 100, config: { query: 'INSERT INTO reports (content) VALUES (?)' }, variables: { 'content': '2.response' } }
      ],
      edges: [
        { id: 'e1-2', source: '1', target: '2' },
        { id: 'e2-3', source: '2', target: '3' }
      ]
    },
    {
      id: 'research-parallel',
      name: 'Parallel Research',
      description: 'Multi-source → Synthesis → Save',
      nodes: [
        { id: '1a', label: 'Source A', type: 'Shell', x: 100, y: 80, config: { cmd: 'curl "https://api.source-a.com"' } },
        { id: '1b', label: 'Source B', type: 'Shell', x: 100, y: 200, config: { cmd: 'curl "https://api.source-b.com"' } },
        { id: '2', label: 'Synthesize', type: 'AiPrompt', x: 450, y: 140, config: { prompt: 'Combine and synthesize findings', model: 'gemini-2.0-flash' }, variables: { 'a': '1a.stdout', 'b': '1b.stdout' } },
        { id: '3', label: 'Store Results', type: 'Database', x: 800, y: 140, config: { query: 'INSERT INTO research (findings) VALUES (?)' }, variables: { 'findings': '2.response' } }
      ],
      edges: [
        { id: 'e1a-2', source: '1a', target: '2' },
        { id: 'e1b-2', source: '1b', target: '2' },
        { id: 'e2-3', source: '2', target: '3' }
      ]
    },
    {
      id: 'research-comprehensive',
      name: 'Comprehensive Analysis',
      description: 'Parallel data gathering → Aggregate → AI Review',
      nodes: [
        // Parallel data sources
        { id: '1', label: 'Academic Papers', type: 'HttpRequest', x: 100, y: 60, config: { url: 'https://api.semantic scholar.org/search', method: 'GET' } },
        { id: '2', label: 'News Articles', type: 'HttpRequest', x: 100, y: 160, config: { url: 'https://newsapi.org/v2/everything', method: 'GET' } },
        { id: '3', label: 'Social Media', type: 'HttpRequest', x: 100, y: 260, config: { url: 'https://api.twitter.com/2/tweets/search', method: 'GET' } },
        { id: '4', label: 'Market Data', type: 'Shell', x: 100, y: 360, config: { cmd: 'curl "https://api.marketdata.com/latest"' } },

        // Aggregator to collect all results
        { id: 'agg', label: 'Gather All Results', type: 'Aggregator', x: 450, y: 210, config: { required_inputs: '4' }, requiredInputs: 4 },

        // AI comprehensive review
        { id: 'review', label: 'Comprehensive Review', type: 'AiPrompt', x: 800, y: 210, config: {
          prompt: 'Analyze all gathered data and provide a comprehensive review with key insights, trends, and recommendations',
          model: 'gemini-2.0-flash'
        }, variables: { 'all_data': 'agg.inputs' } },

        // Save final report
        { id: 'save', label: 'Save Report', type: 'FileWrite', x: 1150, y: 210, config: {
          path: '/reports/comprehensive-analysis.md',
          content: '{{review.response}}'
        } },

        // Notify completion
        { id: 'notify', label: 'Notify Complete', type: 'Notify', x: 1150, y: 330, config: {
          title: 'Analysis Complete',
          message: 'Comprehensive research analysis has been completed'
        } }
      ],
      edges: [
        // All sources feed into aggregator
        { id: 'e1-agg', source: '1', target: 'agg' },
        { id: 'e2-agg', source: '2', target: 'agg' },
        { id: 'e3-agg', source: '3', target: 'agg' },
        { id: 'e4-agg', source: '4', target: 'agg' },
        // Aggregator feeds into AI review
        { id: 'eagg-review', source: 'agg', target: 'review' },
        // Review branches to save and notify
        { id: 'ereview-save', source: 'review', target: 'save' },
        { id: 'ereview-notify', source: 'review', target: 'notify' }
      ]
    }
  ]);

  let aiPromptText = $state('');
  let isGenerating = $state(false);

  // --- Helpers ---
  function screenToWorld(sx: number, sy: number) {
    if (!canvasRef) return { x: 0, y: 0 };
    const rect = canvasRef.getBoundingClientRect();
    return {
      x: (sx - rect.left - view.x) / view.zoom,
      y: (sy - rect.top - view.y) / view.zoom
    };
  }

  function addLog(level: LogEntry['level'], source: string, message: string) {
    logs.push({
      id: Math.random().toString(36),
      timestamp: new Date().toLocaleTimeString(),
      level,
      source,
      message
    });
  }

  // --- Event Handlers ---
  function handleMouseDown(e: MouseEvent, nodeId?: string) {
    if (e.button === 2) return; // Right click
    e.stopPropagation();

    if (nodeId) {
      selectedNodeId = nodeId;
      draggingNodeId = nodeId;
      isPanning = false;
    } else {
      selectedNodeId = null;
      isPanning = true;
      lastMousePos = { x: e.clientX, y: e.clientY };
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (isPanning) {
      const dx = e.clientX - lastMousePos.x;
      const dy = e.clientY - lastMousePos.y;
      view.x += dx;
      view.y += dy;
      lastMousePos = { x: e.clientX, y: e.clientY };
      return;
    }

    const worldPos = screenToWorld(e.clientX, e.clientY);

    if (connectingNodeId) {
      connectionPreviewPos = worldPos;

      // Check if hovering over any node for auto-connect
      hoverTargetNodeId = null;
      for (const node of nodes) {
        if (node.id === connectingNodeId) continue; // Skip source node

        // Check if cursor is within node bounds (with some padding)
        const padding = 10;
        if (
          worldPos.x >= node.x - padding &&
          worldPos.x <= node.x + 192 + padding &&
          worldPos.y >= node.y - padding &&
          worldPos.y <= node.y + 100 + padding
        ) {
          hoverTargetNodeId = node.id;
          // Snap preview to target node's input port
          connectionPreviewPos = { x: node.x, y: node.y + 40 };
          break;
        }
      }
    }

    if (draggingNodeId) {
      const node = nodes.find(n => n.id === draggingNodeId);
      if (node) {
        // Snap to grid (10px)
        node.x = Math.round((worldPos.x - 96) / 10) * 10;
        node.y = Math.round((worldPos.y - 32) / 10) * 10;
      }
    }
  }

  function handleMouseUp() {
    // Complete connection if hovering over a target node
    if (connectingNodeId && hoverTargetNodeId && connectingNodeId !== hoverTargetNodeId) {
      // Prevent duplicates
      if (!edges.some(e => e.source === connectingNodeId && e.target === hoverTargetNodeId)) {
        edges.push({
          id: `e-${Date.now()}`,
          source: connectingNodeId,
          target: hoverTargetNodeId
        });
      }
    }

    draggingNodeId = null;
    connectingNodeId = null;
    hoverTargetNodeId = null;
    isPanning = false;
  }

  function handleWheel(e: WheelEvent) {
    if (!canvasRef) return;
    e.preventDefault();
    const rect = canvasRef.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;
    const worldX = (mouseX - view.x) / view.zoom;
    const worldY = (mouseY - view.y) / view.zoom;
    
    const ZOOM_SPEED = 0.001;
    const newZoom = Math.max(0.1, Math.min(3, view.zoom - e.deltaY * ZOOM_SPEED));
    
    view.x = mouseX - worldX * newZoom;
    view.y = mouseY - worldY * newZoom;
    view.zoom = newZoom;
  }

  function startConnection(nodeId: string, e: MouseEvent) {
    e.stopPropagation();
    connectingNodeId = nodeId;
    connectionPreviewPos = screenToWorld(e.clientX, e.clientY);
  }

  function endConnection(targetId: string, e: MouseEvent) {
    e.stopPropagation();
    if (connectingNodeId && connectingNodeId !== targetId) {
      // Prevent duplicates
      if (!edges.some(e => e.source === connectingNodeId && e.target === targetId)) {
        edges.push({
          id: `e-${Date.now()}`,
          source: connectingNodeId,
          target: targetId
        });
      }
    }
    connectingNodeId = null;
  }

  // --- Template Management ---
  function loadTemplate(template: Template) {
    nodes = JSON.parse(JSON.stringify(template.nodes));
    edges = JSON.parse(JSON.stringify(template.edges));
    selectedNodeId = null;
    addLog('info', 'System', `Loaded template: ${template.name}`);
  }

  function addNodeFromPalette(type: NodeData['type']) {
    const newNode: NodeData = {
      id: Date.now().toString(),
      label: `New ${type}`,
      type,
      x: 200,
      y: 200,
      config: {}
    };
    nodes.push(newNode);
    selectedNodeId = newNode.id;
  }

  async function generateWithAI() {
    if (!aiPromptText.trim() || isGenerating) return;

    isGenerating = true;
    addLog('info', 'AI', 'Generating workflow...');

    // Mock AI generation for now
    await new Promise(r => setTimeout(r, 2000));

    // Create a simple workflow based on the prompt
    const newNodes: NodeData[] = [
      { id: '1', label: 'Fetch Data', type: 'Shell', x: 100, y: 150, config: { cmd: `echo "Processing: ${aiPromptText}"` } },
      { id: '2', label: 'AI Analysis', type: 'AiPrompt', x: 450, y: 150, config: { prompt: aiPromptText, model: 'gemini-2.0-flash' }, variables: { 'input': '1.stdout' } },
      { id: '3', label: 'Save Result', type: 'Database', x: 800, y: 150, config: { query: 'INSERT INTO results (data) VALUES (?)' }, variables: { 'result': '2.response' } }
    ];

    const newEdges: EdgeData[] = [
      { id: 'e1-2', source: '1', target: '2' },
      { id: 'e2-3', source: '2', target: '3' }
    ];

    nodes = newNodes;
    edges = newEdges;

    addLog('success', 'AI', 'Workflow generated successfully');
    isGenerating = false;
    aiPromptText = '';
  }

  // --- Workflow Execution ---
  let progressUnlisten: (() => void) | null = null;

  async function runWorkflow() {
    if (isExecuting) return;

    isExecuting = true;
    logs = [];
    nodeStatus = {};
    executionData = {};
    activeTab = 'logs';
    isConsoleOpen = true;

    try {
      addLog('info', 'System', 'Initializing workflow execution...');

      // Set up progress listener
      progressUnlisten = await onWorkflowProgress((result: NodeExecutionResult) => {
        nodeStatus[result.node_id] = result.status;

        if (result.status === 'running') {
          const node = nodes.find(n => n.id === result.node_id);
          if (node) {
            addLog('info', node.type, `Executing ${node.label}...`);
          }
        } else if (result.status === 'success') {
          const node = nodes.find(n => n.id === result.node_id);
          if (node) {
            addLog('success', node.type, `Completed ${node.label}`);
          }

          executionData[result.node_id] = {
            input: {},
            output: result.output,
            timestamp: new Date(result.start_time).getTime(),
            duration: result.duration_ms || 0
          };
        } else if (result.status === 'error') {
          const node = nodes.find(n => n.id === result.node_id);
          if (node) {
            addLog('error', node.type, `Failed: ${node.label} - ${result.error || 'Unknown error'}`);
          }
        }
      });

      // Create workflow object for Tauri
      const workflow: WorkflowType = {
        id: crypto.randomUUID(),
        name: 'User Workflow',
        description: 'Workflow created in Flight Tracker Pro',
        nodes: nodes.map(n => ({
          id: n.id,
          label: n.label,
          type: n.type,
          x: n.x,
          y: n.y,
          config: n.config,
          variables: n.variables
        })),
        edges: edges.map(e => ({
          id: e.id,
          source: e.source,
          target: e.target
        })),
        metadata: {}
      };

      // Execute workflow via Tauri
      const workflowId = await executeWorkflow(workflow);
      currentWorkflowId = workflowId;
      addLog('success', 'System', `Workflow started with ID: ${workflowId}`);

      // Poll for workflow completion
      let running = true;
      let pollCount = 0;
      const maxPolls = 300; // 5 minutes max (300 * 1000ms)

      while (running && pollCount < maxPolls) {
        await new Promise(r => setTimeout(r, 1000));
        running = await isWorkflowRunning(workflowId);
        pollCount++;
      }

      if (pollCount >= maxPolls) {
        addLog('warn', 'System', 'Workflow timeout - execution may still be running');
      }

      // Load checkpoints after execution
      await loadCheckpoints(workflowId);

      // Check final status of all nodes
      const successCount = Object.values(nodeStatus).filter(s => s === 'success').length;
      const errorCount = Object.values(nodeStatus).filter(s => s === 'error').length;
      const totalNodes = nodes.length;

      if (errorCount > 0) {
        addLog('warn', 'System', `Workflow completed with ${errorCount} error(s) out of ${totalNodes} nodes`);
      } else {
        addLog('success', 'System', `Workflow execution completed: ${successCount}/${totalNodes} nodes succeeded`);
      }
      addLog('info', 'Git', `${checkpoints.length} checkpoints created`);
    } catch (error: any) {
      addLog('error', 'System', `Workflow execution failed: ${error}`);
    } finally {
      isExecuting = false;
      if (progressUnlisten) {
        progressUnlisten();
        progressUnlisten = null;
      }
    }
  }

  async function loadCheckpoints(workflowId: string) {
    if (!workflowId) {
      checkpoints = [];
      return;
    }

    try {
      isLoadingCheckpoints = true;
      checkpoints = await getCheckpointHistory(workflowId);

      if (checkpoints.length > 0) {
        addLog('info', 'Git', `Loaded ${checkpoints.length} checkpoints from git history`);
      } else {
        addLog('info', 'Git', 'No checkpoints found - they will appear as workflow executes');
      }
    } catch (error: any) {
      // Silently handle checkpoint errors - they're not critical
      console.warn('Failed to load checkpoints:', error);
      checkpoints = [];
    } finally {
      isLoadingCheckpoints = false;
    }
  }

  // --- AI Workflow Generation ---
  async function generateWorkflowWithAI() {
    if (!aiPromptText.trim() || isGenerating) return;

    isGenerating = true;
    addLog('info', 'AI', `Generating workflow with ${selectedAiProvider}...`);

    try {
      const workflow = await generateWorkflowAI(aiPromptText, selectedAiProvider);

      // Load generated workflow to canvas
      nodes = workflow.nodes.map(n => ({
        id: n.id,
        label: n.label,
        type: n.type as NodeData['type'],
        x: n.x,
        y: n.y,
        config: n.config,
        comments: n.comments,
        requiredInputs: n.required_inputs,
        variables: n.variables
      }));

      edges = workflow.edges.map(e => ({
        id: e.id,
        source: e.source,
        target: e.target
      }));

      addLog('success', 'AI', `Generated: ${workflow.name} (${nodes.length} nodes, ${edges.length} edges)`);

      // Clear prompt and close panel (user can reopen to generate more)
      aiPromptText = '';

      // Save provider as default if checked
      if (isDefaultProvider) {
        localStorage.setItem('defaultAiProvider', selectedAiProvider);
      }
    } catch (error: any) {
      addLog('error', 'AI', `Generation failed: ${error}`);
    } finally {
      isGenerating = false;
    }
  }

  // --- Save/Export Workflow ---
  async function saveWorkflow() {
    try {
      const filePath = await save({
        defaultPath: 'workflow.json',
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });

      if (!filePath) return; // User cancelled

      const workflow: WorkflowType = {
        id: '',
        name: 'Custom Workflow',
        description: 'Exported workflow',
        nodes: nodes.map(n => ({
          id: n.id,
          label: n.label,
          type: n.type,
          x: n.x,
          y: n.y,
          config: n.config,
          comments: n.comments,
          required_inputs: n.requiredInputs,
          variables: n.variables
        })),
        edges: edges.map(e => ({
          id: e.id,
          source: e.source,
          target: e.target
        }))
      };

      await exportWorkflow(workflow, filePath);
      addLog('success', 'System', `Workflow saved to ${filePath}`);
    } catch (error: any) {
      addLog('error', 'System', `Failed to save workflow: ${error}`);
    }
  }

  // --- Save Current Workflow as Template ---
  let showTemplateSaveDialog = $state(false);
  let templateNameInput = $state('');
  let templateDescriptionInput = $state('');

  function openTemplateSaveDialog() {
    if (nodes.length === 0) {
      addLog('warn', 'System', 'No nodes to save as template');
      return;
    }
    templateNameInput = '';
    templateDescriptionInput = '';
    showTemplateSaveDialog = true;
  }

  function saveAsTemplate() {
    if (!templateNameInput.trim()) {
      addLog('warn', 'System', 'Template name is required');
      return;
    }

    const newTemplate: Template = {
      id: `custom-${Date.now()}`,
      name: templateNameInput.trim(),
      description: templateDescriptionInput.trim() || 'Custom template',
      nodes: nodes.map(n => ({ ...n })),
      edges: edges.map(e => ({ ...e }))
    };

    templates = [...templates, newTemplate];
    addLog('success', 'System', `Template "${templateNameInput}" saved successfully`);

    // Persist to localStorage
    try {
      localStorage.setItem('workflow-templates', JSON.stringify(templates));
    } catch (error: any) {
      addLog('warn', 'System', `Template saved but couldn't persist: ${error}`);
    }

    showTemplateSaveDialog = false;
    templateNameInput = '';
    templateDescriptionInput = '';
  }

  // Load default AI provider and custom templates on mount
  onMount(() => {
    const savedProvider = localStorage.getItem('defaultAiProvider');
    if (savedProvider) {
      selectedAiProvider = savedProvider as 'gemini' | 'deepseek' | 'grok';
    }

    // Load custom templates from localStorage
    try {
      const savedTemplates = localStorage.getItem('workflow-templates');
      if (savedTemplates) {
        const parsed = JSON.parse(savedTemplates);
        // Keep system templates and add saved custom templates
        templates = [...templates, ...parsed.filter((t: Template) => t.id.startsWith('custom-'))];
      }
    } catch (error) {
      console.error('Failed to load custom templates:', error);
    }
  });

  // Clean up listeners
  onMount(() => {
    window.addEventListener('mouseup', handleMouseUp);
  });
  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('mouseup', handleMouseUp);
      if (progressUnlisten) {
        progressUnlisten();
      }
    }
  });

</script>

<div class="workflow-container flex flex-col h-full font-sans {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}">

  <!-- Main Workspace -->
  <div class="flex-1 flex overflow-hidden relative">

    <!-- Left Sidebar: Templates & AI Generator -->
    <div class="workflow-sidebar w-72 border-r flex flex-col z-20 shadow-xl">
      <!-- Header -->
      <div class="workflow-header h-12 px-4 flex items-center justify-between border-b shrink-0">
        <h3 class="font-bold text-sm">Workflow Builder</h3>
        <button class="text-slate-500 hover:text-white p-1">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
      </div>

      <div class="flex-1 overflow-y-auto custom-scrollbar p-4 space-y-6">

        <!-- AI Generator Section -->
        <div class="space-y-3">
          <div class="flex items-center gap-2">
            <svg class="w-4 h-4 text-violet-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            <h4 class="text-xs font-bold text-slate-300 uppercase">AI Generator</h4>
          </div>
          <textarea
            bind:value={aiPromptText}
            placeholder="Describe your workflow... e.g. 'Research climate change papers and summarize findings'"
            rows="4"
            class="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-xs text-slate-300 placeholder-slate-600 outline-none focus:border-violet-500 resize-none"
          ></textarea>
          <button
            onclick={generateWithAI}
            disabled={isGenerating || !aiPromptText.trim()}
            class="action-btn action-btn-generate w-full disabled:opacity-50 disabled:cursor-not-allowed text-xs font-bold px-4 py-2.5 rounded flex items-center justify-center gap-2 transition-all duration-200"
          >
            {#if isGenerating}
              <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              Generating...
            {:else}
              <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 24 24">
                <path d="M13 10V3L4 14h7v7l9-11h-7z" />
              </svg>
              Generate Workflow
            {/if}
          </button>
        </div>

        <!-- Templates Section -->
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <svg class="w-4 h-4 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z" />
              </svg>
              <h4 class="text-xs font-bold text-slate-300 uppercase">Templates</h4>
            </div>
            <button
              onclick={openTemplateSaveDialog}
              disabled={nodes.length === 0}
              class="text-xs font-bold text-cyan-400 hover:text-cyan-300 disabled:opacity-50 disabled:cursor-not-allowed px-2 py-1 rounded border border-cyan-600/30 hover:border-cyan-500 transition-colors"
            >
              SAVE
            </button>
          </div>

          <!-- Custom Templates -->
          {#if templates.filter(t => t.id.startsWith('custom-')).length > 0}
            <div class="space-y-2">
              <p class="text-[10px] font-bold text-slate-500 uppercase">Your Templates</p>
              <div class="space-y-2">
                {#each templates.filter(t => t.id.startsWith('custom-')) as template}
                  <button
                    onclick={() => loadTemplate(template)}
                    class="w-full bg-slate-950 hover:bg-slate-800 border border-slate-800 hover:border-cyan-600 rounded p-3 text-left transition-colors"
                  >
                    <div class="flex items-start justify-between gap-2">
                      <div>
                        <p class="text-xs font-medium text-slate-200">{template.name}</p>
                        <p class="text-[10px] text-slate-500 mt-1">{template.description}</p>
                      </div>
                      <svg class="w-3 h-3 text-slate-600 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                      </svg>
                    </div>
                  </button>
                {/each}
              </div>
            </div>
          {/if}

          <!-- System Templates -->
          <div class="space-y-2">
            <p class="text-[10px] font-bold text-slate-500 uppercase">System Templates</p>
            <div class="space-y-2">
              {#each templates.filter(t => !t.id.startsWith('custom-')) as template}
                <button
                  onclick={() => loadTemplate(template)}
                  class="w-full bg-slate-950 hover:bg-slate-800 border border-slate-800 hover:border-cyan-600 rounded p-3 text-left transition-colors"
                >
                  <div class="flex items-start justify-between gap-2">
                    <div>
                      <p class="text-xs font-medium text-slate-200">{template.name}</p>
                      <p class="text-[10px] text-slate-500 mt-1">{template.description}</p>
                    </div>
                    <svg class="w-3 h-3 text-slate-600 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                    </svg>
                  </div>
                </button>
              {/each}
            </div>
          </div>
        </div>

        <!-- Node Palette -->
        <div class="node-palette space-y-4">
          <div class="flex items-center gap-2">
            <svg class="w-4 h-4 palette-title-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
            </svg>
            <h4 class="palette-title text-xs font-bold uppercase">Add Node</h4>
          </div>

          <!-- Nodes grouped by category with futuristic styling -->
          {#each [
            { name: 'Execution', color: 'violet', glow: 'rgba(139, 92, 246, 0.3)', icon: 'M13 10V3L4 14h7v7l9-11h-7z' },
            { name: 'Data', color: 'blue', glow: 'rgba(59, 130, 246, 0.3)', icon: 'M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7' },
            { name: 'Control Flow', color: 'amber', glow: 'rgba(245, 158, 11, 0.3)', icon: 'M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4' },
            { name: 'Output', color: 'rose', glow: 'rgba(244, 63, 94, 0.3)', icon: 'M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11' }
          ] as cat}
            {@const categoryNodes = Object.entries(NODE_SPECS).filter(([_, spec]) => spec.category === cat.name)}
            {#if categoryNodes.length > 0}
              <div class="category-group category-{cat.color}">
                <div class="category-header flex items-center gap-2 mb-2 pb-2">
                  <svg class="w-3.5 h-3.5 category-icon text-{cat.color}-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path stroke-linecap="round" stroke-linejoin="round" d={cat.icon} />
                  </svg>
                  <span class="category-label text-[10px] font-bold uppercase tracking-wider text-{cat.color}-400">{cat.name}</span>
                  <div class="category-line flex-1 h-px"></div>
                </div>
                <div class="grid grid-cols-2 gap-1.5">
                  {#each categoryNodes as [type, spec]}
                    <button
                      onclick={() => addNodeFromPalette(type as NodeData['type'])}
                      class="node-btn node-btn-{spec.color} rounded-md p-2 text-left transition-all duration-200 group"
                      title={type}
                    >
                      <div class="flex items-center gap-1.5">
                        <div class="node-btn-icon-wrap p-1 rounded">
                          <svg class="w-3 h-3 node-btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d={spec.icon} />
                          </svg>
                        </div>
                        <span class="node-btn-label text-[8px] font-bold uppercase truncate">{type}</span>
                      </div>
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          {/each}
        </div>

      </div>
    </div>

    <!-- Canvas -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      bind:this={canvasRef}
      class="workflow-canvas flex-1 relative overflow-hidden cursor-default outline-none"
      onmousedown={(e) => handleMouseDown(e)}
      onmousemove={handleMouseMove}
      onwheel={handleWheel}
    >
      <!-- Grid Background -->
      <div
        class="workflow-grid absolute inset-0 pointer-events-none opacity-30"
        style:background-size="{20 * view.zoom}px {20 * view.zoom}px"
        style:background-position="{view.x}px {view.y}px"
      ></div>

      <!-- Transform Layer -->
      <div 
        class="absolute top-0 left-0 w-full h-full origin-top-left pointer-events-none"
        style:transform="translate({view.x}px, {view.y}px) scale({view.zoom})"
      >
        <!-- SVG Edges -->
        <svg class="absolute overflow-visible w-full h-full z-0 pointer-events-auto">
          <defs>
            <marker id="arrow" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
              <polygon points="0 0, 10 3.5, 0 7" fill={$theme === 'skynet' ? 'rgba(0,180,255,0.6)' : $theme === 'cyberpunk' ? 'rgba(0,217,255,0.6)' : '#475569'} />
            </marker>
            <marker id="arrow-data" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
              <polygon points="0 0, 10 3.5, 0 7" fill={$theme === 'skynet' ? '#00b4ff' : $theme === 'cyberpunk' ? '#00d9ff' : '#06b6d4'} />
            </marker>
          </defs>

          {#each edges as edge (edge.id)}
            {@const source = nodes.find(n => n.id === edge.source)}
            {@const target = nodes.find(n => n.id === edge.target)}
            {#if source && target}
              {@const startX = source.x + 192}
              {@const startY = source.y + 40}
              {@const endX = target.x}
              {@const endY = target.y + 40}
              {@const path = `M ${startX} ${startY} C ${startX + 50} ${startY}, ${endX - 50} ${endY}, ${endX} ${endY}`}
              {@const isData = target.variables && Object.values(target.variables).some(v => v.startsWith(source.id))}
              
              <path
                d={path}
                stroke={isData
                  ? ($theme === 'skynet' ? '#00b4ff' : $theme === 'cyberpunk' ? '#00d9ff' : '#06b6d4')
                  : ($theme === 'skynet' ? 'rgba(0,180,255,0.5)' : $theme === 'cyberpunk' ? 'rgba(0,217,255,0.5)' : '#475569')}
                stroke-width={isData ? 3 : 2}
                fill="none"
                marker-end={isData ? "url(#arrow-data)" : "url(#arrow)"}
                style:filter={isData
                  ? ($theme === 'skynet' ? 'drop-shadow(0px 0px 5px rgba(0,180,255,0.6))' : $theme === 'cyberpunk' ? 'drop-shadow(0px 0px 5px rgba(0,217,255,0.6))' : 'drop-shadow(0px 0px 3px rgba(6,182,212,0.5))')
                  : ''}
              />
              {#if isData}
                <circle r="3" fill="#cffafe">
                  <animateMotion dur="1.5s" repeatCount="indefinite" path={path} />
                </circle>
              {/if}
            {/if}
          {/each}

          <!-- Connection Preview -->
          {#if connectingNodeId}
            {@const source = nodes.find(n => n.id === connectingNodeId)}
            {#if source}
              {@const startX = source.x + 192}
              {@const startY = source.y + 40}
              {@const path = `M ${startX} ${startY} C ${startX + 50} ${startY}, ${connectionPreviewPos.x - 50} ${connectionPreviewPos.y}, ${connectionPreviewPos.x} ${connectionPreviewPos.y}`}
              {@const strokeColor = hoverTargetNodeId
                ? ($theme === 'skynet' ? '#00b4ff' : $theme === 'cyberpunk' ? '#00d9ff' : '#22d3ee')
                : ($theme === 'skynet' ? '#0080ff' : $theme === 'cyberpunk' ? '#b000ff' : '#6366f1')}
              {@const strokeWidth = hoverTargetNodeId ? 3 : 2}
              <path
                d={path}
                stroke={strokeColor}
                stroke-width={strokeWidth}
                fill="none"
                stroke-dasharray="5,5"
                style:filter={hoverTargetNodeId
                  ? ($theme === 'skynet' ? 'drop-shadow(0px 0px 6px rgba(0,180,255,0.7))' : $theme === 'cyberpunk' ? 'drop-shadow(0px 0px 6px rgba(0,217,255,0.7))' : 'drop-shadow(0px 0px 4px rgba(34,211,238,0.6))')
                  : ''}
              />
            {/if}
          {/if}
        </svg>

        <!-- Nodes -->
        {#each nodes as node (node.id)}
          {@const spec = NODE_SPECS[node.type]}
          {@const status = nodeStatus[node.id] || 'idle'}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="workflow-node absolute z-10 w-48 border rounded-lg cursor-move pointer-events-auto transition-all
              {hoverTargetNodeId === node.id ? 'hover-target' : ''}
              {status === 'running' ? 'status-running' : status === 'success' ? 'status-success' : status === 'error' ? 'status-error' : ''}
              {selectedNodeId === node.id ? 'selected' : ''}
            "
            style:left="{node.x}px"
            style:top="{node.y}px"
            onmousedown={(e) => handleMouseDown(e, node.id)}
          >
            <!-- Header -->
            <div class="workflow-node-header px-3 py-2 rounded-t-lg flex justify-between items-center">
              <div class="flex items-center gap-2">
                <!-- Icon based on type -->
                <svg class="w-3 h-3 workflow-accent" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path stroke-linecap="round" stroke-linejoin="round" d={spec.icon} />
                </svg>
                <span class="text-[10px] font-bold font-mono uppercase workflow-accent">{node.type}</span>
              </div>
              <div class="w-2 h-2 rounded-full transition-all {status === 'success' ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.8)]' : status === 'running' ? 'bg-cyan-400 shadow-[0_0_8px_rgba(34,211,238,0.8)] animate-pulse' : status === 'error' ? 'bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.8)]' : 'bg-slate-600'}"></div>
            </div>
            
            <!-- Body -->
            <div class="p-3">
              <p class="text-sm font-medium text-slate-200 truncate">{node.label}</p>
              {#if node.requiredInputs && node.requiredInputs > 1}
                <div class="mt-1 flex items-center gap-1 text-[9px] text-amber-400 font-medium">
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 14v6m-3-3h6M6 10h2a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v2a2 2 0 002 2zm10 0h2a2 2 0 002-2V6a2 2 0 00-2-2h-2a2 2 0 00-2 2v2a2 2 0 002 2z" />
                  </svg>
                  Waits for {node.requiredInputs} inputs
                </div>
              {/if}
            </div>

            <!-- Ports -->
            <!-- Input -->
            <div
              class="absolute top-10 -left-2 w-4 h-4 rounded-full border-2 transition-all flex items-center justify-center
                {hoverTargetNodeId === node.id ? 'border-cyan-400 bg-cyan-400/20 scale-125 shadow-[0_0_10px_rgba(34,211,238,0.6)]' : 'border-slate-600 bg-slate-950 hover:border-indigo-400 hover:scale-110'}"
              onmouseup={(e) => endConnection(node.id, e)}
            >
              <div class="w-1.5 h-1.5 rounded-full pointer-events-none {hoverTargetNodeId === node.id ? 'bg-cyan-300' : 'bg-slate-500'}"></div>
            </div>
            <!-- Output -->
            <div 
              class="absolute top-10 -right-2 w-4 h-4 rounded-full border-2 border-slate-600 bg-slate-950 hover:border-indigo-400 hover:scale-110 transition-all flex items-center justify-center cursor-crosshair"
              onmousedown={(e) => startConnection(node.id, e)}
            >
              <div class="w-1.5 h-1.5 bg-slate-500 rounded-full pointer-events-none"></div>
            </div>
          </div>
        {/each}
      </div>
    </div>

    <!-- Right Sidebar: Inspector -->
    {#if selectedNodeId}
      {@const node = nodes.find(n => n.id === selectedNodeId)}
      {#if node}
        {@const spec = NODE_SPECS[node.type]}
        {@const parentNodes = nodes.filter(n => edges.some(e => e.source === n.id && e.target === node.id))}
        <div class="inspector-panel w-80 flex flex-col z-20 shadow-xl" transition:slide={{ axis: 'x', duration: 200 }}>
          <!-- Header -->
          <div class="inspector-header h-12 px-4 flex items-center justify-between shrink-0">
            <h3 class="inspector-title font-bold text-sm">Node Inspector</h3>
            <button class="inspector-close-btn" onclick={() => selectedNodeId = null}>
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div class="flex-1 overflow-y-auto custom-scrollbar p-4 space-y-6">
            <!-- Node Header Info -->
            <div>
              <label class="inspector-label block text-[10px] font-bold uppercase mb-1">Label</label>
              <input
                type="text"
                bind:value={node.label}
                class="inspector-input w-full rounded px-3 py-2 text-sm outline-none transition-colors"
              />
              <div class="flex items-center justify-between mt-2">
                <span class="inspector-muted text-[10px] font-mono">ID: {node.id}</span>
                <span class="text-[10px] px-1.5 py-0.5 rounded font-mono bg-{spec.color}-500/20 text-{spec.color}-400">
                  {node.type}
                </span>
              </div>
            </div>

            <!-- Configuration Section -->
            <div class="space-y-4">
              <h4 class="inspector-section-title text-xs font-bold uppercase tracking-wider pb-2">
                Configuration
              </h4>

              <!-- Required Inputs (for Aggregator nodes) -->
              {#if node.type === 'Aggregator' || node.type === 'Merge'}
                <div class="relative group">
                  <label class="inspector-field-label text-xs font-bold mb-1 block">Required Inputs</label>
                  <input
                    type="number"
                    bind:value={node.requiredInputs}
                    min="1"
                    placeholder="Number of inputs to wait for"
                    class="inspector-input w-full rounded px-3 py-2 text-sm outline-none transition-colors"
                  />
                  <p class="inspector-hint text-[10px] mt-1">Node will wait until this many parent nodes complete</p>
                </div>
              {/if}
              {#each spec.config as field}
                <div class="relative group">
                  <div class="flex justify-between items-baseline mb-1">
                    <label class="inspector-field-label text-xs font-bold">{field.label}</label>

                    <!-- Variable Picker for text/textarea fields -->
                    {#if (field.type === 'text' || field.type === 'textarea') && parentNodes.length > 0}
                      <div class="relative group/picker">
                        <button class="inspector-var-btn text-[10px] font-mono px-1.5 rounded">
                          {'{ }'}
                        </button>
                        <div class="inspector-var-dropdown absolute right-0 top-full mt-1 w-48 rounded shadow-xl py-1 z-50 hidden group-hover/picker:block">
                          <div class="inspector-var-header px-2 py-1 text-[10px] uppercase font-bold mb-1">
                            Insert Variable
                          </div>
                          {#each parentNodes as parent}
                            <button
                              onclick={() => {
                                const currentVal = node.config[field.key] || '';
                                node.config[field.key] = currentVal + `{{${parent.id}.stdout}}`;
                              }}
                              class="inspector-var-item w-full text-left px-3 py-1.5 text-xs truncate"
                            >
                              {parent.label} → stdout
                            </button>
                          {/each}
                        </div>
                      </div>
                    {/if}
                  </div>

                  {#if field.type === 'textarea'}
                    <textarea
                      bind:value={node.config[field.key]}
                      rows="4"
                      placeholder="Enter {field.label.toLowerCase()}..."
                      class="inspector-input w-full rounded px-3 py-2 text-xs font-mono outline-none transition-colors resize-none"
                    ></textarea>
                  {:else if field.type === 'select' && 'options' in field && field.options}
                    <select
                      bind:value={node.config[field.key]}
                      class="inspector-input w-full rounded px-3 py-2 text-sm outline-none appearance-none"
                    >
                      <option value="">Select option...</option>
                      {#each field.options as opt}
                        <option value={opt}>{opt}</option>
                      {/each}
                    </select>
                  {:else if field.type === 'number'}
                    <input
                      type="number"
                      bind:value={node.config[field.key]}
                      class="inspector-input w-full rounded px-3 py-2 text-sm outline-none transition-colors"
                    />
                  {:else}
                    <input
                      type="text"
                      bind:value={node.config[field.key]}
                      placeholder="Enter {field.label.toLowerCase()}..."
                      class="inspector-input w-full rounded px-3 py-2 text-sm outline-none transition-colors"
                    />
                  {/if}
                </div>
              {/each}
            </div>

            <!-- Data Links Section -->
            {#if node.variables && Object.keys(node.variables).length > 0}
              <div class="inspector-section pt-4">
                <h4 class="inspector-section-title text-xs font-bold uppercase tracking-wider mb-3">Data Links</h4>
                <div class="space-y-2">
                  {#each Object.entries(node.variables) as [key, val]}
                    <div class="inspector-data-link flex items-center justify-between p-2 rounded">
                      <span class="inspector-muted text-xs font-mono">{key}</span>
                      <div class="flex items-center gap-2">
                        <svg class="w-3 h-3 inspector-muted" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 8l4 4m0 0l-4 4m4-4H3" />
                        </svg>
                        <span class="inspector-accent text-xs font-mono">{val}</span>
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}

            <!-- Parent Nodes Section -->
            {#if parentNodes.length > 0}
              <div class="inspector-section pt-4">
                <h4 class="inspector-section-title text-xs font-bold uppercase tracking-wider mb-3">Dependencies</h4>
                <div class="space-y-2">
                  {#each parentNodes as parent}
                    <div class="inspector-data-link flex items-center gap-2 p-2 rounded">
                      <svg class="w-3 h-3 text-{NODE_SPECS[parent.type].color}-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d={NODE_SPECS[parent.type].icon} />
                      </svg>
                      <span class="inspector-text text-xs truncate">{parent.label}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}

            <!-- Actions -->
            <div class="inspector-section pt-4">
              <button
                onclick={() => {
                  nodes = nodes.filter(n => n.id !== node.id);
                  edges = edges.filter(e => e.source !== node.id && e.target !== node.id);
                  selectedNodeId = null;
                  addLog('warn', 'System', `Deleted node: ${node.label}`);
                }}
                class="inspector-delete-btn w-full text-white text-xs font-bold px-4 py-2 rounded flex items-center justify-center gap-2 transition-colors"
              >
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                </svg>
                Delete Node
              </button>
            </div>
          </div>
        </div>
      {/if}
    {/if}

    <!-- Right Sidebar: AI Workflow Generator -->
    {#if isAiPanelOpen}
      <div class="w-80 bg-slate-900 border-l border-slate-800 flex flex-col overflow-hidden shrink-0">
        <!-- Panel Header -->
        <div class="flex items-center justify-between px-4 h-12 bg-slate-950/50 border-b border-slate-800 shrink-0">
          <div class="flex items-center gap-2">
            <svg class="w-4 h-4 text-violet-400" fill="currentColor" viewBox="0 0 24 24">
              <path d="M13 10V3L4 14h7v7l9-11h-7z"/>
            </svg>
            <h3 class="font-bold text-xs text-white uppercase tracking-wider">AI Generator</h3>
          </div>
          <button
            onclick={() => isAiPanelOpen = false}
            class="text-slate-500 hover:text-white text-xs font-bold"
          >
            ✕
          </button>
        </div>

        <div class="flex-1 overflow-y-auto p-4 space-y-4">
          <!-- Provider Selector -->
          <div>
            <label class="block text-[10px] font-bold text-slate-400 uppercase tracking-wider mb-2">
              AI Provider
            </label>
            <select
              bind:value={selectedAiProvider}
              class="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-xs text-white focus:outline-none focus:ring-2 focus:ring-violet-500"
            >
              <option value="gemini">Gemini 2.5 Pro</option>
              <option value="deepseek">DeepSeek Chat</option>
              <option value="grok">Grok 4 Fast Reasoning</option>
            </select>
          </div>

          <!-- Default Provider Checkbox -->
          <label class="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              bind:checked={isDefaultProvider}
              class="w-4 h-4 bg-slate-950 border-slate-700 rounded text-violet-600 focus:ring-violet-500"
            />
            <span class="text-xs text-slate-300">Set as default provider</span>
          </label>

          <!-- Example Prompts -->
          <div>
            <label class="block text-[10px] font-bold text-slate-400 uppercase tracking-wider mb-2">
              Example Prompts
            </label>
            <div class="flex flex-wrap gap-2">
              <button
                onclick={() => aiPromptText = 'Build a web scraper that fetches data from a website and analyzes it with AI'}
                class="px-2 py-1 text-[10px] bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded border border-slate-700 transition-colors"
              >
                Web Scraper
              </button>
              <button
                onclick={() => aiPromptText = 'Create a data pipeline that reads a file, transforms the data, and saves the result'}
                class="px-2 py-1 text-[10px] bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded border border-slate-700 transition-colors"
              >
                Data Pipeline
              </button>
              <button
                onclick={() => aiPromptText = 'Build a sales website workflow with API calls and database operations'}
                class="px-2 py-1 text-[10px] bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded border border-slate-700 transition-colors"
              >
                Sales Website
              </button>
              <button
                onclick={() => aiPromptText = 'Create a notification system that monitors a database and sends alerts'}
                class="px-2 py-1 text-[10px] bg-slate-800 hover:bg-slate-700 text-slate-300 hover:text-white rounded border border-slate-700 transition-colors"
              >
                Notification System
              </button>
            </div>
          </div>

          <!-- Prompt Input -->
          <div>
            <label class="block text-[10px] font-bold text-slate-400 uppercase tracking-wider mb-2">
              Workflow Description
            </label>
            <textarea
              bind:value={aiPromptText}
              placeholder="Describe the workflow you want to create... (e.g., 'Build a web scraper that fetches product prices and analyzes trends')"
              class="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-xs text-white placeholder-slate-600 focus:outline-none focus:ring-2 focus:ring-violet-500 resize-none font-mono"
              rows="6"
            ></textarea>
          </div>

          <!-- Generate Button -->
          <button
            onclick={generateWorkflowWithAI}
            disabled={!aiPromptText.trim() || isGenerating}
            class="w-full bg-violet-600 hover:bg-violet-500 disabled:bg-slate-700 disabled:cursor-not-allowed text-white text-xs font-bold px-4 py-3 rounded flex items-center justify-center gap-2 transition-colors"
          >
            {#if isGenerating}
              <svg class="animate-spin w-4 h-4" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              Generating...
            {:else}
              <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                <path d="M13 10V3L4 14h7v7l9-11h-7z"/>
              </svg>
              Generate Workflow
            {/if}
          </button>

          <!-- Info Box -->
          <div class="bg-slate-950 border border-slate-800 rounded p-3 space-y-1">
            <div class="flex items-start gap-2">
              <svg class="w-3 h-3 text-blue-400 mt-0.5 shrink-0" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"/>
              </svg>
              <p class="text-[10px] text-slate-400 leading-relaxed">
                The AI will generate a complete workflow with nodes, connections, and configurations based on your description. You can edit it after generation.
              </p>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <!-- Bottom Console -->
  <div class="border-t border-slate-800 bg-slate-950 flex flex-col transition-all duration-300" style:height={isConsoleOpen ? '320px' : '48px'}>
    <div class="flex items-center justify-between px-4 h-12 bg-slate-900/50 border-b border-slate-800 shrink-0">
      <div class="flex items-center gap-4">
        <button onclick={() => isConsoleOpen = !isConsoleOpen} class="text-xs font-mono text-slate-400 hover:text-white flex items-center gap-2">
          <div class="w-2 h-2 rounded-full {isExecuting ? 'bg-amber-500 animate-pulse' : 'bg-slate-600'}"></div>
          CONSOLE {isConsoleOpen ? '▼' : '▲'}
        </button>
        
        {#if isConsoleOpen}
          <div class="flex bg-slate-900 rounded border border-slate-800 p-0.5">
            <button onclick={() => activeTab = 'logs'} class="px-3 py-1 text-[10px] font-bold rounded {activeTab === 'logs' ? 'bg-slate-700 text-white' : 'text-slate-500 hover:text-slate-300'}">LOGS</button>
            <button onclick={() => activeTab = 'data'} class="px-3 py-1 text-[10px] font-bold rounded {activeTab === 'data' ? 'bg-indigo-600 text-white' : 'text-slate-500 hover:text-slate-300'}">DATA</button>
            <button onclick={() => activeTab = 'checkpoints'} class="px-3 py-1 text-[10px] font-bold rounded {activeTab === 'checkpoints' ? 'bg-emerald-600 text-white' : 'text-slate-500 hover:text-slate-300'}">
              GIT
              {#if checkpoints.length > 0}
                <span class="ml-1 bg-emerald-400 text-emerald-950 rounded-full px-1.5 text-[9px]">{checkpoints.length}</span>
              {/if}
            </button>
          </div>
        {/if}
      </div>

      <button onclick={saveWorkflow} disabled={nodes.length === 0} class="console-save-btn disabled:opacity-50 text-xs font-bold px-4 py-1.5 rounded flex items-center gap-2 transition-all duration-200">
        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"/></svg>
        Save
      </button>

      <button onclick={runWorkflow} disabled={isExecuting} class="action-btn action-btn-run disabled:opacity-50 text-xs font-bold px-4 py-1.5 rounded flex items-center gap-2 transition-all duration-200">
        {isExecuting ? 'Running...' : 'Run Workflow'}
        <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
      </button>

      <button
        onclick={() => isAiPanelOpen = !isAiPanelOpen}
        class="action-btn action-btn-ai text-xs font-bold px-4 py-1.5 rounded flex items-center gap-2 transition-all duration-200 {isAiPanelOpen ? 'action-btn-active' : ''}"
      >
        <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 24 24"><path d="M13 10V3L4 14h7v7l9-11h-7z"/></svg>
        AI Generate
      </button>
    </div>

    {#if isConsoleOpen}
      <div class="flex-1 overflow-hidden relative">
        {#if activeTab === 'logs'}
          <div class="h-full overflow-y-auto p-4 font-mono text-xs space-y-1 custom-scrollbar">
            {#if logs.length === 0}
              <div class="text-slate-600 italic">Ready to execute.</div>
            {/if}
            {#each logs as log (log.id)}
              <div class="flex gap-3">
                <span class="text-slate-600">[{log.timestamp}]</span>
                <span class="font-bold w-20 shrink-0 {log.level === 'error' ? 'text-rose-500' : log.level === 'success' ? 'text-emerald-500' : 'text-indigo-400'}">[{log.source}]</span>
                <span class="text-slate-300">{log.message}</span>
              </div>
            {/each}
          </div>
        {:else if activeTab === 'data'}
          <div class="h-full p-4">
            {#if selectedNodeId && executionData[selectedNodeId]}
              <div class="grid grid-cols-2 gap-4 h-full">
                <div class="bg-slate-900 border border-slate-800 rounded p-2 overflow-auto">
                  <div class="text-[10px] font-bold text-slate-500 mb-2">INPUT</div>
                  <pre class="font-mono text-xs text-slate-300">{JSON.stringify(executionData[selectedNodeId].input, null, 2)}</pre>
                </div>
                <div class="bg-slate-900 border border-slate-800 rounded p-2 overflow-auto">
                  <div class="text-[10px] font-bold text-slate-500 mb-2">OUTPUT</div>
                  <pre class="font-mono text-xs text-emerald-400">{JSON.stringify(executionData[selectedNodeId].output, null, 2)}</pre>
                </div>
              </div>
            {:else}
              <div class="h-full flex items-center justify-center text-slate-500">Select a node to view data.</div>
            {/if}
          </div>
        {:else if activeTab === 'checkpoints'}
          <div class="h-full overflow-y-auto p-4 custom-scrollbar">
            {#if isLoadingCheckpoints}
              <div class="flex items-center justify-center h-full text-slate-500">
                <svg class="animate-spin h-5 w-5 mr-2" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                Loading checkpoints...
              </div>
            {:else if checkpoints.length === 0}
              <div class="h-full flex flex-col items-center justify-center text-slate-500 space-y-2">
                <svg class="w-12 h-12 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                <p class="text-sm">No checkpoints yet</p>
                <p class="text-xs">Run a workflow to create git checkpoints</p>
              </div>
            {:else}
              <div class="space-y-3">
                <div class="flex items-center justify-between mb-4">
                  <h3 class="text-sm font-bold text-slate-300">Git Checkpoint History</h3>
                  {#if currentWorkflowId}
                    <span class="text-[9px] font-mono text-slate-500 bg-slate-900 px-2 py-1 rounded">
                      {currentWorkflowId.substring(0, 8)}
                    </span>
                  {/if}
                </div>

                {#each checkpoints as checkpoint, i (checkpoint.commit_hash)}
                  <div class="bg-slate-900 border border-slate-800 rounded-lg p-3 hover:border-emerald-500/30 transition-colors">
                    <div class="flex items-start gap-3">
                      <!-- Timeline dot -->
                      <div class="flex flex-col items-center">
                        <div class="w-3 h-3 rounded-full {i === 0 ? 'bg-emerald-500' : 'bg-slate-600'}"></div>
                        {#if i !== checkpoints.length - 1}
                          <div class="w-0.5 h-full bg-slate-700 mt-1"></div>
                        {/if}
                      </div>

                      <!-- Content -->
                      <div class="flex-1 min-w-0">
                        <div class="flex items-center gap-2 mb-1">
                          <svg class="w-3 h-3 text-emerald-400 shrink-0" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
                          </svg>
                          <p class="text-xs font-medium text-slate-200 truncate">{checkpoint.message}</p>
                        </div>

                        <div class="flex items-center gap-3 text-[10px] text-slate-500">
                          <span class="font-mono">{checkpoint.commit_hash.substring(0, 7)}</span>
                          <span>{new Date(checkpoint.timestamp).toLocaleTimeString()}</span>
                        </div>
                      </div>

                      <!-- Badge -->
                      {#if i === 0}
                        <span class="text-[9px] font-bold text-emerald-400 bg-emerald-950 px-2 py-0.5 rounded">LATEST</span>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Agent Tracker - Floating Monitor -->
  <AgentTracker />

  <!-- Template Save Dialog -->
  {#if showTemplateSaveDialog}
    <div
      class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4"
      onclick={() => showTemplateSaveDialog = false}
    >
      <div
        class="bg-slate-800 rounded-lg shadow-2xl p-6 max-w-md w-full"
        onclick={(e) => e.stopPropagation()}
      >
        <h3 class="text-lg font-bold text-slate-200 mb-4">Save as Template</h3>

        <div class="space-y-4">
          <div>
            <label class="block text-xs font-medium text-slate-300 mb-2">
              Template Name <span class="text-red-400">*</span>
            </label>
            <input
              type="text"
              bind:value={templateNameInput}
              placeholder="e.g., Research Pipeline"
              class="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-slate-300 placeholder-slate-600 outline-none focus:border-cyan-500"
              autofocus
            />
          </div>

          <div>
            <label class="block text-xs font-medium text-slate-300 mb-2">
              Description (optional)
            </label>
            <textarea
              bind:value={templateDescriptionInput}
              placeholder="Brief description of what this template does..."
              rows="3"
              class="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-slate-300 placeholder-slate-600 outline-none focus:border-cyan-500 resize-none"
            ></textarea>
          </div>
        </div>

        <div class="flex gap-3 mt-6">
          <button
            onclick={() => showTemplateSaveDialog = false}
            class="flex-1 bg-slate-700 hover:bg-slate-600 text-white text-sm font-bold px-4 py-2 rounded transition-colors"
          >
            Cancel
          </button>
          <button
            onclick={saveAsTemplate}
            disabled={!templateNameInput.trim()}
            class="flex-1 bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 disabled:cursor-not-allowed text-white text-sm font-bold px-4 py-2 rounded transition-colors"
          >
            Save Template
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* ============================================
     DEFAULT THEME (Dark Slate)
     ============================================ */
  .workflow-container.theme-default {
    background-color: #020617; /* slate-950 */
    color: #e2e8f0; /* slate-200 */
  }

  .theme-default .workflow-sidebar {
    background-color: #0f172a; /* slate-900 */
    border-color: #1e293b; /* slate-800 */
  }

  .theme-default .workflow-header {
    background-color: rgba(30, 41, 59, 0.5); /* slate-800/50 */
    border-color: #1e293b;
  }

  .theme-default .workflow-canvas {
    background-color: #020617;
  }

  .theme-default .workflow-node {
    background-color: #1e293b;
    border-color: #334155;
  }

  .theme-default .workflow-node:hover {
    border-color: #3b82f6;
  }

  .theme-default .workflow-input {
    background-color: #020617;
    border-color: #334155;
    color: #cbd5e1;
  }

  .theme-default .workflow-input:focus {
    border-color: #8b5cf6;
  }

  /* ============================================
     SKYNET THEME (Cyan/Black)
     ============================================ */
  .workflow-container.theme-skynet {
    background-color: #000000;
    color: #d4e6ff;
  }

  .theme-skynet .workflow-sidebar {
    background-color: #000000;
    border-color: rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .workflow-header {
    background-color: rgba(0, 10, 20, 0.8);
    border-color: rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .workflow-header h3 {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .workflow-canvas {
    background-color: #000000;
    background-image:
      radial-gradient(circle at 25% 25%, rgba(0, 180, 255, 0.03) 0%, transparent 50%),
      radial-gradient(circle at 75% 75%, rgba(0, 128, 255, 0.03) 0%, transparent 50%);
  }

  .theme-skynet .workflow-node {
    background: linear-gradient(135deg, #0a0a15, #000510);
    border: 2px solid rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .workflow-node:hover {
    border-color: #00b4ff;
    box-shadow: 0 0 25px rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .workflow-node.selected {
    border-color: #00b4ff;
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.5), inset 0 0 20px rgba(0, 180, 255, 0.1);
  }

  .theme-skynet .workflow-input {
    background-color: #050510;
    border: 2px solid rgba(0, 180, 255, 0.3);
    color: #d4e6ff;
  }

  .theme-skynet .workflow-input:focus {
    border-color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .workflow-btn-primary {
    background: linear-gradient(135deg, #0040ff, #00b4ff);
    border: 2px solid #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.4);
    color: #ffffff;
    text-shadow: 0 0 5px rgba(255, 255, 255, 0.5);
  }

  .theme-skynet .workflow-btn-primary:hover {
    box-shadow: 0 0 25px rgba(0, 180, 255, 0.6);
  }

  .theme-skynet .workflow-accent {
    color: #00b4ff;
    text-shadow: 0 0 5px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .workflow-muted {
    color: #8fb3d9;
  }

  .theme-skynet .workflow-edge {
    stroke: rgba(0, 180, 255, 0.6);
    filter: drop-shadow(0 0 3px rgba(0, 180, 255, 0.5));
  }

  /* ============================================
     CYBERPUNK THEME (Cyan/Pink Neon)
     ============================================ */
  .workflow-container.theme-cyberpunk {
    background-color: #0a0a0a;
    color: #ffffff;
  }

  .theme-cyberpunk .workflow-sidebar {
    background-color: #1a1a1a;
    border-color: rgba(0, 217, 255, 0.3);
    box-shadow: 0 0 20px rgba(0, 217, 255, 0.1);
  }

  .theme-cyberpunk .workflow-header {
    background-color: rgba(15, 15, 15, 0.8);
    border-color: rgba(0, 217, 255, 0.3);
  }

  .theme-cyberpunk .workflow-header h3 {
    color: #00d9ff;
    text-shadow: 0 0 10px rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .workflow-canvas {
    background-color: #0a0a0a;
    background-image:
      radial-gradient(circle at 25% 25%, rgba(0, 217, 255, 0.03) 0%, transparent 50%),
      radial-gradient(circle at 75% 75%, rgba(255, 0, 128, 0.03) 0%, transparent 50%);
  }

  .theme-cyberpunk .workflow-node {
    background: linear-gradient(135deg, #1a1a1a, #0f0f0f);
    border: 2px solid rgba(0, 217, 255, 0.4);
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.2);
  }

  .theme-cyberpunk .workflow-node:hover {
    border-color: #00d9ff;
    box-shadow: 0 0 25px rgba(0, 217, 255, 0.4);
  }

  .theme-cyberpunk .workflow-node.selected {
    border-color: #ff0080;
    box-shadow: 0 0 30px rgba(255, 0, 128, 0.5), inset 0 0 20px rgba(255, 0, 128, 0.1);
  }

  .theme-cyberpunk .workflow-input {
    background-color: #0f0f0f;
    border: 2px solid rgba(0, 217, 255, 0.3);
    color: #ffffff;
  }

  .theme-cyberpunk .workflow-input:focus {
    border-color: #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.3);
  }

  .theme-cyberpunk .workflow-btn-primary {
    background: linear-gradient(135deg, #00d9ff, #b000ff);
    border: 2px solid #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.4);
    color: #ffffff;
    text-shadow: 0 0 5px rgba(255, 255, 255, 0.5);
  }

  .theme-cyberpunk .workflow-btn-primary:hover {
    box-shadow: 0 0 25px rgba(0, 217, 255, 0.6);
  }

  .theme-cyberpunk .workflow-accent {
    color: #00d9ff;
    text-shadow: 0 0 5px rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .workflow-accent-secondary {
    color: #ff0080;
    text-shadow: 0 0 5px rgba(255, 0, 128, 0.5);
  }

  .theme-cyberpunk .workflow-muted {
    color: #a0a0a0;
  }

  .theme-cyberpunk .workflow-edge {
    stroke: rgba(0, 217, 255, 0.6);
    filter: drop-shadow(0 0 3px rgba(0, 217, 255, 0.5));
  }

  /* ============================================
     SHARED STYLES
     ============================================ */
  .custom-scrollbar::-webkit-scrollbar { width: 6px; }

  .theme-default .custom-scrollbar::-webkit-scrollbar-track { background: #0f172a; }
  .theme-default .custom-scrollbar::-webkit-scrollbar-thumb { background: #334155; border-radius: 3px; }

  .theme-skynet .custom-scrollbar::-webkit-scrollbar-track { background: #000000; border: 1px solid rgba(0, 180, 255, 0.2); }
  .theme-skynet .custom-scrollbar::-webkit-scrollbar-thumb { background: linear-gradient(180deg, #0080ff, #00b4ff); border-radius: 3px; box-shadow: 0 0 5px rgba(0, 180, 255, 0.5); }

  .theme-cyberpunk .custom-scrollbar::-webkit-scrollbar-track { background: #0a0a0a; border: 1px solid rgba(0, 217, 255, 0.2); }
  .theme-cyberpunk .custom-scrollbar::-webkit-scrollbar-thumb { background: linear-gradient(180deg, #00d9ff, #b000ff); border-radius: 3px; box-shadow: 0 0 5px rgba(0, 217, 255, 0.5); }

  /* Grid pattern for canvas */
  .workflow-grid {
    background-size: 20px 20px;
  }

  .theme-default .workflow-grid {
    background-image:
      linear-gradient(to right, rgba(51, 65, 85, 0.3) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(51, 65, 85, 0.3) 1px, transparent 1px);
  }

  .theme-skynet .workflow-grid {
    background-image:
      linear-gradient(to right, rgba(0, 180, 255, 0.1) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(0, 180, 255, 0.1) 1px, transparent 1px);
  }

  .theme-cyberpunk .workflow-grid {
    background-image:
      linear-gradient(to right, rgba(0, 217, 255, 0.1) 1px, transparent 1px),
      linear-gradient(to bottom, rgba(0, 217, 255, 0.1) 1px, transparent 1px);
  }

  /* Node status styles */
  .workflow-node.hover-target {
    transform: scale(1.02);
  }

  .theme-default .workflow-node.hover-target {
    border-color: #22d3ee;
    box-shadow: 0 0 20px rgba(34, 211, 238, 0.4);
  }

  .theme-skynet .workflow-node.hover-target {
    border-color: #00b4ff;
    box-shadow: 0 0 25px rgba(0, 180, 255, 0.5);
  }

  .theme-cyberpunk .workflow-node.hover-target {
    border-color: #00d9ff;
    box-shadow: 0 0 25px rgba(0, 217, 255, 0.5);
  }

  /* Running state - Cyan glow with pulse animation */
  .workflow-node.status-running {
    animation: node-pulse 1.5s ease-in-out infinite;
  }

  .theme-default .workflow-node.status-running {
    border-color: #22d3ee;
    box-shadow: 0 0 20px rgba(34, 211, 238, 0.5), 0 0 40px rgba(34, 211, 238, 0.2);
    background: linear-gradient(135deg, rgba(34, 211, 238, 0.1), transparent);
  }

  .theme-skynet .workflow-node.status-running {
    border-color: #00b4ff;
    box-shadow: 0 0 25px rgba(0, 180, 255, 0.6), 0 0 50px rgba(0, 180, 255, 0.3);
    background: linear-gradient(135deg, rgba(0, 180, 255, 0.15), transparent);
  }

  .theme-cyberpunk .workflow-node.status-running {
    border-color: #00d9ff;
    box-shadow: 0 0 25px rgba(0, 217, 255, 0.6), 0 0 50px rgba(0, 217, 255, 0.3);
    background: linear-gradient(135deg, rgba(0, 217, 255, 0.15), transparent);
  }

  /* Success state - Green glow */
  .workflow-node.status-success {
    animation: success-glow 0.5s ease-out forwards;
  }

  .theme-default .workflow-node.status-success {
    border-color: #10b981;
    box-shadow: 0 0 20px rgba(16, 185, 129, 0.5), 0 0 40px rgba(16, 185, 129, 0.2);
    background: linear-gradient(135deg, rgba(16, 185, 129, 0.1), transparent);
  }

  .theme-skynet .workflow-node.status-success {
    border-color: #34d399;
    box-shadow: 0 0 25px rgba(52, 211, 153, 0.5), 0 0 50px rgba(52, 211, 153, 0.25);
    background: linear-gradient(135deg, rgba(52, 211, 153, 0.1), transparent);
  }

  .theme-cyberpunk .workflow-node.status-success {
    border-color: #00ff88;
    box-shadow: 0 0 25px rgba(0, 255, 136, 0.6), 0 0 50px rgba(0, 255, 136, 0.3);
    background: linear-gradient(135deg, rgba(0, 255, 136, 0.15), transparent);
  }

  /* Error state - Red glow */
  .workflow-node.status-error {
    animation: error-shake 0.5s ease-out;
  }

  .theme-default .workflow-node.status-error {
    border-color: #ef4444;
    box-shadow: 0 0 20px rgba(239, 68, 68, 0.5), 0 0 40px rgba(239, 68, 68, 0.2);
    background: linear-gradient(135deg, rgba(239, 68, 68, 0.1), transparent);
  }

  .theme-skynet .workflow-node.status-error {
    border-color: #f87171;
    box-shadow: 0 0 25px rgba(248, 113, 113, 0.5), 0 0 50px rgba(248, 113, 113, 0.25);
    background: linear-gradient(135deg, rgba(248, 113, 113, 0.1), transparent);
  }

  .theme-cyberpunk .workflow-node.status-error {
    border-color: #ff0040;
    box-shadow: 0 0 25px rgba(255, 0, 64, 0.6), 0 0 50px rgba(255, 0, 64, 0.3);
    background: linear-gradient(135deg, rgba(255, 0, 64, 0.15), transparent);
  }

  /* Node header theming */
  .workflow-node-header {
    border-bottom: 1px solid;
  }

  .theme-default .workflow-node-header {
    background-color: rgba(30, 41, 59, 0.5);
    border-color: #1e293b;
  }

  .theme-skynet .workflow-node-header {
    background-color: rgba(0, 20, 40, 0.6);
    border-color: rgba(0, 180, 255, 0.2);
  }

  .theme-cyberpunk .workflow-node-header {
    background-color: rgba(15, 15, 15, 0.6);
    border-color: rgba(0, 217, 255, 0.2);
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }

  @keyframes node-pulse {
    0%, 100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.85;
      transform: scale(1.02);
    }
  }

  @keyframes success-glow {
    0% {
      transform: scale(1);
    }
    50% {
      transform: scale(1.03);
    }
    100% {
      transform: scale(1);
    }
  }

  @keyframes error-shake {
    0%, 100% { transform: translateX(0); }
    10%, 30%, 50%, 70%, 90% { transform: translateX(-3px); }
    20%, 40%, 60%, 80% { transform: translateX(3px); }
  }

  /* ============================================
     INSPECTOR PANEL STYLES
     ============================================ */

  /* Default Theme Inspector */
  .theme-default .inspector-panel {
    background-color: #0f172a;
    border-left: 1px solid #1e293b;
  }

  .theme-default .inspector-header {
    background-color: rgba(30, 41, 59, 0.5);
    border-bottom: 1px solid #1e293b;
  }

  .theme-default .inspector-title { color: #e2e8f0; }
  .theme-default .inspector-close-btn { color: #64748b; }
  .theme-default .inspector-close-btn:hover { color: #ffffff; }
  .theme-default .inspector-label { color: #64748b; }
  .theme-default .inspector-field-label { color: #cbd5e1; }
  .theme-default .inspector-muted { color: #64748b; }
  .theme-default .inspector-hint { color: #64748b; }
  .theme-default .inspector-text { color: #cbd5e1; }
  .theme-default .inspector-accent { color: #22d3ee; }

  .theme-default .inspector-input {
    background-color: #020617;
    border: 1px solid #334155;
    color: #e2e8f0;
  }

  .theme-default .inspector-input:focus {
    border-color: #6366f1;
    box-shadow: 0 0 0 1px #6366f1;
  }

  .theme-default .inspector-section { border-top: 1px solid #1e293b; }
  .theme-default .inspector-section-title { color: #94a3b8; border-bottom: 1px solid #1e293b; }

  .theme-default .inspector-data-link {
    background-color: #020617;
    border: 1px solid #1e293b;
  }

  .theme-default .inspector-var-btn {
    color: #6366f1;
    background-color: rgba(99, 102, 241, 0.1);
  }

  .theme-default .inspector-var-btn:hover { color: #818cf8; }

  .theme-default .inspector-var-dropdown {
    background-color: #1e293b;
    border: 1px solid #334155;
  }

  .theme-default .inspector-var-header {
    color: #64748b;
    border-bottom: 1px solid #334155;
  }

  .theme-default .inspector-var-item { color: #cbd5e1; }
  .theme-default .inspector-var-item:hover { background-color: #6366f1; color: #ffffff; }

  .theme-default .inspector-delete-btn { background-color: #e11d48; }
  .theme-default .inspector-delete-btn:hover { background-color: #f43f5e; }

  /* Skynet Theme Inspector */
  .theme-skynet .inspector-panel {
    background-color: #000000;
    border-left: 1px solid rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .inspector-header {
    background: linear-gradient(180deg, rgba(0, 40, 80, 0.6), rgba(0, 20, 40, 0.4));
    border-bottom: 1px solid rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .inspector-title { color: #00b4ff; text-shadow: 0 0 10px rgba(0, 180, 255, 0.5); }
  .theme-skynet .inspector-close-btn { color: #0080ff; }
  .theme-skynet .inspector-close-btn:hover { color: #00b4ff; }
  .theme-skynet .inspector-label { color: #0080ff; }
  .theme-skynet .inspector-field-label { color: #00b4ff; }
  .theme-skynet .inspector-muted { color: #005299; }
  .theme-skynet .inspector-hint { color: #005299; }
  .theme-skynet .inspector-text { color: #8fb3d9; }
  .theme-skynet .inspector-accent { color: #00b4ff; text-shadow: 0 0 5px rgba(0, 180, 255, 0.5); }

  .theme-skynet .inspector-input {
    background-color: rgba(0, 10, 20, 0.8);
    border: 1px solid rgba(0, 180, 255, 0.3);
    color: #d4e6ff;
  }

  .theme-skynet .inspector-input:focus {
    border-color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .inspector-section { border-top: 1px solid rgba(0, 180, 255, 0.2); }
  .theme-skynet .inspector-section-title { color: #0080ff; border-bottom: 1px solid rgba(0, 180, 255, 0.2); }

  .theme-skynet .inspector-data-link {
    background-color: rgba(0, 40, 80, 0.3);
    border: 1px solid rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .inspector-var-btn {
    color: #00b4ff;
    background-color: rgba(0, 180, 255, 0.1);
  }

  .theme-skynet .inspector-var-dropdown {
    background-color: rgba(0, 20, 40, 0.95);
    border: 1px solid rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .inspector-var-header { color: #0080ff; border-bottom: 1px solid rgba(0, 180, 255, 0.3); }
  .theme-skynet .inspector-var-item { color: #8fb3d9; }
  .theme-skynet .inspector-var-item:hover { background-color: #0080ff; color: #ffffff; }

  .theme-skynet .inspector-delete-btn {
    background: linear-gradient(135deg, #cc0000, #ff3333);
    border: 1px solid #ff4444;
  }

  /* Cyberpunk Theme Inspector */
  .theme-cyberpunk .inspector-panel {
    background-color: #0a0a0a;
    border-left: 1px solid rgba(0, 217, 255, 0.4);
    box-shadow: 0 0 30px rgba(0, 217, 255, 0.2), 0 0 60px rgba(255, 0, 128, 0.1);
  }

  .theme-cyberpunk .inspector-header {
    background: linear-gradient(180deg, rgba(20, 20, 20, 0.8), rgba(10, 10, 10, 0.6));
    border-bottom: 1px solid rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .inspector-title { color: #00d9ff; text-shadow: 0 0 10px rgba(0, 217, 255, 0.5); }
  .theme-cyberpunk .inspector-close-btn { color: #ff0080; }
  .theme-cyberpunk .inspector-close-btn:hover { color: #00d9ff; }
  .theme-cyberpunk .inspector-label { color: #b000ff; }
  .theme-cyberpunk .inspector-field-label { color: #00d9ff; }
  .theme-cyberpunk .inspector-muted { color: #b000ff; }
  .theme-cyberpunk .inspector-hint { color: #b000ff; }
  .theme-cyberpunk .inspector-text { color: #ffffff; }
  .theme-cyberpunk .inspector-accent { color: #ff0080; text-shadow: 0 0 5px rgba(255, 0, 128, 0.5); }

  .theme-cyberpunk .inspector-input {
    background-color: rgba(10, 10, 10, 0.9);
    border: 1px solid rgba(0, 217, 255, 0.3);
    color: #ffffff;
  }

  .theme-cyberpunk .inspector-input:focus {
    border-color: #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.4);
  }

  .theme-cyberpunk .inspector-section { border-top: 1px solid rgba(255, 0, 128, 0.2); }
  .theme-cyberpunk .inspector-section-title { color: #ff0080; border-bottom: 1px solid rgba(255, 0, 128, 0.2); }

  .theme-cyberpunk .inspector-data-link {
    background-color: rgba(20, 20, 20, 0.5);
    border: 1px solid rgba(176, 0, 255, 0.3);
  }

  .theme-cyberpunk .inspector-var-btn {
    color: #00d9ff;
    background-color: rgba(0, 217, 255, 0.1);
  }

  .theme-cyberpunk .inspector-var-dropdown {
    background-color: rgba(15, 15, 15, 0.95);
    border: 1px solid rgba(0, 217, 255, 0.4);
  }

  .theme-cyberpunk .inspector-var-header { color: #ff0080; border-bottom: 1px solid rgba(255, 0, 128, 0.3); }
  .theme-cyberpunk .inspector-var-item { color: #ffffff; }
  .theme-cyberpunk .inspector-var-item:hover { background: linear-gradient(90deg, #00d9ff, #ff0080); color: #000000; }

  .theme-cyberpunk .inspector-delete-btn {
    background: linear-gradient(135deg, #ff0080, #b000ff);
    border: 1px solid #ff0080;
  }

  /* ============================================
     NODE PALETTE STYLES (Futuristic)
     ============================================ */

  .theme-default .palette-title-icon { color: #10b981; }
  .theme-default .palette-title { color: #cbd5e1; }

  .theme-default .category-group { padding: 0.75rem; background: rgba(15, 23, 42, 0.5); border-radius: 0.5rem; border: 1px solid #1e293b; }
  .theme-default .category-header { border-bottom: 1px solid #1e293b; }
  .theme-default .category-line { background: linear-gradient(90deg, #334155, transparent); }

  .theme-default .node-btn {
    background: #020617;
    border: 1px solid #1e293b;
  }

  .theme-default .node-btn:hover {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .theme-default .node-btn-icon-wrap { background: rgba(255, 255, 255, 0.05); }
  .theme-default .node-btn-label { color: #94a3b8; }
  .theme-default .node-btn:hover .node-btn-label { color: #e2e8f0; }

  /* Skynet Theme Palette */
  .theme-skynet .palette-title-icon { color: #00b4ff; }
  .theme-skynet .palette-title { color: #00b4ff; text-shadow: 0 0 5px rgba(0, 180, 255, 0.3); }

  .theme-skynet .category-group {
    padding: 0.75rem;
    background: linear-gradient(135deg, rgba(0, 20, 40, 0.5), rgba(0, 10, 20, 0.4));
    border-radius: 0.5rem;
    border: 1px solid rgba(0, 180, 255, 0.25);
    box-shadow: inset 0 0 20px rgba(0, 180, 255, 0.05);
  }

  .theme-skynet .category-header { border-bottom: 1px solid rgba(0, 180, 255, 0.2); }
  .theme-skynet .category-line { background: linear-gradient(90deg, rgba(0, 180, 255, 0.4), transparent); }

  .theme-skynet .node-btn {
    background: rgba(0, 10, 20, 0.8);
    border: 1px solid rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .node-btn:hover {
    border-color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3), inset 0 0 10px rgba(0, 180, 255, 0.1);
    transform: translateY(-1px);
  }

  .theme-skynet .node-btn-icon-wrap { background: rgba(0, 180, 255, 0.1); }
  .theme-skynet .node-btn-icon { color: #00b4ff; }
  .theme-skynet .node-btn-label { color: #0080ff; }
  .theme-skynet .node-btn:hover .node-btn-label { color: #00b4ff; text-shadow: 0 0 5px rgba(0, 180, 255, 0.5); }

  /* Cyberpunk Theme Palette */
  .theme-cyberpunk .palette-title-icon { color: #00d9ff; }
  .theme-cyberpunk .palette-title { color: #00d9ff; text-shadow: 0 0 5px rgba(0, 217, 255, 0.3); }

  .theme-cyberpunk .category-group {
    padding: 0.75rem;
    background: linear-gradient(135deg, rgba(20, 20, 20, 0.8), rgba(10, 10, 10, 0.6));
    border-radius: 0.5rem;
    border: 1px solid rgba(0, 217, 255, 0.2);
    box-shadow: inset 0 0 20px rgba(255, 0, 128, 0.03);
  }

  .theme-cyberpunk .category-header { border-bottom: 1px solid rgba(255, 0, 128, 0.2); }
  .theme-cyberpunk .category-line { background: linear-gradient(90deg, rgba(255, 0, 128, 0.4), transparent); }

  .theme-cyberpunk .node-btn {
    background: rgba(15, 15, 15, 0.8);
    border: 1px solid rgba(176, 0, 255, 0.2);
  }

  .theme-cyberpunk .node-btn:hover {
    border-color: #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.3), 0 0 30px rgba(255, 0, 128, 0.1);
    transform: translateY(-1px);
  }

  .theme-cyberpunk .node-btn-icon-wrap { background: rgba(255, 0, 128, 0.1); }
  .theme-cyberpunk .node-btn-icon { color: #ff0080; }
  .theme-cyberpunk .node-btn-label { color: #b000ff; }
  .theme-cyberpunk .node-btn:hover .node-btn-label { color: #00d9ff; text-shadow: 0 0 5px rgba(0, 217, 255, 0.5); }

  /* ============================================
     ACTION BUTTONS (Cyan with dark pressed)
     ============================================ */

  /* Default theme action buttons */
  .theme-default .action-btn-generate,
  .theme-default .action-btn-run,
  .theme-default .action-btn-ai {
    background: linear-gradient(135deg, #0891b2, #06b6d4);
    border: 1px solid #22d3ee;
    color: #ffffff;
    box-shadow: 0 2px 8px rgba(6, 182, 212, 0.3);
  }

  .theme-default .action-btn-generate:hover,
  .theme-default .action-btn-run:hover,
  .theme-default .action-btn-ai:hover {
    background: linear-gradient(135deg, #06b6d4, #22d3ee);
    box-shadow: 0 4px 15px rgba(6, 182, 212, 0.4);
  }

  .theme-default .action-btn-generate:active,
  .theme-default .action-btn-run:active,
  .theme-default .action-btn-ai:active,
  .theme-default .action-btn-active {
    background: linear-gradient(135deg, #0e7490, #0891b2);
    border-color: #06b6d4;
    box-shadow: inset 0 2px 8px rgba(0, 0, 0, 0.3), 0 0 15px rgba(6, 182, 212, 0.3);
  }

  .theme-default .console-save-btn {
    background: #4f46e5;
    border: 1px solid #6366f1;
    color: #ffffff;
  }

  .theme-default .console-save-btn:hover { background: #6366f1; }

  /* Skynet theme action buttons */
  .theme-skynet .action-btn-generate,
  .theme-skynet .action-btn-run,
  .theme-skynet .action-btn-ai {
    background: linear-gradient(135deg, #0040ff, #00b4ff);
    border: 2px solid #00b4ff;
    color: #ffffff;
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.4);
    text-shadow: 0 0 5px rgba(255, 255, 255, 0.3);
  }

  .theme-skynet .action-btn-generate:hover,
  .theme-skynet .action-btn-run:hover,
  .theme-skynet .action-btn-ai:hover {
    background: linear-gradient(135deg, #0060ff, #00d4ff);
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.6);
  }

  .theme-skynet .action-btn-generate:active,
  .theme-skynet .action-btn-run:active,
  .theme-skynet .action-btn-ai:active,
  .theme-skynet .action-btn-active {
    background: linear-gradient(135deg, #001a4d, #003366);
    border-color: #00b4ff;
    box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.5), 0 0 20px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .console-save-btn {
    background: linear-gradient(135deg, #003366, #0060aa);
    border: 1px solid #0080ff;
    color: #00b4ff;
  }

  .theme-skynet .console-save-btn:hover { box-shadow: 0 0 15px rgba(0, 180, 255, 0.3); }

  /* Cyberpunk theme action buttons */
  .theme-cyberpunk .action-btn-generate,
  .theme-cyberpunk .action-btn-run,
  .theme-cyberpunk .action-btn-ai {
    background: linear-gradient(135deg, #00a0cc, #00d9ff);
    border: 2px solid #00d9ff;
    color: #000000;
    font-weight: 800;
    box-shadow: 0 0 20px rgba(0, 217, 255, 0.4), 0 0 40px rgba(255, 0, 128, 0.1);
    text-shadow: none;
  }

  .theme-cyberpunk .action-btn-generate:hover,
  .theme-cyberpunk .action-btn-run:hover,
  .theme-cyberpunk .action-btn-ai:hover {
    background: linear-gradient(135deg, #00d9ff, #00ffff);
    box-shadow: 0 0 30px rgba(0, 217, 255, 0.6), 0 0 60px rgba(255, 0, 128, 0.2);
  }

  .theme-cyberpunk .action-btn-generate:active,
  .theme-cyberpunk .action-btn-run:active,
  .theme-cyberpunk .action-btn-ai:active,
  .theme-cyberpunk .action-btn-active {
    background: linear-gradient(135deg, #003344, #005566);
    border-color: #00d9ff;
    color: #00d9ff;
    box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.5), 0 0 20px rgba(0, 217, 255, 0.3);
  }

  .theme-cyberpunk .console-save-btn {
    background: linear-gradient(135deg, #660040, #ff0080);
    border: 1px solid #ff0080;
    color: #ffffff;
  }

  .theme-cyberpunk .console-save-btn:hover { box-shadow: 0 0 15px rgba(255, 0, 128, 0.4); }
</style>
