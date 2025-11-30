<!-- High-performance Network Graph with Canvas rendering and Web Worker -->
<!-- Inspired by god_view_service force simulation parameters -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { PassengerNetworkData } from '$lib/types/analytics';

  interface Props {
    data?: PassengerNetworkData;
  }

  let { data }: Props = $props();

  // Canvas and container refs
  let container: HTMLDivElement | null = $state(null);
  let canvas: HTMLCanvasElement | null = $state(null);
  let ctx: CanvasRenderingContext2D | null = $state(null);

  // Simulation state
  let worker: Worker | null = null;
  let nodes: Map<string, { id: string; x: number; y: number; radius: number; label: string; total_flights: number }> = $state(new Map());
  let edges: { source: { x: number; y: number }; target: { x: number; y: number }; flight_count: number }[] = $state([]);

  // View state
  let transform = $state({ x: 0, y: 0, scale: 1 });
  let isDragging = $state(false);
  let dragStart = $state({ x: 0, y: 0 });
  let draggedNode: string | null = $state(null);
  let hoveredNode: string | null = $state(null);
  let selectedNode: string | null = $state(null);
  let isPanning = $state(false);
  let isRunning = $state(false);
  let error = $state<string | null>(null);

  // Layout selector
  let currentLayout = $state<'force' | 'radial' | 'grid'>('force');

  // Legend & Filter state
  let legendOpen = $state(true);
  let legendTab = $state<'people' | 'stats'>('people');
  let filterTopN = $state(10);
  let showAllNodes = $state(false);
  let filteredData = $state<PassengerNetworkData | null>(null);

  // Export state
  let exporting = $state(false);

  // Colors
  const colors = {
    background: '#0f172a',
    node: '#3b82f6',
    nodeHover: '#60a5fa',
    nodeSelected: '#f59e0b',
    nodeHighFlight: '#22c55e',
    edge: 'rgba(148, 163, 184, 0.25)',
    edgeHighlight: 'rgba(96, 165, 250, 0.7)',
    text: '#e5e7eb',
    textSecondary: '#9ca3af'
  };

  function initWorker() {
    if (worker) {
      worker.terminate();
    }

    // Web Worker code - adapted from god_view_service parameters
    const workerCode = `
      let nodes = [];
      let edges = [];
      let config = {
        width: 1200,
        height: 800,
        // Adaptive parameters - will be overridden based on graph size
        chargeStrength: -500,
        linkDistance: 120,
        collisionRadius: 25,
        centerStrength: 0.05,
        linkStrength: 0.3
      };
      let alpha = 1;
      let alphaTarget = 0;
      let alphaDecay = 0.01;  // Slower cooling for better spread
      let alphaMin = 0.001;
      let velocityDecay = 0.6;  // Higher friction for stability
      let running = false;

      function initializeNodes() {
        const centerX = config.width / 2;
        const centerY = config.height / 2;
        const nodeCount = nodes.length;

        // Calculate initial spread radius based on node count
        // Larger graphs need more initial spread to prevent crushing
        const spreadRadius = Math.max(
          Math.min(config.width, config.height) * 0.4,
          Math.sqrt(nodeCount) * 30
        );

        nodes.forEach((node, i) => {
          if (node.x === undefined) {
            // Use golden angle spiral for even distribution
            const goldenAngle = Math.PI * (3 - Math.sqrt(5));
            const angle = i * goldenAngle;
            const r = spreadRadius * Math.sqrt(i / nodeCount);
            node.x = centerX + Math.cos(angle) * r;
            node.y = centerY + Math.sin(angle) * r;
          }
          node.vx = node.vx || 0;
          node.vy = node.vy || 0;
          // Dynamic node radius based on flight count
          node.radius = Math.max(6, Math.min(25, Math.sqrt(node.total_flights || 1) * 4));
        });
      }

      function resolveEdges() {
        const nodeMap = new Map(nodes.map(n => [n.id, n]));
        edges = edges.map(e => ({
          ...e,
          source: typeof e.source === 'string' ? nodeMap.get(e.source) : e.source,
          target: typeof e.target === 'string' ? nodeMap.get(e.target) : e.target
        })).filter(e => e.source && e.target);
      }

      function applyForces() {
        const centerX = config.width / 2;
        const centerY = config.height / 2;
        const nodeCount = nodes.length;

        // Center force - pull toward center
        nodes.forEach(node => {
          if (node.fx == null) {
            node.vx += (centerX - node.x) * config.centerStrength;
            node.vy += (centerY - node.y) * config.centerStrength;
          }
        });

        // Charge force (repulsion) - Barnes-Hut approximation for large graphs
        // For smaller graphs, use O(n^2) for accuracy
        if (nodeCount < 500) {
          for (let i = 0; i < nodeCount; i++) {
            for (let j = i + 1; j < nodeCount; j++) {
              const a = nodes[i], b = nodes[j];
              const dx = b.x - a.x, dy = b.y - a.y;
              const distSq = dx * dx + dy * dy;
              const dist = Math.sqrt(distSq) || 1;

              // Stronger repulsion for nodes with more flights
              const strengthMult = Math.sqrt((a.total_flights || 1) * (b.total_flights || 1)) * 0.1;
              const force = (config.chargeStrength * (1 + strengthMult)) / distSq;
              const fx = (dx / dist) * force;
              const fy = (dy / dist) * force;

              if (a.fx == null) { a.vx -= fx; a.vy -= fy; }
              if (b.fx == null) { b.vx += fx; b.vy += fy; }
            }
          }
        } else {
          // For large graphs, use quadtree approximation (simplified)
          const gridSize = 100;
          const grid = new Map();

          // Build grid
          nodes.forEach(node => {
            const gx = Math.floor(node.x / gridSize);
            const gy = Math.floor(node.y / gridSize);
            const key = gx + ',' + gy;
            if (!grid.has(key)) grid.set(key, []);
            grid.get(key).push(node);
          });

          // Apply forces from nearby grid cells
          nodes.forEach(node => {
            const gx = Math.floor(node.x / gridSize);
            const gy = Math.floor(node.y / gridSize);

            for (let dx = -2; dx <= 2; dx++) {
              for (let dy = -2; dy <= 2; dy++) {
                const key = (gx + dx) + ',' + (gy + dy);
                const cell = grid.get(key);
                if (!cell) continue;

                cell.forEach(other => {
                  if (other === node) return;
                  const ddx = other.x - node.x, ddy = other.y - node.y;
                  const distSq = ddx * ddx + ddy * ddy;
                  if (distSq < 1) return;
                  const dist = Math.sqrt(distSq);
                  const force = config.chargeStrength / distSq;
                  if (node.fx == null) {
                    node.vx -= (ddx / dist) * force;
                    node.vy -= (ddy / dist) * force;
                  }
                });
              }
            }
          });
        }

        // Link force (attraction along edges)
        edges.forEach(e => {
          if (!e.source || !e.target) return;
          const dx = e.target.x - e.source.x;
          const dy = e.target.y - e.source.y;
          const dist = Math.sqrt(dx * dx + dy * dy) || 1;

          // Variable link distance based on edge weight
          const targetDist = config.linkDistance + (e.flight_count || 1) * 5;
          const diff = (dist - targetDist) / dist;
          const strength = config.linkStrength;
          const fx = dx * diff * strength;
          const fy = dy * diff * strength;

          if (e.source.fx == null) { e.source.vx += fx; e.source.vy += fy; }
          if (e.target.fx == null) { e.target.vx -= fx; e.target.vy -= fy; }
        });

        // Collision force
        for (let i = 0; i < nodeCount; i++) {
          for (let j = i + 1; j < nodeCount; j++) {
            const a = nodes[i], b = nodes[j];
            const dx = b.x - a.x, dy = b.y - a.y;
            const dist = Math.sqrt(dx * dx + dy * dy) || 1;
            const minDist = (a.radius || 10) + (b.radius || 10) + config.collisionRadius;

            if (dist < minDist) {
              const diff = (minDist - dist) / dist * 0.5;
              const fx = dx * diff, fy = dy * diff;
              if (a.fx == null) { a.x -= fx; a.y -= fy; }
              if (b.fx == null) { b.x += fx; b.y += fy; }
            }
          }
        }
      }

      function tick() {
        alpha += (alphaTarget - alpha) * alphaDecay;
        if (alpha < alphaMin) {
          running = false;
          self.postMessage({ type: 'end' });
          return;
        }

        applyForces();

        // Apply velocity and decay
        const padding = 50;
        nodes.forEach(node => {
          if (node.fx != null) { node.x = node.fx; node.vx = 0; }
          else { node.vx *= velocityDecay; node.x += node.vx * alpha; }
          if (node.fy != null) { node.y = node.fy; node.vy = 0; }
          else { node.vy *= velocityDecay; node.y += node.vy * alpha; }

          // Soft boundary - push back gently instead of hard clamp
          if (node.x < padding) node.vx += (padding - node.x) * 0.1;
          if (node.x > config.width - padding) node.vx -= (node.x - config.width + padding) * 0.1;
          if (node.y < padding) node.vy += (padding - node.y) * 0.1;
          if (node.y > config.height - padding) node.vy -= (node.y - config.height + padding) * 0.1;
        });

        self.postMessage({
          type: 'tick',
          nodes: nodes.map(n => ({ id: n.id, x: n.x, y: n.y, radius: n.radius, label: n.label, total_flights: n.total_flights })),
          edges: edges.filter(e => e.source && e.target).map(e => ({
            source: { x: e.source.x, y: e.source.y },
            target: { x: e.target.x, y: e.target.y },
            flight_count: e.flight_count
          })),
          alpha
        });

        if (running) setTimeout(tick, 16);
      }

      self.onmessage = (e) => {
        const { type, payload } = e.data;

        if (type === 'init') {
          nodes = payload.nodes.map(n => ({
            ...n,
            radius: Math.max(6, Math.min(25, Math.sqrt(n.total_flights || 1) * 4))
          }));
          edges = payload.edges;

          // Adaptive parameters based on graph size
          const nodeCount = nodes.length;
          const edgeCount = edges.length;
          const density = edgeCount / (nodeCount * nodeCount) || 0;

          config = {
            ...config,
            ...payload.config,
            // Increase repulsion for larger graphs
            chargeStrength: nodeCount > 500 ? -800 : nodeCount > 200 ? -600 : -400,
            // Increase link distance for denser graphs
            linkDistance: 80 + Math.sqrt(nodeCount) * 3,
            // Stronger center force for sparse graphs
            centerStrength: density < 0.01 ? 0.08 : 0.03,
            // Weaker link strength for dense graphs
            linkStrength: density > 0.05 ? 0.2 : 0.4,
            collisionRadius: nodeCount > 500 ? 15 : 20
          };

          initializeNodes();
          resolveEdges();
          alpha = 1;
          running = true;
          tick();
        } else if (type === 'stop') {
          running = false;
        } else if (type === 'restart') {
          alpha = payload?.alpha ?? 1;
          alphaTarget = 0;
          if (!running) { running = true; tick(); }
        } else if (type === 'drag') {
          const node = nodes.find(n => n.id === payload.nodeId);
          if (node) {
            if (payload.phase === 'start') {
              node.fx = node.x; node.fy = node.y;
              alphaTarget = 0.3;
              if (!running) { running = true; tick(); }
            } else if (payload.phase === 'drag') {
              node.fx = payload.x; node.fy = payload.y;
            } else if (payload.phase === 'end') {
              node.fx = null; node.fy = null;
              alphaTarget = 0;
            }
          }
        } else if (type === 'resize') {
          config.width = payload.width;
          config.height = payload.height;
        } else if (type === 'layout') {
          // Different layout algorithms
          if (payload.type === 'radial') {
            const centerX = config.width / 2;
            const centerY = config.height / 2;
            const maxRadius = Math.min(config.width, config.height) * 0.45;

            // Sort nodes by flight count
            const sorted = [...nodes].sort((a, b) => (b.total_flights || 0) - (a.total_flights || 0));
            sorted.forEach((node, i) => {
              const ring = Math.floor(i / 8);
              const posInRing = i % 8;
              const ringRadius = (ring + 1) * (maxRadius / Math.ceil(nodes.length / 8));
              const angle = (posInRing / 8) * Math.PI * 2;
              node.x = centerX + Math.cos(angle) * ringRadius;
              node.y = centerY + Math.sin(angle) * ringRadius;
              node.vx = 0;
              node.vy = 0;
            });

            self.postMessage({
              type: 'tick',
              nodes: nodes.map(n => ({ id: n.id, x: n.x, y: n.y, radius: n.radius, label: n.label, total_flights: n.total_flights })),
              edges: edges.filter(e => e.source && e.target).map(e => ({
                source: { x: e.source.x, y: e.source.y },
                target: { x: e.target.x, y: e.target.y },
                flight_count: e.flight_count
              })),
              alpha: 0
            });
          } else if (payload.type === 'grid') {
            const cols = Math.ceil(Math.sqrt(nodes.length));
            const cellWidth = (config.width - 100) / cols;
            const cellHeight = (config.height - 100) / Math.ceil(nodes.length / cols);

            nodes.forEach((node, i) => {
              node.x = 50 + (i % cols) * cellWidth + cellWidth / 2;
              node.y = 50 + Math.floor(i / cols) * cellHeight + cellHeight / 2;
              node.vx = 0;
              node.vy = 0;
            });

            self.postMessage({
              type: 'tick',
              nodes: nodes.map(n => ({ id: n.id, x: n.x, y: n.y, radius: n.radius, label: n.label, total_flights: n.total_flights })),
              edges: edges.filter(e => e.source && e.target).map(e => ({
                source: { x: e.source.x, y: e.source.y },
                target: { x: e.target.x, y: e.target.y },
                flight_count: e.flight_count
              })),
              alpha: 0
            });
          }
        }
      };
    `;

    const blob = new Blob([workerCode], { type: 'application/javascript' });
    worker = new Worker(URL.createObjectURL(blob));

    worker.onmessage = (e) => {
      const { type, nodes: newNodes, edges: newEdges, alpha } = e.data;

      if (type === 'tick') {
        nodes = new Map(newNodes.map((n: any) => [n.id, n]));
        edges = newEdges;
        isRunning = alpha > 0.001;
        requestAnimationFrame(render);
      } else if (type === 'end') {
        isRunning = false;
        requestAnimationFrame(render);
      }
    };
  }

  // Get sorted list of passengers by flight count
  function getSortedPassengers(): { id: string; label: string; total_flights: number; visible: boolean }[] {
    if (!data?.nodes) return [];
    const visibleIds = new Set(filteredData?.nodes?.map(n => n.id) || []);
    return [...data.nodes]
      .sort((a, b) => (b.total_flights || 0) - (a.total_flights || 0))
      .map(n => ({
        id: n.id,
        label: n.label,
        total_flights: n.total_flights || 0,
        visible: showAllNodes || visibleIds.has(n.id)
      }));
  }

  // Filter data to top N passengers
  function applyFilter() {
    if (!data?.nodes || !data?.edges) {
      filteredData = null;
      return;
    }

    if (showAllNodes) {
      filteredData = data;
      return;
    }

    // Get top N nodes by flight count
    const sortedNodes = [...data.nodes].sort((a, b) => (b.total_flights || 0) - (a.total_flights || 0));
    const topNodes = sortedNodes.slice(0, filterTopN);
    const topNodeIds = new Set(topNodes.map(n => n.id));

    // Filter edges to only include connections between top nodes
    const filteredEdges = data.edges.filter(e =>
      topNodeIds.has(e.source) && topNodeIds.has(e.target)
    );

    filteredData = {
      nodes: topNodes,
      edges: filteredEdges
    };
  }

  // Toggle visibility of a specific passenger
  function togglePassengerVisibility(passengerId: string) {
    if (!data?.nodes || !data?.edges) return;

    const currentVisibleIds = new Set(filteredData?.nodes?.map(n => n.id) || []);

    if (currentVisibleIds.has(passengerId)) {
      // Remove this passenger
      currentVisibleIds.delete(passengerId);
    } else {
      // Add this passenger
      currentVisibleIds.add(passengerId);
    }

    // Rebuild filtered data
    const visibleNodes = data.nodes.filter(n => currentVisibleIds.has(n.id));
    const visibleEdges = data.edges.filter(e =>
      currentVisibleIds.has(e.source) && currentVisibleIds.has(e.target)
    );

    filteredData = {
      nodes: visibleNodes,
      edges: visibleEdges
    };

    // Restart simulation with new data
    startSimulation();
  }

  function startSimulation() {
    const activeData = filteredData || data;
    if (!activeData?.nodes || !activeData?.edges || !container) return;

    // Use full container dimensions
    const width = container.clientWidth || window.innerWidth;
    const height = container.clientHeight || window.innerHeight;

    worker?.postMessage({
      type: 'init',
      payload: {
        nodes: activeData.nodes.map(n => ({
          id: n.id,
          label: n.label,
          total_flights: n.total_flights,
          total_distance_km: n.total_distance_km
        })),
        edges: activeData.edges.map(e => ({
          source: e.source,
          target: e.target,
          flight_count: e.flight_count
        })),
        config: { width, height }
      }
    });

    isRunning = true;
  }

  function render() {
    if (!ctx || !canvas) return;
    const context = ctx; // Local const for TypeScript narrowing in callbacks

    const width = canvas.width;
    const height = canvas.height;

    // Clear canvas with dark background
    context.fillStyle = colors.background;
    context.fillRect(0, 0, width, height);

    context.save();
    context.translate(transform.x + width / 2, transform.y + height / 2);
    context.scale(transform.scale, transform.scale);
    context.translate(-width / 2, -height / 2);

    // Draw edges first (underneath nodes)
    edges.forEach(edge => {
      if (!edge.source || !edge.target) return;

      const isHighlighted = selectedNode && nodes.has(selectedNode) && (
        Math.abs(nodes.get(selectedNode)!.x - edge.source.x) < 1 ||
        Math.abs(nodes.get(selectedNode)!.x - edge.target.x) < 1
      );

      context.strokeStyle = isHighlighted ? colors.edgeHighlight : colors.edge;
      context.lineWidth = Math.max(0.5, Math.sqrt(edge.flight_count || 1) * 0.8) / transform.scale;
      context.beginPath();
      context.moveTo(edge.source.x, edge.source.y);
      context.lineTo(edge.target.x, edge.target.y);
      context.stroke();
    });

    // Draw nodes
    nodes.forEach((node, id) => {
      const isHovered = hoveredNode === id;
      const isSelected = selectedNode === id;
      const highFlights = (node.total_flights || 0) > 50;

      context.beginPath();
      context.arc(node.x, node.y, node.radius, 0, Math.PI * 2);

      // Color based on state and flight count
      if (isSelected) {
        context.fillStyle = colors.nodeSelected;
        context.shadowColor = colors.nodeSelected;
        context.shadowBlur = 20;
      } else if (isHovered) {
        context.fillStyle = colors.nodeHover;
        context.shadowColor = colors.nodeHover;
        context.shadowBlur = 15;
      } else if (highFlights) {
        context.fillStyle = colors.nodeHighFlight;
        context.shadowBlur = 0;
      } else {
        context.fillStyle = colors.node;
        context.shadowBlur = 0;
      }

      context.fill();
      context.shadowBlur = 0;

      // Draw labels when zoomed in enough or for selected/hovered nodes
      if (transform.scale > 0.6 || isHovered || isSelected) {
        context.fillStyle = colors.text;
        context.font = `${Math.max(9, 11 / transform.scale)}px -apple-system, BlinkMacSystemFont, sans-serif`;
        context.textAlign = 'center';
        context.textBaseline = 'top';
        context.fillText(node.label || 'Unknown', node.x, node.y + node.radius + 4);

        if (isHovered || isSelected) {
          context.fillStyle = colors.textSecondary;
          context.font = `${Math.max(8, 9 / transform.scale)}px -apple-system, BlinkMacSystemFont, sans-serif`;
          context.fillText(`${node.total_flights} flights`, node.x, node.y + node.radius + 18);
        }
      }
    });

    context.restore();

    // UI Overlay
    context.fillStyle = 'rgba(15, 23, 42, 0.8)';
    context.fillRect(8, 8, 220, 50);
    context.fillStyle = colors.text;
    context.font = '11px monospace';
    context.textAlign = 'left';
    context.fillText(`Nodes: ${nodes.size} | Edges: ${edges.length}`, 16, 24);
    context.fillText(`Zoom: ${(transform.scale * 100).toFixed(0)}% | Layout: ${currentLayout}`, 16, 40);

    if (isRunning) {
      context.fillStyle = '#22c55e';
      context.beginPath();
      context.arc(218, 30, 5, 0, Math.PI * 2);
      context.fill();
    }
  }

  function screenToWorld(sx: number, sy: number): { x: number; y: number } {
    if (!canvas) return { x: 0, y: 0 };
    const width = canvas.width;
    const height = canvas.height;
    return {
      x: (sx - transform.x - width / 2) / transform.scale + width / 2,
      y: (sy - transform.y - height / 2) / transform.scale + height / 2
    };
  }

  function findNodeAt(x: number, y: number): string | null {
    for (const [id, node] of nodes) {
      const dx = node.x - x;
      const dy = node.y - y;
      // Larger hit area for easier clicking
      const hitRadius = Math.max(node.radius * 1.5, 15);
      if (dx * dx + dy * dy <= hitRadius * hitRadius) {
        return id;
      }
    }
    return null;
  }

  function handleMouseDown(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const { x, y } = screenToWorld(sx, sy);

    const nodeId = findNodeAt(x, y);
    if (nodeId) {
      draggedNode = nodeId;
      worker?.postMessage({ type: 'drag', payload: { nodeId, phase: 'start' } });
    } else {
      isPanning = true;
      dragStart = { x: e.clientX - transform.x, y: e.clientY - transform.y };
    }
    isDragging = true;
  }

  function handleMouseMove(e: MouseEvent) {
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const { x, y } = screenToWorld(sx, sy);

    if (draggedNode) {
      worker?.postMessage({ type: 'drag', payload: { nodeId: draggedNode, x, y, phase: 'drag' } });
    } else if (isPanning) {
      transform.x = e.clientX - dragStart.x;
      transform.y = e.clientY - dragStart.y;
      requestAnimationFrame(render);
    } else {
      const nodeId = findNodeAt(x, y);
      if (nodeId !== hoveredNode) {
        hoveredNode = nodeId;
        canvas.style.cursor = nodeId ? 'pointer' : 'grab';
        requestAnimationFrame(render);
      }
    }
  }

  function handleMouseUp() {
    if (draggedNode) {
      worker?.postMessage({ type: 'drag', payload: { nodeId: draggedNode, phase: 'end' } });
      draggedNode = null;
    }
    isPanning = false;
    isDragging = false;
  }

  function handleClick(e: MouseEvent) {
    if (!canvas || isDragging) return;
    const rect = canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;
    const { x, y } = screenToWorld(sx, sy);
    const nodeId = findNodeAt(x, y);

    if (nodeId) {
      selectedNode = selectedNode === nodeId ? null : nodeId;
      requestAnimationFrame(render);
    }
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const sx = e.clientX - rect.left;
    const sy = e.clientY - rect.top;

    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    const newScale = Math.max(0.1, Math.min(8, transform.scale * delta));

    const scaleChange = newScale / transform.scale;
    transform.x = sx - (sx - transform.x) * scaleChange;
    transform.y = sy - (sy - transform.y) * scaleChange;
    transform.scale = newScale;

    requestAnimationFrame(render);
  }

  function resetView() {
    transform = { x: 0, y: 0, scale: 1 };
    requestAnimationFrame(render);
  }

  function zoomIn() {
    transform.scale = Math.min(8, transform.scale * 1.3);
    requestAnimationFrame(render);
  }

  function zoomOut() {
    transform.scale = Math.max(0.1, transform.scale / 1.3);
    requestAnimationFrame(render);
  }

  function fitToView() {
    if (nodes.size === 0) return;

    let minX = Infinity, maxX = -Infinity;
    let minY = Infinity, maxY = -Infinity;

    nodes.forEach(node => {
      minX = Math.min(minX, node.x - node.radius);
      maxX = Math.max(maxX, node.x + node.radius);
      minY = Math.min(minY, node.y - node.radius);
      maxY = Math.max(maxY, node.y + node.radius);
    });

    if (!canvas) return;
    const padding = 60;
    const graphWidth = maxX - minX + padding * 2;
    const graphHeight = maxY - minY + padding * 2;

    const scaleX = canvas.width / graphWidth;
    const scaleY = canvas.height / graphHeight;
    transform.scale = Math.min(scaleX, scaleY, 2);

    const centerX = (minX + maxX) / 2;
    const centerY = (minY + maxY) / 2;
    transform.x = canvas.width / 2 - centerX * transform.scale;
    transform.y = canvas.height / 2 - centerY * transform.scale;

    requestAnimationFrame(render);
  }

  function reheat() {
    worker?.postMessage({ type: 'restart', payload: { alpha: 1 } });
  }

  function setLayout(layout: 'force' | 'radial' | 'grid') {
    currentLayout = layout;
    if (layout === 'force') {
      worker?.postMessage({ type: 'restart', payload: { alpha: 1 } });
    } else {
      worker?.postMessage({ type: 'layout', payload: { type: layout } });
    }
  }

  // Export graph as high-resolution PNG
  async function exportGraph(scale: number = 2) {
    if (!canvas || nodes.size === 0) return;
    exporting = true;

    try {
      // Calculate bounds of all nodes with padding
      let minX = Infinity, maxX = -Infinity;
      let minY = Infinity, maxY = -Infinity;

      nodes.forEach(node => {
        minX = Math.min(minX, node.x - node.radius - 50);
        maxX = Math.max(maxX, node.x + node.radius + 50);
        minY = Math.min(minY, node.y - node.radius - 50);
        maxY = Math.max(maxY, node.y + node.radius + 50);
      });

      const padding = 100;
      const exportWidth = (maxX - minX + padding * 2) * scale;
      const exportHeight = (maxY - minY + padding * 2) * scale;

      // Create export canvas
      const exportCanvas = document.createElement('canvas');
      exportCanvas.width = exportWidth;
      exportCanvas.height = exportHeight;
      const exportCtx = exportCanvas.getContext('2d');
      if (!exportCtx) throw new Error('Failed to create export context');

      // Fill background
      exportCtx.fillStyle = colors.background;
      exportCtx.fillRect(0, 0, exportWidth, exportHeight);

      // Transform to center the graph
      exportCtx.save();
      exportCtx.scale(scale, scale);
      exportCtx.translate(-minX + padding, -minY + padding);

      // Draw edges
      edges.forEach(edge => {
        if (!edge.source || !edge.target) return;
        exportCtx.strokeStyle = colors.edge;
        exportCtx.lineWidth = Math.max(1, Math.sqrt(edge.flight_count || 1) * 1.5);
        exportCtx.beginPath();
        exportCtx.moveTo(edge.source.x, edge.source.y);
        exportCtx.lineTo(edge.target.x, edge.target.y);
        exportCtx.stroke();
      });

      // Draw nodes with labels
      nodes.forEach((node) => {
        const highFlights = (node.total_flights || 0) > 50;

        // Node circle
        exportCtx.beginPath();
        exportCtx.arc(node.x, node.y, node.radius, 0, Math.PI * 2);
        exportCtx.fillStyle = highFlights ? colors.nodeHighFlight : colors.node;
        exportCtx.fill();

        // Label
        exportCtx.fillStyle = colors.text;
        exportCtx.font = '12px -apple-system, BlinkMacSystemFont, sans-serif';
        exportCtx.textAlign = 'center';
        exportCtx.textBaseline = 'top';
        exportCtx.fillText(node.label || 'Unknown', node.x, node.y + node.radius + 6);

        // Flight count
        exportCtx.fillStyle = colors.textSecondary;
        exportCtx.font = '10px -apple-system, BlinkMacSystemFont, sans-serif';
        exportCtx.fillText(`${node.total_flights} flights`, node.x, node.y + node.radius + 22);
      });

      exportCtx.restore();

      // Add title and metadata
      exportCtx.fillStyle = colors.text;
      exportCtx.font = `bold ${24 * scale}px -apple-system, BlinkMacSystemFont, sans-serif`;
      exportCtx.textAlign = 'left';
      exportCtx.fillText('Passenger Network Graph', 20 * scale, 30 * scale);

      exportCtx.fillStyle = colors.textSecondary;
      exportCtx.font = `${14 * scale}px -apple-system, BlinkMacSystemFont, sans-serif`;
      exportCtx.fillText(`${nodes.size} passengers, ${edges.length} connections`, 20 * scale, 55 * scale);
      exportCtx.fillText(`Exported: ${new Date().toLocaleString()}`, 20 * scale, 75 * scale);

      // Download
      const link = document.createElement('a');
      link.download = `network-graph-${new Date().toISOString().split('T')[0]}.png`;
      link.href = exportCanvas.toDataURL('image/png');
      link.click();
    } catch (err) {
      console.error('Export failed:', err);
      alert('Failed to export graph: ' + err);
    } finally {
      exporting = false;
    }
  }

  // Export as SVG for vector quality
  async function exportSVG() {
    if (nodes.size === 0) return;
    exporting = true;

    try {
      // Calculate bounds
      let minX = Infinity, maxX = -Infinity;
      let minY = Infinity, maxY = -Infinity;

      nodes.forEach(node => {
        minX = Math.min(minX, node.x - node.radius - 50);
        maxX = Math.max(maxX, node.x + node.radius + 50);
        minY = Math.min(minY, node.y - node.radius - 50);
        maxY = Math.max(maxY, node.y + node.radius + 50);
      });

      const padding = 100;
      const width = maxX - minX + padding * 2;
      const height = maxY - minY + padding * 2;

      // Build SVG string
      let svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
  <rect width="100%" height="100%" fill="${colors.background}"/>
  <g transform="translate(${-minX + padding}, ${-minY + padding})">
    <!-- Edges -->
    <g stroke="${colors.edge}" fill="none">`;

      edges.forEach(edge => {
        if (!edge.source || !edge.target) return;
        const strokeWidth = Math.max(1, Math.sqrt(edge.flight_count || 1) * 1.5);
        svg += `\n      <line x1="${edge.source.x}" y1="${edge.source.y}" x2="${edge.target.x}" y2="${edge.target.y}" stroke-width="${strokeWidth}"/>`;
      });

      svg += `\n    </g>\n    <!-- Nodes -->\n    <g>`;

      nodes.forEach((node) => {
        const highFlights = (node.total_flights || 0) > 50;
        const fillColor = highFlights ? colors.nodeHighFlight : colors.node;
        svg += `\n      <circle cx="${node.x}" cy="${node.y}" r="${node.radius}" fill="${fillColor}"/>`;
        svg += `\n      <text x="${node.x}" y="${node.y + node.radius + 16}" text-anchor="middle" fill="${colors.text}" font-size="12" font-family="-apple-system, BlinkMacSystemFont, sans-serif">${node.label || 'Unknown'}</text>`;
        svg += `\n      <text x="${node.x}" y="${node.y + node.radius + 30}" text-anchor="middle" fill="${colors.textSecondary}" font-size="10" font-family="-apple-system, BlinkMacSystemFont, sans-serif">${node.total_flights} flights</text>`;
      });

      svg += `\n    </g>\n  </g>\n</svg>`;

      // Download
      const blob = new Blob([svg], { type: 'image/svg+xml' });
      const link = document.createElement('a');
      link.download = `network-graph-${new Date().toISOString().split('T')[0]}.svg`;
      link.href = URL.createObjectURL(blob);
      link.click();
      URL.revokeObjectURL(link.href);
    } catch (err) {
      console.error('SVG export failed:', err);
      alert('Failed to export SVG: ' + err);
    } finally {
      exporting = false;
    }
  }

  function handleResize() {
    if (!canvas || !container) return;
    const width = container.clientWidth || container.offsetWidth || 800;
    const height = container.clientHeight || container.offsetHeight || 600;
    canvas.width = width;
    canvas.height = height;
    worker?.postMessage({ type: 'resize', payload: { width, height } });
    requestAnimationFrame(render);
  }

  let lastDataHash = '';
  $effect(() => {
    const hash = JSON.stringify(data?.nodes?.length) + JSON.stringify(data?.edges?.length);
    if (hash !== lastDataHash && data?.nodes?.length) {
      lastDataHash = hash;
      // Apply initial filter and start simulation
      applyFilter();
      setTimeout(() => startSimulation(), 100);
    }
  });

  // Re-apply filter when filterTopN or showAllNodes changes
  $effect(() => {
    if (data?.nodes?.length) {
      // Capture current filter values
      const _filterTopN = filterTopN;
      const _showAll = showAllNodes;
      applyFilter();
      setTimeout(() => startSimulation(), 50);
    }
  });

  onMount(() => {
    if (canvas && container) {
      // Ensure container has dimensions - use fallbacks if needed
      const width = container.clientWidth || container.offsetWidth || 800;
      const height = container.clientHeight || container.offsetHeight || 600;

      canvas.width = width;
      canvas.height = height;
      ctx = canvas.getContext('2d');

      console.log('[NetworkGraph] Initialized with dimensions:', width, 'x', height);
      console.log('[NetworkGraph] Data:', data?.nodes?.length, 'nodes,', data?.edges?.length, 'edges');

      initWorker();

      if (data?.nodes?.length) {
        // Small delay to ensure layout is complete
        setTimeout(() => startSimulation(), 100);
      }

      window.addEventListener('resize', handleResize);
    }
  });

  onDestroy(() => {
    worker?.terminate();
    worker = null;
    window.removeEventListener('resize', handleResize);
  });
</script>

{#if error}
  <div class="flex items-center justify-center w-full h-full bg-red-900/20 rounded-lg p-6">
    <div class="text-center">
      <p class="text-red-400 font-semibold mb-2">Graph Error</p>
      <p class="text-red-300 text-sm">{error}</p>
    </div>
  </div>
{:else if !data || !data.nodes || data.nodes.length === 0}
  <div class="flex items-center justify-center w-full h-full bg-slate-900 rounded-lg p-6">
    <div class="text-center">
      <div class="text-5xl mb-4">🕸️</div>
      <p class="text-gray-400 text-lg">No network data available</p>
      <p class="text-gray-500 text-sm mt-2">Add passengers with shared flights to see connections</p>
    </div>
  </div>
{:else}
  <div bind:this={container} class="relative w-full h-full min-h-[600px] bg-slate-900 overflow-hidden">
    <canvas
      bind:this={canvas}
      class="absolute inset-0 w-full h-full"
      style="cursor: {isDragging ? 'grabbing' : hoveredNode ? 'pointer' : 'grab'}"
      onmousedown={handleMouseDown}
      onmousemove={handleMouseMove}
      onmouseup={handleMouseUp}
      onmouseleave={handleMouseUp}
      onclick={handleClick}
      onwheel={handleWheel}
    ></canvas>

    <!-- Collapsible Legend Panel -->
    <div
      class="absolute top-4 left-4 z-20 transition-all duration-300 {legendOpen ? 'w-72' : 'w-10'}"
    >
      <!-- Toggle Button -->
      <button
        onclick={() => legendOpen = !legendOpen}
        class="absolute top-0 {legendOpen ? 'right-0' : 'left-0'} w-10 h-10 bg-slate-800 hover:bg-slate-700 text-white rounded-lg flex items-center justify-center shadow-lg border border-slate-600 z-10"
        title={legendOpen ? 'Collapse Legend' : 'Expand Legend'}
      >
        {legendOpen ? '◀' : '▶'}
      </button>

      {#if legendOpen}
        <div class="bg-slate-800/95 backdrop-blur-sm rounded-xl shadow-xl border border-slate-700 overflow-hidden mr-2">
          <!-- Tab Headers -->
          <div class="flex border-b border-slate-700">
            <button
              onclick={() => legendTab = 'people'}
              class="flex-1 px-4 py-2 text-sm font-medium transition {legendTab === 'people' ? 'bg-slate-700 text-white' : 'text-gray-400 hover:text-white'}"
            >People</button>
            <button
              onclick={() => legendTab = 'stats'}
              class="flex-1 px-4 py-2 text-sm font-medium transition {legendTab === 'stats' ? 'bg-slate-700 text-white' : 'text-gray-400 hover:text-white'}"
            >Stats</button>
          </div>

          <!-- Tab Content -->
          <div class="max-h-[400px] overflow-y-auto">
            {#if legendTab === 'people'}
              <!-- Filter Controls -->
              <div class="p-3 border-b border-slate-700 space-y-2">
                <div class="flex items-center gap-2">
                  <label class="text-xs text-gray-400">Show:</label>
                  <select
                    bind:value={filterTopN}
                    disabled={showAllNodes}
                    class="flex-1 px-2 py-1 text-sm bg-slate-700 border border-slate-600 rounded text-white disabled:opacity-50"
                  >
                    <option value={5}>Top 5</option>
                    <option value={10}>Top 10</option>
                    <option value={20}>Top 20</option>
                    <option value={50}>Top 50</option>
                    <option value={100}>Top 100</option>
                  </select>
                </div>
                <label class="flex items-center gap-2 text-xs text-gray-400 cursor-pointer">
                  <input type="checkbox" bind:checked={showAllNodes} class="rounded bg-slate-700 border-slate-600" />
                  Show all ({data?.nodes?.length || 0} passengers)
                </label>
              </div>

              <!-- Passenger List -->
              <div class="divide-y divide-slate-700/50">
                {#each getSortedPassengers() as passenger, idx}
                  <button
                    onclick={() => togglePassengerVisibility(passenger.id)}
                    class="w-full px-3 py-2 flex items-center gap-2 hover:bg-slate-700/50 transition text-left"
                  >
                    <span class="w-5 h-5 flex items-center justify-center rounded text-xs font-mono {passenger.visible ? 'bg-blue-600 text-white' : 'bg-slate-600 text-gray-400'}">
                      {passenger.visible ? '✓' : idx + 1}
                    </span>
                    <div class="flex-1 min-w-0">
                      <p class="text-sm text-white truncate {!passenger.visible && 'opacity-50'}">{passenger.label}</p>
                      <p class="text-xs text-gray-500">{passenger.total_flights} flights</p>
                    </div>
                    {#if passenger.total_flights > 50}
                      <span class="w-2 h-2 rounded-full bg-green-500" title="High-frequency traveler"></span>
                    {/if}
                  </button>
                {/each}
              </div>
            {:else}
              <!-- Stats Tab -->
              <div class="p-4 space-y-4">
                <div>
                  <p class="text-xs text-gray-400 uppercase tracking-wider mb-1">Total Passengers</p>
                  <p class="text-2xl font-bold text-white">{data?.nodes?.length || 0}</p>
                </div>
                <div>
                  <p class="text-xs text-gray-400 uppercase tracking-wider mb-1">Visible</p>
                  <p class="text-2xl font-bold text-blue-400">{filteredData?.nodes?.length || 0}</p>
                </div>
                <div>
                  <p class="text-xs text-gray-400 uppercase tracking-wider mb-1">Connections</p>
                  <p class="text-2xl font-bold text-green-400">{filteredData?.edges?.length || 0}</p>
                </div>
                <div class="pt-2 border-t border-slate-700">
                  <p class="text-xs text-gray-400 uppercase tracking-wider mb-2">Color Legend</p>
                  <div class="space-y-1">
                    <div class="flex items-center gap-2">
                      <span class="w-3 h-3 rounded-full bg-blue-500"></span>
                      <span class="text-xs text-gray-300">Standard traveler</span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="w-3 h-3 rounded-full bg-green-500"></span>
                      <span class="text-xs text-gray-300">50+ flights</span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="w-3 h-3 rounded-full bg-amber-500"></span>
                      <span class="text-xs text-gray-300">Selected</span>
                    </div>
                  </div>
                </div>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- Top Controls Bar -->
    <div class="absolute top-4 left-80 flex gap-2 z-10">
      <!-- Layout Selector -->
      <div class="flex gap-1 bg-slate-800/90 rounded-lg p-1 border border-slate-700">
        <button
          onclick={() => setLayout('force')}
          class="px-3 py-1.5 rounded text-sm font-medium transition {currentLayout === 'force' ? 'bg-blue-600 text-white' : 'text-gray-300 hover:bg-slate-700'}"
        >Force</button>
        <button
          onclick={() => setLayout('radial')}
          class="px-3 py-1.5 rounded text-sm font-medium transition {currentLayout === 'radial' ? 'bg-blue-600 text-white' : 'text-gray-300 hover:bg-slate-700'}"
        >Radial</button>
        <button
          onclick={() => setLayout('grid')}
          class="px-3 py-1.5 rounded text-sm font-medium transition {currentLayout === 'grid' ? 'bg-blue-600 text-white' : 'text-gray-300 hover:bg-slate-700'}"
        >Grid</button>
      </div>

      <!-- Export Buttons -->
      <div class="flex gap-1 bg-slate-800/90 rounded-lg p-1 border border-slate-700">
        <button
          onclick={() => exportGraph(2)}
          disabled={exporting}
          class="px-3 py-1.5 rounded text-sm font-medium transition text-gray-300 hover:bg-slate-700 disabled:opacity-50"
          title="Export as PNG (2x resolution)"
        >
          {exporting ? '...' : 'PNG'}
        </button>
        <button
          onclick={() => exportGraph(4)}
          disabled={exporting}
          class="px-3 py-1.5 rounded text-sm font-medium transition text-gray-300 hover:bg-slate-700 disabled:opacity-50"
          title="Export as PNG (4x resolution - large)"
        >
          PNG 4x
        </button>
        <button
          onclick={exportSVG}
          disabled={exporting}
          class="px-3 py-1.5 rounded text-sm font-medium transition text-gray-300 hover:bg-slate-700 disabled:opacity-50"
          title="Export as SVG (vector - infinite scale)"
        >
          SVG
        </button>
      </div>
    </div>

    <!-- Zoom Controls -->
    <div class="absolute top-4 right-4 flex flex-col gap-2 z-10">
      <button
        onclick={zoomIn}
        class="w-10 h-10 bg-slate-800 hover:bg-slate-700 text-white rounded-lg flex items-center justify-center text-xl font-bold shadow-lg border border-slate-600"
        title="Zoom In"
      >+</button>
      <button
        onclick={zoomOut}
        class="w-10 h-10 bg-slate-800 hover:bg-slate-700 text-white rounded-lg flex items-center justify-center text-xl font-bold shadow-lg border border-slate-600"
        title="Zoom Out"
      >−</button>
      <button
        onclick={fitToView}
        class="w-10 h-10 bg-slate-800 hover:bg-slate-700 text-white rounded-lg flex items-center justify-center shadow-lg border border-slate-600"
        title="Fit to View"
      >⊡</button>
      <button
        onclick={resetView}
        class="w-10 h-10 bg-slate-800 hover:bg-slate-700 text-white rounded-lg flex items-center justify-center shadow-lg border border-slate-600"
        title="Reset View"
      >⌂</button>
      <div class="h-px bg-slate-600 my-1"></div>
      <button
        onclick={reheat}
        class="w-10 h-10 bg-blue-600 hover:bg-blue-500 text-white rounded-lg flex items-center justify-center shadow-lg"
        title="Reheat Simulation"
      >↻</button>
    </div>

    <!-- Selected Node Panel -->
    {#if selectedNode && nodes.get(selectedNode)}
      {@const node = nodes.get(selectedNode)}
      <div class="absolute bottom-4 left-4 bg-slate-800/95 backdrop-blur-sm rounded-xl p-5 max-w-xs shadow-xl border border-slate-700 z-10">
        <div class="flex items-center justify-between mb-3">
          <h4 class="font-bold text-white text-lg">{node?.label}</h4>
          <button onclick={() => selectedNode = null} class="text-gray-400 hover:text-white text-xl leading-none">&times;</button>
        </div>
        <div class="text-sm space-y-2">
          <p class="text-gray-300">
            Flights: <span class="text-blue-400 font-semibold">{node?.total_flights}</span>
          </p>
        </div>
      </div>
    {/if}

    <!-- Instructions -->
    <div class="absolute bottom-4 right-4 text-xs text-gray-500 bg-slate-800/80 backdrop-blur-sm px-3 py-2 rounded-lg z-10">
      Scroll to zoom &bull; Drag to pan &bull; Click nodes to select &bull; Drag nodes to reposition
    </div>
  </div>
{/if}
