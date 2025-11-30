<!-- FuelTracker.svelte - AI-powered fuel price search and tracking -->
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { translations } from '$lib/i18n';
  import { theme } from '$lib/theme';

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  // Search state
  let searchQuery = $state('');
  let fuelType = $state('jet_a');
  let region = $state('');
  let searching = $state(false);
  let searchError = $state<string | null>(null);
  let searchResults = $state<any | null>(null);

  // Cached prices
  let cachedPrices = $state<any[]>([]);
  let loadingCached = $state(false);

  // Fuel entries
  let fuelEntries = $state<any[]>([]);
  let fuelStats = $state<any | null>(null);
  let loadingEntries = $state(false);

  // Custom fuel types
  let fuelTypes = $state<any[]>([]);
  let showAddFuelType = $state(false);
  let newFuelType = $state({ code: '', name: '', description: '' });
  let addingFuelType = $state(false);

  // New entry form
  let showAddForm = $state(false);
  let newEntry = $state({
    airport_code: '',
    location_name: '',
    fuel_type: 'jet_a',
    gallons: 0,
    price_per_gallon: 0,
    purchase_date: new Date().toISOString().split('T')[0],
    fbo_name: '',
    notes: ''
  });
  let addingEntry = $state(false);

  // Tab state
  let activeTab = $state<'search' | 'entries' | 'prices'>('search');

  async function searchFuelPrices() {
    if (!searchQuery.trim()) return;

    searching = true;
    searchError = null;
    searchResults = null;

    try {
      const result = await invoke('search_fuel_prices', {
        request: {
          query: searchQuery,
          fuel_type: fuelType,
          region: region || null
        }
      });
      searchResults = result;
      // Refresh cached prices after search
      await loadCachedPrices();
    } catch (err) {
      searchError = String(err);
    } finally {
      searching = false;
    }
  }

  async function loadCachedPrices() {
    loadingCached = true;
    try {
      cachedPrices = await invoke('get_cached_fuel_prices', {
        fuelType: null,
        region: null,
        limit: 50
      });
    } catch (err) {
      console.error('Failed to load cached prices:', err);
    } finally {
      loadingCached = false;
    }
  }

  async function loadFuelEntries() {
    loadingEntries = true;
    try {
      const [entries, stats] = await Promise.all([
        invoke('get_fuel_entries', { userId, limit: 50 }),
        invoke('get_fuel_stats', { userId })
      ]);
      fuelEntries = entries as any[];
      fuelStats = stats;
    } catch (err) {
      console.error('Failed to load fuel entries:', err);
    } finally {
      loadingEntries = false;
    }
  }

  async function addFuelEntry() {
    if (newEntry.gallons <= 0 || newEntry.price_per_gallon <= 0) {
      return;
    }

    addingEntry = true;
    try {
      await invoke('add_fuel_entry', {
        entry: {
          user_id: userId,
          flight_id: null,
          aircraft_id: null,
          airport_code: newEntry.airport_code || null,
          location_name: newEntry.location_name || null,
          fuel_type: newEntry.fuel_type,
          gallons: newEntry.gallons,
          price_per_gallon: newEntry.price_per_gallon,
          currency: 'USD',
          purchase_date: newEntry.purchase_date,
          fbo_name: newEntry.fbo_name || null,
          receipt_number: null,
          notes: newEntry.notes || null
        }
      });

      // Reset form and reload
      newEntry = {
        airport_code: '',
        location_name: '',
        fuel_type: 'jet_a',
        gallons: 0,
        price_per_gallon: 0,
        purchase_date: new Date().toISOString().split('T')[0],
        fbo_name: '',
        notes: ''
      };
      showAddForm = false;
      await loadFuelEntries();
    } catch (err) {
      console.error('Failed to add fuel entry:', err);
    } finally {
      addingEntry = false;
    }
  }

  async function deleteFuelEntry(entryId: string) {
    if (!confirm('Delete this fuel entry?')) return;

    try {
      await invoke('delete_fuel_entry', { entryId });
      await loadFuelEntries();
    } catch (err) {
      console.error('Failed to delete entry:', err);
    }
  }

  async function loadFuelTypes() {
    try {
      fuelTypes = await invoke('get_fuel_types', { userId });
    } catch (err) {
      console.error('Failed to load fuel types:', err);
    }
  }

  async function addCustomFuelType() {
    if (!newFuelType.code.trim() || !newFuelType.name.trim()) return;

    addingFuelType = true;
    try {
      await invoke('add_fuel_type', {
        userId,
        fuelType: {
          code: newFuelType.code,
          name: newFuelType.name,
          description: newFuelType.description || null,
          category: 'aviation'
        }
      });

      newFuelType = { code: '', name: '', description: '' };
      showAddFuelType = false;
      await loadFuelTypes();
    } catch (err) {
      console.error('Failed to add fuel type:', err);
      alert(String(err));
    } finally {
      addingFuelType = false;
    }
  }

  async function deleteFuelType(fuelTypeId: string) {
    if (!confirm('Delete this custom fuel type?')) return;

    try {
      await invoke('delete_fuel_type', { fuelTypeId });
      await loadFuelTypes();
    } catch (err) {
      console.error('Failed to delete fuel type:', err);
    }
  }

  function formatCurrency(amount: number): string {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD'
    }).format(amount);
  }

  function formatDate(dateStr: string): string {
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric'
    });
  }

  function getFuelTypeLabel(type: string): string {
    // First check custom types
    const customType = fuelTypes.find(ft => ft.code === type);
    if (customType) return customType.name;

    // Default types
    switch (type) {
      case 'jet_a': return 'Jet-A';
      case 'avgas_100ll': return '100LL Avgas';
      case 'mogas': return 'Mogas';
      default: return type;
    }
  }

  function getConfidenceColor(confidence: string): string {
    switch (confidence) {
      case 'high': return 'text-green-600 dark:text-green-400';
      case 'medium': return 'text-yellow-600 dark:text-yellow-400';
      case 'low': return 'text-red-600 dark:text-red-400';
      default: return 'text-gray-600 dark:text-gray-400';
    }
  }

  onMount(() => {
    loadCachedPrices();
    loadFuelEntries();
    loadFuelTypes();
  });
