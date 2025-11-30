<!-- AirportInfoSidebar.svelte - Slide-out panel for airport details -->
<script lang="ts">
  import { slide } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core';
  import { theme } from '$lib/theme';

  interface AirportData {
    code: string;
    name?: string;
    lat: number;
    lng: number;
    count: number; // Flight count from globe
  }

  interface AirportStats {
    totalDepartures: number;
    totalArrivals: number;
    uniqueRoutes: number;
    topDestinations: { code: string; count: number }[];
    topOrigins: { code: string; count: number }[];
    firstFlight?: string;
    lastFlight?: string;
    avgFlightsPerMonth: number;
  }

  interface Props {
    airport: AirportData | null;
    userId?: string;
    onClose: () => void;
  }

  let { airport, userId = '', onClose }: Props = $props();

  let loading = $state(false);
  let stats = $state<AirportStats | null>(null);
  let airportDetails = $state<any>(null);
  let error = $state<string | null>(null);

  // Load airport stats when airport changes
  $effect(() => {
    if (airport) {
      loadAirportData();
    } else {
      stats = null;
      airportDetails = null;
    }
  });

  async function loadAirportData() {
    if (!airport) return;

    loading = true;
    error = null;

    try {
      // Load airport details from database (if exists)
      const airports = await invoke<any[]>('list_all_airports');
      airportDetails = airports.find(
        a => a.iata_code === airport.code || a.icao_code === airport.code
      );

      // Load flight statistics for this airport
      const flights = await invoke<any[]>('list_flights', { userId, limit: 10000, offset: 0 });

      // Calculate stats
      const departures = flights.filter(f => f.departure_airport === airport.code);
      const arrivals = flights.filter(f => f.arrival_airport === airport.code);

      // Top destinations (from departures)
      const destCounts = new Map<string, number>();
      departures.forEach(f => {
        if (f.arrival_airport) {
          destCounts.set(f.arrival_airport, (destCounts.get(f.arrival_airport) || 0) + 1);
        }
      });
      const topDestinations = Array.from(destCounts.entries())
        .map(([code, count]) => ({ code, count }))
        .sort((a, b) => b.count - a.count)
        .slice(0, 5);

      // Top origins (from arrivals)
      const originCounts = new Map<string, number>();
      arrivals.forEach(f => {
        if (f.departure_airport) {
          originCounts.set(f.departure_airport, (originCounts.get(f.departure_airport) || 0) + 1);
        }
      });
      const topOrigins = Array.from(originCounts.entries())
        .map(([code, count]) => ({ code, count }))
        .sort((a, b) => b.count - a.count)
        .slice(0, 5);

      // Unique routes
      const routes = new Set<string>();
      departures.forEach(f => {
        if (f.arrival_airport) routes.add(`${airport.code}-${f.arrival_airport}`);
      });
      arrivals.forEach(f => {
        if (f.departure_airport) routes.add(`${f.departure_airport}-${airport.code}`);
      });

      // Date range
      const allFlights = [...departures, ...arrivals].sort((a, b) =>
        new Date(a.departure_datetime).getTime() - new Date(b.departure_datetime).getTime()
      );

      const firstFlight = allFlights[0]?.departure_datetime;
      const lastFlight = allFlights[allFlights.length - 1]?.departure_datetime;

      // Average flights per month
      let avgFlightsPerMonth = 0;
      if (firstFlight && lastFlight) {
        const months = Math.max(1,
          (new Date(lastFlight).getTime() - new Date(firstFlight).getTime()) / (1000 * 60 * 60 * 24 * 30)
        );
        avgFlightsPerMonth = Math.round((departures.length + arrivals.length) / months * 10) / 10;
      }

      stats = {
        totalDepartures: departures.length,
        totalArrivals: arrivals.length,
        uniqueRoutes: routes.size,
        topDestinations,
        topOrigins,
        firstFlight,
        lastFlight,
        avgFlightsPerMonth,
      };
    } catch (err) {
      console.error('Failed to load airport data:', err);
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function formatDate(dateStr?: string): string {
    if (!dateStr) return 'N/A';
    try {
      return new Date(dateStr).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
      });
    } catch {
      return 'N/A';
    }
  }

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
</script>

<svelte:window onkeydown={handleKeydown} />

