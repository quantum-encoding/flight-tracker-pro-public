<script lang="ts">
import { onMount } from 'svelte';
import type {  NodeData,
  EdgeData,
  ViewState,
  HistoryState,
  ExecutionStatus,
  LogEntry,
  NodeType
} from '$lib/types/workflow';
import { NODE_SPECS, NODE_CATEGORIES } from '$lib/utils/nodeSpecs';
import { detectCycle, topologicalSort } from '$lib/utils/cycleDetection';
import { validateWorkflow, executeWorkflow, cancelWorkflow } from '$lib/workflow-commands';

// Reactive state using Svelte 5 runes
let nodes = $state<NodeData[]>([]);
let edges = $state<EdgeData[]>([]);
let viewState = $state<ViewState>({ x: 0, y: 0, zoom: 1 });
let history = $state<HistoryState[]>([]);
let historyIndex = $state<number>(-1);

// Execution state
let isExecuting = $state(false);
let executionLogs = $state<LogEntry[]>([]);
let nodeStatuses = $state<Record<string, ExecutionStatus>>({});

// UI state
let selectedNode = $state<NodeData | null>(null);
let contextMenu = $state<{ x: number; y: number; show: boolean }>({ x: 0, y: 0, show: false });
let notification = $state<{ message: string; type: 'error' | 'success' | 'info' } | null>(null);
let dragState = $state<{
  type: 'node' | 'edge' | 'canvas' | null;
  nodeId?: string;
  sourceId?: string;
  startX?: number;
  startY?: number;
  currentX?: number;
  currentY?: number;
}>({ type: null });

// Canvas ref
let canvasEl: SVGSVGElement;

// Derived values
const workflow = $derived({
  id: 'current-workflow',
  name: 'Untitled Workflow',
  nodes: nodes as any[],
  edges: edges as any[]
});

const canUndo = $derived(historyIndex > 0);
const canRedo = $derived(historyIndex < history.length - 1);

// Generate unique ID
function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

// Add to history
function pushHistory() {
  const newHistory = history.slice(0, historyIndex + 1);
  newHistory.push({ nodes: JSON.parse(JSON.stringify(nodes)), edges: JSON.parse(JSON.stringify(edges)) });
  history = newHistory;
  historyIndex = newHistory.length - 1;
}

// Undo/Redo
function undo() {
  if (canUndo) {
    historyIndex--;
    const state = history[historyIndex];
    nodes = JSON.parse(JSON.stringify(state.nodes));
    edges = JSON.parse(JSON.stringify(state.edges));
  }
}

function redo() {
  if (canRedo) {
    historyIndex++;
    const state = history[historyIndex];
    nodes = JSON.parse(JSON.stringify(state.nodes));
    edges = JSON.parse(JSON.stringify(state.edges));
  }
}

// Node operations
function addNode(type: NodeType, x: number, y: number) {
  const id = generateId();
  const newNode: NodeData = {
    id,
    label: `${type} ${nodes.length + 1}`,
    type,
    x,
    y,
    config: {}
  };

  nodes = [...nodes, newNode];
  nodeStatuses = { ...nodeStatuses, [id]: 'idle' };
  pushHistory();

  showNotification(`Added ${type} node`, 'success');
}

function deleteNode(nodeId: string) {
  nodes = nodes.filter(n => n.id !== nodeId);
  edges = edges.filter(e => e.source !== nodeId && e.target !== nodeId);
  delete nodeStatuses[nodeId];
  if (selectedNode?.id === nodeId) selectedNode = null;
  pushHistory();

  showNotification('Node deleted', 'info');
}

function updateNodePosition(nodeId: string, x: number, y: number) {
  nodes = nodes.map(n => n.id === nodeId ? { ...n, x, y } : n);
}

// Edge operations
function addEdge(sourceId: string, targetId: string) {
  if (sourceId === targetId) {
    showNotification('Cannot connect node to itself', 'error');
    return;
  }

  const edgeExists = edges.some(e => e.source === sourceId && e.target === targetId);
  if (edgeExists) {
    showNotification('Edge already exists', 'error');
    return;
  }

  const newEdge: EdgeData = {
    id: generateId(),
    source: sourceId,
    target: targetId
  };

  if (detectCycle(nodes, edges, newEdge)) {
    showNotification('Cannot create edge: would form a cycle', 'error');
    return;
  }

  edges = [...edges, newEdge];
  pushHistory();
  showNotification('Edge created', 'success');
}

