// Web Worker for D3 Force Simulation
// Offloads heavy computation from main thread

interface SimNode {
  id: string;
  label: string;
  total_flights: number;
  total_distance_km: number;
  x?: number;
  y?: number;
  vx?: number;
  vy?: number;
  fx?: number | null;
  fy?: number | null;
  radius: number;
  collapsed?: boolean;
  children?: SimNode[];
}

interface SimEdge {
  source: string | SimNode;
  target: string | SimNode;
  flight_count: number;
}

interface SimulationConfig {
  width: number;
  height: number;
  chargeStrength: number;
  linkDistance: number;
  collisionRadius: number;
}

let nodes: SimNode[] = [];
let edges: SimEdge[] = [];
let config: SimulationConfig = {
  width: 800,
  height: 600,
  chargeStrength: -400,
  linkDistance: 100,
  collisionRadius: 30
};

let alpha = 1;
let alphaTarget = 0;
let alphaDecay = 0.0228;
let alphaMin = 0.001;
let velocityDecay = 0.4;
let running = false;

function initializeNodes() {
  const centerX = config.width / 2;
  const centerY = config.height / 2;

  nodes.forEach((node, i) => {
    if (node.x === undefined) {
      const angle = (i / nodes.length) * 2 * Math.PI;
      const radius = Math.min(config.width, config.height) / 3;
      node.x = centerX + Math.cos(angle) * radius;
      node.y = centerY + Math.sin(angle) * radius;
    }
    node.vx = node.vx || 0;
    node.vy = node.vy || 0;
    node.radius = Math.max(8, Math.sqrt(node.total_flights || 1) * 5);
  });
}

function resolveEdges() {
  const nodeMap = new Map(nodes.map(n => [n.id, n]));
  edges = edges.map(e => ({
    ...e,
    source: typeof e.source === 'string' ? nodeMap.get(e.source) || e.source : e.source,
    target: typeof e.target === 'string' ? nodeMap.get(e.target) || e.target : e.target
  }));
}

function applyForces() {
  const centerX = config.width / 2;
  const centerY = config.height / 2;

  // Center force
  nodes.forEach(node => {
    if (node.fx === null || node.fx === undefined) {
      node.vx! += (centerX - node.x!) * 0.01;
    }
    if (node.fy === null || node.fy === undefined) {
      node.vy! += (centerY - node.y!) * 0.01;
    }
  });

  // Charge force (repulsion)
  for (let i = 0; i < nodes.length; i++) {
    for (let j = i + 1; j < nodes.length; j++) {
      const nodeA = nodes[i];
      const nodeB = nodes[j];
      const dx = nodeB.x! - nodeA.x!;
      const dy = nodeB.y! - nodeA.y!;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const force = config.chargeStrength / (dist * dist);
      const fx = (dx / dist) * force;
      const fy = (dy / dist) * force;

      if (nodeA.fx === null || nodeA.fx === undefined) {
        nodeA.vx! -= fx;
        nodeA.vy! -= fy;
      }
      if (nodeB.fx === null || nodeB.fx === undefined) {
        nodeB.vx! += fx;
        nodeB.vy! += fy;
      }
    }
  }

  // Link force
  edges.forEach(edge => {
    const source = edge.source as SimNode;
    const target = edge.target as SimNode;
    if (!source.x || !target.x) return;

    const dx = target.x - source.x;
    const dy = target.y! - source.y!;
    const dist = Math.sqrt(dx * dx + dy * dy) || 1;
    const diff = (dist - config.linkDistance) / dist;
    const strength = 0.3;
    const fx = dx * diff * strength;
    const fy = dy * diff * strength;

    if (source.fx === null || source.fx === undefined) {
      source.vx! += fx;
      source.vy! += fy;
    }
    if (target.fx === null || target.fx === undefined) {
      target.vx! -= fx;
      target.vy! -= fy;
    }
  });

  // Collision force
  for (let i = 0; i < nodes.length; i++) {
    for (let j = i + 1; j < nodes.length; j++) {
      const nodeA = nodes[i];
      const nodeB = nodes[j];
      const dx = nodeB.x! - nodeA.x!;
      const dy = nodeB.y! - nodeA.y!;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const minDist = nodeA.radius + nodeB.radius + 5;

      if (dist < minDist) {
        const diff = (minDist - dist) / dist * 0.5;
        const fx = dx * diff;
        const fy = dy * diff;

        if (nodeA.fx === null || nodeA.fx === undefined) {
          nodeA.x! -= fx;
          nodeA.y! -= fy;
        }
        if (nodeB.fx === null || nodeB.fx === undefined) {
          nodeB.x! += fx;
          nodeB.y! += fy;
        }
      }
    }
  }
}

function tick() {
  alpha += (alphaTarget - alpha) * alphaDecay;

  if (alpha < alphaMin) {
    running = false;
    self.postMessage({ type: 'end', nodes: nodes.map(n => ({
      id: n.id, x: n.x, y: n.y, radius: n.radius
    })) });
    return;
  }

  applyForces();

  // Apply velocity and decay
  nodes.forEach(node => {
    if (node.fx !== null && node.fx !== undefined) {
      node.x = node.fx;
      node.vx = 0;
    } else {
      node.vx! *= velocityDecay;
      node.x! += node.vx! * alpha;
    }
    if (node.fy !== null && node.fy !== undefined) {
      node.y = node.fy;
      node.vy = 0;
    } else {
      node.vy! *= velocityDecay;
      node.y! += node.vy! * alpha;
    }

    // Keep nodes in bounds
    const padding = 50;
    node.x = Math.max(padding, Math.min(config.width - padding, node.x!));
    node.y = Math.max(padding, Math.min(config.height - padding, node.y!));
  });

  // Send tick update
  self.postMessage({
    type: 'tick',
    nodes: nodes.map(n => ({ id: n.id, x: n.x, y: n.y, radius: n.radius })),
    edges: edges.map(e => ({
      source: { x: (e.source as SimNode).x, y: (e.source as SimNode).y },
      target: { x: (e.target as SimNode).x, y: (e.target as SimNode).y },
      flight_count: e.flight_count
    })),
    alpha
  });

  if (running) {
    setTimeout(tick, 16); // ~60fps
  }
}

self.onmessage = (e: MessageEvent) => {
  const { type, payload } = e.data;

  switch (type) {
    case 'init':
      nodes = payload.nodes.map((n: any) => ({ ...n, radius: Math.max(8, Math.sqrt(n.total_flights || 1) * 5) }));
      edges = payload.edges;
      config = { ...config, ...payload.config };
      initializeNodes();
      resolveEdges();
      alpha = 1;
      running = true;
      tick();
      break;

    case 'stop':
      running = false;
      break;

    case 'restart':
      alpha = payload?.alpha ?? 0.3;
      alphaTarget = 0;
      if (!running) {
        running = true;
        tick();
      }
      break;

    case 'drag':
      const node = nodes.find(n => n.id === payload.nodeId);
      if (node) {
        if (payload.phase === 'start') {
          node.fx = node.x;
          node.fy = node.y;
          alphaTarget = 0.3;
          if (!running) {
            running = true;
            tick();
          }
        } else if (payload.phase === 'drag') {
          node.fx = payload.x;
          node.fy = payload.y;
        } else if (payload.phase === 'end') {
          node.fx = null;
          node.fy = null;
          alphaTarget = 0;
        }
      }
      break;

    case 'resize':
      config.width = payload.width;
      config.height = payload.height;
      break;
  }
};

export {};
