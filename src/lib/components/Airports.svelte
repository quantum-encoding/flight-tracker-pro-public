<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { translations } from '$lib/i18n';
  import { theme } from '$lib/theme';

  interface Airport {
    id: string;
    icao_code: string | null;
    iata_code: string | null;
    name: string;
    city: string | null;
    country: string | null;
    latitude: number | null;
    longitude: number | null;
    timezone: string | null;
    website: string | null;
    elevation_ft: number | null;
  }

  interface CsvAirportData {
    ident: string;
    iata_code: string | null;
    name: string;
    latitude_deg: number | null;
    longitude_deg: number | null;
    municipality: string | null;
    iso_country: string | null;
  }

  interface AirportSearchResult {
    airport: CsvAirportData;
    match_type: string;
    similarity_score: number;
  }

  interface AirportLookupResponse {
    exact_match: CsvAirportData | null;
    suggestions: AirportSearchResult[];
  }

  let airports = $state<Airport[]>([]);
  let loading = $state(true);
  let showModal = $state(false);
  let editingAirport = $state<Airport | null>(null);
  let selectedAirport = $state<Airport | null>(null);

  // Form fields
  let formIcaoCode = $state('');
  let formIataCode = $state('');
  let formName = $state('');
  let formCity = $state('');
  let formCountry = $state('');
  let formLatitude = $state<number | null>(null);
  let formLongitude = $state<number | null>(null);
  let formTimezone = $state('');
  let formWebsite = $state('');
  let formElevation = $state<number | null>(null);

  // Airport lookup/autocomplete
  let lookupSuggestions = $state<AirportSearchResult[]>([]);
  let showSuggestions = $state(false);
  let lookupLoading = $state(false);
  let lookupDebounceTimer: ReturnType<typeof setTimeout> | null = null;
  let csvAirportCount = $state(0);

  // Search and filter
  let searchQuery = $state('');
  let filterCountry = $state('');
  let sortBy = $state<'name' | 'icao' | 'country' | 'city'>('name');

  // Import state
  let importing = $state(false);
  let smartImportResult = $state<{ codes_found: number; already_in_db: number; imported: number; not_found_in_csv: string[] } | null>(null);
  let visitedAirportCount = $state(0);

  // Get unique countries for filter
  let countries = $derived.by(() => {
    const countrySet = new Set<string>();
    airports.forEach(a => {
      if (a.country) countrySet.add(a.country);
    });
    return Array.from(countrySet).sort();
  });

  let filteredAirports = $derived.by(() => {
    let result = airports;

    // Apply search filter
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      result = result.filter(a =>
        a.name.toLowerCase().includes(query) ||
        a.icao_code?.toLowerCase().includes(query) ||
        a.iata_code?.toLowerCase().includes(query) ||
        a.city?.toLowerCase().includes(query) ||
        a.country?.toLowerCase().includes(query)
      );
    }

    // Apply country filter
    if (filterCountry) {
      result = result.filter(a => a.country === filterCountry);
    }

    // Apply sorting
    result = [...result].sort((a, b) => {
      switch (sortBy) {
        case 'icao':
          return (a.icao_code || '').localeCompare(b.icao_code || '');
        case 'country':
          return (a.country || '').localeCompare(b.country || '');
        case 'city':
          return (a.city || '').localeCompare(b.city || '');
        default:
          return a.name.localeCompare(b.name);
      }
    });

    return result;
  });

  onMount(async () => {
    await loadAirports();
    // Get CSV airport count and visited airport count for info display
    try {
      csvAirportCount = await invoke('get_csv_airport_count');
      const visitedCodes = await invoke<string[]>('get_visited_airport_codes');
      visitedAirportCount = visitedCodes.length;
    } catch (e) {
      console.warn('Could not load airport counts:', e);
    }
  });

  async function loadAirports() {
    loading = true;
    try {
      airports = await invoke('list_all_airports');
    } catch (error) {
      console.error('Failed to load airports:', error);
    } finally {
      loading = false;
    }
  }

  // Lookup airport from CSV with debounce
  async function lookupAirportCode(code: string) {
    if (lookupDebounceTimer) clearTimeout(lookupDebounceTimer);

    if (!code || code.length < 2) {
      lookupSuggestions = [];
      showSuggestions = false;
      return;
    }

    lookupDebounceTimer = setTimeout(async () => {
      lookupLoading = true;
      try {
        const response = await invoke<AirportLookupResponse>('lookup_airport', { code });

        if (response.exact_match) {
          // Auto-fill the form with exact match
          applyAirportData(response.exact_match);
          lookupSuggestions = [];
          showSuggestions = false;
        } else if (response.suggestions.length > 0) {
          lookupSuggestions = response.suggestions;
          showSuggestions = true;
        } else {
          lookupSuggestions = [];
          showSuggestions = false;
        }
      } catch (e) {
        console.error('Airport lookup failed:', e);
      } finally {
        lookupLoading = false;
      }
    }, 300);
  }

  // Apply data from CSV airport to form
  function applyAirportData(airport: CsvAirportData) {
    formIcaoCode = airport.ident || '';
    formIataCode = airport.iata_code || '';
    formName = airport.name || '';
    formCity = airport.municipality || '';
    formCountry = airport.iso_country || '';
    formLatitude = airport.latitude_deg;
    formLongitude = airport.longitude_deg;
    showSuggestions = false;
  }

  // Select a suggestion
  function selectSuggestion(result: AirportSearchResult) {
    applyAirportData(result.airport);
  }

  function openCreateModal() {
    editingAirport = null;
    resetForm();
    showModal = true;
  }

  function openEditModal(airport: Airport) {
    editingAirport = airport;
    formIcaoCode = airport.icao_code || '';
    formIataCode = airport.iata_code || '';
    formName = airport.name;
    formCity = airport.city || '';
    formCountry = airport.country || '';
    formLatitude = airport.latitude;
    formLongitude = airport.longitude;
    formTimezone = airport.timezone || '';
    formWebsite = (airport as any).website || '';
    formElevation = (airport as any).elevation_ft || null;
    showModal = true;
  }

  function resetForm() {
    formIcaoCode = '';
    formIataCode = '';
    formName = '';
    formCity = '';
    formCountry = '';
    formLatitude = null;
    formLongitude = null;
    formTimezone = '';
    formWebsite = '';
    formElevation = null;
    // Clear lookup state
    lookupSuggestions = [];
    showSuggestions = false;
  }

  async function saveAirport() {
    try {
      const airport = {
        icao_code: formIcaoCode || null,
        iata_code: formIataCode || null,
        name: formName,
        city: formCity || null,
        country: formCountry || null,
        latitude: formLatitude,
        longitude: formLongitude,
        timezone: formTimezone || null,
      };

      if (editingAirport) {
        await invoke('update_airport', {
          airportId: editingAirport.id,
          airport
        });
      } else {
        await invoke('create_airport', { airport });
      }

      showModal = false;
      await loadAirports();
    } catch (error) {
      console.error('Failed to save airport:', error);
      alert(`Failed to save airport: ${error}`);
    }
  }

  async function deleteAirport(id: string) {
    if (!confirm('Are you sure you want to delete this airport?')) return;

    try {
      await invoke('delete_airport', { airportId: id });
      selectedAirport = null;
      await loadAirports();
    } catch (error) {
      console.error('Failed to delete airport:', error);
      alert(`Failed to delete airport: ${error}`);
    }
  }

  function getGoogleMapsUrl(lat: number, lng: number): string {
    return `https://www.google.com/maps?q=${lat},${lng}`;
  }

  function getOpenStreetMapUrl(lat: number, lng: number): string {
    return `https://www.openstreetmap.org/?mlat=${lat}&mlon=${lng}&zoom=14`;
  }

  async function importVisitedAirports() {
    importing = true;
    smartImportResult = null;
    try {
      const result = await invoke<{ codes_found: number; already_in_db: number; imported: number; not_found_in_csv: string[] }>('import_visited_airports');
      smartImportResult = result;
      // Reload airports list after import
      await loadAirports();
    } catch (e) {
      console.error('Failed to import airports:', e);
      alert(`Failed to import airports: ${e}`);
    } finally {
      importing = false;
    }
  }

  function getAirportWebsite(airport: Airport): string | null {
    // Known airport websites based on ICAO codes
    const knownWebsites: Record<string, string> = {
      'KJFK': 'https://www.jfkairport.com',
      'KLAX': 'https://www.flylax.com',
      'EGLL': 'https://www.heathrow.com',
      'LFPG': 'https://www.parisaeroport.fr',
      'EDDF': 'https://www.frankfurt-airport.com',
      'EHAM': 'https://www.schiphol.nl',
      'LEMD': 'https://www.aena.es/en/madrid-barajas.html',
      'LFPO': 'https://www.parisaeroport.fr',
      'EGKK': 'https://www.gatwickairport.com',
      'LEBL': 'https://www.aena.es/en/barcelona-el-prat.html',
      'LIRF': 'https://www.adr.it/fiumicino',
      'EDDM': 'https://www.munich-airport.com',
      'LOWW': 'https://www.viennaairport.com',
      'LSZH': 'https://www.zurich-airport.com',
      'EKCH': 'https://www.cph.dk',
      'ENGM': 'https://avinor.no/en/airport/oslo-airport/',
      'ESSA': 'https://www.swedavia.com/arlanda/',
      'EFHK': 'https://www.finavia.fi/en/helsinki-airport',
      'EIDW': 'https://www.dublinairport.com',
      'LPPT': 'https://www.aeroportolisboa.pt',
      'LGAV': 'https://www.aia.gr',
      'LTFM': 'https://www.igairport.com',
      'OMDB': 'https://www.dubaiairports.ae',
      'VHHH': 'https://www.hongkongairport.com',
      'WSSS': 'https://www.changiairport.com',
      'RJTT': 'https://www.tokyo-airport-bldg.co.jp/en/',
      'RKSI': 'https://www.airport.kr/ap/en/index.do',
      'ZBAA': 'https://en.bcia.com.cn',
      'YSSY': 'https://www.sydneyairport.com.au',
      'NZAA': 'https://www.aucklandairport.co.nz',
      'CYYZ': 'https://www.torontopearson.com',
      'CYVR': 'https://www.yvr.ca',
      'MMMX': 'https://www.aicm.com.mx',
      'SBGR': 'https://www.gru.com.br',
    };

    if (airport.icao_code && knownWebsites[airport.icao_code]) {
      return knownWebsites[airport.icao_code];
    }
    return null;
  }