function deleteEdge(edgeId: string) {
  edges = edges.filter(e => e.id !== edgeId);
  pushHistory();
  showNotification('Edge deleted', 'info');
}

// Mouse event handlers
function handleCanvasMouseDown(e: MouseEvent) {
  if (e.target === canvasEl || (e.target as Element).classList.contains('canvas-bg')) {
    dragState = { type: 'canvas', startX: e.clientX, startY: e.clientY };
  }
}

function handleNodeMouseDown(e: MouseEvent, nodeId: string) {
  e.stopPropagation();
  const node = nodes.find(n => n.id === nodeId);
  if (!node) return;

  selectedNode = node;
  dragState = {
    type: 'node',
    nodeId,
    startX: e.clientX - node.x * viewState.zoom,
    startY: e.clientY - node.y * viewState.zoom
  };
}

function handleOutputPortMouseDown(e: MouseEvent, nodeId: string) {
  e.stopPropagation();
  dragState = {
    type: 'edge',
    sourceId: nodeId,
    currentX: e.clientX,
    currentY: e.clientY
  };
}

function handleMouseMove(e: MouseEvent) {
  if (dragState.type === 'node' && dragState.nodeId && dragState.startX !== undefined && dragState.startY !== undefined) {
    const newX = (e.clientX - dragState.startX) / viewState.zoom;
    const newY = (e.clientY - dragState.startY) / viewState.zoom;
    updateNodePosition(dragState.nodeId, Math.round(newX / 10) * 10, Math.round(newY / 10) * 10);
  } else if (dragState.type === 'edge') {
    dragState = { ...dragState, currentX: e.clientX, currentY: e.clientY };
  } else if (dragState.type === 'canvas' && dragState.startX !== undefined && dragState.startY !== undefined) {
    const dx = e.clientX - dragState.startX;
    const dy = e.clientY - dragState.startY;
    viewState = {
      ...viewState,
      x: viewState.x + dx,
      y: viewState.y + dy
    };
    dragState = { ...dragState, startX: e.clientX, startY: e.clientY };
  }
}

function handleMouseUp(e: MouseEvent) {
  if (dragState.type === 'edge' && dragState.sourceId) {
    const target = e.target as Element;
    const inputPort = target.closest('[data-input-port]');
    if (inputPort) {
      const targetNodeId = inputPort.getAttribute('data-node-id');
      if (targetNodeId) {
        addEdge(dragState.sourceId, targetNodeId);
      }
    }
  } else if (dragState.type === 'node') {
    pushHistory();
  }

  dragState = { type: null };
}

// Context menu
function handleContextMenu(e: MouseEvent) {
  e.preventDefault();
  const target = e.target as Element;

  if (target.closest('[data-node-id]')) {
    const nodeEl = target.closest('[data-node-id]');
    const nodeId = nodeEl?.getAttribute('data-node-id');
    if (nodeId) {
      contextMenu = { x: e.clientX, y: e.clientY, show: true };
      selectedNode = nodes.find(n => n.id === nodeId) || null;
    }
  } else {
    const rect = canvasEl.getBoundingClientRect();
    const x = (e.clientX - rect.left - viewState.x) / viewState.zoom;
    const y = (e.clientY - rect.top - viewState.y) / viewState.zoom;
    contextMenu = { x: e.clientX, y: e.clientY, show: true };
  }
}

function closeContextMenu() {
  contextMenu = { ...contextMenu, show: false };
}

function handleAddNodeFromMenu(type: NodeType) {
  const rect = canvasEl.getBoundingClientRect();
  const x = (contextMenu.x - rect.left - viewState.x) / viewState.zoom;
  const y = (contextMenu.y - rect.top - viewState.y) / viewState.zoom;
  addNode(type, x, y);
  closeContextMenu();
}

// Edge click with SHIFT to delete
function handleEdgeClick(e: MouseEvent, edgeId: string) {
  if (e.shiftKey) {
    deleteEdge(edgeId);
  }
}

