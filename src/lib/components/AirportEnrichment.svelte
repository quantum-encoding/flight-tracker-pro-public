<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface EnrichmentResult {
    total_processed: number;
    enriched_count: number;
    new_airports: number;
    errors: string[];
  }

  let totalAirports = $state(0);
  let missingCoordinates = $state(0);
  let isEnriching = $state(false);
  let enrichmentResult = $state<EnrichmentResult | null>(null);
  let error = $state('');
  let isLoading = $state(true);

  async function loadStats() {
    try {
      isLoading = true;
      error = '';
      totalAirports = await invoke<number>('get_total_airports_count');
      missingCoordinates = await invoke<number>('get_missing_coordinates_count');
    } catch (err) {
      error = `Failed to load statistics: ${err}`;
      console.error(err);
    } finally {
      isLoading = false;
    }
  }

  async function runEnrichment() {
    try {
      isEnriching = true;
      error = '';
      enrichmentResult = null;

      const result = await invoke<EnrichmentResult>('enrich_airport_data');
      enrichmentResult = result;

      // Reload stats after enrichment
      await loadStats();
    } catch (err) {
      error = `Enrichment failed: ${err}`;
      console.error(err);
    } finally {
      isEnriching = false;
    }
  }

  onMount(() => {
    loadStats();
  });
</script>

<div class="space-y-6">
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
    <h3 class="text-xl font-bold mb-4 text-gray-900 dark:text-gray-100">
      Airport Data Enrichment
    </h3>

    <p class="text-sm text-gray-600 dark:text-gray-400 mb-6">
      Populate missing airport coordinates from the OurAirports database. This will enable map visualizations and distance calculations for your flights.
    </p>

    {#if isLoading}
      <div class="flex items-center justify-center py-8">
        <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
      </div>
    {:else}
      <!-- Statistics Cards -->
      <div class="grid grid-cols-2 gap-4 mb-6">
        <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
          <div class="text-2xl font-bold text-gray-900 dark:text-gray-100">
            {totalAirports}
          </div>
          <div class="text-sm text-gray-600 dark:text-gray-400">
            Total Airports
          </div>
        </div>

        <div class="bg-gray-50 dark:bg-gray-700 rounded-lg p-4">
          <div class="text-2xl font-bold text-orange-600 dark:text-orange-400">
            {missingCoordinates}
          </div>
          <div class="text-sm text-gray-600 dark:text-gray-400">
            Missing Coordinates
          </div>
        </div>
      </div>

      <!-- Action Button -->
      <button
        onclick={runEnrichment}
        disabled={isEnriching || missingCoordinates === 0}
        class="w-full bg-gradient-to-r from-blue-600 to-indigo-600 text-white px-6 py-3 rounded-lg font-medium hover:from-blue-700 hover:to-indigo-700 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      >
        {#if isEnriching}
          <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
          <span>Enriching Airport Data...</span>
        {:else if missingCoordinates === 0}
          <span>✓ All Airports Have Coordinates</span>
        {:else}
          <span>🌍 Enrich Airport Data ({missingCoordinates} airports)</span>
        {/if}
      </button>

      {#if error}
        <div class="mt-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
          <div class="flex items-start gap-2">
            <span class="text-red-600 dark:text-red-400 text-xl">⚠️</span>
            <div class="flex-1">
              <p class="text-sm text-red-800 dark:text-red-300 font-medium">Error</p>
              <p class="text-sm text-red-700 dark:text-red-400 mt-1">{error}</p>
            </div>
          </div>
        </div>
      {/if}

      {#if enrichmentResult}
        <div class="mt-6 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-4">
          <div class="flex items-start gap-2 mb-3">
            <span class="text-green-600 dark:text-green-400 text-xl">✓</span>
            <h4 class="text-sm font-bold text-green-800 dark:text-green-300">
              Enrichment Complete
            </h4>
          </div>

          <div class="space-y-2 text-sm">
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">Total Processed:</span>
              <span class="font-medium text-gray-900 dark:text-gray-100">
                {enrichmentResult.total_processed}
              </span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">Coordinates Updated:</span>
              <span class="font-medium text-green-600 dark:text-green-400">
                {enrichmentResult.enriched_count}
              </span>
            </div>
            <div class="flex justify-between">
              <span class="text-gray-600 dark:text-gray-400">New Airports Added:</span>
              <span class="font-medium text-blue-600 dark:text-blue-400">
                {enrichmentResult.new_airports}
              </span>
            </div>
            {#if enrichmentResult.errors.length > 0}
              <div class="mt-3 pt-3 border-t border-green-200 dark:border-green-800">
                <p class="text-orange-600 dark:text-orange-400 font-medium mb-2">
                  Errors ({enrichmentResult.errors.length}):
                </p>
                <div class="max-h-32 overflow-y-auto space-y-1">
                  {#each enrichmentResult.errors as errorMsg}
                    <p class="text-xs text-gray-700 dark:text-gray-300">• {errorMsg}</p>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    {/if}
  </div>

  <!-- Information Panel -->
  <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
    <h4 class="text-sm font-bold text-blue-800 dark:text-blue-300 mb-2">
      📊 About Airport Enrichment
    </h4>
    <ul class="text-xs text-blue-700 dark:text-blue-400 space-y-1">
      <li>• Fetches data from OurAirports (davidmegginson.github.io/ourairports-data)</li>
      <li>• Includes small, medium, and large airports with IATA/ICAO codes</li>
      <li>• Updates existing airports missing coordinates</li>
      <li>• Adds new airports not yet in your database</li>
      <li>• Enables map visualizations and distance calculations</li>
    </ul>
  </div>
</div>
