<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import PassengerDetail from './PassengerDetail.svelte';
  import { translations } from '$lib/i18n';

  // Types matching the Rust identity fusion architecture
  interface CanonicalPassenger {
    id: string;
    canonical_name: string;
    notes: string | null;
    total_flights: number;
    alias_count: number;
    first_seen_date: string | null;
    last_seen_date: string | null;
  }

  interface PassengerAlias {
    id: string;
    passenger_id: string;
    raw_name: string;
    usage_count: number;
    source_document: string | null;
    match_type: string | null;
    confidence: number;
  }

  interface IdentityStats {
    total_passengers: number;
    total_aliases: number;
    unmerged_count: number;
    merged_count: number;
  }

  interface BootstrapResult {
    passengers_created: number;
    aliases_created: number;
    flight_links_created: number;
  }

  interface BootstrapBatchResult {
    batch_number: number;
    total_batches: number;
    passengers_created_this_batch: number;
    aliases_created_this_batch: number;
    flight_links_created_this_batch: number;
    total_passengers_created: number;
    total_aliases_created: number;
    total_flight_links_created: number;
    is_complete: boolean;
    phase: string;
  }

  interface NoDedupPassenger {
    passenger_id: string;
    canonical_name: string;
    reason: string | null;
    total_flights: number;
    created_at: string;
  }

  interface SplitPreview {
    original_passenger_id: string;
    original_name: string;
    proposed_names: string[];
    flight_count: number;
  }

  interface SplitResult {
    original_deleted: boolean;
    new_passengers_created: string[];
    flights_reassigned: number;
  }

  interface BatchSplitCandidate {
    passenger_id: string;
    canonical_name: string;
    total_flights: number;
    detected_delimiter: string | null;
    proposed_names: string[];
  }

  interface BatchSplitResult {
    total_processed: number;
    total_new_passengers: number;
    total_flights_reassigned: number;
    errors: string[];
  }

  interface SearchPassengerResult {
    id: string;
    canonical_name: string;
    notes: string | null;
    total_flights: number;
    alias_count: number;
    first_seen_date: string | null;
    last_seen_date: string | null;
    matched_alias: string | null;
    aliases: string[];
  }

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  // Core state
  let canonicalPassengers: CanonicalPassenger[] = $state([]);
  let unmergedPassengers: CanonicalPassenger[] = $state([]);
  let noDedupPassengers: NoDedupPassenger[] = $state([]);
  let noDedupIds: Set<string> = $state(new Set());
  let stats: IdentityStats | null = $state(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // View controls - top level view and subtabs
  let mainView = $state<'passengers' | 'fusion'>('passengers');
  let activeTab = $state<'all' | 'unmerged' | 'nodedup'>('all');
  let searchQuery = $state('');
  let selectedPassenger: CanonicalPassenger | null = $state(null);

  // Search results state (for alias-aware search)
  let searchResults: SearchPassengerResult[] = $state([]);
  let searchLoading = $state(false);
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = $state(null);

  // Merge modal state
  let showMergeModal = $state(false);
  let mergeSource: CanonicalPassenger | null = $state(null);
  let mergeTargetSearch = $state('');
  let merging = $state(false);

  // Bootstrap state
  let bootstrapping = $state(false);
  let bootstrapResult: BootstrapResult | null = $state(null);
  let bootstrapProgress = $state<BootstrapBatchResult | null>(null);

  // Split modal state
  let showSplitModal = $state(false);
  let splitSource: CanonicalPassenger | null = $state(null);
  let splitPreview: SplitPreview | null = $state(null);
  let splitNames: string[] = $state([]);
  let splitDelimiter = $state(' ');
  let splitting = $state(false);

  // Batch split state
  let showBatchSplitModal = $state(false);
  let batchSplitCandidates: BatchSplitCandidate[] = $state([]);
  let batchSplitSelected: Set<string> = $state(new Set());
  let batchSplitSearching = $state(false);
  let batchSplitting = $state(false);
  let batchSplitSearch = $state('');

  onMount(async () => {
    await loadData();
  });

  async function loadData() {
    loading = true;
    error = null;
    try {
      const [canonical, unmerged, identityStats, nodedup] = await Promise.all([
        invoke<CanonicalPassenger[]>('list_canonical_passengers'),
        invoke<CanonicalPassenger[]>('list_unmerged_passengers'),
        invoke<IdentityStats>('get_identity_stats'),
        invoke<NoDedupPassenger[]>('list_no_dedup_passengers')
      ]);
      canonicalPassengers = canonical;
      unmergedPassengers = unmerged;
      stats = identityStats;
      noDedupPassengers = nodedup;
      noDedupIds = new Set(nodedup.map(p => p.passenger_id));

      // Also load search results for main passengers view
      await performSearch('');
    } catch (err) {
      console.error('Failed to load identity data:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  // Debounced search function for alias-aware search
  function handleSearchInput(query: string) {
    searchQuery = query;

    // Clear previous timer
    if (searchDebounceTimer) {
      clearTimeout(searchDebounceTimer);
    }

    // Debounce search by 300ms
    searchDebounceTimer = setTimeout(() => {
      performSearch(query);
    }, 300);
  }

  async function performSearch(query: string) {
    searchLoading = true;
    try {
      const results = await invoke<SearchPassengerResult[]>('search_passengers', {
        query: query
      });
      searchResults = results;
    } catch (err) {
      console.error('Search failed:', err);
      // Fallback to local filtering if search fails
      searchResults = [];
    } finally {
      searchLoading = false;
    }
  }

  async function bootstrapIdentities() {
    bootstrapping = true;
    bootstrapResult = null;
    bootstrapProgress = null;

    try {
      const batchSize = 200;
      let batchNumber = 0;
      let isComplete = false;

      while (!isComplete) {
        const result = await invoke<BootstrapBatchResult>('bootstrap_identities_batch', {
          userId,
          batchNumber,
          batchSize
        });

        bootstrapProgress = result;
        isComplete = result.is_complete;
        batchNumber++;

        // Small delay to let UI update
        await new Promise(resolve => setTimeout(resolve, 10));
      }

      // Convert to final result format
      if (bootstrapProgress) {
        bootstrapResult = {
          passengers_created: bootstrapProgress.total_passengers_created,
          aliases_created: bootstrapProgress.total_aliases_created,
          flight_links_created: bootstrapProgress.total_flight_links_created
        };
      }

      // Reload data to show new identities
      await loadData();
    } catch (err) {
      console.error('Bootstrap failed:', err);
      alert(`Bootstrap failed: ${err}`);
    } finally {
      bootstrapping = false;
      bootstrapProgress = null;
    }
  }

  function openMergeModal(passenger: CanonicalPassenger) {
    mergeSource = passenger;
    mergeTargetSearch = '';
    showMergeModal = true;
  }

  function closeMergeModal() {
    mergeSource = null;
    mergeTargetSearch = '';
    showMergeModal = false;
  }

  async function mergeIntoTarget(target: CanonicalPassenger) {
    if (!mergeSource) return;
    if (mergeSource.id === target.id) {
      alert('Cannot merge a passenger into itself');
      return;
    }

    merging = true;
    try {
      // The source's canonical_name is also its self-alias (unmerged pattern)
      await invoke('merge_alias', {
        sourceRawName: mergeSource.canonical_name,
        targetPassengerId: target.id
      });
      closeMergeModal();
      await loadData();
    } catch (err) {
      console.error('Merge failed:', err);
      alert(`Merge failed: ${err}`);
    } finally {
      merging = false;
    }
  }

  async function renamePassenger(passenger: CanonicalPassenger) {
    const newName = prompt('Enter new canonical name:', passenger.canonical_name);
    if (!newName || newName.trim() === passenger.canonical_name) return;

    try {
      await invoke('rename_canonical_passenger', {
        passengerId: passenger.id,
        newCanonicalName: newName.trim()
      });
      await loadData();
    } catch (err) {
      console.error('Rename failed:', err);
      alert(`Rename failed: ${err}`);
    }
  }

  async function markNoDedup(passenger: CanonicalPassenger) {
    const reason = prompt('Why should this not be deduplicated? (e.g., "Generic descriptor")', 'Generic passenger descriptor');
    try {
      await invoke('mark_no_dedup', {
        passengerId: passenger.id,
        reason: reason || null
      });
      await loadData();
    } catch (err) {
      console.error('Failed to mark as no-dedup:', err);
      alert(`Failed: ${err}`);
    }
  }

  async function unmarkNoDedup(passengerId: string) {
    try {
      await invoke('unmark_no_dedup', { passengerId });
      await loadData();
    } catch (err) {
      console.error('Failed to unmark no-dedup:', err);
      alert(`Failed: ${err}`);
    }
  }

  async function openSplitModal(passenger: CanonicalPassenger) {
    splitSource = passenger;
    splitDelimiter = ' ';
    splitting = false;

    // Auto-detect delimiter
    try {
      const detected = await invoke<string | null>('detect_compound_name', { passengerId: passenger.id });
      if (detected) {
        splitDelimiter = detected;
      }
    } catch (err) {
      console.error('Failed to detect delimiter:', err);
    }

    // Get preview
    await refreshSplitPreview();
    showSplitModal = true;
  }

  async function refreshSplitPreview() {
    if (!splitSource) return;
    try {
      const preview = await invoke<SplitPreview>('preview_split_passenger', {
        passengerId: splitSource.id,
        delimiter: splitDelimiter || ' '
      });
      splitPreview = preview;
      splitNames = [...preview.proposed_names];
    } catch (err) {
      console.error('Failed to preview split:', err);
    }
  }

  function closeSplitModal() {
    showSplitModal = false;
    splitSource = null;
    splitPreview = null;
    splitNames = [];
  }

  function addSplitName() {
    splitNames = [...splitNames, ''];
  }

  function removeSplitName(index: number) {
    splitNames = splitNames.filter((_, i) => i !== index);
  }

  function updateSplitName(index: number, value: string) {
    splitNames = splitNames.map((n, i) => i === index ? value.toUpperCase() : n);
  }

  async function executeSplit() {
    if (!splitSource || splitNames.length === 0) return;

    // Filter empty names
    const validNames = splitNames.filter(n => n.trim().length >= 2);
    if (validNames.length === 0) {
      alert('Please provide at least one valid name (2+ characters)');
      return;
    }

    splitting = true;
    try {
      const result = await invoke<SplitResult>('split_passenger', {
        passengerId: splitSource.id,
        newNames: validNames
      });

      alert(`Split complete!\nCreated ${result.new_passengers_created.length} new passengers\nReassigned ${result.flights_reassigned} flight links`);
      closeSplitModal();
      await loadData();
    } catch (err) {
      console.error('Split failed:', err);
      alert(`Split failed: ${err}`);
    } finally {
      splitting = false;
    }
  }

  // Batch split functions
  async function openBatchSplitModal() {
    showBatchSplitModal = true;
    batchSplitSelected = new Set();
    await findSplittableCandidates();
  }

  function closeBatchSplitModal() {
    showBatchSplitModal = false;
    batchSplitCandidates = [];
    batchSplitSelected = new Set();
    batchSplitSearch = '';
  }

  async function findSplittableCandidates() {
    batchSplitSearching = true;
    try {
      // Use batch split modal search if set, otherwise null to get all
      const query = batchSplitSearch.trim() || null;
      const candidates = await invoke<BatchSplitCandidate[]>('find_splittable_passengers', {
        searchQuery: query
      });
      batchSplitCandidates = candidates;
    } catch (err) {
      console.error('Failed to find splittable passengers:', err);
      alert(`Failed to search: ${err}`);
    } finally {
      batchSplitSearching = false;
    }
  }

  function toggleBatchSplitSelection(passengerId: string) {
    const newSet = new Set(batchSplitSelected);
    if (newSet.has(passengerId)) {
      newSet.delete(passengerId);
    } else {
      newSet.add(passengerId);
    }
    batchSplitSelected = newSet;
  }

  function selectAllBatchSplit() {
    batchSplitSelected = new Set(batchSplitCandidates.map(c => c.passenger_id));
  }

  function deselectAllBatchSplit() {
    batchSplitSelected = new Set();
  }

  async function executeBatchSplit() {
    if (batchSplitSelected.size === 0) {
      alert('Please select at least one passenger to split');
      return;
    }

    batchSplitting = true;
    try {
      const passengerIds = Array.from(batchSplitSelected);
      const result = await invoke<BatchSplitResult>('batch_split_passengers', {
        passengerIds
      });

      let message = `Batch split complete!\n\nProcessed: ${result.total_processed}\nNew passengers: ${result.total_new_passengers}\nFlights reassigned: ${result.total_flights_reassigned}`;
      if (result.errors.length > 0) {
        message += `\n\nErrors (${result.errors.length}):\n${result.errors.slice(0, 5).join('\n')}`;
        if (result.errors.length > 5) {
          message += `\n... and ${result.errors.length - 5} more`;
        }
      }
      alert(message);

      closeBatchSplitModal();
      await loadData();
    } catch (err) {
      console.error('Batch split failed:', err);
      alert(`Batch split failed: ${err}`);
    } finally {
      batchSplitting = false;
    }
  }

  // Filtered lists based on search
  const filteredCanonical = $derived(() => {
    if (!searchQuery.trim()) return canonicalPassengers;
    const query = searchQuery.toLowerCase();
    return canonicalPassengers.filter(p =>
      p.canonical_name.toLowerCase().includes(query)
    );
  });

  const filteredUnmerged = $derived(() => {
    if (!searchQuery.trim()) return unmergedPassengers;
    const query = searchQuery.toLowerCase();
    return unmergedPassengers.filter(p =>
      p.canonical_name.toLowerCase().includes(query)
    );
  });

  // Filtered no-dedup list
  const filteredNoDedup = $derived(() => {
    if (!searchQuery.trim()) return noDedupPassengers;
    const query = searchQuery.toLowerCase();
    return noDedupPassengers.filter(p =>
      p.canonical_name.toLowerCase().includes(query)
    );
  });

  // For merge modal - filter potential targets (exclude the source and no-dedup passengers)
  const mergeTargets = $derived(() => {
    if (!mergeSource) return [];
    let targets = canonicalPassengers.filter(p => p.id !== mergeSource!.id && !noDedupIds.has(p.id));
    if (mergeTargetSearch.trim()) {
      const query = mergeTargetSearch.toLowerCase();
      targets = targets.filter(p => p.canonical_name.toLowerCase().includes(query));
    }
    return targets;
  });

  // Current display list based on active tab (for all/unmerged tabs - canonical passengers)
  const displayList = $derived(() => {
    if (activeTab === 'nodedup') return []; // handled separately
    return activeTab === 'all' ? filteredCanonical() : filteredUnmerged();
  });
</script>

<div class="h-full flex flex-col">
  <!-- Main Header with View Tabs -->
  <div class="mb-6">
    <div class="flex items-center justify-between mb-4">
      <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Passengers</h2>
    </div>

    <!-- Main View Tabs -->
    <div class="flex items-center gap-2 border-b border-gray-200 dark:border-gray-700">
      <button
        onclick={() => mainView = 'passengers'}
        class="px-4 py-2 text-sm font-medium border-b-2 transition {mainView === 'passengers' ? 'border-primary-600 text-primary-600 dark:text-primary-400' : 'border-transparent text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'}"
      >
        All Passengers ({canonicalPassengers.length})
      </button>
      <button
        onclick={() => mainView = 'fusion'}
        class="px-4 py-2 text-sm font-medium border-b-2 transition {mainView === 'fusion' ? 'border-purple-600 text-purple-600 dark:text-purple-400' : 'border-transparent text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'}"
      >
        Identity Fusion
        {#if stats && stats.unmerged_count > 0}
          <span class="ml-1 px-1.5 py-0.5 text-xs rounded-full bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200">
            {stats.unmerged_count}
          </span>
        {/if}
      </button>
    </div>
  </div>

  {#if mainView === 'passengers'}
    <!-- MAIN PASSENGERS VIEW -->
    {#if loading}
      <div class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    {:else if error}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
        <div class="text-red-500 text-4xl mb-4">⚠️</div>
        <p class="text-red-600 dark:text-red-400 font-medium">Error loading data</p>
        <p class="text-gray-600 dark:text-gray-400 text-sm mt-2">{error}</p>
        <button
          onclick={loadData}
          class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg transition"
        >
          Retry
        </button>
      </div>
    {:else if canonicalPassengers.length === 0}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
        <div class="text-gray-400 text-4xl mb-4">👥</div>
        <p class="text-gray-600 dark:text-gray-400 font-medium">No passengers found</p>
        <p class="text-sm text-gray-500 dark:text-gray-500 mt-2">
          Import flight data or use Identity Fusion to bootstrap passengers from flight notes.
        </p>
        <button
          onclick={() => mainView = 'fusion'}
          class="mt-4 bg-purple-600 hover:bg-purple-700 text-white px-4 py-2 rounded-lg transition"
        >
          Go to Identity Fusion
        </button>
      </div>
    {:else}
      <!-- Search Bar for Main Passengers View (searches canonical names + aliases) -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-4 mb-6">
        <div class="relative">
          <input
            type="text"
            value={searchQuery}
            oninput={(e) => handleSearchInput((e.target as HTMLInputElement).value)}
            placeholder="Search passengers by name or alias (e.g., 'EPSTEIN', 'JE')..."
            class="w-full px-4 py-2 pr-10 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white placeholder-gray-500"
          />
          {#if searchLoading}
            <div class="absolute right-3 top-1/2 -translate-y-1/2">
              <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-primary-600"></div>
            </div>
          {:else if searchQuery}
            <button
              onclick={() => handleSearchInput('')}
              class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
            >
              ✕
            </button>
          {/if}
        </div>
        <p class="text-xs text-gray-500 dark:text-gray-400 mt-2">
          Searches both canonical names and all known aliases
        </p>
      </div>

      <!-- Main Passenger List -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden flex-1">
        <div class="overflow-x-auto max-h-[600px] overflow-y-auto">
          <table class="w-full">
            <thead class="bg-gray-50 dark:bg-gray-900 sticky top-0">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Name
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Aliases
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Flights
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
              {#each searchResults as passenger}
                <tr class="hover:bg-gray-50 dark:hover:bg-gray-900/50 transition">
                  <td class="px-6 py-4">
                    <div class="flex flex-col gap-1">
                      <button
                        onclick={() => selectedPassenger = { id: passenger.id, canonical_name: passenger.canonical_name, notes: passenger.notes, total_flights: passenger.total_flights, alias_count: passenger.alias_count, first_seen_date: passenger.first_seen_date, last_seen_date: passenger.last_seen_date }}
                        class="text-sm font-semibold text-primary-600 dark:text-primary-400 hover:text-primary-700 hover:underline text-left"
                      >
                        {passenger.canonical_name}
                      </button>
                      {#if passenger.matched_alias}
                        <span class="text-xs text-green-600 dark:text-green-400">
                          matched alias: <span class="font-mono">{passenger.matched_alias}</span>
                        </span>
                      {/if}
                    </div>
                  </td>
                  <td class="px-6 py-4">
                    <div class="flex flex-col gap-1">
                      <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium w-fit {passenger.alias_count > 1 ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200'}">
                        {passenger.alias_count} {passenger.alias_count === 1 ? 'alias' : 'aliases'}
                      </span>
                      {#if passenger.aliases.length > 1}
                        <div class="flex flex-wrap gap-1 mt-1 max-w-xs">
                          {#each passenger.aliases.slice(0, 5) as alias}
                            <span class="text-xs px-1.5 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 rounded font-mono">
                              {alias}
                            </span>
                          {/each}
                          {#if passenger.aliases.length > 5}
                            <span class="text-xs text-gray-500 dark:text-gray-400">
                              +{passenger.aliases.length - 5} more
                            </span>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap">
                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                      {passenger.total_flights} flights
                    </span>
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap text-sm">
                    <button
                      onclick={() => selectedPassenger = { id: passenger.id, canonical_name: passenger.canonical_name, notes: passenger.notes, total_flights: passenger.total_flights, alias_count: passenger.alias_count, first_seen_date: passenger.first_seen_date, last_seen_date: passenger.last_seen_date }}
                      class="px-3 py-1.5 bg-primary-600 hover:bg-primary-700 text-white text-xs font-medium rounded transition"
                    >
                      View Profile
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <!-- Footer with count -->
        <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900">
          <p class="text-sm text-gray-600 dark:text-gray-400">
            Showing {searchResults.length} of {canonicalPassengers.length} passengers
            {#if searchQuery}
              <span class="text-primary-600 dark:text-primary-400">
                (filtered by "{searchQuery}")
              </span>
            {/if}
          </p>
        </div>
      </div>
    {/if}

  {:else}
    <!-- IDENTITY FUSION VIEW -->
    <!-- Fusion Header -->
    <div class="flex items-center justify-between mb-4">
      <p class="text-gray-600 dark:text-gray-400">
        Manage canonical passenger identities, merge duplicates, and clean up data
      </p>
      <button
        onclick={bootstrapIdentities}
        disabled={bootstrapping}
        class="px-4 py-2 bg-purple-600 hover:bg-purple-700 disabled:bg-gray-400 text-white rounded-lg transition text-sm font-medium flex items-center gap-2"
      >
        {#if bootstrapping}
          <span class="animate-spin">⏳</span> Scanning...
        {:else}
          🔍 Bootstrap from Flights
        {/if}
      </button>
    </div>

    <!-- Bootstrap Progress Banner -->
    {#if bootstrapProgress}
      <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 mb-6">
        <div class="flex items-center justify-between mb-2">
          <p class="font-semibold text-blue-800 dark:text-blue-200">
            {bootstrapProgress.phase === 'creating_passengers' ? 'Creating Passengers...' :
             bootstrapProgress.phase === 'linking_flights' ? 'Linking Flights...' : 'Processing...'}
          </p>
          <span class="text-sm text-blue-600 dark:text-blue-400">
            Batch {bootstrapProgress.batch_number + 1} / {bootstrapProgress.total_batches}
          </span>
        </div>
        <div class="w-full bg-blue-200 dark:bg-blue-800 rounded-full h-2">
          <div
            class="bg-blue-600 dark:bg-blue-400 h-2 rounded-full transition-all duration-300"
            style="width: {((bootstrapProgress.batch_number + 1) / Math.max(bootstrapProgress.total_batches, 1)) * 100}%"
          ></div>
        </div>
        <p class="text-xs text-blue-600 dark:text-blue-400 mt-2">
          {bootstrapProgress.total_passengers_created} passengers,
          {bootstrapProgress.total_aliases_created} aliases,
          {bootstrapProgress.total_flight_links_created} flight links
        </p>
      </div>
    {/if}

    <!-- Bootstrap Result Banner -->
    {#if bootstrapResult && !bootstrapProgress}
      <div class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-4 mb-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="font-semibold text-green-800 dark:text-green-200">Bootstrap Complete</p>
            <p class="text-sm text-green-700 dark:text-green-300 mt-1">
              Created {bootstrapResult.passengers_created} passengers,
              {bootstrapResult.aliases_created} aliases,
              {bootstrapResult.flight_links_created} flight links
            </p>
          </div>
          <button
            onclick={() => bootstrapResult = null}
            class="text-green-600 hover:text-green-800 dark:text-green-400 dark:hover:text-green-200"
          >
            ✕
          </button>
        </div>
      </div>
    {/if}

    {#if loading}
      <div class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    {:else if error}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
        <div class="text-red-500 text-4xl mb-4">⚠️</div>
        <p class="text-red-600 dark:text-red-400 font-medium">Error loading data</p>
        <p class="text-gray-600 dark:text-gray-400 text-sm mt-2">{error}</p>
        <button
          onclick={loadData}
          class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg transition"
        >
          Retry
        </button>
      </div>
    {:else}
      <!-- Stats Cards -->
      {#if stats}
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
          <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-4">
            <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">Total Identities</p>
            <p class="text-2xl font-bold text-gray-900 dark:text-white">{stats.total_passengers}</p>
          </div>
          <div class="bg-blue-50 dark:bg-blue-900/20 rounded-lg shadow p-4">
            <p class="text-sm text-blue-700 dark:text-blue-300 mb-1">Total Aliases</p>
            <p class="text-2xl font-bold text-blue-900 dark:text-blue-100">{stats.total_aliases}</p>
          </div>
          <div class="bg-green-50 dark:bg-green-900/20 rounded-lg shadow p-4">
            <p class="text-sm text-green-700 dark:text-green-300 mb-1">Merged</p>
            <p class="text-2xl font-bold text-green-900 dark:text-green-100">{stats.merged_count}</p>
          </div>
          <div class="bg-orange-50 dark:bg-orange-900/20 rounded-lg shadow p-4">
            <p class="text-sm text-orange-700 dark:text-orange-300 mb-1">Unmerged</p>
            <p class="text-2xl font-bold text-orange-900 dark:text-orange-100">{stats.unmerged_count}</p>
          </div>
        </div>
      {/if}

      <!-- Tab Buttons & Search -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-4 mb-6">
      <div class="flex items-center gap-4 flex-wrap">
        <!-- Tabs -->
        <div class="flex items-center gap-1 bg-gray-100 dark:bg-gray-700 rounded-lg p-1">
          <button
            onclick={() => activeTab = 'all'}
            class="px-4 py-2 text-sm font-medium rounded-md transition {activeTab === 'all' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900'}"
          >
            All Identities ({canonicalPassengers.length})
          </button>
          <button
            onclick={() => activeTab = 'unmerged'}
            class="px-4 py-2 text-sm font-medium rounded-md transition {activeTab === 'unmerged' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900'}"
          >
            Unmerged ({unmergedPassengers.length})
          </button>
          <button
            onclick={() => activeTab = 'nodedup'}
            class="px-4 py-2 text-sm font-medium rounded-md transition {activeTab === 'nodedup' ? 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900'}"
          >
            No Dedup ({noDedupPassengers.length})
          </button>
        </div>

        <!-- Search -->
        <div class="flex-1">
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search passengers..."
            class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white placeholder-gray-500"
          />
        </div>

        <!-- Batch Split Button -->
        <button
          onclick={openBatchSplitModal}
          class="px-4 py-2 bg-orange-600 hover:bg-orange-700 text-white rounded-lg text-sm font-medium transition flex items-center gap-2"
          title="Find and split compound names in batch"
        >
          Batch Split
        </button>
      </div>
    </div>

    <!-- Passenger List -->
    {#if activeTab === 'nodedup'}
      <!-- No Dedup Tab -->
      {#if filteredNoDedup().length === 0}
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
          <div class="text-gray-400 text-4xl mb-4">🚫</div>
          <p class="text-gray-600 dark:text-gray-400">
            {#if noDedupPassengers.length === 0}
              No passengers marked as "do not deduplicate" yet.
            {:else}
              No passengers match your search
            {/if}
          </p>
          <p class="text-sm text-gray-500 dark:text-gray-500 mt-2">
            Mark generic names like "1 FEMALE", "2 FEMALES" from the Unmerged tab.
          </p>
        </div>
      {:else}
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden flex-1">
          <div class="overflow-x-auto max-h-[600px] overflow-y-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900 sticky top-0">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Name
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Reason
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Flights
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each filteredNoDedup() as ndPassenger}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900/50 transition bg-red-50/30 dark:bg-red-900/10">
                    <td class="px-6 py-4">
                      <span class="text-sm font-semibold text-gray-900 dark:text-white">
                        {ndPassenger.canonical_name}
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-sm text-gray-600 dark:text-gray-400 italic">
                        {ndPassenger.reason || 'No reason provided'}
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                        {ndPassenger.total_flights} flights
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm">
                      <button
                        onclick={() => unmarkNoDedup(ndPassenger.passenger_id)}
                        class="px-3 py-1.5 bg-green-600 hover:bg-green-700 text-white text-xs font-medium rounded transition"
                      >
                        Allow Dedup
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}
    {:else if displayList().length === 0}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
        <div class="text-gray-400 text-4xl mb-4">👥</div>
        <p class="text-gray-600 dark:text-gray-400">
          {#if canonicalPassengers.length === 0}
            No identities found. Click "Bootstrap from Flights" to scan flight data.
          {:else if searchQuery}
            No passengers match your search
          {:else}
            No {activeTab === 'unmerged' ? 'unmerged' : ''} passengers found
          {/if}
        </p>
      </div>
    {:else}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden flex-1">
        <div class="overflow-x-auto max-h-[600px] overflow-y-auto">
          <table class="w-full">
            <thead class="bg-gray-50 dark:bg-gray-900 sticky top-0">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Canonical Name
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Aliases
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Flights
                </th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
              {#each displayList() as passenger}
                <tr class="hover:bg-gray-50 dark:hover:bg-gray-900/50 transition {noDedupIds.has(passenger.id) ? 'bg-red-50/30 dark:bg-red-900/10' : ''}">
                  <td class="px-6 py-4">
                    <button
                      onclick={() => selectedPassenger = passenger}
                      class="text-sm font-semibold text-primary-600 dark:text-primary-400 hover:text-primary-700 hover:underline"
                    >
                      {passenger.canonical_name}
                    </button>
                    {#if noDedupIds.has(passenger.id)}
                      <span class="ml-2 inline-flex items-center px-1.5 py-0.5 rounded text-xs font-medium bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200">
                        No Dedup
                      </span>
                    {/if}
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap">
                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {passenger.alias_count > 1 ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' : 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200'}">
                      {passenger.alias_count} {passenger.alias_count === 1 ? 'alias' : 'aliases'}
                    </span>
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap">
                    <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                      {passenger.total_flights} flights
                    </span>
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap text-sm">
                    <div class="flex items-center gap-2">
                      {#if activeTab === 'unmerged' && !noDedupIds.has(passenger.id)}
                        <button
                          onclick={() => openMergeModal(passenger)}
                          class="px-3 py-1.5 bg-purple-600 hover:bg-purple-700 text-white text-xs font-medium rounded transition"
                        >
                          Merge Into...
                        </button>
                        <button
                          onclick={() => openSplitModal(passenger)}
                          class="px-3 py-1.5 bg-orange-600 hover:bg-orange-700 text-white text-xs font-medium rounded transition"
                          title="Split compound name into separate identities"
                        >
                          Split
                        </button>
                        <button
                          onclick={() => markNoDedup(passenger)}
                          class="px-3 py-1.5 bg-red-600 hover:bg-red-700 text-white text-xs font-medium rounded transition"
                          title="Mark as generic name - do not deduplicate"
                        >
                          Skip Dedup
                        </button>
                      {/if}
                      <button
                        onclick={() => renamePassenger(passenger)}
                        class="px-3 py-1.5 bg-gray-600 hover:bg-gray-700 text-white text-xs font-medium rounded transition"
                      >
                        Rename
                      </button>
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <!-- Progress Footer -->
        {#if stats && stats.total_passengers > 0}
          <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900">
            <div class="flex items-center justify-between mb-2">
              <span class="text-sm text-gray-600 dark:text-gray-400">
                Merge Progress
              </span>
              <span class="text-sm font-semibold text-gray-900 dark:text-white">
                {stats.merged_count} / {stats.total_passengers} ({stats.total_passengers > 0 ? Math.round((stats.merged_count / stats.total_passengers) * 100) : 0}%)
              </span>
            </div>
            <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5">
              <div
                class="bg-green-600 h-2.5 rounded-full transition-all duration-300"
                style="width: {stats.total_passengers > 0 ? (stats.merged_count / stats.total_passengers) * 100 : 0}%"
              ></div>
            </div>
          </div>
        {/if}
      </div>
    {/if}
    {/if}
  {/if}
  <!-- End of mainView === 'fusion' -->

  <!-- Passenger Detail Modal -->
  {#if selectedPassenger}
    <PassengerDetail
      {userId}
      passengerId={selectedPassenger.id}
      canonicalName={selectedPassenger.canonical_name}
      onClose={() => selectedPassenger = null}
      onDeleted={loadData}
    />
  {/if}

  <!-- Merge Modal -->
  {#if showMergeModal && mergeSource}
    <div
      class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
      onclick={(e) => { if (e.target === e.currentTarget) closeMergeModal(); }}
      role="dialog"
    >
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-2xl max-h-[80vh] flex flex-col">
        <!-- Modal Header -->
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-gradient-to-r from-purple-50 to-indigo-50 dark:from-purple-900/20 dark:to-indigo-900/20 rounded-t-xl">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="text-lg font-bold text-gray-900 dark:text-white">
                Merge "{mergeSource.canonical_name}"
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
                Select a target identity to merge into
              </p>
            </div>
            <button
              onclick={closeMergeModal}
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-2xl font-bold w-8 h-8 flex items-center justify-center rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
            >
              ×
            </button>
          </div>
        </div>

        <!-- Search -->
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <input
            type="text"
            bind:value={mergeTargetSearch}
            placeholder="Search target identities..."
            class="w-full px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white placeholder-gray-500"
          />
        </div>

        <!-- Target List -->
        <div class="flex-1 overflow-y-auto px-6 py-4">
          {#if mergeTargets().length === 0}
            <div class="text-center py-12 text-gray-500 dark:text-gray-400">
              <span class="text-4xl mb-2 block">📭</span>
              <p>No matching targets found</p>
            </div>
          {:else}
            <div class="space-y-2">
              {#each mergeTargets() as target}
                <button
                  onclick={() => mergeIntoTarget(target)}
                  disabled={merging}
                  class="w-full px-4 py-3 text-left bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg hover:bg-purple-50 dark:hover:bg-purple-900/20 hover:border-purple-300 dark:hover:border-purple-700 transition disabled:opacity-50"
                >
                  <div class="flex items-center justify-between">
                    <div>
                      <p class="font-semibold text-gray-900 dark:text-white">{target.canonical_name}</p>
                      <p class="text-sm text-gray-500 dark:text-gray-400">
                        {target.alias_count} aliases · {target.total_flights} flights
                      </p>
                    </div>
                    <span class="text-purple-600 dark:text-purple-400">→</span>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Modal Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900 rounded-b-xl">
          <div class="flex items-center justify-between">
            <p class="text-sm text-gray-600 dark:text-gray-400">
              {mergeTargets().length} targets available
            </p>
            <button
              onclick={closeMergeModal}
              class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg font-medium transition"
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Split Modal -->
  {#if showSplitModal && splitSource}
    <div
      class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
      onclick={(e) => { if (e.target === e.currentTarget) closeSplitModal(); }}
      role="dialog"
    >
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-lg max-h-[80vh] flex flex-col">
        <!-- Modal Header -->
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-gradient-to-r from-orange-50 to-amber-50 dark:from-orange-900/20 dark:to-amber-900/20 rounded-t-xl">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="text-lg font-bold text-gray-900 dark:text-white">
                Split "{splitSource.canonical_name}"
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
                Split compound name into separate identities ({splitSource.total_flights} flights)
              </p>
            </div>
            <button
              onclick={closeSplitModal}
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-2xl font-bold w-8 h-8 flex items-center justify-center rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
            >
              ×
            </button>
          </div>
        </div>

        <!-- Delimiter Selector -->
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Delimiter
          </label>
          <div class="flex items-center gap-2">
            <select
              bind:value={splitDelimiter}
              onchange={() => refreshSplitPreview()}
              class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white"
            >
              <option value=" ">Space</option>
              <option value="/">Slash (/)</option>
              <option value=",">,Comma (,)</option>
              <option value="&">Ampersand (&)</option>
              <option value=" AND ">AND</option>
            </select>
          </div>
        </div>

        <!-- Names List -->
        <div class="flex-1 overflow-y-auto px-6 py-4">
          <div class="flex items-center justify-between mb-3">
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Names to create ({splitNames.length})
            </label>
            <button
              onclick={addSplitName}
              class="px-3 py-1 text-sm bg-green-600 hover:bg-green-700 text-white rounded transition"
            >
              + Add
            </button>
          </div>

          <div class="space-y-2">
            {#each splitNames as name, index}
              <div class="flex items-center gap-2">
                <input
                  type="text"
                  value={name}
                  oninput={(e) => updateSplitName(index, (e.target as HTMLInputElement).value)}
                  placeholder="Enter name..."
                  class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white font-mono uppercase"
                />
                <button
                  onclick={() => removeSplitName(index)}
                  class="px-3 py-2 text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-900/20 rounded transition"
                  title="Remove"
                >
                  ✕
                </button>
              </div>
            {/each}
          </div>

          {#if splitNames.length === 0}
            <p class="text-center text-gray-500 dark:text-gray-400 py-4">
              No names detected. Add names manually or adjust delimiter.
            </p>
          {/if}
        </div>

        <!-- Modal Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900 rounded-b-xl">
          <div class="flex items-center justify-between">
            <p class="text-sm text-gray-600 dark:text-gray-400">
              Flights will be linked to all new identities
            </p>
            <div class="flex items-center gap-2">
              <button
                onclick={closeSplitModal}
                class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg font-medium transition"
              >
                Cancel
              </button>
              <button
                onclick={executeSplit}
                disabled={splitting || splitNames.filter(n => n.trim().length >= 2).length === 0}
                class="px-4 py-2 bg-orange-600 hover:bg-orange-700 disabled:bg-gray-400 text-white rounded-lg font-medium transition"
              >
                {#if splitting}
                  Splitting...
                {:else}
                  Split into {splitNames.filter(n => n.trim().length >= 2).length} identities
                {/if}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Batch Split Modal -->
  {#if showBatchSplitModal}
    <div
      class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
      onclick={(e) => { if (e.target === e.currentTarget) closeBatchSplitModal(); }}
      role="dialog"
    >
      <div class="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-3xl max-h-[85vh] flex flex-col">
        <!-- Modal Header -->
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-gradient-to-r from-orange-50 to-amber-50 dark:from-orange-900/20 dark:to-amber-900/20 rounded-t-xl">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="text-lg font-bold text-gray-900 dark:text-white">
                Batch Split Compound Names
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
                Find and split compound names (JE GM, AB/CD, etc.) in batch
              </p>
            </div>
            <button
              onclick={closeBatchSplitModal}
              class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-2xl font-bold w-8 h-8 flex items-center justify-center rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
            >
              ×
            </button>
          </div>
        </div>

        <!-- Search Bar -->
        <div class="px-6 py-3 border-b border-gray-200 dark:border-gray-700">
          <div class="flex items-center gap-2">
            <input
              type="text"
              bind:value={batchSplitSearch}
              placeholder="Search compound names..."
              class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white placeholder-gray-500"
              onkeydown={(e) => { if (e.key === 'Enter') findSplittableCandidates(); }}
            />
            <button
              onclick={findSplittableCandidates}
              disabled={batchSplitSearching}
              class="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 text-white rounded-lg font-medium transition"
            >
              {#if batchSplitSearching}
                Searching...
              {:else}
                Search
              {/if}
            </button>
          </div>
        </div>

        <!-- Selection Controls -->
        <div class="px-6 py-3 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-4">
              <span class="text-sm text-gray-600 dark:text-gray-400">
                {batchSplitCandidates.length} compound names found
              </span>
              <span class="text-sm font-semibold text-orange-600 dark:text-orange-400">
                {batchSplitSelected.size} selected
              </span>
            </div>
            <div class="flex items-center gap-2">
              <button
                onclick={selectAllBatchSplit}
                class="px-3 py-1.5 text-xs font-medium bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded transition"
              >
                Select All
              </button>
              <button
                onclick={deselectAllBatchSplit}
                class="px-3 py-1.5 text-xs font-medium bg-gray-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-300 rounded transition"
              >
                Deselect All
              </button>
            </div>
          </div>
        </div>

        <!-- Candidates List -->
        <div class="flex-1 overflow-y-auto px-6 py-4">
          {#if batchSplitSearching}
            <div class="flex items-center justify-center py-12">
              <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-orange-600"></div>
              <span class="ml-3 text-gray-600 dark:text-gray-400">Searching for compound names...</span>
            </div>
          {:else if batchSplitCandidates.length === 0}
            <div class="text-center py-12 text-gray-500 dark:text-gray-400">
              <span class="text-4xl mb-2 block">✨</span>
              <p class="font-medium">No compound names found</p>
              <p class="text-sm mt-1">All passenger names look properly formatted</p>
            </div>
          {:else}
            <div class="space-y-2">
              {#each batchSplitCandidates as candidate}
                <button
                  onclick={() => toggleBatchSplitSelection(candidate.passenger_id)}
                  class="w-full px-4 py-3 text-left border rounded-lg transition {batchSplitSelected.has(candidate.passenger_id) ? 'bg-orange-50 dark:bg-orange-900/20 border-orange-300 dark:border-orange-700' : 'bg-white dark:bg-gray-900 border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800'}"
                >
                  <div class="flex items-start justify-between gap-4">
                    <div class="flex-1">
                      <div class="flex items-center gap-2">
                        <span class="w-5 h-5 flex items-center justify-center rounded border {batchSplitSelected.has(candidate.passenger_id) ? 'bg-orange-600 border-orange-600 text-white' : 'border-gray-300 dark:border-gray-600'}">
                          {#if batchSplitSelected.has(candidate.passenger_id)}✓{/if}
                        </span>
                        <span class="font-semibold text-gray-900 dark:text-white font-mono">
                          {candidate.canonical_name}
                        </span>
                        <span class="text-xs px-2 py-0.5 rounded bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200">
                          {candidate.total_flights} flights
                        </span>
                      </div>
                      <div class="mt-2 ml-7 flex items-center gap-2 flex-wrap">
                        <span class="text-xs text-gray-500 dark:text-gray-400">
                          Will split into:
                        </span>
                        {#each candidate.proposed_names as name}
                          <span class="text-xs px-2 py-0.5 rounded bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200 font-mono">
                            {name}
                          </span>
                        {/each}
                      </div>
                    </div>
                    {#if candidate.detected_delimiter}
                      <span class="text-xs px-2 py-0.5 rounded bg-gray-100 text-gray-600 dark:bg-gray-700 dark:text-gray-400">
                        delim: "{candidate.detected_delimiter === ' ' ? 'space' : candidate.detected_delimiter}"
                      </span>
                    {/if}
                  </div>
                </button>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Modal Footer -->
        <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900 rounded-b-xl">
          <div class="flex items-center justify-between">
            <p class="text-sm text-gray-600 dark:text-gray-400">
              Each compound name will be split into separate identities
            </p>
            <div class="flex items-center gap-2">
              <button
                onclick={closeBatchSplitModal}
                class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg font-medium transition"
              >
                Cancel
              </button>
              <button
                onclick={executeBatchSplit}
                disabled={batchSplitting || batchSplitSelected.size === 0}
                class="px-4 py-2 bg-orange-600 hover:bg-orange-700 disabled:bg-gray-400 text-white rounded-lg font-medium transition"
              >
                {#if batchSplitting}
                  Splitting...
                {:else}
                  Split {batchSplitSelected.size} Selected
                {/if}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
