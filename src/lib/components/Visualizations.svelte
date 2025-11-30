<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  // Sub-tab state - lazy loading (Charts default, Network Graph secondary due to intensity)
  type VisualizationTab = 'charts' | 'passengers' | 'network';
  let activeTab = $state<VisualizationTab>('charts');

  // Network graph data and state
  let networkData = $state<any | null>(null);
  let networkLoading = $state(false);
  let networkError = $state<string | null>(null);
  let networkMounted = $state(false);

  // Chart data states (only comparative loads here - temporal/geospatial are self-loading)
  let comparativeData = $state<any[]>([]);
  let chartsLoading = $state(false);
  let chartsError = $state<string | null>(null);
  let chartsMounted = $state(false);

  // Passengers tab state
  let passengersMounted = $state(false);

  // Dynamic imports for heavy components - only load when needed
  let NetworkGraph: any = $state(null);
  let TemporalChart: any = $state(null);
  let GeospatialChart: any = $state(null);
  let ComparativeChart: any = $state(null);
  let PassengerPieChart: any = $state(null);

  // Load network data
  async function loadNetworkData() {
    if (networkData) return;
    networkLoading = true;
    networkError = null;
    try {
      const data = await invoke('get_passenger_network', {
        request: { user_id: userId, min_flights_together: 1 }
      });
      networkData = data || null;
    } catch (err) {
      console.error('Failed to load network data:', err);
      networkError = String(err);
    } finally {
      networkLoading = false;
    }
  }

  // Load chart data (only comparative - temporal/geospatial are self-loading now)
  async function loadChartData() {
    if (comparativeData.length > 0) return;
    chartsLoading = true;
    chartsError = null;
    try {
      const comparative = await invoke('get_comparative_metrics', {
        request: { user_id: userId, rank_by: 'flights', limit: 10 }
      });
      comparativeData = Array.isArray(comparative) ? comparative : [];
    } catch (err) {
      console.error('Failed to load chart data:', err);
      chartsError = String(err);
    } finally {
      chartsLoading = false;
    }
  }

  // Handle tab change with lazy loading and cleanup
  async function handleTabChange(tab: VisualizationTab) {
    // Cleanup previous tab to free resources
    if (activeTab === 'network') {
      networkMounted = false;
    } else if (activeTab === 'charts') {
      chartsMounted = false;
    } else if (activeTab === 'passengers') {
      passengersMounted = false;
    }

    activeTab = tab;

    // Load data and components for the new tab
    if (tab === 'network') {
      if (!NetworkGraph) {
        const module = await import('./analytics/NetworkGraph.svelte');
        NetworkGraph = module.default;
      }
      await loadNetworkData();
      networkMounted = true;
    } else if (tab === 'charts') {
      if (!TemporalChart) {
        const [tempMod, geoMod, compMod] = await Promise.all([
          import('./analytics/TemporalChart.svelte'),
          import('./analytics/GeospatialChart.svelte'),
          import('./analytics/ComparativeChart.svelte')
        ]);
        TemporalChart = tempMod.default;
        GeospatialChart = geoMod.default;
        ComparativeChart = compMod.default;
      }
      await loadChartData();
      chartsMounted = true;
    } else if (tab === 'passengers') {
      if (!PassengerPieChart) {
        const module = await import('./analytics/PassengerPieChart.svelte');
        PassengerPieChart = module.default;
      }
      passengersMounted = true;
    }
  }

  // Cleanup on destroy
  onDestroy(() => {
    networkMounted = false;
    chartsMounted = false;
    passengersMounted = false;
    networkData = null;
  });

  // Load initial tab (charts first - lighter weight)
  onMount(() => {
    handleTabChange('charts');
  });
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Header -->
  <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex-shrink-0">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-2">Visualizations</h2>
    <p class="text-gray-600 dark:text-gray-400 text-sm">
      Interactive visualizations of your flight data. Components are lazy-loaded and destroyed when switching tabs to save resources.
    </p>
  </div>

  <!-- Tab Navigation -->
  <div class="px-6 border-b border-gray-200 dark:border-gray-700 flex-shrink-0">
    <nav class="-mb-px flex gap-6">
      <button
        onclick={() => handleTabChange('charts')}
        class="py-3 px-1 border-b-2 text-sm font-medium transition {
          activeTab === 'charts'
            ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
            : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400'
        }"
      >
        <span class="mr-2">📊</span>Charts
      </button>
      <button
        onclick={() => handleTabChange('passengers')}
        class="py-3 px-1 border-b-2 text-sm font-medium transition {
          activeTab === 'passengers'
            ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
            : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400'
        }"
      >
        <span class="mr-2">👥</span>Passengers
      </button>
      <button
        onclick={() => handleTabChange('network')}
        class="py-3 px-1 border-b-2 text-sm font-medium transition {
          activeTab === 'network'
            ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
            : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400'
        }"
      >
        <span class="mr-2">🕸️</span>Network Graph
      </button>
    </nav>
  </div>

  <!-- Content Area -->
  <div class="flex-1 overflow-hidden">
    <!-- Network Graph Tab -->
    {#if activeTab === 'network'}
      <div class="h-full">
        {#if networkLoading}
          <div class="h-full flex items-center justify-center bg-gray-900">
            <div class="text-center">
              <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto mb-4"></div>
              <p class="text-gray-400">Loading passenger network...</p>
            </div>
          </div>
        {:else if networkError}
          <div class="h-full flex items-center justify-center bg-gray-900">
            <div class="bg-red-900/30 border border-red-800 rounded-lg p-6 max-w-md">
              <p class="text-red-400 font-medium mb-2">Failed to load network</p>
              <p class="text-red-300 text-sm">{networkError}</p>
              <button
                onclick={() => { networkData = null; loadNetworkData(); }}
                class="mt-4 bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-lg text-sm"
              >
                Retry
              </button>
            </div>
          </div>
        {:else if networkData && NetworkGraph && networkMounted}
          <svelte:component this={NetworkGraph} data={networkData} />
        {:else}
          <div class="h-full flex items-center justify-center bg-gray-900">
            <div class="text-center text-gray-400">
              <span class="text-4xl mb-4 block">🕸️</span>
              <p>No network data available</p>
              <p class="text-sm text-gray-500 mt-2">Add passengers with shared flights to see connections</p>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Charts Tab -->
    {#if activeTab === 'charts'}
      <div class="h-full overflow-y-auto p-6 bg-gray-50 dark:bg-gray-900">
        {#if chartsLoading}
          <div class="flex items-center justify-center py-12">
            <div class="text-center">
              <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto mb-4"></div>
              <p class="text-gray-600 dark:text-gray-400">Loading charts...</p>
            </div>
          </div>
        {:else if chartsError}
          <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-6">
            <p class="text-red-600 dark:text-red-400 font-medium mb-2">Failed to load charts</p>
            <p class="text-red-500 dark:text-red-300 text-sm">{chartsError}</p>
            <button
              onclick={() => { comparativeData = []; loadChartData(); }}
              class="mt-4 bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-lg text-sm"
            >
              Retry
            </button>
          </div>
        {:else if chartsMounted}
          <div class="space-y-6">
            <!-- Temporal Chart - Self-loading with filters -->
            {#if TemporalChart}
              <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Flight Patterns Over Time</h3>
                <svelte:component this={TemporalChart} {userId} initialGranularity="month" />
              </div>
            {/if}

            <!-- Geospatial Chart - Self-loading with filters -->
            {#if GeospatialChart}
              <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Airport Concentration</h3>
                <svelte:component this={GeospatialChart} {userId} initialLimit={50} />
              </div>
            {/if}

            <!-- Comparative Chart -->
            {#if ComparativeChart && comparativeData.length > 0}
              <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Passenger Metrics</h3>
                <svelte:component this={ComparativeChart} data={comparativeData} metric="flights" />
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Passengers Tab -->
    {#if activeTab === 'passengers'}
      <div class="h-full overflow-y-auto p-6 bg-gray-50 dark:bg-gray-900">
        {#if passengersMounted && PassengerPieChart}
          <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Passenger Flight Distribution</h3>
            <svelte:component this={PassengerPieChart} {userId} />
          </div>
        {:else}
          <div class="flex items-center justify-center py-12">
            <div class="text-center">
              <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600 mx-auto mb-4"></div>
              <p class="text-gray-600 dark:text-gray-400">Loading passenger data...</p>
            </div>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>
