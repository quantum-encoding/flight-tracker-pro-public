<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { theme } from '$lib/theme';
  import { translations } from '$lib/i18n';
  import { developerMode, initDeveloperMode, userLocation, initUserLocation } from '$lib/stores/settings';
  import SetupWizard from '$lib/components/SetupWizard.svelte';
  import WelcomeScreen from '$lib/components/WelcomeScreen.svelte';
  import Settings from '$lib/components/Settings.svelte';
  import InvestigationPanel from '$lib/components/InvestigationPanel.svelte';
  import BulkInvestigation from '$lib/components/BulkInvestigation.svelte';
  import FlightMap from '$lib/components/FlightMap.svelte';
  import FlightDetail from '$lib/components/FlightDetail.svelte';
  import AirportList from '$lib/components/AirportList.svelte';
  import Documents from '$lib/components/Documents.svelte';
  import Analytics from '$lib/components/Analytics.svelte';
  import DatasetSelector from '$lib/components/DatasetSelector.svelte';
  import Passengers from '$lib/components/Passengers.svelte';
  import ManualFlightEntry from '$lib/components/ManualFlightEntry.svelte';
  import BatchOCRUploader from '$lib/components/BatchOCRUploader.svelte';
  import Researchers from '$lib/components/Researchers.svelte';
  import ResearchDocuments from '$lib/components/ResearchDocuments.svelte';
  import CsvImportPreview from '$lib/components/CsvImportPreview.svelte';
  import Journeys from '$lib/components/Journeys.svelte';
  import PilotLogbook from '$lib/components/PilotLogbook.svelte';
  import FrequentFlyerPrograms from '$lib/components/FrequentFlyerPrograms.svelte';
  import Airports from '$lib/components/Airports.svelte';
  import AircraftTypes from '$lib/components/AircraftTypes.svelte';
  import CommandCentre from '$lib/components/CommandCentre.svelte';
  import RelationshipGraph from '$lib/components/RelationshipGraph.svelte';
  import FuelTracker from '$lib/components/FuelTracker.svelte';
  import SchemaBuilder from '$lib/components/SchemaBuilder.svelte';
  import Insights from '$lib/components/Insights.svelte';
  import TaskTreeBuilder from '$lib/components/views/TaskTreeBuilder.svelte';
  import NetworkSentinel from '$lib/components/views/NetworkSentinel.svelte';
  import GlobeVisualization from '$lib/components/views/GlobeVisualization.svelte';
  import Visualizations from '$lib/components/Visualizations.svelte';
  import MediaGallery from '$lib/components/MediaGallery.svelte';
  import CarbonFootprint from '$lib/components/CarbonFootprint.svelte';
  import DonateModal from '$lib/components/DonateModal.svelte';

  let userExists = $state(false);
  let loading = $state(true);
  let user: any = $state(null);
  let stats = $state({
    total_flights: 0,
    total_distance_km: 0,
    total_flight_time_hours: 0,
    airports_visited: 0,
    total_carbon_kg: 0,
  });
  let flights: any[] = $state([]);
  let allFlights: any[] = $state([]); // For map view
  let importResult = $state<any>(null);
  let investigatingFlight: any = $state(null);
  let showBulkInvestigation = $state(false);
  let selectedFlight: any = $state(null);
  let currentView = $state<'list' | 'map' | 'globe' | 'documents' | 'analytics' | 'visualizations' | 'passengers' | 'journeys' | 'logbook' | 'ffp' | 'fuel' | 'schemas' | 'insights' | 'airports' | 'aircraft' | 'researchers' | 'researchdocs' | 'csvimport' | 'commandcentre' | 'warroom' | 'workflows' | 'sentinel' | 'media' | 'carbon' | 'settings'>('list');
  let showAirportList = $state(false);
  let showFlightsPopup = $state(false);
  let showDistancePopup = $state(false);
  let showDurationPopup = $state(false);
  let showCO2Popup = $state(false);
  let showNewDatasetWizard = $state(false);
  let showManualEntry = $state(false);
  let showBatchOCRUploader = $state(false);
  let showDonateModal = $state(false);
  let mapDarkMode = $state(false);

  // Map visualization mode (flights or network)
  let mapMode = $state<'flights' | 'network'>('flights');
  let globeMode = $state<'flights' | 'network'>('flights');
  let networkFlows = $state<any[]>([]);
  let networkConnectionsForMap = $state<any[]>([]);

  // War Room state
  let warRoomTarget = $state('');
  let warRoomEntityType = $state('person');
  let warRoomMaxDepth = $state(2);
  let warRoomActive = $state(false);
  let warRoomBuilding = $state(false);
  let warRoomStats = $state<{total_relationships: number; person_count: number; flight_count: number; airport_count: number} | null>(null);
  let warRoomEntityOptions = $state<{id: string; label: string}[]>([]);
  let warRoomLoadingEntities = $state(false);

  // Build relationship graph from flight data
  async function buildRelationshipGraph() {
    warRoomBuilding = true;
    try {
      const result = await invoke('build_flight_relationships') as {relationships_created: number; passengers_processed: number; flights_processed: number};
      console.log('Built relationships:', result);
      // Reload stats
      await loadRelationshipStats();
      alert(`Graph built! Created ${result.relationships_created} relationships from ${result.flights_processed} flights and ${result.passengers_processed} passenger records.`);
    } catch (error) {
      console.error('Failed to build relationships:', error);
      alert(`Failed to build graph: ${error}`);
    } finally {
      warRoomBuilding = false;
    }
  }

  // Load relationship graph stats
  async function loadRelationshipStats() {
    try {
      warRoomStats = await invoke('get_relationship_stats') as {total_relationships: number; person_count: number; flight_count: number; airport_count: number};
    } catch (error) {
      console.error('Failed to load relationship stats:', error);
      warRoomStats = null;
    }
  }

  // Load available entities for the war room dropdown based on entity type
  async function loadWarRoomEntities() {
    if (!user?.id) return;
    warRoomLoadingEntities = true;
    warRoomEntityOptions = [];
    warRoomTarget = '';

    try {
      if (warRoomEntityType === 'person') {
        // Load passengers
        const passengers = await invoke('get_all_passenger_names', { userId: user.id }) as {abbreviation: string; full_name: string | null}[];
        warRoomEntityOptions = passengers.map(p => ({
          id: p.abbreviation,
          label: p.full_name ? `${p.full_name} (${p.abbreviation})` : p.abbreviation
        }));
      } else if (warRoomEntityType === 'flight') {
        // Load flights - use existing flights data or fetch
        const flightList = flights.length > 0 ? flights : await invoke('list_flights', { userId: user.id, limit: 1000, offset: 0 }) as any[];
        warRoomEntityOptions = flightList.map((f: any) => ({
          id: f.id,
          label: `${f.flight_number || 'Unknown'} - ${f.departure_airport} → ${f.arrival_airport} (${f.departure_time?.split('T')[0] || 'No date'})`
        }));
      } else if (warRoomEntityType === 'location') {
        // Load unique airports from flights
        const flightList = flights.length > 0 ? flights : await invoke('list_flights', { userId: user.id, limit: 1000, offset: 0 }) as any[];
        const airports = new Set<string>();
        flightList.forEach((f: any) => {
          if (f.departure_airport) airports.add(f.departure_airport);
          if (f.arrival_airport) airports.add(f.arrival_airport);
        });
        warRoomEntityOptions = Array.from(airports).sort().map(code => ({
          id: code,
          label: code
        }));
      } else {
        // For document_chunk and entity types, show what's in the relationship graph
        warRoomEntityOptions = [];
      }
    } catch (error) {
      console.error('Failed to load war room entities:', error);
      warRoomEntityOptions = [];
    } finally {
      warRoomLoadingEntities = false;
    }
  }

  // Auto-sync map dark mode with cyberpunk and skynet themes
  $effect(() => {
    const currentTheme = $theme;
    if (currentTheme === 'cyberpunk' || currentTheme === 'skynet' || currentTheme === 'dark') {
      mapDarkMode = true;
    }
  });

  // Reload entities when entity type changes in war room
  $effect(() => {
    if (warRoomEntityType && user?.id) {
      loadWarRoomEntities();
    }
  });

  // Search and filter state
  let searchQuery = $state('');
  let filterDateFrom = $state('');
  let filterDateTo = $state('');
  let filterOrigin = $state('');
  let filterDestination = $state('');
  let sortColumn = $state<'date' | 'route' | 'distance' | null>(null);
  let sortDirection = $state<'asc' | 'desc'>('desc');

  // Quick filter state
  let quickFilter = $state<'all' | 'popular' | 'rare' | 'A-E' | 'F-J' | 'K-O' | 'P-T' | 'U-Z'>('all');

  // Calculate route popularity for quick filters
  let routePopularity = $derived(() => {
    const routeCounts: Record<string, number> = {};
    for (const flight of flights) {
      const route = `${flight.departure_airport}-${flight.arrival_airport}`;
      routeCounts[route] = (routeCounts[route] || 0) + 1;
    }
    return routeCounts;
  });

  // Filtered and sorted flights
  let filteredFlights = $derived(() => {
    let result = [...flights];

    // Apply quick filter first
    if (quickFilter !== 'all') {
      const popularity = routePopularity();
      const avgPopularity = Object.values(popularity).reduce((a, b) => a + b, 0) / Object.keys(popularity).length || 1;

      if (quickFilter === 'popular') {
        // Routes with above-average frequency
        result = result.filter(f => {
          const route = `${f.departure_airport}-${f.arrival_airport}`;
          return (popularity[route] || 0) >= avgPopularity;
        });
      } else if (quickFilter === 'rare') {
        // Routes with below-average frequency (flown only once or twice)
        result = result.filter(f => {
          const route = `${f.departure_airport}-${f.arrival_airport}`;
          return (popularity[route] || 0) <= 2;
        });
      } else {
        // Alphabetical filters (A-E, F-J, etc.) - based on departure airport
        const rangeMap: Record<string, [string, string]> = {
          'A-E': ['A', 'E'],
          'F-J': ['F', 'J'],
          'K-O': ['K', 'O'],
          'P-T': ['P', 'T'],
          'U-Z': ['U', 'Z'],
        };
        const range = rangeMap[quickFilter];
        if (range) {
          result = result.filter(f => {
            const firstChar = f.departure_airport?.charAt(0).toUpperCase() || '';
            return firstChar >= range[0] && firstChar <= range[1];
          });
        }
      }
    }

    // Apply search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(f =>
        f.departure_airport?.toLowerCase().includes(query) ||
        f.arrival_airport?.toLowerCase().includes(query) ||
        f.flight_number?.toLowerCase().includes(query) ||
        f.notes?.toLowerCase().includes(query) ||
        f.departure_datetime?.includes(query)
      );
    }

    // Apply date range filter
    if (filterDateFrom) {
      result = result.filter(f => f.departure_datetime >= filterDateFrom);
    }
    if (filterDateTo) {
      result = result.filter(f => f.departure_datetime <= filterDateTo + 'T23:59:59');
    }

    // Apply airport filters (exact match from dropdown)
    if (filterOrigin) {
      result = result.filter(f => f.departure_airport === filterOrigin);
    }
    if (filterDestination) {
      result = result.filter(f => f.arrival_airport === filterDestination);
    }

    // Apply sorting
    if (sortColumn) {
      result.sort((a, b) => {
        let aVal, bVal;

        switch (sortColumn) {
          case 'date':
            aVal = new Date(a.departure_datetime).getTime();
            bVal = new Date(b.departure_datetime).getTime();
            break;
          case 'route':
            aVal = `${a.departure_airport}-${a.arrival_airport}`;
            bVal = `${b.departure_airport}-${b.arrival_airport}`;
            break;
          case 'distance':
            aVal = a.distance_km || 0;
            bVal = b.distance_km || 0;
            break;
          default:
            return 0;
        }

        if (aVal < bVal) return sortDirection === 'asc' ? -1 : 1;
        if (aVal > bVal) return sortDirection === 'asc' ? 1 : -1;
        return 0;
      });
    }

    return result;
  });

  function toggleSort(column: 'date' | 'route' | 'distance') {
    if (sortColumn === column) {
      sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
    } else {
      sortColumn = column;
      sortDirection = 'desc';
    }
  }

  function clearFilters() {
    searchQuery = '';
    filterDateFrom = '';
    filterDateTo = '';
    filterOrigin = '';
    filterDestination = '';
    sortColumn = null;
    sortDirection = 'desc';
    quickFilter = 'all';
  }

  let hasActiveFilters = $derived(
    searchQuery.trim() !== '' ||
    filterDateFrom !== '' ||
    filterDateTo !== '' ||
    filterOrigin !== '' ||
    filterDestination !== '' ||
    quickFilter !== 'all'
  );

  // Unique airport codes for dropdown filters
  let uniqueDepartureAirports = $derived(() => {
    const airports = new Set<string>();
    for (const flight of flights) {
      if (flight.departure_airport) {
        airports.add(flight.departure_airport);
      }
    }
    return Array.from(airports).sort();
  });

  let uniqueArrivalAirports = $derived(() => {
    const airports = new Set<string>();
    for (const flight of flights) {
      if (flight.arrival_airport) {
        airports.add(flight.arrival_airport);
      }
    }
    return Array.from(airports).sort();
  });

  onMount(() => {
    // Run async initialization
    (async () => {
      await initDeveloperMode();
      await initUserLocation();
      await loadData();
    })();

    // Return sync cleanup function
    return () => {
      unsubscribeLocation();
    };
  });

  async function loadData() {
    loading = true;
    try {
      // Check if user is initialized
      userExists = await invoke('initialize_app');

      // Auto-enrich airport data in background if needed (fire-and-forget)
      autoEnrichAirports();

      if (userExists) {
        // Only get primary user if we don't already have a user set
        if (!user) {
          user = await invoke('get_primary_user');
        }

        if (user) {
          // Load statistics for the current user
          stats = await invoke('get_statistics', { userId: user.id });
          // Load ALL flights (for both list and map view - we'll filter client-side)
          flights = await invoke('list_flights', { userId: user.id, limit: 10000, offset: 0 });
          allFlights = flights; // Use same data for map
        }
      }
    } catch (error) {
      console.error('Error initializing app:', error);
    } finally {
      loading = false;
    }
  }

  async function autoEnrichAirports() {
    try {
      const missingCount = await invoke<number>('get_missing_coordinates_count');
      if (missingCount > 0) {
        console.log(`⏳ Auto-enriching ${missingCount} airports with missing coordinates...`);
        const result = await invoke('enrich_airport_data');
        console.log('✓ Airport enrichment complete:', result);
      }
    } catch (error) {
      console.warn('Airport enrichment failed (non-critical):', error);
    }
  }

  // City/country coordinate mappings for network visualization
  const cityCoordinates: Record<string, { lat: number; lng: number }> = {
    // North America
    'San Francisco': { lat: 37.7749, lng: -122.4194 },
    'New York': { lat: 40.7128, lng: -74.0060 },
    'Los Angeles': { lat: 34.0522, lng: -118.2437 },
    'Seattle': { lat: 47.6062, lng: -122.3321 },
    'Chicago': { lat: 41.8781, lng: -87.6298 },
    'Dallas': { lat: 32.7767, lng: -96.7970 },
    'Miami': { lat: 25.7617, lng: -80.1918 },
    'Atlanta': { lat: 33.7490, lng: -84.3880 },
    'Denver': { lat: 39.7392, lng: -104.9903 },
    'Phoenix': { lat: 33.4484, lng: -112.0740 },
    'Toronto': { lat: 43.6532, lng: -79.3832 },
    'Vancouver': { lat: 49.2827, lng: -123.1207 },
    'Montreal': { lat: 45.5017, lng: -73.5673 },
    'Mexico City': { lat: 19.4326, lng: -99.1332 },
    // Europe
    'London': { lat: 51.5074, lng: -0.1278 },
    'Paris': { lat: 48.8566, lng: 2.3522 },
    'Berlin': { lat: 52.5200, lng: 13.4050 },
    'Frankfurt': { lat: 50.1109, lng: 8.6821 },
    'Amsterdam': { lat: 52.3676, lng: 4.9041 },
    'Dublin': { lat: 53.3498, lng: -6.2603 },
    'Madrid': { lat: 40.4168, lng: -3.7038 },
    'Barcelona': { lat: 41.3851, lng: 2.1734 },
    'Valencia': { lat: 39.4699, lng: -0.3763 },
    'Seville': { lat: 37.3891, lng: -5.9845 },
    'Bilbao': { lat: 43.2630, lng: -2.9350 },
    'Malaga': { lat: 36.7213, lng: -4.4214 },
    'Lisbon': { lat: 38.7223, lng: -9.1393 },
    'Rome': { lat: 41.9028, lng: 12.4964 },
    'Milan': { lat: 45.4642, lng: 9.1900 },
    'Vienna': { lat: 48.2082, lng: 16.3738 },
    'Zurich': { lat: 47.3769, lng: 8.5417 },
    'Brussels': { lat: 50.8503, lng: 4.3517 },
    'Stockholm': { lat: 59.3293, lng: 18.0686 },
    'Oslo': { lat: 59.9139, lng: 10.7522 },
    'Copenhagen': { lat: 55.6761, lng: 12.5683 },
    'Helsinki': { lat: 60.1699, lng: 24.9384 },
    'Warsaw': { lat: 52.2297, lng: 21.0122 },
    'Prague': { lat: 50.0755, lng: 14.4378 },
    'Budapest': { lat: 47.4979, lng: 19.0402 },
    'Athens': { lat: 37.9838, lng: 23.7275 },
    'Moscow': { lat: 55.7558, lng: 37.6173 },
    // Asia
    'Tokyo': { lat: 35.6762, lng: 139.6503 },
    'Singapore': { lat: 1.3521, lng: 103.8198 },
    'Hong Kong': { lat: 22.3193, lng: 114.1694 },
    'Mumbai': { lat: 19.0760, lng: 72.8777 },
    'Bangalore': { lat: 12.9716, lng: 77.5946 },
    'Delhi': { lat: 28.6139, lng: 77.2090 },
    'Shanghai': { lat: 31.2304, lng: 121.4737 },
    'Beijing': { lat: 39.9042, lng: 116.4074 },
    'Seoul': { lat: 37.5665, lng: 126.9780 },
    'Bangkok': { lat: 13.7563, lng: 100.5018 },
    'Jakarta': { lat: -6.2088, lng: 106.8456 },
    'Manila': { lat: 14.5995, lng: 120.9842 },
    'Taipei': { lat: 25.0330, lng: 121.5654 },
    'Dubai': { lat: 25.2048, lng: 55.2708 },
    'Tel Aviv': { lat: 32.0853, lng: 34.7818 },
    // Oceania
    'Sydney': { lat: -33.8688, lng: 151.2093 },
    'Melbourne': { lat: -37.8136, lng: 144.9631 },
    'Auckland': { lat: -36.8509, lng: 174.7645 },
    // South America
    'São Paulo': { lat: -23.5505, lng: -46.6333 },
    'Buenos Aires': { lat: -34.6037, lng: -58.3816 },
    'Rio de Janeiro': { lat: -22.9068, lng: -43.1729 },
    'Santiago': { lat: -33.4489, lng: -70.6693 },
    'Lima': { lat: -12.0464, lng: -77.0428 },
    'Bogota': { lat: 4.7110, lng: -74.0721 },
    // Africa
    'Cape Town': { lat: -33.9249, lng: 18.4241 },
    'Johannesburg': { lat: -26.2041, lng: 28.0473 },
    'Cairo': { lat: 30.0444, lng: 31.2357 },
    'Lagos': { lat: 6.5244, lng: 3.3792 },
    'Nairobi': { lat: -1.2921, lng: 36.8219 },
  };

  const countryCoordinates: Record<string, { lat: number; lng: number }> = {
    // Americas
    'US': { lat: 37.0902, lng: -95.7129 },
    'CA': { lat: 56.1304, lng: -106.3468 },
    'MX': { lat: 23.6345, lng: -102.5528 },
    'BR': { lat: -14.2350, lng: -51.9253 },
    'AR': { lat: -38.4161, lng: -63.6167 },
    'CL': { lat: -35.6751, lng: -71.5430 },
    'CO': { lat: 4.5709, lng: -74.2973 },
    'PE': { lat: -9.1900, lng: -75.0152 },
    // Europe
    'GB': { lat: 55.3781, lng: -3.4360 },
    'DE': { lat: 51.1657, lng: 10.4515 },
    'FR': { lat: 46.2276, lng: 2.2137 },
    'ES': { lat: 40.4637, lng: -3.7492 },
    'IT': { lat: 41.8719, lng: 12.5674 },
    'NL': { lat: 52.1326, lng: 5.2913 },
    'BE': { lat: 50.5039, lng: 4.4699 },
    'PT': { lat: 39.3999, lng: -8.2245 },
    'IE': { lat: 53.1424, lng: -7.6921 },
    'AT': { lat: 47.5162, lng: 14.5501 },
    'CH': { lat: 46.8182, lng: 8.2275 },
    'SE': { lat: 60.1282, lng: 18.6435 },
    'NO': { lat: 60.4720, lng: 8.4689 },
    'DK': { lat: 56.2639, lng: 9.5018 },
    'FI': { lat: 61.9241, lng: 25.7482 },
    'PL': { lat: 51.9194, lng: 19.1451 },
    'CZ': { lat: 49.8175, lng: 15.4730 },
    'HU': { lat: 47.1625, lng: 19.5033 },
    'GR': { lat: 39.0742, lng: 21.8243 },
    'RU': { lat: 61.5240, lng: 105.3188 },
    'UA': { lat: 48.3794, lng: 31.1656 },
    // Asia
    'JP': { lat: 36.2048, lng: 138.2529 },
    'CN': { lat: 35.8617, lng: 104.1954 },
    'IN': { lat: 20.5937, lng: 78.9629 },
    'KR': { lat: 35.9078, lng: 127.7669 },
    'SG': { lat: 1.3521, lng: 103.8198 },
    'HK': { lat: 22.3193, lng: 114.1694 },
    'TW': { lat: 23.6978, lng: 120.9605 },
    'TH': { lat: 15.8700, lng: 100.9925 },
    'VN': { lat: 14.0583, lng: 108.2772 },
    'ID': { lat: -0.7893, lng: 113.9213 },
    'MY': { lat: 4.2105, lng: 101.9758 },
    'PH': { lat: 12.8797, lng: 121.7740 },
    'AE': { lat: 23.4241, lng: 53.8478 },
    'IL': { lat: 31.0461, lng: 34.8516 },
    'SA': { lat: 23.8859, lng: 45.0792 },
    'TR': { lat: 38.9637, lng: 35.2433 },
    // Oceania
    'AU': { lat: -25.2744, lng: 133.7751 },
    'NZ': { lat: -40.9006, lng: 174.8860 },
    // Africa
    'ZA': { lat: -30.5595, lng: 22.9375 },
    'EG': { lat: 26.8206, lng: 30.8025 },
    'NG': { lat: 9.0820, lng: 8.6753 },
    'KE': { lat: -0.0236, lng: 37.9062 },
    'MA': { lat: 31.7917, lng: -7.0926 },
  };

  // Local coordinates - from user settings store
  let localCoordinates = $state({ lat: 40.4168, lng: -3.7038 });

  // Subscribe to userLocation store
  const unsubscribeLocation = userLocation.subscribe(loc => {
    localCoordinates = { lat: loc.lat, lng: loc.lng };
  });

  // Load network data for map visualization
  async function loadNetworkDataForMap() {
    try {
      const flows = await invoke<any[]>('get_network_flows', { limit: 500, timeRange: '24h' });
      networkFlows = flows;

      // Convert flows to map connections
      const connections: any[] = [];
      const seenLocations = new Set<string>();

      for (const flow of flows) {
        let targetCoords = flow.geo_city ? cityCoordinates[flow.geo_city] : null;
        if (!targetCoords && flow.geo_country) {
          targetCoords = countryCoordinates[flow.geo_country];
        }
        if (!targetCoords) continue;

        const locationKey = `${flow.geo_city || flow.geo_country}`;
        if (seenLocations.has(locationKey)) continue;
        seenLocations.add(locationKey);

        let color = '#00b4ff';
        if (flow.is_anomaly) color = '#ff0040';
        else if (flow.process_name === 'claude') color = '#b000ff';
        else if (flow.process_name?.includes('firefox') || flow.process_name?.includes('brave') || flow.process_name?.includes('Chrome')) {
          color = '#00ff88';
        }

        connections.push({
          id: `flow-${flow.id}`,
          source: { lat: localCoordinates.lat, lng: localCoordinates.lng, name: 'Local' },
          target: { lat: targetCoords.lat, lng: targetCoords.lng, name: flow.geo_city || flow.geo_country || 'Unknown' },
          color,
          process: flow.process_name,
          isAnomaly: flow.is_anomaly,
        });
      }

      networkConnectionsForMap = connections;
    } catch (error) {
      console.error('Failed to load network data for map:', error);
      networkConnectionsForMap = [];
    }
  }

  let processingOcr = $state(false);
  let ocrResult = $state<any>(null);
  let bulkProcessing = $state(false);
  let bulkProgress = $state({ current: 0, total: 0 });
  let bulkResults = $state<any>(null);

  async function importOcrImage() {
    console.log('Import OCR Image clicked');
    try {
      const file = await open({
        multiple: false,
        filters: [{
          name: 'Image',
          extensions: ['png', 'jpg', 'jpeg', 'pdf']
        }],
        directory: false
      });

      if (file && user) {
        processingOcr = true;
        ocrResult = null;

        try {
          console.log('Analyzing boarding pass:', file);
          const result: any = await invoke('analyze_boarding_pass', { filePath: file });
          console.log('OCR Result:', result);

          // Create a flight from OCR result
          if (result && (result.departure_airport || result.arrival_airport)) {
            // Build notes field with all extracted metadata
            const noteParts = [];
            if (result.passenger_name) noteParts.push(`Passengers: ${result.passenger_name}`);
            if (result.aircraft_type) noteParts.push(`Aircraft: ${result.aircraft_type}`);
            if (result.gate) noteParts.push(`Gate: ${result.gate}`);
            if (result.terminal) noteParts.push(`Terminal: ${result.terminal}`);
            if (result.airline) noteParts.push(`Airline: ${result.airline}`);
            if (result.frequent_flyer_number) noteParts.push(`FFN: ${result.frequent_flyer_number}`);

            // Calculate distance between airports
            let distanceData: { distance_km: number; distance_nm: number } | null = null;
            try {
              if (result.departure_airport && result.arrival_airport) {
                distanceData = await invoke<{ distance_km: number; distance_nm: number }>('calculate_distance', {
                  fromAirport: result.departure_airport,
                  toAirport: result.arrival_airport
                });
                console.log('Calculated distance:', distanceData);
              }
            } catch (err) {
              console.warn('Could not calculate distance:', err);
            }

            // Calculate CO2 emissions
            let co2Data: { co2_kg: number } | null = null;
            try {
              if (distanceData) {
                co2Data = await invoke<{ co2_kg: number }>('calculate_co2_emissions', {
                  distanceKm: distanceData.distance_km,
                  aircraftType: result.aircraft_type || null
                });
                console.log('Calculated CO2:', co2Data);
              }
            } catch (err) {
              console.warn('Could not calculate CO2:', err);
            }

            // Calculate flight duration
            let flightDuration = null;
            try {
              if (distanceData) {
                flightDuration = await invoke('calculate_flight_duration', {
                  distanceKm: distanceData.distance_km,
                  aircraftType: result.aircraft_type || null
                });
                console.log('Calculated flight duration:', flightDuration);
              }
            } catch (err) {
              console.warn('Could not calculate flight duration:', err);
            }

            // Prepare flight data with all calculated fields
            const flightData = {
              id: '',
              flight_number: result.flight_number || null,
              departure_airport: result.departure_airport || 'UNKNOWN',
              arrival_airport: result.arrival_airport || 'UNKNOWN',
              departure_datetime: result.departure_datetime || new Date().toISOString(),
              arrival_datetime: result.arrival_datetime || null,
              aircraft_registration: null, // OCR doesn't extract registration
              booking_reference: result.booking_reference || null,
              ticket_number: result.ticket_number || null,
              seat_number: result.seat_number || null,
              fare_class: result.fare_class || null,
              notes: noteParts.join(' | ') || null,
              distance_nm: distanceData?.distance_nm || null,
              distance_km: distanceData?.distance_km || null,
              carbon_emissions_kg: co2Data?.co2_kg || null,
              flight_duration: flightDuration || null,
              total_duration: flightDuration || null,
              aircraft_type_id: null,
              base_fare: null,
              taxes: null,
              total_cost: null,
              currency: null,
              attachment_path: null,
              created_at: '',
              updated_at: ''
            };

            // Create the flight
            const flightId = await invoke('create_flight', {
              userId: user.id,
              flight: flightData
            });

            console.log('Flight created from OCR:', flightId);
            ocrResult = { success: true, flight: result };

            // Reload data
            await loadData();

            alert('✅ Boarding pass analyzed and flight added successfully!');
          } else {
            ocrResult = { success: false, error: 'Could not extract flight information from image' };
            alert('⚠️ Could not extract enough flight information from the image. Please try a clearer image or enter manually.');
          }
        } catch (err) {
          console.error('OCR processing error:', err);
          ocrResult = { success: false, error: err };
          alert(`OCR processing failed: ${err}`);
        } finally {
          processingOcr = false;
        }
      }
    } catch (error) {
      console.error('OCR import failed:', error);
      alert(`Import failed: ${error}`);
      processingOcr = false;
    }
  }

  function bulkImportBoardingPasses() {
    // Open the new batch OCR uploader modal
    showBatchOCRUploader = true;
  }

  async function handleBatchOCRComplete(results: any[]) {
    // Process each successful OCR result and create flights
    let successCount = 0;
    let errorCount = 0;

    for (const item of results) {
      try {
        if (item.result && (item.result.departure_airport || item.result.arrival_airport)) {
          const result = item.result;
          const flightData = {
            id: '',
            flight_number: result.flight_number || null,
            departure_airport: result.departure_airport || 'UNKNOWN',
            arrival_airport: result.arrival_airport || 'UNKNOWN',
            departure_datetime: result.departure_datetime || new Date().toISOString(),
            arrival_datetime: result.arrival_datetime || null,
            aircraft_registration: result.aircraft_type || null,
            booking_reference: result.booking_reference || null,
            ticket_number: result.ticket_number || null,
            seat_number: result.seat_number || null,
            fare_class: result.fare_class || null,
            notes: result.passenger_name ? `Passengers: ${result.passenger_name}` : null,
            distance_nm: null,
            flight_duration: null,
            total_duration: null,
            aircraft_type_id: null,
            base_fare: null,
            taxes: null,
            total_cost: null,
            currency: null,
            attachment_path: null,
            created_at: '',
            updated_at: ''
          };

          await invoke('create_flight', {
            userId: user.id,
            flight: flightData
          });

          successCount++;
        } else {
          errorCount++;
        }
      } catch (err) {
        console.error('Failed to create flight:', err);
        errorCount++;
      }
    }

    // Reload data
    await loadData();

    // Close the modal
    showBatchOCRUploader = false;

    // Show success message
    if (successCount > 0 || errorCount > 0) {
      alert(`✅ Batch import complete!\n${successCount} flights added${errorCount > 0 ? `, ${errorCount} errors` : ''}`);
    }
  }

  function formatDate(dateStr: string) {
    return new Date(dateStr).toLocaleDateString();
  }

  function handleReset() {
    // Reload the entire app after reset
    window.location.reload();
  }

  async function switchDataset(newUser: any) {
    user = newUser;
    await loadData();
  }

  function createNewDataset() {
    showNewDatasetWizard = true;
  }

  async function handleDatasetCreated() {
    showNewDatasetWizard = false;

    try {
      // Load all users to find the newly created one
      const allUsers: any[] = await invoke('list_all_users');

      if (allUsers && allUsers.length > 0) {
        // Switch to the most recently created user (last in array sorted by created_at ASC)
        const newestUser = allUsers[allUsers.length - 1];
        user = newestUser;

        // Load data for the new user
        await loadData();
      }
    } catch (error) {
      console.error('Failed to switch to new dataset:', error);
      // Fallback: just reload data
      await loadData();
    }
  }
