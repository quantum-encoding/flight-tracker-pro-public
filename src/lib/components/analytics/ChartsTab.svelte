<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { t } from '$lib/i18n';

  import TemporalChart from './TemporalChart.svelte';
  import GeospatialChart from './GeospatialChart.svelte';
  import NetworkGraph from './NetworkGraph.svelte';
  import ComparativeChart from './ComparativeChart.svelte';
  import AircraftUtilizationChart from './AircraftUtilizationChart.svelte';
  import CostBreakdownDonut from './CostBreakdownDonut.svelte';
  import DayNightRadar from './DayNightRadar.svelte';
  import LongHaulMap from './LongHaulMap.svelte';
  import MonthlyCostTrend from './MonthlyCostTrend.svelte';
  import PilotCurrencyDashboard from './PilotCurrencyDashboard.svelte';
  import RunwayRiskTreemap from './RunwayRiskTreemap.svelte';

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  // Section 1: Temporal & Geospatial (now self-loading components)
  let temporalGeoLoaded = $state(false);

  // Section 2: Network & Comparative
  let networkData = $state<any | null>(null);
  let comparativeData = $state<any[]>([]);
  let networkCompLoading = $state(false);
  let networkCompError = $state<string | null>(null);
  let networkCompLoaded = $state(false);

  // Section 3: Aircraft & Costs
  let aircraftUtilData = $state<any[]>([]);
  let costBreakdownData = $state<any[]>([]);
  let aircraftCostLoading = $state(false);
  let aircraftCostError = $state<string | null>(null);
  let aircraftCostLoaded = $state(false);

  // Section 4: Flight Operations
  let dayNightStats = $state<any | null>(null);
  let longHaulFlights = $state<any[]>([]);
  let flightOpsLoading = $state(false);
  let flightOpsError = $state<string | null>(null);
  let flightOpsLoaded = $state(false);

  // Section 5: Pilot Metrics
  let currencyItems = $state<any[]>([]);
  let monthlyCostData = $state<any[]>([]);
  let pilotMetricsLoading = $state(false);
  let pilotMetricsError = $state<string | null>(null);
  let pilotMetricsLoaded = $state(false);

  // Section 6: Runway Risk
  let runwayRiskData = $state<any[]>([]);
  let runwayRiskLoading = $state(false);
  let runwayRiskError = $state<string | null>(null);
  let runwayRiskLoaded = $state(false);

  // Temporal & Geospatial charts now self-load their data via userId prop
  function showTemporalGeo() {
    temporalGeoLoaded = true;
  }

  async function loadNetworkComp() {
    networkCompLoading = true;
    networkCompError = null;
    try {
      console.log('[ChartsTab] Loading network/comparative data for user:', userId);
      const network = await invoke('get_passenger_network', {
        request: { user_id: userId, min_flights_together: 1 }
      });
      const comparative = await invoke('get_comparative_metrics', {
        request: { user_id: userId, rank_by: 'flights', limit: 10 }
      });
      console.log('[ChartsTab] Received network data:', network);
      console.log('[ChartsTab] Received comparative data:', Array.isArray(comparative) ? comparative.length : 'not array', comparative);

      networkData = network || null;
      comparativeData = Array.isArray(comparative) ? comparative : [];
      networkCompLoaded = true;
      console.log('[ChartsTab] Network/comparative loaded successfully');
    } catch (err) {
      console.error('[ChartsTab] Error loading network/comparative data:', err);
      networkCompError = String(err);
    } finally {
      networkCompLoading = false;
    }
  }

  async function loadAircraftCost() {
    aircraftCostLoading = true;
    aircraftCostError = null;
    try {
      console.log('[ChartsTab] Loading aircraft/cost data for user:', userId);
      const aircraftUtil = await invoke('get_aircraft_utilization', { userId });
      const costBreakdown = await invoke('get_cost_breakdown', { userId });
      console.log('[ChartsTab] Received aircraft util:', Array.isArray(aircraftUtil) ? aircraftUtil.length : 'not array', aircraftUtil);
      console.log('[ChartsTab] Received cost breakdown:', Array.isArray(costBreakdown) ? costBreakdown.length : 'not array', costBreakdown);

      aircraftUtilData = Array.isArray(aircraftUtil) ? aircraftUtil : [];
      costBreakdownData = Array.isArray(costBreakdown) ? costBreakdown : [];
      aircraftCostLoaded = true;
      console.log('[ChartsTab] Aircraft/cost loaded successfully');
    } catch (err) {
      console.error('[ChartsTab] Error loading aircraft/cost data:', err);
      aircraftCostError = String(err);
    } finally {
      aircraftCostLoading = false;
    }
  }

  async function loadFlightOps() {
    flightOpsLoading = true;
    flightOpsError = null;
    try {
      console.log('[ChartsTab] Loading flight ops data for user:', userId);
      const dayNight = await invoke('get_day_night_stats', { userId });
      const longHaul = await invoke('get_long_haul_flights', { userId, limit: 10 });
      console.log('[ChartsTab] Received day/night stats:', dayNight);
      console.log('[ChartsTab] Received long haul:', Array.isArray(longHaul) ? longHaul.length : 'not array', longHaul);

      dayNightStats = dayNight || null;
      longHaulFlights = Array.isArray(longHaul) ? longHaul : [];
      flightOpsLoaded = true;
      console.log('[ChartsTab] Flight ops loaded successfully');
    } catch (err) {
      console.error('[ChartsTab] Error loading flight ops data:', err);
      flightOpsError = String(err);
    } finally {
      flightOpsLoading = false;
    }
  }

  async function loadPilotMetrics() {
    pilotMetricsLoading = true;
    pilotMetricsError = null;
    try {
      console.log('[ChartsTab] Loading pilot metrics for user:', userId);
      const currency = await invoke('get_pilot_currency', { userId });
      const monthlyCost = await invoke('get_monthly_cost_trend', { userId });
      console.log('[ChartsTab] Received currency:', Array.isArray(currency) ? currency.length : 'not array', currency);
      console.log('[ChartsTab] Received monthly cost:', Array.isArray(monthlyCost) ? monthlyCost.length : 'not array', monthlyCost);

      currencyItems = Array.isArray(currency) ? currency : [];
      monthlyCostData = Array.isArray(monthlyCost) ? monthlyCost : [];
      pilotMetricsLoaded = true;
      console.log('[ChartsTab] Pilot metrics loaded successfully');
    } catch (err) {
      console.error('[ChartsTab] Error loading pilot metrics:', err);
      pilotMetricsError = String(err);
    } finally {
      pilotMetricsLoading = false;
    }
  }

  async function loadRunwayRisk() {
    runwayRiskLoading = true;
    runwayRiskError = null;
    try {
      console.log('[ChartsTab] Loading runway risk data for user:', userId);
      const runwayRisk = await invoke('get_runway_risk_data', { userId });
      console.log('[ChartsTab] Received runway risk:', Array.isArray(runwayRisk) ? runwayRisk.length : 'not array', runwayRisk);

      runwayRiskData = Array.isArray(runwayRisk) ? runwayRisk : [];
      runwayRiskLoaded = true;
      console.log('[ChartsTab] Runway risk loaded successfully');
    } catch (err) {
      console.error('[ChartsTab] Error loading runway risk data:', err);
      runwayRiskError = String(err);
    } finally {
      runwayRiskLoading = false;
    }
  }
