<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import FlightResearch from './FlightResearch.svelte';

  interface Props {
    flight: any;
    onClose: () => void;
    onInvestigate: () => void;
  }

  let { flight, onClose, onInvestigate }: Props = $props();

  let investigation: any = $state(null);
  let loadingInvestigation = $state(true);
  let showResearch = $state(false);
  let customFields: [string, string, string][] = $state([]);
  let showAddCustomField = $state(false);
  let newFieldName = $state('');
  let newFieldValue = $state('');
  let newFieldType = $state('text');

  async function loadCustomFields() {
    try {
      customFields = await invoke('get_flight_custom_fields', { flightId: flight.id });
    } catch (e) {
      console.error('Failed to load custom fields:', e);
    }
  }

  async function addCustomField() {
    if (!newFieldName.trim() || !newFieldValue.trim()) return;
    try {
      await invoke('set_flight_custom_field', {
        flightId: flight.id,
        fieldName: newFieldName,
        fieldValue: newFieldValue,
        fieldType: newFieldType
      });
      await loadCustomFields();
      newFieldName = '';
      newFieldValue = '';
      showAddCustomField = false;
    } catch (e) {
      console.error('Failed to add custom field:', e);
    }
  }

  async function loadInvestigation() {
    try {
      const result = await invoke('get_flight_investigation', {
        flightId: flight.id
      });
      investigation = result;
    } catch (error) {
      console.error('Error loading investigation:', error);
    } finally {
      loadingInvestigation = false;
    }
  }

  function formatDate(dateStr: string) {
    return new Date(dateStr).toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
  }

  function formatTime(dateStr: string) {
    return new Date(dateStr).toLocaleTimeString('en-US', {
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function calculateFuelEstimate(distanceKm: number): number {
    // Simple estimate: ~3.5 kg/km for typical narrow-body
    const fuelKg = distanceKm * 3.5 * 1.1 + 100;
    return Math.round(fuelKg);
  }

  function fuelKgToGallons(fuelKg: number): number {
    return Math.round(fuelKg * 2.20462 / 6.7);
  }

  onMount(() => {
    loadInvestigation();
    loadCustomFields();
  });
</script>

<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50 p-4">
  <div class="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-4xl w-full max-h-[90vh] overflow-y-auto">
    <!-- Header -->
    <div class="sticky top-0 px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between bg-white dark:bg-gray-800 z-10 rounded-t-xl">
      <div class="flex-1">
        <h3 class="text-xl font-bold text-gray-900 dark:text-white">Flight Details</h3>
        <p class="text-gray-600 dark:text-gray-400 text-sm mt-1">
          {flight.departure_airport} → {flight.arrival_airport}
        </p>
      </div>
      <button
        onclick={onClose}
        class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition text-gray-500 dark:text-gray-400"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <!-- Content -->
    <div class="px-6 py-4 space-y-6">
      <!-- Flight Information -->
      <section>
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
          ✈️ Flight Information
        </h3>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">Date</p>
            <p class="text-sm font-semibold text-gray-900 dark:text-white">
              {formatDate(flight.departure_datetime)}
            </p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">Flight Number</p>
            <p class="text-sm font-semibold text-gray-900 dark:text-white">
              {flight.flight_number || 'N/A'}
            </p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">Departure Time</p>
            <p class="text-sm font-semibold text-gray-900 dark:text-white">
              {formatTime(flight.departure_datetime)}
            </p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">Arrival Time</p>
            <p class="text-sm font-semibold text-gray-900 dark:text-white">
              {flight.arrival_datetime ? formatTime(flight.arrival_datetime) : 'N/A'}
            </p>
          </div>
        </div>
      </section>

      <!-- Journey Statistics -->
      <section>
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
          📊 Journey Statistics
        </h3>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          {#if flight.distance_km}
            <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 text-center">
              <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">
                {Math.round(flight.distance_km).toLocaleString()}
              </p>
              <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">kilometers</p>
            </div>
            <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 text-center">
              <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">
                {Math.round(flight.distance_km / 1.609).toLocaleString()}
              </p>
              <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">miles</p>
            </div>
          {/if}

          {#if flight.flight_duration}
            <div class="bg-purple-50 dark:bg-purple-900/20 border border-purple-200 dark:border-purple-800 rounded-lg p-4 text-center">
              <p class="text-2xl font-bold text-purple-600 dark:text-purple-400">
                {Math.floor(flight.flight_duration / 60)}h {flight.flight_duration % 60}m
              </p>
              <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">flight time</p>
            </div>
          {/if}

          {#if flight.distance_km}
            <div class="bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800 rounded-lg p-4 text-center">
              <p class="text-2xl font-bold text-orange-600 dark:text-orange-400">
                {Math.round(flight.distance_km / (flight.flight_duration || 60) * 60)}
              </p>
              <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">km/h avg speed</p>
            </div>
          {/if}
        </div>
      </section>

      <!-- Environmental Impact -->
      {#if flight.carbon_emissions_kg || flight.distance_km}
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
            🌱 Environmental Impact
          </h3>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            {#if flight.carbon_emissions_kg}
              <div class="bg-green-50 dark:bg-green-900/20 border-2 border-green-200 dark:border-green-800 rounded-lg p-4">
                <p class="text-xs text-green-700 dark:text-green-300 mb-1">CO₂ Emissions</p>
                <p class="text-2xl font-bold text-green-900 dark:text-green-100">
                  {Math.round(flight.carbon_emissions_kg).toLocaleString()} kg
                </p>
                <p class="text-xs text-green-600 dark:text-green-400 mt-1">
                  {(flight.carbon_emissions_kg / 1000).toFixed(2)} tonnes
                </p>
              </div>
            {/if}

            {#if flight.distance_km}
              <div class="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4">
                <p class="text-xs text-amber-700 dark:text-amber-300 mb-1">Estimated Fuel</p>
                <p class="text-2xl font-bold text-amber-900 dark:text-amber-100">
                  {calculateFuelEstimate(flight.distance_km).toLocaleString()} kg
                </p>
                <p class="text-xs text-amber-600 dark:text-amber-400 mt-1">
                  ~{fuelKgToGallons(calculateFuelEstimate(flight.distance_km)).toLocaleString()} gallons
                </p>
              </div>

              <div class="bg-teal-50 dark:bg-teal-900/20 border border-teal-200 dark:border-teal-800 rounded-lg p-4">
                <p class="text-xs text-teal-700 dark:text-teal-300 mb-1">Carbon Equivalent</p>
                <p class="text-xl font-bold text-teal-900 dark:text-teal-100">
                  {Math.round((flight.carbon_emissions_kg || 0) / 21).toLocaleString()}
                </p>
                <p class="text-xs text-teal-600 dark:text-teal-400 mt-1">
                  trees needed for 1 year
                </p>
              </div>
            {/if}
          </div>
        </section>
      {/if}

      <!-- Passenger Notes -->
      {#if flight.notes}
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
            📝 Notes
          </h3>
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <p class="text-sm text-gray-700 dark:text-gray-300">{flight.notes}</p>
          </div>
        </section>
      {/if}

      <!-- Custom Fields -->
      <section>
        <div class="flex items-center justify-between mb-3">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white flex items-center gap-2">
            🏷️ Custom Fields
          </h3>
          <button
            onclick={() => showAddCustomField = !showAddCustomField}
            class="text-sm text-indigo-600 hover:text-indigo-800 dark:text-indigo-400"
          >
            + Add Field
          </button>
        </div>

        {#if showAddCustomField}
          <div class="bg-indigo-50 dark:bg-indigo-900/20 rounded-lg p-4 mb-3">
            <div class="grid grid-cols-3 gap-2 mb-2">
              <input
                type="text"
                bind:value={newFieldName}
                placeholder="Field name"
                class="px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white text-sm"
              />
              <input
                type="text"
                bind:value={newFieldValue}
                placeholder="Value"
                class="px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white text-sm"
              />
              <select
                bind:value={newFieldType}
                class="px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white text-sm"
              >
                <option value="text">Text</option>
                <option value="number">Number</option>
                <option value="date">Date</option>
                <option value="currency">Currency</option>
              </select>
            </div>
            <button
              onclick={addCustomField}
              class="px-3 py-1 bg-indigo-600 text-white rounded text-sm hover:bg-indigo-700"
            >
              Save
            </button>
          </div>
        {/if}

        {#if customFields.length > 0}
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg divide-y dark:divide-gray-700">
            {#each customFields as [name, value, type]}
              <div class="p-3 flex justify-between items-center">
                <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{name}</span>
                <span class="text-sm text-gray-900 dark:text-white">
                  {#if type === 'currency'}${value}{:else}{value}{/if}
                </span>
              </div>
            {/each}
          </div>
        {:else if !showAddCustomField}
          <p class="text-sm text-gray-500 dark:text-gray-400">No custom fields. Click "Add Field" to add your own data.</p>
        {/if}
      </section>

      <!-- Investigation Results -->
      <section>
        <div class="flex items-center justify-between mb-3">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white flex items-center gap-2">
            🔍 Investigation Results
          </h3>
          <button
            onclick={() => showResearch = true}
            class="bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded-lg text-sm font-medium transition"
          >
            🚀 DeepSeek Research
          </button>
        </div>

        {#if loadingInvestigation}
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-8 text-center">
            <div class="animate-spin text-4xl mb-2">⚙️</div>
            <p class="text-sm text-gray-600 dark:text-gray-400">Checking for investigations...</p>
          </div>
        {:else if investigation}
          <div class="bg-indigo-50 dark:bg-indigo-900/20 border-2 border-indigo-200 dark:border-indigo-800 rounded-lg p-4">
            <div class="flex items-start justify-between mb-3">
              <div>
                <p class="text-sm font-semibold text-indigo-900 dark:text-indigo-100">
                  Investigation Found
                </p>
                <p class="text-xs text-indigo-600 dark:text-indigo-400 mt-1">
                  Completed on {new Date(investigation.created_at).toLocaleDateString()}
                </p>
              </div>
              {#if investigation.corroboration_score}
                <div class="text-right">
                  <p class="text-2xl font-bold {
                    investigation.corroboration_score >= 0.7 ? 'text-green-600 dark:text-green-400' :
                    investigation.corroboration_score >= 0.4 ? 'text-yellow-600 dark:text-yellow-400' :
                    'text-red-600 dark:text-red-400'
                  }">
                    {Math.round(investigation.corroboration_score * 100)}%
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400">confidence</p>
                </div>
              {/if}
            </div>

            {#if investigation.ai_summary}
              <div class="bg-white dark:bg-gray-800 rounded p-3 mb-3">
                <p class="text-sm text-gray-700 dark:text-gray-300 line-clamp-3">
                  {investigation.ai_summary}
                </p>
              </div>
            {/if}

            {#if investigation.sources_json}
              {@const sources = JSON.parse(investigation.sources_json)}
              {#if sources && sources.length > 0}
                <div class="space-y-2 mb-3">
                  <p class="text-xs font-semibold text-indigo-700 dark:text-indigo-300">
                    📎 Sources Found ({sources.length})
                  </p>
                  {#each sources.slice(0, 3) as source}
                    <a
                      href={source.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      class="block bg-white dark:bg-gray-800 rounded p-2 hover:bg-indigo-100 dark:hover:bg-indigo-900 transition text-xs"
                    >
                      <p class="font-semibold text-indigo-600 dark:text-indigo-400 truncate">
                        {source.title}
                      </p>
                      <p class="text-gray-500 dark:text-gray-400 truncate mt-1">
                        {source.url}
                      </p>
                    </a>
                  {/each}
                  {#if sources.length > 3}
                    <p class="text-xs text-indigo-600 dark:text-indigo-400 text-center">
                      + {sources.length - 3} more sources
                    </p>
                  {/if}
                </div>
              {/if}
            {/if}

            <button
              onclick={onInvestigate}
              class="w-full bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-lg text-sm font-medium transition"
            >
              🔍 View Full Investigation
            </button>
          </div>
        {:else}
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-6 text-center">
            <div class="text-4xl mb-2">🔍</div>
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">
              No investigation has been run for this flight yet
            </p>
            <button
              onclick={onInvestigate}
              class="bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-lg text-sm font-medium transition"
            >
              🔍 Investigate with AI
            </button>
          </div>
        {/if}
      </section>

      <!-- Additional Details -->
      {#if flight.aircraft_registration || flight.seat_number || flight.booking_reference}
        <section>
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
            ℹ️ Additional Details
          </h3>
          <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
            {#if flight.aircraft_registration}
              <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3">
                <p class="text-xs text-gray-500 dark:text-gray-400">Aircraft</p>
                <p class="text-sm font-semibold text-gray-900 dark:text-white">
                  {flight.aircraft_registration}
                </p>
              </div>
            {/if}
            {#if flight.seat_number}
              <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3">
                <p class="text-xs text-gray-500 dark:text-gray-400">Seat</p>
                <p class="text-sm font-semibold text-gray-900 dark:text-white">
                  {flight.seat_number}
                </p>
              </div>
            {/if}
            {#if flight.booking_reference}
              <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-3">
                <p class="text-xs text-gray-500 dark:text-gray-400">Booking Ref</p>
                <p class="text-sm font-semibold text-gray-900 dark:text-white">
                  {flight.booking_reference}
                </p>
              </div>
            {/if}
          </div>
        </section>
      {/if}
    </div>

    <!-- Footer -->
    <div class="sticky bottom-0 bg-gray-50 dark:bg-gray-900/50 px-6 py-4 border-t border-gray-200 dark:border-gray-700 rounded-b-xl flex justify-end">
      <button
        onclick={onClose}
        class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600 rounded-lg transition font-medium"
      >
        Close
      </button>
    </div>
  </div>
</div>

<!-- DeepSeek Research Modal -->
{#if showResearch}
  <FlightResearch
    flightId={flight.id}
    onClose={() => showResearch = false}
  />
{/if}
