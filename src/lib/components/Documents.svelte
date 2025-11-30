<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import FlightDetail from './FlightDetail.svelte';

  interface InvestigationSummary {
    flight_id: string;
    flight_number: string | null;
    route: string;
    date: string;
    corroboration_score: number;
    passenger_names: string;
    created_at: string;
  }

  interface CustomDocument {
    id: string;
    user_id: string;
    title: string;
    content: string;
    category: string | null;
    tags: string | null;
    flight_id: string | null;
    journey_id: string | null;
    passenger_name: string | null;
    fuel_entry_id: string | null;
    created_at: string;
    updated_at: string;
  }

  interface Flight {
    id: string;
    flight_number: string | null;
    origin: string;
    destination: string;
    date: string;
  }

  interface Journey {
    id: string;
    name: string;
    description: string | null;
  }

  interface FuelEntry {
    id: string;
    date: string;
    fuel_type: string;
    quantity: number;
    unit: string;
    location: string | null;
  }

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  let investigations: InvestigationSummary[] = $state([]);
  let customDocuments: CustomDocument[] = $state([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedFlightId: string | null = $state(null);
  let selectedFlight: any = $state(null);

  // Entity lists for linking
  let flights: Flight[] = $state([]);
  let journeys: Journey[] = $state([]);
  let passengers: string[] = $state([]);
  let fuelEntries: FuelEntry[] = $state([]);

  // View mode
  let viewMode = $state<'investigations' | 'documents'>('investigations');

  // Custom documents state
  let showCreateModal = $state(false);
  let editingDocument: CustomDocument | null = $state(null);
  let formTitle = $state('');
  let formContent = $state('');
  let formCategory = $state('');
  let formFlightId = $state<string | null>(null);
  let formJourneyId = $state<string | null>(null);
  let formPassengerName = $state<string | null>(null);
  let formFuelEntryId = $state<string | null>(null);
  let saving = $state(false);

  // OCR import state
  let showOcrModal = $state(false);
  let ocrTitle = $state('');
  let ocrCategory = $state('');
  let ocrImagePath = $state('');
  let ocrProcessing = $state(false);
  let ocrProgress = $state('');

  onMount(async () => {
    await Promise.all([
      loadInvestigations(),
      loadCustomDocuments(),
      loadEntityLists(),
    ]);
  });

  async function loadInvestigations() {
    loading = true;
    error = null;
    try {
      investigations = await invoke('list_all_investigations', { userId });
    } catch (err) {
      console.error('Failed to load investigations:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  async function loadCustomDocuments() {
    try {
      customDocuments = await invoke('list_user_documents', { userId });
    } catch (err) {
      console.error('Failed to load custom documents:', err);
    }
  }

  async function loadEntityLists() {
    try {
      // Load all entity lists in parallel
      const [flightList, journeyList, passengerList, fuelList] = await Promise.all([
        invoke<Flight[]>('list_flights', { userId }),
        invoke<Journey[]>('list_user_journeys', { userId }),
        invoke<string[]>('get_all_passenger_names', { userId }),
        invoke<FuelEntry[]>('get_fuel_entries', { userId }),
      ]);
      flights = flightList || [];
      journeys = journeyList || [];
      passengers = passengerList || [];
      fuelEntries = fuelList || [];
    } catch (err) {
      console.error('Failed to load entity lists:', err);
    }
  }

  async function viewFlight(flightId: string) {
    try {
      selectedFlight = await invoke('get_flight', { flightId });
    } catch (err) {
      console.error('Failed to load flight:', err);
      alert(`Failed to load flight: ${err}`);
    }
  }

  function formatDate(dateStr: string) {
    return new Date(dateStr).toLocaleDateString();
  }

  function getScoreColor(score: number): string {
    if (score >= 0.8) return 'text-green-600 dark:text-green-400';
    if (score >= 0.6) return 'text-yellow-600 dark:text-yellow-400';
    if (score >= 0.4) return 'text-orange-600 dark:text-orange-400';
    return 'text-red-600 dark:text-red-400';
  }

  function getScoreBadge(score: number): string {
    if (score >= 0.8) return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
    if (score >= 0.6) return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
    if (score >= 0.4) return 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200';
    return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
  }

  async function saveDocument() {
    if (!formTitle.trim() || !formContent.trim()) {
      alert('Please enter both title and content');
      return;
    }

    saving = true;
    try {
      const documentData = {
        title: formTitle,
        content: formContent,
        category: formCategory || null,
        tags: null,
        flight_id: formFlightId || null,
        journey_id: formJourneyId || null,
        passenger_name: formPassengerName || null,
        fuel_entry_id: formFuelEntryId || null,
      };

      if (editingDocument) {
        await invoke('update_custom_document', {
          documentId: editingDocument.id,
          document: documentData,
        });
      } else {
        await invoke('create_custom_document', {
          userId,
          document: documentData,
        });
      }

      // Reset form
      resetForm();
      showCreateModal = false;

      // Reload documents
      await loadCustomDocuments();
    } catch (err) {
      console.error('Failed to save document:', err);
      alert(`Failed to save document: ${err}`);
    } finally {
      saving = false;
    }
  }

  function resetForm() {
    formTitle = '';
    formContent = '';
    formCategory = '';
    formFlightId = null;
    formJourneyId = null;
    formPassengerName = null;
    formFuelEntryId = null;
    editingDocument = null;
  }

  function openEditModal(doc: CustomDocument) {
    editingDocument = doc;
    formTitle = doc.title;
    formContent = doc.content;
    formCategory = doc.category || '';
    formFlightId = doc.flight_id;
    formJourneyId = doc.journey_id;
    formPassengerName = doc.passenger_name;
    formFuelEntryId = doc.fuel_entry_id;
    showCreateModal = true;
  }

  async function deleteDocument(docId: string) {
    if (!confirm('Are you sure you want to delete this document?')) return;

    try {
      await invoke('delete_custom_document', { documentId: docId });
      await loadCustomDocuments();
    } catch (err) {
      console.error('Failed to delete document:', err);
      alert(`Failed to delete document: ${err}`);
    }
  }

  function getFlightLabel(flightId: string): string {
    const flight = flights.find(f => f.id === flightId);
    if (!flight) return flightId;
    return `${flight.flight_number || 'N/A'} - ${flight.origin}→${flight.destination} (${flight.date})`;
  }

  function getJourneyLabel(journeyId: string): string {
    const journey = journeys.find(j => j.id === journeyId);
    return journey?.name || journeyId;
  }

  function getFuelEntryLabel(entryId: string): string {
    const entry = fuelEntries.find(f => f.id === entryId);
    if (!entry) return entryId;
    return `${entry.fuel_type} - ${entry.quantity} ${entry.unit} (${entry.date})`;
  }

  async function selectImageForOcr() {
    try {
      const selected = await open({
        title: 'Select Document Image',
        multiple: false,
        filters: [{
          name: 'Images',
          extensions: ['jpg', 'jpeg', 'png', 'webp', 'gif']
        }]
      });

      if (selected) {
        ocrImagePath = selected as string;
        showOcrModal = true;
      }
    } catch (err) {
      console.error('Failed to select image:', err);
      alert(`Failed to select image: ${err}`);
    }
  }

  async function processOcrImport() {
    if (!ocrImagePath) {
      alert('No image selected');
      return;
    }

    ocrProcessing = true;
    ocrProgress = 'Extracting text from image using Gemini 2.5 Flash Lite...';

    try {
      const documentId = await invoke('import_document_with_ocr', {
        userId,
        imagePath: ocrImagePath,
        title: ocrTitle,
        category: ocrCategory || null,
      });

      // Reset form
      ocrTitle = '';
      ocrCategory = '';
      ocrImagePath = '';
      ocrProgress = '';
      showOcrModal = false;

      alert('Document imported successfully with OCR!');
    } catch (err) {
      console.error('Failed to import document with OCR:', err);
      alert(`Failed to import document: ${err}`);
      ocrProgress = '';
    } finally {
      ocrProcessing = false;
    }
  }
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="mb-6 flex items-center justify-between">
    <div>
      <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-2">Documents & Notes</h2>
      <p class="text-gray-600 dark:text-gray-400">
        Flight investigations and custom notes
      </p>
    </div>
    <div class="flex gap-3">
      <button
        onclick={selectImageForOcr}
        class="bg-green-600 hover:bg-green-700 text-white px-6 py-2 rounded-lg transition font-medium flex items-center gap-2"
      >
        📄 Import with OCR
      </button>
      <button
        onclick={() => { resetForm(); showCreateModal = true; }}
        class="bg-primary-600 hover:bg-primary-700 text-white px-6 py-2 rounded-lg transition font-medium"
      >
        ➕ Create Document
      </button>
    </div>
  </div>

  <!-- View Mode Tabs -->
  <div class="mb-4 flex gap-2">
    <button
      onclick={() => viewMode = 'investigations'}
      class="px-4 py-2 rounded-lg font-medium transition {
        viewMode === 'investigations'
          ? 'bg-primary-600 text-white'
          : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
      }"
    >
      🔍 Investigations ({investigations.length})
    </button>
    <button
      onclick={() => viewMode = 'documents'}
      class="px-4 py-2 rounded-lg font-medium transition {
        viewMode === 'documents'
          ? 'bg-primary-600 text-white'
          : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'
      }"
    >
      📝 Custom Documents ({customDocuments.length})
    </button>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
    </div>
  {:else if viewMode === 'documents'}
    <!-- Custom Documents View -->
    {#if customDocuments.length === 0}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
        <div class="text-6xl mb-4">📝</div>
        <h3 class="text-xl font-semibold mb-2 text-gray-900 dark:text-white">No Documents Yet</h3>
        <p class="text-gray-600 dark:text-gray-400 mb-6">
          Create custom documents to store notes, receipts, and other travel info
        </p>
        <button
          onclick={() => { resetForm(); showCreateModal = true; }}
          class="bg-primary-600 hover:bg-primary-700 text-white px-6 py-2 rounded-lg transition font-medium"
        >
          ➕ Create First Document
        </button>
      </div>
    {:else}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden flex-1">
        <div class="overflow-x-auto">
          <table class="w-full">
            <thead class="bg-gray-50 dark:bg-gray-900">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Title</th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Category</th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Linked To</th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Created</th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
              {#each customDocuments as doc}
                <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                  <td class="px-6 py-4 text-sm font-medium text-gray-900 dark:text-gray-100">
                    {doc.title}
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                    {doc.category || '—'}
                  </td>
                  <td class="px-6 py-4 text-sm text-gray-600 dark:text-gray-400">
                    <div class="flex flex-wrap gap-1">
                      {#if doc.flight_id}
                        <span class="px-2 py-0.5 bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 rounded text-xs">✈️ Flight</span>
                      {/if}
                      {#if doc.journey_id}
                        <span class="px-2 py-0.5 bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200 rounded text-xs">🗺️ Journey</span>
                      {/if}
                      {#if doc.passenger_name}
                        <span class="px-2 py-0.5 bg-purple-100 dark:bg-purple-900 text-purple-800 dark:text-purple-200 rounded text-xs">👤 {doc.passenger_name}</span>
                      {/if}
                      {#if doc.fuel_entry_id}
                        <span class="px-2 py-0.5 bg-orange-100 dark:bg-orange-900 text-orange-800 dark:text-orange-200 rounded text-xs">⛽ Fuel</span>
                      {/if}
                      {#if !doc.flight_id && !doc.journey_id && !doc.passenger_name && !doc.fuel_entry_id}
                        <span class="text-gray-400">—</span>
                      {/if}
                    </div>
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                    {formatDate(doc.created_at)}
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap text-sm">
                    <div class="flex gap-2">
                      <button
                        onclick={() => openEditModal(doc)}
                        class="bg-primary-600 hover:bg-primary-700 text-white px-3 py-1 rounded text-xs font-medium transition"
                      >
                        Edit
                      </button>
                      <button
                        onclick={() => deleteDocument(doc.id)}
                        class="bg-red-600 hover:bg-red-700 text-white px-3 py-1 rounded text-xs font-medium transition"
                      >
                        Delete
                      </button>
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900">
          <div class="text-sm text-gray-600 dark:text-gray-400">
            <span class="font-semibold text-gray-900 dark:text-white">{customDocuments.length}</span>
            {customDocuments.length === 1 ? 'document' : 'documents'}
          </div>
        </div>
      </div>
    {/if}
  {:else if error}
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
      <div class="text-red-500 text-4xl mb-4">⚠️</div>
      <p class="text-red-600 dark:text-red-400 font-medium">Failed to load investigations</p>
      <p class="text-gray-600 dark:text-gray-400 text-sm mt-2">{error}</p>
      <button
        onclick={loadInvestigations}
        class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg transition"
      >
        Try Again
      </button>
    </div>
  {:else if investigations.length === 0}
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
      <div class="text-6xl mb-4">🔍</div>
      <h3 class="text-xl font-semibold mb-2 text-gray-900 dark:text-white">No Investigations Yet</h3>
      <p class="text-gray-600 dark:text-gray-400 mb-6">
        Run investigations on your flights to cross-reference with public records
      </p>
      <p class="text-sm text-gray-500 dark:text-gray-400">
        Click the "🔍 Investigate" button on any flight to get started
      </p>
    </div>
  {:else}
    <!-- Documents List -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden flex-1">
      <div class="overflow-x-auto">
        <table class="w-full">
          <thead class="bg-gray-50 dark:bg-gray-900">
            <tr>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Date
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Flight
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Route
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Passengers
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Corroboration
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Investigated
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                Actions
              </th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
            {#each investigations as inv}
              <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                  {formatDate(inv.date)}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-gray-100">
                  {inv.flight_number || 'N/A'}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100">
                  {inv.route}
                </td>
                <td class="px-6 py-4 text-sm text-gray-600 dark:text-gray-400 max-w-xs truncate">
                  {inv.passenger_names}
                </td>
                <td class="px-6 py-4 whitespace-nowrap">
                  <div class="flex items-center gap-2">
                    <span class="px-2.5 py-0.5 rounded-full text-xs font-medium {getScoreBadge(inv.corroboration_score)}">
                      {Math.round(inv.corroboration_score * 100)}%
                    </span>
                  </div>
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                  {formatDate(inv.created_at)}
                </td>
                <td class="px-6 py-4 whitespace-nowrap text-sm">
                  <button
                    onclick={() => viewFlight(inv.flight_id)}
                    class="bg-primary-600 hover:bg-primary-700 text-white px-3 py-1 rounded text-xs font-medium transition"
                  >
                    View Flight
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <!-- Summary Footer -->
      <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900">
        <div class="flex items-center justify-between text-sm text-gray-600 dark:text-gray-400">
          <div>
            <span class="font-semibold text-gray-900 dark:text-white">{investigations.length}</span>
            {investigations.length === 1 ? 'investigation' : 'investigations'} performed
          </div>
          <div>
            Average corroboration:
            <span class="font-semibold {getScoreColor(investigations.reduce((sum, i) => sum + i.corroboration_score, 0) / investigations.length)}">
              {Math.round((investigations.reduce((sum, i) => sum + i.corroboration_score, 0) / investigations.length) * 100)}%
            </span>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Flight Detail Modal -->
  {#if selectedFlight}
    <FlightDetail
      flight={selectedFlight}
      onClose={() => selectedFlight = null}
      onInvestigate={() => {}}
    />
  {/if}

  <!-- Create/Edit Document Modal -->
  {#if showCreateModal}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" onclick={() => { resetForm(); showCreateModal = false; }}>
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-3xl mx-4 max-h-[90vh] overflow-y-auto" onclick={(e) => e.stopPropagation()}>
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-xl font-bold text-gray-900 dark:text-white">
            {editingDocument ? 'Edit Document' : 'Create New Document'}
          </h3>
          <button onclick={() => { resetForm(); showCreateModal = false; }} class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
            ✕
          </button>
        </div>

        <form onsubmit={(e) => { e.preventDefault(); saveDocument(); }}>
          <div class="space-y-4">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label for="title" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Title *
                </label>
                <input
                  type="text"
                  id="title"
                  bind:value={formTitle}
                  placeholder="Enter document title..."
                  required
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white"
                />
              </div>

              <div>
                <label for="category" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Category
                </label>
                <input
                  type="text"
                  id="category"
                  bind:value={formCategory}
                  placeholder="Notes, Planning, Reports..."
                  list="category-suggestions"
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white"
                />
                <datalist id="category-suggestions">
                  <option value="Notes">
                  <option value="Planning">
                  <option value="Receipt">
                  <option value="Report">
                  <option value="Itinerary">
                  <option value="Booking">
                  <option value="Insurance">
                  <option value="Visa">
                </datalist>
              </div>
            </div>

            <!-- Entity Linking Section -->
            <div class="border-t border-gray-200 dark:border-gray-700 pt-4">
              <h4 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">Link to Entities (optional)</h4>
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <!-- Flight Link -->
                <div>
                  <label for="flight-link" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    ✈️ Flight
                  </label>
                  <select
                    id="flight-link"
                    bind:value={formFlightId}
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white"
                  >
                    <option value={null}>— No flight linked —</option>
                    {#each flights as flight}
                      <option value={flight.id}>
                        {flight.flight_number || 'N/A'} - {flight.origin}→{flight.destination} ({flight.date})
                      </option>
                    {/each}
                  </select>
                </div>

                <!-- Journey Link -->
                <div>
                  <label for="journey-link" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    🗺️ Journey
                  </label>
                  <select
                    id="journey-link"
                    bind:value={formJourneyId}
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white"
                  >
                    <option value={null}>— No journey linked —</option>
                    {#each journeys as journey}
                      <option value={journey.id}>{journey.name}</option>
                    {/each}
                  </select>
                </div>

                <!-- Passenger Link -->
                <div>
                  <label for="passenger-link" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    👤 Passenger
                  </label>
                  <select
                    id="passenger-link"
                    bind:value={formPassengerName}
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white"
                  >
                    <option value={null}>— No passenger linked —</option>
                    {#each passengers as passenger}
                      <option value={passenger}>{passenger}</option>
                    {/each}
                  </select>
                </div>

                <!-- Fuel Entry Link -->
                <div>
                  <label for="fuel-link" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    ⛽ Fuel Entry
                  </label>
                  <select
                    id="fuel-link"
                    bind:value={formFuelEntryId}
                    class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white"
                  >
                    <option value={null}>— No fuel entry linked —</option>
                    {#each fuelEntries as entry}
                      <option value={entry.id}>
                        {entry.fuel_type} - {entry.quantity} {entry.unit} ({entry.date})
                      </option>
                    {/each}
                  </select>
                </div>
              </div>
            </div>

            <div>
              <label for="content" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Content *
              </label>
              <textarea
                id="content"
                bind:value={formContent}
                placeholder="Enter your notes here..."
                required
                rows="8"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white resize-none"
              ></textarea>
            </div>
          </div>

          <div class="flex justify-end gap-3 mt-6">
            <button
              type="button"
              onclick={() => { resetForm(); showCreateModal = false; }}
              class="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              class="px-4 py-2 bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 text-white rounded-lg transition"
            >
              {saving ? 'Saving...' : editingDocument ? 'Update Document' : 'Create Document'}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  <!-- OCR Import Modal -->
  {#if showOcrModal}
    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50" onclick={() => !ocrProcessing && (showOcrModal = false)}>
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-2xl mx-4" onclick={(e) => e.stopPropagation()}>
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-xl font-bold text-gray-900 dark:text-white">Import Document with OCR</h3>
          {#if !ocrProcessing}
            <button onclick={() => showOcrModal = false} class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">
              ✕
            </button>
          {/if}
        </div>

        <div class="mb-4 p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
          <p class="text-sm text-blue-800 dark:text-blue-200">
            <strong>Using Gemini 2.5 Flash Lite</strong> - Ultra-cheap OCR at €0.10/M input tokens, €0.40/M output tokens
          </p>
          <p class="text-sm text-blue-700 dark:text-blue-300 mt-1">
            Selected image: <span class="font-mono">{ocrImagePath.split('/').pop()}</span>
          </p>
        </div>

        <form onsubmit={(e) => { e.preventDefault(); processOcrImport(); }}>
          <div class="space-y-4">
            <div>
              <label for="ocr-title" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Document Title (optional)
              </label>
              <input
                type="text"
                id="ocr-title"
                bind:value={ocrTitle}
                placeholder="Leave blank to auto-generate from filename"
                disabled={ocrProcessing}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white disabled:bg-gray-100 dark:disabled:bg-gray-700"
              />
            </div>

            <div>
              <label for="ocr-category" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                Category (optional)
              </label>
              <input
                type="text"
                id="ocr-category"
                bind:value={ocrCategory}
                placeholder="OCR Import, Scanned Documents, etc."
                disabled={ocrProcessing}
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white disabled:bg-gray-100 dark:disabled:bg-gray-700"
              />
            </div>

            {#if ocrProgress}
              <div class="p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg">
                <div class="flex items-center gap-3">
                  <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-yellow-600"></div>
                  <p class="text-sm text-yellow-800 dark:text-yellow-200">{ocrProgress}</p>
                </div>
              </div>
            {/if}
          </div>

          <div class="flex justify-end gap-3 mt-6">
            {#if !ocrProcessing}
              <button
                type="button"
                onclick={() => showOcrModal = false}
                class="px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 transition"
              >
                Cancel
              </button>
            {/if}
            <button
              type="submit"
              disabled={ocrProcessing}
              class="px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-400 text-white rounded-lg transition flex items-center gap-2"
            >
              {#if ocrProcessing}
                <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
                Processing...
              {:else}
                📄 Extract Text & Save
              {/if}
            </button>
          </div>
        </form>
      </div>
    </div>
  {/if}
</div>
