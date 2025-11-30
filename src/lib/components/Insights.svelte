<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { translations } from '$lib/i18n';
  import { theme } from '$lib/theme';

  interface Stats {
    corrections_count: number;
    patterns_count: number;
    anomalies_pending: number;
    duplicates_pending: number;
    cached_responses: number;
    fuel_prices_stored: number;
    routes_learned: number;
  }

  interface Anomaly {
    id: string;
    flight_id: string;
    anomaly_type: string;
    severity: string;
    description: string;
    suggested_fix: string | null;
  }

  interface Duplicate {
    id: string;
    flight_id_1: string;
    flight_id_2: string;
    similarity_score: number;
    match_reasons: string[];
  }

  interface RouteStats {
    departure_airport: string;
    arrival_airport: string;
    avg_duration_minutes: number | null;
    avg_distance_km: number | null;
    flight_count: number;
  }

  interface FuelPrice {
    id: string;
    airport_code: string | null;
    location_name: string;
    fuel_type: string;
    price_per_gallon: number;
    currency: string;
    effective_date: string;
    source: string;
  }

  let { userId }: { userId: string } = $props();

  let stats: Stats | null = $state(null);
  let anomalies: Anomaly[] = $state([]);
  let duplicates: Duplicate[] = $state([]);
  let fuelPrices: FuelPrice[] = $state([]);
  let routes: RouteStats[] = $state([]);
  let loading = $state(true);
  let populating = $state(false);
  let activeTab = $state<'overview' | 'anomalies' | 'duplicates' | 'fuel' | 'routes'>('overview');

  onMount(async () => {
    await loadAll();
  });

  async function loadAll() {
    loading = true;
    try {
      const [statsData, anomalyData, dupData, fuelData, defaultsData] = await Promise.all([
        invoke('get_self_improvement_stats'),
        invoke('get_pending_anomalies'),
        invoke('get_pending_duplicates'),
        invoke('get_fuel_price_history', { airportCode: null, fuelType: null, limit: 50 }),
        invoke('get_smart_defaults', { userId }),
      ]);
      stats = statsData as Stats;
      anomalies = anomalyData as Anomaly[];
      duplicates = dupData as Duplicate[];
      fuelPrices = fuelData as FuelPrice[];
      routes = (defaultsData as any).routes || [];
    } catch (e) {
      console.error('Failed to load insights:', e);
    }
    loading = false;
  }

  async function resolveAnomaly(id: string) {
    try {
      await invoke('resolve_anomaly', { anomalyId: id });
      anomalies = anomalies.filter(a => a.id !== id);
      if (stats) stats.anomalies_pending--;
    } catch (e) {
      console.error('Failed to resolve:', e);
    }
  }

  async function populateRouteStats() {
    populating = true;
    try {
      const result = await invoke<{ routes_added: number; routes_updated: number; total_routes: number }>('populate_route_statistics');
      console.log('Route statistics populated:', result);
      // Reload stats to show updated counts
      await loadAll();
    } catch (e) {
      console.error('Failed to populate route statistics:', e);
    }
    populating = false;
  }

  function getSeverityColor(severity: string): string {
    switch (severity) {
      case 'error': return 'text-red-600 bg-red-100 dark:bg-red-900/30';
      case 'warning': return 'text-yellow-600 bg-yellow-100 dark:bg-yellow-900/30';
      default: return 'text-blue-600 bg-blue-100 dark:bg-blue-900/30';
    }
  }
</script>

