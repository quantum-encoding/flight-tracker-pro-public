<script lang="ts">
import { onMount, onDestroy } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import cytoscape from 'cytoscape';
import fcose from 'cytoscape-fcose';
import { theme } from '$lib/theme';

// Register the force-directed layout extension
cytoscape.use(fcose);

interface Props {
  rootType?: string;
  rootId?: string;
  maxDepth?: number;
}

let { rootType = 'person', rootId = '', maxDepth = 2 }: Props = $props();

// Component state
let cyContainer = $state<HTMLDivElement | null>(null);
let cy = $state<cytoscape.Core | null>(null);
let loading = $state(true);
let error = $state('');
let selectedNode = $state<any>(null);
let showDetails = $state(false);

// Relationship data from Rust
interface GraphRelationship {
  id: string;
  source_type: string;
  source_id: string;
  target_type: string;
  target_id: string;
  relationship_type: string;
  weight: number;
  evidence?: string;
}

async function loadGraphData() {
  loading = true;
  error = '';

  try {
    const relationships: GraphRelationship[] = await invoke('query_relationship_graph', {
      sourceType: rootType,
      sourceId: rootId,
      maxDepth,
    });

    console.log(`📊 Loaded ${relationships.length} relationships`);

    if (relationships.length === 0) {
      error = 'No relationships found for this node';
      loading = false;
      return;
    }

    // Transform relationships into Cytoscape elements
    const elements = transformToCytoscapeElements(relationships);

    // Initialize the graph
    initializeCytoscape(elements);

    loading = false;
  } catch (err) {
    console.error('Failed to load graph:', err);
    error = `Failed to load relationship graph: ${err}`;
    loading = false;
  }
}

function transformToCytoscapeElements(relationships: GraphRelationship[]) {
  const nodesMap = new Map<string, any>();
  const edges: any[] = [];

  // Extract unique nodes and create edges
  for (const rel of relationships) {
    // Source node
    const sourceKey = `${rel.source_type}:${rel.source_id}`;
    if (!nodesMap.has(sourceKey)) {
      nodesMap.set(sourceKey, {
        data: {
          id: sourceKey,
          label: rel.source_id,
          type: rel.source_type,
          nodeType: rel.source_type,
        },
      });
    }

    // Target node
    const targetKey = `${rel.target_type}:${rel.target_id}`;
    if (!nodesMap.has(targetKey)) {
      nodesMap.set(targetKey, {
        data: {
          id: targetKey,
          label: rel.target_id,
          type: rel.target_type,
          nodeType: rel.target_type,
        },
      });
    }

    // Edge
    edges.push({
      data: {
        id: rel.id,
        source: sourceKey,
        target: targetKey,
        label: rel.relationship_type,
        weight: rel.weight,
        evidence: rel.evidence,
      },
    });
  }

  return {
    nodes: Array.from(nodesMap.values()),
    edges,
  };
}

