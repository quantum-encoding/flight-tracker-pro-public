<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import L from 'leaflet';
  import 'leaflet/dist/leaflet.css';
  import { theme as appTheme } from '$lib/theme';
  import type { Theme } from '$lib/theme';
  import AirportInfoSidebar from '$lib/components/AirportInfoSidebar.svelte';
  import NetworkLocationSidebar from '$lib/components/NetworkLocationSidebar.svelte';

  // Network connection interface for sentinel data
  interface NetworkConnection {
    id: string;
    source: { lat: number; lng: number; name: string };
    target: { lat: number; lng: number; name: string };
    color?: string;
    process?: string;
    isAnomaly?: boolean;
  }

  interface Props {
    flights?: any[];
    darkMode?: boolean;
    theme?: Theme;
    mode?: 'flights' | 'network';
    networkConnections?: NetworkConnection[];
    userId?: string;
  }

  let { flights = [], darkMode = false, theme = 'light', mode = 'flights', networkConnections = [], userId = '' }: Props = $props();

  let mapElement: HTMLDivElement;
  let map: L.Map | null = null;
  let flightLayers: L.LayerGroup | null = null;

  // Sidebar state
  let sidebarAirport = $state<{ code: string; name?: string; lat: number; lng: number; count: number } | null>(null);
  let sidebarNetworkLocation = $state<{ name: string; lat: number; lng: number; count: number; isLocal?: boolean } | null>(null);

  // Selected airport for highlighting connections
  let selectedAirport = $state<string | null>(null);
  let connectionLayers: L.LayerGroup | null = null;

  function closeSidebar() {
    sidebarAirport = null;
    sidebarNetworkLocation = null;
  }

  // Clear airport selection and remove highlighted connections
  function clearAirportSelection() {
    selectedAirport = null;
    if (connectionLayers) {
      connectionLayers.clearLayers();
    }
    // Restore opacity on all flight paths
    if (flightLayers) {
      flightLayers.eachLayer((layer: any) => {
        if (layer.setStyle) {
          layer.setStyle({ opacity: layer.options.originalOpacity || 0.6 });
        }
      });
    }
  }

  // Highlight connections for a selected airport
  function highlightAirportConnections(airportCode: string) {
    if (!map || !flightLayers || !connectionLayers) return;

    // Clear previous highlights
    connectionLayers.clearLayers();

    // Find all flights connected to this airport
    const connectedFlights = flights.filter(
      f => f.departure_airport.toUpperCase() === airportCode.toUpperCase() ||
           f.arrival_airport.toUpperCase() === airportCode.toUpperCase()
    );

    // Get connected airport codes
    const connectedAirports = new Set<string>();
    connectedFlights.forEach(f => {
      connectedAirports.add(f.departure_airport.toUpperCase());
      connectedAirports.add(f.arrival_airport.toUpperCase());
    });

    const isDarkTheme = theme === 'cyberpunk' || theme === 'skynet' || darkMode;

    // Dim all existing routes
    flightLayers.eachLayer((layer: any) => {
      if (layer.setStyle) {
        // Store original opacity if not already stored
        if (layer.options.originalOpacity === undefined) {
          layer.options.originalOpacity = layer.options.opacity;
        }
        layer.setStyle({ opacity: 0.15 });
      }
    });

    // Draw highlighted connections on the connection layer
    connectedFlights.forEach(flight => {
      const fromCoords = getAirportCoords(flight.departure_airport);
      const toCoords = getAirportCoords(flight.arrival_airport);

      if (!fromCoords || !toCoords) return;

      const pathCoords = createCurvedPath(fromCoords, toCoords);

      // Highlight color based on theme
      let highlightColor = '#fbbf24'; // Amber/gold default
      if (theme === 'skynet') {
        highlightColor = '#00ffff'; // Bright cyan
      } else if (theme === 'cyberpunk' || darkMode) {
        highlightColor = '#ff00ff'; // Neon magenta
      }

      const polyline = L.polyline(pathCoords, {
        color: highlightColor,
        weight: 4,
        opacity: 1,
        dashArray: undefined, // Solid line for highlighted routes
        className: isDarkTheme ? 'highlighted-path' : '',
      }).addTo(connectionLayers!);

      // Add popup with flight details
      polyline.bindPopup(`
        <div class="font-sans">
          <div class="font-bold text-lg mb-2">${flight.departure_airport} → ${flight.arrival_airport}</div>
          <div class="text-sm">
            <div><strong>Date:</strong> ${new Date(flight.departure_datetime).toLocaleDateString()}</div>
            ${flight.distance_km ? `<div><strong>Distance:</strong> ${Math.round(flight.distance_km)} km</div>` : ''}
            ${flight.carbon_emissions_kg ? `<div><strong>CO₂:</strong> ${Math.round(flight.carbon_emissions_kg)} kg</div>` : ''}
          </div>
        </div>
      `);
    });

    // Highlight connected airport markers
    connectedAirports.forEach(code => {
      if (code === airportCode.toUpperCase()) return; // Skip the selected airport itself

      const coords = getAirportCoords(code);
      if (!coords) return;

      let markerColor = '#fbbf24'; // Amber/gold
      if (theme === 'skynet') {
        markerColor = '#00ffff';
      } else if (theme === 'cyberpunk' || darkMode) {
        markerColor = '#ff00ff';
      }

      // Add a ring around connected airports
      L.circleMarker(coords, {
        radius: 18,
        fillColor: 'transparent',
        color: markerColor,
        weight: 3,
        opacity: 0.8,
        fillOpacity: 0,
        className: isDarkTheme ? 'highlighted-marker-ring' : '',
      }).addTo(connectionLayers!);
    });

    // Add a prominent ring around the selected airport
    const selectedCoords = getAirportCoords(airportCode);
    if (selectedCoords) {
      let selectedColor = '#ef4444'; // Red for selected
      if (theme === 'skynet') {
        selectedColor = '#00ff88'; // Green for skynet
      } else if (theme === 'cyberpunk' || darkMode) {
        selectedColor = '#00ff88'; // Neon green
      }

      L.circleMarker(selectedCoords, {
        radius: 22,
        fillColor: 'transparent',
        color: selectedColor,
        weight: 4,
        opacity: 1,
        fillOpacity: 0,
        className: isDarkTheme ? 'selected-marker-ring' : '',
      }).addTo(connectionLayers!);
    }
  }

  // Airport coordinates database (expanded from geo.rs)
  const airportCoords: Record<string, [number, number]> = {
    // North America
    'JFK': [40.6413, -73.7781], 'LAX': [33.9416, -118.4085], 'ORD': [41.9742, -87.9073],
    'SFO': [37.6213, -122.3790], 'MIA': [25.7959, -80.2870], 'ATL': [33.6407, -84.4277],
    'DFW': [32.8998, -97.0403], 'DEN': [39.8561, -104.6737], 'SEA': [47.4502, -122.3088],
    'LAS': [36.0840, -115.1537], 'PHX': [33.4484, -112.0740], 'IAH': [29.9902, -95.3368],
    'MCO': [28.4312, -81.3081], 'EWR': [40.6895, -74.1745], 'BOS': [42.3656, -71.0096],
    // Europe
    'LHR': [51.4700, -0.4543], 'CDG': [49.0097, 2.5479], 'FRA': [50.0379, 8.5622],
    'AMS': [52.3105, 4.7683], 'MAD': [40.4983, -3.5676], 'FCO': [41.8003, 12.2389],
    'MUC': [48.3537, 11.7750], 'ZRH': [47.4582, 8.5556], 'VIE': [48.1103, 16.5697],
    'CPH': [55.6180, 12.6508],
    // Asia
    'HND': [35.5494, 139.7798], 'NRT': [35.7653, 140.3861], 'HKG': [22.3080, 113.9185],
    'SIN': [1.3644, 103.9915], 'ICN': [37.4602, 126.4407], 'PEK': [40.0799, 116.6031],
    'PVG': [31.1443, 121.8083], 'BKK': [13.6900, 100.7501], 'DXB': [25.2532, 55.3657],
    // Australia & Oceania
    'SYD': [-33.9461, 151.1772], 'MEL': [-37.6690, 144.8410], 'AKL': [-37.0082, 174.7850],
    // South America
    'GRU': [-23.4356, -46.4731], 'GIG': [-22.8099, -43.2505], 'SCL': [-33.3930, -70.7859],
    'BOG': [4.7016, -74.1469], 'LIM': [-12.0219, -77.1143],
    // Africa
    'CPT': [-33.9715, 18.6021], 'JNB': [-26.1392, 28.2460], 'CAI': [30.1219, 31.4056],
    // Additional airports from the CSV
    'TEB': [40.8501, -74.0608], 'PBI': [26.6832, -80.0956], 'ACK': [41.2531, -70.0602],
    'BWI': [39.1774, -76.6684], 'DCA': [38.8521, -77.0377], 'CMH': [39.9980, -82.8919],
    'DAY': [39.9024, -84.2194], 'TVC': [44.7414, -85.5822],
  };

  // Dynamically looked-up airport coordinates (only for airports in flights)
  let lookedUpCoords: Record<string, [number, number]> = $state({});

  interface CsvAirportData {
    ident: string;
    iata_code: string | null;
    name: string;
    latitude_deg: number | null;
    longitude_deg: number | null;
    municipality: string | null;
    iso_country: string | null;
  }

  // Look up coordinates for specific airport codes from CSV
  async function lookupAirportCoordinates(codes: string[]) {
    if (codes.length === 0) return;

    try {
      // Only fetch coordinates for the specific codes we need
      const airports = await invoke<CsvAirportData[]>('get_csv_airports_by_codes', { codes });
      const coords: Record<string, [number, number]> = { ...lookedUpCoords };

      for (const airport of airports) {
        if (airport.latitude_deg && airport.longitude_deg) {
          if (airport.ident) {
            coords[airport.ident.toUpperCase()] = [airport.latitude_deg, airport.longitude_deg];
          }
          if (airport.iata_code) {
            coords[airport.iata_code.toUpperCase()] = [airport.latitude_deg, airport.longitude_deg];
          }
        }
      }

      lookedUpCoords = coords;
      console.log(`Looked up ${airports.length} airport coordinates for ${codes.length} codes`);
    } catch (err) {
      console.warn('Could not look up airport coordinates:', err);
    }
  }

  // Extract unique airport codes from flights
  function getUniqueAirportCodes(): string[] {
    const codes = new Set<string>();
    for (const flight of flights) {
      if (flight.departure_airport) codes.add(flight.departure_airport.toUpperCase());
      if (flight.arrival_airport) codes.add(flight.arrival_airport.toUpperCase());
    }
    return Array.from(codes);
  }

  function getAirportCoords(code: string): [number, number] | null {
    const upperCode = code.toUpperCase();
    // Check looked-up coords first, then fallback to hardcoded
    return lookedUpCoords[upperCode] || airportCoords[upperCode] || null;
  }

  let tileLayer: L.TileLayer | null = null;
  let boundaryLayer: L.TileLayer | null = null;

  function initializeMap() {
    if (!mapElement) return;

    // Create map centered on the world
    map = L.map(mapElement, {
      center: [20, 0],
      zoom: 2,
      zoomControl: true,
      attributionControl: true,
    });

    // Add tile layer based on mode
    updateTileLayer();

    // Create layer group for flight paths
    flightLayers = L.layerGroup().addTo(map);

    // Create layer group for highlighted connections (on top)
    connectionLayers = L.layerGroup().addTo(map);

    // Add click handler to deselect airport when clicking on map background
    map.on('click', (e: L.LeafletMouseEvent) => {
      // Only deselect if clicking on map itself, not on a marker
      if (selectedAirport && !(e.originalEvent.target as HTMLElement).closest('.leaflet-marker-icon, .leaflet-interactive')) {
        clearAirportSelection();
      }
    });

    // Draw all flights
    updateFlightPaths();
  }

  function updateTileLayer() {
    if (!map) return;

    // Remove existing tile layers
    if (tileLayer) {
      map.removeLayer(tileLayer);
    }
    if (boundaryLayer) {
      map.removeLayer(boundaryLayer);
    }

    // Add appropriate tile layer based on theme/dark mode
    const isDarkTheme = theme === 'cyberpunk' || theme === 'skynet' || darkMode;
    if (isDarkTheme) {
      // CartoDB Dark Matter (no labels) - free, no API key, perfect for neon overlay
      tileLayer = L.tileLayer('https://{s}.basemaps.cartocdn.com/dark_nolabels/{z}/{x}/{y}{r}.png', {
        attribution: '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors © <a href="https://carto.com/attributions">CARTO</a>',
        subdomains: 'abcd',
        maxZoom: 20,
      }).addTo(map);

      // Add themed boundary overlay using Stamen Toner Lines (boundaries only)
      const boundaryClass = theme === 'skynet' ? 'skynet-boundary-layer' : 'neon-boundary-layer';
      boundaryLayer = L.tileLayer('https://tiles.stadiamaps.com/tiles/stamen_toner_lines/{z}/{x}/{y}{r}.png', {
        maxZoom: 20,
        className: boundaryClass,
        opacity: 0.7,
      }).addTo(map);
    } else {
      // OpenStreetMap tiles (light mode)
      tileLayer = L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
        attribution: '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
        maxZoom: 18,
      }).addTo(map);
    }
  }

  function updateFlightPaths() {
    if (!map || !flightLayers) return;

    // Clear existing layers
    flightLayers.clearLayers();

    const airportCounts: Record<string, number> = {};
    const routeCounts: Record<string, number> = {};

    // Count frequency of airports and routes
    flights.forEach(flight => {
      airportCounts[flight.departure_airport] = (airportCounts[flight.departure_airport] || 0) + 1;
      airportCounts[flight.arrival_airport] = (airportCounts[flight.arrival_airport] || 0) + 1;

      const routeKey = [flight.departure_airport, flight.arrival_airport].sort().join('-');
      routeCounts[routeKey] = (routeCounts[routeKey] || 0) + 1;
    });

    // Draw flight routes
    flights.forEach(flight => {
      const fromCoords = getAirportCoords(flight.departure_airport);
      const toCoords = getAirportCoords(flight.arrival_airport);

      if (!fromCoords || !toCoords) return;

      const routeKey = [flight.departure_airport, flight.arrival_airport].sort().join('-');
      const frequency = routeCounts[routeKey];

      // Create curved path (great circle approximation)
      const pathCoords = createCurvedPath(fromCoords, toCoords);

      // Color based on CO2 emissions with theme-specific variants
      let pathColor = '#3b82f6'; // Default blue

      if (theme === 'skynet') {
        // Skynet theme: All blues
        pathColor = '#0080ff'; // Electric blue default
        if (flight.carbon_emissions_kg) {
          if (flight.carbon_emissions_kg > 10000) {
            pathColor = '#6600ff'; // Deep purple-blue for high emissions
          } else if (flight.carbon_emissions_kg > 5000) {
            pathColor = '#0040ff'; // Deep electric blue for medium
          } else {
            pathColor = '#00b4ff'; // Light cyan-blue for low
          }
        }
      } else if (theme === 'cyberpunk' || darkMode) {
        // Cyberpunk theme: Neon colors
        pathColor = '#00d9ff'; // Cyan neon
        if (flight.carbon_emissions_kg) {
          if (flight.carbon_emissions_kg > 10000) {
            pathColor = '#ff0080'; // Neon pink for high emissions
          } else if (flight.carbon_emissions_kg > 5000) {
            pathColor = '#ffaa00'; // Neon orange for medium
          } else {
            pathColor = '#00ff88'; // Neon green for low
          }
        }
      } else if (theme === 'light') {
        // Light theme: Standard colors
        pathColor = '#3b82f6'; // Blue
        if (flight.carbon_emissions_kg) {
          if (flight.carbon_emissions_kg > 10000) {
            pathColor = '#ef4444'; // Red for high emissions
          } else if (flight.carbon_emissions_kg > 5000) {
            pathColor = '#f59e0b'; // Orange for medium
          } else {
            pathColor = '#10b981'; // Green for low
          }
        }
      }

      // Line width based on frequency
      const lineWidth = Math.min(1 + frequency * 0.5, 5);

      // Draw flight path with neon glow effect in dark themes
      const isDarkTheme = theme === 'cyberpunk' || theme === 'skynet' || darkMode;
      const polyline = L.polyline(pathCoords, {
        color: pathColor,
        weight: lineWidth,
        opacity: isDarkTheme ? 0.9 : 0.6,
        dashArray: '5, 5',
        className: isDarkTheme ? (theme === 'skynet' ? 'skynet-path' : 'neon-path') : '',
      }).addTo(flightLayers!);

      // Add popup with flight details
      polyline.bindPopup(`
        <div class="font-sans">
          <div class="font-bold text-lg mb-2">${flight.departure_airport} → ${flight.arrival_airport}</div>
          <div class="text-sm">
            <div><strong>Date:</strong> ${new Date(flight.departure_datetime).toLocaleDateString()}</div>
            ${flight.distance_km ? `<div><strong>Distance:</strong> ${Math.round(flight.distance_km)} km</div>` : ''}
            ${flight.carbon_emissions_kg ? `<div><strong>CO₂:</strong> ${Math.round(flight.carbon_emissions_kg)} kg</div>` : ''}
            ${flight.notes ? `<div><strong>Notes:</strong> ${flight.notes}</div>` : ''}
          </div>
        </div>
      `);
    });

    // Draw airport markers
    const uniqueAirports = new Set<string>();
    flights.forEach(flight => {
      uniqueAirports.add(flight.departure_airport);
      uniqueAirports.add(flight.arrival_airport);
    });

    uniqueAirports.forEach(airportCode => {
      const coords = getAirportCoords(airportCode);
      if (!coords) return;

      const count = airportCounts[airportCode] || 0;
      const radius = Math.min(5 + count * 2, 20);

      const isDarkTheme = theme === 'cyberpunk' || theme === 'skynet' || darkMode;

      let markerFillColor = '#6366f1'; // Indigo
      let markerBorderColor = '#ffffff'; // White

      if (theme === 'skynet') {
        markerFillColor = '#0080ff'; // Electric blue
        markerBorderColor = '#00b4ff'; // Cyan blue
      } else if (theme === 'cyberpunk' || darkMode) {
        markerFillColor = '#ff00ff'; // Neon purple
        markerBorderColor = '#00ffff'; // Cyan
      }

      const marker = L.circleMarker(coords, {
        radius,
        fillColor: markerFillColor,
        color: markerBorderColor,
        weight: isDarkTheme ? 3 : 2,
        opacity: 1,
        fillOpacity: isDarkTheme ? 0.9 : 0.8,
        className: isDarkTheme ? (theme === 'skynet' ? 'skynet-marker' : 'neon-marker') : '',
      }).addTo(flightLayers!);

      // Add click handler to highlight connections
      marker.on('click', (e: L.LeafletMouseEvent) => {
        L.DomEvent.stopPropagation(e);
        if (selectedAirport === airportCode) {
          // Clicking the same airport again deselects it
          clearAirportSelection();
        } else {
          selectedAirport = airportCode;
          highlightAirportConnections(airportCode);
        }
      });

      // Create popup with click handler for details button
      const popupContent = document.createElement('div');
      popupContent.className = 'font-sans';
      popupContent.innerHTML = `
        <div class="font-bold text-lg">${airportCode}</div>
        <div class="text-sm">${count} flight${count > 1 ? 's' : ''}</div>
        <div class="text-xs text-gray-500 mt-1">Click to show connections</div>
        <button class="details-btn text-xs text-blue-500 mt-2 cursor-pointer hover:text-blue-700 underline">View Details →</button>
      `;

      const detailsBtn = popupContent.querySelector('.details-btn');
      if (detailsBtn) {
        detailsBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          sidebarAirport = {
            code: airportCode,
            lat: coords[0],
            lng: coords[1],
            count,
          };
          marker.closePopup();
        });
      }

      marker.bindPopup(popupContent);
    });

    // Fit map to show all routes
    if (uniqueAirports.size > 0) {
      const allCoords: [number, number][] = [];
      uniqueAirports.forEach(airportCode => {
        const coords = getAirportCoords(airportCode);
        if (coords) allCoords.push(coords);
      });

      if (allCoords.length > 0) {
        const bounds = L.latLngBounds(allCoords);
        map!.fitBounds(bounds, { padding: [50, 50] });
      }
    }
  }

  // Create curved path between two points (great circle approximation)
  function createCurvedPath(from: [number, number], to: [number, number]): [number, number][] {
    const points: [number, number][] = [];
    const numPoints = 50;

    for (let i = 0; i <= numPoints; i++) {
      const t = i / numPoints;

      // Linear interpolation for lat/lng
      const lat = from[0] + (to[0] - from[0]) * t;
      const lng = from[1] + (to[1] - from[1]) * t;

      // Add curve height based on distance
      const distance = Math.sqrt(
        Math.pow(to[0] - from[0], 2) + Math.pow(to[1] - from[1], 2)
      );
      const curveHeight = distance * 0.1 * Math.sin(t * Math.PI);

      points.push([lat + curveHeight, lng]);
    }

    return points;
  }

  // Draw network connections from sentinel data
  function updateNetworkPaths() {
    if (!map || !flightLayers) return;

    // Clear existing layers
    flightLayers.clearLayers();

    const locationCounts: Record<string, { count: number; name: string; coords: [number, number] }> = {};

    // Count connections per location
    networkConnections.forEach(conn => {
      const targetKey = `${conn.target.lat.toFixed(2)}_${conn.target.lng.toFixed(2)}`;
      if (!locationCounts[targetKey]) {
        locationCounts[targetKey] = {
          count: 0,
          name: conn.target.name,
          coords: [conn.target.lat, conn.target.lng]
        };
      }
      locationCounts[targetKey].count++;
    });

    // Draw network connections
    networkConnections.forEach(conn => {
      const fromCoords: [number, number] = [conn.source.lat, conn.source.lng];
      const toCoords: [number, number] = [conn.target.lat, conn.target.lng];

      // Create curved path
      const pathCoords = createCurvedPath(fromCoords, toCoords);

      // Determine color based on connection properties
      let pathColor = conn.color || '#00b4ff'; // Default cyan
      if (conn.isAnomaly) {
        pathColor = '#ff0040'; // Red for anomalies
      }

      // Apply theme-specific styling
      const isDarkTheme = theme === 'cyberpunk' || theme === 'skynet' || darkMode;

      // Adjust colors for themes if no specific color provided
      if (!conn.color && !conn.isAnomaly) {
        if (theme === 'skynet') {
          pathColor = '#00b4ff'; // Electric cyan
        } else if (theme === 'cyberpunk' || darkMode) {
          pathColor = '#00ffcc'; // Neon teal
        } else {
          pathColor = '#6366f1'; // Indigo for light mode
        }
      }

      const polyline = L.polyline(pathCoords, {
        color: pathColor,
        weight: 2,
        opacity: isDarkTheme ? 0.85 : 0.7,
        dashArray: conn.isAnomaly ? '10, 5' : '5, 5',
        className: isDarkTheme ? (theme === 'skynet' ? 'skynet-path' : 'neon-path') : '',
      }).addTo(flightLayers!);

      // Add popup with connection details
      polyline.bindPopup(`
        <div class="font-sans">
          <div class="font-bold text-lg mb-2">${conn.source.name} → ${conn.target.name}</div>
          <div class="text-sm">
            ${conn.process ? `<div><strong>Process:</strong> ${conn.process}</div>` : ''}
            ${conn.isAnomaly ? `<div class="text-red-500 font-bold">⚠️ ANOMALY DETECTED</div>` : ''}
          </div>
        </div>
      `);
    });

    // Draw location markers
    const isDarkTheme = theme === 'cyberpunk' || theme === 'skynet' || darkMode;

    // Draw source (local machine) marker
    if (networkConnections.length > 0) {
      const source = networkConnections[0].source;
      let sourceFillColor = '#00ff88'; // Green for local
      let sourceBorderColor = '#ffffff';

      if (theme === 'skynet') {
        sourceFillColor = '#00ff88';
        sourceBorderColor = '#00b4ff';
      } else if (theme === 'cyberpunk' || darkMode) {
        sourceFillColor = '#00ff88';
        sourceBorderColor = '#00ffff';
      }

      const sourceMarker = L.circleMarker([source.lat, source.lng], {
        radius: 12,
        fillColor: sourceFillColor,
        color: sourceBorderColor,
        weight: 3,
        opacity: 1,
        fillOpacity: 0.9,
        className: isDarkTheme ? (theme === 'skynet' ? 'skynet-marker' : 'neon-marker') : '',
      }).addTo(flightLayers!);

      // Create popup with click handler for details button
      const sourcePopupContent = document.createElement('div');
      sourcePopupContent.className = 'font-sans';
      sourcePopupContent.innerHTML = `
        <div class="font-bold text-lg">📍 ${source.name}</div>
        <div class="text-sm text-green-600">Local Machine</div>
        <div class="text-sm">${networkConnections.length} outbound connections</div>
        <button class="details-btn text-xs text-blue-500 mt-2 cursor-pointer hover:text-blue-700 underline">View Details →</button>
      `;

      const sourceDetailsBtn = sourcePopupContent.querySelector('.details-btn');
      if (sourceDetailsBtn) {
        sourceDetailsBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          sidebarNetworkLocation = {
            name: source.name,
            lat: source.lat,
            lng: source.lng,
            count: networkConnections.length,
            isLocal: true,
          };
          sourceMarker.closePopup();
        });
      }

      sourceMarker.bindPopup(sourcePopupContent);
    }

    // Draw target location markers
    Object.entries(locationCounts).forEach(([key, data]) => {
      const hasAnomaly = networkConnections.some(
        conn => `${conn.target.lat.toFixed(2)}_${conn.target.lng.toFixed(2)}` === key && conn.isAnomaly
      );

      const radius = Math.min(6 + data.count * 1.5, 15);

      let markerFillColor = hasAnomaly ? '#ff0040' : '#6366f1';
      let markerBorderColor = '#ffffff';

      if (theme === 'skynet') {
        markerFillColor = hasAnomaly ? '#ff0040' : '#0080ff';
        markerBorderColor = '#00b4ff';
      } else if (theme === 'cyberpunk' || darkMode) {
        markerFillColor = hasAnomaly ? '#ff0040' : '#ff00ff';
        markerBorderColor = '#00ffff';
      }

      const marker = L.circleMarker(data.coords, {
        radius,
        fillColor: markerFillColor,
        color: markerBorderColor,
        weight: isDarkTheme ? 3 : 2,
        opacity: 1,
        fillOpacity: isDarkTheme ? 0.9 : 0.8,
        className: isDarkTheme ? (theme === 'skynet' ? 'skynet-marker' : 'neon-marker') : '',
      }).addTo(flightLayers!);

      // Create popup with click handler for details button
      const targetPopupContent = document.createElement('div');
      targetPopupContent.className = 'font-sans';
      targetPopupContent.innerHTML = `
        <div class="font-bold text-lg">${data.name}</div>
        <div class="text-sm">${data.count} connection${data.count > 1 ? 's' : ''}</div>
        ${hasAnomaly ? `<div class="text-red-500 font-bold text-sm">⚠️ Contains anomalies</div>` : ''}
        <button class="details-btn text-xs text-blue-500 mt-2 cursor-pointer hover:text-blue-700 underline">View Details →</button>
      `;

      const targetDetailsBtn = targetPopupContent.querySelector('.details-btn');
      if (targetDetailsBtn) {
        targetDetailsBtn.addEventListener('click', (e) => {
          e.stopPropagation();
          sidebarNetworkLocation = {
            name: data.name,
            lat: data.coords[0],
            lng: data.coords[1],
            count: data.count,
            isLocal: false,
          };
          marker.closePopup();
        });
      }

      marker.bindPopup(targetPopupContent);
    });

    // Fit map to show all connections
    if (networkConnections.length > 0) {
      const allCoords: [number, number][] = [];
      networkConnections.forEach(conn => {
        allCoords.push([conn.source.lat, conn.source.lng]);
        allCoords.push([conn.target.lat, conn.target.lng]);
      });

      if (allCoords.length > 0) {
        const bounds = L.latLngBounds(allCoords);
        map!.fitBounds(bounds, { padding: [50, 50] });
      }
    }
  }

  // Update map based on current mode
  function updateMap() {
    if (!map || !flightLayers) return;

    if (mode === 'network') {
      updateNetworkPaths();
    } else {
      updateFlightPaths();
    }
  }

  // Update map when data changes
  $effect(() => {
    // Track dependencies by accessing their length/values
    const flightCount = flights.length;
    const networkCount = networkConnections.length;
    const currentMode = mode;

    if (map) {
      console.log(`FlightMap update: mode=${currentMode}, flights=${flightCount}, network=${networkCount}`);
      updateMap();
    }
  });

  // Update tile layer and colors when theme or dark mode changes
  $effect(() => {
    // Access theme and darkMode to track them as dependencies
    const currentTheme = theme;
    const isDark = darkMode;
    if (map) {
      updateTileLayer();
      updateMap();
    }
  });

  onMount(() => {
    // Look up coordinates only for airports that appear in flights
    const codes = getUniqueAirportCodes();
    lookupAirportCoordinates(codes).then(() => {
      initializeMap();
    });

    return () => {
      if (map) {
        map.remove();
        map = null;
      }
    };
  });
