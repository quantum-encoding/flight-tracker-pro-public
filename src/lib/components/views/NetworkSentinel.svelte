<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { fade, slide } from 'svelte/transition';
  import { theme } from '$lib/theme';
  import GlobeVisualization from './GlobeVisualization.svelte';
  import { userLocation, initUserLocation } from '$lib/stores/settings';

  // Types for Network Sentinel data
  interface NetworkFlow {
    id: number;
    process_name: string;
    pid: number;
    local_addr: string;
    local_port: number;
    remote_addr: string;
    remote_port: number;
    protocol: string;
    direction: 'inbound' | 'outbound';
    bytes_sent: number;
    bytes_recv: number;
    timestamp: string;
    geo_country?: string;
    geo_city?: string;
    geo_asn?: string;
    is_anomaly: boolean;
    anomaly_reason?: string;
  }

  interface NetworkStats {
    total_flows: number;
    anomalous_flows: number;
    unique_processes: number;
    unique_destinations: number;
    top_talkers: Array<{ process: string; flows: number }>;
  }

  interface GlobeConnection {
    id: string;
    source: { lat: number; lng: number; name: string };
    target: { lat: number; lng: number; name: string };
    color?: string;
    process?: string;
  }

  // State
  let stats = $state<NetworkStats | null>(null);
  let flows = $state<NetworkFlow[]>([]);
  let anomalies = $state<NetworkFlow[]>([]);
  let globeConnections = $state<GlobeConnection[]>([]);
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let selectedTimeRange = $state<'live' | '1h' | '24h' | '7d'>('live');
  let selectedProcess = $state<string>('all');
  let showAnomaliesOnly = $state(false);
  let searchQuery = $state('');
  let autoRefresh = $state(true);
  let refreshInterval: number | null = null;

  // Globe state
  let globeContainer: HTMLDivElement;
  let globeInitialized = $state(false);

  // Sidebar state
  let showAnomalySidebar = $state(true);

  // City to coordinates mapping for globe visualization
  const cityCoordinates: Record<string, { lat: number; lng: number }> = {
    // North America
    'San Francisco': { lat: 37.7749, lng: -122.4194 },
    'New York': { lat: 40.7128, lng: -74.0060 },
    'Los Angeles': { lat: 34.0522, lng: -118.2437 },
    'Seattle': { lat: 47.6062, lng: -122.3321 },
    'Chicago': { lat: 41.8781, lng: -87.6298 },
    'Dallas': { lat: 32.7767, lng: -96.7970 },
    'Miami': { lat: 25.7617, lng: -80.1918 },
    'Atlanta': { lat: 33.7490, lng: -84.3880 },
    'Denver': { lat: 39.7392, lng: -104.9903 },
    'Phoenix': { lat: 33.4484, lng: -112.0740 },
    'Toronto': { lat: 43.6532, lng: -79.3832 },
    'Vancouver': { lat: 49.2827, lng: -123.1207 },
    'Montreal': { lat: 45.5017, lng: -73.5673 },
    'Mexico City': { lat: 19.4326, lng: -99.1332 },
    // Europe
    'London': { lat: 51.5074, lng: -0.1278 },
    'Paris': { lat: 48.8566, lng: 2.3522 },
    'Berlin': { lat: 52.5200, lng: 13.4050 },
    'Frankfurt': { lat: 50.1109, lng: 8.6821 },
    'Amsterdam': { lat: 52.3676, lng: 4.9041 },
    'Dublin': { lat: 53.3498, lng: -6.2603 },
    'Madrid': { lat: 40.4168, lng: -3.7038 },
    'Barcelona': { lat: 41.3851, lng: 2.1734 },
    'Valencia': { lat: 39.4699, lng: -0.3763 },
    'Seville': { lat: 37.3891, lng: -5.9845 },
    'Bilbao': { lat: 43.2630, lng: -2.9350 },
    'Malaga': { lat: 36.7213, lng: -4.4214 },
    'Lisbon': { lat: 38.7223, lng: -9.1393 },
    'Rome': { lat: 41.9028, lng: 12.4964 },
    'Milan': { lat: 45.4642, lng: 9.1900 },
    'Vienna': { lat: 48.2082, lng: 16.3738 },
    'Zurich': { lat: 47.3769, lng: 8.5417 },
    'Brussels': { lat: 50.8503, lng: 4.3517 },
    'Stockholm': { lat: 59.3293, lng: 18.0686 },
    'Oslo': { lat: 59.9139, lng: 10.7522 },
    'Copenhagen': { lat: 55.6761, lng: 12.5683 },
    'Helsinki': { lat: 60.1699, lng: 24.9384 },
    'Warsaw': { lat: 52.2297, lng: 21.0122 },
    'Prague': { lat: 50.0755, lng: 14.4378 },
    'Budapest': { lat: 47.4979, lng: 19.0402 },
    'Athens': { lat: 37.9838, lng: 23.7275 },
    'Moscow': { lat: 55.7558, lng: 37.6173 },
    // Asia
    'Tokyo': { lat: 35.6762, lng: 139.6503 },
    'Singapore': { lat: 1.3521, lng: 103.8198 },
    'Hong Kong': { lat: 22.3193, lng: 114.1694 },
    'Mumbai': { lat: 19.0760, lng: 72.8777 },
    'Bangalore': { lat: 12.9716, lng: 77.5946 },
    'Delhi': { lat: 28.6139, lng: 77.2090 },
    'Shanghai': { lat: 31.2304, lng: 121.4737 },
    'Beijing': { lat: 39.9042, lng: 116.4074 },
    'Seoul': { lat: 37.5665, lng: 126.9780 },
    'Bangkok': { lat: 13.7563, lng: 100.5018 },
    'Dubai': { lat: 25.2048, lng: 55.2708 },
    // Oceania
    'Sydney': { lat: -33.8688, lng: 151.2093 },
    'Melbourne': { lat: -37.8136, lng: 144.9631 },
    // South America
    'São Paulo': { lat: -23.5505, lng: -46.6333 },
    'Buenos Aires': { lat: -34.6037, lng: -58.3816 },
  };

  // Country to coordinates mapping (fallback)
  const countryCoordinates: Record<string, { lat: number; lng: number }> = {
    // Americas
    'US': { lat: 37.0902, lng: -95.7129 },
    'CA': { lat: 56.1304, lng: -106.3468 },
    'MX': { lat: 23.6345, lng: -102.5528 },
    'BR': { lat: -14.2350, lng: -51.9253 },
    'AR': { lat: -38.4161, lng: -63.6167 },
    // Europe
    'GB': { lat: 55.3781, lng: -3.4360 },
    'DE': { lat: 51.1657, lng: 10.4515 },
    'FR': { lat: 46.2276, lng: 2.2137 },
    'ES': { lat: 40.4637, lng: -3.7492 },
    'IT': { lat: 41.8719, lng: 12.5674 },
    'NL': { lat: 52.1326, lng: 5.2913 },
    'BE': { lat: 50.5039, lng: 4.4699 },
    'PT': { lat: 39.3999, lng: -8.2245 },
    'IE': { lat: 53.1424, lng: -7.6921 },
    'AT': { lat: 47.5162, lng: 14.5501 },
    'CH': { lat: 46.8182, lng: 8.2275 },
    'SE': { lat: 60.1282, lng: 18.6435 },
    'NO': { lat: 60.4720, lng: 8.4689 },
    'DK': { lat: 56.2639, lng: 9.5018 },
    'FI': { lat: 61.9241, lng: 25.7482 },
    'PL': { lat: 51.9194, lng: 19.1451 },
    'RU': { lat: 61.5240, lng: 105.3188 },
    // Asia
    'JP': { lat: 36.2048, lng: 138.2529 },
    'CN': { lat: 35.8617, lng: 104.1954 },
    'IN': { lat: 20.5937, lng: 78.9629 },
    'KR': { lat: 35.9078, lng: 127.7669 },
    'SG': { lat: 1.3521, lng: 103.8198 },
    'HK': { lat: 22.3193, lng: 114.1694 },
    'TH': { lat: 15.8700, lng: 100.9925 },
    'AE': { lat: 23.4241, lng: 53.8478 },
    // Oceania
    'AU': { lat: -25.2744, lng: 133.7751 },
    'NZ': { lat: -40.9006, lng: 174.8860 },
  };

  // Local machine coordinates - from user settings store
  let localCoordinates = $state({ lat: 40.4168, lng: -3.7038 });

  // Subscribe to userLocation store
  const unsubscribeLocation = userLocation.subscribe(loc => {
    localCoordinates = { lat: loc.lat, lng: loc.lng };
  });

  // Time-travel state
  let timeTravelMode = $state(false);
  let timeTravelDate = $state('');
  let timeTravelStartTime = $state('00:00');
  let timeTravelEndTime = $state('23:59');
  let timeTravelResults = $state<NetworkFlow[]>([]);

  // Computed: unique processes
  const uniqueProcesses = $derived(() => {
    const processes = new Set(flows.map(f => f.process_name));
    return Array.from(processes).sort();
  });

  // Computed: filtered flows
  const filteredFlows = $derived(() => {
    return flows.filter(f => {
      if (showAnomaliesOnly && !f.is_anomaly) return false;
      if (selectedProcess !== 'all' && f.process_name !== selectedProcess) return false;
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        return (
          f.process_name.toLowerCase().includes(query) ||
          f.remote_addr.includes(query) ||
          f.geo_country?.toLowerCase().includes(query) ||
          f.geo_city?.toLowerCase().includes(query)
        );
      }
      return true;
    });
  });

  // Computed: convert flows to globe connections
  const globeNetworkConnections = $derived(() => {
    const connections: GlobeConnection[] = [];
    const seenLocations = new Set<string>();

    for (const flow of flows) {
      // Get target coordinates from city or country
      let targetCoords = flow.geo_city ? cityCoordinates[flow.geo_city] : null;
      if (!targetCoords && flow.geo_country) {
        targetCoords = countryCoordinates[flow.geo_country];
      }

      // Skip if no coordinates found
      if (!targetCoords) continue;

      // Create unique key for deduplication
      const locationKey = `${flow.geo_city || flow.geo_country}`;
      if (seenLocations.has(locationKey)) continue;
      seenLocations.add(locationKey);

      // Determine color based on process or anomaly status
      let color = '#00b4ff'; // Default cyan
      if (flow.is_anomaly) {
        color = '#ff0040'; // Red for anomalies
      } else if (flow.process_name === 'claude') {
        color = '#b000ff'; // Purple for Claude
      } else if (flow.process_name.includes('firefox') || flow.process_name.includes('brave') || flow.process_name.includes('Chrome')) {
        color = '#00ff88'; // Green for browsers
      }

      connections.push({
        id: `flow-${flow.id}`,
        source: {
          lat: localCoordinates.lat,
          lng: localCoordinates.lng,
          name: 'Local'
        },
        target: {
          lat: targetCoords.lat,
          lng: targetCoords.lng,
          name: flow.geo_city || flow.geo_country || 'Unknown'
        },
        color,
        process: flow.process_name
      });
    }

    return connections;
  });

  // Fetch stats from D-Bus via Tauri
  async function fetchStats() {
    try {
      const result = await invoke<NetworkStats>('get_network_stats');
      stats = result;
    } catch (e: any) {
      console.error('Failed to fetch network stats:', e);
      // Fallback to mock data for development
      stats = {
        total_flows: 5372,
        anomalous_flows: 1968,
        unique_processes: 12,
        unique_destinations: 87,
        top_talkers: [
          { process: 'claude', flows: 4414 },
          { process: 'firefox', flows: 523 },
          { process: 'brave', flows: 234 },
          { process: 'cargo', flows: 156 },
          { process: 'npm', flows: 45 }
        ]
      };
    }
  }

  // Fetch recent flows
  async function fetchFlows() {
    try {
      const result = await invoke<NetworkFlow[]>('get_network_flows', {
        limit: 100,
        timeRange: selectedTimeRange
      });
      flows = result;
    } catch (e: any) {
      console.error('Failed to fetch network flows:', e);
      // Mock data for development
      flows = generateMockFlows(50);
    }
  }

  // Fetch anomalies
  async function fetchAnomalies() {
    try {
      const result = await invoke<NetworkFlow[]>('get_network_anomalies', { limit: 20 });
      anomalies = result;
    } catch (e: any) {
      console.error('Failed to fetch anomalies:', e);
      anomalies = flows.filter(f => f.is_anomaly).slice(0, 20);
    }
  }

  // Time-travel query
  async function executeTimeTravel() {
    if (!timeTravelDate) return;

    try {
      const result = await invoke<NetworkFlow[]>('query_network_history', {
        date: timeTravelDate,
        startTime: timeTravelStartTime,
        endTime: timeTravelEndTime,
        processFilter: selectedProcess !== 'all' ? selectedProcess : null
      });
      timeTravelResults = result;
    } catch (e: any) {
      console.error('Time travel query failed:', e);
      // Mock results
      timeTravelResults = generateMockFlows(25);
    }
  }

  // Generate mock flows for development
  function generateMockFlows(count: number): NetworkFlow[] {
    const processes = ['claude', 'firefox', 'brave', 'cargo', 'npm', 'node', 'Chrome_ChildIOT'];
    const countries = ['US', 'DE', 'JP', 'GB', 'NL', 'SG'];
    const cities = ['San Francisco', 'Frankfurt', 'Tokyo', 'London', 'Amsterdam', 'Singapore'];

    return Array.from({ length: count }, (_, i) => ({
      id: i + 1,
      process_name: processes[Math.floor(Math.random() * processes.length)],
      pid: 1000 + Math.floor(Math.random() * 50000),
      local_addr: '192.168.1.100',
      local_port: 40000 + Math.floor(Math.random() * 20000),
      remote_addr: `${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}`,
      remote_port: [80, 443, 53, 8080, 3000][Math.floor(Math.random() * 5)],
      protocol: Math.random() > 0.2 ? 'TCP' : 'UDP',
      direction: Math.random() > 0.3 ? 'outbound' : 'inbound',
      bytes_sent: Math.floor(Math.random() * 100000),
      bytes_recv: Math.floor(Math.random() * 500000),
      timestamp: new Date(Date.now() - Math.floor(Math.random() * 3600000)).toISOString(),
      geo_country: countries[Math.floor(Math.random() * countries.length)],
      geo_city: cities[Math.floor(Math.random() * cities.length)],
      geo_asn: `AS${Math.floor(Math.random() * 60000)}`,
      is_anomaly: Math.random() > 0.85,
      anomaly_reason: Math.random() > 0.85 ? 'Unusual port' : undefined
    }));
  }

  // Format bytes
  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  // Format timestamp
  function formatTime(ts: string): string {
    return new Date(ts).toLocaleTimeString();
  }

  // Get protocol color
  function getProtocolColor(protocol: string): string {
    return protocol === 'TCP' ? 'text-cyan-400' : 'text-amber-400';
  }

  // Get direction icon
  function getDirectionIcon(direction: string): string {
    return direction === 'outbound' ? '↗' : '↙';
  }

  // Refresh data
  async function refreshData() {
    isLoading = true;
    await Promise.all([fetchStats(), fetchFlows(), fetchAnomalies()]);
    isLoading = false;
  }

  // Initialize
  onMount(async () => {
    // Initialize user location from settings
    await initUserLocation();

    await refreshData();

    // Auto-refresh every 5 seconds if enabled
    refreshInterval = setInterval(() => {
      if (autoRefresh && !timeTravelMode) {
        refreshData();
      }
    }, 5000);
  });

  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
    // Unsubscribe from location store
    unsubscribeLocation();
  });