function initializeCytoscape(elements: any) {
  if (!cyContainer) return;

  // Destroy existing instance
  if (cy) {
    cy.destroy();
  }

  // Create new Cytoscape instance
  cy = cytoscape({
    container: cyContainer,
    elements: [...elements.nodes, ...elements.edges],
    style: [
      // Node base styles
      {
        selector: 'node',
        style: {
          'label': 'data(label)',
          'text-valign': 'center',
          'text-halign': 'center',
          'font-size': '12px',
          'font-weight': 'bold',
          'text-outline-width': 2,
          'text-outline-color': '#1a1b26',
          'color': '#ffffff',
          'width': 40,
          'height': 40,
          'border-width': 2,
          'border-opacity': 0.9,
        },
      },
      // Person nodes - Bright Cyan Circle
      {
        selector: 'node[nodeType="person"]',
        style: {
          'background-color': '#06b6d4',
          'border-color': '#22d3ee',
          'shape': 'ellipse',
          'width': 50,
          'height': 50,
        },
      },
      // Flight nodes - Neon Purple Triangle/Diamond
      {
        selector: 'node[nodeType="flight"]',
        style: {
          'background-color': '#a855f7',
          'border-color': '#c084fc',
          'shape': 'diamond',
          'width': 45,
          'height': 45,
        },
      },
      // Document nodes - Amber/Orange Square
      {
        selector: 'node[nodeType="document_chunk"]',
        style: {
          'background-color': '#f59e0b',
          'border-color': '#fbbf24',
          'shape': 'rectangle',
          'width': 40,
          'height': 40,
        },
      },
      // Location nodes - Emerald Green Pentagon
      {
        selector: 'node[nodeType="location"]',
        style: {
          'background-color': '#10b981',
          'border-color': '#34d399',
          'shape': 'pentagon',
          'width': 45,
          'height': 45,
        },
      },
      // Airport nodes - Emerald Green Pentagon (same as location)
      {
        selector: 'node[nodeType="airport"]',
        style: {
          'background-color': '#10b981',
          'border-color': '#34d399',
          'shape': 'pentagon',
          'width': 45,
          'height': 45,
        },
      },
      // Entity nodes - Default style
      {
        selector: 'node[nodeType="entity"]',
        style: {
          'background-color': '#64748b',
          'border-color': '#94a3b8',
          'shape': 'hexagon',
          'width': 35,
          'height': 35,
        },
      },
      // Edge base styles
      {
        selector: 'edge',
        style: {
          'width': 'data(weight)',
          'line-color': '#475569',
          'target-arrow-color': '#475569',
          'target-arrow-shape': 'triangle',
          'curve-style': 'bezier',
          'opacity': 0.6,
          'label': 'data(label)',
          'font-size': '10px',
          'text-rotation': 'autorotate',
          'text-margin-y': -10,
          'color': '#94a3b8',
          'text-outline-width': 2,
          'text-outline-color': '#1a1b26',
        },
      },
      // Strong edges (high confidence)
      {
        selector: 'edge[weight >= 0.8]',
        style: {
          'line-color': '#ffffff',
          'target-arrow-color': '#ffffff',
          'width': 3,
          'opacity': 0.9,
        },
      },
      // Medium edges
      {
        selector: 'edge[weight >= 0.5][weight < 0.8]',
        style: {
          'line-color': '#94a3b8',
          'target-arrow-color': '#94a3b8',
          'width': 2,
          'opacity': 0.7,
        },
      },
      // Weak edges
      {
        selector: 'edge[weight < 0.5]',
        style: {
          'line-color': '#475569',
          'target-arrow-color': '#475569',
          'width': 1,
          'opacity': 0.4,
        },
      },
      // Hover states
      {
        selector: 'node:selected',
        style: {
          'border-width': 4,
          'border-opacity': 1,
          'overlay-opacity': 0.3,
          'overlay-color': '#ffffff',
          'overlay-padding': 5,
        },
      },
      {
        selector: 'edge:selected',
        style: {
          'line-color': '#ffffff',
          'target-arrow-color': '#ffffff',
          'width': 4,
          'opacity': 1,
        },
      },
      // Dimmed state for non-connected nodes during hover
      {
        selector: 'node.dimmed',
        style: {
          'opacity': 0.2,
        },
      },
      {
        selector: 'edge.dimmed',
        style: {
          'opacity': 0.1,
        },
      },
    ],
    layout: {
      name: 'fcose',
      quality: 'proof',
      randomize: false,
      animate: true,
      animationDuration: 1000,
      animationEasing: 'ease-out',
      nodeDimensionsIncludeLabels: true,
      uniformNodeDimensions: false,
      packComponents: true,
      nodeRepulsion: 4500,
      idealEdgeLength: 100,
      edgeElasticity: 0.45,
      nestingFactor: 0.1,
      gravity: 0.25,
      numIter: 2500,
      tile: true,
      tilingPaddingVertical: 10,
      tilingPaddingHorizontal: 10,
      gravityRangeCompound: 1.5,
      gravityCompound: 1.0,
      gravityRange: 3.8,
    } as any,
  });

  // Add interactivity
  setupInteractivity();
}

function setupInteractivity() {
  if (!cy) return;

  // Hover effects: Highlight connected nodes
  cy.on('mouseover', 'node', (event) => {
    const node = event.target;
    const connectedEdges = node.connectedEdges();
    const connectedNodes = connectedEdges.connectedNodes();

    // Dim all other elements
    cy!.elements().addClass('dimmed');

    // Highlight the hovered node and its connections
    node.removeClass('dimmed');
    connectedNodes.removeClass('dimmed');
    connectedEdges.removeClass('dimmed');
  });

  cy.on('mouseout', 'node', () => {
    // Remove dimmed class from all elements
    cy!.elements().removeClass('dimmed');
  });

  // Click handler: Show node details
  cy.on('tap', 'node', (event) => {
    const node = event.target;
    selectedNode = {
      id: node.id(),
      type: node.data('nodeType'),
      label: node.data('label'),
      degree: node.degree(),
      neighbors: node.neighborhood().nodes().length,
    };
    showDetails = true;

    console.log('🔍 Node clicked:', selectedNode);
  });

  // Click handler for edges
  cy.on('tap', 'edge', (event) => {
    const edge = event.target;
    selectedNode = {
      id: edge.id(),
      type: 'edge',
      label: edge.data('label'),
      weight: edge.data('weight'),
      evidence: edge.data('evidence'),
      source: edge.data('source'),
      target: edge.data('target'),
    };
    showDetails = true;

    console.log('🔗 Edge clicked:', selectedNode);
  });

  // Double-click to expand node (future feature)
  cy.on('dbltap', 'node', (event) => {
    const node = event.target;
    console.log('🔄 Double-click to expand:', node.id());
    // TODO: Load additional relationships for this node
  });
}

