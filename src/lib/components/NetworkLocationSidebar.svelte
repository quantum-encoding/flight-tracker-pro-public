<!-- NetworkLocationSidebar.svelte - Slide-out panel for network location details -->
<script lang="ts">
  import { slide } from 'svelte/transition';
  import { theme } from '$lib/theme';

  interface NetworkConnection {
    id: string;
    source: { lat: number; lng: number; name: string };
    target: { lat: number; lng: number; name: string };
    color?: string;
    process?: string;
    isAnomaly?: boolean;
  }

  interface LocationData {
    name: string;
    lat: number;
    lng: number;
    count: number;
    isLocal?: boolean;
  }

  interface Props {
    location: LocationData | null;
    connections: NetworkConnection[];
    onClose: () => void;
  }

  let { location, connections = [], onClose }: Props = $props();

  // Get connections for this location
  const locationConnections = $derived(() => {
    if (!location) return [];

    if (location.isLocal) {
      // For local machine, show all outbound connections
      return connections;
    } else {
      // For remote locations, show connections to this destination
      return connections.filter(c =>
        c.target.name === location.name ||
        (Math.abs(c.target.lat - location.lat) < 0.1 && Math.abs(c.target.lng - location.lng) < 0.1)
      );
    }
  });

  // Get unique processes connecting to this location
  const processBreakdown = $derived(() => {
    const processes = new Map<string, { count: number; hasAnomaly: boolean }>();

    for (const conn of locationConnections()) {
      const proc = conn.process || 'Unknown';
      const existing = processes.get(proc) || { count: 0, hasAnomaly: false };
      existing.count++;
      if (conn.isAnomaly) existing.hasAnomaly = true;
      processes.set(proc, existing);
    }

    return Array.from(processes.entries())
      .map(([name, data]) => ({ name, ...data }))
      .sort((a, b) => b.count - a.count);
  });

  // Count anomalies
  const anomalyCount = $derived(() => {
    return locationConnections().filter(c => c.isAnomaly).length;
  });

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }

  function getProcessIcon(name: string): string {
    const lower = name.toLowerCase();
    if (lower.includes('firefox') || lower.includes('chrome') || lower.includes('brave') || lower.includes('safari')) return '🌐';
    if (lower.includes('claude') || lower.includes('anthropic')) return '🤖';
    if (lower.includes('code') || lower.includes('vscode')) return '💻';
    if (lower.includes('docker') || lower.includes('container')) return '🐳';
    if (lower.includes('ssh') || lower.includes('terminal')) return '🖥️';
    if (lower.includes('git')) return '📦';
    if (lower.includes('node') || lower.includes('npm')) return '📗';
    if (lower.includes('python')) return '🐍';
    if (lower.includes('rust') || lower.includes('cargo')) return '🦀';
    return '📡';
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if location}
  <!-- Backdrop for click-away -->
  <div
    class="fixed inset-0 z-40"
    onclick={handleBackdropClick}
    role="button"
    tabindex="-1"
    aria-label="Close sidebar"
    onkeydown={(e) => e.key === 'Enter' && onClose()}
  ></div>

  <!-- Sidebar Panel -->
  <div
    class="network-sidebar fixed top-0 right-0 h-full w-96 z-50 flex flex-col shadow-2xl {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}"
    transition:slide={{ axis: 'x', duration: 250 }}
  >
    <!-- Header -->
    <div class="sidebar-header h-14 px-4 flex items-center justify-between shrink-0">
      <div class="flex items-center gap-3">
        <div class="sidebar-icon w-8 h-8 rounded-lg flex items-center justify-center">
          {#if location.isLocal}
            <span class="text-lg">📍</span>
          {:else}
            <span class="text-lg">🌐</span>
          {/if}
        </div>
        <div>
          <h2 class="sidebar-title font-bold text-lg">{location.name}</h2>
          <p class="sidebar-subtitle text-xs opacity-70">
            {location.isLocal ? 'Local Machine' : 'Remote Location'}
          </p>
        </div>
      </div>
      <button
        onclick={onClose}
        class="sidebar-close-btn w-8 h-8 rounded-lg flex items-center justify-center transition"
        aria-label="Close"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-4 space-y-4">
      <!-- Location Info -->
      <div class="sidebar-section p-4 rounded-lg">
        <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Coordinates</h3>
        <div class="grid grid-cols-2 gap-3 text-sm">
          <div>
            <p class="sidebar-label text-xs opacity-60">Latitude</p>
            <p class="sidebar-value font-mono">{location.lat.toFixed(4)}</p>
          </div>
          <div>
            <p class="sidebar-label text-xs opacity-60">Longitude</p>
            <p class="sidebar-value font-mono">{location.lng.toFixed(4)}</p>
          </div>
        </div>
      </div>

      <!-- Connection Statistics -->
      <div class="sidebar-section p-4 rounded-lg">
        <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Connection Stats</h3>
        <div class="grid grid-cols-2 gap-3">
          <div class="sidebar-stat p-3 rounded-lg text-center">
            <p class="sidebar-stat-value text-2xl font-bold">{location.count}</p>
            <p class="sidebar-stat-label text-xs opacity-60">Connections</p>
          </div>
          <div class="sidebar-stat p-3 rounded-lg text-center {anomalyCount() > 0 ? 'anomaly-stat' : ''}">
            <p class="sidebar-stat-value text-2xl font-bold {anomalyCount() > 0 ? 'text-red-400' : ''}">{anomalyCount()}</p>
            <p class="sidebar-stat-label text-xs opacity-60">Anomalies</p>
          </div>
          <div class="sidebar-stat p-3 rounded-lg text-center col-span-2">
            <p class="sidebar-stat-value text-2xl font-bold">{processBreakdown().length}</p>
            <p class="sidebar-stat-label text-xs opacity-60">Unique Processes</p>
          </div>
        </div>
      </div>

      <!-- Process Breakdown -->
      {#if processBreakdown().length > 0}
        <div class="sidebar-section p-4 rounded-lg">
          <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Process Breakdown</h3>
          <div class="space-y-2">
            {#each processBreakdown() as proc, i}
              <div class="flex items-center justify-between p-2 rounded-lg {proc.hasAnomaly ? 'bg-red-500/10 border border-red-500/30' : 'bg-white/5'}">
                <div class="flex items-center gap-2">
                  <span class="text-base">{getProcessIcon(proc.name)}</span>
                  <span class="sidebar-process-name text-sm font-medium truncate max-w-[180px]">{proc.name}</span>
                </div>
                <div class="flex items-center gap-2">
                  {#if proc.hasAnomaly}
                    <span class="text-[10px] px-1.5 py-0.5 bg-red-500/20 text-red-400 rounded font-bold">⚠️</span>
                  {/if}
                  <span class="sidebar-process-count text-sm opacity-70">{proc.count}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Recent Connections -->
      {#if locationConnections().length > 0}
        <div class="sidebar-section p-4 rounded-lg">
          <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">
            {location.isLocal ? 'Outbound Connections' : 'Connection Details'}
          </h3>
          <div class="space-y-2 max-h-60 overflow-y-auto">
            {#each locationConnections().slice(0, 20) as conn}
              <div class="flex items-center gap-2 p-2 rounded text-xs {conn.isAnomaly ? 'bg-red-500/10 border border-red-500/30' : 'bg-white/5'}">
                <span class="text-base">{getProcessIcon(conn.process || '')}</span>
                <div class="flex-1 min-w-0">
                  <p class="font-medium truncate">{conn.process || 'Unknown'}</p>
                  <p class="opacity-50 truncate">{conn.source.name} → {conn.target.name}</p>
                </div>
                {#if conn.isAnomaly}
                  <span class="px-1.5 py-0.5 bg-red-500/20 text-red-400 rounded text-[10px] font-bold shrink-0">ANOMALY</span>
                {/if}
              </div>
            {/each}
            {#if locationConnections().length > 20}
              <p class="text-xs text-center opacity-50 pt-2">
                +{locationConnections().length - 20} more connections
              </p>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Status indicator -->
      <div class="sidebar-section p-4 rounded-lg">
        <div class="flex items-center justify-between">
          <span class="text-xs opacity-60">Status</span>
          {#if anomalyCount() > 0}
            <span class="flex items-center gap-1.5 text-xs font-bold text-red-400">
              <span class="w-2 h-2 rounded-full bg-red-500 animate-pulse"></span>
              Anomalies Detected
            </span>
          {:else}
            <span class="flex items-center gap-1.5 text-xs font-bold text-green-400">
              <span class="w-2 h-2 rounded-full bg-green-500"></span>
              Normal Activity
            </span>
          {/if}
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div class="sidebar-footer h-12 px-4 flex items-center justify-between shrink-0 text-xs opacity-50">
      <span>Network Sentinel</span>
      <span>Press ESC to close</span>
    </div>
  </div>
{/if}

<style>
  /* Theme: Default */
  .theme-default.network-sidebar {
    background: linear-gradient(180deg, #0f172a 0%, #1e293b 100%);
    border-left: 1px solid rgba(148, 163, 184, 0.2);
  }

  .theme-default .sidebar-header {
    background: rgba(30, 41, 59, 0.8);
    border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  }

  .theme-default .sidebar-icon {
    background: rgba(0, 180, 255, 0.2);
    color: #00b4ff;
  }

  .theme-default .sidebar-title {
    color: #f1f5f9;
  }

  .theme-default .sidebar-subtitle {
    color: #94a3b8;
  }

  .theme-default .sidebar-close-btn {
    background: rgba(148, 163, 184, 0.1);
    color: #94a3b8;
  }

  .theme-default .sidebar-close-btn:hover {
    background: rgba(148, 163, 184, 0.2);
    color: #f1f5f9;
  }

  .theme-default .sidebar-section {
    background: rgba(30, 41, 59, 0.5);
    border: 1px solid rgba(148, 163, 184, 0.1);
  }

  .theme-default .sidebar-section-title {
    color: #00b4ff;
  }

  .theme-default .sidebar-label,
  .theme-default .sidebar-stat-label {
    color: #94a3b8;
  }

  .theme-default .sidebar-value,
  .theme-default .sidebar-stat-value {
    color: #f1f5f9;
  }

  .theme-default .sidebar-stat {
    background: rgba(0, 180, 255, 0.1);
  }

  .theme-default .sidebar-process-name {
    color: #f1f5f9;
  }

  .theme-default .sidebar-process-count {
    color: #94a3b8;
  }

  .theme-default .sidebar-footer {
    background: rgba(15, 23, 42, 0.8);
    border-top: 1px solid rgba(148, 163, 184, 0.2);
    color: #64748b;
  }

  .theme-default .anomaly-stat {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
  }

  /* Theme: Skynet */
  .theme-skynet.network-sidebar {
    background: linear-gradient(180deg, #000510 0%, #001020 100%);
    border-left: 1px solid rgba(0, 180, 255, 0.3);
    box-shadow: -10px 0 40px rgba(0, 180, 255, 0.15);
  }

  .theme-skynet .sidebar-header {
    background: rgba(0, 20, 40, 0.9);
    border-bottom: 1px solid rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .sidebar-icon {
    background: rgba(0, 180, 255, 0.2);
    color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .sidebar-title {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .sidebar-subtitle {
    color: rgba(0, 180, 255, 0.6);
  }

  .theme-skynet .sidebar-close-btn {
    background: rgba(0, 180, 255, 0.1);
    color: #00b4ff;
    border: 1px solid rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .sidebar-close-btn:hover {
    background: rgba(0, 180, 255, 0.2);
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .sidebar-section {
    background: rgba(0, 30, 60, 0.5);
    border: 1px solid rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .sidebar-section-title {
    color: #00b4ff;
    text-shadow: 0 0 8px rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .sidebar-label,
  .theme-skynet .sidebar-stat-label {
    color: rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .sidebar-value,
  .theme-skynet .sidebar-stat-value {
    color: #00b4ff;
  }

  .theme-skynet .sidebar-stat {
    background: rgba(0, 180, 255, 0.1);
    border: 1px solid rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .sidebar-process-name {
    color: #00b4ff;
  }

  .theme-skynet .sidebar-process-count {
    color: rgba(0, 180, 255, 0.6);
  }

  .theme-skynet .sidebar-footer {
    background: rgba(0, 10, 20, 0.9);
    border-top: 1px solid rgba(0, 180, 255, 0.2);
    color: rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .anomaly-stat {
    background: rgba(255, 0, 80, 0.1);
    border: 1px solid rgba(255, 0, 80, 0.3);
  }

  /* Theme: Cyberpunk */
  .theme-cyberpunk.network-sidebar {
    background: linear-gradient(180deg, #0a0015 0%, #150025 100%);
    border-left: 1px solid rgba(0, 217, 255, 0.3);
    box-shadow: -10px 0 40px rgba(255, 0, 128, 0.1), -5px 0 20px rgba(0, 217, 255, 0.1);
  }

  .theme-cyberpunk .sidebar-header {
    background: rgba(20, 0, 40, 0.9);
    border-bottom: 1px solid rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .sidebar-icon {
    background: linear-gradient(135deg, rgba(255, 0, 128, 0.2), rgba(0, 217, 255, 0.2));
    color: #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.3);
  }

  .theme-cyberpunk .sidebar-title {
    color: #00d9ff;
    text-shadow: 0 0 10px rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .sidebar-subtitle {
    color: rgba(255, 0, 128, 0.7);
  }

  .theme-cyberpunk .sidebar-close-btn {
    background: rgba(255, 0, 128, 0.1);
    color: #ff0080;
    border: 1px solid rgba(255, 0, 128, 0.2);
  }

  .theme-cyberpunk .sidebar-close-btn:hover {
    background: rgba(255, 0, 128, 0.2);
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .sidebar-section {
    background: rgba(30, 0, 50, 0.5);
    border: 1px solid rgba(0, 217, 255, 0.2);
  }

  .theme-cyberpunk .sidebar-section-title {
    color: #00d9ff;
    text-shadow: 0 0 8px rgba(0, 217, 255, 0.4);
  }

  .theme-cyberpunk .sidebar-label,
  .theme-cyberpunk .sidebar-stat-label {
    color: rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .sidebar-value,
  .theme-cyberpunk .sidebar-stat-value {
    color: #ff0080;
  }

  .theme-cyberpunk .sidebar-stat {
    background: linear-gradient(135deg, rgba(255, 0, 128, 0.1), rgba(0, 217, 255, 0.1));
    border: 1px solid rgba(255, 0, 128, 0.2);
  }

  .theme-cyberpunk .sidebar-process-name {
    color: #ff0080;
  }

  .theme-cyberpunk .sidebar-process-count {
    color: rgba(0, 217, 255, 0.7);
  }

  .theme-cyberpunk .sidebar-footer {
    background: rgba(10, 0, 20, 0.9);
    border-top: 1px solid rgba(255, 0, 128, 0.2);
    color: rgba(0, 217, 255, 0.4);
  }

  .theme-cyberpunk .anomaly-stat {
    background: rgba(255, 0, 80, 0.1);
    border: 1px solid rgba(255, 0, 80, 0.3);
  }
</style>
