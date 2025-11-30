<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { developerMode } from '$lib/stores/settings';
  import { theme } from '$lib/theme';
  import { fade, fly } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';

  // --- State ---
  let terminalLines: string[] = $state([]);
  let terminalContainer = $state<HTMLDivElement | null>(null);

  let activeAgents = $state(0);
  let totalToolCalls = $state(0);
  let guardianStatus = $state("ACTIVE");
  let systemIntegrity = $state(100);
  let lastActivity = $state<Date | null>(null);

  // View mode toggle
  let viewMode = $state<'agents' | 'network' | 'radar' | 'memory' | 'defense'>('agents');

  // Active Defense state - matches Rust sentinel structs
  interface CpuMetrics {
    load: string;
    frequency: string;
    governor: string;
    turbo_enabled: boolean;
    alert_level: string;
  }

  interface MemoryMetrics {
    available_mb: number;
    memory_percent: number;
    swap_percent: number;
    alert_level: string;
  }

  interface ThermalMetrics {
    hottest: string;
    sensors: string;
    alert_level: string;
  }

  interface GpuMetrics {
    gpus: string;
    utilization: string;
    vram: string;
    thermals: string;
    alert_level: string;
  }

  interface SystemMetrics {
    cpu: CpuMetrics | null;
    memory: MemoryMetrics | null;
    thermal: ThermalMetrics | null;
    gpu: GpuMetrics | null;
  }

  let systemMetrics = $state<SystemMetrics>({
    cpu: null,
    memory: null,
    thermal: null,
    gpu: null
  });

  let defenseLoading = $state(false);
  let defenseAction = $state<string | null>(null);
  let topProcessesCpu = $state<{pid: number; name: string; cpu_percent: number}[]>([]);
  let topProcessesMem = $state<{pid: number; name: string; memory_mb: number}[]>([]);

  // Network monitoring state
  interface NetworkConnection {
    id: string;
    direction: 'incoming' | 'outgoing';
    ip: string;
    port: number;
    protocol: string;
    status: 'active' | 'closed';
    bytesIn: number;
    bytesOut: number;
    timestamp: Date;
    country?: string;
  }

  let networkConnections = $state<NetworkConnection[]>([]);
  let totalBytesIn = $state(0);
  let totalBytesOut = $state(0);
  let activeConnections = $state(0);

  // IP Context Menu state
  interface IpContextMenu {
    visible: boolean;
    x: number;
    y: number;
    ip: string;
  }

  let ipContextMenu = $state<IpContextMenu>({ visible: false, x: 0, y: 0, ip: '' });

  // IP Tools Modal state
  interface IpToolResult {
    tool: string;
    ip: string;
    result: string;
    loading: boolean;
    error: string | null;
  }

  let ipToolsModal = $state<{ visible: boolean; ip: string; results: IpToolResult[] }>({
    visible: false,
    ip: '',
    results: []
  });

  // Blocked IPs tracking
  let blockedIps = $state<Set<string>>(new Set());
  let blockingInProgress = $state<string | null>(null);

  // Radar view state - Physical signal detection
  interface WifiDevice {
    ssid: string;
    bssid: string;
    signal_strength: number;
    frequency: string;
    security: string;
    trusted: boolean;
  }

  interface BluetoothDevice {
    name: string;
    address: string;
    rssi: number;
    device_type: string;
    trusted: boolean;
  }

  let wifiDevices = $state<WifiDevice[]>([]);
  let bluetoothDevices = $state<BluetoothDevice[]>([]);
  let radarScanning = $state(false);
  let lastRadarScan = $state<Date | null>(null);

  // Memory Bank state
  interface AgentMemory {
    id: string;
    agent_name: string;
    memory_type: string;
    flight_id: string | null;
    user_id: string | null;
    query: string | null;
    content: string;
    summary: string | null;
    tokens_used: number;
    cost_usd: number;
    model: string | null;
    created_at: string;
    expires_at: string | null;
    last_accessed: string | null;
    access_count: number;
    confidence_score: number | null;
    verified: boolean;
  }

  interface MemoryStats {
    total_memories: number;
    total_tokens: number;
    total_cost: number;
    memories_by_agent: [string, number][];
    memories_by_type: [string, number][];
  }

  let recentMemories = $state<AgentMemory[]>([]);
  let memoryStats = $state<MemoryStats | null>(null);
  let memorySearchQuery = $state('');
  let memoryAgentFilter = $state<string | null>(null);
  let selectedMemory = $state<AgentMemory | null>(null);

  // Agent tracking state - Live operations board
  interface LiveAgent {
    agent_name: string;
    model: string;
    status: 'idle' | 'thinking' | 'executing' | 'complete' | 'error';
    current_operation: string | null;
    tokens_input: number;
    tokens_output: number;
    cost_usd: number;
    last_updated: string;
  }

  let liveAgents = $state<Map<string, LiveAgent>>(new Map([
    ['Grok', { agent_name: 'Grok', model: 'grok-2-1212', status: 'idle', current_operation: null, tokens_input: 0, tokens_output: 0, cost_usd: 0, last_updated: new Date().toISOString() }],
    ['DeepSeek', { agent_name: 'DeepSeek', model: 'deepseek-chat', status: 'idle', current_operation: null, tokens_input: 0, tokens_output: 0, cost_usd: 0, last_updated: new Date().toISOString() }],
    ['Gemini', { agent_name: 'Gemini', model: 'gemini-2.0-flash-exp', status: 'idle', current_operation: null, tokens_input: 0, tokens_output: 0, cost_usd: 0, last_updated: new Date().toISOString() }],
  ]));

  // Filtered terminal lines based on view mode
  let filteredLogs = $derived(() => {
    if (viewMode === 'network') {
      // Show network-related logs
      return terminalLines.filter(line =>
        line.includes('NET:') ||
        line.includes('NETWORK:') ||
        line.includes('SYSTEM:')
      );
    } else if (viewMode === 'radar') {
      // Show radar-related logs
      return terminalLines.filter(line =>
        line.includes('RADAR:') ||
        line.includes('WIFI:') ||
        line.includes('BLUETOOTH:') ||
        line.includes('SCAN:') ||
        line.includes('SYSTEM:')
      );
    } else if (viewMode === 'memory') {
      // Show memory-related logs
      return terminalLines.filter(line =>
        line.includes('MEMORY:') ||
        line.includes('SYSTEM:')
      );
    } else if (viewMode === 'defense') {
      // Show defense-related logs
      return terminalLines.filter(line =>
        line.includes('DEFENSE:') ||
        line.includes('CPU:') ||
        line.includes('GPU:') ||
        line.includes('THERMAL:') ||
        line.includes('PROCESS:') ||
        line.includes('EMERGENCY:') ||
        line.includes('SYSTEM:')
      );
    } else {
      // Show agent-related logs
      return terminalLines.filter(line =>
        line.includes('✅') ||
        line.includes('❌') ||
        line.includes('TEST:') ||
        line.includes('SYSTEM:')
      );
    }
  });

  // Theme-specific colors
  let themeColors = $derived(() => {
    const currentTheme = $theme;
    if (currentTheme === 'cyberpunk') {
      return {
        primary: 'cyan-500',
        secondary: 'pink-500',
        accent: 'purple-500',
        text: 'cyan-400',
        bg: 'cyan-900',
        border: 'cyan-500',
        glow: 'rgba(0,217,255,0.3)',
        name: 'CYBERPUNK'
      };
    } else if (currentTheme === 'skynet') {
      return {
        primary: 'blue-500',
        secondary: 'cyan-500',
        accent: 'blue-600',
        text: 'blue-400',
        bg: 'blue-900',
        border: 'blue-500',
        glow: 'rgba(0,128,255,0.3)',
        name: 'SKYNET'
      };
    } else {
      // Default green terminal theme
      return {
        primary: 'green-500',
        secondary: 'yellow-500',
        accent: 'cyan-500',
        text: 'green-400',
        bg: 'green-900',
        border: 'green-500',
        glow: 'rgba(0,255,0,0.3)',
        name: 'GUARDIAN'
      };
    }
  });

  // --- Mock Data for the Graph (Placeholder) ---
  // In a real implementation, this would be reactive to backend events
  let nodes = [
    { x: 400, y: 300, type: 'HQ', label: 'COMMAND' },
    { x: 200, y: 150, type: 'AGENT', label: 'GROK-1' },
    { x: 600, y: 150, type: 'AGENT', label: 'CLAUDE-A' },
    { x: 400, y: 100, type: 'TARGET', label: '192.168.1.X' },
  ];

  // --- Event Listeners ---
  let unlistenToolCall: (() => void) | null = null;
  let unlistenToolResult: (() => void) | null = null;
  let unlistenNetworkActivity: (() => void) | null = null;
  let unlistenAgentStatus: (() => void) | null = null;

  // Helper function to format bytes
  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
  }

  // Scan for WiFi and Bluetooth devices (Physical Security Radar)
  async function scanPhysicalDevices() {
    if (radarScanning) return;

    radarScanning = true;
    addLog("SCAN: Initiating physical device scan...", 'system');

    try {
      const dbPath = await invoke('get_setting', { key: 'db_path' }) as string || '';

      // Scan WiFi networks
      addLog("WIFI: Scanning for wireless networks...", 'system');
      const wifi = await invoke('scan_wifi_networks', { dbPath }) as WifiDevice[];
      wifiDevices = wifi;
      addLog(`WIFI: Detected ${wifi.length} wireless networks`, 'info');

      // Log threats (unknown devices)
      const unknownWifi = wifi.filter(d => !d.trusted);
      if (unknownWifi.length > 0) {
        addLog(`RADAR: ⚠️  ${unknownWifi.length} unknown WiFi device(s) detected`, 'error');
      }

      // Scan Bluetooth devices
      addLog("BLUETOOTH: Scanning for BLE devices...", 'system');
      const bt = await invoke('scan_bluetooth_devices', { dbPath }) as BluetoothDevice[];
      bluetoothDevices = bt;
      addLog(`BLUETOOTH: Detected ${bt.length} BLE devices`, 'info');

      // Log threats (unknown devices)
      const unknownBt = bt.filter(d => !d.trusted);
      if (unknownBt.length > 0) {
        addLog(`RADAR: ⚠️  ${unknownBt.length} unknown Bluetooth device(s) detected`, 'error');
      }

      lastRadarScan = new Date();
      addLog("SCAN: Physical device scan complete", 'success');
    } catch (error) {
      addLog(`RADAR: Scan failed - ${error}`, 'error');
    } finally {
      radarScanning = false;
    }
  }

  // Mark device as trusted
  async function trustDevice(type: 'wifi' | 'bluetooth', identifier: string) {
    try {
      const dbPath = await invoke('get_setting', { key: 'db_path' }) as string || '';

      if (type === 'wifi') {
        await invoke('trust_wifi_device', { dbPath, bssid: identifier });
        wifiDevices = wifiDevices.map(d =>
          d.bssid === identifier ? { ...d, trusted: true } : d
        );
        addLog(`RADAR: Marked WiFi device ${identifier} as trusted`, 'success');
      } else {
        await invoke('trust_bluetooth_device', { dbPath, address: identifier });
        bluetoothDevices = bluetoothDevices.map(d =>
          d.address === identifier ? { ...d, trusted: true } : d
        );
        addLog(`RADAR: Marked Bluetooth device ${identifier} as trusted`, 'success');
      }
    } catch (error) {
      addLog(`RADAR: Failed to trust device - ${error}`, 'error');
    }
  }

  // Load memory statistics
  async function loadMemoryStats() {
    try {
      const stats = await invoke('get_memory_stats') as MemoryStats;
      memoryStats = stats;
      addLog(`MEMORY: Loaded ${stats.total_memories} memories`, 'info');
    } catch (error) {
      addLog(`MEMORY: Failed to load statistics - ${error}`, 'error');
    }
  }

  // Load recent memories
  async function loadRecentMemories(limit: number = 50, agentFilter: string | null = null) {
    try {
      const memories = await invoke('get_recent_memories', {
        limit,
        agentFilter
      }) as AgentMemory[];
      recentMemories = memories;
      addLog(`MEMORY: Loaded ${memories.length} recent memories`, 'info');
    } catch (error) {
      addLog(`MEMORY: Failed to load memories - ${error}`, 'error');
    }
  }

  // Search memories
  async function searchMemories(query: string, limit: number = 20) {
    if (!query.trim()) {
      await loadRecentMemories();
      return;
    }

    try {
      const results = await invoke('search_agent_memories', {
        searchQuery: query,
        limit
      }) as { memory: AgentMemory; relevance_score: number }[];

      recentMemories = results.map(r => r.memory);
      addLog(`MEMORY: Found ${results.length} matching memories`, 'info');
    } catch (error) {
      addLog(`MEMORY: Search failed - ${error}`, 'error');
    }
  }

  // ===== ACTIVE DEFENSE FUNCTIONS =====

  // Load all system metrics
  async function loadSystemMetrics() {
    defenseLoading = true;
    addLog("DEFENSE: Fetching system metrics...", 'system');

    try {
      const [cpu, memory, thermal, gpu] = await Promise.all([
        invoke('get_cpu_snapshot').catch((e) => { console.warn('CPU snapshot failed:', e); return null; }),
        invoke('get_memory_snapshot').catch((e) => { console.warn('Memory snapshot failed:', e); return null; }),
        invoke('get_thermal_snapshot').catch((e) => { console.warn('Thermal snapshot failed:', e); return null; }),
        invoke('get_gpu_snapshot').catch((e) => { console.warn('GPU snapshot failed:', e); return null; })
      ]);

      // Tauri already deserializes the response - no need to JSON.parse
      systemMetrics = {
        cpu: cpu as any || null,
        memory: memory as any || null,
        thermal: thermal as any || null,
        gpu: gpu as any || null
      };

      addLog("DEFENSE: System metrics updated", 'success');
    } catch (error) {
      addLog(`DEFENSE: Failed to load metrics - ${error}`, 'error');
    } finally {
      defenseLoading = false;
    }
  }

  // Load top processes
  async function loadTopProcesses() {
    try {
      const [cpuProcs, memProcs] = await Promise.all([
        invoke('process_get_top_cpu', { count: 5 }),
        invoke('process_get_top_memory', { count: 5 })
      ]);

      // Tauri already deserializes - responses are now Vec<ProcessInfo>
      topProcessesCpu = cpuProcs as {pid: number; name: string; cpu_percent: number}[];
      topProcessesMem = memProcs as {pid: number; name: string; memory_mb: number}[];
    } catch (error) {
      addLog(`DEFENSE: Failed to load top processes - ${error}`, 'error');
    }
  }

  // CPU Controls
  async function setCpuGovernor(governor: string) {
    defenseAction = `Setting CPU governor to ${governor}`;
    try {
      await invoke('cpu_set_governor', { governor });
      addLog(`CPU: Governor set to ${governor}`, 'success');
      await loadSystemMetrics();
    } catch (error) {
      addLog(`CPU: Failed to set governor - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  async function toggleTurbo(enable: boolean) {
    defenseAction = enable ? 'Enabling turbo boost' : 'Disabling turbo boost';
    try {
      if (enable) {
        await invoke('cpu_enable_turbo');
        addLog("CPU: Turbo boost enabled", 'success');
      } else {
        await invoke('cpu_disable_turbo');
        addLog("CPU: Turbo boost disabled", 'success');
      }
      await loadSystemMetrics();
    } catch (error) {
      addLog(`CPU: Failed to toggle turbo - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  // Memory Controls
  async function dropCaches() {
    defenseAction = 'Dropping memory caches';
    try {
      await invoke('memory_drop_caches');
      addLog("DEFENSE: Memory caches dropped", 'success');
      await loadSystemMetrics();
    } catch (error) {
      addLog(`DEFENSE: Failed to drop caches - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  async function compactMemory() {
    defenseAction = 'Compacting memory';
    try {
      await invoke('memory_compact');
      addLog("DEFENSE: Memory compacted", 'success');
      await loadSystemMetrics();
    } catch (error) {
      addLog(`DEFENSE: Failed to compact memory - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  // Thermal Controls
  async function emergencyCool() {
    defenseAction = 'Emergency cooling activated';
    try {
      await invoke('thermal_emergency_cool');
      addLog("THERMAL: Emergency cooling activated", 'success');
      await loadSystemMetrics();
    } catch (error) {
      addLog(`THERMAL: Emergency cool failed - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  // GPU Controls
  async function gpuEmergencyThrottle() {
    defenseAction = 'GPU emergency throttle';
    try {
      await invoke('gpu_emergency_throttle');
      addLog("GPU: Emergency throttle activated", 'success');
      await loadSystemMetrics();
    } catch (error) {
      addLog(`GPU: Emergency throttle failed - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  // Process Controls
  async function freezeProcess(pid: number) {
    defenseAction = `Freezing process ${pid}`;
    try {
      await invoke('process_freeze', { pid });
      addLog(`PROCESS: Froze process ${pid}`, 'success');
      await loadTopProcesses();
    } catch (error) {
      addLog(`PROCESS: Failed to freeze ${pid} - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  async function killProcess(pid: number, signal: number = 9) {
    defenseAction = `Killing process ${pid}`;
    try {
      await invoke('process_kill', { pid, signal });
      addLog(`PROCESS: Killed process ${pid}`, 'success');
      await loadTopProcesses();
    } catch (error) {
      addLog(`PROCESS: Failed to kill ${pid} - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  async function reapZombies() {
    defenseAction = 'Reaping zombie processes';
    try {
      const result = await invoke('process_reap_zombies');
      addLog(`PROCESS: ${result}`, 'success');
    } catch (error) {
      addLog(`PROCESS: Failed to reap zombies - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  // PANIC BUTTONS
  async function emergencyAllSystems() {
    defenseAction = 'EMERGENCY ALL SYSTEMS';
    addLog("EMERGENCY: Activating all emergency protocols!", 'error');
    try {
      await invoke('emergency_all_systems');
      addLog("EMERGENCY: All systems in emergency mode", 'success');
      await loadSystemMetrics();
    } catch (error) {
      addLog(`EMERGENCY: Failed - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  async function lockdownNetwork() {
    defenseAction = 'NETWORK LOCKDOWN';
    addLog("EMERGENCY: Network lockdown initiated!", 'error');
    try {
      await invoke('lockdown_network');
      addLog("EMERGENCY: Network locked down", 'success');
    } catch (error) {
      addLog(`EMERGENCY: Lockdown failed - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  async function performanceMode() {
    defenseAction = 'Activating performance mode';
    try {
      await invoke('performance_mode');
      addLog("DEFENSE: Performance mode activated", 'success');
      await loadSystemMetrics();
    } catch (error) {
      addLog(`DEFENSE: Performance mode failed - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  async function resetAllControls() {
    defenseAction = 'Resetting all controls';
    try {
      await invoke('reset_all_controls');
      addLog("DEFENSE: All controls reset to defaults", 'success');
      await loadSystemMetrics();
    } catch (error) {
      addLog(`DEFENSE: Reset failed - ${error}`, 'error');
    } finally {
      defenseAction = null;
    }
  }

  // ===== IP TOOLS FUNCTIONS =====

  function showIpContextMenu(event: MouseEvent, ip: string) {
    event.preventDefault();
    event.stopPropagation();
    ipContextMenu = {
      visible: true,
      x: event.clientX,
      y: event.clientY,
      ip
    };
  }

  function hideIpContextMenu() {
    ipContextMenu = { visible: false, x: 0, y: 0, ip: '' };
  }

  function openIpToolsModal(ip: string) {
    hideIpContextMenu();
    ipToolsModal = {
      visible: true,
      ip,
      results: []
    };
  }

  async function runIpTool(tool: 'whois' | 'nslookup' | 'ping' | 'traceroute' | 'geoip') {
    const ip = ipToolsModal.ip;
    if (!ip) return;

    // Add loading entry
    const newResult: IpToolResult = {
      tool,
      ip,
      result: '',
      loading: true,
      error: null
    };
    ipToolsModal.results = [...ipToolsModal.results, newResult];
    addLog(`NET: Running ${tool} on ${ip}...`, 'system');

    try {
      let result: string;
      switch (tool) {
        case 'whois':
          result = await invoke('network_whois', { ip });
          break;
        case 'nslookup':
          result = await invoke('network_nslookup', { ip });
          break;
        case 'ping':
          result = await invoke('network_ping', { ip });
          break;
        case 'traceroute':
          result = await invoke('network_traceroute', { ip });
          break;
        case 'geoip':
          result = await invoke('network_geoip', { ip });
          break;
        default:
          result = 'Unknown tool';
      }

      // Update result
      ipToolsModal.results = ipToolsModal.results.map(r =>
        r.tool === tool && r.ip === ip && r.loading
          ? { ...r, result, loading: false }
          : r
      );
      addLog(`NET: ${tool} on ${ip} completed`, 'success');
    } catch (error) {
      ipToolsModal.results = ipToolsModal.results.map(r =>
        r.tool === tool && r.ip === ip && r.loading
          ? { ...r, error: String(error), loading: false }
          : r
      );
      addLog(`NET: ${tool} on ${ip} failed - ${error}`, 'error');
    }
  }

  function closeIpToolsModal() {
    ipToolsModal = { visible: false, ip: '', results: [] };
  }

  // ===== IP BLOCKING FUNCTIONS =====

  // SentinelResult type from active_defense commands
  interface SentinelResult {
    success: boolean;
    message: string;
    data: string | null;
  }

  async function loadBlockedIps() {
    // The D-Bus sentinel doesn't have a "get blocked IPs" method currently
    // We track blocked IPs locally in the session
    // In a production system, you'd query the firewall rules
    console.log('Blocked IPs loaded from local state');
  }

  async function blockIp(ip: string) {
    if (blockingInProgress) return;
    blockingInProgress = ip;
    hideIpContextMenu();

    addLog(`NET: Blocking IP ${ip}...`, 'system');

    try {
      const result = await invoke('network_block_ip', { ip }) as SentinelResult;
      if (result.success) {
        blockedIps = new Set([...blockedIps, ip]);
        addLog(`NET: ${result.message}`, 'success');

        // Update the connection status in the list
        networkConnections = networkConnections.map(conn =>
          conn.ip === ip ? { ...conn, status: 'closed' as const } : conn
        );
      } else {
        addLog(`NET: ${result.message}`, 'error');
      }
    } catch (error) {
      addLog(`NET: Failed to block ${ip} - ${error}`, 'error');
    } finally {
      blockingInProgress = null;
    }
  }

  async function unblockIp(ip: string) {
    if (blockingInProgress) return;
    blockingInProgress = ip;

    addLog(`NET: Unblocking IP ${ip}...`, 'system');

    try {
      const result = await invoke('network_unblock_ip', { ip }) as SentinelResult;
      if (result.success) {
        const newBlocked = new Set(blockedIps);
        newBlocked.delete(ip);
        blockedIps = newBlocked;
        addLog(`NET: ${result.message}`, 'success');
      } else {
        addLog(`NET: ${result.message}`, 'error');
      }
    } catch (error) {
      addLog(`NET: Failed to unblock ${ip} - ${error}`, 'error');
    } finally {
      blockingInProgress = null;
    }
  }

  async function clearAllBlocks() {
    if (blockingInProgress) return;
    blockingInProgress = 'all';

    addLog(`NET: Clearing all IP blocks...`, 'system');

    try {
      const result = await invoke('network_clear_all_blocks') as SentinelResult;
      if (result.success) {
        blockedIps = new Set();
        addLog(`NET: ${result.message}`, 'success');
      } else {
        addLog(`NET: ${result.message}`, 'error');
      }
    } catch (error) {
      addLog(`NET: Failed to clear blocks - ${error}`, 'error');
    } finally {
      blockingInProgress = null;
    }
  }

  function isIpBlocked(ip: string): boolean {
    return blockedIps.has(ip);
  }

  // Helper function to simulate network activity (for demo)
  function simulateNetworkActivity() {
    const protocols = ['HTTP', 'HTTPS', 'TCP', 'UDP', 'WebSocket'];
    const countries = ['US', 'GB', 'DE', 'JP', 'FR', 'CA', 'AU', 'BR'];
    const directions: ('incoming' | 'outgoing')[] = ['incoming', 'outgoing'];

    setInterval(() => {
      // Randomly add new connections
      if (Math.random() > 0.7) {
        const connection: NetworkConnection = {
          id: Math.random().toString(36).substr(2, 9),
          direction: directions[Math.floor(Math.random() * directions.length)],
          ip: `${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}`,
          port: Math.floor(Math.random() * 65535),
          protocol: protocols[Math.floor(Math.random() * protocols.length)],
          status: 'active',
          bytesIn: Math.floor(Math.random() * 100000),
          bytesOut: Math.floor(Math.random() * 100000),
          timestamp: new Date(),
          country: countries[Math.floor(Math.random() * countries.length)]
        };

        networkConnections = [connection, ...networkConnections].slice(0, 50);
        totalBytesIn += connection.bytesIn;
        totalBytesOut += connection.bytesOut;
        activeConnections = networkConnections.filter(c => c.status === 'active').length;

        addLog(`NET: ${connection.direction.toUpperCase()} ${connection.ip}:${connection.port} [${connection.protocol}]`, 'info');
      }

      // Randomly close some connections
      networkConnections = networkConnections.map(conn => {
        if (conn.status === 'active' && Math.random() > 0.9) {
          return { ...conn, status: 'closed' as const };
        }
        return conn;
      });

      activeConnections = networkConnections.filter(c => c.status === 'active').length;
    }, 2000);
  }

  onMount(async () => {
    // Listen for AI agent tool calls
    unlistenToolCall = await listen('ai:tool_call', (event: any) => {
      const { agent, tool, args, timestamp } = event.payload;
      const time = new Date(timestamp || Date.now()).toLocaleTimeString();

      const argsStr = JSON.stringify(args, null, 0)
        .substring(0, 80)
        .replace(/\n/g, ' ');

      const line = `[${time}] ${agent || 'AGENT'} >> ${tool}(${argsStr}${args && JSON.stringify(args).length > 80 ? '...' : ''})`;

      addLog(line, 'tool-call');
      totalToolCalls++;
      lastActivity = new Date();

      // Update active agents count (unique agents)
      const agentMatch = line.match(/(\S+)/);
      if (agentMatch) {
        activeAgents = Math.max(activeAgents, 1);
      }
    });

    // Listen for AI agent tool results
    unlistenToolResult = await listen('ai:tool_result', (event: any) => {
      const { agent, tool, success, error, timestamp } = event.payload;
      const time = new Date(timestamp || Date.now()).toLocaleTimeString();

      if (success) {
        addLog(`[${time}] ✅ ${tool} >> SUCCESS`, 'success');
      } else {
        addLog(`[${time}] ❌ ${tool} >> ERROR: ${error || 'Unknown'}`, 'error');
      }
    });

    // Listen for network activity events
    unlistenNetworkActivity = await listen('network:activity', (event: any) => {
      const { direction, ip, port, protocol, bytesIn, bytesOut, country } = event.payload;

      const connection: NetworkConnection = {
        id: Math.random().toString(36).substr(2, 9),
        direction,
        ip,
        port,
        protocol,
        status: 'active',
        bytesIn,
        bytesOut,
        timestamp: new Date(),
        country
      };

      networkConnections = [connection, ...networkConnections].slice(0, 50);
      totalBytesIn += bytesIn;
      totalBytesOut += bytesOut;
      activeConnections = networkConnections.filter(c => c.status === 'active').length;

      addLog(`NET: ${direction.toUpperCase()} ${ip}:${port} [${protocol}]`, 'info');
    });

    // Listen for agent status events (Live Operations Board)
    unlistenAgentStatus = await listen('agent:status', (event: any) => {
      const { agent_name, model, event_type, operation, tokens_input, tokens_output, cost_usd, timestamp } = event.payload;
      const time = new Date(timestamp || Date.now()).toLocaleTimeString();

      // Get or create agent entry
      const currentAgent = liveAgents.get(agent_name) || {
        agent_name,
        model,
        status: 'idle' as const,
        current_operation: null,
        tokens_input: 0,
        tokens_output: 0,
        cost_usd: 0,
        last_updated: timestamp
      };

      // Update agent state based on event type
      switch (event_type) {
        case 'start':
          liveAgents.set(agent_name, {
            ...currentAgent,
            model,
            status: 'executing',
            current_operation: operation || null,
            last_updated: timestamp
          });
          addLog(`[${time}] AGENT: ${agent_name} started - ${operation}`, 'system');
          activeAgents = Array.from(liveAgents.values()).filter(a => a.status !== 'idle').length;
          break;

        case 'thinking':
          liveAgents.set(agent_name, {
            ...currentAgent,
            status: 'thinking',
            last_updated: timestamp
          });
          break;

        case 'executing':
          liveAgents.set(agent_name, {
            ...currentAgent,
            status: 'executing',
            current_operation: operation || currentAgent.current_operation,
            last_updated: timestamp
          });
          break;

        case 'token_update':
          liveAgents.set(agent_name, {
            ...currentAgent,
            tokens_input: tokens_input || currentAgent.tokens_input,
            tokens_output: tokens_output || currentAgent.tokens_output,
            cost_usd: cost_usd || currentAgent.cost_usd,
            last_updated: timestamp
          });
          break;

        case 'complete':
          liveAgents.set(agent_name, {
            ...currentAgent,
            status: 'complete',
            tokens_input: tokens_input || currentAgent.tokens_input,
            tokens_output: tokens_output || currentAgent.tokens_output,
            cost_usd: cost_usd || currentAgent.cost_usd,
            current_operation: null,
            last_updated: timestamp
          });
          addLog(`[${time}] AGENT: ${agent_name} completed - ${currentAgent.tokens_input + (tokens_input || 0)} tokens`, 'success');

          // Reset to idle after brief delay
          setTimeout(() => {
            const agent = liveAgents.get(agent_name);
            if (agent && agent.status === 'complete') {
              liveAgents.set(agent_name, { ...agent, status: 'idle' });
              activeAgents = Array.from(liveAgents.values()).filter(a => a.status !== 'idle').length;
            }
          }, 3000);
          break;

        case 'error':
          liveAgents.set(agent_name, {
            ...currentAgent,
            status: 'error',
            current_operation: operation || currentAgent.current_operation,
            last_updated: timestamp
          });
          addLog(`[${time}] AGENT: ${agent_name} error - ${operation}`, 'error');

          // Reset to idle after delay
          setTimeout(() => {
            const agent = liveAgents.get(agent_name);
            if (agent && agent.status === 'error') {
              liveAgents.set(agent_name, { ...agent, status: 'idle', current_operation: null });
              activeAgents = Array.from(liveAgents.values()).filter(a => a.status !== 'idle').length;
            }
          }, 5000);
          break;
      }

      lastActivity = new Date();
      liveAgents = new Map(liveAgents); // Trigger reactivity
    });

    // Initial boot sequence
    addLog("SYSTEM: Command Centre Initialized", 'system');
    addLog("SYSTEM: Guardian Shield Active", 'system');
    addLog("SYSTEM: Monitoring AI Agent Activity...", 'system');

    // Initialize memory bank data
    loadMemoryStats();
    loadRecentMemories();

    // Simulate some activity for demonstration
    setTimeout(() => {
      addLog("NETWORK: All systems operational", 'system');
      // Start network simulation (for demo purposes)
      simulateNetworkActivity();
      // Load blocked IPs
      loadBlockedIps();
    }, 1000);
  });

  onDestroy(() => {
    if (unlistenToolCall) unlistenToolCall();
    if (unlistenToolResult) unlistenToolResult();
    if (unlistenNetworkActivity) unlistenNetworkActivity();
    if (unlistenAgentStatus) unlistenAgentStatus();
  });

  function addLog(msg: string, type: 'system' | 'tool-call' | 'success' | 'error' | 'info' = 'info') {
    const logEntry = { msg, type, timestamp: Date.now() };
    terminalLines = [...terminalLines, msg];

    // Keep only last 500 lines to prevent memory issues
    if (terminalLines.length > 500) {
      terminalLines = terminalLines.slice(-500);
    }

    // Auto-scroll terminal
    if (terminalContainer) {
      const container = terminalContainer;
      setTimeout(() => container.scrollTop = container.scrollHeight, 10);
    }
  }

  function clearLogs() {
    terminalLines = [];
    addLog("SYSTEM: Logs cleared", 'system');
  }

  async function testConnection() {
    addLog("TEST: Initiating connection test...", 'system');
    try {
      await invoke('get_statistics', { userId: 'test' });
      addLog("TEST: Backend connection successful", 'success');
    } catch (error) {
      addLog(`TEST: Connection failed - ${error}`, 'error');
    }
  }

</script>

<!-- Only show if we are in Developer Mode -->
{#if $developerMode}
  <div class="h-full grid grid-cols-12 gap-4 p-4 bg-black text-{themeColors().primary} font-mono" in:fade>
    
    <!-- Left Column: Status & Control -->
    <div class="col-span-3 flex flex-col gap-4">
      
      <!-- Mode Toggle -->
      <div class="grid grid-cols-5 gap-1 p-1 border border-{themeColors().border}/30 bg-{themeColors().bg}/10 rounded">
        <button
          onclick={() => viewMode = 'agents'}
          class="py-2 px-1 text-[10px] font-bold uppercase transition rounded {
            viewMode === 'agents'
              ? `bg-${themeColors().primary} text-black`
              : `text-${themeColors().text} hover:bg-${themeColors().bg}/20`
          }"
        >
          Agents
        </button>
        <button
          onclick={() => viewMode = 'network'}
          class="py-2 px-1 text-[10px] font-bold uppercase transition rounded {
            viewMode === 'network'
              ? `bg-${themeColors().primary} text-black`
              : `text-${themeColors().text} hover:bg-${themeColors().bg}/20`
          }"
        >
          Network
        </button>
        <button
          onclick={() => viewMode = 'radar'}
          class="py-2 px-1 text-[10px] font-bold uppercase transition rounded flex items-center justify-center gap-1 {
            viewMode === 'radar'
              ? `bg-${themeColors().primary} text-black`
              : `text-${themeColors().text} hover:bg-${themeColors().bg}/20`
          }"
        >
          📡
        </button>
        <button
          onclick={() => { viewMode = 'memory'; loadMemoryStats(); loadRecentMemories(); }}
          class="py-2 px-1 text-[10px] font-bold uppercase transition rounded {
            viewMode === 'memory'
              ? `bg-${themeColors().primary} text-black`
              : `text-${themeColors().text} hover:bg-${themeColors().bg}/20`
          }"
        >
          Memory
        </button>
        <button
          onclick={() => { viewMode = 'defense'; loadSystemMetrics(); loadTopProcesses(); }}
          class="py-2 px-1 text-[10px] font-bold uppercase transition rounded {
            viewMode === 'defense'
              ? `bg-red-500 text-black`
              : `text-red-400 hover:bg-red-900/20`
          }"
        >
          Defense
        </button>
      </div>

      <!-- Status Panel -->
      <div class="border border-{themeColors().border}/30 bg-{themeColors().bg}/10 p-4 rounded" style="box-shadow: 0 0 15px {themeColors().glow}">
        <h2 class="text-xl font-bold mb-4 border-b border-{themeColors().border}/50 pb-2">
          {themeColors().name} // SYSTEM STATUS
        </h2>
        <div class="space-y-2 text-sm">
          <div class="flex justify-between">
            <span>SHIELD:</span>
            <span class="text-{themeColors().text} font-bold animate-pulse">{guardianStatus}</span>
          </div>
          <div class="flex justify-between">
            <span>INTEGRITY:</span>
            <span>{systemIntegrity}%</span>
          </div>

          {#if viewMode === 'agents'}
            <div class="flex justify-between">
              <span>AGENTS:</span>
              <span>{activeAgents} ACTIVE</span>
            </div>
            <div class="flex justify-between">
              <span>TOOL CALLS:</span>
              <span class="text-{themeColors().accent}">{totalToolCalls}</span>
            </div>
          {:else if viewMode === 'network'}
            <div class="flex justify-between">
              <span>CONNECTIONS:</span>
              <span>{activeConnections} ACTIVE</span>
            </div>
            <div class="flex justify-between">
              <span>DATA IN:</span>
              <span class="text-{themeColors().accent}">{formatBytes(totalBytesIn)}</span>
            </div>
            <div class="flex justify-between">
              <span>DATA OUT:</span>
              <span class="text-{themeColors().secondary}">{formatBytes(totalBytesOut)}</span>
            </div>
          {:else if viewMode === 'radar'}
            <div class="flex justify-between">
              <span>WIFI DEVICES:</span>
              <span class="text-{themeColors().accent}">{wifiDevices.length}</span>
            </div>
            <div class="flex justify-between">
              <span>BLUETOOTH:</span>
              <span class="text-{themeColors().secondary}">{bluetoothDevices.length}</span>
            </div>
            <div class="flex justify-between">
              <span>THREATS:</span>
              <span class="text-red-500">{wifiDevices.filter(d => !d.trusted).length + bluetoothDevices.filter(d => !d.trusted).length}</span>
            </div>
          {:else if viewMode === 'memory'}
            <div class="flex justify-between">
              <span>MEMORIES:</span>
              <span class="text-{themeColors().accent}">{memoryStats?.total_memories || 0}</span>
            </div>
            <div class="flex justify-between">
              <span>TOTAL TOKENS:</span>
              <span class="text-{themeColors().secondary}">{memoryStats?.total_tokens.toLocaleString() || 0}</span>
            </div>
            <div class="flex justify-between">
              <span>TOTAL COST:</span>
              <span class="text-{themeColors().text}">${memoryStats?.total_cost.toFixed(4) || '0.0000'}</span>
            </div>
          {:else if viewMode === 'defense'}
            <div class="flex justify-between">
              <span>CPU:</span>
              <span class="text-{systemMetrics.cpu?.alert_level === 'critical' ? 'red-500' : systemMetrics.cpu?.alert_level === 'warning' ? 'yellow-500' : themeColors().accent}">
                {systemMetrics.cpu?.load || '--'}
              </span>
            </div>
            <div class="flex justify-between">
              <span>MEMORY:</span>
              <span class="text-{systemMetrics.memory?.alert_level === 'critical' ? 'red-500' : systemMetrics.memory?.alert_level === 'warning' ? 'yellow-500' : themeColors().secondary}">
                {systemMetrics.memory?.memory_percent?.toFixed(1) || '--'}%
              </span>
            </div>
            <div class="flex justify-between">
              <span>THERMAL:</span>
              <span class="text-{systemMetrics.thermal?.alert_level === 'critical' ? 'red-500' : systemMetrics.thermal?.alert_level === 'warning' ? 'yellow-500' : themeColors().text}">
                {systemMetrics.thermal?.hottest || '--'}
              </span>
            </div>
            <div class="flex justify-between">
              <span>GPU:</span>
              <span class="text-{systemMetrics.gpu?.alert_level === 'critical' ? 'red-500' : systemMetrics.gpu?.alert_level === 'warning' ? 'yellow-500' : themeColors().accent}">
                {systemMetrics.gpu?.utilization || '--'}
              </span>
            </div>
          {/if}

          {#if lastActivity}
            <div class="flex justify-between text-xs pt-2 border-t border-{themeColors().border}/20">
              <span>LAST ACTIVITY:</span>
              <span class="text-{themeColors().secondary}">{lastActivity.toLocaleTimeString()}</span>
            </div>
          {/if}
        </div>
      </div>

      <!-- Control Buttons -->
      <div class="flex flex-col gap-2">
        {#if viewMode === 'radar'}
          <button
            onclick={scanPhysicalDevices}
            disabled={radarScanning}
            class="py-3 bg-{themeColors().primary}/20 border border-{themeColors().primary} text-{themeColors().primary} hover:bg-{themeColors().primary} hover:text-black disabled:opacity-50 disabled:cursor-not-allowed transition-all uppercase font-bold text-sm tracking-wider"
            style="box-shadow: 0 0 15px {themeColors().glow}"
          >
            {radarScanning ? '📡 SCANNING...' : '📡 SCAN DEVICES'}
          </button>
        {:else if viewMode === 'defense'}
          <button
            onclick={loadSystemMetrics}
            disabled={defenseLoading}
            class="py-3 bg-cyan-500/20 border border-cyan-500 text-cyan-500 hover:bg-cyan-500 hover:text-black disabled:opacity-50 disabled:cursor-not-allowed transition-all uppercase font-bold text-sm tracking-wider"
            style="box-shadow: 0 0 15px rgba(0,217,255,0.3)"
          >
            {defenseLoading ? '⟳ LOADING...' : '⟳ REFRESH'}
          </button>
          <button
            onclick={performanceMode}
            disabled={!!defenseAction}
            class="py-3 bg-green-500/20 border border-green-500 text-green-500 hover:bg-green-500 hover:text-black disabled:opacity-50 transition-all uppercase font-bold text-sm tracking-wider"
            style="box-shadow: 0 0 15px rgba(0,255,0,0.3)"
          >
            PERFORMANCE
          </button>
          <button
            onclick={emergencyAllSystems}
            disabled={!!defenseAction}
            class="py-3 bg-red-500/20 border border-red-500 text-red-500 hover:bg-red-500 hover:text-black disabled:opacity-50 transition-all uppercase font-bold text-sm tracking-wider animate-pulse"
            style="box-shadow: 0 0 20px rgba(255,0,0,0.4)"
          >
            EMERGENCY
          </button>
        {:else}
          <button
            onclick={testConnection}
            class="py-3 bg-{themeColors().accent}/20 border border-{themeColors().accent} text-{themeColors().accent} hover:bg-{themeColors().accent} hover:text-black transition-all uppercase font-bold text-sm tracking-wider"
            style="box-shadow: 0 0 15px {themeColors().glow}"
          >
            TEST CONNECTION
          </button>
        {/if}
        <button
          onclick={clearLogs}
          class="py-3 bg-{themeColors().secondary}/20 border border-{themeColors().secondary} text-{themeColors().secondary} hover:bg-{themeColors().secondary} hover:text-black transition-all uppercase font-bold text-sm tracking-wider"
          style="box-shadow: 0 0 15px {themeColors().glow}"
        >
          CLEAR LOGS
        </button>
      </div>
    </div>

    <!-- Center Column: Activity Visualization -->
    <div class="col-span-6 border border-{themeColors().border}/30 bg-black rounded relative overflow-hidden p-4">
        <div class="absolute top-2 left-2 text-xs opacity-50">
          {viewMode === 'agents' ? 'AI AGENT ACTIVITY // LIVE FEED' : viewMode === 'network' ? 'NETWORK MONITOR // LIVE TRAFFIC' : viewMode === 'radar' ? 'PHYSICAL SECURITY RADAR // SIGNAL DETECTION' : viewMode === 'defense' ? 'ACTIVE DEFENSE // SYSTEM CONTROLS' : 'MEMORY BANK // AGENT KNOWLEDGE BASE'}
        </div>

        {#if viewMode === 'agents'}
        <!-- Live Agent Operations Board -->
        <div class="w-full h-full flex flex-col gap-4 p-4 overflow-y-auto">
          <!-- Agent Cards -->
          {#each Array.from(liveAgents.values()) as agent (agent.agent_name)}
            {@const statusColor = agent.status === 'thinking' ? themeColors().primary : agent.status === 'executing' ? themeColors().accent : agent.status === 'error' ? 'red-500' : agent.status === 'complete' ? 'green-500' : 'gray-500'}
            {@const costColor = agent.cost_usd > 0.50 ? 'red-500' : agent.cost_usd > 0.10 ? 'yellow-500' : 'green-500'}

            <div class="border border-{themeColors().border}/30 bg-{themeColors().bg}/5 rounded p-4 hover:border-{themeColors().primary}/50 transition"
                 style="box-shadow: 0 0 15px {agent.status !== 'idle' ? themeColors().glow : 'rgba(0,0,0,0.1)'}"
                 in:fly={{ y: -10, duration: 200 }}>

              <!-- Agent Header -->
              <div class="flex items-center justify-between mb-3">
                <div class="flex items-center gap-3">
                  <!-- Status Indicator -->
                  <div class="relative">
                    <div class="text-2xl"></div>
                    {#if agent.status !== 'idle'}
                      <div class="absolute -top-1 -right-1 w-3 h-3 bg-{statusColor} rounded-full animate-pulse"></div>
                    {/if}
                  </div>

                  <!-- Agent Name & Model -->
                  <div>
                    <div class="text-lg font-bold text-{themeColors().primary}">{agent.agent_name}</div>
                    <div class="text-xs opacity-60">{agent.model}</div>
                  </div>
                </div>

                <!-- Status Badge -->
                <div class="px-3 py-1 bg-{statusColor}/20 border border-{statusColor}/40 rounded text-{statusColor} text-xs font-bold uppercase">
                  {agent.status}
                </div>
              </div>

              <!-- Current Operation -->
              {#if agent.current_operation}
                <div class="mb-3 p-2 bg-black/30 rounded border-l-2 border-{themeColors().accent}">
                  <div class="text-xs opacity-60 mb-1">CURRENT OPERATION</div>
                  <div class="text-sm text-{themeColors().text}">{agent.current_operation}</div>
                </div>
              {/if}

              <!-- Metrics Grid -->
              <div class="grid grid-cols-3 gap-3 text-center text-xs">
                <!-- Tokens In -->
                <div class="bg-black/20 p-2 rounded">
                  <div class="opacity-60 mb-1">TOKENS IN</div>
                  <div class="text-{themeColors().accent} font-bold">{agent.tokens_input.toLocaleString()}</div>
                </div>

                <!-- Tokens Out -->
                <div class="bg-black/20 p-2 rounded">
                  <div class="opacity-60 mb-1">TOKENS OUT</div>
                  <div class="text-{themeColors().secondary} font-bold">{agent.tokens_output.toLocaleString()}</div>
                </div>

                <!-- Cost -->
                <div class="bg-black/20 p-2 rounded">
                  <div class="opacity-60 mb-1">COST</div>
                  <div class="text-{costColor} font-bold">${agent.cost_usd.toFixed(4)}</div>
                </div>
              </div>

              <!-- Progress Bar for Active Agents -->
              {#if agent.status === 'thinking' || agent.status === 'executing'}
                <div class="mt-3 h-1 bg-black/30 rounded overflow-hidden">
                  <div class="h-full bg-{themeColors().primary} animate-pulse" style="width: 100%; animation: pulse-width 2s ease-in-out infinite"></div>
                </div>
              {/if}
            </div>
          {/each}

          <!-- Summary Stats -->
          <div class="grid grid-cols-3 gap-4 text-center text-xs mt-auto pt-4 border-t border-{themeColors().border}/20">
            <div class="bg-{themeColors().bg}/10 p-2 rounded">
              <div class="text-{themeColors().text} font-bold text-lg">{totalToolCalls}</div>
              <div class="opacity-60">TOTAL CALLS</div>
            </div>
            <div class="bg-{themeColors().bg}/10 p-2 rounded">
              <div class="text-{themeColors().accent} font-bold text-lg">{activeAgents}</div>
              <div class="opacity-60">ACTIVE NOW</div>
            </div>
            <div class="bg-{themeColors().bg}/10 p-2 rounded">
              <div class="text-{themeColors().secondary} font-bold text-lg">${Array.from(liveAgents.values()).reduce((sum, a) => sum + a.cost_usd, 0).toFixed(4)}</div>
              <div class="opacity-60">TOTAL COST</div>
            </div>
          </div>
        </div>
        {:else if viewMode === 'network'}
        <!-- Network Monitor -->
        <div class="w-full h-full flex flex-col">
          <!-- Network Stats Header -->
          <div class="flex gap-4 mb-4 text-center">
            <div class="flex-1 border border-{themeColors().primary}/20 bg-{themeColors().bg}/5 p-3 rounded">
              <div class="text-{themeColors().primary} font-bold text-2xl">{activeConnections}</div>
              <div class="opacity-60 text-xs">ACTIVE</div>
            </div>
            <div class="flex-1 border border-{themeColors().accent}/20 bg-{themeColors().bg}/5 p-3 rounded">
              <div class="text-{themeColors().accent} font-bold text-2xl">↓ {formatBytes(totalBytesIn)}</div>
              <div class="opacity-60 text-xs">INCOMING</div>
            </div>
            <div class="flex-1 border border-{themeColors().secondary}/20 bg-{themeColors().bg}/5 p-3 rounded">
              <div class="text-{themeColors().secondary} font-bold text-2xl">↑ {formatBytes(totalBytesOut)}</div>
              <div class="opacity-60 text-xs">OUTGOING</div>
            </div>
          </div>

          <!-- Connection Table -->
          <div class="flex-1 overflow-hidden border border-{themeColors().border}/30 rounded" style="box-shadow: 0 0 15px {themeColors().glow}">
            <div class="bg-{themeColors().bg}/20 p-2 border-b border-{themeColors().border}/30">
              <div class="grid grid-cols-12 gap-2 text-xs font-bold uppercase">
                <div class="col-span-1 text-center">DIR</div>
                <div class="col-span-3">IP ADDRESS</div>
                <div class="col-span-1 text-center">PORT</div>
                <div class="col-span-2">PROTOCOL</div>
                <div class="col-span-2">DATA IN/OUT</div>
                <div class="col-span-2">STATUS</div>
                <div class="col-span-1 text-center">LOC</div>
              </div>
            </div>

            <div class="overflow-y-auto h-full scrollbar-thin">
              {#if networkConnections.length === 0}
                <div class="text-{themeColors().text}/50 text-center py-12 text-sm">
                  <div class="text-4xl mb-3"></div>
                  <div>No network activity detected</div>
                  <div class="text-xs opacity-50 mt-2">Connections will appear here</div>
                </div>
              {:else}
                {#each networkConnections as conn (conn.id)}
                  <div
                    class="grid grid-cols-12 gap-2 items-center p-2 text-xs border-b border-{themeColors().border}/10 hover:bg-{themeColors().bg}/10 transition-colors"
                    in:fly={{ y: -10, duration: 200 }}
                  >
                    <!-- Direction -->
                    <div class="col-span-1 text-center text-lg">
                      {#if conn.direction === 'incoming'}
                        <span class="text-{themeColors().accent}">↓</span>
                      {:else}
                        <span class="text-{themeColors().secondary}">↑</span>
                      {/if}
                    </div>

                    <!-- IP Address -->
                    <div
                      class="col-span-3 font-mono cursor-pointer hover:underline transition flex items-center gap-1 {isIpBlocked(conn.ip) ? 'text-red-500 line-through opacity-60' : `text-${themeColors().text} hover:text-${themeColors().primary}`}"
                      oncontextmenu={(e) => showIpContextMenu(e, conn.ip)}
                      onclick={() => openIpToolsModal(conn.ip)}
                      title={isIpBlocked(conn.ip) ? 'BLOCKED - Right-click to unblock' : 'Right-click for network tools'}
                    >
                      {#if isIpBlocked(conn.ip)}<span class="text-red-500">🚫</span>{/if}
                      {conn.ip}
                    </div>

                    <!-- Port -->
                    <div class="col-span-1 text-center font-mono opacity-80">
                      {conn.port}
                    </div>

                    <!-- Protocol -->
                    <div class="col-span-2">
                      <span class="px-2 py-0.5 bg-{themeColors().primary}/20 border border-{themeColors().primary}/40 rounded text-{themeColors().primary} text-[10px] font-bold">
                        {conn.protocol}
                      </span>
                    </div>

                    <!-- Data Transfer -->
                    <div class="col-span-2 text-[10px] opacity-70">
                      <div class="text-{themeColors().accent}">↓ {formatBytes(conn.bytesIn)}</div>
                      <div class="text-{themeColors().secondary}">↑ {formatBytes(conn.bytesOut)}</div>
                    </div>

                    <!-- Status -->
                    <div class="col-span-2">
                      {#if conn.status === 'active'}
                        <span class="px-2 py-1 bg-{themeColors().primary}/30 text-{themeColors().primary} rounded text-[10px] font-bold uppercase animate-pulse">
                          ● ACTIVE
                        </span>
                      {:else}
                        <span class="px-2 py-1 bg-gray-500/20 text-gray-500 rounded text-[10px] font-bold uppercase">
                          ○ CLOSED
                        </span>
                      {/if}
                    </div>

                    <!-- Country -->
                    <div class="col-span-1 text-center opacity-60 text-[10px]">
                      {conn.country || '??'}
                    </div>
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        </div>
        {:else if viewMode === 'radar'}
        <!-- Radar View - Physical Security Monitor -->
        <div class="w-full h-full flex items-center justify-center relative">
          <!-- Circular Radar Display -->
          <svg viewBox="0 0 400 400" class="w-full h-full max-w-md max-h-md">
            <!-- Radar circles (range rings) -->
            <circle cx="200" cy="200" r="180" fill="none" stroke="currentColor" stroke-width="0.5" opacity="0.2" />
            <circle cx="200" cy="200" r="120" fill="none" stroke="currentColor" stroke-width="0.5" opacity="0.2" />
            <circle cx="200" cy="200" r="60" fill="none" stroke="currentColor" stroke-width="0.5" opacity="0.2" />

            <!-- Crosshairs -->
            <line x1="200" y1="20" x2="200" y2="380" stroke="currentColor" stroke-width="0.5" opacity="0.1" />
            <line x1="20" y1="200" x2="380" y2="200" stroke="currentColor" stroke-width="0.5" opacity="0.1" />

            <!-- Center point removed for cleaner display -->

            <!-- WiFi Devices -->
            {#each wifiDevices as wifi, i}
              {@const angle = (i * (360 / Math.max(wifiDevices.length, 1))) * (Math.PI / 180)}
              {@const distance = Math.max(30, Math.min(170, 200 - (wifi.signal_strength + 100) * 1.5))}
              {@const x = 200 + distance * Math.cos(angle)}
              {@const y = 200 + distance * Math.sin(angle)}
              <g class="cursor-pointer hover:opacity-100 transition" opacity="0.8">
                <circle
                  cx={x}
                  cy={y}
                  r="8"
                  fill={wifi.trusted ? 'currentColor' : '#ff0000'}
                  class={wifi.trusted ? `text-${themeColors().accent}` : ''}
                  stroke="currentColor"
                  stroke-width="2"
                />
                <text x={x} y={y - 12} text-anchor="middle" class="text-[8px]" fill={wifi.trusted ? 'currentColor' : '#ff0000'}>
                  {wifi.ssid.substring(0, 10)}
                </text>
              </g>
            {/each}

            <!-- Bluetooth Devices -->
            {#each bluetoothDevices as bt, i}
              {@const angle = ((i + wifiDevices.length) * (360 / Math.max(bluetoothDevices.length + wifiDevices.length, 1))) * (Math.PI / 180)}
              {@const distance = Math.max(30, Math.min(170, 200 - (bt.rssi + 100) * 1.5))}
              {@const x = 200 + distance * Math.cos(angle)}
              {@const y = 200 + distance * Math.sin(angle)}
              <g class="cursor-pointer hover:opacity-100 transition" opacity="0.8">
                <rect
                  x={x - 6}
                  y={y - 6}
                  width="12"
                  height="12"
                  fill={bt.trusted ? 'currentColor' : '#ff0000'}
                  class={bt.trusted ? `text-${themeColors().secondary}` : ''}
                  stroke="currentColor"
                  stroke-width="2"
                />
                <text x={x} y={y - 12} text-anchor="middle" class="text-[8px]" fill={bt.trusted ? 'currentColor' : '#ff0000'}>
                  {bt.name.substring(0, 10)}
                </text>
              </g>
            {/each}
          </svg>

          <!-- Legend -->
          <div class="absolute bottom-4 left-4 text-xs space-y-1 bg-black/80 p-2 border border-{themeColors().border}/30 rounded">
            <div class="flex items-center gap-2">
              <div class="w-3 h-3 rounded-full bg-{themeColors().accent}"></div>
              <span>WiFi (Trusted)</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-3 h-3 bg-{themeColors().secondary}"></div>
              <span>Bluetooth (Trusted)</span>
            </div>
            <div class="flex items-center gap-2">
              <div class="w-3 h-3 rounded-full bg-red-500"></div>
              <span>Unknown Threat</span>
            </div>
          </div>

          <!-- Scanning Animation -->
          {#if radarScanning}
            <div class="absolute inset-0 flex items-center justify-center">
              <div class="text-{themeColors().primary} text-6xl animate-spin">📡</div>
            </div>
          {/if}

          <!-- No Data State -->
          {#if wifiDevices.length === 0 && bluetoothDevices.length === 0 && !radarScanning}
            <div class="absolute inset-0 flex items-center justify-center text-{themeColors().text}/50 text-sm">
              <div class="text-center">
                <div class="text-4xl mb-3">📡</div>
                <div>No devices detected</div>
                <div class="text-xs opacity-50 mt-2">Click SCAN DEVICES to begin</div>
              </div>
            </div>
          {/if}
        </div>
        {:else if viewMode === 'memory'}
        <!-- Memory Bank View -->
        <div class="w-full h-full flex flex-col gap-4 p-4 overflow-y-auto">
          <!-- Search Bar -->
          <div class="flex gap-2">
            <input
              type="text"
              bind:value={memorySearchQuery}
              onkeydown={(e) => e.key === 'Enter' && searchMemories(memorySearchQuery)}
              placeholder="Search memories..."
              class="flex-1 bg-black/30 border border-{themeColors().border}/30 px-4 py-2 rounded text-{themeColors().text} placeholder-{themeColors().text}/30 focus:border-{themeColors().primary} focus:outline-none"
            />
            <button
              onclick={() => searchMemories(memorySearchQuery)}
              class="px-4 py-2 bg-{themeColors().primary}/20 border border-{themeColors().primary} text-{themeColors().primary} hover:bg-{themeColors().primary} hover:text-black transition rounded font-bold text-sm"
            >
              🔍 SEARCH
            </button>
            <select
              bind:value={memoryAgentFilter}
              onchange={() => loadRecentMemories(50, memoryAgentFilter)}
              class="bg-black/30 border border-{themeColors().border}/30 px-4 py-2 rounded text-{themeColors().text} focus:border-{themeColors().primary} focus:outline-none"
            >
              <option value={null}>All Agents</option>
              <option value="Grok">Grok</option>
              <option value="DeepSeek">DeepSeek</option>
              <option value="Gemini">Gemini</option>
            </select>
          </div>

          <!-- Memory Stats Cards -->
          {#if memoryStats}
            <div class="grid grid-cols-3 gap-4">
              <div class="bg-{themeColors().bg}/10 border border-{themeColors().border}/30 p-4 rounded">
                <div class="text-2xl font-bold text-{themeColors().primary}">{memoryStats.total_memories}</div>
                <div class="text-xs opacity-60">TOTAL MEMORIES</div>
              </div>
              <div class="bg-{themeColors().bg}/10 border border-{themeColors().border}/30 p-4 rounded">
                <div class="text-2xl font-bold text-{themeColors().accent}">{memoryStats.total_tokens.toLocaleString()}</div>
                <div class="text-xs opacity-60">TOTAL TOKENS</div>
              </div>
              <div class="bg-{themeColors().bg}/10 border border-{themeColors().border}/30 p-4 rounded">
                <div class="text-2xl font-bold text-{themeColors().secondary}">${memoryStats.total_cost.toFixed(4)}</div>
                <div class="text-xs opacity-60">TOTAL COST</div>
              </div>
            </div>
          {/if}

          <!-- Memory List -->
          <div class="flex-1 space-y-2">
            {#if recentMemories.length === 0}
              <div class="text-center py-12 text-{themeColors().text}/50">
                <div class="text-4xl mb-3"></div>
                <div>No memories found</div>
                <div class="text-xs opacity-50 mt-2">Agent memories will appear here</div>
              </div>
            {:else}
              {#each recentMemories as memory (memory.id)}
                <div
                  class="border border-{themeColors().border}/30 bg-{themeColors().bg}/5 rounded p-4 hover:border-{themeColors().primary}/50 transition cursor-pointer"
                  onclick={() => selectedMemory = selectedMemory?.id === memory.id ? null : memory}
                  in:fly={{ y: -10, duration: 200 }}
                  style="box-shadow: 0 0 10px {selectedMemory?.id === memory.id ? themeColors().glow : 'transparent'}"
                >
                  <!-- Memory Header -->
                  <div class="flex items-start justify-between mb-2">
                    <div class="flex-1">
                      <div class="text-sm font-bold text-{themeColors().primary}">
                        {memory.agent_name} <span class="text-{themeColors().text}/50 text-xs">({memory.memory_type})</span>
                      </div>
                      {#if memory.model}
                        <div class="text-xs opacity-60">{memory.model}</div>
                      {/if}
                    </div>
                    <div class="text-xs opacity-60">
                      {new Date(memory.created_at).toLocaleDateString()} {new Date(memory.created_at).toLocaleTimeString()}
                    </div>
                  </div>

                  <!-- Memory Summary -->
                  {#if memory.summary}
                    <div class="text-sm text-{themeColors().text}/80 mb-2">
                      {memory.summary}
                    </div>
                  {/if}

                  <!-- Memory Stats -->
                  <div class="grid grid-cols-4 gap-2 text-xs">
                    <div class="bg-black/20 p-2 rounded text-center">
                      <div class="opacity-60">Tokens</div>
                      <div class="text-{themeColors().accent} font-bold">{memory.tokens_used.toLocaleString()}</div>
                    </div>
                    <div class="bg-black/20 p-2 rounded text-center">
                      <div class="opacity-60">Cost</div>
                      <div class="text-{themeColors().secondary} font-bold">${memory.cost_usd.toFixed(4)}</div>
                    </div>
                    <div class="bg-black/20 p-2 rounded text-center">
                      <div class="opacity-60">Accessed</div>
                      <div class="text-{themeColors().text} font-bold">{memory.access_count}x</div>
                    </div>
                    {#if memory.confidence_score}
                      <div class="bg-black/20 p-2 rounded text-center">
                        <div class="opacity-60">Confidence</div>
                        <div class="text-{themeColors().primary} font-bold">{(memory.confidence_score * 100).toFixed(0)}%</div>
                      </div>
                    {:else}
                      <div class="bg-black/20 p-2 rounded text-center">
                        <div class="opacity-60">Status</div>
                        <div class="text-{memory.verified ? 'green-500' : 'yellow-500'} font-bold">{memory.verified ? 'Verified' : 'Unverified'}</div>
                      </div>
                    {/if}
                  </div>

                  <!-- Expanded Details -->
                  {#if selectedMemory?.id === memory.id}
                    <div class="mt-4 pt-4 border-t border-{themeColors().border}/30" in:fly={{ y: -5, duration: 150 }}>
                      {#if memory.query}
                        <div class="mb-3">
                          <div class="text-xs opacity-60 mb-1">QUERY</div>
                          <div class="text-sm bg-black/30 p-2 rounded border-l-2 border-{themeColors().accent}">{memory.query}</div>
                        </div>
                      {/if}
                      <div>
                        <div class="text-xs opacity-60 mb-1">CONTENT</div>
                        <div class="text-xs bg-black/30 p-3 rounded max-h-48 overflow-y-auto font-mono">
                          {memory.content.substring(0, 500)}{memory.content.length > 500 ? '...' : ''}
                        </div>
                      </div>
                      {#if memory.flight_id}
                        <div class="mt-2 text-xs">
                          <span class="opacity-60">Flight ID:</span> <span class="text-{themeColors().primary}">{memory.flight_id}</span>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
        {:else if viewMode === 'defense'}
        <!-- Active Defense View -->
        <div class="w-full h-full flex flex-col gap-4 p-4 overflow-y-auto">
          <!-- Action Indicator -->
          {#if defenseAction}
            <div class="absolute top-12 left-4 right-4 bg-yellow-500/20 border border-yellow-500 text-yellow-500 px-4 py-2 rounded text-sm font-bold animate-pulse z-10">
              {defenseAction}...
            </div>
          {/if}

          <!-- System Metrics Grid -->
          <div class="grid grid-cols-4 gap-4">
            <!-- CPU Panel -->
            <div class="border border-cyan-500/30 bg-cyan-900/10 rounded p-4" style="box-shadow: 0 0 15px rgba(0,217,255,0.2)">
              <div class="flex items-center justify-between mb-3">
                <div class="text-lg font-bold text-cyan-400">CPU</div>
                <div class="text-2xl">🖥️</div>
              </div>
              {#if systemMetrics.cpu}
                <div class="space-y-2 text-xs">
                  <div class="flex justify-between">
                    <span class="opacity-60">Load:</span>
                    <span class="text-cyan-400 font-bold">{systemMetrics.cpu.load}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Frequency:</span>
                    <span class="text-cyan-300">{systemMetrics.cpu.frequency}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Governor:</span>
                    <span class="text-cyan-300">{systemMetrics.cpu.governor}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Turbo:</span>
                    <span class="text-{systemMetrics.cpu.turbo_enabled ? 'green-500' : 'gray-500'}">{systemMetrics.cpu.turbo_enabled ? 'ON' : 'OFF'}</span>
                  </div>
                  <div class="pt-2 grid grid-cols-2 gap-1">
                    <button onclick={() => setCpuGovernor('powersave')} class="px-2 py-1 text-[10px] bg-blue-500/20 border border-blue-500/50 text-blue-400 rounded hover:bg-blue-500 hover:text-black transition">SAVE</button>
                    <button onclick={() => setCpuGovernor('performance')} class="px-2 py-1 text-[10px] bg-red-500/20 border border-red-500/50 text-red-400 rounded hover:bg-red-500 hover:text-black transition">PERF</button>
                  </div>
                </div>
              {:else}
                <div class="text-center text-cyan-400/50 py-4">No data</div>
              {/if}
            </div>

            <!-- Memory Panel -->
            <div class="border border-purple-500/30 bg-purple-900/10 rounded p-4" style="box-shadow: 0 0 15px rgba(168,85,247,0.2)">
              <div class="flex items-center justify-between mb-3">
                <div class="text-lg font-bold text-purple-400">MEMORY</div>
                <div class="text-2xl">🧠</div>
              </div>
              {#if systemMetrics.memory}
                <div class="space-y-2 text-xs">
                  <div class="flex justify-between">
                    <span class="opacity-60">Used:</span>
                    <span class="text-purple-400 font-bold">{systemMetrics.memory.memory_percent?.toFixed(1)}%</span>
                  </div>
                  <div class="h-2 bg-black/30 rounded overflow-hidden">
                    <div class="h-full bg-purple-500 transition-all" style="width: {systemMetrics.memory.memory_percent}%"></div>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Available:</span>
                    <span class="text-purple-300">{(systemMetrics.memory.available_mb / 1024).toFixed(1)} GB</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Swap:</span>
                    <span class="text-purple-300">{systemMetrics.memory.swap_percent?.toFixed(1)}%</span>
                  </div>
                  <div class="pt-2 grid grid-cols-2 gap-1">
                    <button onclick={dropCaches} class="px-2 py-1 text-[10px] bg-purple-500/20 border border-purple-500/50 text-purple-400 rounded hover:bg-purple-500 hover:text-black transition">DROP</button>
                    <button onclick={compactMemory} class="px-2 py-1 text-[10px] bg-purple-500/20 border border-purple-500/50 text-purple-400 rounded hover:bg-purple-500 hover:text-black transition">COMPACT</button>
                  </div>
                </div>
              {:else}
                <div class="text-center text-purple-400/50 py-4">No data</div>
              {/if}
            </div>

            <!-- Thermal Panel -->
            <div class="border border-orange-500/30 bg-orange-900/10 rounded p-4" style="box-shadow: 0 0 15px rgba(249,115,22,0.2)">
              <div class="flex items-center justify-between mb-3">
                <div class="text-lg font-bold text-orange-400">THERMAL</div>
                <div class="text-2xl">🌡️</div>
              </div>
              {#if systemMetrics.thermal}
                <div class="space-y-2 text-xs">
                  <div class="flex justify-between">
                    <span class="opacity-60">Hottest:</span>
                    <span class="text-{systemMetrics.thermal.alert_level === 'critical' ? 'red-500' : systemMetrics.thermal.alert_level === 'warning' ? 'orange-400' : 'green-500'} font-bold">{systemMetrics.thermal.hottest}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Sensors:</span>
                    <span class="text-orange-300 text-[10px]">{systemMetrics.thermal.sensors}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Alert:</span>
                    <span class="text-{systemMetrics.thermal.alert_level === 'critical' ? 'red-500' : systemMetrics.thermal.alert_level === 'warning' ? 'yellow-500' : 'green-500'}">{systemMetrics.thermal.alert_level?.toUpperCase() || 'OK'}</span>
                  </div>
                  <div class="pt-2">
                    <button onclick={emergencyCool} class="w-full px-2 py-1 text-[10px] bg-blue-500/20 border border-blue-500/50 text-blue-400 rounded hover:bg-blue-500 hover:text-black transition">EMERGENCY COOL</button>
                  </div>
                </div>
              {:else}
                <div class="text-center text-orange-400/50 py-4">No data</div>
              {/if}
            </div>

            <!-- GPU Panel -->
            <div class="border border-green-500/30 bg-green-900/10 rounded p-4" style="box-shadow: 0 0 15px rgba(34,197,94,0.2)">
              <div class="flex items-center justify-between mb-3">
                <div class="text-lg font-bold text-green-400">GPU</div>
                <div class="text-2xl">🎮</div>
              </div>
              {#if systemMetrics.gpu}
                <div class="space-y-2 text-xs">
                  <div class="flex justify-between">
                    <span class="opacity-60">GPUs:</span>
                    <span class="text-green-400 font-bold">{systemMetrics.gpu.gpus}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Utilization:</span>
                    <span class="text-green-300">{systemMetrics.gpu.utilization}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">VRAM:</span>
                    <span class="text-green-300">{systemMetrics.gpu.vram}</span>
                  </div>
                  <div class="flex justify-between">
                    <span class="opacity-60">Thermals:</span>
                    <span class="text-green-300">{systemMetrics.gpu.thermals}</span>
                  </div>
                  <div class="pt-2">
                    <button onclick={gpuEmergencyThrottle} class="w-full px-2 py-1 text-[10px] bg-red-500/20 border border-red-500/50 text-red-400 rounded hover:bg-red-500 hover:text-black transition">THROTTLE</button>
                  </div>
                </div>
              {:else}
                <div class="text-center text-green-400/50 py-4">No data</div>
              {/if}
            </div>
          </div>

          <!-- Process Controls Section -->
          <div class="grid grid-cols-2 gap-4">
            <!-- Top CPU Processes -->
            <div class="border border-cyan-500/30 bg-black/30 rounded p-4">
              <div class="text-sm font-bold text-cyan-400 mb-3">TOP CPU PROCESSES</div>
              {#if topProcessesCpu.length > 0}
                <div class="space-y-2">
                  {#each topProcessesCpu as proc}
                    <div class="flex items-center justify-between text-xs bg-black/20 p-2 rounded group">
                      <div class="flex-1">
                        <span class="text-cyan-300 font-mono">{proc.name}</span>
                        <span class="text-gray-500 ml-2">PID:{proc.pid}</span>
                      </div>
                      <div class="text-cyan-400 font-bold mr-3">{proc.cpu_percent?.toFixed(1) || '0'}%</div>
                      <div class="opacity-0 group-hover:opacity-100 transition flex gap-1">
                        <button onclick={() => freezeProcess(proc.pid)} class="px-2 py-0.5 bg-blue-500/20 text-blue-400 rounded text-[10px] hover:bg-blue-500 hover:text-black">❄️</button>
                        <button onclick={() => killProcess(proc.pid)} class="px-2 py-0.5 bg-red-500/20 text-red-400 rounded text-[10px] hover:bg-red-500 hover:text-black">💀</button>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-center text-gray-500 py-4 text-xs">No process data</div>
              {/if}
            </div>

            <!-- Top Memory Processes -->
            <div class="border border-purple-500/30 bg-black/30 rounded p-4">
              <div class="text-sm font-bold text-purple-400 mb-3">TOP MEMORY PROCESSES</div>
              {#if topProcessesMem.length > 0}
                <div class="space-y-2">
                  {#each topProcessesMem as proc}
                    <div class="flex items-center justify-between text-xs bg-black/20 p-2 rounded group">
                      <div class="flex-1">
                        <span class="text-purple-300 font-mono">{proc.name}</span>
                        <span class="text-gray-500 ml-2">PID:{proc.pid}</span>
                      </div>
                      <div class="text-purple-400 font-bold mr-3">{proc.memory_mb?.toFixed(0) || '0'} MB</div>
                      <div class="opacity-0 group-hover:opacity-100 transition flex gap-1">
                        <button onclick={() => freezeProcess(proc.pid)} class="px-2 py-0.5 bg-blue-500/20 text-blue-400 rounded text-[10px] hover:bg-blue-500 hover:text-black">❄️</button>
                        <button onclick={() => killProcess(proc.pid)} class="px-2 py-0.5 bg-red-500/20 text-red-400 rounded text-[10px] hover:bg-red-500 hover:text-black">💀</button>
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <div class="text-center text-gray-500 py-4 text-xs">No process data</div>
              {/if}
            </div>
          </div>

          <!-- Panic Buttons Row -->
          <div class="grid grid-cols-4 gap-4">
            <button onclick={performanceMode} disabled={!!defenseAction} class="p-4 bg-green-500/10 border-2 border-green-500/50 rounded text-green-500 hover:bg-green-500 hover:text-black transition disabled:opacity-50">
              <div class="text-2xl mb-2">🚀</div>
              <div class="text-xs font-bold">PERFORMANCE</div>
            </button>
            <button onclick={emergencyCool} disabled={!!defenseAction} class="p-4 bg-blue-500/10 border-2 border-blue-500/50 rounded text-blue-500 hover:bg-blue-500 hover:text-black transition disabled:opacity-50">
              <div class="text-2xl mb-2">❄️</div>
              <div class="text-xs font-bold">COOL DOWN</div>
            </button>
            <button onclick={lockdownNetwork} disabled={!!defenseAction} class="p-4 bg-yellow-500/10 border-2 border-yellow-500/50 rounded text-yellow-500 hover:bg-yellow-500 hover:text-black transition disabled:opacity-50">
              <div class="text-2xl mb-2">🔒</div>
              <div class="text-xs font-bold">NET LOCKDOWN</div>
            </button>
            <button onclick={resetAllControls} disabled={!!defenseAction} class="p-4 bg-gray-500/10 border-2 border-gray-500/50 rounded text-gray-400 hover:bg-gray-500 hover:text-black transition disabled:opacity-50">
              <div class="text-2xl mb-2">🔄</div>
              <div class="text-xs font-bold">RESET ALL</div>
            </button>
          </div>

          <!-- Zombie Reaper -->
          <div class="flex items-center justify-between p-3 bg-gray-900/50 border border-gray-700/50 rounded">
            <div class="flex items-center gap-3">
              <div class="text-xl">🧟</div>
              <div>
                <div class="text-sm font-bold text-gray-300">Zombie Process Reaper</div>
                <div class="text-xs text-gray-500">Clean up orphaned and zombie processes</div>
              </div>
            </div>
            <button onclick={reapZombies} disabled={!!defenseAction} class="px-4 py-2 bg-gray-700/50 border border-gray-600 text-gray-300 rounded text-sm hover:bg-gray-600 hover:text-white transition disabled:opacity-50">
              REAP ZOMBIES
            </button>
          </div>
        </div>
        {/if}
    </div>

    <!-- Right Column: The Terminal (The Voice) -->
    <div class="col-span-3 flex flex-col border border-{themeColors().border}/30 bg-black rounded overflow-hidden" style="box-shadow: 0 0 20px {themeColors().glow}">
      <div class="bg-{themeColors().bg}/20 p-2 border-b border-{themeColors().border}/30 text-xs flex items-center justify-between">
        <span>>_ TERMINAL // {viewMode === 'agents' ? 'AGENT LOGS' : viewMode === 'network' ? 'NETWORK LOGS' : viewMode === 'radar' ? 'RADAR LOGS' : viewMode === 'defense' ? 'DEFENSE LOGS' : 'MEMORY LOGS'}</span>
        <span class="text-[10px] opacity-50">{filteredLogs().length}/{terminalLines.length} ENTRIES</span>
      </div>
      <div
        bind:this={terminalContainer}
        class="flex-1 overflow-y-auto p-2 font-mono text-[11px] space-y-0.5 scrollbar-thin"
        style="--scrollbar-thumb-color: {themeColors().bg}"
      >
        {#if filteredLogs().length === 0}
          <div class="text-{themeColors().text}/50 text-center py-8">
            {viewMode === 'agents' ? 'Waiting for AI agent activity...' : viewMode === 'network' ? 'Waiting for network activity...' : viewMode === 'radar' ? 'Waiting for radar scan...' : viewMode === 'defense' ? 'Click REFRESH to load system metrics...' : 'Waiting for memory operations...'}
          </div>
        {:else}
          {#each filteredLogs() as line}
            <div class="hover:bg-{themeColors().bg}/10 px-1 py-0.5 transition-colors" style="word-break: break-all">
              <span class="text-{themeColors().text} mr-2">></span>
              <span class:text-yellow-400={line.includes('🤖')}
                    class:text-{themeColors().text}={line.includes('✅')}
                    class:text-{themeColors().secondary}={line.includes('❌')}
                    class:text-{themeColors().accent}={line.includes('TEST')}
              >
                {line}
              </span>
            </div>
          {/each}
        {/if}
      </div>
      <div class="p-2 border-t border-{themeColors().border}/30 text-[10px] opacity-50 text-center">
        MONITORING MODE // READ-ONLY
      </div>
    </div>

  </div>
  <!-- IP Context Menu -->
  {#if ipContextMenu.visible}
    <div
      class="fixed inset-0 z-50"
      onclick={hideIpContextMenu}
      oncontextmenu={(e) => { e.preventDefault(); hideIpContextMenu(); }}
    >
      <div
        class="absolute bg-gray-900 border border-{themeColors().border} rounded shadow-lg py-1 min-w-48"
        style="left: {ipContextMenu.x}px; top: {ipContextMenu.y}px; box-shadow: 0 0 20px {themeColors().glow}"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="px-3 py-2 border-b border-{themeColors().border}/30 text-xs text-{themeColors().text} font-bold">
          🌐 {ipContextMenu.ip}
        </div>
        <button
          onclick={() => { openIpToolsModal(ipContextMenu.ip); runIpTool('whois'); }}
          class="w-full px-3 py-2 text-left text-sm text-{themeColors().text} hover:bg-{themeColors().bg}/30 flex items-center gap-2"
        >
          <span>📋</span> WHOIS Lookup
        </button>
        <button
          onclick={() => { openIpToolsModal(ipContextMenu.ip); runIpTool('nslookup'); }}
          class="w-full px-3 py-2 text-left text-sm text-{themeColors().text} hover:bg-{themeColors().bg}/30 flex items-center gap-2"
        >
          <span>🔍</span> DNS Lookup
        </button>
        <button
          onclick={() => { openIpToolsModal(ipContextMenu.ip); runIpTool('ping'); }}
          class="w-full px-3 py-2 text-left text-sm text-{themeColors().text} hover:bg-{themeColors().bg}/30 flex items-center gap-2"
        >
          <span>📡</span> Ping
        </button>
        <button
          onclick={() => { openIpToolsModal(ipContextMenu.ip); runIpTool('traceroute'); }}
          class="w-full px-3 py-2 text-left text-sm text-{themeColors().text} hover:bg-{themeColors().bg}/30 flex items-center gap-2"
        >
          <span>🛤️</span> Traceroute
        </button>
        <button
          onclick={() => { openIpToolsModal(ipContextMenu.ip); runIpTool('geoip'); }}
          class="w-full px-3 py-2 text-left text-sm text-{themeColors().text} hover:bg-{themeColors().bg}/30 flex items-center gap-2"
        >
          <span>🌍</span> GeoIP Lookup
        </button>
        <div class="border-t border-{themeColors().border}/30 mt-1 pt-1">
          <button
            onclick={() => openIpToolsModal(ipContextMenu.ip)}
            class="w-full px-3 py-2 text-left text-sm text-{themeColors().primary} hover:bg-{themeColors().bg}/30 flex items-center gap-2"
          >
            <span>🧰</span> Open Tools Panel
          </button>
        </div>
        <!-- INTERDICTION SECTION -->
        <div class="border-t border-red-500/30 mt-1 pt-1">
          {#if isIpBlocked(ipContextMenu.ip)}
            <button
              onclick={() => unblockIp(ipContextMenu.ip)}
              disabled={!!blockingInProgress}
              class="w-full px-3 py-2 text-left text-sm text-green-400 hover:bg-green-900/30 flex items-center gap-2 disabled:opacity-50"
            >
              <span>✅</span> Unblock Traffic
            </button>
          {:else}
            <button
              onclick={() => blockIp(ipContextMenu.ip)}
              disabled={!!blockingInProgress}
              class="w-full px-3 py-2 text-left text-sm text-red-400 hover:bg-red-900/30 flex items-center gap-2 disabled:opacity-50"
            >
              <span>🚫</span> Block Traffic
            </button>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- IP Tools Modal -->
  {#if ipToolsModal.visible}
    <div
      class="fixed inset-0 bg-black/80 flex items-center justify-center z-50 p-4"
      onclick={closeIpToolsModal}
    >
      <div
        class="bg-gray-900 border border-{themeColors().border} rounded-lg shadow-2xl w-full max-w-3xl max-h-[80vh] overflow-hidden flex flex-col"
        style="box-shadow: 0 0 30px {themeColors().glow}"
        onclick={(e) => e.stopPropagation()}
      >
        <!-- Header -->
        <div class="px-4 py-3 border-b border-{themeColors().border}/30 flex items-center justify-between bg-{themeColors().bg}/20">
          <div class="flex items-center gap-3">
            <span class="text-2xl">🌐</span>
            <div>
              <h3 class="text-lg font-bold text-{themeColors().primary}">Network Tools</h3>
              <p class="text-xs text-{themeColors().text}/60 font-mono">{ipToolsModal.ip}</p>
            </div>
          </div>
          <button
            onclick={closeIpToolsModal}
            class="p-2 hover:bg-{themeColors().bg}/30 rounded transition text-{themeColors().text}"
          >
            ✕
          </button>
        </div>

        <!-- Tool Buttons -->
        <div class="px-4 py-3 border-b border-{themeColors().border}/30 flex flex-wrap gap-2 items-center">
          <button
            onclick={() => runIpTool('whois')}
            class="px-3 py-1.5 text-xs font-bold bg-{themeColors().primary}/20 border border-{themeColors().primary}/50 text-{themeColors().primary} rounded hover:bg-{themeColors().primary} hover:text-black transition flex items-center gap-1"
          >
            📋 WHOIS
          </button>
          <button
            onclick={() => runIpTool('nslookup')}
            class="px-3 py-1.5 text-xs font-bold bg-{themeColors().accent}/20 border border-{themeColors().accent}/50 text-{themeColors().accent} rounded hover:bg-{themeColors().accent} hover:text-black transition flex items-center gap-1"
          >
            🔍 DNS
          </button>
          <button
            onclick={() => runIpTool('ping')}
            class="px-3 py-1.5 text-xs font-bold bg-{themeColors().secondary}/20 border border-{themeColors().secondary}/50 text-{themeColors().secondary} rounded hover:bg-{themeColors().secondary} hover:text-black transition flex items-center gap-1"
          >
            📡 PING
          </button>
          <button
            onclick={() => runIpTool('traceroute')}
            class="px-3 py-1.5 text-xs font-bold bg-purple-500/20 border border-purple-500/50 text-purple-400 rounded hover:bg-purple-500 hover:text-black transition flex items-center gap-1"
          >
            🛤️ TRACEROUTE
          </button>
          <button
            onclick={() => runIpTool('geoip')}
            class="px-3 py-1.5 text-xs font-bold bg-orange-500/20 border border-orange-500/50 text-orange-400 rounded hover:bg-orange-500 hover:text-black transition flex items-center gap-1"
          >
            🌍 GEOIP
          </button>

          <!-- Separator -->
          <div class="w-px h-6 bg-{themeColors().border}/30 mx-2"></div>

          <!-- INTERDICTION BUTTON -->
          {#if isIpBlocked(ipToolsModal.ip)}
            <button
              onclick={() => unblockIp(ipToolsModal.ip)}
              disabled={!!blockingInProgress}
              class="px-3 py-1.5 text-xs font-bold bg-green-500/20 border-2 border-green-500 text-green-400 rounded hover:bg-green-500 hover:text-black transition flex items-center gap-1 disabled:opacity-50"
            >
              ✅ UNBLOCK
            </button>
          {:else}
            <button
              onclick={() => blockIp(ipToolsModal.ip)}
              disabled={!!blockingInProgress}
              class="px-3 py-1.5 text-xs font-bold bg-red-500/20 border-2 border-red-500 text-red-400 rounded hover:bg-red-500 hover:text-black transition flex items-center gap-1 animate-pulse disabled:opacity-50"
            >
              🚫 BLOCK
            </button>
          {/if}
        </div>

        <!-- Results -->
        <div class="flex-1 overflow-y-auto p-4 space-y-4">
          {#if ipToolsModal.results.length === 0}
            <div class="text-center py-12 text-{themeColors().text}/50">
              <div class="text-4xl mb-3">🧰</div>
              <div>Select a tool above to query this IP address</div>
              <div class="text-xs opacity-50 mt-2">Results will appear here</div>
            </div>
          {:else}
            {#each ipToolsModal.results as result (result.tool + result.ip)}
              <div class="border border-{themeColors().border}/30 rounded bg-black/30">
                <div class="px-3 py-2 border-b border-{themeColors().border}/20 flex items-center justify-between bg-{themeColors().bg}/10">
                  <span class="text-sm font-bold text-{themeColors().primary} uppercase">{result.tool}</span>
                  {#if result.loading}
                    <span class="text-xs text-{themeColors().secondary} animate-pulse">Running...</span>
                  {:else if result.error}
                    <span class="text-xs text-red-500">Error</span>
                  {:else}
                    <span class="text-xs text-green-500">Complete</span>
                  {/if}
                </div>
                <div class="p-3">
                  {#if result.loading}
                    <div class="flex items-center gap-2 text-{themeColors().text}/60">
                      <div class="animate-spin">⚙️</div>
                      <span class="text-sm">Executing {result.tool}...</span>
                    </div>
                  {:else if result.error}
                    <div class="text-red-400 text-sm">{result.error}</div>
                  {:else}
                    <pre class="text-xs text-{themeColors().text} font-mono whitespace-pre-wrap overflow-x-auto max-h-64">{result.result}</pre>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>

        <!-- Footer -->
        <div class="px-4 py-3 border-t border-{themeColors().border}/30 flex justify-end bg-{themeColors().bg}/10">
          <button
            onclick={closeIpToolsModal}
            class="px-4 py-2 text-sm bg-gray-700 hover:bg-gray-600 text-gray-200 rounded transition"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  {/if}

{:else}
  <!-- Fallback if developer mode is not enabled -->
  <div class="flex flex-col items-center justify-center h-full bg-black text-red-500 font-mono">
    <div class="text-6xl mb-4">🔒</div>
    <div class="text-2xl font-bold mb-2">ACCESS DENIED</div>
    <div class="text-sm opacity-60">INSUFFICIENT SECURITY CLEARANCE</div>
    <div class="text-xs mt-4 opacity-40">Enable Developer Mode in Settings to access Command Centre</div>
  </div>
{/if}

<style>
    /* Custom scrollbar for the terminal */
    .scrollbar-thin::-webkit-scrollbar {
        width: 6px;
    }
    .scrollbar-thin::-webkit-scrollbar-track {
        background: #000;
    }
    .scrollbar-thin::-webkit-scrollbar-thumb {
        background: rgba(var(--scrollbar-thumb-color), 0.5);
        border-radius: 3px;
    }

    /* Default scrollbar colors for each theme */
    [data-theme="cyberpunk"] .scrollbar-thin::-webkit-scrollbar-thumb {
        background: rgba(0, 217, 255, 0.3);
    }
    [data-theme="skynet"] .scrollbar-thin::-webkit-scrollbar-thumb {
        background: rgba(0, 128, 255, 0.3);
    }
    [data-theme="light"],
    [data-theme="dark"] {
        .scrollbar-thin::-webkit-scrollbar-thumb {
            background: rgba(0, 255, 0, 0.3);
        }
    }

    /* Pulse glow animation for agent nodes */
    @keyframes pulse-glow {
        0%, 100% {
            opacity: 0.6;
            transform: scale(1);
        }
        50% {
            opacity: 1;
            transform: scale(1.05);
        }
    }

    /* Progress bar animation for active agents */
    @keyframes pulse-width {
        0%, 100% {
            opacity: 0.6;
        }
        50% {
            opacity: 1;
        }
    }
</style>