// Zoom
function handleWheel(e: WheelEvent) {
  e.preventDefault();
  const delta = e.deltaY > 0 ? 0.9 : 1.1;
  viewState = { ...viewState, zoom: Math.max(0.1, Math.min(3, viewState.zoom * delta)) };
}

// Notifications
function showNotification(message: string, type: 'error' | 'success' | 'info') {
  notification = { message, type };
  setTimeout(() => notification = null, 3000);
}

// Workflow execution
async function runWorkflow() {
  try {
    const result = await validateWorkflow(workflow);
    if (!result) {
      showNotification(`Invalid workflow`, 'error');
      return;
    }

    isExecuting = true;
    nodes.forEach(n => nodeStatuses[n.id] = 'idle');
    executionLogs = [];

    const execution = await executeWorkflow(workflow);
    showNotification('Workflow execution started', 'success');

    // Poll for status updates
    // In real implementation, use Tauri events for real-time updates
  } catch (error) {
    showNotification(`Execution error: ${error}`, 'error');
  } finally {
    isExecuting = false;
  }
}

async function resetWorkflow() {
  nodes.forEach(n => nodeStatuses[n.id] = 'idle');
  executionLogs = [];
  showNotification('Workflow reset', 'info');
}

// Keyboard shortcuts
function handleKeyDown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'z') {
    e.preventDefault();
    undo();
  } else if ((e.ctrlKey || e.metaKey) && e.key === 'y') {
    e.preventDefault();
    redo();
  }
}

// Lifecycle
onMount(() => {
  document.addEventListener('click', closeContextMenu);
  document.addEventListener('keydown', handleKeyDown);

  return () => {
    document.removeEventListener('click', closeContextMenu);
    document.removeEventListener('keydown', handleKeyDown);
  };
});
</script>

<svelte:window on:mousemove={handleMouseMove} on:mouseup={handleMouseUp} />

