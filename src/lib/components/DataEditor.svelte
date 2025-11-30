<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  // Tab state
  type EditorTab = 'flights' | 'flagged' | 'passengers' | 'stats';
  let activeTab = $state<EditorTab>('flights');

  // Flight editor state
  interface Flight {
    id: string;
    flight_number: string | null;
    departure_airport: string;
    arrival_airport: string;
    departure_datetime: string;
    arrival_datetime: string | null;
    aircraft_registration: string | null;
    seat_number: string | null;
    fare_class: string | null;
    total_cost: number | null;
    currency: string | null;
    booking_reference: string | null;
    notes: string | null;
    distance_km: number | null;
    flight_duration: number | null;
    carbon_emissions_kg: number | null;
  }

  let flights = $state<Flight[]>([]);
  let flightsLoading = $state(false);
  let flightsTotalCount = $state(0);
  let flightsPage = $state(0);
  let flightsPerPage = $state(50);
  let flightsSearch = $state('');
  let flightsSortBy = $state('departure_datetime');
  let flightsSortDir = $state<'asc' | 'desc'>('desc');
  let selectedFlightIds = $state<Set<string>>(new Set());
  let editingFlight = $state<Flight | null>(null);
  let showEditModal = $state(false);

  // Flagged items state (anomalies from the self-improvement system)
  interface FlightAnomaly {
    id: string;
    flight_id: string;
    anomaly_type: string;
    severity: string;
    description: string;
    suggested_fix: string | null;
    is_resolved: boolean;
  }

  interface DuplicateCandidate {
    id: string;
    flight_id_1: string;
    flight_id_2: string;
    similarity_score: number;
    match_reasons: string[];
  }

  let anomalies = $state<FlightAnomaly[]>([]);
  let duplicateCandidates = $state<DuplicateCandidate[]>([]);
  let flaggedLoading = $state(false);
  let flaggedSubTab = $state<'anomalies' | 'duplicates'>('anomalies');

  // Passenger state
  interface PassengerName {
    abbreviation: string;
    full_name: string | null;
    usage_count: number;
  }

  let passengers = $state<PassengerName[]>([]);
  let passengersLoading = $state(false);
  let passengerSearch = $state('');
  let editingPassenger = $state<PassengerName | null>(null);
  let showPassengerModal = $state(false);
  let newPassengerName = $state('');

  // Stats state
  interface EditorStats {
    total_flights: number;
    total_passengers: number;
    potential_duplicates: number;
    flights_without_notes: number;
    flights_without_distance: number;
  }

  let stats = $state<EditorStats | null>(null);
  let statsLoading = $state(false);

  // Filtered passengers
  let filteredPassengers = $derived.by(() => {
    if (!passengerSearch) return passengers;
    const search = passengerSearch.toLowerCase();
    return passengers.filter(p =>
      p.abbreviation.toLowerCase().includes(search) ||
      (p.full_name && p.full_name.toLowerCase().includes(search))
    );
  });

  // Load flights
  async function loadFlights() {
    flightsLoading = true;
    try {
      const [flightList, count] = await Promise.all([
        invoke<Flight[]>('get_flights_for_editor', {
          userId,
          limit: flightsPerPage,
          offset: flightsPage * flightsPerPage,
          search: flightsSearch || null,
          sortBy: flightsSortBy,
          sortDir: flightsSortDir,
        }),
        invoke<number>('get_flight_count', {
          userId,
          search: flightsSearch || null,
        }),
      ]);
      flights = flightList;
      flightsTotalCount = count;
    } catch (err) {
      console.error('Failed to load flights:', err);
    } finally {
      flightsLoading = false;
    }
  }

  // Load flagged items (anomalies and duplicate candidates)
  async function loadFlaggedItems() {
    flaggedLoading = true;
    try {
      const [anomalyList, duplicateList] = await Promise.all([
        invoke<FlightAnomaly[]>('get_pending_anomalies'),
        invoke<DuplicateCandidate[]>('get_pending_duplicates'),
      ]);
      anomalies = anomalyList;
      duplicateCandidates = duplicateList;
    } catch (err) {
      console.error('Failed to load flagged items:', err);
    } finally {
      flaggedLoading = false;
    }
  }

  // Load passengers
  async function loadPassengers() {
    passengersLoading = true;
    try {
      passengers = await invoke<PassengerName[]>('get_all_passenger_names', { userId });
    } catch (err) {
      console.error('Failed to load passengers:', err);
    } finally {
      passengersLoading = false;
    }
  }

  // Load stats
  async function loadStats() {
    statsLoading = true;
    try {
      stats = await invoke<EditorStats>('get_data_editor_stats', { userId });
    } catch (err) {
      console.error('Failed to load stats:', err);
    } finally {
      statsLoading = false;
    }
  }

  // Handle tab change
  function handleTabChange(tab: EditorTab) {
    activeTab = tab;
    if (tab === 'flights' && flights.length === 0) loadFlights();
    if (tab === 'flagged' && anomalies.length === 0 && duplicateCandidates.length === 0) loadFlaggedItems();
    if (tab === 'passengers' && passengers.length === 0) loadPassengers();
    if (tab === 'stats') loadStats();
  }

  // Flight operations
  function toggleFlightSelection(flightId: string) {
    const newSet = new Set(selectedFlightIds);
    if (newSet.has(flightId)) {
      newSet.delete(flightId);
    } else {
      newSet.add(flightId);
    }
    selectedFlightIds = newSet;
  }

  function selectAllFlights() {
    if (selectedFlightIds.size === flights.length) {
      selectedFlightIds = new Set();
    } else {
      selectedFlightIds = new Set(flights.map(f => f.id));
    }
  }

  async function deleteSelectedFlights() {
    if (selectedFlightIds.size === 0) return;
    if (!confirm(`Delete ${selectedFlightIds.size} flight(s)? This cannot be undone.`)) return;

    try {
      const result = await invoke<{ deleted_count: number; failed_ids: string[] }>('bulk_delete_flights', {
        flightIds: Array.from(selectedFlightIds),
      });
      alert(`Deleted ${result.deleted_count} flight(s)`);
      selectedFlightIds = new Set();
      await loadFlights();
      await loadStats();
    } catch (err) {
      console.error('Failed to delete flights:', err);
      alert('Failed to delete: ' + err);
    }
  }

  function openEditFlight(flight: Flight) {
    editingFlight = { ...flight };
    showEditModal = true;
  }

  async function saveFlightEdit() {
    if (!editingFlight) return;

    try {
      await invoke('update_flight', {
        flightId: editingFlight.id,
        updates: {
          flight_number: editingFlight.flight_number,
          departure_airport: editingFlight.departure_airport,
          arrival_airport: editingFlight.arrival_airport,
          departure_datetime: editingFlight.departure_datetime,
          arrival_datetime: editingFlight.arrival_datetime,
          aircraft_registration: editingFlight.aircraft_registration,
          seat_number: editingFlight.seat_number,
          fare_class: editingFlight.fare_class,
          total_cost: editingFlight.total_cost,
          currency: editingFlight.currency,
          booking_reference: editingFlight.booking_reference,
          notes: editingFlight.notes,
          distance_km: editingFlight.distance_km,
          flight_duration: editingFlight.flight_duration,
          carbon_emissions_kg: editingFlight.carbon_emissions_kg,
        },
      });
      showEditModal = false;
      editingFlight = null;
      await loadFlights();
    } catch (err) {
      console.error('Failed to save flight:', err);
      alert('Failed to save: ' + err);
    }
  }

  async function deleteFlight(flightId: string) {
    if (!confirm('Delete this flight? This cannot be undone.')) return;

    try {
      await invoke('delete_flight', { flightId });
      await loadFlights();
      await loadStats();
    } catch (err) {
      console.error('Failed to delete flight:', err);
      alert('Failed to delete: ' + err);
    }
  }

  // Flagged item operations
  async function resolveAnomaly(anomalyId: string) {
    try {
      await invoke('resolve_anomaly', { anomalyId });
      anomalies = anomalies.filter(a => a.id !== anomalyId);
    } catch (err) {
      console.error('Failed to resolve anomaly:', err);
      alert('Failed to resolve: ' + err);
    }
  }

  async function viewFlightFromAnomaly(flightId: string) {
    // Switch to flights tab and search for this flight
    activeTab = 'flights';
    flightsSearch = flightId.slice(0, 8); // Use partial ID for search
    flightsPage = 0;
    await loadFlights();
  }

  // Get severity color
  function getSeverityColor(severity: string): string {
    switch (severity) {
      case 'error': return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
      case 'warning': return 'bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200';
      default: return 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200';
    }
  }

  // Get anomaly type display name
  function getAnomalyTypeLabel(type: string): string {
    switch (type) {
      case 'impossible_speed': return 'Impossible Speed';
      case 'invalid_distance': return 'Invalid Distance';
      case 'duration_outlier': return 'Duration Outlier';
      case 'missing_data': return 'Missing Data';
      default: return type.replace(/_/g, ' ');
    }
  }

  // Passenger operations
  function openPassengerEdit(passenger: PassengerName) {
    editingPassenger = passenger;
    newPassengerName = passenger.full_name || passenger.abbreviation;
    showPassengerModal = true;
  }

  async function renamePassenger() {
    if (!editingPassenger || !newPassengerName.trim()) return;

    try {
      await invoke<number>('rename_passenger_in_flights', {
        userId,
        oldName: editingPassenger.abbreviation,
        newName: newPassengerName.trim(),
      });
      showPassengerModal = false;
      editingPassenger = null;
      await loadPassengers();
    } catch (err) {
      console.error('Failed to rename passenger:', err);
      alert('Failed to rename: ' + err);
    }
  }

  async function removePassenger(passenger: PassengerName) {
    if (!confirm(`Remove "${passenger.abbreviation}" from all ${passenger.usage_count} flight(s)?`)) return;

    try {
      await invoke<number>('remove_passenger_from_flights', {
        userId,
        passengerName: passenger.abbreviation,
      });
      await loadPassengers();
      await loadStats();
    } catch (err) {
      console.error('Failed to remove passenger:', err);
      alert('Failed to remove: ' + err);
    }
  }

  // Search handler with debounce
  let searchTimeout: ReturnType<typeof setTimeout>;
  function handleSearchChange(value: string) {
    flightsSearch = value;
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => {
      flightsPage = 0;
      loadFlights();
    }, 300);
  }

  // Sort handler
  function handleSort(column: string) {
    if (flightsSortBy === column) {
      flightsSortDir = flightsSortDir === 'asc' ? 'desc' : 'asc';
    } else {
      flightsSortBy = column;
      flightsSortDir = 'desc';
    }
    loadFlights();
  }

  // Pagination
  let totalPages = $derived(Math.ceil(flightsTotalCount / flightsPerPage));

  function goToPage(page: number) {
    if (page >= 0 && page < totalPages) {
      flightsPage = page;
      loadFlights();
    }
  }

  // Format date for display
  function formatDate(dateStr: string | null): string {
    if (!dateStr) return '-';
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString() + ' ' + date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } catch {
      return dateStr;
    }
  }

  onMount(() => {
    loadFlights();
  });
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Header -->
  <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex-shrink-0">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-2">Data Editor</h2>
    <p class="text-gray-600 dark:text-gray-400 text-sm">
      Manage flights, detect duplicates, edit passenger records, and maintain data quality.
    </p>
  </div>

  <!-- Tab Navigation -->
  <div class="px-6 border-b border-gray-200 dark:border-gray-700 flex-shrink-0">
    <nav class="-mb-px flex gap-6">
      <button
        onclick={() => handleTabChange('flights')}
        class="py-3 px-1 border-b-2 text-sm font-medium transition {activeTab === 'flights'
          ? 'border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400'
          : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400'}"
      >
        Flights
      </button>
      <button
        onclick={() => handleTabChange('flagged')}
        class="py-3 px-1 border-b-2 text-sm font-medium transition {activeTab === 'flagged'
          ? 'border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400'
          : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400'}"
      >
        Flagged
        {#if anomalies.length > 0 || duplicateCandidates.length > 0}
          <span class="ml-1 px-1.5 py-0.5 text-xs bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200 rounded-full">
            {anomalies.length + duplicateCandidates.length}
          </span>
        {/if}
      </button>
      <button
        onclick={() => handleTabChange('passengers')}
        class="py-3 px-1 border-b-2 text-sm font-medium transition {activeTab === 'passengers'
          ? 'border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400'
          : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400'}"
      >
        Passengers
      </button>
      <button
        onclick={() => handleTabChange('stats')}
        class="py-3 px-1 border-b-2 text-sm font-medium transition {activeTab === 'stats'
          ? 'border-indigo-600 text-indigo-600 dark:border-indigo-400 dark:text-indigo-400'
          : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400'}"
      >
        Data Quality
      </button>
    </nav>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-hidden">
    <!-- Flights Tab -->
    {#if activeTab === 'flights'}
      <div class="h-full flex flex-col">
        <!-- Toolbar -->
        <div class="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center gap-4 flex-wrap">
          <input
            type="text"
            placeholder="Search flights..."
            value={flightsSearch}
            oninput={(e) => handleSearchChange(e.currentTarget.value)}
            class="px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white w-64"
          />
          <div class="flex-1"></div>
          {#if selectedFlightIds.size > 0}
            <span class="text-sm text-gray-600 dark:text-gray-400">
              {selectedFlightIds.size} selected
            </span>
            <button
              onclick={deleteSelectedFlights}
              class="px-3 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 text-sm"
            >
              Delete Selected
            </button>
          {/if}
          <button
            onclick={loadFlights}
            class="px-3 py-2 bg-gray-100 dark:bg-gray-700 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 text-sm"
          >
            Refresh
          </button>
        </div>

        <!-- Table -->
        <div class="flex-1 overflow-auto p-4">
          {#if flightsLoading}
            <div class="flex items-center justify-center h-full">
              <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
            </div>
          {:else if flights.length === 0}
            <div class="text-center py-12 text-gray-500">
              <p>No flights found</p>
            </div>
          {:else}
            <table class="w-full text-sm">
              <thead class="sticky top-0 bg-gray-50 dark:bg-gray-800">
                <tr class="text-left text-xs uppercase text-gray-500 dark:text-gray-400">
                  <th class="px-3 py-2">
                    <input
                      type="checkbox"
                      checked={selectedFlightIds.size === flights.length && flights.length > 0}
                      onchange={selectAllFlights}
                      class="rounded"
                    />
                  </th>
                  <th class="px-3 py-2 cursor-pointer hover:text-gray-700" onclick={() => handleSort('departure_datetime')}>
                    Date {flightsSortBy === 'departure_datetime' ? (flightsSortDir === 'asc' ? '↑' : '↓') : ''}
                  </th>
                  <th class="px-3 py-2 cursor-pointer hover:text-gray-700" onclick={() => handleSort('flight_number')}>
                    Flight
                  </th>
                  <th class="px-3 py-2 cursor-pointer hover:text-gray-700" onclick={() => handleSort('departure_airport')}>
                    Route
                  </th>
                  <th class="px-3 py-2">Distance</th>
                  <th class="px-3 py-2">Notes</th>
                  <th class="px-3 py-2 text-right">Actions</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each flights as flight}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                    <td class="px-3 py-2">
                      <input
                        type="checkbox"
                        checked={selectedFlightIds.has(flight.id)}
                        onchange={() => toggleFlightSelection(flight.id)}
                        class="rounded"
                      />
                    </td>
                    <td class="px-3 py-2 text-gray-900 dark:text-white">
                      {formatDate(flight.departure_datetime)}
                    </td>
                    <td class="px-3 py-2 font-mono text-indigo-600 dark:text-indigo-400">
                      {flight.flight_number || '-'}
                    </td>
                    <td class="px-3 py-2 font-medium text-gray-900 dark:text-white">
                      {flight.departure_airport} → {flight.arrival_airport}
                    </td>
                    <td class="px-3 py-2 text-gray-600 dark:text-gray-400">
                      {flight.distance_km ? `${Math.round(flight.distance_km)} km` : '-'}
                    </td>
                    <td class="px-3 py-2 text-gray-500 dark:text-gray-400 truncate max-w-[200px]" title={flight.notes || ''}>
                      {flight.notes || '-'}
                    </td>
                    <td class="px-3 py-2 text-right">
                      <button
                        onclick={() => openEditFlight(flight)}
                        class="text-indigo-600 hover:text-indigo-800 dark:text-indigo-400 mr-2"
                      >
                        Edit
                      </button>
                      <button
                        onclick={() => deleteFlight(flight.id)}
                        class="text-red-500 hover:text-red-700"
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>

        <!-- Pagination -->
        {#if totalPages > 1}
          <div class="p-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between">
            <span class="text-sm text-gray-600 dark:text-gray-400">
              Showing {flightsPage * flightsPerPage + 1} - {Math.min((flightsPage + 1) * flightsPerPage, flightsTotalCount)} of {flightsTotalCount}
            </span>
            <div class="flex gap-2">
              <button
                onclick={() => goToPage(flightsPage - 1)}
                disabled={flightsPage === 0}
                class="px-3 py-1 rounded border dark:border-gray-600 disabled:opacity-50"
              >
                Previous
              </button>
              <span class="px-3 py-1">Page {flightsPage + 1} of {totalPages}</span>
              <button
                onclick={() => goToPage(flightsPage + 1)}
                disabled={flightsPage >= totalPages - 1}
                class="px-3 py-1 rounded border dark:border-gray-600 disabled:opacity-50"
              >
                Next
              </button>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Flagged Items Tab -->
    {#if activeTab === 'flagged'}
      <div class="h-full flex flex-col">
        <!-- Sub-tabs for Anomalies vs Duplicates -->
        <div class="px-4 pt-4 border-b border-gray-200 dark:border-gray-700 flex gap-4">
          <button
            onclick={() => flaggedSubTab = 'anomalies'}
            class="pb-3 text-sm font-medium transition {flaggedSubTab === 'anomalies'
              ? 'text-red-600 border-b-2 border-red-600'
              : 'text-gray-500 hover:text-gray-700'}"
          >
            Anomalies
            {#if anomalies.length > 0}
              <span class="ml-1 px-1.5 py-0.5 text-xs bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200 rounded-full">
                {anomalies.length}
              </span>
            {/if}
          </button>
          <button
            onclick={() => flaggedSubTab = 'duplicates'}
            class="pb-3 text-sm font-medium transition {flaggedSubTab === 'duplicates'
              ? 'text-amber-600 border-b-2 border-amber-600'
              : 'text-gray-500 hover:text-gray-700'}"
          >
            Duplicates
            {#if duplicateCandidates.length > 0}
              <span class="ml-1 px-1.5 py-0.5 text-xs bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200 rounded-full">
                {duplicateCandidates.length}
              </span>
            {/if}
          </button>
          <div class="flex-1"></div>
          <button
            onclick={loadFlaggedItems}
            class="mb-2 px-3 py-1.5 bg-gray-100 dark:bg-gray-700 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 text-sm"
          >
            Refresh
          </button>
        </div>

        <div class="flex-1 overflow-auto p-4">
          {#if flaggedLoading}
            <div class="flex items-center justify-center h-full">
              <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
            </div>
          {:else if flaggedSubTab === 'anomalies'}
            {#if anomalies.length === 0}
              <div class="text-center py-12 text-gray-500 flex flex-col items-center justify-center">
                <div class="text-4xl mb-4">✓</div>
                <p class="font-medium">No anomalies detected</p>
                <p class="text-sm mt-1">All flights pass validation checks</p>
              </div>
            {:else}
              <div class="space-y-4">
                {#each anomalies as anomaly}
                  <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
                    <div class="flex items-start justify-between mb-2">
                      <div class="flex items-center gap-2">
                        <span class="px-2 py-1 text-xs font-medium rounded {getSeverityColor(anomaly.severity)}">
                          {anomaly.severity.toUpperCase()}
                        </span>
                        <span class="font-medium text-gray-900 dark:text-white">
                          {getAnomalyTypeLabel(anomaly.anomaly_type)}
                        </span>
                      </div>
                      <div class="flex gap-2">
                        <button
                          onclick={() => viewFlightFromAnomaly(anomaly.flight_id)}
                          class="px-2 py-1 text-xs bg-indigo-100 dark:bg-indigo-900 text-indigo-700 dark:text-indigo-300 rounded hover:bg-indigo-200"
                        >
                          View Flight
                        </button>
                        <button
                          onclick={() => resolveAnomaly(anomaly.id)}
                          class="px-2 py-1 text-xs bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300 rounded hover:bg-green-200"
                        >
                          Mark Resolved
                        </button>
                      </div>
                    </div>
                    <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">{anomaly.description}</p>
                    {#if anomaly.suggested_fix}
                      <p class="text-xs text-blue-600 dark:text-blue-400">
                        Suggestion: {anomaly.suggested_fix}
                      </p>
                    {/if}
                    <p class="text-xs text-gray-400 font-mono mt-2">Flight ID: {anomaly.flight_id.slice(0, 8)}...</p>
                  </div>
                {/each}
              </div>
            {/if}
          {:else}
            {#if duplicateCandidates.length === 0}
              <div class="text-center py-12 text-gray-500 flex flex-col items-center justify-center">
                <div class="text-4xl mb-4">✓</div>
                <p class="font-medium">No duplicate candidates</p>
                <p class="text-sm mt-1">No potential duplicates have been flagged</p>
              </div>
            {:else}
              <div class="space-y-4">
                {#each duplicateCandidates as dup}
                  <div class="bg-white dark:bg-gray-800 rounded-lg border border-amber-200 dark:border-amber-800 p-4">
                    <div class="flex items-center justify-between mb-2">
                      <div class="flex items-center gap-2">
                        <span class="px-2 py-1 text-xs font-medium bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200 rounded">
                          {Math.round(dup.similarity_score * 100)}% match
                        </span>
                        <span class="text-sm text-gray-600 dark:text-gray-400">Potential duplicate pair</span>
                      </div>
                    </div>
                    <div class="flex flex-wrap gap-2 mb-2">
                      {#each dup.match_reasons as reason}
                        <span class="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 rounded">
                          {reason}
                        </span>
                      {/each}
                    </div>
                    <div class="text-xs text-gray-400 font-mono">
                      Flight 1: {dup.flight_id_1.slice(0, 8)}... | Flight 2: {dup.flight_id_2.slice(0, 8)}...
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}
        </div>
      </div>
    {/if}

    <!-- Passengers Tab -->
    {#if activeTab === 'passengers'}
      <div class="h-full flex flex-col">
        <div class="p-4 border-b border-gray-200 dark:border-gray-700">
          <input
            type="text"
            placeholder="Search passengers..."
            bind:value={passengerSearch}
            class="px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white w-64"
          />
        </div>

        <div class="flex-1 overflow-auto p-4">
          {#if passengersLoading}
            <div class="flex items-center justify-center h-full">
              <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
            </div>
          {:else if filteredPassengers.length === 0}
            <div class="text-center py-12 text-gray-500">
              <p>No passengers found</p>
            </div>
          {:else}
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {#each filteredPassengers as passenger}
                <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
                  <div class="flex items-center justify-between mb-2">
                    <span class="font-semibold text-gray-900 dark:text-white">
                      {passenger.full_name || passenger.abbreviation}
                    </span>
                    <span class="text-sm text-gray-500">{passenger.usage_count} flights</span>
                  </div>
                  {#if passenger.full_name && passenger.full_name !== passenger.abbreviation}
                    <p class="text-xs text-gray-500 mb-3 font-mono">Abbr: {passenger.abbreviation}</p>
                  {/if}
                  <div class="flex gap-2">
                    <button
                      onclick={() => openPassengerEdit(passenger)}
                      class="px-2 py-1 text-xs bg-indigo-100 dark:bg-indigo-900 text-indigo-700 dark:text-indigo-300 rounded hover:bg-indigo-200 dark:hover:bg-indigo-800"
                    >
                      Rename
                    </button>
                    <button
                      onclick={() => removePassenger(passenger)}
                      class="px-2 py-1 text-xs bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300 rounded hover:bg-red-200 dark:hover:bg-red-800"
                    >
                      Remove
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Stats Tab -->
    {#if activeTab === 'stats'}
      <div class="h-full overflow-auto p-6">
        {#if statsLoading}
          <div class="flex items-center justify-center h-full">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-indigo-600"></div>
          </div>
        {:else if stats}
          <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-4 mb-8">
            <div class="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-4 text-center">
              <div class="text-3xl font-bold text-blue-600 dark:text-blue-400">{stats.total_flights}</div>
              <div class="text-sm text-blue-600/70 dark:text-blue-400/70">Total Flights</div>
            </div>
            <div class="bg-green-50 dark:bg-green-900/20 rounded-lg p-4 text-center">
              <div class="text-3xl font-bold text-green-600 dark:text-green-400">{stats.total_passengers}</div>
              <div class="text-sm text-green-600/70 dark:text-green-400/70">Unique Passengers</div>
            </div>
            <div class="bg-amber-50 dark:bg-amber-900/20 rounded-lg p-4 text-center">
              <div class="text-3xl font-bold text-amber-600 dark:text-amber-400">{stats.potential_duplicates}</div>
              <div class="text-sm text-amber-600/70 dark:text-amber-400/70">Potential Duplicates</div>
            </div>
            <div class="bg-purple-50 dark:bg-purple-900/20 rounded-lg p-4 text-center">
              <div class="text-3xl font-bold text-purple-600 dark:text-purple-400">{stats.flights_without_notes}</div>
              <div class="text-sm text-purple-600/70 dark:text-purple-400/70">Missing Notes</div>
            </div>
            <div class="bg-red-50 dark:bg-red-900/20 rounded-lg p-4 text-center">
              <div class="text-3xl font-bold text-red-600 dark:text-red-400">{stats.flights_without_distance}</div>
              <div class="text-sm text-red-600/70 dark:text-red-400/70">Missing Distance</div>
            </div>
          </div>

          <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-4">Data Quality Tips</h3>
            <ul class="space-y-3 text-sm text-gray-600 dark:text-gray-400">
              {#if stats.potential_duplicates > 0}
                <li class="flex items-start gap-2">
                  <span class="text-amber-500">⚠</span>
                  <span>You have {stats.potential_duplicates} potential duplicate flight(s). Check the Duplicates tab to review and merge them.</span>
                </li>
              {/if}
              {#if stats.flights_without_distance > 0}
                <li class="flex items-start gap-2">
                  <span class="text-red-500">⚠</span>
                  <span>{stats.flights_without_distance} flight(s) are missing distance data. This affects analytics accuracy.</span>
                </li>
              {/if}
              {#if stats.flights_without_notes > 0}
                <li class="flex items-start gap-2">
                  <span class="text-purple-500">ℹ</span>
                  <span>{stats.flights_without_notes} flight(s) have no notes. Adding passenger information improves analysis.</span>
                </li>
              {/if}
              {#if stats.potential_duplicates === 0 && stats.flights_without_distance === 0}
                <li class="flex items-start gap-2">
                  <span class="text-green-500">✓</span>
                  <span>Your data looks great! No critical issues found.</span>
                </li>
              {/if}
            </ul>
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<!-- Edit Flight Modal -->
{#if showEditModal && editingFlight}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-2xl max-h-[90vh] overflow-y-auto">
      <div class="p-4 border-b dark:border-gray-700 flex items-center justify-between">
        <h3 class="font-semibold text-gray-900 dark:text-white">Edit Flight</h3>
        <button onclick={() => showEditModal = false} class="text-gray-500 hover:text-gray-700">✕</button>
      </div>
      <div class="p-4 space-y-4">
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Flight Number</label>
            <input type="text" bind:value={editingFlight.flight_number} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Aircraft Registration</label>
            <input type="text" bind:value={editingFlight.aircraft_registration} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Departure Airport</label>
            <input type="text" bind:value={editingFlight.departure_airport} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Arrival Airport</label>
            <input type="text" bind:value={editingFlight.arrival_airport} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Departure Date/Time</label>
            <input type="datetime-local" bind:value={editingFlight.departure_datetime} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Arrival Date/Time</label>
            <input type="datetime-local" bind:value={editingFlight.arrival_datetime} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Distance (km)</label>
            <input type="number" bind:value={editingFlight.distance_km} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Duration (minutes)</label>
            <input type="number" bind:value={editingFlight.flight_duration} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Seat Number</label>
            <input type="text" bind:value={editingFlight.seat_number} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Fare Class</label>
            <input type="text" bind:value={editingFlight.fare_class} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Total Cost</label>
            <input type="number" step="0.01" bind:value={editingFlight.total_cost} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Currency</label>
            <input type="text" bind:value={editingFlight.currency} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Booking Reference</label>
            <input type="text" bind:value={editingFlight.booking_reference} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">CO2 Emissions (kg)</label>
            <input type="number" step="0.01" bind:value={editingFlight.carbon_emissions_kg} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Notes</label>
          <textarea bind:value={editingFlight.notes} rows="3" class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"></textarea>
        </div>
      </div>
      <div class="p-4 border-t dark:border-gray-700 flex justify-end gap-2">
        <button onclick={() => showEditModal = false} class="px-4 py-2 text-gray-600 hover:text-gray-800 dark:text-gray-400">Cancel</button>
        <button onclick={saveFlightEdit} class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700">Save Changes</button>
      </div>
    </div>
  </div>
{/if}

<!-- Rename Passenger Modal -->
{#if showPassengerModal && editingPassenger}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-md">
      <div class="p-4 border-b dark:border-gray-700 flex items-center justify-between">
        <h3 class="font-semibold text-gray-900 dark:text-white">Rename Passenger</h3>
        <button onclick={() => showPassengerModal = false} class="text-gray-500 hover:text-gray-700">✕</button>
      </div>
      <div class="p-4">
        <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
          This will rename "{editingPassenger.abbreviation}" in all {editingPassenger.usage_count} flight(s).
        </p>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">New Name</label>
        <input type="text" bind:value={newPassengerName} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
      </div>
      <div class="p-4 border-t dark:border-gray-700 flex justify-end gap-2">
        <button onclick={() => showPassengerModal = false} class="px-4 py-2 text-gray-600 hover:text-gray-800 dark:text-gray-400">Cancel</button>
        <button onclick={renamePassenger} class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700">Rename</button>
      </div>
    </div>
  </div>
{/if}