{#if airport}
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
    class="airport-sidebar fixed top-0 right-0 h-full w-96 z-50 flex flex-col shadow-2xl {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}"
    transition:slide={{ axis: 'x', duration: 250 }}
  >
    <!-- Header -->
    <div class="sidebar-header h-14 px-4 flex items-center justify-between shrink-0">
      <div class="flex items-center gap-3">
        <div class="sidebar-icon w-8 h-8 rounded-lg flex items-center justify-center">
          <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
            <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
          </svg>
        </div>
        <div>
          <h2 class="sidebar-title font-bold text-lg">{airport.code}</h2>
          <p class="sidebar-subtitle text-xs opacity-70">
            {airportDetails?.name || airport.name || 'Airport Information'}
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
      {#if loading}
        <div class="flex items-center justify-center py-12">
          <div class="sidebar-spinner w-8 h-8"></div>
        </div>
      {:else if error}
        <div class="sidebar-error p-4 rounded-lg">
          <p class="text-sm">Failed to load data: {error}</p>
        </div>
      {:else}
        <!-- Location Info -->
        <div class="sidebar-section p-4 rounded-lg">
          <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Location</h3>
          <div class="grid grid-cols-2 gap-3 text-sm">
            <div>
              <p class="sidebar-label text-xs opacity-60">Latitude</p>
              <p class="sidebar-value font-mono">{airport.lat.toFixed(4)}</p>
            </div>
            <div>
              <p class="sidebar-label text-xs opacity-60">Longitude</p>
              <p class="sidebar-value font-mono">{airport.lng.toFixed(4)}</p>
            </div>
            {#if airportDetails?.city}
              <div>
                <p class="sidebar-label text-xs opacity-60">City</p>
                <p class="sidebar-value">{airportDetails.city}</p>
              </div>
            {/if}
            {#if airportDetails?.country}
              <div>
                <p class="sidebar-label text-xs opacity-60">Country</p>
                <p class="sidebar-value">{airportDetails.country}</p>
              </div>
            {/if}
            {#if airportDetails?.timezone}
              <div class="col-span-2">
                <p class="sidebar-label text-xs opacity-60">Timezone</p>
                <p class="sidebar-value">{airportDetails.timezone}</p>
              </div>
            {/if}
          </div>
        </div>

        <!-- Flight Statistics -->
        {#if stats}
          <div class="sidebar-section p-4 rounded-lg">
            <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Flight Statistics</h3>
            <div class="grid grid-cols-2 gap-3">
              <div class="sidebar-stat p-3 rounded-lg text-center">
                <p class="sidebar-stat-value text-2xl font-bold">{stats.totalDepartures}</p>
                <p class="sidebar-stat-label text-xs opacity-60">Departures</p>
              </div>
              <div class="sidebar-stat p-3 rounded-lg text-center">
                <p class="sidebar-stat-value text-2xl font-bold">{stats.totalArrivals}</p>
                <p class="sidebar-stat-label text-xs opacity-60">Arrivals</p>
              </div>
              <div class="sidebar-stat p-3 rounded-lg text-center">
                <p class="sidebar-stat-value text-2xl font-bold">{stats.uniqueRoutes}</p>
                <p class="sidebar-stat-label text-xs opacity-60">Unique Routes</p>
              </div>
              <div class="sidebar-stat p-3 rounded-lg text-center">
                <p class="sidebar-stat-value text-2xl font-bold">{stats.avgFlightsPerMonth}</p>
                <p class="sidebar-stat-label text-xs opacity-60">Avg/Month</p>
              </div>
            </div>
          </div>

          <!-- Date Range -->
          <div class="sidebar-section p-4 rounded-lg">
            <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Activity Period</h3>
            <div class="flex items-center justify-between text-sm">
              <div>
                <p class="sidebar-label text-xs opacity-60">First Flight</p>
                <p class="sidebar-value">{formatDate(stats.firstFlight)}</p>
              </div>
              <div class="sidebar-arrow px-4">→</div>
              <div class="text-right">
                <p class="sidebar-label text-xs opacity-60">Last Flight</p>
                <p class="sidebar-value">{formatDate(stats.lastFlight)}</p>
              </div>
            </div>
          </div>

          <!-- Top Destinations -->
          {#if stats.topDestinations.length > 0}
            <div class="sidebar-section p-4 rounded-lg">
              <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Top Destinations</h3>
              <div class="space-y-2">
                {#each stats.topDestinations as dest, i}
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <span class="sidebar-rank w-5 h-5 rounded text-xs flex items-center justify-center font-bold">
                        {i + 1}
                      </span>
                      <span class="sidebar-route-code font-mono font-bold">{dest.code}</span>
                    </div>
                    <span class="sidebar-route-count text-sm">{dest.count} flights</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Top Origins -->
          {#if stats.topOrigins.length > 0}
            <div class="sidebar-section p-4 rounded-lg">
              <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Top Origins</h3>
              <div class="space-y-2">
                {#each stats.topOrigins as origin, i}
                  <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <span class="sidebar-rank w-5 h-5 rounded text-xs flex items-center justify-center font-bold">
                        {i + 1}
                      </span>
                      <span class="sidebar-route-code font-mono font-bold">{origin.code}</span>
                    </div>
                    <span class="sidebar-route-count text-sm">{origin.count} flights</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        {/if}

        <!-- Database Info -->
        {#if airportDetails}
          <div class="sidebar-section p-4 rounded-lg">
            <h3 class="sidebar-section-title text-xs font-bold uppercase tracking-wider mb-3">Database Record</h3>
            <div class="text-xs space-y-1 opacity-70">
              {#if airportDetails.icao_code}
                <p>ICAO: <span class="font-mono">{airportDetails.icao_code}</span></p>
              {/if}
              {#if airportDetails.iata_code}
                <p>IATA: <span class="font-mono">{airportDetails.iata_code}</span></p>
              {/if}
              <p>ID: <span class="font-mono text-[10px]">{airportDetails.id}</span></p>
            </div>
          </div>
        {:else}
          <div class="sidebar-section sidebar-section-dim p-4 rounded-lg">
            <p class="text-xs opacity-50 text-center">
              Airport not in database.<br/>
              Using hardcoded coordinates.
            </p>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Footer -->
    <div class="sidebar-footer h-12 px-4 flex items-center justify-between shrink-0 text-xs opacity-50">
      <span>Total: {airport.count} flights</span>
      <span>Press ESC to close</span>
    </div>
  </div>
{/if}

<style>
  /* Theme: Default */
  .theme-default.airport-sidebar {
    background: linear-gradient(180deg, #0f172a 0%, #1e293b 100%);
    border-left: 1px solid rgba(148, 163, 184, 0.2);
  }

  .theme-default .sidebar-header {
    background: rgba(30, 41, 59, 0.8);
    border-bottom: 1px solid rgba(148, 163, 184, 0.2);
  }

  .theme-default .sidebar-icon {
    background: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
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
    color: #60a5fa;
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
    background: rgba(59, 130, 246, 0.1);
  }

  .theme-default .sidebar-arrow {
    color: #60a5fa;
  }

  .theme-default .sidebar-rank {
    background: rgba(59, 130, 246, 0.2);
    color: #60a5fa;
  }

  .theme-default .sidebar-route-code {
    color: #f1f5f9;
  }

  .theme-default .sidebar-route-count {
    color: #94a3b8;
  }

  .theme-default .sidebar-footer {
    background: rgba(15, 23, 42, 0.8);
    border-top: 1px solid rgba(148, 163, 184, 0.2);
    color: #64748b;
  }

  .theme-default .sidebar-spinner {
    border: 3px solid rgba(59, 130, 246, 0.2);
    border-top-color: #60a5fa;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  .theme-default .sidebar-error {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: #fca5a5;
  }

  /* Theme: Skynet */
  .theme-skynet.airport-sidebar {
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

  .theme-skynet .sidebar-arrow {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .sidebar-rank {
    background: rgba(0, 180, 255, 0.2);
    color: #00b4ff;
    border: 1px solid rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .sidebar-route-code {
    color: #00b4ff;
  }

  .theme-skynet .sidebar-route-count {
    color: rgba(0, 180, 255, 0.6);
  }

  .theme-skynet .sidebar-footer {
    background: rgba(0, 10, 20, 0.9);
    border-top: 1px solid rgba(0, 180, 255, 0.2);
    color: rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .sidebar-spinner {
    border: 3px solid rgba(0, 180, 255, 0.2);
    border-top-color: #00b4ff;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .sidebar-error {
    background: rgba(255, 0, 80, 0.1);
    border: 1px solid rgba(255, 0, 80, 0.3);
    color: #ff5080;
  }

  /* Theme: Cyberpunk */
  .theme-cyberpunk.airport-sidebar {
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
    color: #ff0080;
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .sidebar-title {
    color: #ff0080;
    text-shadow: 0 0 10px rgba(255, 0, 128, 0.5);
  }

  .theme-cyberpunk .sidebar-subtitle {
    color: rgba(0, 217, 255, 0.7);
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

  .theme-cyberpunk .sidebar-arrow {
    color: #00d9ff;
    text-shadow: 0 0 10px rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .sidebar-rank {
    background: linear-gradient(135deg, rgba(255, 0, 128, 0.2), rgba(0, 217, 255, 0.2));
    color: #ff0080;
    border: 1px solid rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .sidebar-route-code {
    color: #ff0080;
  }

  .theme-cyberpunk .sidebar-route-count {
    color: rgba(0, 217, 255, 0.7);
  }

  .theme-cyberpunk .sidebar-footer {
    background: rgba(10, 0, 20, 0.9);
    border-top: 1px solid rgba(255, 0, 128, 0.2);
    color: rgba(0, 217, 255, 0.4);
  }

  .theme-cyberpunk .sidebar-spinner {
    border: 3px solid rgba(255, 0, 128, 0.2);
    border-top-color: #ff0080;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .sidebar-error {
    background: rgba(255, 0, 80, 0.1);
    border: 1px solid rgba(255, 0, 80, 0.3);
    color: #ff5080;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