function closeDetails() {
  showDetails = false;
  selectedNode = null;
}

// Lifecycle
onMount(() => {
  if (rootId) {
    loadGraphData();
  } else {
    error = 'No root node specified. Provide rootType and rootId props.';
    loading = false;
  }
});

onDestroy(() => {
  if (cy) {
    cy.destroy();
    cy = null;
  }
});
</script>

<!-- War Room Container -->
<div class="relationship-graph-container w-full h-full rounded-lg overflow-hidden relative {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}">
  {#if loading}
    <div class="loading-overlay absolute inset-0 flex items-center justify-center bg-opacity-90 z-10">
      <div class="text-center">
        <div class="loading-spinner animate-spin rounded-full h-16 w-16 border-t-4 border-b-4 mx-auto mb-4"></div>
        <p class="loading-text text-lg font-semibold">Loading relationship graph...</p>
        <p class="loading-subtext text-sm mt-2">Building the red thread...</p>
      </div>
    </div>
  {/if}

  {#if error}
    <div class="error-overlay absolute inset-0 flex items-center justify-center bg-opacity-90 z-10">
      <div class="text-center max-w-md p-8">
        <div class="text-red-400 text-6xl mb-4">⚠️</div>
        <p class="text-red-400 text-lg font-semibold mb-2">Error Loading Graph</p>
        <p class="error-text text-sm">{error}</p>
        <button
          onclick={() => loadGraphData()}
          class="retry-btn mt-6 px-6 py-2 text-white rounded-lg transition-colors"
        >
          Retry
        </button>
      </div>
    </div>
  {/if}

  <!-- Cytoscape Container -->
  <div bind:this={cyContainer} class="w-full h-full"></div>

  <!-- Legend -->
  <div class="legend-panel absolute top-4 left-4 bg-opacity-90 rounded-lg p-4 z-20">
    <h3 class="legend-title font-bold text-sm mb-3">🗺️ Node Types</h3>
    <div class="space-y-2 text-xs">
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 rounded-full bg-cyan-400 border-2 border-cyan-200"></div>
        <span class="legend-label">Person</span>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 bg-purple-500 border-2 border-purple-300" style="clip-path: polygon(50% 0%, 100% 50%, 50% 100%, 0% 50%)"></div>
        <span class="legend-label">Flight</span>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 bg-amber-500 border-2 border-amber-300"></div>
        <span class="legend-label">Document</span>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 bg-emerald-500 border-2 border-emerald-300" style="clip-path: polygon(50% 0%, 100% 38%, 82% 100%, 18% 100%, 0% 38%)"></div>
        <span class="legend-label">Airport/Location</span>
      </div>
      <div class="flex items-center gap-2">
        <div class="w-4 h-4 bg-gray-500 border-2 border-gray-300" style="clip-path: polygon(30% 0%, 70% 0%, 100% 30%, 100% 70%, 70% 100%, 30% 100%, 0% 70%, 0% 30%)"></div>
        <span class="legend-label">Entity</span>
      </div>
    </div>
    <div class="legend-footer mt-4 pt-3">
      <p class="legend-hint text-xs">
        <span class="font-semibold">Hover:</span> Highlight connections<br>
        <span class="font-semibold">Click:</span> View details<br>
        <span class="font-semibold">Double-click:</span> Expand node
      </p>
    </div>
  </div>

  <!-- Node Details Panel -->
  {#if showDetails && selectedNode}
    <div class="details-panel absolute top-4 right-4 bg-opacity-95 rounded-lg p-6 z-20 max-w-md shadow-2xl">
      <div class="flex justify-between items-start mb-4">
        <h3 class="details-title font-bold text-lg">🔍 Details</h3>
        <button
          onclick={closeDetails}
          class="details-close hover:opacity-100 transition-colors text-2xl leading-none"
        >
          ×
        </button>
      </div>

      <div class="space-y-3">
        <div>
          <p class="details-label text-xs uppercase tracking-wide">Type</p>
          <p class="details-value font-semibold capitalize">{selectedNode.type}</p>
        </div>

        <div>
          <p class="details-label text-xs uppercase tracking-wide">Label</p>
          <p class="details-value break-all">{selectedNode.label}</p>
        </div>

        {#if selectedNode.type !== 'edge'}
          <div>
            <p class="details-label text-xs uppercase tracking-wide">Connections</p>
            <p class="details-highlight-primary font-bold text-2xl">{selectedNode.neighbors}</p>
            <p class="details-hint text-xs">Connected nodes</p>
          </div>

          <div>
            <p class="details-label text-xs uppercase tracking-wide">Degree</p>
            <p class="details-highlight-secondary font-bold text-xl">{selectedNode.degree}</p>
            <p class="details-hint text-xs">Total edges</p>
          </div>
        {:else}
          <div>
            <p class="details-label text-xs uppercase tracking-wide">Confidence</p>
            <p class="text-amber-400 font-bold text-2xl">{(selectedNode.weight * 100).toFixed(0)}%</p>
          </div>

          <div>
            <p class="details-label text-xs uppercase tracking-wide">Evidence</p>
            <p class="details-value text-sm">{selectedNode.evidence || 'No evidence recorded'}</p>
          </div>

          <div class="details-hint text-xs">
            <p>{selectedNode.source}</p>
            <p class="my-1">↓</p>
            <p>{selectedNode.target}</p>
          </div>
        {/if}
      </div>

      <div class="details-footer mt-4 pt-4">
        <p class="details-label text-xs">ID: <span class="font-mono opacity-70">{selectedNode.id}</span></p>
      </div>
    </div>
  {/if}

  <!-- Stats Overlay -->
  {#if cy && !loading}
    <div class="stats-overlay absolute bottom-4 left-4 bg-opacity-90 rounded-lg px-4 py-2 z-20">
      <p class="stats-text text-xs">
        <span class="stats-primary font-bold">{cy.nodes().length}</span> nodes •
        <span class="stats-secondary font-bold">{cy.edges().length}</span> edges
      </p>
    </div>
  {/if}
</div>

<style>
  .relationship-graph-container {
    min-height: 600px;
  }

  /* ===== DEFAULT THEME (Dark) ===== */
  .theme-default {
    background-color: #1a1b26;
  }

  .theme-default .loading-overlay,
  .theme-default .error-overlay {
    background-color: #1a1b26;
  }

  .theme-default .loading-spinner {
    border-color: #06b6d4;
  }

  .theme-default .loading-text {
    color: #06b6d4;
  }

  .theme-default .loading-subtext,
  .theme-default .error-text {
    color: #9ca3af;
  }

  .theme-default .retry-btn {
    background-color: #06b6d4;
  }

  .theme-default .retry-btn:hover {
    background-color: #0891b2;
  }

  .theme-default .legend-panel,
  .theme-default .details-panel,
  .theme-default .stats-overlay {
    background-color: #1a1b26;
    border: 1px solid #374151;
  }

  .theme-default .legend-title,
  .theme-default .details-title,
  .theme-default .details-value {
    color: #ffffff;
  }

  .theme-default .legend-label,
  .theme-default .legend-hint,
  .theme-default .details-label,
  .theme-default .stats-text {
    color: #9ca3af;
  }

  .theme-default .legend-footer {
    border-top: 1px solid #4b5563;
  }

  .theme-default .details-footer {
    border-top: 1px solid #374151;
  }

  .theme-default .details-close {
    color: #9ca3af;
  }

  .theme-default .details-hint {
    color: #6b7280;
  }

  .theme-default .details-highlight-primary {
    color: #06b6d4;
  }

  .theme-default .details-highlight-secondary {
    color: #a855f7;
  }

  .theme-default .stats-primary {
    color: #06b6d4;
  }

  .theme-default .stats-secondary {
    color: #a855f7;
  }

  /* ===== SKYNET THEME ===== */
  .theme-skynet {
    background-color: #000000;
  }

  .theme-skynet .loading-overlay,
  .theme-skynet .error-overlay {
    background-color: #000000;
  }

  .theme-skynet .loading-spinner {
    border-color: #00b4ff;
  }

  .theme-skynet .loading-text {
    color: #00b4ff;
  }

  .theme-skynet .loading-subtext,
  .theme-skynet .error-text {
    color: #0080ff;
  }

  .theme-skynet .retry-btn {
    background-color: #0080ff;
    border: 1px solid #00b4ff;
  }

  .theme-skynet .retry-btn:hover {
    background-color: #00b4ff;
  }

  .theme-skynet .legend-panel,
  .theme-skynet .details-panel,
  .theme-skynet .stats-overlay {
    background-color: #000000;
    border: 1px solid #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .legend-title,
  .theme-skynet .details-title,
  .theme-skynet .details-value {
    color: #00b4ff;
  }

  .theme-skynet .legend-label,
  .theme-skynet .legend-hint,
  .theme-skynet .details-label,
  .theme-skynet .stats-text {
    color: #0080ff;
  }

  .theme-skynet .legend-footer {
    border-top: 1px solid #0080ff;
  }

  .theme-skynet .details-footer {
    border-top: 1px solid #0080ff;
  }

  .theme-skynet .details-close {
    color: #0080ff;
  }

  .theme-skynet .details-hint {
    color: #005299;
  }

  .theme-skynet .details-highlight-primary {
    color: #00b4ff;
  }

  .theme-skynet .details-highlight-secondary {
    color: #0080ff;
  }

  .theme-skynet .stats-primary {
    color: #00b4ff;
  }

  .theme-skynet .stats-secondary {
    color: #0080ff;
  }

  /* ===== CYBERPUNK THEME ===== */
  .theme-cyberpunk {
    background-color: #0d0d0d;
  }

  .theme-cyberpunk .loading-overlay,
  .theme-cyberpunk .error-overlay {
    background-color: #0d0d0d;
  }

  .theme-cyberpunk .loading-spinner {
    border-color: #00d9ff;
  }

  .theme-cyberpunk .loading-text {
    color: #00d9ff;
  }

  .theme-cyberpunk .loading-subtext,
  .theme-cyberpunk .error-text {
    color: #ff0080;
  }

  .theme-cyberpunk .retry-btn {
    background-color: #ff0080;
    border: 1px solid #00d9ff;
  }

  .theme-cyberpunk .retry-btn:hover {
    background-color: #00d9ff;
    color: #000000;
  }

  .theme-cyberpunk .legend-panel,
  .theme-cyberpunk .details-panel,
  .theme-cyberpunk .stats-overlay {
    background-color: #0d0d0d;
    border: 1px solid #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.3), 0 0 30px rgba(255, 0, 128, 0.1);
  }

  .theme-cyberpunk .legend-title,
  .theme-cyberpunk .details-title,
  .theme-cyberpunk .details-value {
    color: #00d9ff;
  }

  .theme-cyberpunk .legend-label,
  .theme-cyberpunk .legend-hint,
  .theme-cyberpunk .details-label,
  .theme-cyberpunk .stats-text {
    color: #b000ff;
  }

  .theme-cyberpunk .legend-footer {
    border-top: 1px solid #ff0080;
  }

  .theme-cyberpunk .details-footer {
    border-top: 1px solid #ff0080;
  }

  .theme-cyberpunk .details-close {
    color: #ff0080;
  }

  .theme-cyberpunk .details-hint {
    color: #b000ff;
  }

  .theme-cyberpunk .details-highlight-primary {
    color: #00d9ff;
  }

  .theme-cyberpunk .details-highlight-secondary {
    color: #ff0080;
  }

  .theme-cyberpunk .stats-primary {
    color: #00d9ff;
  }

  .theme-cyberpunk .stats-secondary {
    color: #ff0080;
  }

  /* ===== SCROLLBAR STYLES ===== */
  .theme-default ::-webkit-scrollbar {
    width: 8px;
  }

  .theme-default ::-webkit-scrollbar-track {
    background: #1a1b26;
  }

  .theme-default ::-webkit-scrollbar-thumb {
    background: #475569;
    border-radius: 4px;
  }

  .theme-default ::-webkit-scrollbar-thumb:hover {
    background: #64748b;
  }

  .theme-skynet ::-webkit-scrollbar {
    width: 8px;
  }

  .theme-skynet ::-webkit-scrollbar-track {
    background: #000000;
  }

  .theme-skynet ::-webkit-scrollbar-thumb {
    background: #0080ff;
    border-radius: 4px;
  }

  .theme-skynet ::-webkit-scrollbar-thumb:hover {
    background: #00b4ff;
  }

  .theme-cyberpunk ::-webkit-scrollbar {
    width: 8px;
  }

  .theme-cyberpunk ::-webkit-scrollbar-track {
    background: #0d0d0d;
  }

  .theme-cyberpunk ::-webkit-scrollbar-thumb {
    background: #ff0080;
    border-radius: 4px;
  }

  .theme-cyberpunk ::-webkit-scrollbar-thumb:hover {
    background: #00d9ff;
  }
</style>