</script>

<div class="space-y-8">
  <!-- Section: Temporal & Geospatial Analysis -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
    <div class="flex items-center justify-between mb-6">
      <h3 class="text-2xl font-bold text-gray-900 dark:text-white">{t('analytics.charts.temporalGeo.title')}</h3>
      {#if !temporalGeoLoaded}
        <button
          onclick={showTemporalGeo}
          class="bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg transition"
        >
          {t('analytics.loadButton')}
        </button>
      {/if}
    </div>
    {#if temporalGeoLoaded}
      <div class="space-y-6">
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.temporalGeo.flightPatterns')}</h4>
          <TemporalChart {userId} initialGranularity="month" />
        </div>
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.temporalGeo.airportConcentration')}</h4>
          <GeospatialChart {userId} initialLimit={50} />
        </div>
      </div>
    {/if}
  </div>

  <!-- Section: Network & Comparative Analysis -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
    <div class="flex items-center justify-between mb-6">
      <h3 class="text-2xl font-bold text-gray-900 dark:text-white">{t('analytics.charts.networkComp.title')}</h3>
      {#if !networkCompLoaded}
        <button
          onclick={loadNetworkComp}
          disabled={networkCompLoading}
          class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg transition"
        >
          {networkCompLoading ? t('analytics.loading') : t('analytics.loadButton')}
        </button>
      {/if}
    </div>
    {#if networkCompError}
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-600 dark:text-red-400 text-sm">{networkCompError}</p>
      </div>
    {:else if networkCompLoaded}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.networkComp.passengerNetwork')}</h4>
          {#if networkData}
            <NetworkGraph data={networkData} />
          {/if}
        </div>
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.networkComp.passengerMetrics')}</h4>
          <ComparativeChart data={comparativeData} metric="flights" />
        </div>
      </div>
    {/if}
  </div>

  <!-- Section: Aircraft & Costs -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
    <div class="flex items-center justify-between mb-6">
      <h3 class="text-2xl font-bold text-gray-900 dark:text-white">{t('analytics.charts.aircraftCost.title')}</h3>
      {#if !aircraftCostLoaded}
        <button
          onclick={loadAircraftCost}
          disabled={aircraftCostLoading}
          class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg transition"
        >
          {aircraftCostLoading ? t('analytics.loading') : t('analytics.loadButton')}
        </button>
      {/if}
    </div>
    {#if aircraftCostError}
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-600 dark:text-red-400 text-sm">{aircraftCostError}</p>
      </div>
    {:else if aircraftCostLoaded}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.aircraftCost.aircraftUtilization')}</h4>
          <AircraftUtilizationChart data={aircraftUtilData} />
        </div>
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.aircraftCost.costBreakdown')}</h4>
          <CostBreakdownDonut data={costBreakdownData} />
        </div>
      </div>
    {/if}
  </div>

  <!-- Section: Flight Operations -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
    <div class="flex items-center justify-between mb-6">
      <h3 class="text-2xl font-bold text-gray-900 dark:text-white">{t('analytics.charts.flightOps.title')}</h3>
      {#if !flightOpsLoaded}
        <button
          onclick={loadFlightOps}
          disabled={flightOpsLoading}
          class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg transition"
        >
          {flightOpsLoading ? t('analytics.loading') : t('analytics.loadButton')}
        </button>
      {/if}
    </div>
    {#if flightOpsError}
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-600 dark:text-red-400 text-sm">{flightOpsError}</p>
      </div>
    {:else if flightOpsLoaded}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.flightOps.dayNightStats')}</h4>
          {#if dayNightStats}
            <DayNightRadar stats={dayNightStats} />
          {/if}
        </div>
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.flightOps.longHaulFlights')}</h4>
          <LongHaulMap flights={longHaulFlights} />
        </div>
      </div>
    {/if}
  </div>

  <!-- Section: Pilot Metrics -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
    <div class="flex items-center justify-between mb-6">
      <h3 class="text-2xl font-bold text-gray-900 dark:text-white">{t('analytics.charts.pilotMetrics.title')}</h3>
      {#if !pilotMetricsLoaded}
        <button
          onclick={loadPilotMetrics}
          disabled={pilotMetricsLoading}
          class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg transition"
        >
          {pilotMetricsLoading ? t('analytics.loading') : t('analytics.loadButton')}
        </button>
      {/if}
    </div>
    {#if pilotMetricsError}
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-600 dark:text-red-400 text-sm">{pilotMetricsError}</p>
      </div>
    {:else if pilotMetricsLoaded}
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.pilotMetrics.currencyStatus')}</h4>
          <PilotCurrencyDashboard items={currencyItems} />
        </div>
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
          <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{t('analytics.charts.pilotMetrics.monthlyCostTrend')}</h4>
          <MonthlyCostTrend data={monthlyCostData} />
        </div>
      </div>
    {/if}
  </div>

  <!-- Section: Runway Risk Assessment -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
    <div class="flex items-center justify-between mb-6">
      <h3 class="text-2xl font-bold text-gray-900 dark:text-white">{t('analytics.charts.runwayRisk.title')}</h3>
      {#if !runwayRiskLoaded}
        <button
          onclick={loadRunwayRisk}
          disabled={runwayRiskLoading}
          class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg transition"
        >
          {runwayRiskLoading ? t('analytics.loading') : t('analytics.loadButton')}
        </button>
      {/if}
    </div>
    {#if runwayRiskError}
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-600 dark:text-red-400 text-sm">{runwayRiskError}</p>
      </div>
    {:else if runwayRiskLoaded}
      <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
        <RunwayRiskTreemap data={runwayRiskData} />
      </div>
    {/if}
  </div>
</div>