</script>

<div class="p-6 max-w-7xl mx-auto">
  <!-- Header -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 mb-6">
    <div>
      <h2 class="text-2xl font-bold text-gray-900 dark:text-white">{$translations('airports.title')}</h2>
      <p class="text-gray-600 dark:text-gray-400 mt-1">
        {airports.length} airports in database
        {#if visitedAirportCount > 0}
          <span class="text-primary-500"> · {visitedAirportCount} unique airports in flight logs</span>
        {/if}
      </p>
    </div>
    <div class="flex gap-2 self-start">
      <button
        onclick={importVisitedAirports}
        disabled={importing}
        class="bg-emerald-600 hover:bg-emerald-700 disabled:bg-emerald-400 text-white px-4 py-2.5 rounded-lg font-medium transition flex items-center gap-2"
        title="Import only airports that appear in your flight logs"
      >
        {#if importing}
          <span class="animate-spin">⏳</span> Importing...
        {:else}
          📥 Import Visited Airports
        {/if}
      </button>
      <button
        onclick={openCreateModal}
        class="bg-primary-600 hover:bg-primary-700 text-white px-6 py-2.5 rounded-lg font-medium transition flex items-center gap-2"
      >
        <span class="text-lg">+</span> {$translations('airports.addAirport')}
      </button>
    </div>
  </div>

  <!-- Import Result -->
  {#if smartImportResult}
    <div class="mb-6 p-4 rounded-lg {smartImportResult.not_found_in_csv.length > 0 ? 'bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700' : 'bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-700'}">
      <div class="flex items-start justify-between">
        <div class="flex-1">
          <h4 class="font-medium {smartImportResult.not_found_in_csv.length > 0 ? 'text-amber-800 dark:text-amber-300' : 'text-emerald-800 dark:text-emerald-300'}">
            Smart Import Complete
          </h4>
          <p class="text-sm mt-1 {smartImportResult.not_found_in_csv.length > 0 ? 'text-amber-700 dark:text-amber-400' : 'text-emerald-700 dark:text-emerald-400'}">
            Found {smartImportResult.codes_found} unique airports in flight logs.
            Imported {smartImportResult.imported} new airports,
            {smartImportResult.already_in_db} already in database.
          </p>
          {#if smartImportResult.not_found_in_csv.length > 0}
            <div class="mt-2">
              <p class="text-sm text-amber-700 dark:text-amber-400">
                ⚠️ {smartImportResult.not_found_in_csv.length} codes not found in CSV:
              </p>
              <div class="flex flex-wrap gap-1 mt-1">
                {#each smartImportResult.not_found_in_csv as code}
                  <span class="px-2 py-0.5 bg-amber-100 dark:bg-amber-900/40 text-amber-800 dark:text-amber-300 rounded text-xs font-mono">
                    {code}
                  </span>
                {/each}
              </div>
            </div>
          {/if}
        </div>
        <button
          onclick={() => smartImportResult = null}
          class="text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 ml-4"
        >
          ✕
        </button>
      </div>
    </div>
  {/if}

  <!-- Search and Filters -->
  <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-4 mb-6">
    <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
      <!-- Search -->
      <div class="md:col-span-2">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Search</label>
        <div class="relative">
          <span class="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">🔍</span>
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search by name, code, city, country..."
            class="w-full pl-10 pr-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          />
        </div>
      </div>

      <!-- Country Filter -->
      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Country</label>
        <select
          bind:value={filterCountry}
          class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
        >
          <option value="">All Countries</option>
          {#each countries as country}
            <option value={country}>{country}</option>
          {/each}
        </select>
      </div>

      <!-- Sort By -->
      <div>
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Sort By</label>
        <select
          bind:value={sortBy}
          class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
        >
          <option value="name">Name</option>
          <option value="icao">ICAO Code</option>
          <option value="country">Country</option>
          <option value="city">City</option>
        </select>
      </div>
    </div>

    {#if searchQuery || filterCountry}
      <div class="mt-3 flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400">
        <span>Showing {filteredAirports.length} of {airports.length} airports</span>
        <button
          onclick={() => { searchQuery = ''; filterCountry = ''; }}
          class="text-primary-600 dark:text-primary-400 hover:underline"
        >
          Clear filters
        </button>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-16">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
    </div>
  {:else if airports.length === 0}
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
      <div class="text-6xl mb-4">✈️</div>
      <h3 class="text-xl font-semibold text-gray-900 dark:text-white mb-2">No Airports Yet</h3>
      <p class="text-gray-600 dark:text-gray-400 mb-6">
        Add airports to your database to track flight origins and destinations
      </p>
      <button
        onclick={openCreateModal}
        class="bg-primary-600 hover:bg-primary-700 text-white px-6 py-2.5 rounded-lg font-medium transition"
      >
        + Add First Airport
      </button>
    </div>
  {:else if filteredAirports.length === 0}
    <div class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
      <div class="text-6xl mb-4">🔍</div>
      <h3 class="text-xl font-semibold text-gray-900 dark:text-white mb-2">No Results Found</h3>
      <p class="text-gray-600 dark:text-gray-400">
        No airports match your search criteria
      </p>
    </div>
  {:else}
    <!-- Airport Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each filteredAirports as airport (airport.id)}
        <div
          class="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden hover:shadow-md transition-shadow cursor-pointer"
          onclick={() => selectedAirport = airport}
        >
          <!-- Card Header -->
          <div class="px-4 py-3 bg-gray-50 dark:bg-gray-900/50 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
            <div class="flex items-center gap-2">
              {#if airport.icao_code}
                <span class="px-2 py-1 bg-blue-100 dark:bg-blue-900/50 text-blue-700 dark:text-blue-300 rounded font-mono font-bold text-sm">
                  {airport.icao_code}
                </span>
              {/if}
              {#if airport.iata_code}
                <span class="px-2 py-1 bg-purple-100 dark:bg-purple-900/50 text-purple-700 dark:text-purple-300 rounded font-mono font-bold text-sm">
                  {airport.iata_code}
                </span>
              {/if}
            </div>
            <div class="flex items-center gap-1">
              <button
                onclick={(e) => { e.stopPropagation(); openEditModal(airport); }}
                class="p-1.5 hover:bg-gray-200 dark:hover:bg-gray-700 rounded transition"
                title="Edit"
              >
                ✏️
              </button>
              <button
                onclick={(e) => { e.stopPropagation(); deleteAirport(airport.id); }}
                class="p-1.5 hover:bg-red-100 dark:hover:bg-red-900/30 rounded transition text-red-600 dark:text-red-400"
                title="Delete"
              >
                🗑️
              </button>
            </div>
          </div>

          <!-- Card Body -->
          <div class="p-4">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-2 line-clamp-2">
              {airport.name}
            </h3>

            <div class="space-y-1.5 text-sm">
              {#if airport.city || airport.country}
                <div class="flex items-center gap-2 text-gray-600 dark:text-gray-400">
                  <span>📍</span>
                  <span>{airport.city ? `${airport.city}, ` : ''}{airport.country || ''}</span>
                </div>
              {/if}

              {#if airport.timezone}
                <div class="flex items-center gap-2 text-gray-600 dark:text-gray-400">
                  <span>🕐</span>
                  <span>{airport.timezone}</span>
                </div>
              {/if}

              {#if airport.latitude !== null && airport.longitude !== null}
                <div class="flex items-center gap-2 text-gray-600 dark:text-gray-400">
                  <span>🌐</span>
                  <span class="font-mono text-xs">
                    {airport.latitude.toFixed(4)}, {airport.longitude.toFixed(4)}
                  </span>
                </div>
              {/if}
            </div>

            <!-- Quick Links -->
            {#if airport.latitude !== null && airport.longitude !== null}
              <div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700 flex flex-wrap gap-2">
                <a
                  href={getGoogleMapsUrl(airport.latitude, airport.longitude)}
                  target="_blank"
                  rel="noopener noreferrer"
                  onclick={(e) => e.stopPropagation()}
                  class="text-xs px-2 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-600 transition"
                >
                  📍 Maps
                </a>
                <a
                  href={getOpenStreetMapUrl(airport.latitude, airport.longitude)}
                  target="_blank"
                  rel="noopener noreferrer"
                  onclick={(e) => e.stopPropagation()}
                  class="text-xs px-2 py-1 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-600 transition"
                >
                  🗺️ OSM
                </a>
                {#if getAirportWebsite(airport)}
                  <a
                    href={getAirportWebsite(airport)}
                    target="_blank"
                    rel="noopener noreferrer"
                    onclick={(e) => e.stopPropagation()}
                    class="text-xs px-2 py-1 bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 rounded hover:bg-primary-200 dark:hover:bg-primary-900/50 transition"
                  >
                    🌐 Website
                  </a>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Airport Detail Modal -->
{#if selectedAirport}
  <div
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
    onclick={() => selectedAirport = null}
  >
    <div
      class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-lg overflow-hidden"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            {#if selectedAirport.icao_code}
              <span class="px-3 py-1 bg-blue-100 dark:bg-blue-900/50 text-blue-700 dark:text-blue-300 rounded font-mono font-bold text-lg">
                {selectedAirport.icao_code}
              </span>
            {/if}
            {#if selectedAirport.iata_code}
              <span class="px-3 py-1 bg-purple-100 dark:bg-purple-900/50 text-purple-700 dark:text-purple-300 rounded font-mono font-bold text-lg">
                {selectedAirport.iata_code}
              </span>
            {/if}
          </div>
          <button
            onclick={() => selectedAirport = null}
            class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition text-gray-500 dark:text-gray-400"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        <h3 class="text-xl font-bold text-gray-900 dark:text-white mt-3">{selectedAirport.name}</h3>
        {#if selectedAirport.city || selectedAirport.country}
          <p class="text-gray-600 dark:text-gray-400 mt-1">
            {selectedAirport.city ? `${selectedAirport.city}, ` : ''}{selectedAirport.country || ''}
          </p>
        {/if}
      </div>

      <!-- Body -->
      <div class="p-6 space-y-4">
        <!-- Location Details -->
        <div class="grid grid-cols-2 gap-4">
          {#if selectedAirport.latitude !== null && selectedAirport.longitude !== null}
            <div>
              <label class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Coordinates</label>
              <p class="font-mono text-gray-900 dark:text-white">
                {selectedAirport.latitude.toFixed(6)}, {selectedAirport.longitude.toFixed(6)}
              </p>
            </div>
          {/if}

          {#if selectedAirport.timezone}
            <div>
              <label class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Timezone</label>
              <p class="text-gray-900 dark:text-white">{selectedAirport.timezone}</p>
            </div>
          {/if}
        </div>

        <!-- Map Links -->
        {#if selectedAirport.latitude !== null && selectedAirport.longitude !== null}
          <div>
            <label class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase mb-2 block">View on Map</label>
            <div class="flex flex-wrap gap-2">
              <a
                href={getGoogleMapsUrl(selectedAirport.latitude, selectedAirport.longitude)}
                target="_blank"
                rel="noopener noreferrer"
                class="flex items-center gap-2 px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition"
              >
                <span>📍</span> Google Maps
              </a>
              <a
                href={getOpenStreetMapUrl(selectedAirport.latitude, selectedAirport.longitude)}
                target="_blank"
                rel="noopener noreferrer"
                class="flex items-center gap-2 px-4 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition"
              >
                <span>🗺️</span> OpenStreetMap
              </a>
              <a
                href={`https://www.flightradar24.com/airport/${selectedAirport.iata_code || selectedAirport.icao_code}`}
                target="_blank"
                rel="noopener noreferrer"
                class="flex items-center gap-2 px-4 py-2 bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300 rounded-lg hover:bg-orange-200 dark:hover:bg-orange-900/50 transition"
              >
                <span>✈️</span> FlightRadar24
              </a>
            </div>
          </div>
        {/if}

        <!-- Website -->
        {#if getAirportWebsite(selectedAirport)}
          <div>
            <label class="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase mb-2 block">Official Website</label>
            <a
              href={getAirportWebsite(selectedAirport)}
              target="_blank"
              rel="noopener noreferrer"
              class="flex items-center gap-2 px-4 py-2 bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 rounded-lg hover:bg-primary-200 dark:hover:bg-primary-900/50 transition w-fit"
            >
              <span>🌐</span> {getAirportWebsite(selectedAirport)}
            </a>
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-200 dark:border-gray-700 flex justify-between">
        <button
          onclick={() => { openEditModal(selectedAirport!); selectedAirport = null; }}
          class="px-4 py-2 text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition font-medium"
        >
          ✏️ Edit Airport
        </button>
        <button
          onclick={() => selectedAirport = null}
          class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600 rounded-lg transition font-medium"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Create/Edit Modal -->
{#if showModal}
  <div
    class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4"
    onclick={() => showModal = false}
  >
    <div
      class="bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-xl max-h-[90vh] overflow-y-auto"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between sticky top-0 bg-white dark:bg-gray-800 z-10">
        <h3 class="text-xl font-bold text-gray-900 dark:text-white">
          {editingAirport ? 'Edit Airport' : 'Add New Airport'}
        </h3>
        <button
          onclick={() => showModal = false}
          class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition text-gray-500 dark:text-gray-400"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Form -->
      <form onsubmit={(e) => { e.preventDefault(); saveAirport(); }} class="p-6">
        <div class="grid grid-cols-2 gap-4">
          <!-- ICAO Code with Auto-lookup -->
          <div class="relative">
            <label for="icao" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              ICAO Code
              {#if lookupLoading}
                <span class="ml-2 text-xs text-primary-500">searching...</span>
              {/if}
            </label>
            <input
              type="text"
              id="icao"
              bind:value={formIcaoCode}
              oninput={(e) => lookupAirportCode((e.target as HTMLInputElement).value)}
              onfocus={() => { if (lookupSuggestions.length > 0) showSuggestions = true; }}
              placeholder="KJFK"
              maxlength="4"
              autocomplete="off"
              class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white uppercase font-mono focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            {#if csvAirportCount > 0}
              <div class="text-[10px] text-gray-400 mt-1">
                Search {csvAirportCount.toLocaleString()} airports
              </div>
            {/if}
          </div>

          <!-- IATA Code with Auto-lookup -->
          <div class="relative">
            <label for="iata" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              IATA Code
            </label>
            <input
              type="text"
              id="iata"
              bind:value={formIataCode}
              oninput={(e) => lookupAirportCode((e.target as HTMLInputElement).value)}
              onfocus={() => { if (lookupSuggestions.length > 0) showSuggestions = true; }}
              placeholder="JFK"
              maxlength="3"
              autocomplete="off"
              class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white uppercase font-mono focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>

          <!-- "Did you mean?" Suggestions -->
          {#if showSuggestions && lookupSuggestions.length > 0}
            <div class="col-span-2 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700 rounded-lg p-3">
              <div class="text-sm font-medium text-amber-800 dark:text-amber-300 mb-2">
                Did you mean?
              </div>
              <div class="space-y-2">
                {#each lookupSuggestions as result}
                  <button
                    type="button"
                    onclick={() => selectSuggestion(result)}
                    class="w-full text-left p-2 rounded-lg bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-600 hover:border-primary-500 hover:bg-primary-50 dark:hover:bg-primary-900/20 transition flex items-center gap-3"
                  >
                    <div class="flex gap-1">
                      {#if result.airport.ident}
                        <span class="px-1.5 py-0.5 bg-blue-100 dark:bg-blue-900/50 text-blue-700 dark:text-blue-300 rounded font-mono text-xs font-bold">
                          {result.airport.ident}
                        </span>
                      {/if}
                      {#if result.airport.iata_code}
                        <span class="px-1.5 py-0.5 bg-purple-100 dark:bg-purple-900/50 text-purple-700 dark:text-purple-300 rounded font-mono text-xs font-bold">
                          {result.airport.iata_code}
                        </span>
                      {/if}
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="text-sm text-gray-900 dark:text-white truncate">
                        {result.airport.name}
                      </div>
                      <div class="text-xs text-gray-500 dark:text-gray-400">
                        {result.airport.municipality || ''}{result.airport.municipality && result.airport.iso_country ? ', ' : ''}{result.airport.iso_country || ''}
                      </div>
                    </div>
                    <div class="text-xs text-gray-400">
                      {Math.round(result.similarity_score * 100)}%
                    </div>
                  </button>
                {/each}
              </div>
              <button
                type="button"
                onclick={() => showSuggestions = false}
                class="mt-2 text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
              >
                Dismiss suggestions
              </button>
            </div>
          {/if}

          <!-- Airport Name -->
          <div class="col-span-2">
            <label for="name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Airport Name <span class="text-red-500">*</span>
            </label>
            <input
              type="text"
              id="name"
              bind:value={formName}
              placeholder="John F. Kennedy International Airport"
              required
              class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>

          <!-- City -->
          <div>
            <label for="city" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              City
            </label>
            <input
              type="text"
              id="city"
              bind:value={formCity}
              placeholder="New York"
              class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>

          <!-- Country -->
          <div>
            <label for="country" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Country
            </label>
            <input
              type="text"
              id="country"
              bind:value={formCountry}
              placeholder="United States"
              class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>

          <!-- Latitude -->
          <div>
            <label for="latitude" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Latitude
            </label>
            <input
              type="number"
              id="latitude"
              bind:value={formLatitude}
              placeholder="40.6413"
              step="0.000001"
              min="-90"
              max="90"
              class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white font-mono focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>

          <!-- Longitude -->
          <div>
            <label for="longitude" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Longitude
            </label>
            <input
              type="number"
              id="longitude"
              bind:value={formLongitude}
              placeholder="-73.7781"
              step="0.000001"
              min="-180"
              max="180"
              class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white font-mono focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
          </div>

          <!-- Timezone -->
          <div class="col-span-2">
            <label for="timezone" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Timezone
            </label>
            <input
              type="text"
              id="timezone"
              bind:value={formTimezone}
              placeholder="America/New_York"
              list="timezone-suggestions"
              class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
            />
            <datalist id="timezone-suggestions">
              <option value="America/New_York">
              <option value="America/Los_Angeles">
              <option value="America/Chicago">
              <option value="America/Denver">
              <option value="Europe/London">
              <option value="Europe/Paris">
              <option value="Europe/Berlin">
              <option value="Europe/Amsterdam">
              <option value="Asia/Tokyo">
              <option value="Asia/Singapore">
              <option value="Asia/Dubai">
              <option value="Asia/Hong_Kong">
              <option value="Australia/Sydney">
              <option value="Pacific/Auckland">
            </datalist>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-3 mt-6 pt-4 border-t border-gray-200 dark:border-gray-700">
          <button
            type="button"
            onclick={() => showModal = false}
            class="px-4 py-2.5 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg transition font-medium"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="px-6 py-2.5 bg-primary-600 hover:bg-primary-700 text-white rounded-lg transition font-medium"
          >
            {editingAirport ? 'Update' : 'Create'} Airport
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