<div class="insights-container p-6 space-y-6 {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}">
  <div class="flex items-center justify-between">
    <h2 class="insights-title text-2xl font-bold">{$translations('insights.title')}</h2>
    <div class="flex gap-2">
      <button
        onclick={populateRouteStats}
        disabled={populating || loading}
        class="insights-populate-btn px-4 py-2 rounded-lg disabled:opacity-50 transition"
        title="Analyze flight data to learn route statistics"
      >
        {populating ? 'Analyzing...' : '🔄 Learn from Flights'}
      </button>
      <button
        onclick={loadAll}
        disabled={loading}
        class="insights-refresh-btn px-4 py-2 rounded-lg disabled:opacity-50 transition"
      >
        {loading ? $translations('common.loading') : $translations('insights.refresh')}
      </button>
    </div>
  </div>

  <!-- Tabs -->
  <div class="insights-tabs-border">
    <nav class="flex gap-4">
      {#each ['overview', 'anomalies', 'duplicates', 'fuel', 'routes'] as tab}
        <button
          onclick={() => activeTab = tab as any}
          class="insights-tab pb-2 px-1 text-sm font-medium border-b-2 transition {activeTab === tab ? 'insights-tab-active' : 'insights-tab-inactive'}"
        >
          {$translations(`insights.tabs.${tab}`)}
          {#if tab === 'anomalies' && stats?.anomalies_pending}
            <span class="insights-badge-error ml-1 px-1.5 py-0.5 text-xs rounded-full">{stats.anomalies_pending}</span>
          {/if}
          {#if tab === 'duplicates' && stats?.duplicates_pending}
            <span class="insights-badge-warning ml-1 px-1.5 py-0.5 text-xs rounded-full">{stats.duplicates_pending}</span>
          {/if}
        </button>
      {/each}
    </nav>
  </div>

  <!-- Overview Tab -->
  {#if activeTab === 'overview'}
    {#if loading}
      <div class="insights-loading text-center py-8">{$translations('common.loading')}</div>
    {:else if stats}
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div class="insights-stat-card rounded-xl p-4">
          <div class="insights-stat-value text-3xl font-bold">{stats.routes_learned}</div>
          <div class="insights-stat-label text-sm">{$translations('insights.stats.routesLearned')}</div>
        </div>
        <div class="insights-stat-card rounded-xl p-4">
          <div class="insights-stat-value text-3xl font-bold">{stats.fuel_prices_stored}</div>
          <div class="insights-stat-label text-sm">{$translations('insights.stats.fuelPricesStored')}</div>
        </div>
        <div class="insights-stat-card rounded-xl p-4">
          <div class="insights-stat-value text-3xl font-bold">{stats.cached_responses}</div>
          <div class="insights-stat-label text-sm">{$translations('insights.stats.aiResponsesCached')}</div>
        </div>
        <div class="insights-stat-card rounded-xl p-4">
          <div class="insights-stat-value text-3xl font-bold">{stats.patterns_count}</div>
          <div class="insights-stat-label text-sm">{$translations('insights.stats.userPatterns')}</div>
        </div>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-6 mt-6">
        <!-- Learning Summary -->
        <div class="insights-panel rounded-xl p-6">
          <h3 class="insights-panel-title font-semibold mb-4">{$translations('insights.learningStatus.title')}</h3>
          <div class="space-y-3">
            <div class="flex justify-between items-center">
              <span class="insights-panel-label">{$translations('insights.stats.correctionsLearned')}</span>
              <span class="insights-panel-value font-mono">{stats.corrections_count}</span>
            </div>
            <div class="flex justify-between items-center">
              <span class="insights-panel-label">{$translations('insights.stats.behavioralPatterns')}</span>
              <span class="insights-panel-value font-mono">{stats.patterns_count}</span>
            </div>
            <div class="flex justify-between items-center">
              <span class="insights-panel-label">{$translations('insights.stats.routeStatistics')}</span>
              <span class="insights-panel-value font-mono">{stats.routes_learned}</span>
            </div>
          </div>
        </div>

        <!-- Issues -->
        <div class="insights-panel rounded-xl p-6">
          <h3 class="insights-panel-title font-semibold mb-4">{$translations('insights.issues.title')}</h3>
          <div class="space-y-3">
            <div class="flex justify-between items-center">
              <span class="insights-panel-label">{$translations('insights.issues.anomaliesToReview')}</span>
              <span class="font-mono {stats.anomalies_pending > 0 ? 'insights-value-error' : 'insights-value-success'}">{stats.anomalies_pending}</span>
            </div>
            <div class="flex justify-between items-center">
              <span class="insights-panel-label">{$translations('insights.issues.potentialDuplicates')}</span>
              <span class="font-mono {stats.duplicates_pending > 0 ? 'insights-value-warning' : 'insights-value-success'}">{stats.duplicates_pending}</span>
            </div>
          </div>
        </div>
      </div>
    {/if}
  {/if}

  <!-- Anomalies Tab -->
  {#if activeTab === 'anomalies'}
    {#if anomalies.length === 0}
      <div class="insights-empty text-center py-12">
        <div class="text-4xl mb-2">✓</div>
        <p>{$translations('insights.anomalies.noAnomalies')}</p>
      </div>
    {:else}
      <div class="space-y-3">
        {#each anomalies as anomaly}
          <div class="insights-anomaly-card rounded-lg p-4 border-l-4 {anomaly.severity === 'error' ? 'insights-border-error' : 'insights-border-warning'}">
            <div class="flex items-start justify-between">
              <div>
                <span class="insights-severity-badge px-2 py-0.5 rounded text-xs font-medium {anomaly.severity === 'error' ? 'insights-severity-error' : 'insights-severity-warning'}">
                  {anomaly.severity.toUpperCase()}
                </span>
                <span class="insights-anomaly-type ml-2 text-xs">{anomaly.anomaly_type}</span>
                <p class="insights-anomaly-desc mt-2">{anomaly.description}</p>
                {#if anomaly.suggested_fix}
                  <p class="insights-suggestion mt-1 text-sm">{$translations('insights.anomalies.suggestion')}: {anomaly.suggested_fix}</p>
                {/if}
              </div>
              <button
                onclick={() => resolveAnomaly(anomaly.id)}
                class="insights-resolve-btn text-sm transition"
              >
                {$translations('insights.anomalies.markResolved')}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  <!-- Duplicates Tab -->
  {#if activeTab === 'duplicates'}
    {#if duplicates.length === 0}
      <div class="insights-empty text-center py-12">
        <div class="text-4xl mb-2">✓</div>
        <p>{$translations('insights.duplicates.noDuplicates')}</p>
      </div>
    {:else}
      <div class="space-y-3">
        {#each duplicates as dup}
          <div class="insights-duplicate-card rounded-lg p-4">
            <div class="flex items-center justify-between mb-2">
              <span class="insights-duplicate-score text-sm font-medium">
                {Math.round(dup.similarity_score * 100)}% {$translations('insights.duplicates.match')}
              </span>
              <div class="flex gap-2">
                <button class="insights-action-keep text-xs transition">{$translations('insights.duplicates.keepBoth')}</button>
                <button class="insights-action-merge text-xs transition">{$translations('insights.duplicates.merge')}</button>
              </div>
            </div>
            <div class="flex flex-wrap gap-1">
              {#each dup.match_reasons as reason}
                <span class="insights-reason-tag px-2 py-0.5 text-xs rounded">{reason}</span>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  <!-- Fuel Prices Tab -->
  {#if activeTab === 'fuel'}
    {#if fuelPrices.length === 0}
      <div class="insights-empty text-center py-12">
        <div class="text-4xl mb-2">⛽</div>
        <p>{$translations('insights.fuelPrices.noData')}</p>
        <p class="text-sm mt-2">{$translations('insights.fuelPrices.searchToAdd')}</p>
      </div>
    {:else}
      <div class="insights-table-container overflow-x-auto rounded-lg">
        <table class="w-full text-sm">
          <thead>
            <tr class="insights-table-header">
              <th class="text-left py-2 px-3">{$translations('insights.fuelPrices.location')}</th>
              <th class="text-left py-2 px-3">{$translations('insights.fuelPrices.type')}</th>
              <th class="text-right py-2 px-3">{$translations('insights.fuelPrices.pricePerGal')}</th>
              <th class="text-left py-2 px-3">{$translations('insights.fuelPrices.date')}</th>
              <th class="text-left py-2 px-3">{$translations('insights.fuelPrices.source')}</th>
            </tr>
          </thead>
          <tbody>
            {#each fuelPrices as price}
              <tr class="insights-table-row">
                <td class="py-2 px-3">
                  {#if price.airport_code}
                    <span class="insights-airport-code font-mono">{price.airport_code}</span> -
                  {/if}
                  {price.location_name}
                </td>
                <td class="py-2 px-3 capitalize">{price.fuel_type.replace('_', ' ')}</td>
                <td class="py-2 px-3 text-right font-mono insights-price">${price.price_per_gallon.toFixed(2)}</td>
                <td class="py-2 px-3">{price.effective_date}</td>
                <td class="py-2 px-3 insights-source">{price.source}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <div class="insights-count mt-4 text-sm">
        {fuelPrices.length} {$translations('insights.fuelPrices.count').replace('{count}', '')}
      </div>
    {/if}
  {/if}

  <!-- Routes Tab -->
  {#if activeTab === 'routes'}
    {#if routes.length === 0}
      <div class="insights-empty text-center py-12">
        <div class="text-4xl mb-2">✈️</div>
        <p>{$translations('insights.routes.noData')}</p>
        <p class="text-sm mt-2">{$translations('insights.routes.learnedFromFlights')}</p>
      </div>
    {:else}
      <div class="insights-table-container overflow-x-auto rounded-lg">
        <table class="w-full text-sm">
          <thead>
            <tr class="insights-table-header">
              <th class="text-left py-2 px-3">{$translations('insights.routes.route')}</th>
              <th class="text-right py-2 px-3">{$translations('insights.routes.avgDuration')}</th>
              <th class="text-right py-2 px-3">{$translations('insights.routes.avgDistance')}</th>
              <th class="text-right py-2 px-3">{$translations('insights.routes.flights')}</th>
            </tr>
          </thead>
          <tbody>
            {#each routes as route}
              <tr class="insights-table-row">
                <td class="py-2 px-3 font-mono insights-route">
                  {route.departure_airport} → {route.arrival_airport}
                </td>
                <td class="py-2 px-3 text-right">
                  {#if route.avg_duration_minutes}
                    {Math.round(route.avg_duration_minutes)} min
                  {:else}
                    -
                  {/if}
                </td>
                <td class="py-2 px-3 text-right">
                  {#if route.avg_distance_km}
                    {Math.round(route.avg_distance_km)} km
                  {:else}
                    -
                  {/if}
                </td>
                <td class="py-2 px-3 text-right">{route.flight_count}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<style>
  /* Default Theme */
  .theme-default .insights-title { color: #1f2937; }
  .theme-default .insights-refresh-btn {
    background-color: #4f46e5;
    color: white;
  }
  .theme-default .insights-refresh-btn:hover { background-color: #4338ca; }
  .theme-default .insights-populate-btn {
    background-color: #10b981;
    color: white;
  }
  .theme-default .insights-populate-btn:hover { background-color: #059669; }
  .theme-default .insights-tabs-border { border-bottom: 1px solid #e5e7eb; }
  .theme-default .insights-tab-active { border-color: #4f46e5; color: #4f46e5; }
  .theme-default .insights-tab-inactive { border-color: transparent; color: #6b7280; }
  .theme-default .insights-tab-inactive:hover { color: #374151; }
  .theme-default .insights-badge-error { background-color: #ef4444; color: white; }
  .theme-default .insights-badge-warning { background-color: #f59e0b; color: white; }
  .theme-default .insights-stat-card {
    background: linear-gradient(135deg, #4f46e5, #7c3aed);
    color: white;
  }
  .theme-default .insights-stat-value { color: white; }
  .theme-default .insights-stat-label { opacity: 0.8; }
  .theme-default .insights-panel {
    background-color: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }
  .theme-default .insights-panel-title { color: #1f2937; }
  .theme-default .insights-panel-label { color: #6b7280; }
  .theme-default .insights-panel-value { color: #1f2937; }
  .theme-default .insights-value-error { color: #ef4444; }
  .theme-default .insights-value-warning { color: #f59e0b; }
  .theme-default .insights-value-success { color: #10b981; }
  .theme-default .insights-empty { color: #6b7280; }
  .theme-default .insights-loading { color: #6b7280; }
  .theme-default .insights-anomaly-card {
    background-color: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }
  .theme-default .insights-border-error { border-left-color: #ef4444; }
  .theme-default .insights-border-warning { border-left-color: #f59e0b; }
  .theme-default .insights-severity-error { background-color: #fef2f2; color: #dc2626; }
  .theme-default .insights-severity-warning { background-color: #fefce8; color: #ca8a04; }
  .theme-default .insights-anomaly-type { color: #6b7280; }
  .theme-default .insights-anomaly-desc { color: #1f2937; }
  .theme-default .insights-suggestion { color: #4f46e5; }
  .theme-default .insights-resolve-btn { color: #6b7280; }
  .theme-default .insights-resolve-btn:hover { color: #10b981; }
  .theme-default .insights-duplicate-card {
    background-color: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }
  .theme-default .insights-duplicate-score { color: #1f2937; }
  .theme-default .insights-action-keep { color: #10b981; }
  .theme-default .insights-action-keep:hover { text-decoration: underline; }
  .theme-default .insights-action-merge { color: #ef4444; }
  .theme-default .insights-action-merge:hover { text-decoration: underline; }
  .theme-default .insights-reason-tag { background-color: #f3f4f6; color: #374151; }
  .theme-default .insights-table-container { background-color: white; }
  .theme-default .insights-table-header { border-bottom: 1px solid #e5e7eb; }
  .theme-default .insights-table-header th { color: #374151; }
  .theme-default .insights-table-row { border-bottom: 1px solid #e5e7eb; color: #1f2937; }
  .theme-default .insights-table-row:hover { background-color: #f9fafb; }
  .theme-default .insights-airport-code { color: #4f46e5; }
  .theme-default .insights-price { color: #1f2937; }
  .theme-default .insights-source { color: #6b7280; }
  .theme-default .insights-count { color: #6b7280; }
  .theme-default .insights-route { color: #4f46e5; }

  /* Skynet Theme */
  .theme-skynet .insights-title {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }
  .theme-skynet .insights-refresh-btn {
    background: linear-gradient(135deg, rgba(0, 60, 120, 0.8), rgba(0, 40, 80, 0.8));
    color: #00b4ff;
    border: 1px solid rgba(0, 180, 255, 0.5);
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .insights-refresh-btn:hover {
    background: linear-gradient(135deg, rgba(0, 80, 160, 0.9), rgba(0, 60, 120, 0.9));
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.5);
  }
  .theme-skynet .insights-populate-btn {
    background: linear-gradient(135deg, rgba(0, 100, 60, 0.8), rgba(0, 80, 40, 0.8));
    color: #00ff88;
    border: 1px solid rgba(0, 255, 136, 0.5);
    box-shadow: 0 0 15px rgba(0, 255, 136, 0.3);
  }
  .theme-skynet .insights-populate-btn:hover {
    background: linear-gradient(135deg, rgba(0, 130, 80, 0.9), rgba(0, 100, 60, 0.9));
    box-shadow: 0 0 20px rgba(0, 255, 136, 0.5);
  }
  .theme-skynet .insights-tabs-border { border-bottom: 1px solid rgba(0, 180, 255, 0.3); }
  .theme-skynet .insights-tab-active {
    border-color: #00b4ff;
    color: #00b4ff;
    text-shadow: 0 0 8px rgba(0, 180, 255, 0.5);
  }
  .theme-skynet .insights-tab-inactive { border-color: transparent; color: rgba(0, 180, 255, 0.5); }
  .theme-skynet .insights-tab-inactive:hover { color: #00b4ff; }
  .theme-skynet .insights-badge-error {
    background: linear-gradient(135deg, #ff3366, #ff0044);
    color: white;
    box-shadow: 0 0 10px rgba(255, 51, 102, 0.5);
  }
  .theme-skynet .insights-badge-warning {
    background: linear-gradient(135deg, #ff9500, #ff6600);
    color: white;
    box-shadow: 0 0 10px rgba(255, 149, 0, 0.5);
  }
  .theme-skynet .insights-stat-card {
    background: linear-gradient(135deg, rgba(0, 40, 80, 0.8), rgba(0, 20, 40, 0.6));
    border: 1px solid rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.2);
  }
  .theme-skynet .insights-stat-value {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }
  .theme-skynet .insights-stat-label { color: rgba(0, 180, 255, 0.7); }
  .theme-skynet .insights-panel {
    background: linear-gradient(135deg, rgba(0, 30, 60, 0.9), rgba(0, 15, 30, 0.8));
    border: 1px solid rgba(0, 180, 255, 0.3);
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.15);
  }
  .theme-skynet .insights-panel-title {
    color: #00b4ff;
    text-shadow: 0 0 8px rgba(0, 180, 255, 0.4);
  }
  .theme-skynet .insights-panel-label { color: rgba(0, 180, 255, 0.6); }
  .theme-skynet .insights-panel-value { color: #00b4ff; }
  .theme-skynet .insights-value-error {
    color: #ff3366;
    text-shadow: 0 0 8px rgba(255, 51, 102, 0.5);
  }
  .theme-skynet .insights-value-warning {
    color: #ff9500;
    text-shadow: 0 0 8px rgba(255, 149, 0, 0.5);
  }
  .theme-skynet .insights-value-success {
    color: #00ff88;
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.5);
  }
  .theme-skynet .insights-empty { color: rgba(0, 180, 255, 0.5); }
  .theme-skynet .insights-loading { color: rgba(0, 180, 255, 0.5); }
  .theme-skynet .insights-anomaly-card {
    background: linear-gradient(135deg, rgba(0, 30, 60, 0.9), rgba(0, 15, 30, 0.8));
    border: 1px solid rgba(0, 180, 255, 0.3);
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.15);
  }
  .theme-skynet .insights-border-error { border-left-color: #ff3366; }
  .theme-skynet .insights-border-warning { border-left-color: #ff9500; }
  .theme-skynet .insights-severity-error {
    background: rgba(255, 51, 102, 0.2);
    color: #ff3366;
  }
  .theme-skynet .insights-severity-warning {
    background: rgba(255, 149, 0, 0.2);
    color: #ff9500;
  }
  .theme-skynet .insights-anomaly-type { color: rgba(0, 180, 255, 0.5); }
  .theme-skynet .insights-anomaly-desc { color: rgba(255, 255, 255, 0.9); }
  .theme-skynet .insights-suggestion { color: #00b4ff; }
  .theme-skynet .insights-resolve-btn { color: rgba(0, 180, 255, 0.6); }
  .theme-skynet .insights-resolve-btn:hover {
    color: #00ff88;
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.5);
  }
  .theme-skynet .insights-duplicate-card {
    background: linear-gradient(135deg, rgba(0, 30, 60, 0.9), rgba(0, 15, 30, 0.8));
    border: 1px solid rgba(0, 180, 255, 0.3);
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.15);
  }
  .theme-skynet .insights-duplicate-score { color: #00b4ff; }
  .theme-skynet .insights-action-keep {
    color: #00ff88;
  }
  .theme-skynet .insights-action-keep:hover {
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.5);
  }
  .theme-skynet .insights-action-merge {
    color: #ff3366;
  }
  .theme-skynet .insights-action-merge:hover {
    text-shadow: 0 0 8px rgba(255, 51, 102, 0.5);
  }
  .theme-skynet .insights-reason-tag {
    background: rgba(0, 180, 255, 0.2);
    color: #00b4ff;
    border: 1px solid rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .insights-table-container {
    background: linear-gradient(135deg, rgba(0, 30, 60, 0.9), rgba(0, 15, 30, 0.8));
    border: 1px solid rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .insights-table-header {
    border-bottom: 1px solid rgba(0, 180, 255, 0.3);
    background: rgba(0, 180, 255, 0.1);
  }
  .theme-skynet .insights-table-header th { color: #00b4ff; }
  .theme-skynet .insights-table-row {
    border-bottom: 1px solid rgba(0, 180, 255, 0.15);
    color: rgba(255, 255, 255, 0.9);
  }
  .theme-skynet .insights-table-row:hover {
    background: rgba(0, 180, 255, 0.1);
  }
  .theme-skynet .insights-airport-code {
    color: #00b4ff;
    text-shadow: 0 0 8px rgba(0, 180, 255, 0.4);
  }
  .theme-skynet .insights-price {
    color: #00ff88;
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.3);
  }
  .theme-skynet .insights-source { color: rgba(0, 180, 255, 0.5); }
  .theme-skynet .insights-count { color: rgba(0, 180, 255, 0.5); }
  .theme-skynet .insights-route {
    color: #00b4ff;
    text-shadow: 0 0 8px rgba(0, 180, 255, 0.4);
  }

  /* Cyberpunk Theme */
  .theme-cyberpunk .insights-title {
    color: #ff0080;
    text-shadow: 0 0 10px rgba(255, 0, 128, 0.5);
  }
  .theme-cyberpunk .insights-refresh-btn {
    background: linear-gradient(135deg, rgba(176, 0, 255, 0.3), rgba(255, 0, 128, 0.3));
    color: #00d9ff;
    border: 1px solid rgba(0, 217, 255, 0.5);
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.3);
  }
  .theme-cyberpunk .insights-refresh-btn:hover {
    background: linear-gradient(135deg, rgba(176, 0, 255, 0.5), rgba(255, 0, 128, 0.5));
    box-shadow: 0 0 20px rgba(255, 0, 128, 0.5);
  }
  .theme-cyberpunk .insights-populate-btn {
    background: linear-gradient(135deg, rgba(0, 150, 100, 0.3), rgba(0, 255, 136, 0.3));
    color: #00ff88;
    border: 1px solid rgba(0, 255, 136, 0.5);
    box-shadow: 0 0 15px rgba(0, 255, 136, 0.3);
  }
  .theme-cyberpunk .insights-populate-btn:hover {
    background: linear-gradient(135deg, rgba(0, 180, 120, 0.5), rgba(0, 255, 136, 0.5));
    box-shadow: 0 0 20px rgba(0, 255, 136, 0.5);
  }
  .theme-cyberpunk .insights-tabs-border { border-bottom: 1px solid rgba(255, 0, 128, 0.3); }
  .theme-cyberpunk .insights-tab-active {
    border-color: #ff0080;
    color: #ff0080;
    text-shadow: 0 0 8px rgba(255, 0, 128, 0.5);
  }
  .theme-cyberpunk .insights-tab-inactive { border-color: transparent; color: rgba(255, 0, 128, 0.5); }
  .theme-cyberpunk .insights-tab-inactive:hover { color: #ff0080; }
  .theme-cyberpunk .insights-badge-error {
    background: linear-gradient(135deg, #ff0044, #cc0033);
    color: white;
    box-shadow: 0 0 10px rgba(255, 0, 68, 0.5);
  }
  .theme-cyberpunk .insights-badge-warning {
    background: linear-gradient(135deg, #ff9500, #cc7700);
    color: white;
    box-shadow: 0 0 10px rgba(255, 149, 0, 0.5);
  }
  .theme-cyberpunk .insights-stat-card {
    background: linear-gradient(135deg, rgba(176, 0, 255, 0.2), rgba(255, 0, 128, 0.2));
    border: 1px solid rgba(255, 0, 128, 0.4);
    box-shadow: 0 0 20px rgba(255, 0, 128, 0.2);
  }
  .theme-cyberpunk .insights-stat-value {
    color: #00d9ff;
    text-shadow: 0 0 10px rgba(0, 217, 255, 0.5);
  }
  .theme-cyberpunk .insights-stat-label { color: rgba(255, 0, 128, 0.7); }
  .theme-cyberpunk .insights-panel {
    background: linear-gradient(135deg, rgba(30, 0, 50, 0.9), rgba(50, 0, 30, 0.8));
    border: 1px solid rgba(255, 0, 128, 0.3);
    box-shadow: 0 0 15px rgba(176, 0, 255, 0.15);
  }
  .theme-cyberpunk .insights-panel-title {
    color: #ff0080;
    text-shadow: 0 0 8px rgba(255, 0, 128, 0.4);
  }
  .theme-cyberpunk .insights-panel-label { color: rgba(255, 0, 128, 0.6); }
  .theme-cyberpunk .insights-panel-value { color: #00d9ff; }
  .theme-cyberpunk .insights-value-error {
    color: #ff0044;
    text-shadow: 0 0 8px rgba(255, 0, 68, 0.5);
  }
  .theme-cyberpunk .insights-value-warning {
    color: #ff9500;
    text-shadow: 0 0 8px rgba(255, 149, 0, 0.5);
  }
  .theme-cyberpunk .insights-value-success {
    color: #00ff88;
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.5);
  }
  .theme-cyberpunk .insights-empty { color: rgba(255, 0, 128, 0.5); }
  .theme-cyberpunk .insights-loading { color: rgba(255, 0, 128, 0.5); }
  .theme-cyberpunk .insights-anomaly-card {
    background: linear-gradient(135deg, rgba(30, 0, 50, 0.9), rgba(50, 0, 30, 0.8));
    border: 1px solid rgba(255, 0, 128, 0.3);
    box-shadow: 0 0 15px rgba(176, 0, 255, 0.15);
  }
  .theme-cyberpunk .insights-border-error { border-left-color: #ff0044; }
  .theme-cyberpunk .insights-border-warning { border-left-color: #ff9500; }
  .theme-cyberpunk .insights-severity-error {
    background: rgba(255, 0, 68, 0.2);
    color: #ff0044;
  }
  .theme-cyberpunk .insights-severity-warning {
    background: rgba(255, 149, 0, 0.2);
    color: #ff9500;
  }
  .theme-cyberpunk .insights-anomaly-type { color: rgba(255, 0, 128, 0.5); }
  .theme-cyberpunk .insights-anomaly-desc { color: rgba(255, 255, 255, 0.9); }
  .theme-cyberpunk .insights-suggestion { color: #00d9ff; }
  .theme-cyberpunk .insights-resolve-btn { color: rgba(255, 0, 128, 0.6); }
  .theme-cyberpunk .insights-resolve-btn:hover {
    color: #00ff88;
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.5);
  }
  .theme-cyberpunk .insights-duplicate-card {
    background: linear-gradient(135deg, rgba(30, 0, 50, 0.9), rgba(50, 0, 30, 0.8));
    border: 1px solid rgba(255, 0, 128, 0.3);
    box-shadow: 0 0 15px rgba(176, 0, 255, 0.15);
  }
  .theme-cyberpunk .insights-duplicate-score { color: #00d9ff; }
  .theme-cyberpunk .insights-action-keep {
    color: #00ff88;
  }
  .theme-cyberpunk .insights-action-keep:hover {
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.5);
  }
  .theme-cyberpunk .insights-action-merge {
    color: #ff0044;
  }
  .theme-cyberpunk .insights-action-merge:hover {
    text-shadow: 0 0 8px rgba(255, 0, 68, 0.5);
  }
  .theme-cyberpunk .insights-reason-tag {
    background: rgba(255, 0, 128, 0.2);
    color: #ff0080;
    border: 1px solid rgba(255, 0, 128, 0.3);
  }
  .theme-cyberpunk .insights-table-container {
    background: linear-gradient(135deg, rgba(30, 0, 50, 0.9), rgba(50, 0, 30, 0.8));
    border: 1px solid rgba(255, 0, 128, 0.3);
  }
  .theme-cyberpunk .insights-table-header {
    border-bottom: 1px solid rgba(255, 0, 128, 0.3);
    background: rgba(255, 0, 128, 0.1);
  }
  .theme-cyberpunk .insights-table-header th { color: #ff0080; }
  .theme-cyberpunk .insights-table-row {
    border-bottom: 1px solid rgba(255, 0, 128, 0.15);
    color: rgba(255, 255, 255, 0.9);
  }
  .theme-cyberpunk .insights-table-row:hover {
    background: rgba(255, 0, 128, 0.1);
  }
  .theme-cyberpunk .insights-airport-code {
    color: #ff0080;
    text-shadow: 0 0 8px rgba(255, 0, 128, 0.4);
  }
  .theme-cyberpunk .insights-price {
    color: #00ff88;
    text-shadow: 0 0 8px rgba(0, 255, 136, 0.3);
  }
  .theme-cyberpunk .insights-source { color: rgba(255, 0, 128, 0.5); }
  .theme-cyberpunk .insights-count { color: rgba(255, 0, 128, 0.5); }
  .theme-cyberpunk .insights-route {
    color: #00d9ff;
    text-shadow: 0 0 8px rgba(0, 217, 255, 0.4);
  }

  /* Dark mode defaults */
  :global(.dark) .theme-default .insights-title { color: white; }
  :global(.dark) .theme-default .insights-panel {
    background-color: #1f2937;
  }
  :global(.dark) .theme-default .insights-panel-title { color: white; }
  :global(.dark) .theme-default .insights-panel-label { color: #9ca3af; }
  :global(.dark) .theme-default .insights-panel-value { color: white; }
  :global(.dark) .theme-default .insights-anomaly-card {
    background-color: #1f2937;
  }
  :global(.dark) .theme-default .insights-anomaly-desc { color: white; }
  :global(.dark) .theme-default .insights-severity-error { background-color: rgba(185, 28, 28, 0.3); }
  :global(.dark) .theme-default .insights-severity-warning { background-color: rgba(202, 138, 4, 0.3); }
  :global(.dark) .theme-default .insights-duplicate-card {
    background-color: #1f2937;
  }
  :global(.dark) .theme-default .insights-duplicate-score { color: white; }
  :global(.dark) .theme-default .insights-reason-tag { background-color: #374151; color: #d1d5db; }
  :global(.dark) .theme-default .insights-table-container { background-color: #1f2937; }
  :global(.dark) .theme-default .insights-table-header { border-color: #374151; }
  :global(.dark) .theme-default .insights-table-header th { color: #d1d5db; }
  :global(.dark) .theme-default .insights-table-row { border-color: #374151; color: white; }
  :global(.dark) .theme-default .insights-table-row:hover { background-color: #374151; }
  :global(.dark) .theme-default .insights-price { color: white; }
</style>