</script>

{#if loading}
  <div class="fixed inset-0 flex items-center justify-center bg-gray-50 dark:bg-gray-900 z-50">
    <div class="text-center">
      <div class="animate-spin rounded-full h-16 w-16 border-4 border-primary-600 border-t-transparent mx-auto"></div>
      <p class="mt-6 text-lg text-gray-600 dark:text-gray-400">Loading Flight Tracker Pro...</p>
    </div>
  </div>
{:else}
<div class="w-full px-4 sm:px-6 lg:px-8 py-6 max-w-[2000px] mx-auto">
  {#if !userExists}
    <!-- Welcome Screen with Theme Selection -->
    <WelcomeScreen onComplete={loadData} />
  {:else}
    <!-- Main Dashboard -->
    <div>
      <header class="mb-8 flex items-center justify-between">
        <div>
          <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">
            {$translations('navigation.flights')}
          </h1>
          <p class="text-gray-600 dark:text-gray-400">
            {$translations('analytics.title')}
          </p>
        </div>
        <div class="flex items-center gap-3">
          <!-- Donate Button -->
          <button
            onclick={() => showDonateModal = true}
            class="p-2 rounded-lg text-gray-500 hover:text-pink-500 hover:bg-pink-50 dark:hover:bg-pink-900/20 transition-all group"
            title="Support Development"
          >
            <svg class="w-5 h-5 group-hover:scale-110 transition-transform" fill="currentColor" viewBox="0 0 24 24">
              <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
            </svg>
          </button>
          <DatasetSelector
            currentUser={user}
            onSwitch={switchDataset}
            onCreateNew={createNewDataset}
          />
        </div>
      </header>

      <!-- Statistics Cards -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-6 mb-8">
        <div
          onclick={() => showFlightsPopup = true}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (showFlightsPopup = true)}
          class="bg-white dark:bg-gray-800 rounded-lg shadow p-6 cursor-pointer hover:shadow-lg hover:scale-105 transition-all"
          role="button"
          tabindex="0"
          aria-label="{$translations('logbook.totalFlights')}"
        >
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">{$translations('logbook.totalFlights')}</p>
              <p class="text-3xl font-bold text-gray-900 dark:text-white">{stats.total_flights}</p>
              <p class="text-xs text-primary-600 dark:text-primary-400 mt-1">{$translations('common.help')} →</p>
            </div>
            <div class="text-primary-600 text-4xl">✈️</div>
          </div>
        </div>

        <div
          onclick={() => showDistancePopup = true}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (showDistancePopup = true)}
          class="bg-white dark:bg-gray-800 rounded-lg shadow p-6 cursor-pointer hover:shadow-lg hover:scale-105 transition-all"
          role="button"
          tabindex="0"
          aria-label="{$translations('passengers.totalDistance')}"
        >
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">{$translations('passengers.totalDistance')}</p>
              <p class="text-3xl font-bold text-gray-900 dark:text-white">
                {Math.round(stats.total_distance_km).toLocaleString()} {$translations('units.km')}
              </p>
              <p class="text-xs text-primary-600 dark:text-primary-400 mt-1">{$translations('common.help')} →</p>
            </div>
            <div class="text-primary-600 text-4xl">🌍</div>
          </div>
        </div>

        <div
          onclick={() => showDurationPopup = true}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (showDurationPopup = true)}
          class="bg-white dark:bg-gray-800 rounded-lg shadow p-6 cursor-pointer hover:shadow-lg hover:scale-105 transition-all"
          role="button"
          tabindex="0"
          aria-label="{$translations('flights.duration')}"
        >
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">{$translations('flights.duration')}</p>
              <p class="text-3xl font-bold text-gray-900 dark:text-white">
                {Math.round(stats.total_flight_time_hours)}h
              </p>
              <p class="text-xs text-primary-600 dark:text-primary-400 mt-1">{$translations('common.help')} →</p>
            </div>
            <div class="text-primary-600 text-4xl">⏱️</div>
          </div>
        </div>

        <div
          onclick={() => showAirportList = true}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (showAirportList = true)}
          class="bg-white dark:bg-gray-800 rounded-lg shadow p-6 cursor-pointer hover:shadow-lg hover:scale-105 transition-all"
          role="button"
          tabindex="0"
          aria-label="{$translations('airports.title')}"
        >
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">{$translations('airports.visits')}</p>
              <p class="text-3xl font-bold text-gray-900 dark:text-white">{stats.airports_visited}</p>
              <p class="text-xs text-primary-600 dark:text-primary-400 mt-1">{$translations('common.help')} →</p>
            </div>
            <div class="text-primary-600 text-4xl">🏛️</div>
          </div>
        </div>

        <div
          onclick={() => showCO2Popup = true}
          onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (showCO2Popup = true)}
          class="bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-900 dark:to-emerald-900 rounded-lg shadow p-6 border-2 border-green-200 dark:border-green-700 cursor-pointer hover:shadow-lg hover:scale-105 transition-all"
          role="button"
          tabindex="0"
          aria-label="CO₂ Emissions"
        >
          <div class="flex items-center justify-between">
            <div>
              <p class="text-sm text-green-700 dark:text-green-300 mb-1">CO₂</p>
              <p class="text-3xl font-bold text-green-900 dark:text-green-100">
                {Math.round(stats.total_carbon_kg / 1000).toLocaleString()}t
              </p>
              <p class="text-xs text-green-600 dark:text-green-400 mt-1">{$translations('common.help')} →</p>
            </div>
            <div class="text-green-600 dark:text-green-400 text-4xl">🌱</div>
          </div>
        </div>
      </div>

      <!-- Quick Actions -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6 mb-8">
        <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-white">{$translations('common.actions')}</h2>
        <div class="flex flex-wrap gap-4">
          <button
            onclick={() => currentView = 'csvimport'}
            class="bg-primary-600 hover:bg-primary-700 text-white px-6 py-2 rounded-lg transition font-medium"
          >
            📥 {$translations('flights.importFlights')}
          </button>
          <button
            onclick={importOcrImage}
            disabled={processingOcr || showBatchOCRUploader}
            class="bg-green-600 hover:bg-green-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white px-6 py-2 rounded-lg transition font-medium"
          >
            {processingOcr ? `🔄 ${$translations('common.loading')}...` : `🖼️ ${$translations('flights.importFlights')} (OCR)`}
          </button>
          <button
            onclick={bulkImportBoardingPasses}
            disabled={showBatchOCRUploader || processingOcr}
            class="bg-purple-600 hover:bg-purple-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white px-6 py-2 rounded-lg transition font-medium"
          >
            {showBatchOCRUploader ? `🔄 ${$translations('common.loading')}...` : `📚 ${$translations('flights.importFlights')} (Batch)`}
          </button>
          <button
            onclick={() => showManualEntry = true}
            class="bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-lg transition font-medium"
          >
            ➕ {$translations('flights.manual.title')}
          </button>
          <button
            onclick={() => currentView = 'settings'}
            class="bg-gray-600 hover:bg-gray-700 text-white px-6 py-2 rounded-lg transition font-medium"
          >
            ⚙️ {$translations('navigation.settings')}
          </button>
        </div>
      </div>

      <!-- Import Result -->
      {#if importResult}
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6 mb-8">
          <h3 class="text-lg font-semibold mb-3 text-gray-900 dark:text-white">{$translations('flights.import.title')} {$translations('flights.import.preview')}</h3>
          <div class="space-y-2">
            <p class="text-green-600 dark:text-green-400">
              ✅ {$translations('flights.import.success')}: {importResult.success_count} {$translations('navigation.flights')}
            </p>
            {#if importResult.error_count > 0}
              <p class="text-red-600 dark:text-red-400">
                ❌ {$translations('common.error')}: {importResult.error_count}
              </p>
              <div class="mt-2 max-h-40 overflow-y-auto bg-gray-50 dark:bg-gray-900 p-3 rounded text-sm">
                {#each importResult.errors as error}
                  <div class="text-red-600 dark:text-red-400">{error}</div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Bulk OCR Results -->
      {#if bulkResults}
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6 mb-8">
          <h3 class="text-lg font-semibold mb-3 text-gray-900 dark:text-white">{$translations('flights.import.title')} {$translations('flights.import.preview')} (OCR)</h3>
          <div class="space-y-2">
            <p class="text-green-600 dark:text-green-400">
              ✅ {$translations('flights.import.success')}: {bulkResults.successCount} {$translations('documents.types.boardingPass')}
            </p>
            {#if bulkResults.errorCount > 0}
              <p class="text-red-600 dark:text-red-400">
                ❌ {$translations('common.error')}: {bulkResults.errorCount}
              </p>
              <div class="mt-2 max-h-40 overflow-y-auto bg-gray-50 dark:bg-gray-900 p-3 rounded text-sm">
                {#each bulkResults.errors as error}
                  <div class="text-red-600 dark:text-red-400">{error}</div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Bulk Processing Progress -->
      {#if bulkProcessing}
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6 mb-8">
          <h3 class="text-lg font-semibold mb-3 text-gray-900 dark:text-white">
            {$translations('common.loading')} {$translations('documents.types.boardingPass')}...
          </h3>
          <div class="space-y-3">
            <div class="flex items-center justify-between text-sm text-gray-600 dark:text-gray-400">
              <span>{$translations('common.status')}: {bulkProgress.current} / {bulkProgress.total}</span>
              <span>{Math.round((bulkProgress.current / bulkProgress.total) * 100)}%</span>
            </div>
            <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3">
              <div
                class="bg-purple-600 h-3 rounded-full transition-all duration-300"
                style="width: {(bulkProgress.current / bulkProgress.total) * 100}%"
              ></div>
            </div>
            <p class="text-sm text-gray-500 dark:text-gray-400">
              ⏳ {$translations('common.loading')}...
            </p>
          </div>
        </div>
      {/if}

      <!-- View Tabs -->
      <div class="mb-6">
        <div class="border-b border-gray-200 dark:border-gray-700">
          <nav class="-mb-px grid grid-cols-6 gap-x-4 gap-y-2">
            <button
              onclick={() => currentView = 'list'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'list'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              📋 {$translations('navigation.flights')}
            </button>
            <button
              onclick={() => currentView = 'map'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'map'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🗺️ World Map
            </button>
            <button
              onclick={() => currentView = 'globe'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'globe'
                  ? $theme === 'cyberpunk' ? 'border-cyan-500 text-cyan-500'
                    : $theme === 'skynet' ? 'border-blue-500 text-blue-500'
                    : 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🌐 3D Globe
            </button>
            <button
              onclick={() => currentView = 'documents'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'documents'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              📄 {$translations('navigation.documents')}
            </button>
            <button
              onclick={() => currentView = 'analytics'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'analytics'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              📊 {$translations('navigation.analytics')}
            </button>
            <button
              onclick={() => currentView = 'visualizations'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'visualizations'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🎨 Visualizations
            </button>
            <button
              onclick={() => currentView = 'carbon'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'carbon'
                  ? 'border-green-600 text-green-600 dark:border-green-400 dark:text-green-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🌍 Carbon
            </button>
            <button
              onclick={() => currentView = 'passengers'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'passengers'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              👥 {$translations('navigation.passengers')}
            </button>
            <button
              onclick={() => currentView = 'journeys'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'journeys'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🗺️ {$translations('navigation.journeys')}
            </button>
            <button
              onclick={() => currentView = 'logbook'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'logbook'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              📝 {$translations('navigation.logbook')}
            </button>
            <button
              onclick={() => currentView = 'ffp'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'ffp'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🎫 {$translations('navigation.frequentFlyer')}
            </button>
            <button
              onclick={() => currentView = 'fuel'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'fuel'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              ⛽ Fuel
            </button>
            <button
              onclick={() => currentView = 'media'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'media'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🖼️ Media
            </button>
            <button
              onclick={() => currentView = 'schemas'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'schemas'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🗄️ Custom DB
            </button>
            <button
              onclick={() => currentView = 'insights'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'insights'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🧠 Insights
            </button>
            <button
              onclick={() => currentView = 'airports'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'airports'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              ✈️ {$translations('navigation.airports')}
            </button>
            <button
              onclick={() => currentView = 'aircraft'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'aircraft'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🛩️ {$translations('navigation.aircraft')}
            </button>
            <button
              onclick={() => currentView = 'researchers'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'researchers'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🤖 AI Agents
            </button>
            <button
              onclick={() => currentView = 'researchdocs'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'researchdocs'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              📝 AI Reports
            </button>
            <button
              onclick={() => currentView = 'warroom'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'warroom'
                  ? $theme === 'cyberpunk' ? 'border-cyan-500 text-cyan-500'
                    : $theme === 'skynet' ? 'border-blue-500 text-blue-500'
                    : 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🤝 Relationships
            </button>
            <button
              onclick={() => currentView = 'workflows'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'workflows'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🔄 Workflows
            </button>
            <button
              onclick={() => currentView = 'sentinel'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'sentinel'
                  ? 'border-red-600 text-red-600 dark:border-red-400 dark:text-red-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              🛡️ Sentinel
            </button>
            {#if $developerMode}
              <button
                onclick={() => currentView = 'commandcentre'}
                class="border-b-2 py-4 px-1 text-sm font-medium transition {
                  currentView === 'commandcentre'
                    ? $theme === 'cyberpunk' ? 'border-cyan-500 text-cyan-500 animate-pulse'
                      : $theme === 'skynet' ? 'border-blue-500 text-blue-500 animate-pulse'
                      : 'border-green-500 text-green-500 animate-pulse'
                    : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
                }"
              >
                🎮 Command Centre
              </button>
            {/if}
            <button
              onclick={() => currentView = 'settings'}
              class="border-b-2 py-4 px-1 text-sm font-medium transition {
                currentView === 'settings'
                  ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
                  : 'border-transparent text-gray-500 dark:text-gray-400 hover:border-gray-300 hover:text-gray-700 dark:hover:text-gray-300'
              }"
            >
              ⚙️ {$translations('navigation.settings')}
            </button>
          </nav>
        </div>
      </div>

      <!-- List View -->
      {#if currentView === 'list'}
        {#if flights.length > 0}
          <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
            <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
              <div class="flex items-center justify-between mb-4">
                <h2 class="text-xl font-semibold text-gray-900 dark:text-white">
                  {$translations('flights.title')} ({filteredFlights().length} of {flights.length})
                </h2>
                <div class="flex items-center gap-2">
                  {#if filteredFlights().length > 0}
                    <button
                      onclick={() => showBulkInvestigation = true}
                      class="text-sm px-4 py-2 bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-700 hover:to-indigo-700 text-white rounded-lg font-medium transition shadow-lg"
                      title="Bulk AI Investigation with intelligent retry handling"
                    >
                      🤖 Bulk Investigate ({filteredFlights().length})
                    </button>
                  {/if}
                  {#if hasActiveFilters}
                    <button
                      onclick={clearFilters}
                      class="text-sm px-3 py-1 bg-gray-600 hover:bg-gray-700 text-white rounded transition"
                    >
                      {$translations('common.clear')} {$translations('common.filter')}
                    </button>
                  {/if}
                </div>
              </div>

              <!-- Search and Filters -->
              <div class="space-y-3">
                <!-- Search Bar -->
                <div class="relative">
                  <input
                    id="search-query"
                    type="text"
                    bind:value={searchQuery}
                    placeholder="{$translations('common.search')}..."
                    class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-lg px-4 py-2 pl-10 text-gray-900 dark:text-white"
                    aria-label="{$translations('common.search')}"
                  />
                  <svg class="absolute left-3 top-2.5 h-5 w-5 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                  </svg>
                </div>

                <!-- Quick Filter Bar -->
                <div class="flex flex-wrap gap-2">
                  <span class="text-xs text-gray-500 dark:text-gray-400 self-center mr-1">Quick:</span>
                  <button
                    onclick={() => quickFilter = 'all'}
                    class="px-3 py-1 text-xs font-medium rounded-full transition {quickFilter === 'all' ? 'bg-primary-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'}"
                  >
                    All
                  </button>
                  <button
                    onclick={() => quickFilter = 'popular'}
                    class="px-3 py-1 text-xs font-medium rounded-full transition {quickFilter === 'popular' ? 'bg-green-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'}"
                  >
                    🔥 Popular
                  </button>
                  <button
                    onclick={() => quickFilter = 'rare'}
                    class="px-3 py-1 text-xs font-medium rounded-full transition {quickFilter === 'rare' ? 'bg-purple-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'}"
                  >
                    💎 Rare
                  </button>
                  <span class="text-gray-300 dark:text-gray-600 self-center">|</span>
                  <button
                    onclick={() => quickFilter = 'A-E'}
                    class="px-3 py-1 text-xs font-medium rounded-full transition {quickFilter === 'A-E' ? 'bg-blue-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'}"
                  >
                    A-E
                  </button>
                  <button
                    onclick={() => quickFilter = 'F-J'}
                    class="px-3 py-1 text-xs font-medium rounded-full transition {quickFilter === 'F-J' ? 'bg-blue-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'}"
                  >
                    F-J
                  </button>
                  <button
                    onclick={() => quickFilter = 'K-O'}
                    class="px-3 py-1 text-xs font-medium rounded-full transition {quickFilter === 'K-O' ? 'bg-blue-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'}"
                  >
                    K-O
                  </button>
                  <button
                    onclick={() => quickFilter = 'P-T'}
                    class="px-3 py-1 text-xs font-medium rounded-full transition {quickFilter === 'P-T' ? 'bg-blue-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'}"
                  >
                    P-T
                  </button>
                  <button
                    onclick={() => quickFilter = 'U-Z'}
                    class="px-3 py-1 text-xs font-medium rounded-full transition {quickFilter === 'U-Z' ? 'bg-blue-600 text-white' : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'}"
                  >
                    U-Z
                  </button>
                </div>

                <!-- Advanced Filters -->
                <div class="grid grid-cols-1 md:grid-cols-4 gap-3">
                  <div>
                    <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{$translations('common.date')} From</label>
                    <input
                      type="date"
                      value={filterDateFrom}
                      onchange={(e) => filterDateFrom = (e.target as HTMLInputElement).value}
                      class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded px-3 py-1.5 text-sm text-gray-900 dark:text-white"
                    />
                  </div>
                  <div>
                    <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{$translations('common.date')} To</label>
                    <input
                      type="date"
                      value={filterDateTo}
                      onchange={(e) => filterDateTo = (e.target as HTMLInputElement).value}
                      class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded px-3 py-1.5 text-sm text-gray-900 dark:text-white"
                    />
                  </div>
                  <div>
                    <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{$translations('flights.departure')}</label>
                    <select
                      bind:value={filterOrigin}
                      class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded px-3 py-1.5 text-sm text-gray-900 dark:text-white"
                    >
                      <option value="">All Departures</option>
                      {#each uniqueDepartureAirports() as airport}
                        <option value={airport}>{airport}</option>
                      {/each}
                    </select>
                  </div>
                  <div>
                    <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">{$translations('flights.arrival')}</label>
                    <select
                      bind:value={filterDestination}
                      class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded px-3 py-1.5 text-sm text-gray-900 dark:text-white"
                    >
                      <option value="">All Arrivals</option>
                      {#each uniqueArrivalAirports() as airport}
                        <option value={airport}>{airport}</option>
                      {/each}
                    </select>
                  </div>
                </div>
              </div>
            </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th
                    onclick={() => toggleSort('date')}
                    class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                  >
                    <div class="flex items-center gap-1">
                      {$translations('common.date')}
                      {#if sortColumn === 'date'}
                        <span class="text-primary-600">{sortDirection === 'asc' ? '↑' : '↓'}</span>
                      {/if}
                    </div>
                  </th>
                  <th
                    onclick={() => toggleSort('route')}
                    class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                  >
                    <div class="flex items-center gap-1">
                      {$translations('flights.details.routeInfo')}
                      {#if sortColumn === 'route'}
                        <span class="text-primary-600">{sortDirection === 'asc' ? '↑' : '↓'}</span>
                      {/if}
                    </div>
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{$translations('flights.flightNumber')}</th>
                  <th
                    onclick={() => toggleSort('distance')}
                    class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition"
                  >
                    <div class="flex items-center gap-1">
                      {$translations('flights.distance')}
                      {#if sortColumn === 'distance'}
                        <span class="text-primary-600">{sortDirection === 'asc' ? '↑' : '↓'}</span>
                      {/if}
                    </div>
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{$translations('flights.notes')}</th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{$translations('common.actions')}</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each filteredFlights() as flight}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition cursor-pointer">
                    <td
                      onclick={() => selectedFlight = flight}
                      class="px-6 py-4 whitespace-nowrap text-sm text-gray-900 dark:text-gray-100"
                    >
                      {formatDate(flight.departure_datetime)}
                    </td>
                    <td
                      onclick={() => selectedFlight = flight}
                      class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-gray-100"
                    >
                      {flight.departure_airport} → {flight.arrival_airport}
                    </td>
                    <td
                      onclick={() => selectedFlight = flight}
                      class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400"
                    >
                      {flight.flight_number || '-'}
                    </td>
                    <td
                      onclick={() => selectedFlight = flight}
                      class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400"
                    >
                      {flight.distance_km ? `${Math.round(flight.distance_km)} km` : '-'}
                    </td>
                    <td
                      onclick={() => selectedFlight = flight}
                      class="px-6 py-4 text-sm text-gray-600 dark:text-gray-400 truncate max-w-xs"
                    >
                      {flight.notes || '-'}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm">
                      <button
                        onclick={(e) => { e.stopPropagation(); investigatingFlight = flight; }}
                        class="bg-indigo-600 hover:bg-indigo-700 text-white px-3 py-1 rounded text-xs font-medium transition"
                        title="AI-Powered Investigation"
                      >
                        🔍 {$translations('research.newResearch')}
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
        {:else}
          <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
            <div class="text-6xl mb-4">✈️</div>
            <h3 class="text-xl font-semibold mb-2 text-gray-900 dark:text-white">{$translations('flights.title')}</h3>
            <p class="text-gray-600 dark:text-gray-400 mb-6">
              {$translations('flights.import.title')}
            </p>
            <button
              onclick={() => currentView = 'csvimport'}
              class="bg-primary-600 hover:bg-primary-700 text-white px-6 py-3 rounded-lg font-medium transition"
            >
              {$translations('flights.importFlights')}
            </button>
          </div>
        {/if}
      {/if}

      <!-- Map View -->
      {#if currentView === 'map'}
        <div class="rounded-lg shadow overflow-hidden p-4 {mapDarkMode ? 'bg-gray-900' : 'bg-white dark:bg-gray-800'}">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold {mapDarkMode ? 'text-white' : 'text-gray-900 dark:text-white'}">
              {mapMode === 'flights' ? $translations('flights.details.routeInfo') : '🛡️ Network Connections'}
            </h3>
            <div class="flex items-center gap-3">
              <!-- Mode Toggle -->
              <div class="flex items-center rounded-lg p-1 {mapDarkMode ? 'bg-gray-800' : 'bg-gray-100 dark:bg-gray-700'}">
                <button
                  onclick={() => mapMode = 'flights'}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition {
                    mapMode === 'flights'
                      ? mapDarkMode ? 'bg-gray-700 text-white shadow-sm' : 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm'
                      : mapDarkMode ? 'text-gray-400 hover:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
                  }"
                >
                  ✈️ Flights
                </button>
                <button
                  onclick={async () => {
                    mapMode = 'network';
                    if (networkConnectionsForMap.length === 0) {
                      await loadNetworkDataForMap();
                    }
                  }}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition {
                    mapMode === 'network'
                      ? mapDarkMode ? 'bg-gray-700 text-white shadow-sm' : 'bg-white dark:bg-gray-600 text-gray-900 dark:text-white shadow-sm'
                      : mapDarkMode ? 'text-gray-400 hover:text-white' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
                  }"
                >
                  🌐 Network
                </button>
              </div>
              <!-- Refresh network data button (only in network mode) -->
              {#if mapMode === 'network'}
                <button
                  onclick={loadNetworkDataForMap}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium bg-cyan-600 hover:bg-cyan-700 text-white transition"
                >
                  🔄 Refresh
                </button>
              {/if}
              <!-- Dark mode toggle -->
              <button
                onclick={() => mapDarkMode = !mapDarkMode}
                class="flex items-center gap-2 px-4 py-2 rounded-lg transition {
                  mapDarkMode
                    ? $theme === 'skynet' ? 'bg-blue-600 text-white hover:bg-blue-700' : 'bg-purple-600 text-white hover:bg-purple-700'
                    : 'bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-gray-600'
                }"
              >
                {mapDarkMode ? $theme === 'skynet' ? '🤖' : '🌃' : '🌞'}
                {mapDarkMode ? $theme === 'skynet' ? 'Skynet Mode' : 'Neon Mode' : `${$translations('settings.general.themes.light')} Mode`}
              </button>
            </div>
          </div>

          {#if mapMode === 'flights' && allFlights.length === 0}
            <div class="h-[600px] flex items-center justify-center">
              <div class="text-center">
                <div class="text-6xl mb-4">🗺️</div>
                <h3 class="text-xl font-semibold mb-2 text-gray-900 dark:text-white">{$translations('flights.title')}</h3>
                <p class="text-gray-600 dark:text-gray-400 mb-6">
                  {$translations('flights.import.title')}
                </p>
                <button
                  onclick={() => currentView = 'csvimport'}
                  class="bg-primary-600 hover:bg-primary-700 text-white px-6 py-3 rounded-lg font-medium transition"
                >
                  {$translations('flights.importFlights')}
                </button>
              </div>
            </div>
          {:else if mapMode === 'network' && networkConnectionsForMap.length === 0}
            <div class="h-[600px] flex items-center justify-center">
              <div class="text-center">
                <div class="text-6xl mb-4">🌐</div>
                <h3 class="text-xl font-semibold mb-2 text-gray-900 dark:text-white">No Network Data</h3>
                <p class="text-gray-600 dark:text-gray-400 mb-6">
                  Network Sentinel needs to be running to collect connection data.
                </p>
                <button
                  onclick={loadNetworkDataForMap}
                  class="bg-cyan-600 hover:bg-cyan-700 text-white px-6 py-3 rounded-lg font-medium transition"
                >
                  🔄 Try Loading Again
                </button>
              </div>
            </div>
          {:else}
            <div class="h-[600px] w-full">
              <FlightMap
                flights={allFlights}
                darkMode={mapDarkMode}
                theme={$theme}
                mode={mapMode}
                networkConnections={networkConnectionsForMap}
                userId={user?.id}
              />
            </div>
          {/if}

          <!-- Legend -->
          <div class="mt-4 p-4 rounded-lg {mapDarkMode ? 'bg-gray-800' : 'bg-gray-50 dark:bg-gray-900'}">
            <h3 class="text-sm font-semibold mb-2 {mapDarkMode ? 'text-gray-300' : 'text-gray-700 dark:text-gray-300'}">
              {mapMode === 'flights' ? 'Flight Map Legend' : 'Network Map Legend'}
            </h3>
            {#if mapMode === 'flights'}
              <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs {mapDarkMode ? 'text-gray-400' : 'text-gray-600 dark:text-gray-400'}">
                <div class="flex items-center gap-2">
                  <div class="w-4 h-1 {$theme === 'skynet' ? 'bg-[#00b4ff]' : mapDarkMode ? 'bg-[#00ff88]' : 'bg-green-500'}"></div>
                  <span>Low CO₂ (&lt;5,000 kg)</span>
                </div>
                <div class="flex items-center gap-2">
                  <div class="w-4 h-1 {$theme === 'skynet' ? 'bg-[#0040ff]' : mapDarkMode ? 'bg-[#ffaa00]' : 'bg-orange-500'}"></div>
                  <span>Medium CO₂ (5,000-10,000 kg)</span>
                </div>
                <div class="flex items-center gap-2">
                  <div class="w-4 h-1 {$theme === 'skynet' ? 'bg-[#6600ff]' : mapDarkMode ? 'bg-[#ff0080]' : 'bg-red-500'}"></div>
                  <span>High CO₂ (&gt;10,000 kg)</span>
                </div>
              </div>
              <p class="mt-2 text-xs text-gray-500 dark:text-gray-400">
                Click on flight routes or airports for details. Line thickness represents flight frequency.
              </p>
            {:else}
              <div class="grid grid-cols-1 md:grid-cols-4 gap-4 text-xs {mapDarkMode ? 'text-gray-400' : 'text-gray-600 dark:text-gray-400'}">
                <div class="flex items-center gap-2">
                  <div class="w-4 h-1 bg-[#00ff88]"></div>
                  <span>Local Machine</span>
                </div>
                <div class="flex items-center gap-2">
                  <div class="w-4 h-1 bg-[#00b4ff]"></div>
                  <span>Normal Connection</span>
                </div>
                <div class="flex items-center gap-2">
                  <div class="w-4 h-1 bg-[#b000ff]"></div>
                  <span>AI/Claude Process</span>
                </div>
                <div class="flex items-center gap-2">
                  <div class="w-4 h-1 bg-[#ff0040]"></div>
                  <span>Anomaly Detected</span>
                </div>
              </div>
              <p class="mt-2 text-xs text-gray-500 dark:text-gray-400">
                Click on connections or locations for details. Shows outbound network connections from Network Sentinel.
                <span class="font-medium text-cyan-500">{networkConnectionsForMap.length} unique destinations</span>
              </p>
            {/if}
            {#if $theme === 'skynet'}
              <p class="mt-1 text-xs"><span class="text-blue-400 font-semibold">Skynet mode active!</span></p>
            {:else if mapDarkMode}
              <p class="mt-1 text-xs"><span class="text-purple-400 font-semibold">Neon mode active!</span></p>
            {/if}
          </div>
        </div>
      {/if}

      <!-- 3D Globe View -->
      {#if currentView === 'globe'}
        <div class="bg-gray-900 rounded-lg shadow overflow-hidden" style="height: 800px;">
          <!-- Globe Controls Header -->
          <div class="flex items-center justify-between p-4 border-b border-gray-800">
            <h3 class="text-lg font-semibold text-white">
              {globeMode === 'flights' ? '🌐 3D Flight Globe' : '🛡️ 3D Network Globe'}
            </h3>
            <div class="flex items-center gap-3">
              <!-- Mode Toggle -->
              <div class="flex items-center bg-gray-800 rounded-lg p-1">
                <button
                  onclick={() => globeMode = 'flights'}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition {
                    globeMode === 'flights'
                      ? 'bg-gray-700 text-white shadow-sm'
                      : 'text-gray-400 hover:text-white'
                  }"
                >
                  ✈️ Flights
                </button>
                <button
                  onclick={async () => {
                    globeMode = 'network';
                    if (networkConnectionsForMap.length === 0) {
                      await loadNetworkDataForMap();
                    }
                  }}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium transition {
                    globeMode === 'network'
                      ? 'bg-gray-700 text-white shadow-sm'
                      : 'text-gray-400 hover:text-white'
                  }"
                >
                  🌐 Network
                </button>
              </div>
              <!-- Refresh button (network mode only) -->
              {#if globeMode === 'network'}
                <button
                  onclick={loadNetworkDataForMap}
                  class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium bg-cyan-600 hover:bg-cyan-700 text-white transition"
                >
                  🔄 Refresh
                </button>
              {/if}
            </div>
          </div>
          <!-- Globe Container -->
          <div style="height: calc(100% - 72px);">
            <GlobeVisualization
              flights={allFlights}
              userId={user?.id}
              mode={globeMode}
              networkConnections={networkConnectionsForMap}
            />
          </div>
        </div>
      {/if}

      <!-- Documents View -->
      {#if currentView === 'documents'}
        <Documents userId={user.id} />
      {/if}

      <!-- Analytics View -->
      {#if currentView === 'analytics'}
        <Analytics userId={user.id} />
      {/if}

      <!-- Visualizations View -->
      {#if currentView === 'visualizations'}
        <Visualizations userId={user.id} />
      {/if}

      <!-- Carbon Footprint View -->
      {#if currentView === 'carbon'}
        <CarbonFootprint userId={user.id} />
      {/if}

      <!-- Passengers View -->
      {#if currentView === 'passengers'}
        <Passengers userId={user.id} />
      {/if}

      <!-- Journeys View -->
      {#if currentView === 'journeys'}
        <Journeys userId={user.id} />
      {/if}

      <!-- Pilot Logbook View -->
      {#if currentView === 'logbook'}
        <PilotLogbook />
      {/if}

      <!-- FFP View -->
      {#if currentView === 'ffp'}
        <FrequentFlyerPrograms userId={user.id} />
      {/if}

      <!-- Fuel Tracker View -->
      {#if currentView === 'fuel'}
        <FuelTracker userId={user.id} />
      {/if}

      <!-- Media Gallery View -->
      {#if currentView === 'media'}
        <MediaGallery userId={user.id} />
      {/if}

      <!-- Custom Schema Builder View -->
      {#if currentView === 'schemas'}
        <SchemaBuilder userId={user.id} />
      {/if}

      <!-- Insights View -->
      {#if currentView === 'insights'}
        <Insights userId={user.id} />
      {/if}

      <!-- Airports View -->
      {#if currentView === 'airports'}
        <Airports />
      {/if}

      <!-- Aircraft Types View -->
      {#if currentView === 'aircraft'}
        <AircraftTypes />
      {/if}

      <!-- Researchers View -->
      {#if currentView === 'researchers'}
        <Researchers user={user} />
      {/if}

      <!-- Research Documents View -->
      {#if currentView === 'researchdocs'}
        <ResearchDocuments user={user} />
      {/if}

      <!-- Relationships View -->
      {#if currentView === 'warroom'}
        <div class="relationships-container space-y-6 {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}">
          <!-- Relationships Header -->
          <div class="relationships-header rounded-lg p-6">
            <div class="flex items-center justify-between mb-4">
              <div>
                <h2 class="relationships-title text-3xl font-bold">
                  RELATIONSHIP TRACER
                </h2>
                <p class="relationships-subtitle mt-2">
                  Visualize connections between people, flights, locations, and documents
                </p>
              </div>
            </div>

            <!-- Search Interface -->
            <div class="grid grid-cols-1 md:grid-cols-4 gap-4 items-end">
              <div>
                <label for="war-room-entity-type" class="relationships-label block text-sm font-medium mb-2">
                  Entity Type
                </label>
                <select
                  id="war-room-entity-type"
                  bind:value={warRoomEntityType}
                  class="relationships-select w-full px-4 py-3 rounded-lg focus:outline-none"
                >
                  <option value="person">Person</option>
                  <option value="flight">Flight</option>
                  <option value="location">Airport/Location</option>
                </select>
              </div>

              <div class="md:col-span-2">
                <label for="war-room-target" class="relationships-label block text-sm font-medium mb-2">
                  Select Target
                </label>
                <select
                  id="war-room-target"
                  bind:value={warRoomTarget}
                  disabled={warRoomLoadingEntities || warRoomEntityOptions.length === 0}
                  class="relationships-select w-full px-4 py-3 rounded-lg focus:outline-none disabled:opacity-50"
                >
                  {#if warRoomLoadingEntities}
                    <option value="">Loading...</option>
                  {:else if warRoomEntityOptions.length === 0}
                    <option value="">No {warRoomEntityType === 'person' ? 'passengers' : warRoomEntityType === 'flight' ? 'flights' : 'locations'} found</option>
                  {:else}
                    <option value="">Select a {warRoomEntityType}...</option>
                    {#each warRoomEntityOptions as entity}
                      <option value={entity.id}>{entity.label}</option>
                    {/each}
                  {/if}
                </select>
              </div>

              <div>
                <label for="war-room-max-depth" class="relationships-label block text-sm font-medium mb-2">
                  Depth
                </label>
                <select
                  id="war-room-max-depth"
                  bind:value={warRoomMaxDepth}
                  class="relationships-select w-full px-4 py-3 rounded-lg focus:outline-none"
                >
                  <option value={1}>1 (Direct)</option>
                  <option value={2}>2 (Extended)</option>
                  <option value={3}>3 (Deep)</option>
                  <option value={4}>4 (Maximum)</option>
                </select>
              </div>
            </div>

            <div class="mt-4 flex gap-4 flex-wrap">
              <button
                onclick={() => {
                  if (warRoomTarget.trim()) {
                    warRoomActive = true;
                  } else {
                    alert('Please enter a target entity');
                  }
                }}
                disabled={!warRoomTarget.trim()}
                class="relationships-trace-btn px-6 py-3 font-semibold rounded-lg transition-all transform hover:scale-105 disabled:scale-100 disabled:cursor-not-allowed disabled:opacity-50"
              >
                Trace Network
              </button>

              <button
                onclick={buildRelationshipGraph}
                disabled={warRoomBuilding}
                class="relationships-build-btn px-6 py-3 font-semibold rounded-lg transition-all transform hover:scale-105 disabled:opacity-50 disabled:cursor-wait"
              >
                {warRoomBuilding ? '⏳ Building...' : '🔨 Build Graph'}
              </button>

              <button
                onclick={loadRelationshipStats}
                class="relationships-stats-btn px-4 py-3 font-semibold rounded-lg transition-all"
                title="Refresh statistics"
              >
                📊
              </button>

              {#if warRoomActive}
                <button
                  onclick={() => {
                    warRoomActive = false;
                    warRoomTarget = '';
                  }}
                  class="relationships-reset-btn px-6 py-3 font-semibold rounded-lg transition-all"
                >
                  Reset
                </button>
              {/if}
            </div>

            <!-- Graph Statistics -->
            {#if warRoomStats}
              <div class="mt-4 p-3 rounded-lg relationships-stats-panel">
                <div class="grid grid-cols-4 gap-4 text-center text-sm">
                  <div>
                    <div class="text-2xl font-bold relationships-stats-number">{warRoomStats.total_relationships}</div>
                    <div class="text-xs opacity-60">Relationships</div>
                  </div>
                  <div>
                    <div class="text-2xl font-bold text-cyan-400">{warRoomStats.person_count}</div>
                    <div class="text-xs opacity-60">People</div>
                  </div>
                  <div>
                    <div class="text-2xl font-bold text-purple-400">{warRoomStats.flight_count}</div>
                    <div class="text-xs opacity-60">Flights</div>
                  </div>
                  <div>
                    <div class="text-2xl font-bold text-emerald-400">{warRoomStats.airport_count}</div>
                    <div class="text-xs opacity-60">Airports</div>
                  </div>
                </div>
              </div>
            {:else}
              <div class="mt-4 p-3 rounded-lg relationships-stats-panel text-center text-sm opacity-60">
                <p>No graph data. Click "Build Graph" to analyze flight relationships.</p>
              </div>
            {/if}

            <!-- Quick Examples -->
            <div class="relationships-examples mt-4 pt-4">
              <p class="text-xs mb-2 opacity-60">Quick examples:</p>
              <div class="flex flex-wrap gap-2">
                <button
                  onclick={() => {
                    warRoomTarget = 'N908JE';
                    warRoomEntityType = 'flight';
                    warRoomActive = true;
                  }}
                  class="relationships-example-btn px-3 py-1 text-xs rounded transition-colors"
                >
                  Trace N908JE
                </button>
                <button
                  onclick={() => {
                    warRoomTarget = 'Santa Fe';
                    warRoomEntityType = 'location';
                    warRoomActive = true;
                  }}
                  class="relationships-example-btn px-3 py-1 text-xs rounded transition-colors"
                >
                  Santa Fe Connections
                </button>
              </div>
            </div>
          </div>

          <!-- Relationship Graph -->
          {#if warRoomActive && warRoomTarget.trim()}
            <div class="relationships-graph-container rounded-lg p-4" style="height: 800px;">
              <RelationshipGraph
                rootType={warRoomEntityType === 'location' ? 'airport' : warRoomEntityType}
                rootId={warRoomTarget}
                maxDepth={warRoomMaxDepth}
              />
            </div>
          {:else}
            <div class="relationships-empty rounded-lg p-12 text-center">
              <h3 class="relationships-empty-title text-xl font-semibold mb-2">Ready to Trace</h3>
              <p class="relationships-empty-text">
                Enter a target entity above and click "Trace Network" to visualize the web of connections.
              </p>
              <div class="mt-6 grid grid-cols-2 md:grid-cols-4 gap-4 max-w-2xl mx-auto text-left">
                <div class="relationships-card p-4 rounded">
                  <p class="relationships-card-title text-sm font-semibold">People</p>
                  <p class="relationships-card-desc text-xs mt-1">Track passengers and associates</p>
                </div>
                <div class="relationships-card p-4 rounded">
                  <p class="relationships-card-title text-sm font-semibold">Flights</p>
                  <p class="relationships-card-desc text-xs mt-1">Trace aircraft movements</p>
                </div>
                <div class="relationships-card p-4 rounded">
                  <p class="relationships-card-title text-sm font-semibold">Documents</p>
                  <p class="relationships-card-desc text-xs mt-1">Discover evidence trails</p>
                </div>
                <div class="relationships-card p-4 rounded">
                  <p class="relationships-card-title text-sm font-semibold">Locations</p>
                  <p class="relationships-card-desc text-xs mt-1">Map geographic connections</p>
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/if}

      {#if currentView === 'csvimport'}
        <CsvImportPreview user={user} onImportComplete={async () => {
          await loadData();
          currentView = 'list';
        }} />
      {/if}
    </div>
  {/if}

  <!-- New Dataset Wizard -->
  {#if showNewDatasetWizard}
    <SetupWizard onComplete={handleDatasetCreated} asModal={true} />
  {/if}

  <!-- Manual Flight Entry Modal -->
  {#if showManualEntry && user}
    <ManualFlightEntry
      userId={user.id}
      onClose={() => showManualEntry = false}
      onSuccess={loadData}
    />
  {/if}

  <!-- Batch OCR Uploader Modal -->
  {#if showBatchOCRUploader && user}
    <BatchOCRUploader
      isOpen={showBatchOCRUploader}
      onClose={() => showBatchOCRUploader = false}
      onComplete={handleBatchOCRComplete}
    />
  {/if}

  <!-- Donate Modal -->
  <DonateModal
    visible={showDonateModal}
    onClose={() => showDonateModal = false}
  />

  <!-- Workflows View -->
  {#if currentView === 'workflows'}
    <div class="h-screen">
      <TaskTreeBuilder />
    </div>
  {/if}

  <!-- Network Sentinel View -->
  {#if currentView === 'sentinel'}
    <div class="h-screen">
      <NetworkSentinel />
    </div>
  {/if}

  <!-- Command Centre View (Developer Mode Only) -->
  {#if currentView === 'commandcentre'}
    <div class="h-screen">
      <CommandCentre />
    </div>
  {/if}

  <!-- Settings View -->
  {#if currentView === 'settings' && user}
    <Settings
      user={user}
      onReset={handleReset}
    />
  {/if}

  <!-- Investigation Panel -->
  {#if investigatingFlight}
    <InvestigationPanel
      flight={investigatingFlight}
      onClose={() => investigatingFlight = null}
    />
  {/if}

  <!-- Bulk Investigation Panel -->
  {#if showBulkInvestigation}
    <BulkInvestigation
      flights={filteredFlights()}
      onClose={() => showBulkInvestigation = false}
    />
  {/if}

  <!-- Flight Detail Modal -->
  {#if selectedFlight}
    <FlightDetail
      flight={selectedFlight}
      onClose={() => selectedFlight = null}
      onInvestigate={() => {
        investigatingFlight = selectedFlight;
        selectedFlight = null;
      }}
    />
  {/if}

  <!-- Airport List Modal -->
  {#if showAirportList && user}
    <AirportList
      userId={user.id}
      onClose={() => showAirportList = false}
    />
  {/if}

  <!-- Total Flights Popup -->
  {#if showFlightsPopup}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onclick={() => showFlightsPopup = false}
      onkeydown={(e) => e.key === 'Escape' && (showFlightsPopup = false)}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div
        class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl p-8 max-w-md w-full mx-4 transform transition-all"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between mb-6">
          <div class="flex items-center gap-3">
            <span class="text-5xl">✈️</span>
            <h2 class="text-2xl font-bold text-gray-900 dark:text-white">{$translations('logbook.totalFlights')}</h2>
          </div>
          <button
            onclick={() => showFlightsPopup = false}
            class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full transition"
          >
            <svg class="w-6 h-6 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="space-y-4">
          <div class="text-center py-4 bg-primary-50 dark:bg-primary-900/30 rounded-xl">
            <p class="text-5xl font-bold text-primary-600 dark:text-primary-400">{stats.total_flights}</p>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">flights recorded</p>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
              <p class="text-sm text-gray-500 dark:text-gray-400">Average per month</p>
              <p class="text-xl font-bold text-gray-900 dark:text-white">
                {stats.total_flights > 0 ? Math.round(stats.total_flights / 12) : 0}
              </p>
            </div>
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
              <p class="text-sm text-gray-500 dark:text-gray-400">Average per week</p>
              <p class="text-xl font-bold text-gray-900 dark:text-white">
                {stats.total_flights > 0 ? (stats.total_flights / 52).toFixed(1) : 0}
              </p>
            </div>
          </div>

          <div class="p-4 bg-blue-50 dark:bg-blue-900/30 rounded-xl">
            <p class="text-sm text-blue-700 dark:text-blue-300">
              {stats.total_flights >= 100 ? '🏆 Frequent Flyer Status!' : stats.total_flights >= 50 ? '⭐ Regular Traveler' : '✈️ Getting Started'}
            </p>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Total Distance Popup -->
  {#if showDistancePopup}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onclick={() => showDistancePopup = false}
      onkeydown={(e) => e.key === 'Escape' && (showDistancePopup = false)}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div
        class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl p-8 max-w-md w-full mx-4 transform transition-all"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between mb-6">
          <div class="flex items-center gap-3">
            <span class="text-5xl">🌍</span>
            <h2 class="text-2xl font-bold text-gray-900 dark:text-white">{$translations('passengers.totalDistance')}</h2>
          </div>
          <button
            onclick={() => showDistancePopup = false}
            class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full transition"
          >
            <svg class="w-6 h-6 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="space-y-4">
          <div class="text-center py-4 bg-blue-50 dark:bg-blue-900/30 rounded-xl">
            <p class="text-4xl font-bold text-blue-600 dark:text-blue-400">{Math.round(stats.total_distance_km).toLocaleString()}</p>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">kilometers traveled</p>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
              <p class="text-sm text-gray-500 dark:text-gray-400">In miles</p>
              <p class="text-xl font-bold text-gray-900 dark:text-white">
                {Math.round(stats.total_distance_km * 0.621371).toLocaleString()} mi
              </p>
            </div>
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
              <p class="text-sm text-gray-500 dark:text-gray-400">Per flight avg</p>
              <p class="text-xl font-bold text-gray-900 dark:text-white">
                {stats.total_flights > 0 ? Math.round(stats.total_distance_km / stats.total_flights).toLocaleString() : 0} km
              </p>
            </div>
          </div>

          <div class="space-y-2">
            <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <span class="text-sm text-gray-600 dark:text-gray-400">🌍 Earth circumferences</span>
              <span class="font-bold text-gray-900 dark:text-white">{(stats.total_distance_km / 40075).toFixed(2)}x</span>
            </div>
            <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <span class="text-sm text-gray-600 dark:text-gray-400">🌙 To the Moon</span>
              <span class="font-bold text-gray-900 dark:text-white">{((stats.total_distance_km / 384400) * 100).toFixed(1)}%</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Duration Popup -->
  {#if showDurationPopup}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onclick={() => showDurationPopup = false}
      onkeydown={(e) => e.key === 'Escape' && (showDurationPopup = false)}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div
        class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl p-8 max-w-md w-full mx-4 transform transition-all"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between mb-6">
          <div class="flex items-center gap-3">
            <span class="text-5xl">⏱️</span>
            <h2 class="text-2xl font-bold text-gray-900 dark:text-white">{$translations('flights.duration')}</h2>
          </div>
          <button
            onclick={() => showDurationPopup = false}
            class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full transition"
          >
            <svg class="w-6 h-6 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="space-y-4">
          <div class="text-center py-4 bg-amber-50 dark:bg-amber-900/30 rounded-xl">
            <p class="text-5xl font-bold text-amber-600 dark:text-amber-400">{Math.round(stats.total_flight_time_hours)}</p>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">hours in the air</p>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
              <p class="text-sm text-gray-500 dark:text-gray-400">In days</p>
              <p class="text-xl font-bold text-gray-900 dark:text-white">
                {(stats.total_flight_time_hours / 24).toFixed(1)} days
              </p>
            </div>
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
              <p class="text-sm text-gray-500 dark:text-gray-400">Average flight</p>
              <p class="text-xl font-bold text-gray-900 dark:text-white">
                {stats.total_flights > 0 ? (stats.total_flight_time_hours / stats.total_flights).toFixed(1) : 0}h
              </p>
            </div>
          </div>

          <div class="space-y-2">
            <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <span class="text-sm text-gray-600 dark:text-gray-400">📚 Books you could read</span>
              <span class="font-bold text-gray-900 dark:text-white">~{Math.round(stats.total_flight_time_hours / 6)}</span>
            </div>
            <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <span class="text-sm text-gray-600 dark:text-gray-400">🎬 Movies watched (2h each)</span>
              <span class="font-bold text-gray-900 dark:text-white">~{Math.round(stats.total_flight_time_hours / 2)}</span>
            </div>
            <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <span class="text-sm text-gray-600 dark:text-gray-400">😴 Full night sleeps</span>
              <span class="font-bold text-gray-900 dark:text-white">~{Math.round(stats.total_flight_time_hours / 8)}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- CO2 Emissions Popup -->
  {#if showCO2Popup}
    <div
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onclick={() => showCO2Popup = false}
      onkeydown={(e) => e.key === 'Escape' && (showCO2Popup = false)}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div
        class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl p-8 max-w-md w-full mx-4 transform transition-all"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="flex items-center justify-between mb-6">
          <div class="flex items-center gap-3">
            <span class="text-5xl">🌱</span>
            <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Carbon Footprint</h2>
          </div>
          <button
            onclick={() => showCO2Popup = false}
            class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full transition"
          >
            <svg class="w-6 h-6 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div class="space-y-4">
          <div class="text-center py-4 bg-green-50 dark:bg-green-900/30 rounded-xl">
            <p class="text-4xl font-bold text-green-600 dark:text-green-400">{Math.round(stats.total_carbon_kg).toLocaleString()}</p>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">kg of CO₂ emissions</p>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
              <p class="text-sm text-gray-500 dark:text-gray-400">In tonnes</p>
              <p class="text-xl font-bold text-gray-900 dark:text-white">
                {(stats.total_carbon_kg / 1000).toFixed(2)}t
              </p>
            </div>
            <div class="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
              <p class="text-sm text-gray-500 dark:text-gray-400">Per flight avg</p>
              <p class="text-xl font-bold text-gray-900 dark:text-white">
                {stats.total_flights > 0 ? Math.round(stats.total_carbon_kg / stats.total_flights) : 0} kg
              </p>
            </div>
          </div>

          <div class="space-y-2">
            <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <span class="text-sm text-gray-600 dark:text-gray-400">🚗 Car equivalent (km)</span>
              <span class="font-bold text-gray-900 dark:text-white">{Math.round(stats.total_carbon_kg / 0.21).toLocaleString()} km</span>
            </div>
            <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <span class="text-sm text-gray-600 dark:text-gray-400">🌳 Trees needed/year</span>
              <span class="font-bold text-gray-900 dark:text-white">{Math.round(stats.total_carbon_kg / 21)}</span>
            </div>
            <div class="flex justify-between items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <span class="text-sm text-gray-600 dark:text-gray-400">💨 Per km traveled</span>
              <span class="font-bold text-gray-900 dark:text-white">{stats.total_distance_km > 0 ? (stats.total_carbon_kg / stats.total_distance_km * 100).toFixed(1) : 0} g/km</span>
            </div>
          </div>

          <div class="p-4 bg-emerald-50 dark:bg-emerald-900/30 rounded-xl border border-emerald-200 dark:border-emerald-700">
            <p class="text-sm text-emerald-700 dark:text-emerald-300 font-medium">💡 Offset Tip</p>
            <p class="text-xs text-emerald-600 dark:text-emerald-400 mt-1">
              Consider carbon offset programs to neutralize your flight emissions.
              {Math.round(stats.total_carbon_kg / 21)} trees planted would absorb this in a year.
            </p>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
{/if}

<style>
  /* Relationships View - Default Theme */
  .theme-default .relationships-header {
    background: linear-gradient(135deg, rgba(79, 70, 229, 0.1), rgba(147, 51, 234, 0.1));
    border: 1px solid rgba(79, 70, 229, 0.3);
  }
  .theme-default .relationships-title {
    color: #4f46e5;
  }
  .theme-default .relationships-subtitle {
    color: #6b7280;
  }
  .theme-default .relationships-label {
    color: #374151;
  }
  .theme-default .relationships-input {
    background-color: #f3f4f6;
    border: 1px solid #d1d5db;
    color: #1f2937;
  }
  .theme-default .relationships-input::placeholder {
    color: #9ca3af;
  }
  .theme-default .relationships-input:focus {
    border-color: #4f46e5;
    box-shadow: 0 0 0 2px rgba(79, 70, 229, 0.2);
  }
  .theme-default .relationships-select {
    background-color: #f3f4f6;
    border: 1px solid #d1d5db;
    color: #1f2937;
  }
  .theme-default .relationships-select:focus {
    border-color: #4f46e5;
  }
  .theme-default .relationships-trace-btn {
    background: linear-gradient(135deg, #4f46e5, #7c3aed);
    color: white;
  }
  .theme-default .relationships-trace-btn:hover {
    background: linear-gradient(135deg, #4338ca, #6d28d9);
  }
  .theme-default .relationships-reset-btn {
    background-color: #6b7280;
    color: white;
  }
  .theme-default .relationships-reset-btn:hover {
    background-color: #4b5563;
  }
  .theme-default .relationships-examples {
    border-top: 1px solid #e5e7eb;
  }
  .theme-default .relationships-example-btn {
    background-color: rgba(79, 70, 229, 0.1);
    color: #4f46e5;
    border: 1px solid rgba(79, 70, 229, 0.3);
  }
  .theme-default .relationships-example-btn:hover {
    background-color: rgba(79, 70, 229, 0.2);
  }
  .theme-default .relationships-graph-container {
    background-color: #1f2937;
    border: 1px solid #374151;
  }
  .theme-default .relationships-empty {
    background-color: #1f2937;
    border: 1px solid #374151;
  }
  .theme-default .relationships-empty-title {
    color: #d1d5db;
  }
  .theme-default .relationships-empty-text {
    color: #9ca3af;
  }
  .theme-default .relationships-card {
    background-color: #374151;
    border: 1px solid #4b5563;
  }
  .theme-default .relationships-card-title {
    color: #60a5fa;
  }
  .theme-default .relationships-card-desc {
    color: #9ca3af;
  }

  /* Relationships View - Skynet Theme */
  .theme-skynet .relationships-header {
    background: linear-gradient(135deg, rgba(0, 40, 80, 0.8), rgba(0, 20, 40, 0.6));
    border: 1px solid rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.2);
  }
  .theme-skynet .relationships-title {
    color: #00b4ff;
    text-shadow: 0 0 20px rgba(0, 180, 255, 0.5);
    letter-spacing: 0.1em;
  }
  .theme-skynet .relationships-subtitle {
    color: rgba(0, 180, 255, 0.6);
  }
  .theme-skynet .relationships-label {
    color: rgba(0, 180, 255, 0.7);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .theme-skynet .relationships-input {
    background-color: rgba(0, 20, 40, 0.8);
    border: 1px solid rgba(0, 180, 255, 0.4);
    color: #00b4ff;
    box-shadow: inset 0 0 10px rgba(0, 180, 255, 0.1);
  }
  .theme-skynet .relationships-input::placeholder {
    color: rgba(0, 180, 255, 0.4);
  }
  .theme-skynet .relationships-input:focus {
    border-color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.4), inset 0 0 10px rgba(0, 180, 255, 0.1);
  }
  .theme-skynet .relationships-select {
    background-color: rgba(0, 20, 40, 0.8);
    border: 1px solid rgba(0, 180, 255, 0.4);
    color: #00b4ff;
  }
  .theme-skynet .relationships-select:focus {
    border-color: #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.4);
  }
  .theme-skynet .relationships-select option {
    background-color: #001428;
    color: #00b4ff;
  }
  .theme-skynet .relationships-trace-btn {
    background: linear-gradient(135deg, rgba(0, 80, 160, 0.8), rgba(0, 128, 255, 0.8));
    color: #ffffff;
    border: 1px solid rgba(0, 180, 255, 0.6);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }
  .theme-skynet .relationships-trace-btn:hover {
    background: linear-gradient(135deg, rgba(0, 100, 200, 0.9), rgba(0, 180, 255, 0.9));
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.6);
  }
  .theme-skynet .relationships-trace-btn:active {
    background: rgba(0, 20, 40, 0.9);
    border-color: #00b4ff;
    box-shadow: inset 0 0 15px rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .relationships-reset-btn {
    background: rgba(0, 40, 80, 0.6);
    color: rgba(0, 180, 255, 0.8);
    border: 1px solid rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .relationships-reset-btn:hover {
    background: rgba(0, 60, 120, 0.8);
    border-color: rgba(0, 180, 255, 0.5);
  }
  .theme-skynet .relationships-examples {
    border-top: 1px solid rgba(0, 180, 255, 0.2);
  }
  .theme-skynet .relationships-example-btn {
    background: rgba(0, 180, 255, 0.1);
    color: #00b4ff;
    border: 1px solid rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .relationships-example-btn:hover {
    background: rgba(0, 180, 255, 0.2);
    box-shadow: 0 0 10px rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .relationships-graph-container {
    background: linear-gradient(135deg, rgba(0, 10, 20, 0.95), rgba(0, 5, 10, 0.9));
    border: 1px solid rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 30px rgba(0, 180, 255, 0.2);
  }
  .theme-skynet .relationships-empty {
    background: linear-gradient(135deg, rgba(0, 20, 40, 0.9), rgba(0, 10, 20, 0.8));
    border: 1px solid rgba(0, 180, 255, 0.3);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.15);
  }
  .theme-skynet .relationships-empty-title {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.4);
  }
  .theme-skynet .relationships-empty-text {
    color: rgba(0, 180, 255, 0.6);
  }
  .theme-skynet .relationships-card {
    background: linear-gradient(135deg, rgba(0, 40, 80, 0.6), rgba(0, 20, 40, 0.4));
    border: 1px solid rgba(0, 180, 255, 0.3);
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.1);
    transition: all 0.3s ease;
  }
  .theme-skynet .relationships-card:hover {
    border-color: rgba(0, 180, 255, 0.5);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .relationships-card-title {
    color: #00b4ff;
    text-shadow: 0 0 8px rgba(0, 180, 255, 0.4);
  }
  .theme-skynet .relationships-card-desc {
    color: rgba(0, 180, 255, 0.5);
  }

  /* Relationships View - Cyberpunk Theme */
  .theme-cyberpunk .relationships-header {
    background: linear-gradient(135deg, rgba(50, 0, 80, 0.6), rgba(80, 0, 50, 0.4));
    border: 1px solid rgba(255, 0, 128, 0.4);
    box-shadow: 0 0 30px rgba(255, 0, 128, 0.2);
  }
  .theme-cyberpunk .relationships-title {
    color: #ff0080;
    text-shadow: 0 0 20px rgba(255, 0, 128, 0.5);
    letter-spacing: 0.1em;
  }
  .theme-cyberpunk .relationships-subtitle {
    color: rgba(0, 217, 255, 0.6);
  }
  .theme-cyberpunk .relationships-label {
    color: rgba(255, 0, 128, 0.7);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .theme-cyberpunk .relationships-input {
    background-color: rgba(20, 0, 30, 0.8);
    border: 1px solid rgba(0, 217, 255, 0.4);
    color: #00d9ff;
    box-shadow: inset 0 0 10px rgba(0, 217, 255, 0.1);
  }
  .theme-cyberpunk .relationships-input::placeholder {
    color: rgba(0, 217, 255, 0.4);
  }
  .theme-cyberpunk .relationships-input:focus {
    border-color: #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.4), inset 0 0 10px rgba(0, 217, 255, 0.1);
  }
  .theme-cyberpunk .relationships-select {
    background-color: rgba(20, 0, 30, 0.8);
    border: 1px solid rgba(255, 0, 128, 0.4);
    color: #00d9ff;
  }
  .theme-cyberpunk .relationships-select:focus {
    border-color: #ff0080;
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.4);
  }
  .theme-cyberpunk .relationships-select option {
    background-color: #140020;
    color: #00d9ff;
  }
  .theme-cyberpunk .relationships-trace-btn {
    background: linear-gradient(135deg, rgba(255, 0, 128, 0.8), rgba(176, 0, 255, 0.8));
    color: #ffffff;
    border: 1px solid rgba(0, 217, 255, 0.6);
    box-shadow: 0 0 20px rgba(255, 0, 128, 0.4);
    text-transform: uppercase;
    letter-spacing: 0.1em;
  }
  .theme-cyberpunk .relationships-trace-btn:hover {
    background: linear-gradient(135deg, rgba(255, 0, 128, 0.9), rgba(200, 0, 255, 0.9));
    box-shadow: 0 0 30px rgba(255, 0, 128, 0.6);
  }
  .theme-cyberpunk .relationships-trace-btn:active {
    background: rgba(20, 0, 30, 0.9);
    border-color: #00d9ff;
    box-shadow: inset 0 0 15px rgba(255, 0, 128, 0.3);
  }
  .theme-cyberpunk .relationships-reset-btn {
    background: rgba(50, 0, 80, 0.6);
    color: rgba(0, 217, 255, 0.8);
    border: 1px solid rgba(255, 0, 128, 0.3);
  }
  .theme-cyberpunk .relationships-reset-btn:hover {
    background: rgba(80, 0, 120, 0.8);
    border-color: rgba(255, 0, 128, 0.5);
  }
  .theme-cyberpunk .relationships-examples {
    border-top: 1px solid rgba(255, 0, 128, 0.2);
  }
  .theme-cyberpunk .relationships-example-btn {
    background: rgba(255, 0, 128, 0.1);
    color: #ff0080;
    border: 1px solid rgba(255, 0, 128, 0.3);
  }
  .theme-cyberpunk .relationships-example-btn:hover {
    background: rgba(255, 0, 128, 0.2);
    box-shadow: 0 0 10px rgba(255, 0, 128, 0.3);
  }
  .theme-cyberpunk .relationships-graph-container {
    background: linear-gradient(135deg, rgba(20, 0, 30, 0.95), rgba(10, 0, 15, 0.9));
    border: 1px solid rgba(255, 0, 128, 0.4);
    box-shadow: 0 0 30px rgba(176, 0, 255, 0.2);
  }
  .theme-cyberpunk .relationships-empty {
    background: linear-gradient(135deg, rgba(30, 0, 50, 0.9), rgba(20, 0, 30, 0.8));
    border: 1px solid rgba(255, 0, 128, 0.3);
    box-shadow: 0 0 20px rgba(176, 0, 255, 0.15);
  }
  .theme-cyberpunk .relationships-empty-title {
    color: #ff0080;
    text-shadow: 0 0 10px rgba(255, 0, 128, 0.4);
  }
  .theme-cyberpunk .relationships-empty-text {
    color: rgba(0, 217, 255, 0.6);
  }
  .theme-cyberpunk .relationships-card {
    background: linear-gradient(135deg, rgba(50, 0, 80, 0.6), rgba(30, 0, 50, 0.4));
    border: 1px solid rgba(255, 0, 128, 0.3);
    box-shadow: 0 0 15px rgba(176, 0, 255, 0.1);
    transition: all 0.3s ease;
  }
  .theme-cyberpunk .relationships-card:hover {
    border-color: rgba(255, 0, 128, 0.5);
    box-shadow: 0 0 20px rgba(255, 0, 128, 0.3);
  }
  .theme-cyberpunk .relationships-card-title {
    color: #00d9ff;
    text-shadow: 0 0 8px rgba(0, 217, 255, 0.4);
  }
  .theme-cyberpunk .relationships-card-desc {
    color: rgba(255, 0, 128, 0.5);
  }

  /* Dark mode support for default theme */
  :global(.dark) .theme-default .relationships-header {
    background: linear-gradient(135deg, rgba(79, 70, 229, 0.2), rgba(147, 51, 234, 0.2));
  }
  :global(.dark) .theme-default .relationships-subtitle {
    color: #9ca3af;
  }
  :global(.dark) .theme-default .relationships-label {
    color: #d1d5db;
  }
  :global(.dark) .theme-default .relationships-input {
    background-color: #1f2937;
    border-color: #4b5563;
    color: #f9fafb;
  }
  :global(.dark) .theme-default .relationships-input::placeholder {
    color: #6b7280;
  }
  :global(.dark) .theme-default .relationships-select {
    background-color: #1f2937;
    border-color: #4b5563;
    color: #f9fafb;
  }
  :global(.dark) .theme-default .relationships-examples {
    border-top-color: #374151;
  }

  /* Build Graph Button */
  .theme-default .relationships-build-btn {
    background: linear-gradient(135deg, #059669, #10b981);
    color: #ffffff;
    border: none;
  }
  .theme-default .relationships-build-btn:hover {
    background: linear-gradient(135deg, #10b981, #34d399);
  }
  .theme-skynet .relationships-build-btn {
    background: linear-gradient(135deg, rgba(0, 180, 255, 0.6), rgba(0, 128, 255, 0.6));
    color: #00b4ff;
    border: 1px solid #00b4ff;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.3);
  }
  .theme-skynet .relationships-build-btn:hover {
    background: linear-gradient(135deg, rgba(0, 180, 255, 0.8), rgba(0, 128, 255, 0.8));
  }
  .theme-cyberpunk .relationships-build-btn {
    background: linear-gradient(135deg, rgba(0, 217, 255, 0.6), rgba(176, 0, 255, 0.6));
    color: #00d9ff;
    border: 1px solid #00d9ff;
    box-shadow: 0 0 15px rgba(0, 217, 255, 0.3);
  }
  .theme-cyberpunk .relationships-build-btn:hover {
    background: linear-gradient(135deg, rgba(0, 217, 255, 0.8), rgba(176, 0, 255, 0.8));
  }

  /* Stats Button */
  .theme-default .relationships-stats-btn {
    background: rgba(107, 114, 128, 0.2);
    color: #6b7280;
    border: 1px solid rgba(107, 114, 128, 0.3);
  }
  .theme-default .relationships-stats-btn:hover {
    background: rgba(107, 114, 128, 0.3);
  }
  .theme-skynet .relationships-stats-btn {
    background: rgba(0, 128, 255, 0.2);
    color: #0080ff;
    border: 1px solid rgba(0, 128, 255, 0.4);
  }
  .theme-skynet .relationships-stats-btn:hover {
    background: rgba(0, 128, 255, 0.3);
  }
  .theme-cyberpunk .relationships-stats-btn {
    background: rgba(176, 0, 255, 0.2);
    color: #b000ff;
    border: 1px solid rgba(176, 0, 255, 0.4);
  }
  .theme-cyberpunk .relationships-stats-btn:hover {
    background: rgba(176, 0, 255, 0.3);
  }

  /* Stats Panel */
  .theme-default .relationships-stats-panel {
    background: rgba(107, 114, 128, 0.1);
    border: 1px solid rgba(107, 114, 128, 0.2);
  }
  .theme-default .relationships-stats-number {
    color: #4f46e5;
  }
  .theme-skynet .relationships-stats-panel {
    background: rgba(0, 128, 255, 0.1);
    border: 1px solid rgba(0, 180, 255, 0.3);
    box-shadow: 0 0 10px rgba(0, 180, 255, 0.1);
  }
  .theme-skynet .relationships-stats-number {
    color: #00b4ff;
  }
  .theme-cyberpunk .relationships-stats-panel {
    background: rgba(176, 0, 255, 0.1);
    border: 1px solid rgba(255, 0, 128, 0.3);
    box-shadow: 0 0 10px rgba(176, 0, 255, 0.1);
  }
  .theme-cyberpunk .relationships-stats-number {
    color: #ff0080;
  }
</style>