<div class="workflow-builder h-full flex flex-col bg-slate-950">
  <!-- Toolbar -->
  <div class="toolbar flex items-center justify-between px-4 py-2 bg-slate-900 border-b border-slate-800">
    <div class="flex items-center gap-2">
      <button
        onclick={undo}
        disabled={!canUndo}
        class="px-3 py-1 rounded bg-slate-800 hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
      >
        Undo
      </button>
      <button
        onclick={redo}
        disabled={!canRedo}
        class="px-3 py-1 rounded bg-slate-800 hover:bg-slate-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm"
      >
        Redo
      </button>
    </div>

    <div class="flex items-center gap-2">
      <button
        onclick={resetWorkflow}
        disabled={isExecuting}
        class="px-4 py-1.5 rounded bg-amber-500/10 border border-amber-500 text-amber-400 hover:bg-amber-500/20 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium"
      >
        Reset
      </button>
      <button
        onclick={runWorkflow}
        disabled={isExecuting}
        class="px-4 py-1.5 rounded bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium"
      >
        {isExecuting ? 'Running...' : 'Run Workflow'}
      </button>
    </div>
  </div>

  <!-- Canvas -->
  <div class="flex-1 relative overflow-hidden">
    <svg
      bind:this={canvasEl}
      class="canvas-bg w-full h-full"
      onmousedown={handleCanvasMouseDown}
      oncontextmenu={handleContextMenu}
      onwheel={handleWheel}
    >
      <g transform="translate({viewState.x}, {viewState.y}) scale({viewState.zoom})">
        <!-- Edges -->
        {#each edges as edge (edge.id)}
          {@const sourceNode = nodes.find(n => n.id === edge.source)}
          {@const targetNode = nodes.find(n => n.id === edge.target)}
          {#if sourceNode && targetNode}
            <line
              x1={sourceNode.x + 100}
              y1={sourceNode.y + 40}
              x2={targetNode.x}
              y2={targetNode.y + 40}
              stroke="rgb(59, 130, 246)"
              stroke-width="2"
              class="edge cursor-pointer hover:stroke-purple-500"
              onclick={(e) => handleEdgeClick(e, edge.id)}
            />
          {/if}
        {/each}

        <!-- Edge being dragged -->
        {#if dragState.type === 'edge' && dragState.sourceId && dragState.currentX !== undefined && dragState.currentY !== undefined}
          {@const sourceNode = nodes.find(n => n.id === dragState.sourceId)}
          {#if sourceNode}
            <line
              x1={sourceNode.x + 100}
              y1={sourceNode.y + 40}
              x2={(dragState.currentX - viewState.x) / viewState.zoom}
              y2={(dragState.currentY - viewState.y) / viewState.zoom}
              stroke="rgb(59, 130, 246)"
              stroke-width="2"
              stroke-dasharray="4"
              opacity="0.6"
            />
          {/if}
        {/if}

        <!-- Nodes -->
        {#each nodes as node (node.id)}
          <g
            transform="translate({node.x}, {node.y})"
            data-node-id={node.id}
            class="node cursor-move"
            onmousedown={(e) => handleNodeMouseDown(e, node.id)}
          >
            <!-- Node body -->
            <rect
              width="200"
              height="80"
              rx="8"
              class="fill-slate-800 stroke-slate-600"
              class:ring-2={selectedNode?.id === node.id}
              class:ring-indigo-500={selectedNode?.id === node.id}
              stroke-width="1.5"
            />

            <!-- Node label -->
            <text x="100" y="25" text-anchor="middle" class="fill-slate-200 text-sm font-medium pointer-events-none">
              {node.label}
            </text>
            <text x="100" y="45" text-anchor="middle" class="fill-slate-400 text-xs pointer-events-none">
              {node.type}
            </text>

            <!-- Status indicator -->
            <circle
              cx="10"
              cy="10"
              r="4"
              class:fill-slate-600={nodeStatuses[node.id] === 'idle'}
              class:fill-blue-500={nodeStatuses[node.id] === 'running'}
              class:fill-green-500={nodeStatuses[node.id] === 'success'}
              class:fill-red-500={nodeStatuses[node.id] === 'error'}
            />

            <!-- Input port -->
            <circle
              cx="0"
              cy="40"
              r="6"
              class="fill-slate-700 stroke-slate-500 hover:fill-green-500 cursor-pointer"
              data-input-port="true"
              data-node-id={node.id}
              stroke-width="2"
            />

            <!-- Output port -->
            <circle
              cx="200"
              cy="40"
              r="6"
              class="fill-slate-700 stroke-blue-500 hover:fill-blue-500 cursor-pointer"
              onmousedown={(e) => handleOutputPortMouseDown(e, node.id)}
              stroke-width="2"
            />
          </g>
        {/each}
      </g>
    </svg>
  </div>

  <!-- Context Menu -->
  {#if contextMenu.show}
    <div
      class="context-menu fixed bg-slate-900 border border-slate-700 rounded-lg shadow-xl py-2 z-50"
      style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
    >
      {#if selectedNode}
        <button
          onclick={() => { deleteNode(selectedNode!.id); closeContextMenu(); }}
          class="w-full px-4 py-2 text-left hover:bg-slate-800 text-sm text-red-400"
        >
          Delete Node
        </button>
      {:else}
        {#each Object.entries(NODE_CATEGORIES) as [category, nodeTypes]}
          <div class="px-4 py-1 text-xs font-bold text-slate-500 uppercase">{category}</div>
          {#each nodeTypes as type}
            <button
              onclick={() => handleAddNodeFromMenu(type)}
              class="w-full px-4 py-2 text-left hover:bg-slate-800 text-sm"
            >
              {type}
            </button>
          {/each}
        {/each}
      {/if}
    </div>
  {/if}

  <!-- Notification Toast -->
  {#if notification}
    <div class="fixed top-20 left-1/2 transform -translate-x-1/2 z-50 px-4 py-2 rounded-full border backdrop-blur-md shadow-xl
                {notification.type === 'error' ? 'bg-rose-500/10 border-rose-500 text-rose-400' : ''}
                {notification.type === 'success' ? 'bg-emerald-500/10 border-emerald-500 text-emerald-400' : ''}
                {notification.type === 'info' ? 'bg-indigo-500/10 border-indigo-500 text-indigo-400' : ''}">
      <span class="text-sm font-medium">{notification.message}</span>
    </div>
  {/if}
</div>

<style>
  .canvas-bg {
    background-image:
      linear-gradient(rgba(100, 116, 139, 0.1) 1px, transparent 1px),
      linear-gradient(90deg, rgba(100, 116, 139, 0.1) 1px, transparent 1px);
    background-size: 20px 20px;
  }
</style>
