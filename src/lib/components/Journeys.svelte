<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { translations } from '$lib/i18n';

  interface Journey {
    id: string;
    user_id: string;
    name: string;
    description: string | null;
    start_date: string;
    end_date: string | null;
    is_favorite: number;
    thumbnail_path: string | null;
    created_at: string;
    updated_at: string;
  }

  interface Flight {
    id: string;
    departure_airport: string;
    arrival_airport: string;
    departure_datetime: string;
    arrival_datetime: string | null;
    flight_number: string | null;
    distance_km: number | null;
    carbon_emissions_kg: number | null;
  }

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  let journeys: Journey[] = $state([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedJourney: Journey | null = $state(null);
  let journeyFlights: Flight[] = $state([]);
  let loadingFlights = $state(false);

  // Create/Edit journey state
  let showJourneyForm = $state(false);
  let editingJourney: Journey | null = $state(null);
  let formName = $state('');
  let formDescription = $state('');
  let formStartDate = $state('');
  let formEndDate = $state('');
  let saving = $state(false);

  // Add flight to journey
  let showFlightSelector = $state(false);
  let availableFlights: Flight[] = $state([]);
  let loadingAvailable = $state(false);
  let selectedFlightId = $state('');

  onMount(async () => {
    await loadJourneys();
  });

  async function loadJourneys() {
    loading = true;
    error = null;
    try {
      journeys = await invoke('list_user_journeys', { userId });
    } catch (err) {
      console.error('Failed to load journeys:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  async function loadJourneyFlights(journeyId: string) {
    loadingFlights = true;
    try {
      journeyFlights = await invoke('get_journey_flights', { journeyId });
    } catch (err) {
      console.error('Failed to load journey flights:', err);
      alert(`Failed to load flights: ${err}`);
    } finally {
      loadingFlights = false;
    }
  }

  async function selectJourney(journey: Journey) {
    selectedJourney = journey;
    await loadJourneyFlights(journey.id);
  }

  function openCreateForm() {
    editingJourney = null;
    formName = '';
    formDescription = '';
    formStartDate = '';
    formEndDate = '';
    showJourneyForm = true;
  }

  function openEditForm(journey: Journey) {
    editingJourney = journey;
    formName = journey.name;
    formDescription = journey.description || '';
    formStartDate = journey.start_date.split('T')[0];
    formEndDate = journey.end_date ? journey.end_date.split('T')[0] : '';
    showJourneyForm = true;
  }

  async function saveJourney() {
    if (!formName.trim() || !formStartDate) {
      alert('Please fill in journey name and start date');
      return;
    }

    saving = true;
    try {
      if (editingJourney) {
        // Update existing journey
        await invoke('update_journey', {
          journeyId: editingJourney.id,
          name: formName.trim(),
          description: formDescription.trim() || null,
          startDate: formStartDate,
          endDate: formEndDate || null,
          isFavorite: editingJourney.is_favorite
        });
      } else {
        // Create new journey
        await invoke('create_journey', {
          userId,
          name: formName.trim(),
          description: formDescription.trim() || null,
          startDate: formStartDate,
          endDate: formEndDate || null
        });
      }

      showJourneyForm = false;
      await loadJourneys();
    } catch (err) {
      console.error('Failed to save journey:', err);
      alert(`Failed to save journey: ${err}`);
    } finally {
      saving = false;
    }
  }

  async function toggleFavorite(journey: Journey) {
    try {
      await invoke('update_journey', {
        journeyId: journey.id,
        name: journey.name,
        description: journey.description,
        startDate: journey.start_date,
        endDate: journey.end_date,
        isFavorite: journey.is_favorite === 1 ? 0 : 1
      });
      await loadJourneys();
      if (selectedJourney?.id === journey.id) {
        selectedJourney = journeys.find(j => j.id === journey.id) || null;
      }
    } catch (err) {
      console.error('Failed to toggle favorite:', err);
      alert(`Failed to update: ${err}`);
    }
  }

  async function deleteJourney(journeyId: string) {
    if (!confirm('Are you sure you want to delete this journey? This will not delete the flights.')) {
      return;
    }

    try {
      await invoke('delete_journey', { journeyId });
      await loadJourneys();
      if (selectedJourney?.id === journeyId) {
        selectedJourney = null;
        journeyFlights = [];
      }
    } catch (err) {
      console.error('Failed to delete journey:', err);
      alert(`Failed to delete: ${err}`);
    }
  }

  async function loadAvailableFlights() {
    loadingAvailable = true;
    try {
      // Get all user flights
      const allFlights: Flight[] = await invoke('list_flights', { userId });

      // Filter out flights already in this journey
      const journeyFlightIds = new Set(journeyFlights.map(f => f.id));
      availableFlights = allFlights.filter(f => !journeyFlightIds.has(f.id));
    } catch (err) {
      console.error('Failed to load available flights:', err);
      alert(`Failed to load flights: ${err}`);
    } finally {
      loadingAvailable = false;
    }
  }

  async function addFlightToJourney() {
    if (!selectedJourney || !selectedFlightId) {
      return;
    }

    try {
      // Add flight with sequence order based on current flight count
      const sequenceOrder = journeyFlights.length;
      await invoke('add_flight_to_journey', {
        journeyId: selectedJourney.id,
        flightId: selectedFlightId,
        sequenceOrder
      });

      // Reload flights for this journey
      await loadJourneyFlights(selectedJourney.id);
      showFlightSelector = false;
      selectedFlightId = '';
    } catch (err) {
      console.error('Failed to add flight:', err);
      alert(`Failed to add flight: ${err}`);
    }
  }

  async function removeFlightFromJourney(flightId: string) {
    if (!selectedJourney) return;

    if (!confirm('Remove this flight from the journey?')) {
      return;
    }

    try {
      await invoke('remove_flight_from_journey', {
        journeyId: selectedJourney.id,
        flightId
      });
      await loadJourneyFlights(selectedJourney.id);
    } catch (err) {
      console.error('Failed to remove flight:', err);
      alert(`Failed to remove flight: ${err}`);
    }
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  }

  function calculateJourneyStats(flights: Flight[]) {
    const totalDistance = flights.reduce((sum, f) => sum + (f.distance_km || 0), 0);
    const totalCO2 = flights.reduce((sum, f) => sum + (f.carbon_emissions_kg || 0), 0);
    return { totalDistance, totalCO2, flightCount: flights.length };
  }

  $effect(() => {
    if (showFlightSelector && selectedJourney) {
      loadAvailableFlights();
    }
  });
</script>

<div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
      ✈️ {$translations('journeys.title')}
    </h2>
    <button
      onclick={openCreateForm}
      class="px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white rounded-lg font-medium transition"
    >
      + {$translations('journeys.createJourney')}
    </button>
  </div>

  {#if loading}
    <div class="text-center py-12">
      <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
      <p class="mt-2 text-gray-600 dark:text-gray-400">{$translations('common.loading')}...</p>
    </div>
  {:else if error}
    <div class="text-center py-12 text-red-600 dark:text-red-400">
      <p>{$translations('common.error')}: {error}</p>
    </div>
  {:else if journeys.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-600 dark:text-gray-400">{$translations('journeys.title')}</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Journey List -->
      <div class="lg:col-span-1 space-y-3">
        {#each journeys as journey}
          <div
            role="button"
            tabindex="0"
            class="border rounded-lg p-4 cursor-pointer transition {
              selectedJourney?.id === journey.id
                ? 'border-primary-600 bg-primary-50 dark:bg-primary-900/20'
                : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
            }"
            onclick={() => selectJourney(journey)}
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && selectJourney(journey)}
          >
            <div class="flex items-start justify-between mb-2">
              <h3 class="font-semibold text-gray-900 dark:text-white">
                {journey.name}
              </h3>
              <button
                onclick={(e) => { e.stopPropagation(); toggleFavorite(journey); }}
                class="text-xl {journey.is_favorite === 1 ? 'text-yellow-500' : 'text-gray-300 hover:text-yellow-500'}"
              >
                {journey.is_favorite === 1 ? '⭐' : '☆'}
              </button>
            </div>
            <p class="text-sm text-gray-600 dark:text-gray-400">
              {formatDate(journey.start_date)}
              {#if journey.end_date}
                → {formatDate(journey.end_date)}
              {/if}
            </p>
            {#if journey.description}
              <p class="text-sm text-gray-500 dark:text-gray-500 mt-2 line-clamp-2">
                {journey.description}
              </p>
            {/if}
          </div>
        {/each}
      </div>

      <!-- Journey Details -->
      <div class="lg:col-span-2">
        {#if selectedJourney}
          <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-6">
            <div class="flex items-start justify-between mb-4">
              <div>
                <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-2">
                  {selectedJourney.name}
                  {#if selectedJourney.is_favorite === 1}
                    <span class="text-yellow-500 ml-2">⭐</span>
                  {/if}
                </h3>
                <p class="text-sm text-gray-600 dark:text-gray-400">
                  {formatDate(selectedJourney.start_date)}
                  {#if selectedJourney.end_date}
                    → {formatDate(selectedJourney.end_date)}
                  {/if}
                </p>
                {#if selectedJourney.description}
                  <p class="text-gray-700 dark:text-gray-300 mt-2">
                    {selectedJourney.description}
                  </p>
                {/if}
              </div>
              <div class="flex gap-2">
                <button
                  onclick={() => openEditForm(selectedJourney!)}
                  class="px-3 py-1 text-sm bg-gray-600 hover:bg-gray-700 text-white rounded transition"
                >
                  {$translations('common.edit')}
                </button>
                <button
                  onclick={() => deleteJourney(selectedJourney!.id)}
                  class="px-3 py-1 text-sm bg-red-600 hover:bg-red-700 text-white rounded transition"
                >
                  {$translations('common.delete')}
                </button>
              </div>
            </div>

            {#if loadingFlights}
              <div class="text-center py-8">
                <div class="inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-primary-600"></div>
              </div>
            {:else}
              <!-- Journey Stats -->
              {#if journeyFlights.length > 0}
                {@const stats = calculateJourneyStats(journeyFlights)}
                <div class="grid grid-cols-3 gap-4 mb-6 p-4 bg-gray-50 dark:bg-gray-900 rounded-lg">
                  <div class="text-center">
                    <p class="text-2xl font-bold text-primary-600 dark:text-primary-400">
                      {stats.flightCount}
                    </p>
                    <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('navigation.flights')}</p>
                  </div>
                  <div class="text-center">
                    <p class="text-2xl font-bold text-primary-600 dark:text-primary-400">
                      {stats.totalDistance.toFixed(0)}
                    </p>
                    <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('units.km')}</p>
                  </div>
                  <div class="text-center">
                    <p class="text-2xl font-bold text-primary-600 dark:text-primary-400">
                      {stats.totalCO2.toFixed(1)}
                    </p>
                    <p class="text-xs text-gray-600 dark:text-gray-400">kg CO₂</p>
                  </div>
                </div>
              {/if}

              <!-- Flights in Journey -->
              <div class="mb-4">
                <div class="flex items-center justify-between mb-3">
                  <h4 class="font-semibold text-gray-900 dark:text-white">
                    {$translations('journeys.flights')} ({journeyFlights.length})
                  </h4>
                  <button
                    onclick={() => showFlightSelector = true}
                    class="px-3 py-1 text-sm bg-primary-600 hover:bg-primary-700 text-white rounded transition"
                  >
                    + {$translations('flights.addFlight')}
                  </button>
                </div>

                {#if journeyFlights.length === 0}
                  <p class="text-center py-8 text-gray-500 dark:text-gray-400">
                    {$translations('journeys.flights')}
                  </p>
                {:else}
                  <div class="space-y-2">
                    {#each journeyFlights as flight, index}
                      <div class="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-900 rounded-lg">
                        <div class="flex items-center gap-3">
                          <span class="text-gray-500 dark:text-gray-400 font-mono text-sm">
                            {index + 1}.
                          </span>
                          <div>
                            <p class="font-medium text-gray-900 dark:text-white">
                              {flight.departure_airport} → {flight.arrival_airport}
                              {#if flight.flight_number}
                                <span class="text-sm text-gray-500">({flight.flight_number})</span>
                              {/if}
                            </p>
                            <p class="text-sm text-gray-600 dark:text-gray-400">
                              {formatDate(flight.departure_datetime)}
                            </p>
                          </div>
                        </div>
                        <button
                          onclick={() => removeFlightFromJourney(flight.id)}
                          class="px-2 py-1 text-sm text-red-600 hover:text-red-700 dark:text-red-400"
                        >
                          {$translations('common.delete')}
                        </button>
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {:else}
          <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-12 text-center">
            <p class="text-gray-500 dark:text-gray-400">
              {$translations('journeys.title')}
            </p>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<!-- Create/Edit Journey Modal -->
{#if showJourneyForm}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-lg max-w-md w-full p-6">
      <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-4">
        {editingJourney ? $translations('common.edit') : $translations('journeys.createJourney')}
      </h3>

      <div class="space-y-4">
        <div>
          <label for="journey-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            {$translations('journeys.journeyName')} *
          </label>
          <input
            id="journey-name"
            type="text"
            bind:value={formName}
            placeholder="{$translations('journeys.journeyName')}..."
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          />
        </div>

        <div>
          <label for="journey-description" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            {$translations('common.help')}
          </label>
          <textarea
            id="journey-description"
            bind:value={formDescription}
            placeholder="{$translations('common.optional')}..."
            rows="3"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          ></textarea>
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="journey-start-date" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              {$translations('journeys.startDate')} *
            </label>
            <input
              id="journey-start-date"
              type="date"
              bind:value={formStartDate}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>

          <div>
            <label for="journey-end-date" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              {$translations('journeys.endDate')}
            </label>
            <input
              id="journey-end-date"
              type="date"
              bind:value={formEndDate}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
        </div>
      </div>

      <div class="flex gap-3 mt-6">
        <button
          onclick={() => showJourneyForm = false}
          class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition"
        >
          {$translations('common.cancel')}
        </button>
        <button
          onclick={saveJourney}
          disabled={saving}
          class="flex-1 px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white rounded-lg font-medium transition disabled:opacity-50"
        >
          {saving ? $translations('common.loading') : $translations('common.save')}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Add Flight Selector Modal -->
{#if showFlightSelector && selectedJourney}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-lg max-w-2xl w-full p-6 max-h-[80vh] overflow-y-auto">
      <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-4">
        {$translations('flights.addFlight')}
      </h3>

      {#if loadingAvailable}
        <div class="text-center py-8">
          <div class="inline-block animate-spin rounded-full h-6 w-6 border-b-2 border-primary-600"></div>
        </div>
      {:else if availableFlights.length === 0}
        <p class="text-center py-8 text-gray-500 dark:text-gray-400">
          {$translations('navigation.flights')}
        </p>
      {:else}
        <div class="space-y-2 mb-4">
          {#each availableFlights as flight}
            <label class="flex items-center p-3 border border-gray-200 dark:border-gray-700 rounded-lg cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700">
              <input
                type="radio"
                name="flight-select"
                value={flight.id}
                bind:group={selectedFlightId}
                class="mr-3"
              />
              <div class="flex-1">
                <p class="font-medium text-gray-900 dark:text-white">
                  {flight.departure_airport} → {flight.arrival_airport}
                  {#if flight.flight_number}
                    <span class="text-sm text-gray-500">({flight.flight_number})</span>
                  {/if}
                </p>
                <p class="text-sm text-gray-600 dark:text-gray-400">
                  {formatDate(flight.departure_datetime)}
                </p>
              </div>
            </label>
          {/each}
        </div>
      {/if}

      <div class="flex gap-3 mt-6">
        <button
          onclick={() => { showFlightSelector = false; selectedFlightId = ''; }}
          class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition"
        >
          {$translations('common.cancel')}
        </button>
        <button
          onclick={addFlightToJourney}
          disabled={!selectedFlightId}
          class="flex-1 px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white rounded-lg font-medium transition disabled:opacity-50"
        >
          {$translations('flights.addFlight')}
        </button>
      </div>
    </div>
  </div>
{/if}