</script>

<div class="fuel-tracker-container space-y-6 {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}">
  <!-- Header with Stats -->
  {#if fuelStats}
    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
      <div class="fuel-stat-card fuel-stat-primary rounded-xl p-4">
        <p class="fuel-stat-label text-sm">{$translations('fuel.stats.totalSpent')}</p>
        <p class="fuel-stat-value text-2xl font-bold">{formatCurrency(fuelStats.total_spent)}</p>
      </div>
      <div class="fuel-stat-card fuel-stat-secondary rounded-xl p-4">
        <p class="fuel-stat-label text-sm">{$translations('fuel.stats.totalGallons')}</p>
        <p class="fuel-stat-value text-2xl font-bold">{fuelStats.total_gallons.toFixed(1)}</p>
      </div>
      <div class="fuel-stat-card fuel-stat-tertiary rounded-xl p-4">
        <p class="fuel-stat-label text-sm">{$translations('fuel.stats.avgPrice')}</p>
        <p class="fuel-stat-value text-2xl font-bold">{formatCurrency(fuelStats.avg_price_per_gallon)}</p>
      </div>
      <div class="fuel-stat-card fuel-stat-quaternary rounded-xl p-4">
        <p class="fuel-stat-label text-sm">{$translations('fuel.stats.entries')}</p>
        <p class="fuel-stat-value text-2xl font-bold">{fuelStats.entry_count}</p>
      </div>
    </div>
  {/if}

  <!-- Tab Navigation -->
  <div class="fuel-tab-nav flex space-x-2">
    <button
      onclick={() => activeTab = 'search'}
      class="fuel-tab px-4 py-2 font-medium transition-all duration-200 {activeTab === 'search' ? 'fuel-tab-active' : ''}"
    >
      {$translations('fuel.tabs.search')}
    </button>
    <button
      onclick={() => activeTab = 'entries'}
      class="fuel-tab px-4 py-2 font-medium transition-all duration-200 {activeTab === 'entries' ? 'fuel-tab-active' : ''}"
    >
      {$translations('fuel.tabs.entries')}
    </button>
    <button
      onclick={() => activeTab = 'prices'}
      class="fuel-tab px-4 py-2 font-medium transition-all duration-200 {activeTab === 'prices' ? 'fuel-tab-active' : ''}"
    >
      {$translations('fuel.tabs.prices')}
    </button>
  </div>

  <!-- Search Tab -->
  {#if activeTab === 'search'}
    <div class="fuel-panel rounded-xl p-6">
      <h3 class="fuel-panel-title text-xl font-bold mb-4">AI Fuel Price Search</h3>
      <p class="fuel-panel-subtitle text-sm mb-4">
        Search for current fuel prices by location, airport, or region. Results are cached to build your price database.
      </p>

      <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
        <div class="md:col-span-2">
          <label for="fuel-search-query" class="fuel-label block text-sm font-medium mb-1">Search Query</label>
          <input
            id="fuel-search-query"
            type="text"
            bind:value={searchQuery}
            placeholder="e.g., 'Jet fuel at KSFO' or 'Avgas prices in Texas'"
            class="fuel-input w-full px-4 py-2 rounded-lg"
            onkeydown={(e) => e.key === 'Enter' && searchFuelPrices()}
          />
        </div>
        <div>
          <label for="fuel-type-select" class="fuel-label block text-sm font-medium mb-1">Fuel Type</label>
          <select
            id="fuel-type-select"
            bind:value={fuelType}
            class="fuel-input w-full px-4 py-2 rounded-lg"
          >
            {#each fuelTypes as ft}
              <option value={ft.code}>{ft.name}</option>
            {/each}
          </select>
        </div>
        <div>
          <label for="fuel-region" class="fuel-label block text-sm font-medium mb-1">Region (optional)</label>
          <input
            id="fuel-region"
            type="text"
            bind:value={region}
            placeholder="e.g., California, Europe"
            class="fuel-input w-full px-4 py-2 rounded-lg"
          />
        </div>
      </div>

      <button
        onclick={searchFuelPrices}
        disabled={searching || !searchQuery.trim()}
        class="fuel-btn-primary px-6 py-2 rounded-lg font-medium transition-all duration-200 disabled:opacity-50"
      >
        {searching ? $translations('fuel.search.searching') : $translations('fuel.search.button')}
      </button>

      {#if searchError}
        <div class="mt-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
          <p class="text-red-600 dark:text-red-400">{searchError}</p>
        </div>
      {/if}

      {#if searchResults}
        <div class="mt-6 space-y-4">
          <!-- AI Summary -->
          {#if searchResults.ai_summary}
            <div class="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
              <h4 class="font-semibold text-blue-800 dark:text-blue-300 mb-2">AI Summary</h4>
              <p class="text-blue-700 dark:text-blue-400 whitespace-pre-wrap">{searchResults.ai_summary}</p>
            </div>
          {/if}

          <!-- Price Results -->
          {#if searchResults.prices && searchResults.prices.length > 0}
            <div class="overflow-x-auto">
              <table class="w-full text-sm">
                <thead class="bg-gray-100 dark:bg-gray-700">
                  <tr>
                    <th class="px-4 py-2 text-left">Location</th>
                    <th class="px-4 py-2 text-left">Airport</th>
                    <th class="px-4 py-2 text-right">Price/Gallon</th>
                    <th class="px-4 py-2 text-left">Date</th>
                    <th class="px-4 py-2 text-left">Source</th>
                    <th class="px-4 py-2 text-center">Confidence</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-200 dark:divide-gray-600">
                  {#each searchResults.prices as price}
                    <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                      <td class="px-4 py-3">
                        <div class="font-medium">{price.location_name}</div>
                        {#if price.region}
                          <div class="text-xs text-gray-500">{price.region}, {price.country || ''}</div>
                        {/if}
                      </td>
                      <td class="px-4 py-3 font-mono">{price.airport_code || '-'}</td>
                      <td class="px-4 py-3 text-right font-bold text-green-600 dark:text-green-400">
                        {formatCurrency(price.price_per_gallon)}
                      </td>
                      <td class="px-4 py-3 text-gray-600 dark:text-gray-400">{price.effective_date}</td>
                      <td class="px-4 py-3 text-xs text-gray-500">{price.source || 'AI Research'}</td>
                      <td class="px-4 py-3 text-center">
                        <span class="px-2 py-1 rounded text-xs font-medium {getConfidenceColor(price.confidence)}">
                          {price.confidence}
                        </span>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}

          <!-- Sources -->
          {#if searchResults.sources && searchResults.sources.length > 0}
            <div class="text-sm text-gray-500 dark:text-gray-400">
              <strong>Sources:</strong> {searchResults.sources.join(', ')}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Fuel Entries Tab -->
  {#if activeTab === 'entries'}
    <div class="fuel-panel rounded-xl p-6">
      <div class="flex justify-between items-center mb-4">
        <h3 class="fuel-panel-title text-xl font-bold">{$translations('fuel.tabs.entries')}</h3>
        <button
          onclick={() => showAddForm = !showAddForm}
          class="fuel-btn-secondary px-4 py-2 rounded-lg font-medium transition-all duration-200"
        >
          {showAddForm ? $translations('common.cancel') : '+ ' + $translations('fuel.entry.add')}
        </button>
      </div>

      <!-- Add Entry Form -->
      {#if showAddForm}
        <div class="mb-6 p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
          <h4 class="font-semibold mb-4">{$translations('fuel.entry.add')}</h4>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div>
              <label for="fuel-entry-airport" class="block text-sm font-medium mb-1">{$translations('fuel.entry.airport')}</label>
              <input id="fuel-entry-airport" type="text" bind:value={newEntry.airport_code} placeholder="KSFO" class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500" />
            </div>
            <div>
              <label for="fuel-entry-fbo" class="block text-sm font-medium mb-1">{$translations('fuel.entry.fboName')}</label>
              <input id="fuel-entry-fbo" type="text" bind:value={newEntry.fbo_name} placeholder="Signature Flight" class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500" />
            </div>
            <div>
              <label for="fuel-entry-type" class="block text-sm font-medium mb-1">{$translations('fuel.entry.fuelType')}</label>
              <select id="fuel-entry-type" bind:value={newEntry.fuel_type} class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500">
                {#each fuelTypes as ft}
                  <option value={ft.code}>{ft.name}</option>
                {/each}
              </select>
            </div>
            <div>
              <label for="fuel-entry-gallons" class="block text-sm font-medium mb-1">{$translations('fuel.entry.gallons')}</label>
              <input id="fuel-entry-gallons" type="number" bind:value={newEntry.gallons} step="0.1" min="0" class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500" />
            </div>
            <div>
              <label for="fuel-entry-price" class="block text-sm font-medium mb-1">{$translations('fuel.entry.pricePerGallon')}</label>
              <input id="fuel-entry-price" type="number" bind:value={newEntry.price_per_gallon} step="0.01" min="0" class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500" />
            </div>
            <div>
              <label for="fuel-entry-date" class="block text-sm font-medium mb-1">{$translations('fuel.entry.purchaseDate')}</label>
              <input id="fuel-entry-date" type="date" bind:value={newEntry.purchase_date} class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500" />
            </div>
            <div class="md:col-span-2">
              <label for="fuel-entry-notes" class="block text-sm font-medium mb-1">{$translations('fuel.entry.notes')}</label>
              <input id="fuel-entry-notes" type="text" bind:value={newEntry.notes} placeholder="Optional notes" class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500" />
            </div>
            <div class="flex items-end">
              <button
                onclick={addFuelEntry}
                disabled={addingEntry || newEntry.gallons <= 0 || newEntry.price_per_gallon <= 0}
                class="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded font-medium"
              >
                {addingEntry ? 'Adding...' : `Add (${formatCurrency(newEntry.gallons * newEntry.price_per_gallon)})`}
              </button>
            </div>
          </div>
        </div>
      {/if}

      <!-- Entries List -->
      {#if loadingEntries}
        <div class="text-center py-8 text-gray-500">{$translations('common.loading')}</div>
      {:else if fuelEntries.length === 0}
        <div class="text-center py-8 text-gray-500">
          <p>{$translations('fuel.empty.entries')}</p>
          <p class="text-sm mt-2">{$translations('fuel.empty.addFirst')}</p>
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead class="bg-gray-100 dark:bg-gray-700">
              <tr>
                <th class="px-4 py-2 text-left">Date</th>
                <th class="px-4 py-2 text-left">Location</th>
                <th class="px-4 py-2 text-left">Fuel</th>
                <th class="px-4 py-2 text-right">Gallons</th>
                <th class="px-4 py-2 text-right">$/Gal</th>
                <th class="px-4 py-2 text-right">Total</th>
                <th class="px-4 py-2"></th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200 dark:divide-gray-600">
              {#each fuelEntries as entry}
                <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                  <td class="px-4 py-3">{formatDate(entry.purchase_date)}</td>
                  <td class="px-4 py-3">
                    {#if entry.airport_code}
                      <span class="font-mono">{entry.airport_code}</span>
                    {/if}
                    {#if entry.fbo_name}
                      <div class="text-xs text-gray-500">{entry.fbo_name}</div>
                    {/if}
                  </td>
                  <td class="px-4 py-3">{getFuelTypeLabel(entry.fuel_type)}</td>
                  <td class="px-4 py-3 text-right">{entry.gallons.toFixed(1)}</td>
                  <td class="px-4 py-3 text-right">{formatCurrency(entry.price_per_gallon)}</td>
                  <td class="px-4 py-3 text-right font-bold text-green-600 dark:text-green-400">
                    {formatCurrency(entry.total_cost)}
                  </td>
                  <td class="px-4 py-3">
                    <button
                      onclick={() => deleteFuelEntry(entry.id)}
                      class="text-red-500 hover:text-red-700 text-xs"
                    >
                      {$translations('common.delete')}
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <!-- Custom Fuel Types Section -->
      <div class="mt-8 pt-6 border-t border-gray-200 dark:border-gray-600">
        <div class="flex justify-between items-center mb-4">
          <h4 class="fuel-panel-title text-lg font-semibold">Custom Fuel Types</h4>
          <button
            onclick={() => showAddFuelType = !showAddFuelType}
            class="fuel-btn-secondary px-3 py-1.5 rounded-lg text-sm font-medium transition-all duration-200"
          >
            {showAddFuelType ? 'Cancel' : '+ Add Fuel Type'}
          </button>
        </div>

        <!-- Add Fuel Type Form -->
        {#if showAddFuelType}
          <div class="mb-4 p-4 bg-gray-50 dark:bg-gray-700 rounded-lg">
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div>
                <label for="new-fuel-code" class="block text-sm font-medium mb-1">Code</label>
                <input
                  id="new-fuel-code"
                  type="text"
                  bind:value={newFuelType.code}
                  placeholder="e.g., jet_b, saf"
                  class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500 text-sm"
                />
                <p class="text-xs text-gray-500 mt-1">Lowercase, no spaces</p>
              </div>
              <div>
                <label for="new-fuel-name" class="block text-sm font-medium mb-1">Display Name</label>
                <input
                  id="new-fuel-name"
                  type="text"
                  bind:value={newFuelType.name}
                  placeholder="e.g., Jet-B, SAF"
                  class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500 text-sm"
                />
              </div>
              <div>
                <label for="new-fuel-desc" class="block text-sm font-medium mb-1">Description (optional)</label>
                <input
                  id="new-fuel-desc"
                  type="text"
                  bind:value={newFuelType.description}
                  placeholder="Brief description"
                  class="w-full px-3 py-2 border rounded bg-white dark:bg-gray-600 dark:border-gray-500 text-sm"
                />
              </div>
              <div class="flex items-end">
                <button
                  onclick={addCustomFuelType}
                  disabled={addingFuelType || !newFuelType.code.trim() || !newFuelType.name.trim()}
                  class="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded font-medium text-sm"
                >
                  {addingFuelType ? 'Adding...' : 'Add Type'}
                </button>
              </div>
            </div>
          </div>
        {/if}

        <!-- Fuel Types List -->
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {#each fuelTypes as ft}
            <div class="flex items-center justify-between p-3 rounded-lg {ft.is_default ? 'bg-gray-100 dark:bg-gray-700/50' : 'bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-700'}">
              <div>
                <div class="flex items-center gap-2">
                  <span class="font-medium">{ft.name}</span>
                  <span class="text-xs text-gray-500 font-mono">({ft.code})</span>
                  {#if ft.is_default}
                    <span class="text-xs bg-gray-200 dark:bg-gray-600 px-1.5 py-0.5 rounded">Default</span>
                  {:else}
                    <span class="text-xs bg-blue-200 dark:bg-blue-700 text-blue-800 dark:text-blue-200 px-1.5 py-0.5 rounded">Custom</span>
                  {/if}
                </div>
                {#if ft.description}
                  <p class="text-xs text-gray-500 mt-1">{ft.description}</p>
                {/if}
              </div>
              {#if !ft.is_default}
                <button
                  onclick={() => deleteFuelType(ft.id)}
                  class="text-red-500 hover:text-red-700 text-xs ml-2"
                >
                  Delete
                </button>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>
  {/if}

  <!-- Cached Prices Tab -->
  {#if activeTab === 'prices'}
    <div class="fuel-panel rounded-xl p-6">
      <h3 class="fuel-panel-title text-xl font-bold mb-4">{$translations('fuel.tabs.prices')}</h3>
      <p class="fuel-panel-subtitle text-sm mb-4">
        Prices collected from AI searches. This database grows as you search for more locations.
      </p>

      {#if loadingCached}
        <div class="text-center py-8 text-gray-500">{$translations('common.loading')}</div>
      {:else if cachedPrices.length === 0}
        <div class="text-center py-8 text-gray-500">
          <p>{$translations('fuel.empty.prices')}</p>
          <p class="text-sm mt-2">{$translations('insights.fuelPrices.searchToAdd')}</p>
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead class="bg-gray-100 dark:bg-gray-700">
              <tr>
                <th class="px-4 py-2 text-left">Location</th>
                <th class="px-4 py-2 text-left">Airport</th>
                <th class="px-4 py-2 text-left">Fuel</th>
                <th class="px-4 py-2 text-right">Price/Gal</th>
                <th class="px-4 py-2 text-left">Date</th>
                <th class="px-4 py-2 text-left">Source</th>
                <th class="px-4 py-2 text-center">Confidence</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200 dark:divide-gray-600">
              {#each cachedPrices as price}
                <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                  <td class="px-4 py-3">
                    <div class="font-medium">{price.location_name}</div>
                    {#if price.region}
                      <div class="text-xs text-gray-500">{price.region}</div>
                    {/if}
                  </td>
                  <td class="px-4 py-3 font-mono">{price.airport_code || '-'}</td>
                  <td class="px-4 py-3">{getFuelTypeLabel(price.fuel_type)}</td>
                  <td class="px-4 py-3 text-right font-bold text-green-600 dark:text-green-400">
                    {formatCurrency(price.price_per_gallon)}
                  </td>
                  <td class="px-4 py-3 text-gray-600 dark:text-gray-400">{price.effective_date}</td>
                  <td class="px-4 py-3 text-xs text-gray-500">{price.source || 'AI'}</td>
                  <td class="px-4 py-3 text-center">
                    <span class="px-2 py-1 rounded text-xs font-medium {getConfidenceColor(price.confidence)}">
                      {price.confidence}
                    </span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* ===== DEFAULT THEME ===== */
  .theme-default .fuel-stat-card {
    border: 1px solid #334155;
  }

  .theme-default .fuel-stat-primary {
    background: linear-gradient(135deg, #0891b2, #06b6d4);
    color: #ffffff;
  }

  .theme-default .fuel-stat-secondary {
    background: linear-gradient(135deg, #059669, #10b981);
    color: #ffffff;
  }

  .theme-default .fuel-stat-tertiary {
    background: linear-gradient(135deg, #7c3aed, #8b5cf6);
    color: #ffffff;
  }

  .theme-default .fuel-stat-quaternary {
    background: linear-gradient(135deg, #ea580c, #f97316);
    color: #ffffff;
  }

  .theme-default .fuel-stat-label { opacity: 0.85; }
  .theme-default .fuel-stat-value { color: #ffffff; }

  .theme-default .fuel-tab-nav { border-bottom: 1px solid #374151; }
  .theme-default .fuel-tab { color: #9ca3af; }
  .theme-default .fuel-tab:hover { color: #e5e7eb; }
  .theme-default .fuel-tab-active { color: #06b6d4; border-bottom: 2px solid #06b6d4; }

  .theme-default .fuel-panel {
    background-color: #1f2937;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.3);
  }

  .theme-default .fuel-panel-title { color: #ffffff; }
  .theme-default .fuel-panel-subtitle { color: #9ca3af; }
  .theme-default .fuel-label { color: #d1d5db; }

  .theme-default .fuel-input {
    background-color: #374151;
    border: 1px solid #4b5563;
    color: #ffffff;
  }

  .theme-default .fuel-input:focus {
    border-color: #06b6d4;
    outline: none;
    box-shadow: 0 0 0 2px rgba(6, 182, 212, 0.2);
  }

  .theme-default .fuel-btn-primary {
    background: linear-gradient(135deg, #0891b2, #06b6d4);
    border: 1px solid #22d3ee;
    color: #ffffff;
  }

  .theme-default .fuel-btn-primary:hover { background: linear-gradient(135deg, #06b6d4, #22d3ee); }

  .theme-default .fuel-btn-secondary {
    background: linear-gradient(135deg, #059669, #10b981);
    border: 1px solid #34d399;
    color: #ffffff;
  }

  .theme-default .fuel-btn-secondary:hover { background: linear-gradient(135deg, #10b981, #34d399); }

  /* ===== SKYNET THEME ===== */
  .theme-skynet .fuel-stat-card {
    background: linear-gradient(135deg, rgba(0, 40, 80, 0.8), rgba(0, 20, 40, 0.6));
    border: 1px solid rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .fuel-stat-primary { border-color: #00b4ff; box-shadow: 0 0 25px rgba(0, 180, 255, 0.3); }
  .theme-skynet .fuel-stat-secondary { border-color: #0080ff; box-shadow: 0 0 25px rgba(0, 128, 255, 0.3); }
  .theme-skynet .fuel-stat-tertiary { border-color: #00d4ff; box-shadow: 0 0 25px rgba(0, 212, 255, 0.3); }
  .theme-skynet .fuel-stat-quaternary { border-color: #0060ff; box-shadow: 0 0 25px rgba(0, 96, 255, 0.3); }

  .theme-skynet .fuel-stat-label { color: #0080ff; }
  .theme-skynet .fuel-stat-value { color: #00b4ff; text-shadow: 0 0 10px rgba(0, 180, 255, 0.5); }

  .theme-skynet .fuel-tab-nav { border-bottom: 1px solid rgba(0, 180, 255, 0.3); }
  .theme-skynet .fuel-tab { color: #0080ff; }
  .theme-skynet .fuel-tab:hover { color: #00b4ff; text-shadow: 0 0 5px rgba(0, 180, 255, 0.3); }
  .theme-skynet .fuel-tab-active {
    color: #00b4ff;
    border-bottom: 2px solid #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
    background: linear-gradient(180deg, transparent, rgba(0, 180, 255, 0.1));
  }

  .theme-skynet .fuel-panel {
    background: linear-gradient(135deg, rgba(0, 20, 40, 0.9), rgba(0, 10, 20, 0.8));
    border: 1px solid rgba(0, 180, 255, 0.3);
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.15);
  }

  .theme-skynet .fuel-panel-title { color: #00b4ff; text-shadow: 0 0 10px rgba(0, 180, 255, 0.5); }
  .theme-skynet .fuel-panel-subtitle { color: #0080ff; }
  .theme-skynet .fuel-label { color: #00b4ff; }

  .theme-skynet .fuel-input {
    background-color: rgba(0, 10, 20, 0.8);
    border: 1px solid rgba(0, 180, 255, 0.3);
    color: #d4e6ff;
  }

  .theme-skynet .fuel-input:focus {
    border-color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .fuel-btn-primary {
    background: linear-gradient(135deg, #0040ff, #00b4ff);
    border: 2px solid #00b4ff;
    color: #ffffff;
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.4);
    text-shadow: 0 0 5px rgba(255, 255, 255, 0.3);
  }

  .theme-skynet .fuel-btn-primary:hover {
    background: linear-gradient(135deg, #0060ff, #00d4ff);
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.6);
  }

  .theme-skynet .fuel-btn-primary:active {
    background: linear-gradient(135deg, #001a4d, #003366);
    box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.5), 0 0 20px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .fuel-btn-secondary {
    background: linear-gradient(135deg, #003366, #0080ff);
    border: 2px solid #0080ff;
    color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 128, 255, 0.3);
  }

  .theme-skynet .fuel-btn-secondary:hover {
    background: linear-gradient(135deg, #0060aa, #00b4ff);
    box-shadow: 0 0 25px rgba(0, 180, 255, 0.4);
    color: #ffffff;
  }

  /* ===== CYBERPUNK THEME ===== */
  .theme-cyberpunk .fuel-stat-card {
    background: linear-gradient(135deg, rgba(20, 20, 20, 0.9), rgba(10, 10, 10, 0.8));
    border: 1px solid rgba(0, 217, 255, 0.4);
    box-shadow: 0 0 20px rgba(0, 217, 255, 0.2), 0 0 40px rgba(255, 0, 128, 0.1);
  }

  .theme-cyberpunk .fuel-stat-primary { border-color: #00d9ff; box-shadow: 0 0 25px rgba(0, 217, 255, 0.3); }
  .theme-cyberpunk .fuel-stat-secondary { border-color: #ff0080; box-shadow: 0 0 25px rgba(255, 0, 128, 0.3); }
  .theme-cyberpunk .fuel-stat-tertiary { border-color: #b000ff; box-shadow: 0 0 25px rgba(176, 0, 255, 0.3); }
  .theme-cyberpunk .fuel-stat-quaternary { border-color: #00ff88; box-shadow: 0 0 25px rgba(0, 255, 136, 0.3); }

  .theme-cyberpunk .fuel-stat-label { color: #b000ff; }
  .theme-cyberpunk .fuel-stat-value { color: #00d9ff; text-shadow: 0 0 10px rgba(0, 217, 255, 0.5); }

  .theme-cyberpunk .fuel-tab-nav { border-bottom: 1px solid rgba(255, 0, 128, 0.3); }
  .theme-cyberpunk .fuel-tab { color: #b000ff; }
  .theme-cyberpunk .fuel-tab:hover { color: #00d9ff; text-shadow: 0 0 5px rgba(0, 217, 255, 0.3); }
  .theme-cyberpunk .fuel-tab-active {
    color: #00d9ff;
    border-bottom: 2px solid #ff0080;
    text-shadow: 0 0 10px rgba(0, 217, 255, 0.5);
    background: linear-gradient(180deg, transparent, rgba(255, 0, 128, 0.1));
  }

  .theme-cyberpunk .fuel-panel {
    background: linear-gradient(135deg, rgba(15, 15, 15, 0.95), rgba(10, 10, 10, 0.9));
    border: 1px solid rgba(0, 217, 255, 0.3);
    box-shadow: 0 0 30px rgba(0, 217, 255, 0.15), 0 0 60px rgba(255, 0, 128, 0.1);
  }

  .theme-cyberpunk .fuel-panel-title { color: #00d9ff; text-shadow: 0 0 10px rgba(0, 217, 255, 0.5); }
  .theme-cyberpunk .fuel-panel-subtitle { color: #b000ff; }
  .theme-cyberpunk .fuel-label { color: #ff0080; }

  .theme-cyberpunk .fuel-input {
    background-color: rgba(10, 10, 10, 0.9);
    border: 1px solid rgba(0, 217, 255, 0.3);
    color: #ffffff;
  }

  .theme-cyberpunk .fuel-input:focus {
    border-color: #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.4), 0 0 30px rgba(255, 0, 128, 0.2);
  }

  .theme-cyberpunk .fuel-btn-primary {
    background: linear-gradient(135deg, #00a0cc, #00d9ff);
    border: 2px solid #00d9ff;
    color: #000000;
    font-weight: 800;
    box-shadow: 0 0 20px rgba(0, 217, 255, 0.4), 0 0 40px rgba(255, 0, 128, 0.1);
  }

  .theme-cyberpunk .fuel-btn-primary:hover {
    background: linear-gradient(135deg, #00d9ff, #00ffff);
    box-shadow: 0 0 30px rgba(0, 217, 255, 0.6), 0 0 60px rgba(255, 0, 128, 0.2);
  }

  .theme-cyberpunk .fuel-btn-primary:active {
    background: linear-gradient(135deg, #003344, #005566);
    color: #00d9ff;
    box-shadow: inset 0 0 20px rgba(0, 0, 0, 0.5), 0 0 20px rgba(0, 217, 255, 0.3);
  }

  .theme-cyberpunk .fuel-btn-secondary {
    background: linear-gradient(135deg, #660040, #ff0080);
    border: 2px solid #ff0080;
    color: #ffffff;
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .fuel-btn-secondary:hover {
    background: linear-gradient(135deg, #ff0080, #ff4da6);
    box-shadow: 0 0 25px rgba(255, 0, 128, 0.5);
  }
</style>