</script>

<div class="relative w-full h-full" style="z-index: 1;">
  <div bind:this={mapElement} class="w-full h-full rounded-lg shadow-lg"></div>
</div>

<!-- Sidebars rendered outside map container for proper z-index -->
<!-- Airport Info Sidebar (flights mode only) -->
{#if mode === 'flights'}
  <AirportInfoSidebar
    airport={sidebarAirport}
    {userId}
    onClose={closeSidebar}
  />
{/if}

<!-- Network Location Sidebar (network mode only) -->
{#if mode === 'network'}
  <NetworkLocationSidebar
    location={sidebarNetworkLocation}
    connections={networkConnections}
    onClose={closeSidebar}
  />
{/if}

<style>
  :global(.leaflet-container) {
    font-family: system-ui, -apple-system, sans-serif;
  }

  :global(.leaflet-popup-content-wrapper) {
    border-radius: 8px;
  }

  :global(.leaflet-popup-content) {
    margin: 12px;
  }

  /* Neon glow effects for dark mode - toned down for cross-platform consistency */
  :global(.neon-path) {
    filter: drop-shadow(0 0 3px currentColor);
  }

  :global(.neon-marker) {
    filter: drop-shadow(0 0 4px currentColor);
  }

  /* Neon green boundary overlay for land masses (Cyberpunk) */
  :global(.neon-boundary-layer) {
    filter:
      hue-rotate(90deg)
      saturate(1.5)
      brightness(1.1)
      drop-shadow(0 0 2px rgba(0, 255, 136, 0.5));
    opacity: 0.8;
  }

  /* Skynet blue boundary overlay for land masses */
  :global(.skynet-boundary-layer) {
    filter:
      hue-rotate(180deg)
      saturate(1.5)
      brightness(1.1)
      drop-shadow(0 0 2px rgba(0, 128, 255, 0.5));
    opacity: 0.8;
  }

  /* Skynet-specific path and marker effects - clean, no glow */
  :global(.skynet-path) {
    /* No glow filter - clean solid lines */
  }

  :global(.skynet-marker) {
    /* No glow filter - clean markers */
  }

  /* Pulse animation for markers - subtle effect (neon only) */
  @keyframes pulse-neon {
    0%, 100% {
      filter: drop-shadow(0 0 4px currentColor);
    }
    50% {
      filter: drop-shadow(0 0 6px currentColor);
    }
  }

  :global(.neon-marker) {
    animation: pulse-neon 2s ease-in-out infinite;
  }

  /* Highlighted connection styles */
  :global(.highlighted-path) {
    filter: drop-shadow(0 0 6px currentColor);
  }

  :global(.highlighted-marker-ring) {
    filter: drop-shadow(0 0 4px currentColor);
  }

  :global(.selected-marker-ring) {
    filter: drop-shadow(0 0 6px currentColor);
  }

  /* Pulse animation for selected airport ring */
  @keyframes pulse-selected {
    0%, 100% {
      opacity: 1;
    }
    50% {
      opacity: 0.6;
    }
  }

  :global(.selected-marker-ring) {
    animation: pulse-selected 1.5s ease-in-out infinite;
  }
</style>