</script>

<div class="sentinel-container flex flex-col h-full font-sans {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}">
  <!-- Header -->
  <div class="sentinel-header h-14 px-6 flex items-center justify-between border-b shrink-0">
    <div class="flex items-center gap-4">
      <div class="flex items-center gap-2">
        <div class="sentinel-logo w-8 h-8 rounded-lg flex items-center justify-center">
          <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
          </svg>
        </div>
        <h1 class="sentinel-title text-xl font-bold">Network Sentinel</h1>
      </div>
      <span class="sentinel-badge px-2 py-0.5 text-[10px] font-bold rounded border">
        eBPF ACTIVE
      </span>
    </div>

    <div class="flex items-center gap-4">
      <!-- Time Range Selector -->
      <div class="flex items-center gap-1 bg-slate-800 rounded-lg p-1">
        {#each ['live', '1h', '24h', '7d'] as range}
          <button
            onclick={() => { selectedTimeRange = range as any; refreshData(); }}
            class="px-3 py-1 text-xs font-medium rounded transition {selectedTimeRange === range ? 'bg-red-600 text-white' : 'text-slate-400 hover:text-white'}"
          >
            {range === 'live' ? '● Live' : range}
          </button>
        {/each}
      </div>

      <!-- Auto-refresh toggle -->
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={autoRefresh} class="sr-only peer" />
        <div class="w-8 h-4 bg-slate-700 rounded-full peer-checked:bg-green-600 transition relative">
          <div class="absolute top-0.5 left-0.5 w-3 h-3 bg-white rounded-full transition peer-checked:translate-x-4"></div>
        </div>
        <span class="text-xs text-slate-400">Auto</span>
      </label>

      <!-- Time Travel Toggle -->
      <button
        onclick={() => timeTravelMode = !timeTravelMode}
        class="px-3 py-1.5 text-xs font-bold rounded transition flex items-center gap-2 {timeTravelMode ? 'bg-purple-600 text-white' : 'bg-slate-800 text-slate-400 hover:text-white'}"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        Time Travel
      </button>
    </div>
  </div>

  <!-- Time Travel Panel -->
  {#if timeTravelMode}
    <div class="px-6 py-4 bg-purple-900/20 border-b border-purple-500/30" transition:slide>
      <div class="flex items-end gap-4">
        <div>
          <label class="block text-xs font-medium text-purple-300 mb-1">Date</label>
          <input
            type="date"
            bind:value={timeTravelDate}
            class="bg-slate-800 border border-purple-500/50 rounded px-3 py-1.5 text-sm text-white"
          />
        </div>
        <div>
          <label class="block text-xs font-medium text-purple-300 mb-1">Start Time</label>
          <input
            type="time"
            bind:value={timeTravelStartTime}
            class="bg-slate-800 border border-purple-500/50 rounded px-3 py-1.5 text-sm text-white"
          />
        </div>
        <div>
          <label class="block text-xs font-medium text-purple-300 mb-1">End Time</label>
          <input
            type="time"
            bind:value={timeTravelEndTime}
            class="bg-slate-800 border border-purple-500/50 rounded px-3 py-1.5 text-sm text-white"
          />
        </div>
        <div>
          <label class="block text-xs font-medium text-purple-300 mb-1">Process</label>
          <select
            bind:value={selectedProcess}
            class="bg-slate-800 border border-purple-500/50 rounded px-3 py-1.5 text-sm text-white"
          >
            <option value="all">All Processes</option>
            {#each uniqueProcesses() as process}
              <option value={process}>{process}</option>
            {/each}
          </select>
        </div>
        <button
          onclick={executeTimeTravel}
          disabled={!timeTravelDate}
          class="px-4 py-1.5 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white text-sm font-bold rounded transition"
        >
          Query History
        </button>
      </div>

      {#if timeTravelResults.length > 0}
        <div class="mt-4 p-3 bg-slate-900/50 rounded-lg">
          <p class="text-xs text-purple-300 mb-2">Found {timeTravelResults.length} connections during this period:</p>
          <div class="max-h-40 overflow-y-auto space-y-1">
            {#each timeTravelResults.slice(0, 10) as flow}
              <div class="flex items-center gap-3 text-xs p-2 bg-slate-800/50 rounded">
                <span class="font-mono text-purple-400">{formatTime(flow.timestamp)}</span>
                <span class="font-medium text-white">{flow.process_name}</span>
                <span class="text-slate-500">→</span>
                <span class="font-mono text-cyan-400">{flow.remote_addr}:{flow.remote_port}</span>
                <span class={getProtocolColor(flow.protocol)}>{flow.protocol}</span>
                {#if flow.is_anomaly}
                  <span class="px-1.5 py-0.5 bg-red-500/20 text-red-400 rounded text-[10px]">ANOMALY</span>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <div class="flex-1 flex overflow-hidden relative">
    <!-- Left Panel: Stats & Globe -->
    <div class="sentinel-panel w-96 border-r flex flex-col">
      <!-- Stats Cards -->
      <div class="p-4 space-y-3 border-b border-inherit">
        <div class="grid grid-cols-2 gap-3">
          <div class="sentinel-card rounded-lg p-3">
            <p class="text-[10px] font-bold opacity-60 uppercase">Total Flows</p>
            <p class="text-2xl font-bold">{stats?.total_flows.toLocaleString() ?? '—'}</p>
          </div>
          <div class="sentinel-anomaly rounded-lg p-3 border">
            <p class="text-[10px] font-bold sentinel-accent-secondary uppercase">Anomalies</p>
            <p class="text-2xl font-bold sentinel-accent-secondary">{stats?.anomalous_flows.toLocaleString() ?? '—'}</p>
          </div>
          <div class="sentinel-card rounded-lg p-3">
            <p class="text-[10px] font-bold opacity-60 uppercase">Processes</p>
            <p class="text-2xl font-bold sentinel-accent">{stats?.unique_processes ?? '—'}</p>
          </div>
          <div class="sentinel-card rounded-lg p-3">
            <p class="text-[10px] font-bold opacity-60 uppercase">Destinations</p>
            <p class="text-2xl font-bold text-amber-400">{stats?.unique_destinations ?? '—'}</p>
          </div>
        </div>
      </div>

      <!-- Top Talkers -->
      <div class="p-4 border-b border-slate-800">
        <h3 class="text-xs font-bold text-slate-400 uppercase mb-3">Top Talkers</h3>
        <div class="space-y-2">
          {#if stats?.top_talkers}
            {#each stats.top_talkers.slice(0, 5) as talker, i}
              <div class="flex items-center gap-3">
                <span class="w-5 h-5 rounded bg-slate-700 flex items-center justify-center text-[10px] font-bold text-slate-400">
                  {i + 1}
                </span>
                <span class="flex-1 text-sm font-medium text-white truncate">{talker.process}</span>
                <span class="text-xs text-slate-500">{talker.flows.toLocaleString()}</span>
                <div class="w-20 h-1.5 bg-slate-800 rounded-full overflow-hidden">
                  <div
                    class="h-full bg-gradient-to-r from-cyan-500 to-blue-500 rounded-full"
                    style="width: {(talker.flows / (stats.top_talkers[0]?.flows || 1)) * 100}%"
                  ></div>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>

      <!-- 3D Globe Visualization -->
      <div class="flex-1 p-2 min-h-0">
        <div class="h-full rounded-lg border border-slate-700 overflow-hidden relative">
          <div class="absolute top-2 left-2 z-10 px-2 py-1 bg-black/60 rounded text-[10px] text-cyan-400 font-bold uppercase tracking-wide">
            Network Globe
          </div>
          <GlobeVisualization
            embedded={true}
            mode="network"
            networkConnections={globeNetworkConnections()}
          />
        </div>
      </div>
    </div>

    <!-- Main Content: Flow Matrix -->
    <div class="flex-1 flex flex-col overflow-hidden">
      <!-- Toolbar -->
      <div class="h-12 px-4 flex items-center gap-4 bg-slate-900/50 border-b border-slate-800 shrink-0">
        <!-- Search -->
        <div class="flex-1 max-w-md">
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search flows by process, IP, country..."
            class="w-full bg-slate-800 border border-slate-700 rounded px-3 py-1.5 text-sm text-slate-300 placeholder-slate-500 outline-none focus:border-cyan-500"
          />
        </div>

        <!-- Process Filter -->
        <select
          bind:value={selectedProcess}
          class="bg-slate-800 border border-slate-700 rounded px-3 py-1.5 text-sm text-slate-300"
        >
          <option value="all">All Processes</option>
          {#each uniqueProcesses() as process}
            <option value={process}>{process}</option>
          {/each}
        </select>

        <!-- Anomalies Toggle -->
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={showAnomaliesOnly} class="sr-only peer" />
          <div class="w-8 h-4 bg-slate-700 rounded-full peer-checked:bg-red-600 transition relative">
            <div class="absolute top-0.5 left-0.5 w-3 h-3 bg-white rounded-full transition peer-checked:translate-x-4"></div>
          </div>
          <span class="text-xs text-slate-400">Anomalies Only</span>
        </label>

        <!-- Refresh Button -->
        <button
          onclick={refreshData}
          disabled={isLoading}
          class="p-2 text-slate-400 hover:text-white transition disabled:opacity-50"
        >
          <svg class="w-4 h-4 {isLoading ? 'animate-spin' : ''}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
        </button>
      </div>

      <!-- Flow Table -->
      <div class="flex-1 overflow-auto">
        <table class="w-full">
          <thead class="bg-slate-900 sticky top-0">
            <tr class="text-[10px] font-bold text-slate-500 uppercase">
              <th class="px-4 py-3 text-left">Time</th>
              <th class="px-4 py-3 text-left">Process</th>
              <th class="px-4 py-3 text-left">Direction</th>
              <th class="px-4 py-3 text-left">Remote</th>
              <th class="px-4 py-3 text-left">Protocol</th>
              <th class="px-4 py-3 text-left">Location</th>
              <th class="px-4 py-3 text-right">Traffic</th>
              <th class="px-4 py-3 text-center">Status</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-800">
            {#each filteredFlows() as flow, i (flow.id)}
              <tr
                class="hover:bg-slate-800/50 transition {flow.is_anomaly ? 'bg-red-900/10' : ''}"
                in:fade={{ delay: i * 20, duration: 200 }}
              >
                <td class="px-4 py-3 text-xs font-mono text-slate-400">
                  {formatTime(flow.timestamp)}
                </td>
                <td class="px-4 py-3">
                  <div class="flex items-center gap-2">
                    <span class="text-sm font-medium text-white">{flow.process_name}</span>
                    <span class="text-[10px] text-slate-600">({flow.pid})</span>
                  </div>
                </td>
                <td class="px-4 py-3">
                  <span class="text-lg {flow.direction === 'outbound' ? 'text-cyan-400' : 'text-amber-400'}">
                    {getDirectionIcon(flow.direction)}
                  </span>
                </td>
                <td class="px-4 py-3">
                  <span class="font-mono text-xs text-slate-300">{flow.remote_addr}</span>
                  <span class="text-slate-600">:</span>
                  <span class="font-mono text-xs text-cyan-400">{flow.remote_port}</span>
                </td>
                <td class="px-4 py-3">
                  <span class="px-2 py-0.5 text-[10px] font-bold rounded {flow.protocol === 'TCP' ? 'bg-cyan-500/20 text-cyan-400' : 'bg-amber-500/20 text-amber-400'}">
                    {flow.protocol}
                  </span>
                </td>
                <td class="px-4 py-3 text-xs text-slate-400">
                  {#if flow.geo_country}
                    <span class="mr-1">{flow.geo_country}</span>
                    {#if flow.geo_city}
                      <span class="text-slate-600">• {flow.geo_city}</span>
                    {/if}
                  {:else}
                    <span class="text-slate-600">—</span>
                  {/if}
                </td>
                <td class="px-4 py-3 text-right">
                  <div class="text-xs">
                    <span class="text-green-400">↑{formatBytes(flow.bytes_sent)}</span>
                    <span class="text-slate-600 mx-1">/</span>
                    <span class="text-blue-400">↓{formatBytes(flow.bytes_recv)}</span>
                  </div>
                </td>
                <td class="px-4 py-3 text-center">
                  {#if flow.is_anomaly}
                    <span class="px-2 py-0.5 text-[10px] font-bold bg-red-500/20 text-red-400 rounded" title={flow.anomaly_reason}>
                      ANOMALY
                    </span>
                  {:else}
                    <span class="text-green-400">●</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>

        {#if filteredFlows().length === 0}
          <div class="flex flex-col items-center justify-center h-64 text-slate-500">
            <svg class="w-12 h-12 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9.172 16.172a4 4 0 015.656 0M9 10h.01M15 10h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <p class="text-sm">No flows matching your criteria</p>
          </div>
        {/if}
      </div>
    </div>

    <!-- Anomaly Sidebar Toggle Button (when collapsed) -->
    {#if !showAnomalySidebar}
      <button
        onclick={() => showAnomalySidebar = true}
        class="absolute right-0 top-1/2 -translate-y-1/2 z-10 flex items-center gap-1 px-2 py-3 bg-red-600/90 hover:bg-red-500 text-white rounded-l-lg shadow-lg transition-all group"
        title="Show Anomaly Feed"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <span class="px-1.5 py-0.5 text-[10px] font-bold bg-white/20 rounded">{anomalies.length}</span>
        <svg class="w-3 h-3 opacity-60 group-hover:opacity-100 transition" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </button>
    {/if}

    <!-- Right Panel: Anomalies (Collapsible) -->
    {#if showAnomalySidebar}
      <div class="w-80 bg-slate-900 border-l border-slate-800 flex flex-col transition-all" transition:slide={{ axis: 'x', duration: 200 }}>
        <div class="h-12 px-4 flex items-center justify-between border-b border-slate-800 shrink-0">
          <h3 class="text-sm font-bold text-red-400 flex items-center gap-2">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
            Anomaly Feed
          </h3>
          <div class="flex items-center gap-2">
            <span class="px-2 py-0.5 text-[10px] font-bold bg-red-500/20 text-red-400 rounded">
              {anomalies.length}
            </span>
            <button
              onclick={() => showAnomalySidebar = false}
              class="p-1 text-slate-500 hover:text-white transition rounded hover:bg-slate-800"
              title="Collapse Anomaly Feed"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
              </svg>
            </button>
          </div>
        </div>

        <div class="flex-1 overflow-y-auto p-3 space-y-2">
          {#each anomalies as anomaly (anomaly.id)}
            <div class="bg-red-900/10 border border-red-500/20 rounded-lg p-3 hover:bg-red-900/20 transition cursor-pointer">
              <div class="flex items-start justify-between gap-2 mb-2">
                <span class="text-xs font-bold text-white">{anomaly.process_name}</span>
                <span class="text-[10px] text-slate-500">{formatTime(anomaly.timestamp)}</span>
              </div>
              <div class="font-mono text-[11px] text-slate-400 mb-2">
                {anomaly.remote_addr}:{anomaly.remote_port}
              </div>
              {#if anomaly.anomaly_reason}
                <div class="text-[10px] text-red-400 flex items-center gap-1">
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  </svg>
                  {anomaly.anomaly_reason}
                </div>
              {/if}
              {#if anomaly.geo_country}
                <div class="text-[10px] text-slate-500 mt-1">
                  {anomaly.geo_country} {anomaly.geo_city ? `• ${anomaly.geo_city}` : ''}
                </div>
              {/if}
            </div>
          {/each}

          {#if anomalies.length === 0}
            <div class="flex flex-col items-center justify-center h-32 text-slate-600">
              <svg class="w-8 h-8 mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <p class="text-xs">No anomalies detected</p>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  /* ============================================
     DEFAULT THEME (Dark Slate)
     ============================================ */
  .sentinel-container.theme-default {
    background-color: #020617;
    color: #e2e8f0;
  }

  .theme-default .sentinel-header {
    background: linear-gradient(to right, #0f172a, #1e293b, #0f172a);
    border-color: #334155;
  }

  .theme-default .sentinel-logo {
    background: linear-gradient(135deg, #ef4444, #ea580c);
  }

  .theme-default .sentinel-title {
    color: #ffffff;
  }

  .theme-default .sentinel-badge {
    background-color: rgba(34, 197, 94, 0.2);
    color: #4ade80;
    border-color: rgba(34, 197, 94, 0.3);
  }

  .theme-default .sentinel-panel {
    background-color: #0f172a;
    border-color: #1e293b;
  }

  .theme-default .sentinel-card {
    background-color: rgba(30, 41, 59, 0.5);
    border-color: #334155;
  }

  .theme-default .sentinel-input {
    background-color: #1e293b;
    border-color: #334155;
    color: #cbd5e1;
  }

  .theme-default .sentinel-btn-active {
    background-color: #dc2626;
    color: #ffffff;
  }

  .theme-default .sentinel-btn {
    color: #94a3b8;
  }

  .theme-default .sentinel-btn:hover {
    color: #ffffff;
  }

  /* ============================================
     SKYNET THEME (Cyan/Black)
     ============================================ */
  .sentinel-container.theme-skynet {
    background-color: #000000;
    color: #d4e6ff;
  }

  .theme-skynet .sentinel-header {
    background: linear-gradient(to right, #000510, #0a0a15, #000510);
    border-color: rgba(0, 180, 255, 0.3);
    box-shadow: 0 2px 20px rgba(0, 180, 255, 0.1);
  }

  .theme-skynet .sentinel-logo {
    background: linear-gradient(135deg, #0080ff, #00b4ff);
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .sentinel-title {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .sentinel-badge {
    background-color: rgba(0, 180, 255, 0.15);
    color: #00b4ff;
    border-color: rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 10px rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .sentinel-panel {
    background-color: #0a0a15;
    border-color: rgba(0, 180, 255, 0.3);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.1);
  }

  .theme-skynet .sentinel-card {
    background: linear-gradient(135deg, #0a0a15, #000510);
    border: 1px solid rgba(0, 180, 255, 0.2);
    box-shadow: 0 0 10px rgba(0, 180, 255, 0.1);
  }

  .theme-skynet .sentinel-card:hover {
    border-color: rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .sentinel-input {
    background-color: #050510;
    border: 2px solid rgba(0, 180, 255, 0.3);
    color: #d4e6ff;
  }

  .theme-skynet .sentinel-input:focus {
    border-color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .sentinel-btn-active {
    background: linear-gradient(135deg, #0080ff, #00b4ff);
    color: #ffffff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .sentinel-btn {
    color: #8fb3d9;
  }

  .theme-skynet .sentinel-btn:hover {
    color: #00b4ff;
    text-shadow: 0 0 5px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .sentinel-accent {
    color: #00b4ff;
    text-shadow: 0 0 5px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .sentinel-anomaly {
    background-color: rgba(0, 100, 200, 0.1);
    border-color: rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .sentinel-table-header {
    background: linear-gradient(135deg, #0a0a15, #050510);
    color: #00b4ff;
    border-color: rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .sentinel-table-row:hover {
    background-color: rgba(0, 180, 255, 0.05);
  }

  /* ============================================
     CYBERPUNK THEME (Cyan/Pink Neon)
     ============================================ */
  .sentinel-container.theme-cyberpunk {
    background-color: #0a0a0a;
    color: #ffffff;
  }

  .theme-cyberpunk .sentinel-header {
    background: linear-gradient(to right, #0f0f0f, #1a1a1a, #0f0f0f);
    border-color: rgba(0, 217, 255, 0.3);
    box-shadow: 0 2px 20px rgba(0, 217, 255, 0.1);
  }

  .theme-cyberpunk .sentinel-logo {
    background: linear-gradient(135deg, #ff0080, #00d9ff);
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.5);
  }

  .theme-cyberpunk .sentinel-title {
    color: #00d9ff;
    text-shadow: 0 0 10px rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .sentinel-badge {
    background-color: rgba(0, 255, 136, 0.15);
    color: #00ff88;
    border-color: rgba(0, 255, 136, 0.4);
    box-shadow: 0 0 10px rgba(0, 255, 136, 0.2);
  }

  .theme-cyberpunk .sentinel-panel {
    background-color: #1a1a1a;
    border-color: rgba(0, 217, 255, 0.3);
    box-shadow: 0 0 20px rgba(0, 217, 255, 0.1);
  }

  .theme-cyberpunk .sentinel-card {
    background: linear-gradient(135deg, #1a1a1a, #0f0f0f);
    border: 1px solid rgba(0, 217, 255, 0.2);
    box-shadow: 0 0 10px rgba(0, 217, 255, 0.1);
  }

  .theme-cyberpunk .sentinel-card:hover {
    border-color: rgba(0, 217, 255, 0.4);
    box-shadow: 0 0 20px rgba(0, 217, 255, 0.2);
  }

  .theme-cyberpunk .sentinel-input {
    background-color: #0f0f0f;
    border: 2px solid rgba(0, 217, 255, 0.3);
    color: #ffffff;
  }

  .theme-cyberpunk .sentinel-input:focus {
    border-color: #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.3);
  }

  .theme-cyberpunk .sentinel-btn-active {
    background: linear-gradient(135deg, #ff0080, #b000ff);
    color: #ffffff;
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.4);
  }

  .theme-cyberpunk .sentinel-btn {
    color: #a0a0a0;
  }

  .theme-cyberpunk .sentinel-btn:hover {
    color: #00d9ff;
    text-shadow: 0 0 5px rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .sentinel-accent {
    color: #00d9ff;
    text-shadow: 0 0 5px rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .sentinel-accent-secondary {
    color: #ff0080;
    text-shadow: 0 0 5px rgba(255, 0, 128, 0.5);
  }

  .theme-cyberpunk .sentinel-anomaly {
    background-color: rgba(255, 0, 128, 0.1);
    border-color: rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .sentinel-table-header {
    background: linear-gradient(135deg, #1a1a1a, #0f0f0f);
    color: #00d9ff;
    border-color: rgba(0, 217, 255, 0.3);
  }

  .theme-cyberpunk .sentinel-table-row:hover {
    background-color: rgba(0, 217, 255, 0.05);
  }

  /* ============================================
     SHARED STYLES & SCROLLBARS
     ============================================ */
  .theme-default ::-webkit-scrollbar { width: 6px; height: 6px; }
  .theme-default ::-webkit-scrollbar-track { background: #0f172a; }
  .theme-default ::-webkit-scrollbar-thumb { background: #334155; border-radius: 3px; }
  .theme-default ::-webkit-scrollbar-thumb:hover { background: #475569; }

  .theme-skynet ::-webkit-scrollbar { width: 6px; height: 6px; }
  .theme-skynet ::-webkit-scrollbar-track { background: #000000; border: 1px solid rgba(0, 180, 255, 0.2); }
  .theme-skynet ::-webkit-scrollbar-thumb { background: linear-gradient(180deg, #0080ff, #00b4ff); border-radius: 3px; box-shadow: 0 0 5px rgba(0, 180, 255, 0.5); }

  .theme-cyberpunk ::-webkit-scrollbar { width: 6px; height: 6px; }
  .theme-cyberpunk ::-webkit-scrollbar-track { background: #0a0a0a; border: 1px solid rgba(0, 217, 255, 0.2); }
  .theme-cyberpunk ::-webkit-scrollbar-thumb { background: linear-gradient(180deg, #00d9ff, #b000ff); border-radius: 3px; box-shadow: 0 0 5px rgba(0, 217, 255, 0.5); }
</style>
