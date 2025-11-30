<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';

  // Types matching the Rust identity fusion architecture
  interface PassengerAlias {
    id: string;
    passenger_id: string;
    raw_name: string;
    usage_count: number;
    source_document: string | null;
    match_type: string | null;
    confidence: number;
  }

  interface CanonicalPassenger {
    id: string;
    canonical_name: string;
    notes: string | null;
    total_flights: number;
    alias_count: number;
    first_seen_date: string | null;
    last_seen_date: string | null;
  }

  // Types for aggregated passenger details (alias-aware)
  interface PassengerRoute {
    route: string;
    departure_airport: string;
    arrival_airport: string;
    flight_count: number;
  }

  interface PassengerCompanion {
    name: string;
    flight_count: number;
  }

  interface PassengerDetailsAggregated {
    passenger_id: string;
    canonical_name: string;
    total_flights: number;
    total_distance_km: number;
    top_routes: PassengerRoute[];
    travel_companions: PassengerCompanion[];
    first_flight_date: string | null;
    last_flight_date: string | null;
    total_co2_kg: number;
    avg_co2_per_flight_kg: number;
    carbon_offset_purchased: boolean;
    aliases: string[];
  }

  interface DeletePassengerResult {
    passenger_deleted: boolean;
    aliases_deleted: number;
    flight_links_removed: number;
  }

  interface Props {
    userId: string;
    passengerId: string;
    canonicalName: string;
    onClose: () => void;
    onDeleted?: () => void;
  }

  let { userId, passengerId, canonicalName, onClose, onDeleted }: Props = $props();

  // Core state
  let aliases: PassengerAlias[] = $state([]);
  let details: PassengerDetailsAggregated | null = $state(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let exporting = $state(false);
  let exportSuccess = $state<string | null>(null);

  // Delete state
  let showDeleteConfirm = $state(false);
  let deleting = $state(false);

  // Tab management
  let activeTab = $state<'overview' | 'aliases'>('overview');

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = null;
    try {
      // Load aliases for this canonical passenger
      aliases = await invoke<PassengerAlias[]>('get_passenger_aliases', { passengerId });

      // Load aggregated details - this properly counts stats across ALL aliases
      try {
        details = await invoke<PassengerDetailsAggregated>('get_passenger_details_aggregated', {
          userId,
          passengerId
        });
      } catch (e) {
        console.error('Failed to load aggregated details:', e);
        // If aggregated details fail, create minimal details from aliases
        details = {
          passenger_id: passengerId,
          canonical_name: canonicalName,
          total_flights: aliases.reduce((sum, a) => sum + a.usage_count, 0),
          total_distance_km: 0,
          top_routes: [],
          travel_companions: [],
          first_flight_date: null,
          last_flight_date: null,
          total_co2_kg: 0,
          avg_co2_per_flight_kg: 0,
          carbon_offset_purchased: false,
          aliases: aliases.map(a => a.raw_name)
        };
      }
    } catch (err) {
      console.error('Failed to load passenger data:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  async function exportDossier() {
    if (!details) return;

    try {
      exporting = true;
      error = null;
      exportSuccess = null;

      const filePath = await save({
        defaultPath: `${canonicalName}-Travel-Dossier-${new Date().toISOString().split('T')[0]}.pdf`,
        filters: [{
          name: 'PDF Document',
          extensions: ['pdf']
        }]
      });

      if (!filePath) {
        exporting = false;
        return;
      }

      const result = await invoke<string>('export_passenger_dossier', {
        userId,
        passengerName: canonicalName,
        outputPath: filePath
      });

      exportSuccess = result;
      setTimeout(() => {
        exportSuccess = null;
      }, 5000);
    } catch (err) {
      console.error('Failed to export dossier:', err);
      error = `Export failed: ${err}`;
    } finally {
      exporting = false;
    }
  }

  async function deletePassenger() {
    deleting = true;
    try {
      const result = await invoke<DeletePassengerResult>('delete_passenger', {
        passengerId
      });

      if (result.passenger_deleted) {
        alert(`Deleted "${canonicalName}"\n\nRemoved:\n- ${result.aliases_deleted} aliases\n- ${result.flight_links_removed} flight links`);
        onClose();
        if (onDeleted) {
          onDeleted();
        }
      } else {
        alert('Failed to delete passenger - record not found');
      }
    } catch (err) {
      console.error('Failed to delete passenger:', err);
      alert(`Delete failed: ${err}`);
    } finally {
      deleting = false;
      showDeleteConfirm = false;
    }
  }

  function formatDate(dateStr: string | null) {
    if (!dateStr) return 'N/A';
    return new Date(dateStr).toLocaleDateString();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  // Stats derived from aliases
  const totalUsage = $derived(() => aliases.reduce((sum, a) => sum + a.usage_count, 0));
</script>

<!-- Modal Backdrop -->
<div
  class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
  onclick={handleBackdropClick}
  role="dialog"
>
  <!-- Modal Container -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col">
    {#if loading}
      <div class="p-12 flex items-center justify-center">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    {:else if error}
      <div class="p-12 text-center">
        <div class="text-red-500 text-4xl mb-4">⚠️</div>
        <p class="text-red-600 dark:text-red-400 font-medium">Failed to load passenger details</p>
        <p class="text-gray-600 dark:text-gray-400 text-sm mt-2">{error}</p>
        <button
          onclick={loadData}
          class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg transition"
        >
          Try Again
        </button>
      </div>
    {:else}
      <!-- Header -->
      <div class="border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between bg-gradient-to-r from-primary-50 to-indigo-50 dark:from-primary-900/20 dark:to-indigo-900/20">
        <div>
          <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
            {canonicalName}
          </h2>
          <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
            {aliases.length} {aliases.length === 1 ? 'alias' : 'aliases'} · {totalUsage()} total flights
          </p>
        </div>
        <div class="flex items-center gap-2">
          <button
            onclick={exportDossier}
            disabled={exporting}
            class="bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-700 hover:to-emerald-700 disabled:from-gray-400 disabled:to-gray-500 text-white px-4 py-2 rounded-lg font-medium transition-all flex items-center gap-2 shadow-lg hover:shadow-xl disabled:cursor-not-allowed"
            title="Export as PDF Dossier"
          >
            {#if exporting}
              <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
              <span>Exporting...</span>
            {:else}
              <span>📄</span>
              <span>Export Dossier</span>
            {/if}
          </button>
          <button
            onclick={() => showDeleteConfirm = true}
            class="bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-lg font-medium transition-all flex items-center gap-2"
            title="Delete this passenger"
          >
            <span>🗑️</span>
            <span>Delete</span>
          </button>
          <button
            onclick={onClose}
            class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-2xl font-bold w-8 h-8 flex items-center justify-center rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
            title="Close"
          >
            ×
          </button>
        </div>
      </div>

      {#if exportSuccess}
        <div class="mx-6 mt-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-3 flex items-center gap-2">
          <span class="text-green-600 dark:text-green-400 text-xl">✓</span>
          <p class="text-sm text-green-800 dark:text-green-300">{exportSuccess}</p>
        </div>
      {/if}

      <!-- Tab Navigation -->
      <div class="border-b border-gray-200 dark:border-gray-700 px-6">
        <nav class="flex gap-4" aria-label="Tabs">
          <button
            onclick={() => activeTab = 'overview'}
            class="py-3 px-1 border-b-2 font-medium text-sm transition-colors {activeTab === 'overview'
              ? 'border-primary-500 text-primary-600 dark:text-primary-400'
              : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300 hover:border-gray-300'}"
          >
            📊 Overview
          </button>
          <button
            onclick={() => activeTab = 'aliases'}
            class="py-3 px-1 border-b-2 font-medium text-sm transition-colors {activeTab === 'aliases'
              ? 'border-primary-500 text-primary-600 dark:text-primary-400'
              : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300 hover:border-gray-300'}"
          >
            🏷️ Aliases ({aliases.length})
          </button>
        </nav>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if activeTab === 'overview' && details}
          <!-- Summary Stats -->
          <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
            <div class="bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20 rounded-lg p-4 border border-blue-200 dark:border-blue-800">
              <p class="text-sm text-blue-700 dark:text-blue-300 mb-1">Total Flights</p>
              <p class="text-3xl font-bold text-blue-900 dark:text-blue-100">{details.total_flights}</p>
            </div>

            <div class="bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-900/20 dark:to-emerald-900/20 rounded-lg p-4 border border-green-200 dark:border-green-800">
              <p class="text-sm text-green-700 dark:text-green-300 mb-1">Distance Flown</p>
              <p class="text-3xl font-bold text-green-900 dark:text-green-100">
                {Math.round(details.total_distance_km).toLocaleString()}
              </p>
              <p class="text-xs text-green-600 dark:text-green-400 mt-1">km</p>
            </div>

            <div class="bg-gradient-to-br from-purple-50 to-pink-50 dark:from-purple-900/20 dark:to-pink-900/20 rounded-lg p-4 border border-purple-200 dark:border-purple-800">
              <p class="text-sm text-purple-700 dark:text-purple-300 mb-1">First Flight</p>
              <p class="text-lg font-bold text-purple-900 dark:text-purple-100">
                {formatDate(details.first_flight_date)}
              </p>
            </div>

            <div class="bg-gradient-to-br from-orange-50 to-amber-50 dark:from-orange-900/20 dark:to-amber-900/20 rounded-lg p-4 border border-orange-200 dark:border-orange-800">
              <p class="text-sm text-orange-700 dark:text-orange-300 mb-1">Last Flight</p>
              <p class="text-lg font-bold text-orange-900 dark:text-orange-100">
                {formatDate(details.last_flight_date)}
              </p>
            </div>
          </div>

          <!-- CO2 Environmental Impact -->
          {#if details.total_co2_kg > 0}
            <div class="bg-gradient-to-br from-green-50 via-emerald-50 to-teal-50 dark:from-green-900/20 dark:via-emerald-900/20 dark:to-teal-900/20 rounded-lg border-2 border-green-300 dark:border-green-700 p-6 mb-6">
              <div class="flex items-center gap-3 mb-4">
                <span class="text-3xl">🌍</span>
                <div>
                  <h3 class="text-xl font-bold text-gray-900 dark:text-white">Environmental Impact</h3>
                  <p class="text-sm text-gray-600 dark:text-gray-400">Carbon footprint from travel</p>
                </div>
              </div>

              <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div class="bg-white dark:bg-gray-800 rounded-lg p-4 border border-green-200 dark:border-green-800">
                  <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">Total CO₂ Emissions</p>
                  <p class="text-2xl font-bold text-green-900 dark:text-green-100">
                    {Math.round(details.total_co2_kg).toLocaleString()}
                  </p>
                  <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">kg CO₂</p>
                </div>

                <div class="bg-white dark:bg-gray-800 rounded-lg p-4 border border-green-200 dark:border-green-800">
                  <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">Average per Flight</p>
                  <p class="text-2xl font-bold text-green-900 dark:text-green-100">
                    {Math.round(details.avg_co2_per_flight_kg).toLocaleString()}
                  </p>
                  <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">kg CO₂ per flight</p>
                </div>

                <div class="bg-white dark:bg-gray-800 rounded-lg p-4 border border-green-200 dark:border-green-800">
                  <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">Carbon Offset</p>
                  <p class="text-2xl font-bold">
                    {#if details.carbon_offset_purchased}
                      <span class="text-green-600 dark:text-green-400">✓ Yes</span>
                    {:else}
                      <span class="text-gray-500 dark:text-gray-400">No</span>
                    {/if}
                  </p>
                </div>
              </div>
            </div>
          {/if}

          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- Top Routes -->
            <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
              <div class="px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">🛫 Top Routes</h3>
              </div>
              <div class="p-4">
                {#if details.top_routes.length === 0}
                  <p class="text-gray-500 dark:text-gray-400 text-sm text-center py-8">No routes found</p>
                {:else}
                  <div class="space-y-3">
                    {#each details.top_routes as route, index}
                      <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                        <div class="flex items-center gap-3">
                          {#if index === 0}
                            <span class="text-2xl">🥇</span>
                          {:else if index === 1}
                            <span class="text-2xl">🥈</span>
                          {:else if index === 2}
                            <span class="text-2xl">🥉</span>
                          {:else}
                            <span class="text-sm text-gray-500 dark:text-gray-400 w-8">#{index + 1}</span>
                          {/if}
                          <p class="font-semibold text-gray-900 dark:text-white">
                            {route.route}
                          </p>
                        </div>
                        <span class="text-lg font-bold text-primary-600 dark:text-primary-400">
                          {route.flight_count}x
                        </span>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>

            <!-- Travel Companions -->
            <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
              <div class="px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">👥 Travel Companions</h3>
              </div>
              <div class="p-4 max-h-96 overflow-y-auto">
                {#if details.travel_companions.length === 0}
                  <p class="text-gray-500 dark:text-gray-400 text-sm text-center py-8">No travel companions</p>
                {:else}
                  <div class="space-y-2">
                    {#each details.travel_companions as companion, index}
                      <div class="flex items-center justify-between p-2.5 hover:bg-gray-50 dark:hover:bg-gray-800 rounded-lg transition">
                        <div class="flex items-center gap-2">
                          <span class="w-6 text-sm text-gray-500 dark:text-gray-400">
                            {index + 1}.
                          </span>
                          <p class="text-sm font-medium text-gray-900 dark:text-white">
                            {companion.name}
                          </p>
                        </div>
                        <span class="text-sm font-semibold text-gray-600 dark:text-gray-400">
                          {companion.flight_count} {companion.flight_count === 1 ? 'flight' : 'flights'}
                        </span>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          </div>

        {:else if activeTab === 'aliases'}
          <!-- Aliases Tab Content -->
          <div class="space-y-6">
            <!-- Info banner -->
            <div class="bg-gradient-to-r from-purple-50 to-indigo-50 dark:from-purple-900/20 dark:to-indigo-900/20 rounded-lg p-4 border border-purple-200 dark:border-purple-800">
              <div class="flex items-center gap-3">
                <span class="text-2xl">🔗</span>
                <div>
                  <h4 class="font-semibold text-gray-900 dark:text-white">Canonical Identity</h4>
                  <p class="text-sm text-gray-600 dark:text-gray-400">
                    All these aliases resolve to the canonical name <strong>{canonicalName}</strong>
                  </p>
                </div>
              </div>
            </div>

            <!-- Aliases list -->
            <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
              <div class="px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Known Aliases</h3>
              </div>
              <div class="p-4">
                {#if aliases.length === 0}
                  <p class="text-gray-500 dark:text-gray-400 text-sm text-center py-8">No aliases found</p>
                {:else}
                  <div class="space-y-2">
                    {#each aliases as alias}
                      <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                        <div class="flex items-center gap-3">
                          <span class="font-mono font-bold text-primary-600 dark:text-primary-400">
                            {alias.raw_name}
                          </span>
                          {#if alias.match_type}
                            <span class="text-xs px-2 py-0.5 rounded-full {
                              alias.match_type === 'exact' ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-300' :
                              alias.match_type === 'manual' ? 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300' :
                              'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300'
                            }">
                              {alias.match_type}
                            </span>
                          {/if}
                        </div>
                        <div class="flex items-center gap-4">
                          <span class="text-sm text-gray-600 dark:text-gray-400">
                            {alias.usage_count} {alias.usage_count === 1 ? 'flight' : 'flights'}
                          </span>
                          <span class="text-xs text-gray-500">
                            {Math.round(alias.confidence * 100)}% confidence
                          </span>
                        </div>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>

            <!-- Stats -->
            <div class="grid grid-cols-3 gap-4">
              <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
                <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">Total Aliases</p>
                <p class="text-2xl font-bold text-purple-600 dark:text-purple-400">{aliases.length}</p>
              </div>
              <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
                <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">Total Usage</p>
                <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{totalUsage()}</p>
              </div>
              <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
                <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">Manual Merges</p>
                <p class="text-2xl font-bold text-green-600 dark:text-green-400">
                  {aliases.filter(a => a.match_type === 'manual').length}
                </p>
              </div>
            </div>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900">
        <div class="flex items-center justify-between">
          <p class="text-sm text-gray-600 dark:text-gray-400">
            Canonical passenger: {canonicalName}
          </p>
          <button
            onclick={onClose}
            class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-lg transition"
          >
            Close
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<!-- Delete Confirmation Modal -->
{#if showDeleteConfirm}
  <div
    class="fixed inset-0 bg-black bg-opacity-50 z-[60] flex items-center justify-center p-4"
    onclick={(e) => { if (e.target === e.currentTarget) showDeleteConfirm = false; }}
    role="dialog"
  >
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full p-6">
      <div class="text-center mb-6">
        <div class="text-5xl mb-4">⚠️</div>
        <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-2">
          Delete Passenger?
        </h3>
        <p class="text-gray-600 dark:text-gray-400">
          Are you sure you want to delete <strong>{canonicalName}</strong>?
        </p>
        <p class="text-sm text-gray-500 dark:text-gray-500 mt-2">
          This will remove {aliases.length} {aliases.length === 1 ? 'alias' : 'aliases'} and all flight associations.
          This action cannot be undone.
        </p>
      </div>

      <div class="flex items-center gap-3">
        <button
          onclick={() => showDeleteConfirm = false}
          class="flex-1 px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg font-medium transition"
        >
          Cancel
        </button>
        <button
          onclick={deletePassenger}
          disabled={deleting}
          class="flex-1 px-4 py-2 bg-red-600 hover:bg-red-700 disabled:bg-gray-400 text-white rounded-lg font-medium transition flex items-center justify-center gap-2"
        >
          {#if deleting}
            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
            Deleting...
          {:else}
            🗑️ Delete
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}
