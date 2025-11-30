<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { theme } from '$lib/theme';

  // Agent tracking types
  interface AgentEvent {
    agent_name: string;
    model: string;
    event_type: 'start' | 'thinking' | 'executing' | 'token_update' | 'complete' | 'error';
    operation?: string;
    tokens_input?: number;
    tokens_output?: number;
    cost_usd?: number;
    timestamp: string;
  }

  interface AgentState {
    agent_name: string;
    model: string;
    status: 'idle' | 'thinking' | 'executing' | 'complete' | 'error';
    current_operation?: string;
    tokens_input: number;
    tokens_output: number;
    cost_usd: number;
    started_at: string;
    last_updated: string;
  }

  // Component props
  interface Props {
    initialOpen?: boolean;
    position?: 'bottom-right' | 'top-right' | 'side';
  }

  let { initialOpen = false, position = 'bottom-right' }: Props = $props();

  // State
  let isOpen = $state(initialOpen);
  let agents = $state<Map<string, AgentState>>(new Map());
  let eventHistory = $state<AgentEvent[]>([]);
  let unlisten: UnlistenFn | null = null;
  let totalCost = $derived(
    Array.from(agents.values()).reduce((sum, agent) => sum + agent.cost_usd, 0)
  );
  let totalTokens = $derived(
    Array.from(agents.values()).reduce(
      (sum, agent) => sum + agent.tokens_input + agent.tokens_output,
      0
    )
  );

  // Format currency
  function formatCost(cost: number): string {
    if (cost < 0.01) return `$${(cost * 1000).toFixed(4)}‰`;
    return `$${cost.toFixed(4)}`;
  }

  // Format tokens with K/M suffix
  function formatTokens(tokens: number): string {
    if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(2)}M`;
    if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}K`;
    return tokens.toString();
  }

  // Get status color
  function getStatusColor(status: string): string {
    switch (status) {
      case 'thinking':
        return 'status-thinking';
      case 'executing':
        return 'status-executing';
      case 'complete':
        return 'status-complete';
      case 'error':
        return 'status-error';
      default:
        return 'status-idle';
    }
  }

  // Get status icon
  function getStatusIcon(status: string): string {
    switch (status) {
      case 'thinking':
        return '🤔';
      case 'executing':
        return '⚡';
      case 'complete':
        return '✓';
      case 'error':
        return '✕';
      default:
        return '○';
    }
  }

  // Handle agent events
  function handleAgentEvent(event: AgentEvent) {
    const key = `${event.agent_name}-${event.model}`;

    // Add to event history (keep last 50)
    eventHistory.unshift(event);
    if (eventHistory.length > 50) {
      eventHistory = eventHistory.slice(0, 50);
    }

    // Update or create agent state
    const existing = agents.get(key);

    if (event.event_type === 'start') {
      agents.set(key, {
        agent_name: event.agent_name,
        model: event.model,
        status: 'thinking',
        current_operation: event.operation,
        tokens_input: 0,
        tokens_output: 0,
        cost_usd: 0,
        started_at: event.timestamp,
        last_updated: event.timestamp,
      });
    } else if (existing) {
      existing.last_updated = event.timestamp;

      if (event.event_type === 'complete') {
        existing.status = 'complete';
        existing.tokens_input += event.tokens_input || 0;
        existing.tokens_output += event.tokens_output || 0;
        existing.cost_usd += event.cost_usd || 0;
      } else if (event.event_type === 'error') {
        existing.status = 'error';
        existing.current_operation = event.operation;
      } else if (event.event_type === 'thinking') {
        existing.status = 'thinking';
      } else if (event.event_type === 'executing') {
        existing.status = 'executing';
        existing.current_operation = event.operation;
      } else if (event.event_type === 'token_update') {
        existing.tokens_input += event.tokens_input || 0;
        existing.tokens_output += event.tokens_output || 0;
        existing.cost_usd += event.cost_usd || 0;
      }

      agents.set(key, existing);
    }

    // Trigger reactivity
    agents = new Map(agents);
  }

  // Setup event listener
  onMount(async () => {
    unlisten = await listen<AgentEvent>('agent:status', (event) => {
      handleAgentEvent(event.payload);
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  // Clear history
  function clearHistory() {
    agents.clear();
    agents = new Map();
    eventHistory = [];
  }

  const agentList = $derived(Array.from(agents.values()));
</script>

<!-- Floating Agent Tracker -->
{#if isOpen}
  <div
    class="agent-tracker-container fixed z-50 rounded-lg shadow-2xl {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'} {position ===
    'bottom-right'
      ? 'bottom-4 right-4 w-96'
      : position === 'top-right'
        ? 'top-4 right-4 w-96'
        : 'top-0 right-0 h-full w-80 rounded-none border-t-0 border-r-0 border-b-0'}"
  >
    <!-- Header -->
    <div class="tracker-header flex items-center justify-between px-4 py-3">
      <div class="flex items-center gap-2">
        <div class="status-indicator w-2 h-2 rounded-full animate-pulse"></div>
        <h3 class="header-title font-bold text-xs uppercase tracking-wider">Agent Monitor</h3>
      </div>
      <div class="flex items-center gap-2">
        <button
          onclick={clearHistory}
          class="header-btn text-xs"
          title="Clear History"
        >
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
            />
          </svg>
        </button>
        <button
          onclick={() => (isOpen = false)}
          class="header-btn text-xs font-bold"
        >
          ✕
        </button>
      </div>
    </div>

    <!-- Summary Stats -->
    <div class="stats-section grid grid-cols-2 gap-2 p-3">
      <div class="stat-card rounded p-2">
        <div class="stat-label text-[10px] uppercase tracking-wider mb-1">Total Cost</div>
        <div class="stat-value-cost text-lg font-bold">{formatCost(totalCost)}</div>
      </div>
      <div class="stat-card rounded p-2">
        <div class="stat-label text-[10px] uppercase tracking-wider mb-1">Total Tokens</div>
        <div class="stat-value-tokens text-lg font-bold">{formatTokens(totalTokens)}</div>
      </div>
    </div>

    <!-- Active Agents -->
    <div class="agents-list overflow-y-auto max-h-96 p-3 space-y-2">
      {#if agentList.length === 0}
        <div class="empty-state text-center py-8 text-xs">
          <div class="text-2xl mb-2">🤖</div>
          <p>No active agents</p>
          <p class="text-[10px] mt-1">Run a workflow with AI nodes to see activity</p>
        </div>
      {:else}
        {#each agentList as agent (agent.agent_name + agent.model)}
          <div class="agent-card rounded p-3 space-y-2">
            <!-- Agent Header -->
            <div class="flex items-start justify-between">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2 mb-1">
                  <span class="text-lg">{getStatusIcon(agent.status)}</span>
                  <span class="agent-name text-xs font-bold truncate">{agent.agent_name}</span>
                </div>
                <div class="agent-model text-[10px] truncate">{agent.model}</div>
              </div>
              <div class={`text-xs font-bold uppercase ${getStatusColor(agent.status)}`}>
                {agent.status}
              </div>
            </div>

            <!-- Current Operation -->
            {#if agent.current_operation}
              <div class="operation-text text-[10px] rounded px-2 py-1">
                {agent.current_operation}
              </div>
            {/if}

            <!-- Metrics -->
            <div class="grid grid-cols-3 gap-2 text-center">
              <div>
                <div class="metric-label text-[9px] uppercase tracking-wide">Input</div>
                <div class="metric-input text-xs font-mono">
                  {formatTokens(agent.tokens_input)}
                </div>
              </div>
              <div>
                <div class="metric-label text-[9px] uppercase tracking-wide">Output</div>
                <div class="metric-output text-xs font-mono">
                  {formatTokens(agent.tokens_output)}
                </div>
              </div>
              <div>
                <div class="metric-label text-[9px] uppercase tracking-wide">Cost</div>
                <div class="metric-cost text-xs font-mono">{formatCost(agent.cost_usd)}</div>
              </div>
            </div>

            <!-- Timing -->
            <div class="timing-info text-[9px] flex justify-between">
              <span>Started: {new Date(agent.started_at).toLocaleTimeString()}</span>
              {#if agent.status === 'complete' || agent.status === 'error'}
                <span
                  >Duration: {Math.round((new Date(agent.last_updated).getTime() - new Date(agent.started_at).getTime()) / 1000)}s</span
                >
              {/if}
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </div>
{/if}

<!-- Toggle Button (when closed) -->
{#if !isOpen}
  <button
    onclick={() => (isOpen = true)}
    class="toggle-btn fixed bottom-4 right-4 z-40 rounded-lg px-4 py-3 shadow-xl transition-all hover:shadow-2xl group {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}"
    title="Open Agent Monitor"
  >
    <div class="flex items-center gap-2">
      <div class="status-indicator w-2 h-2 rounded-full animate-pulse"></div>
      <span class="toggle-label text-xs font-bold">Agent Monitor</span>
      {#if totalCost > 0}
        <span class="toggle-cost text-xs font-mono ml-2">{formatCost(totalCost)}</span>
      {/if}
    </div>
  </button>
{/if}

<style>
  /* ===== DEFAULT THEME (Slate) ===== */
  .theme-default.agent-tracker-container,
  .theme-default.toggle-btn {
    background-color: #0f172a;
    border: 1px solid #334155;
  }

  .theme-default .tracker-header {
    background-color: rgba(2, 6, 23, 0.5);
    border-bottom: 1px solid #1e293b;
  }

  .theme-default .status-indicator {
    background-color: #22c55e;
  }

  .theme-default .header-title,
  .theme-default .toggle-label,
  .theme-default .agent-name {
    color: #ffffff;
  }

  .theme-default .header-btn {
    color: #64748b;
  }

  .theme-default .header-btn:hover {
    color: #ffffff;
  }

  .theme-default .stats-section {
    background-color: rgba(2, 6, 23, 0.3);
    border-bottom: 1px solid #1e293b;
  }

  .theme-default .stat-card,
  .theme-default .agent-card {
    background-color: rgba(15, 23, 42, 0.5);
    border: 1px solid #1e293b;
  }

  .theme-default .stat-label,
  .theme-default .agent-model,
  .theme-default .metric-label {
    color: #94a3b8;
  }

  .theme-default .stat-value-cost,
  .theme-default .metric-cost,
  .theme-default .toggle-cost {
    color: #34d399;
  }

  .theme-default .stat-value-tokens,
  .theme-default .metric-input {
    color: #60a5fa;
  }

  .theme-default .metric-output {
    color: #a78bfa;
  }

  .theme-default .operation-text {
    color: #94a3b8;
    background-color: #0f172a;
    border: 1px solid #1e293b;
  }

  .theme-default .timing-info,
  .theme-default .empty-state {
    color: #64748b;
  }

  .theme-default .status-thinking { color: #60a5fa; }
  .theme-default .status-executing { color: #a78bfa; }
  .theme-default .status-complete { color: #4ade80; }
  .theme-default .status-error { color: #f87171; }
  .theme-default .status-idle { color: #94a3b8; }

  /* ===== SKYNET THEME ===== */
  .theme-skynet.agent-tracker-container,
  .theme-skynet.toggle-btn {
    background-color: #000000;
    border: 1px solid #00b4ff;
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .tracker-header {
    background-color: rgba(0, 0, 0, 0.8);
    border-bottom: 1px solid #0080ff;
  }

  .theme-skynet .status-indicator {
    background-color: #00b4ff;
  }

  .theme-skynet .header-title,
  .theme-skynet .toggle-label,
  .theme-skynet .agent-name {
    color: #00b4ff;
  }

  .theme-skynet .header-btn {
    color: #0080ff;
  }

  .theme-skynet .header-btn:hover {
    color: #00b4ff;
  }

  .theme-skynet .stats-section {
    background-color: rgba(0, 0, 0, 0.5);
    border-bottom: 1px solid #0080ff;
  }

  .theme-skynet .stat-card,
  .theme-skynet .agent-card {
    background-color: rgba(0, 0, 0, 0.5);
    border: 1px solid #0080ff;
  }

  .theme-skynet .stat-label,
  .theme-skynet .agent-model,
  .theme-skynet .metric-label {
    color: #0080ff;
  }

  .theme-skynet .stat-value-cost,
  .theme-skynet .metric-cost,
  .theme-skynet .toggle-cost {
    color: #00ff88;
  }

  .theme-skynet .stat-value-tokens,
  .theme-skynet .metric-input {
    color: #00b4ff;
  }

  .theme-skynet .metric-output {
    color: #0080ff;
  }

  .theme-skynet .operation-text {
    color: #0080ff;
    background-color: #000000;
    border: 1px solid #0080ff;
  }

  .theme-skynet .timing-info,
  .theme-skynet .empty-state {
    color: #005299;
  }

  .theme-skynet .status-thinking { color: #00b4ff; }
  .theme-skynet .status-executing { color: #0080ff; }
  .theme-skynet .status-complete { color: #00ff88; }
  .theme-skynet .status-error { color: #ff4444; }
  .theme-skynet .status-idle { color: #005299; }

  /* ===== CYBERPUNK THEME ===== */
  .theme-cyberpunk.agent-tracker-container,
  .theme-cyberpunk.toggle-btn {
    background-color: #0d0d0d;
    border: 1px solid #00d9ff;
    box-shadow: 0 0 20px rgba(0, 217, 255, 0.3), 0 0 40px rgba(255, 0, 128, 0.1);
  }

  .theme-cyberpunk .tracker-header {
    background-color: rgba(13, 13, 13, 0.9);
    border-bottom: 1px solid #ff0080;
  }

  .theme-cyberpunk .status-indicator {
    background-color: #ff0080;
  }

  .theme-cyberpunk .header-title,
  .theme-cyberpunk .toggle-label,
  .theme-cyberpunk .agent-name {
    color: #00d9ff;
  }

  .theme-cyberpunk .header-btn {
    color: #ff0080;
  }

  .theme-cyberpunk .header-btn:hover {
    color: #00d9ff;
  }

  .theme-cyberpunk .stats-section {
    background-color: rgba(13, 13, 13, 0.5);
    border-bottom: 1px solid #ff0080;
  }

  .theme-cyberpunk .stat-card,
  .theme-cyberpunk .agent-card {
    background-color: rgba(13, 13, 13, 0.5);
    border: 1px solid #b000ff;
  }

  .theme-cyberpunk .stat-label,
  .theme-cyberpunk .agent-model,
  .theme-cyberpunk .metric-label {
    color: #b000ff;
  }

  .theme-cyberpunk .stat-value-cost,
  .theme-cyberpunk .metric-cost,
  .theme-cyberpunk .toggle-cost {
    color: #00ff88;
  }

  .theme-cyberpunk .stat-value-tokens,
  .theme-cyberpunk .metric-input {
    color: #00d9ff;
  }

  .theme-cyberpunk .metric-output {
    color: #ff0080;
  }

  .theme-cyberpunk .operation-text {
    color: #b000ff;
    background-color: #0d0d0d;
    border: 1px solid #ff0080;
  }

  .theme-cyberpunk .timing-info,
  .theme-cyberpunk .empty-state {
    color: #b000ff;
  }

  .theme-cyberpunk .status-thinking { color: #00d9ff; }
  .theme-cyberpunk .status-executing { color: #ff0080; }
  .theme-cyberpunk .status-complete { color: #00ff88; }
  .theme-cyberpunk .status-error { color: #ff4444; }
  .theme-cyberpunk .status-idle { color: #b000ff; }

  /* ===== SCROLLBAR STYLES ===== */
  .theme-default .agents-list::-webkit-scrollbar,
  .theme-skynet .agents-list::-webkit-scrollbar,
  .theme-cyberpunk .agents-list::-webkit-scrollbar {
    width: 6px;
  }

  .theme-default .agents-list::-webkit-scrollbar-track {
    background: #0f172a;
  }

  .theme-default .agents-list::-webkit-scrollbar-thumb {
    background: #475569;
    border-radius: 3px;
  }

  .theme-skynet .agents-list::-webkit-scrollbar-track {
    background: #000000;
  }

  .theme-skynet .agents-list::-webkit-scrollbar-thumb {
    background: #0080ff;
    border-radius: 3px;
  }

  .theme-cyberpunk .agents-list::-webkit-scrollbar-track {
    background: #0d0d0d;
  }

  .theme-cyberpunk .agents-list::-webkit-scrollbar-thumb {
    background: #ff0080;
    border-radius: 3px;
  }
</style>
