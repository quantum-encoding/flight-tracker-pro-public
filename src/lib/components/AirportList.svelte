<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface AirportVisit {
    airport_code: string;
    visit_count: number;
    departure_count: number;
    arrival_count: number;
  }

  interface Props {
    userId: string;
    onClose: () => void;
  }

  let { userId, onClose }: Props = $props();

  let airports: AirportVisit[] = $state([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let searchQuery = $state('');
  let sortBy = $state<'code' | 'visits' | 'departures' | 'arrivals'>('visits');
  let sortOrder = $state<'asc' | 'desc'>('desc');

  onMount(async () => {
    await loadAirports();
  });

  async function loadAirports() {
    loading = true;
    error = null;
    try {
      airports = await invoke('get_airport_list', { userId });
    } catch (err) {
      console.error('Failed to load airports:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  const filteredAirports = $derived(() => {
    let filtered = airports;

    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(a =>
        a.airport_code.toLowerCase().includes(query)
      );
    }

    // Apply sorting
    const sorted = [...filtered].sort((a, b) => {
      let compareValue = 0;

      switch (sortBy) {
        case 'code':
          compareValue = a.airport_code.localeCompare(b.airport_code);
          break;
        case 'visits':
          compareValue = a.visit_count - b.visit_count;
          break;
        case 'departures':
          compareValue = a.departure_count - b.departure_count;
          break;
        case 'arrivals':
          compareValue = a.arrival_count - b.arrival_count;
          break;
      }

      return sortOrder === 'asc' ? compareValue : -compareValue;
    });

    return sorted;
  });

  function toggleSort(column: typeof sortBy) {
    if (sortBy === column) {
      sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = column;
      sortOrder = 'desc';
    }
  }

  function getSortIcon(column: typeof sortBy): string {
    if (sortBy !== column) return '⇅';
    return sortOrder === 'asc' ? '↑' : '↓';
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }
</script>

<!-- Modal Backdrop -->
<div
  class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
  onclick={handleBackdropClick}
  role="button"
  tabindex="0"
>
  <!-- Modal Container -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-4xl w-full max-h-[90vh] overflow-hidden flex flex-col">
    <!-- Header -->
    <div class="border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between">
      <div>
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Airport Visits</h2>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
          All airports you've visited across {airports.length} locations
        </p>
      </div>
      <button
        onclick={onClose}
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-2xl font-bold w-8 h-8 flex items-center justify-center rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
        title="Close"
      >
        ×
      </button>
    </div>

    <!-- Search Bar -->
    <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search airports (e.g., JFK, LAX, LHR)..."
        class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100 placeholder-gray-500 dark:placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
      />
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto">
      {#if loading}
        <div class="flex items-center justify-center py-12">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
        </div>
      {:else if error}
        <div class="px-6 py-12 text-center">
          <div class="text-red-500 text-4xl mb-4">⚠️</div>
          <p class="text-red-600 dark:text-red-400 font-medium">Failed to load airports</p>
          <p class="text-gray-600 dark:text-gray-400 text-sm mt-2">{error}</p>
          <button
            onclick={loadAirports}
            class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg transition"
          >
            Try Again
          </button>
        </div>
      {:else if filteredAirports().length === 0}
        <div class="px-6 py-12 text-center">
          <div class="text-gray-400 text-4xl mb-4">🔍</div>
          <p class="text-gray-600 dark:text-gray-400">
            {searchQuery ? 'No airports match your search' : 'No airports found'}
          </p>
        </div>
      {:else}
        <table class="w-full">
          <thead class="bg-gray-50 dark:bg-gray-900 sticky top-0">
            <tr>
              <th
                class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                onclick={() => toggleSort('code')}
              >
                <div class="flex items-center gap-2">
                  <span>Airport Code</span>
                  <span class="text-gray-400">{getSortIcon('code')}</span>
                </div>
              </th>
              <th
                class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                onclick={() => toggleSort('visits')}
              >
                <div class="flex items-center gap-2">
                  <span>Total Visits</span>
                  <span class="text-gray-400">{getSortIcon('visits')}</span>
                </div>
              </th>
              <th
                class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                onclick={() => toggleSort('departures')}
              >
                <div class="flex items-center gap-2">
                  <span>Departures</span>
                  <span class="text-gray-400">{getSortIcon('departures')}</span>
                </div>
              </th>
              <th
                class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                onclick={() => toggleSort('arrivals')}
              >
                <div class="flex items-center gap-2">
                  <span>Arrivals</span>
                  <span class="text-gray-400">{getSortIcon('arrivals')}</span>
                </div>
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
            {#each filteredAirports() as airport}
              <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="flex items-center">
                    <span class="text-lg font-bold text-gray-900 dark:text-white font-mono">
                      {airport.airport_code}
                    </span>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="flex items-center gap-2">
                    <span class="text-2xl font-bold text-primary-600 dark:text-primary-400">
                      {airport.visit_count}
                    </span>
                    <span class="text-xs text-gray-500 dark:text-gray-400">
                      visit{airport.visit_count !== 1 ? 's' : ''}
                    </span>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="flex items-center gap-2">
                    <span class="text-lg font-semibold text-blue-600 dark:text-blue-400">
                      {airport.departure_count}
                    </span>
                    <span class="text-xs text-gray-500 dark:text-gray-400">✈️</span>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="flex items-center gap-2">
                    <span class="text-lg font-semibold text-green-600 dark:text-green-400">
                      {airport.arrival_count}
                    </span>
                    <span class="text-xs text-gray-500 dark:text-gray-400">🛬</span>
                  </div>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <!-- Footer with Summary -->
    {#if !loading && !error && airports.length > 0}
      <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900">
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
          <div>
            <p class="text-gray-600 dark:text-gray-400">Total Airports</p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">{airports.length}</p>
          </div>
          <div>
            <p class="text-gray-600 dark:text-gray-400">Total Departures</p>
            <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">
              {airports.reduce((sum, a) => sum + a.departure_count, 0)}
            </p>
          </div>
          <div>
            <p class="text-gray-600 dark:text-gray-400">Total Arrivals</p>
            <p class="text-2xl font-bold text-green-600 dark:text-green-400">
              {airports.reduce((sum, a) => sum + a.arrival_count, 0)}
            </p>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>
