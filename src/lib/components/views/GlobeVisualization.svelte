<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { theme } from '$lib/theme';
  import AirportInfoSidebar from '$lib/components/AirportInfoSidebar.svelte';
  import NetworkLocationSidebar from '$lib/components/NetworkLocationSidebar.svelte';

  // CSV airport interface (from airports.csv)
  interface CsvAirportData {
    ident: string;
    iata_code: string | null;
    name: string;
    latitude_deg: number | null;
    longitude_deg: number | null;
    municipality: string | null;
    iso_country: string | null;
  }

  // Dynamically looked-up airport coordinates (only for airports in flights)
  let lookedUpCoords: Record<string, { lat: number; lng: number }> = $state({});

  // Look up coordinates for specific airport codes from CSV
  async function lookupAirportCoordinates(codes: string[]) {
    if (codes.length === 0) return;

    try {
      const airports = await invoke<CsvAirportData[]>('get_csv_airports_by_codes', { codes });
      const coords: Record<string, { lat: number; lng: number }> = { ...lookedUpCoords };
      for (const airport of airports) {
        if (airport.latitude_deg != null && airport.longitude_deg != null) {
          if (airport.ident) {
            coords[airport.ident.toUpperCase()] = { lat: airport.latitude_deg, lng: airport.longitude_deg };
          }
          if (airport.iata_code) {
            coords[airport.iata_code.toUpperCase()] = { lat: airport.latitude_deg, lng: airport.longitude_deg };
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

  // Get airport coordinates - looked-up coords first, then fallback to hardcoded
  function getAirportCoords(code: string): { lat: number; lng: number } | null {
    const upperCode = code.toUpperCase();
    return lookedUpCoords[upperCode] || airportCoords[upperCode] || null;
  }

  // Props
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
    userId?: string;
    embedded?: boolean;  // When true, hides controls and adjusts for embedded display
    networkConnections?: NetworkConnection[];  // For network sentinel view
    mode?: 'flights' | 'network';  // Visualization mode
  }
  let { flights = [], userId = '', embedded = false, networkConnections = [], mode = 'flights' }: Props = $props();

  // Sidebar state
  let sidebarAirport = $state<{ code: string; name?: string; lat: number; lng: number; count: number } | null>(null);
  let sidebarNetworkLocation = $state<{ name: string; lat: number; lng: number; count: number; isLocal?: boolean } | null>(null);

  // Types
  interface FlightArc {
    id: string;
    from: { lat: number; lng: number; name: string };
    to: { lat: number; lng: number; name: string };
    color: string;
    count: number;
  }

  interface GlobePoint {
    lat: number;
    lng: number;
    name: string;
    count: number;
    type: 'airport' | 'connection';
  }

  // State
  let canvas: HTMLCanvasElement;
  let container: HTMLDivElement;
  let animationId: number | null = null;
  let rotation = $state({ x: 0, y: 0 });
  let autoRotate = $state(true);
  let zoom = $state(1);
  let isDragging = $state(false);
  let lastMouse = { x: 0, y: 0 };
  let selectedPoint = $state<GlobePoint | null>(null);
  let hoveredPoint = $state<GlobePoint | null>(null);
  let flightArcs = $state<FlightArc[]>([]);
  let points = $state<GlobePoint[]>([]);
  let loading = $state(true);
  let showArcs = $state(true);
  let showPoints = $state(true);
  let viewMode = $state<'routes' | 'heatmap' | 'connections'>('routes');

  // Selected airport for highlighting connections
  let selectedAirportForConnections = $state<string | null>(null);
  let connectedAirportCodes = $state<Set<string>>(new Set());

  // Compute connected airports when one is selected
  function selectAirportForConnections(airportCode: string | null) {
    if (airportCode === null || airportCode === selectedAirportForConnections) {
      // Deselect
      selectedAirportForConnections = null;
      connectedAirportCodes = new Set();
    } else {
      selectedAirportForConnections = airportCode;
      // Find all airports connected to this one via flight arcs
      const connected = new Set<string>();
      connected.add(airportCode);
      for (const arc of flightArcs) {
        if (arc.from.name.toUpperCase() === airportCode.toUpperCase()) {
          connected.add(arc.to.name.toUpperCase());
        } else if (arc.to.name.toUpperCase() === airportCode.toUpperCase()) {
          connected.add(arc.from.name.toUpperCase());
        }
      }
      connectedAirportCodes = connected;
    }
  }

  // Check if an arc is connected to the selected airport
  function isArcConnected(arc: FlightArc): boolean {
    if (!selectedAirportForConnections) return true;
    const fromCode = arc.from.name.toUpperCase();
    const toCode = arc.to.name.toUpperCase();
    const selectedCode = selectedAirportForConnections.toUpperCase();
    return fromCode === selectedCode || toCode === selectedCode;
  }

  // Airport coordinate database (commonly used airports)
  const airportCoords: Record<string, { lat: number; lng: number }> = {
    JFK: { lat: 40.6413, lng: -73.7781 },
    LAX: { lat: 33.9416, lng: -118.4085 },
    LHR: { lat: 51.4700, lng: -0.4543 },
    CDG: { lat: 49.0097, lng: 2.5479 },
    DXB: { lat: 25.2532, lng: 55.3657 },
    SIN: { lat: 1.3644, lng: 103.9915 },
    HKG: { lat: 22.3080, lng: 113.9185 },
    NRT: { lat: 35.7647, lng: 140.3864 },
    SYD: { lat: -33.9399, lng: 151.1753 },
    ORD: { lat: 41.9742, lng: -87.9073 },
    MIA: { lat: 25.7959, lng: -80.2870 },
    FRA: { lat: 50.0379, lng: 8.5622 },
    AMS: { lat: 52.3105, lng: 4.7683 },
    SFO: { lat: 37.6213, lng: -122.3790 },
    ATL: { lat: 33.6407, lng: -84.4277 },
    BOS: { lat: 42.3656, lng: -71.0096 },
    SEA: { lat: 47.4502, lng: -122.3088 },
    DEN: { lat: 39.8561, lng: -104.6737 },
    LAS: { lat: 36.0840, lng: -115.1537 },
    PHX: { lat: 33.4373, lng: -112.0078 },
    DFW: { lat: 32.8998, lng: -97.0403 },
    IAH: { lat: 29.9902, lng: -95.3368 },
    EWR: { lat: 40.6895, lng: -74.1745 },
    MSP: { lat: 44.8820, lng: -93.2218 },
    DTW: { lat: 42.2162, lng: -83.3554 },
    PHL: { lat: 39.8729, lng: -75.2437 },
    CLT: { lat: 35.2140, lng: -80.9431 },
    BWI: { lat: 39.1774, lng: -76.6684 },
    SAN: { lat: 32.7338, lng: -117.1933 },
    TPA: { lat: 27.9756, lng: -82.5333 },
    PDX: { lat: 45.5898, lng: -122.5951 },
    STL: { lat: 38.7487, lng: -90.3700 },
    BNA: { lat: 36.1263, lng: -86.6774 },
    AUS: { lat: 30.1975, lng: -97.6664 },
    SAT: { lat: 29.5337, lng: -98.4698 },
    MCI: { lat: 39.2976, lng: -94.7139 },
    IND: { lat: 39.7173, lng: -86.2944 },
    CLE: { lat: 41.4058, lng: -81.8539 },
    CMH: { lat: 39.9999, lng: -82.8919 },
    PIT: { lat: 40.4915, lng: -80.2329 },
    RDU: { lat: 35.8776, lng: -78.7875 },
    MKE: { lat: 42.9472, lng: -87.8966 },
    // International
    PEK: { lat: 40.0799, lng: 116.6031 },
    PVG: { lat: 31.1443, lng: 121.8083 },
    ICN: { lat: 37.4602, lng: 126.4407 },
    BKK: { lat: 13.6900, lng: 100.7501 },
    KUL: { lat: 2.7456, lng: 101.7099 },
    DEL: { lat: 28.5562, lng: 77.1000 },
    BOM: { lat: 19.0896, lng: 72.8656 },
    IST: { lat: 41.2753, lng: 28.7519 },
    FCO: { lat: 41.8003, lng: 12.2389 },
    MAD: { lat: 40.4983, lng: -3.5676 },
    BCN: { lat: 41.2974, lng: 2.0833 },
    MUC: { lat: 48.3537, lng: 11.7750 },
    ZRH: { lat: 47.4647, lng: 8.5492 },
    VIE: { lat: 48.1103, lng: 16.5697 },
    CPH: { lat: 55.6180, lng: 12.6560 },
    ARN: { lat: 59.6498, lng: 17.9238 },
    OSL: { lat: 60.1939, lng: 11.1004 },
    HEL: { lat: 60.3172, lng: 24.9633 },
    DUB: { lat: 53.4264, lng: -6.2499 },
    LIS: { lat: 38.7742, lng: -9.1342 },
    GRU: { lat: -23.4356, lng: -46.4731 },
    EZE: { lat: -34.8222, lng: -58.5358 },
    SCL: { lat: -33.3930, lng: -70.7858 },
    MEX: { lat: 19.4361, lng: -99.0719 },
    CUN: { lat: 21.0365, lng: -86.8771 },
    YYZ: { lat: 43.6777, lng: -79.6248 },
    YVR: { lat: 49.1967, lng: -123.1815 },
    JNB: { lat: -26.1367, lng: 28.2411 },
    CPT: { lat: -33.9715, lng: 18.6021 },
    CAI: { lat: 30.1219, lng: 31.4056 },
    DPS: { lat: -8.7482, lng: 115.1672 },
    MLE: { lat: 4.1918, lng: 73.5291 },
    // Smaller/Regional US
    SNA: { lat: 33.6757, lng: -117.8678 },
    OAK: { lat: 37.7213, lng: -122.2208 },
    SJC: { lat: 37.3639, lng: -121.9289 },
    SMF: { lat: 38.6954, lng: -121.5908 },
    SAF: { lat: 35.6170, lng: -106.0893 },
    ABQ: { lat: 35.0402, lng: -106.6090 },
    TEB: { lat: 40.8501, lng: -74.0608 },
    VNY: { lat: 34.2098, lng: -118.4895 },
  };

  // Process flights into arcs
  function processFlights() {
    const arcMap = new Map<string, FlightArc>();
    const pointMap = new Map<string, GlobePoint>();

    for (const flight of flights) {
      const fromCode = flight.departure_airport;
      const toCode = flight.arrival_airport;

      if (!fromCode || !toCode) continue;

      // Get coordinates - use merged database + hardcoded lookup
      const fromCoords = getAirportCoords(fromCode);
      const toCoords = getAirportCoords(toCode);

      // Skip if we don't have valid coordinates
      if (!fromCoords || !toCoords) continue;

      // Create arc key
      const arcKey = `${fromCode}-${toCode}`;

      if (arcMap.has(arcKey)) {
        const arc = arcMap.get(arcKey)!;
        arc.count++;
      } else {
        // Color based on distance or theme
        const distance = Math.sqrt(
          Math.pow(toCoords.lat - fromCoords.lat, 2) +
          Math.pow(toCoords.lng - fromCoords.lng, 2)
        );

        let color = '#00b4ff'; // Default cyan
        if (distance > 100) color = '#ff0080'; // Long haul - pink
        else if (distance > 50) color = '#b000ff'; // Medium - purple
        else color = '#00ff88'; // Short - green

        arcMap.set(arcKey, {
          id: arcKey,
          from: { ...fromCoords, name: fromCode },
          to: { ...toCoords, name: toCode },
          color,
          count: 1,
        });
      }

      // Add points
      if (!pointMap.has(fromCode)) {
        pointMap.set(fromCode, {
          ...fromCoords,
          name: fromCode,
          count: 1,
          type: 'airport',
        });
      } else {
        pointMap.get(fromCode)!.count++;
      }

      if (!pointMap.has(toCode)) {
        pointMap.set(toCode, {
          ...toCoords,
          name: toCode,
          count: 1,
          type: 'airport',
        });
      } else {
        pointMap.get(toCode)!.count++;
      }
    }

    flightArcs = Array.from(arcMap.values());
    points = Array.from(pointMap.values());
  }

  // Process network connections into arcs
  function processNetworkConnections() {
    const arcMap = new Map<string, FlightArc>();
    const pointMap = new Map<string, GlobePoint>();

    for (const conn of networkConnections) {
      // Create arc key based on connection ID
      const arcKey = conn.id;

      // Color based on anomaly status or provided color
      let color = conn.color || '#00b4ff';
      if (conn.isAnomaly) {
        color = '#ff0040'; // Red for anomalies
      }

      arcMap.set(arcKey, {
        id: arcKey,
        from: { lat: conn.source.lat, lng: conn.source.lng, name: conn.source.name },
        to: { lat: conn.target.lat, lng: conn.target.lng, name: conn.target.name },
        color,
        count: 1,
      });

      // Add source point (local machine)
      const sourceKey = `${conn.source.lat.toFixed(2)}_${conn.source.lng.toFixed(2)}`;
      if (!pointMap.has(sourceKey)) {
        pointMap.set(sourceKey, {
          lat: conn.source.lat,
          lng: conn.source.lng,
          name: conn.source.name,
          count: 1,
          type: 'connection',
        });
      } else {
        pointMap.get(sourceKey)!.count++;
      }

      // Add target point (remote server)
      const targetKey = `${conn.target.lat.toFixed(2)}_${conn.target.lng.toFixed(2)}`;
      if (!pointMap.has(targetKey)) {
        pointMap.set(targetKey, {
          lat: conn.target.lat,
          lng: conn.target.lng,
          name: conn.target.name,
          count: 1,
          type: 'connection',
        });
      } else {
        pointMap.get(targetKey)!.count++;
      }
    }

    flightArcs = Array.from(arcMap.values());
    points = Array.from(pointMap.values());
  }

  // Convert lat/lng to 3D coordinates on sphere
  function latLngTo3D(lat: number, lng: number, radius: number): { x: number; y: number; z: number } {
    const phi = (90 - lat) * (Math.PI / 180);
    const theta = (lng + 180) * (Math.PI / 180);

    return {
      x: -radius * Math.sin(phi) * Math.cos(theta),
      y: radius * Math.cos(phi),
      z: radius * Math.sin(phi) * Math.sin(theta),
    };
  }

  // Project 3D point to 2D screen coordinates
  function project3DTo2D(
    point: { x: number; y: number; z: number },
    width: number,
    height: number,
    rotX: number,
    rotY: number,
    scale: number
  ): { x: number; y: number; visible: boolean } {
    // Apply rotation
    let x = point.x;
    let y = point.y;
    let z = point.z;

    // Rotate around Y axis
    const cosY = Math.cos(rotY);
    const sinY = Math.sin(rotY);
    const x1 = x * cosY - z * sinY;
    const z1 = x * sinY + z * cosY;

    // Rotate around X axis
    const cosX = Math.cos(rotX);
    const sinX = Math.sin(rotX);
    const y1 = y * cosX - z1 * sinX;
    const z2 = y * sinX + z1 * cosX;

    // Simple perspective projection
    const fov = 500;
    const projScale = fov / (fov + z2);

    return {
      x: width / 2 + x1 * projScale * scale,
      y: height / 2 - y1 * projScale * scale, // Negate Y to flip (screen Y is inverted)
      visible: z2 < 100, // Point is visible if in front of camera
    };
  }

  // Draw the globe
  function draw() {
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const width = canvas.width;
    const height = canvas.height;
    const radius = Math.min(width, height) * 0.35;
    const scale = zoom * radius;

    // Clear canvas
    ctx.clearRect(0, 0, width, height);

    // Get theme colors
    const isSkynet = $theme === 'skynet';
    const isCyberpunk = $theme === 'cyberpunk';

    const bgGradient = ctx.createRadialGradient(width / 2, height / 2, 0, width / 2, height / 2, Math.max(width, height) / 2);
    if (isSkynet) {
      bgGradient.addColorStop(0, '#000510');
      bgGradient.addColorStop(1, '#000000');
    } else if (isCyberpunk) {
      bgGradient.addColorStop(0, '#0a0015');
      bgGradient.addColorStop(1, '#000000');
    } else {
      bgGradient.addColorStop(0, '#0f172a');
      bgGradient.addColorStop(1, '#020617');
    }
    ctx.fillStyle = bgGradient;
    ctx.fillRect(0, 0, width, height);

    // Draw star field
    ctx.fillStyle = 'rgba(255, 255, 255, 0.3)';
    for (let i = 0; i < 200; i++) {
      const x = (Math.sin(i * 12345.6789) * 0.5 + 0.5) * width;
      const y = (Math.cos(i * 98765.4321) * 0.5 + 0.5) * height;
      const size = Math.random() * 1.5;
      ctx.beginPath();
      ctx.arc(x, y, size, 0, Math.PI * 2);
      ctx.fill();
    }

    // Draw globe sphere
    const globeGradient = ctx.createRadialGradient(
      width / 2 - scale * 0.3,
      height / 2 - scale * 0.3,
      0,
      width / 2,
      height / 2,
      scale
    );

    if (isSkynet) {
      globeGradient.addColorStop(0, 'rgba(0, 60, 120, 0.8)');
      globeGradient.addColorStop(0.5, 'rgba(0, 30, 60, 0.6)');
      globeGradient.addColorStop(1, 'rgba(0, 15, 30, 0.4)');
    } else if (isCyberpunk) {
      globeGradient.addColorStop(0, 'rgba(50, 0, 80, 0.8)');
      globeGradient.addColorStop(0.5, 'rgba(30, 0, 50, 0.6)');
      globeGradient.addColorStop(1, 'rgba(15, 0, 25, 0.4)');
    } else {
      globeGradient.addColorStop(0, 'rgba(30, 64, 175, 0.6)');
      globeGradient.addColorStop(0.5, 'rgba(30, 58, 138, 0.4)');
      globeGradient.addColorStop(1, 'rgba(30, 41, 59, 0.3)');
    }

    ctx.beginPath();
    ctx.arc(width / 2, height / 2, scale, 0, Math.PI * 2);
    ctx.fillStyle = globeGradient;
    ctx.fill();

    // Draw globe outline glow
    ctx.beginPath();
    ctx.arc(width / 2, height / 2, scale, 0, Math.PI * 2);
    ctx.strokeStyle = isSkynet ? 'rgba(0, 180, 255, 0.5)' : isCyberpunk ? 'rgba(0, 217, 255, 0.5)' : 'rgba(96, 165, 250, 0.5)';
    ctx.lineWidth = 2;
    ctx.stroke();

    // Draw latitude/longitude grid
    ctx.strokeStyle = isSkynet ? 'rgba(0, 180, 255, 0.15)' : isCyberpunk ? 'rgba(255, 0, 128, 0.15)' : 'rgba(148, 163, 184, 0.15)';
    ctx.lineWidth = 0.5;

    // Latitude lines
    for (let lat = -60; lat <= 60; lat += 30) {
      ctx.beginPath();
      let firstVisible = true;
      for (let lng = -180; lng <= 180; lng += 5) {
        const point3D = latLngTo3D(lat, lng, 1);
        const point2D = project3DTo2D(point3D, width, height, rotation.x, rotation.y, scale);

        if (point2D.visible) {
          if (firstVisible) {
            ctx.moveTo(point2D.x, point2D.y);
            firstVisible = false;
          } else {
            ctx.lineTo(point2D.x, point2D.y);
          }
        } else {
          firstVisible = true;
        }
      }
      ctx.stroke();
    }

    // Longitude lines
    for (let lng = -180; lng < 180; lng += 30) {
      ctx.beginPath();
      let firstVisible = true;
      for (let lat = -90; lat <= 90; lat += 5) {
        const point3D = latLngTo3D(lat, lng, 1);
        const point2D = project3DTo2D(point3D, width, height, rotation.x, rotation.y, scale);

        if (point2D.visible) {
          if (firstVisible) {
            ctx.moveTo(point2D.x, point2D.y);
            firstVisible = false;
          } else {
            ctx.lineTo(point2D.x, point2D.y);
          }
        } else {
          firstVisible = true;
        }
      }
      ctx.stroke();
    }

    // Draw flight arcs
    if (showArcs) {
      // First pass: draw dimmed non-connected arcs (when airport is selected)
      if (selectedAirportForConnections) {
        for (const arc of flightArcs) {
          if (isArcConnected(arc)) continue; // Skip connected arcs for now

          const from3D = latLngTo3D(arc.from.lat, arc.from.lng, 1);
          const to3D = latLngTo3D(arc.to.lat, arc.to.lng, 1);

          const fromProj = project3DTo2D(from3D, width, height, rotation.x, rotation.y, scale);
          const toProj = project3DTo2D(to3D, width, height, rotation.x, rotation.y, scale);

          if (!fromProj.visible && !toProj.visible) continue;

          const midLat = (arc.from.lat + arc.to.lat) / 2;
          const midLng = (arc.from.lng + arc.to.lng) / 2;
          const distance = Math.sqrt(
            Math.pow(arc.to.lat - arc.from.lat, 2) +
            Math.pow(arc.to.lng - arc.from.lng, 2)
          );
          const elevation = 1 + distance * 0.005;
          const mid3D = latLngTo3D(midLat, midLng, elevation);
          const midProj = project3DTo2D(mid3D, width, height, rotation.x, rotation.y, scale);

          // Draw dimmed arc
          ctx.beginPath();
          ctx.moveTo(fromProj.x, fromProj.y);
          ctx.quadraticCurveTo(midProj.x, midProj.y, toProj.x, toProj.y);

          ctx.strokeStyle = 'rgba(100, 100, 100, 0.15)';
          ctx.lineWidth = Math.min(arc.count, 3);
          ctx.stroke();
        }
      }

      // Second pass: draw connected or all arcs (on top)
      for (const arc of flightArcs) {
        // Skip non-connected arcs if an airport is selected (they're already drawn dimmed)
        if (selectedAirportForConnections && !isArcConnected(arc)) continue;

        const from3D = latLngTo3D(arc.from.lat, arc.from.lng, 1);
        const to3D = latLngTo3D(arc.to.lat, arc.to.lng, 1);

        const fromProj = project3DTo2D(from3D, width, height, rotation.x, rotation.y, scale);
        const toProj = project3DTo2D(to3D, width, height, rotation.x, rotation.y, scale);

        // Only draw if at least one point is visible
        if (!fromProj.visible && !toProj.visible) continue;

        // Calculate arc control point (elevated above surface)
        const midLat = (arc.from.lat + arc.to.lat) / 2;
        const midLng = (arc.from.lng + arc.to.lng) / 2;
        const distance = Math.sqrt(
          Math.pow(arc.to.lat - arc.from.lat, 2) +
          Math.pow(arc.to.lng - arc.from.lng, 2)
        );
        const elevation = 1 + distance * 0.005; // Elevate based on distance
        const mid3D = latLngTo3D(midLat, midLng, elevation);
        const midProj = project3DTo2D(mid3D, width, height, rotation.x, rotation.y, scale);

        // Draw arc with gradient
        ctx.beginPath();
        ctx.moveTo(fromProj.x, fromProj.y);
        ctx.quadraticCurveTo(midProj.x, midProj.y, toProj.x, toProj.y);

        // Use highlight color if this arc is connected to selected airport
        let arcColor: string;
        if (selectedAirportForConnections && isArcConnected(arc)) {
          // Highlight color based on theme
          arcColor = isSkynet ? '#00ffff' : isCyberpunk ? '#ff00ff' : '#fbbf24';
        } else {
          arcColor = isSkynet ? '#00b4ff' : isCyberpunk ? '#ff0080' : arc.color;
        }

        const gradient = ctx.createLinearGradient(fromProj.x, fromProj.y, toProj.x, toProj.y);
        gradient.addColorStop(0, arcColor + '80');
        gradient.addColorStop(0.5, arcColor);
        gradient.addColorStop(1, arcColor + '80');

        ctx.strokeStyle = gradient;
        ctx.lineWidth = selectedAirportForConnections ? Math.min(arc.count + 2, 7) : Math.min(arc.count, 5);
        ctx.stroke();

        // Draw glow effect (stronger for highlighted arcs)
        ctx.strokeStyle = arcColor + (selectedAirportForConnections ? '50' : '30');
        ctx.lineWidth = (selectedAirportForConnections ? Math.min(arc.count + 2, 7) : Math.min(arc.count, 5)) + 4;
        ctx.stroke();
      }
    }

    // Draw airport points
    if (showPoints) {
      for (const point of points) {
        const point3D = latLngTo3D(point.lat, point.lng, 1.01);
        const point2D = project3DTo2D(point3D, width, height, rotation.x, rotation.y, scale);

        if (!point2D.visible) continue;

        const isHovered = hoveredPoint?.name === point.name;
        const isSelected = selectedPoint?.name === point.name;
        const isConnectedToSelection = connectedAirportCodes.has(point.name.toUpperCase());
        const isTheSelectedAirport = selectedAirportForConnections?.toUpperCase() === point.name.toUpperCase();
        const pointSize = Math.min(3 + point.count * 0.5, 10) * (isHovered || isSelected || isTheSelectedAirport ? 1.5 : 1);

        // Determine if this point should be dimmed
        const shouldDim = selectedAirportForConnections && !isConnectedToSelection;

        // Draw connection ring for connected airports
        if (isConnectedToSelection && !isTheSelectedAirport) {
          const ringColor = isSkynet ? '#00ffff' : isCyberpunk ? '#ff00ff' : '#fbbf24';
          ctx.beginPath();
          ctx.arc(point2D.x, point2D.y, pointSize + 8, 0, Math.PI * 2);
          ctx.strokeStyle = ringColor;
          ctx.lineWidth = 2;
          ctx.stroke();
          // Glow for ring
          ctx.beginPath();
          ctx.arc(point2D.x, point2D.y, pointSize + 8, 0, Math.PI * 2);
          ctx.strokeStyle = ringColor + '40';
          ctx.lineWidth = 6;
          ctx.stroke();
        }

        // Draw selection ring for the selected airport itself
        if (isTheSelectedAirport) {
          const selectedRingColor = isSkynet ? '#00ff88' : isCyberpunk ? '#00ff88' : '#ef4444';
          ctx.beginPath();
          ctx.arc(point2D.x, point2D.y, pointSize + 10, 0, Math.PI * 2);
          ctx.strokeStyle = selectedRingColor;
          ctx.lineWidth = 3;
          ctx.stroke();
          // Glow for selection ring
          ctx.beginPath();
          ctx.arc(point2D.x, point2D.y, pointSize + 10, 0, Math.PI * 2);
          ctx.strokeStyle = selectedRingColor + '50';
          ctx.lineWidth = 8;
          ctx.stroke();
        }

        // Draw point glow
        const glowColor = isSkynet ? '#00b4ff' : isCyberpunk ? '#00d9ff' : '#60a5fa';
        ctx.beginPath();
        ctx.arc(point2D.x, point2D.y, pointSize + 4, 0, Math.PI * 2);
        ctx.fillStyle = shouldDim ? 'rgba(100, 100, 100, 0.1)' : glowColor + '30';
        ctx.fill();

        // Draw point
        ctx.beginPath();
        ctx.arc(point2D.x, point2D.y, pointSize, 0, Math.PI * 2);
        let pointFillColor: string;
        if (shouldDim) {
          pointFillColor = 'rgba(100, 100, 100, 0.3)';
        } else if (isTheSelectedAirport) {
          pointFillColor = isSkynet ? '#00ff88' : isCyberpunk ? '#00ff88' : '#ef4444';
        } else if (isSelected) {
          pointFillColor = '#ff0080';
        } else if (isHovered) {
          pointFillColor = '#00ff88';
        } else {
          pointFillColor = glowColor;
        }
        ctx.fillStyle = pointFillColor;
        ctx.fill();

        // Draw label for selected/hovered point or connected airports
        if (isHovered || isSelected || isTheSelectedAirport || (isConnectedToSelection && selectedAirportForConnections)) {
          ctx.font = 'bold 12px monospace';
          ctx.fillStyle = '#ffffff';
          ctx.textAlign = 'center';
          ctx.fillText(point.name, point2D.x, point2D.y - pointSize - 8);
          ctx.font = '10px monospace';
          ctx.fillStyle = '#94a3b8';
          ctx.fillText(`${point.count} flights`, point2D.x, point2D.y - pointSize - 20);
        }
      }
    }

    // Draw title/stats overlay (skip in embedded mode)
    if (!embedded) {
      ctx.font = 'bold 14px sans-serif';
      ctx.fillStyle = isSkynet ? '#00b4ff' : isCyberpunk ? '#ff0080' : '#60a5fa';
      ctx.textAlign = 'left';
      ctx.fillText('3D GLOBE VISUALIZATION', 20, 30);

      ctx.font = '12px monospace';
      ctx.fillStyle = '#94a3b8';
      ctx.fillText(`Routes: ${flightArcs.length}`, 20, 50);
      ctx.fillText(`Airports: ${points.length}`, 20, 65);
      ctx.fillText(`Flights: ${flights.length}`, 20, 80);
    }
  }

  // Animation loop
  function animate() {
    if (autoRotate && !isDragging) {
      rotation.y += 0.003;
    }
    draw();
    animationId = requestAnimationFrame(animate);
  }

  // Handle mouse events
  function handleMouseDown(e: MouseEvent) {
    isDragging = true;
    lastMouse = { x: e.clientX, y: e.clientY };
    autoRotate = false;
  }

  function handleMouseMove(e: MouseEvent) {
    if (isDragging) {
      const dx = e.clientX - lastMouse.x;
      const dy = e.clientY - lastMouse.y;

      rotation.y += dx * 0.005;
      rotation.x += dy * 0.005;

      // Clamp X rotation
      rotation.x = Math.max(-Math.PI / 2, Math.min(Math.PI / 2, rotation.x));

      lastMouse = { x: e.clientX, y: e.clientY };
    }

    // Check for hover on points
    if (canvas) {
      const rect = canvas.getBoundingClientRect();
      const mouseX = (e.clientX - rect.left) * (canvas.width / rect.width);
      const mouseY = (e.clientY - rect.top) * (canvas.height / rect.height);
      const width = canvas.width;
      const height = canvas.height;
      const scale = zoom * Math.min(width, height) * 0.35;

      let foundHover = false;
      for (const point of points) {
        const point3D = latLngTo3D(point.lat, point.lng, 1.01);
        const point2D = project3DTo2D(point3D, width, height, rotation.x, rotation.y, scale);

        if (point2D.visible) {
          const dist = Math.sqrt(Math.pow(mouseX - point2D.x, 2) + Math.pow(mouseY - point2D.y, 2));
          if (dist < 15) {
            hoveredPoint = point;
            foundHover = true;
            break;
          }
        }
      }
      if (!foundHover) {
        hoveredPoint = null;
      }
    }
  }

  function handleMouseUp() {
    isDragging = false;
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    zoom = Math.max(0.5, Math.min(3, zoom - e.deltaY * 0.001));
  }

  function handleClick(e: MouseEvent) {
    if (hoveredPoint) {
      selectedPoint = selectedPoint?.name === hoveredPoint.name ? null : hoveredPoint;

      // Toggle connection highlighting for flights mode
      if (mode === 'flights') {
        selectAirportForConnections(hoveredPoint.name);
      }

      // Open sidebar based on mode
      if (selectedPoint) {
        if (mode === 'network') {
          // Check if this is the local machine point
          const isLocal = selectedPoint.name === 'Local' ||
            (networkConnections.length > 0 &&
             Math.abs(selectedPoint.lat - networkConnections[0].source.lat) < 0.01 &&
             Math.abs(selectedPoint.lng - networkConnections[0].source.lng) < 0.01);

          sidebarNetworkLocation = {
            name: selectedPoint.name,
            lat: selectedPoint.lat,
            lng: selectedPoint.lng,
            count: selectedPoint.count,
            isLocal,
          };
          sidebarAirport = null;
        } else {
          sidebarAirport = {
            code: selectedPoint.name,
            lat: selectedPoint.lat,
            lng: selectedPoint.lng,
            count: selectedPoint.count,
          };
          sidebarNetworkLocation = null;
        }
      }
    } else {
      // Clicked on empty space - clear connection highlighting
      if (selectedAirportForConnections) {
        selectAirportForConnections(null);
      }
    }
  }

  function closeSidebar() {
    sidebarAirport = null;
    sidebarNetworkLocation = null;
    selectedPoint = null;
    selectAirportForConnections(null);
  }

  // Handle resize
  function handleResize() {
    if (canvas && container) {
      canvas.width = container.clientWidth;
      canvas.height = container.clientHeight;
      draw();
    }
  }

  // Reset view
  function resetView() {
    rotation = { x: 0, y: 0 };
    zoom = 1;
    autoRotate = true;
    selectedPoint = null;
    selectAirportForConnections(null);
  }

  onMount(async () => {
    if (canvas && container) {
      canvas.width = container.clientWidth;
      canvas.height = container.clientHeight;

      // Look up coordinates only for airports that appear in flights
      if (mode === 'flights') {
        const codes = getUniqueAirportCodes();
        await lookupAirportCoordinates(codes);
      }

      // Process data based on mode
      if (mode === 'network' && networkConnections.length > 0) {
        processNetworkConnections();
      } else {
        processFlights();
      }
      loading = false;
      animate();

      window.addEventListener('resize', handleResize);
    }
  });

  onDestroy(() => {
    if (animationId) {
      cancelAnimationFrame(animationId);
    }
    window.removeEventListener('resize', handleResize);
  });

  // Re-process data when it changes
  $effect(() => {
    if (mode === 'network' && networkConnections) {
      processNetworkConnections();
    } else if (flights) {
      processFlights();
    }
  });
</script>

<div
  class="globe-container relative w-full h-full overflow-hidden {$theme === 'skynet' ? 'theme-skynet' : $theme === 'cyberpunk' ? 'theme-cyberpunk' : 'theme-default'}"
  bind:this={container}
>
  <!-- Canvas -->
  <canvas
    bind:this={canvas}
    class="w-full h-full cursor-grab"
    class:cursor-grabbing={isDragging}
    onmousedown={handleMouseDown}
    onmousemove={handleMouseMove}
    onmouseup={handleMouseUp}
    onmouseleave={handleMouseUp}
    onwheel={handleWheel}
    onclick={handleClick}
  ></canvas>

  <!-- Loading Overlay -->
  {#if loading}
    <div class="absolute inset-0 flex items-center justify-center bg-black/80">
      <div class="text-center">
        <div class="globe-spinner w-16 h-16 mx-auto mb-4"></div>
        <p class="globe-loading-text">Initializing Globe...</p>
      </div>
    </div>
  {/if}

  <!-- Controls Panel (hidden in embedded mode) -->
  {#if !embedded}
    <div class="absolute top-4 right-4 globe-controls p-4 rounded-lg space-y-3">
      <h3 class="globe-controls-title text-sm font-bold mb-3">Controls</h3>

      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={autoRotate} class="globe-checkbox" />
        <span class="text-xs">Auto Rotate</span>
      </label>

      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={showArcs} class="globe-checkbox" />
        <span class="text-xs">Show Routes</span>
      </label>

      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={showPoints} class="globe-checkbox" />
        <span class="text-xs">Show Airports</span>
      </label>

      <div class="pt-2 border-t globe-border">
        <label class="block text-xs mb-1">Zoom: {zoom.toFixed(1)}x</label>
        <input
          type="range"
          min="0.5"
          max="3"
          step="0.1"
          bind:value={zoom}
          class="w-full globe-slider"
        />
      </div>

      <button
        onclick={resetView}
        class="globe-reset-btn w-full px-3 py-1.5 text-xs font-bold rounded transition"
      >
        Reset View
      </button>
    </div>
  {/if}

  <!-- Selected Point Info (compact in embedded mode) -->
  {#if selectedPoint && !embedded}
    <div class="absolute bottom-4 left-4 globe-info-panel p-4 rounded-lg max-w-xs">
      <div class="flex items-center justify-between mb-2">
        <h3 class="globe-info-title font-bold">{selectedPoint.name}</h3>
        <button
          onclick={() => selectedPoint = null}
          class="globe-close-btn w-6 h-6 rounded flex items-center justify-center text-xs"
        >
          ×
        </button>
      </div>
      <div class="space-y-1 text-xs">
        <p class="globe-info-label">
          Coordinates: <span class="globe-info-value">{selectedPoint.lat.toFixed(4)}, {selectedPoint.lng.toFixed(4)}</span>
        </p>
        <p class="globe-info-label">
          Total Flights: <span class="globe-info-value">{selectedPoint.count}</span>
        </p>
      </div>
    </div>
  {:else if selectedPoint && embedded}
    <!-- Compact tooltip for embedded mode -->
    <div class="absolute bottom-2 left-2 globe-info-panel px-2 py-1 rounded text-[10px]">
      <span class="font-bold">{selectedPoint.name}</span>
      <span class="opacity-70 ml-2">{selectedPoint.count} flights</span>
    </div>
  {/if}

  <!-- Legend (hidden in embedded mode) -->
  {#if !embedded}
    <div class="absolute bottom-4 right-4 globe-legend p-3 rounded-lg">
      <h4 class="text-[10px] font-bold mb-2 opacity-70">LEGEND</h4>
      <div class="space-y-1 text-[10px]">
        <div class="flex items-center gap-2">
          <div class="w-3 h-0.5 bg-[#00ff88] rounded"></div>
          <span>Short Haul</span>
        </div>
        <div class="flex items-center gap-2">
          <div class="w-3 h-0.5 bg-[#b000ff] rounded"></div>
          <span>Medium Haul</span>
        </div>
        <div class="flex items-center gap-2">
          <div class="w-3 h-0.5 bg-[#ff0080] rounded"></div>
          <span>Long Haul</span>
        </div>
      </div>
    </div>
  {/if}

  <!-- Instructions (hidden in embedded mode) -->
  {#if !embedded}
    <div class="absolute top-24 left-4 globe-instructions p-3 rounded-lg text-[10px] opacity-70">
      <p>Drag to rotate</p>
      <p>Scroll to zoom</p>
      <p>Click {mode === 'network' ? 'locations' : 'airports'} for details</p>
    </div>
  {/if}

  <!-- Airport Info Sidebar (hidden in embedded mode, flights mode only) -->
  {#if !embedded && mode === 'flights'}
    <AirportInfoSidebar
      airport={sidebarAirport}
      {userId}
      onClose={closeSidebar}
    />
  {/if}

  <!-- Network Location Sidebar (hidden in embedded mode, network mode only) -->
  {#if !embedded && mode === 'network'}
    <NetworkLocationSidebar
      location={sidebarNetworkLocation}
      connections={networkConnections}
      onClose={closeSidebar}
    />
  {/if}
</div>

<style>
  /* Theme: Default */
  .theme-default {
    background-color: #020617;
  }

  .theme-default .globe-controls {
    background: rgba(15, 23, 42, 0.9);
    border: 1px solid rgba(148, 163, 184, 0.3);
  }

  .theme-default .globe-controls-title {
    color: #60a5fa;
  }

  .theme-default .globe-border {
    border-color: rgba(148, 163, 184, 0.2);
  }

  .theme-default .globe-reset-btn {
    background-color: #3b82f6;
    color: white;
  }

  .theme-default .globe-reset-btn:hover {
    background-color: #2563eb;
  }

  .theme-default .globe-info-panel {
    background: rgba(15, 23, 42, 0.95);
    border: 1px solid rgba(96, 165, 250, 0.4);
  }

  .theme-default .globe-info-title {
    color: #60a5fa;
  }

  .theme-default .globe-close-btn {
    background: rgba(148, 163, 184, 0.2);
    color: #94a3b8;
  }

  .theme-default .globe-close-btn:hover {
    background: rgba(148, 163, 184, 0.3);
  }

  .theme-default .globe-info-label {
    color: #94a3b8;
  }

  .theme-default .globe-info-value {
    color: #f1f5f9;
  }

  .theme-default .globe-legend,
  .theme-default .globe-instructions {
    background: rgba(15, 23, 42, 0.8);
    color: #94a3b8;
  }

  .theme-default .globe-loading-text {
    color: #60a5fa;
  }

  .theme-default .globe-spinner {
    border: 3px solid rgba(96, 165, 250, 0.2);
    border-top-color: #60a5fa;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  /* Theme: Skynet */
  .theme-skynet {
    background-color: #000000;
  }

  .theme-skynet .globe-controls {
    background: rgba(0, 10, 20, 0.95);
    border: 1px solid rgba(0, 180, 255, 0.4);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .globe-controls-title {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .globe-border {
    border-color: rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .globe-checkbox {
    accent-color: #00b4ff;
  }

  .theme-skynet .globe-slider {
    accent-color: #00b4ff;
  }

  .theme-skynet .globe-reset-btn {
    background: linear-gradient(135deg, #0080ff, #00b4ff);
    color: white;
    box-shadow: 0 0 15px rgba(0, 180, 255, 0.4);
  }

  .theme-skynet .globe-reset-btn:hover {
    box-shadow: 0 0 25px rgba(0, 180, 255, 0.6);
  }

  .theme-skynet .globe-info-panel {
    background: rgba(0, 10, 20, 0.95);
    border: 1px solid rgba(0, 180, 255, 0.5);
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .globe-info-title {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .globe-close-btn {
    background: rgba(0, 180, 255, 0.2);
    color: #00b4ff;
    border: 1px solid rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .globe-close-btn:hover {
    background: rgba(0, 180, 255, 0.3);
  }

  .theme-skynet .globe-info-label {
    color: rgba(0, 180, 255, 0.6);
  }

  .theme-skynet .globe-info-value {
    color: #00b4ff;
  }

  .theme-skynet .globe-legend,
  .theme-skynet .globe-instructions {
    background: rgba(0, 10, 20, 0.9);
    color: rgba(0, 180, 255, 0.7);
    border: 1px solid rgba(0, 180, 255, 0.2);
  }

  .theme-skynet .globe-loading-text {
    color: #00b4ff;
    text-shadow: 0 0 10px rgba(0, 180, 255, 0.5);
  }

  .theme-skynet .globe-spinner {
    border: 3px solid rgba(0, 180, 255, 0.2);
    border-top-color: #00b4ff;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    box-shadow: 0 0 20px rgba(0, 180, 255, 0.4);
  }

  /* Theme: Cyberpunk */
  .theme-cyberpunk {
    background-color: #0a0a0a;
  }

  .theme-cyberpunk .globe-controls {
    background: rgba(20, 0, 30, 0.95);
    border: 1px solid rgba(255, 0, 128, 0.4);
    box-shadow: 0 0 20px rgba(255, 0, 128, 0.2);
  }

  .theme-cyberpunk .globe-controls-title {
    color: #ff0080;
    text-shadow: 0 0 10px rgba(255, 0, 128, 0.5);
  }

  .theme-cyberpunk .globe-border {
    border-color: rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .globe-checkbox {
    accent-color: #ff0080;
  }

  .theme-cyberpunk .globe-slider {
    accent-color: #00d9ff;
  }

  .theme-cyberpunk .globe-reset-btn {
    background: linear-gradient(135deg, #ff0080, #b000ff);
    color: white;
    box-shadow: 0 0 15px rgba(255, 0, 128, 0.4);
  }

  .theme-cyberpunk .globe-reset-btn:hover {
    box-shadow: 0 0 25px rgba(255, 0, 128, 0.6);
  }

  .theme-cyberpunk .globe-info-panel {
    background: rgba(20, 0, 30, 0.95);
    border: 1px solid rgba(0, 217, 255, 0.5);
    box-shadow: 0 0 20px rgba(0, 217, 255, 0.3);
  }

  .theme-cyberpunk .globe-info-title {
    color: #00d9ff;
    text-shadow: 0 0 10px rgba(0, 217, 255, 0.5);
  }

  .theme-cyberpunk .globe-close-btn {
    background: rgba(255, 0, 128, 0.2);
    color: #ff0080;
    border: 1px solid rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .globe-close-btn:hover {
    background: rgba(255, 0, 128, 0.3);
  }

  .theme-cyberpunk .globe-info-label {
    color: rgba(255, 0, 128, 0.6);
  }

  .theme-cyberpunk .globe-info-value {
    color: #00d9ff;
  }

  .theme-cyberpunk .globe-legend,
  .theme-cyberpunk .globe-instructions {
    background: rgba(20, 0, 30, 0.9);
    color: rgba(0, 217, 255, 0.7);
    border: 1px solid rgba(255, 0, 128, 0.2);
  }

  .theme-cyberpunk .globe-loading-text {
    color: #ff0080;
    text-shadow: 0 0 10px rgba(255, 0, 128, 0.5);
  }

  .theme-cyberpunk .globe-spinner {
    border: 3px solid rgba(255, 0, 128, 0.2);
    border-top-color: #ff0080;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    box-shadow: 0 0 20px rgba(255, 0, 128, 0.4);
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Common styles */
  .globe-controls,
  .globe-info-panel,
  .globe-legend,
  .globe-instructions {
    color: #e2e8f0;
  }

  .globe-checkbox {
    width: 14px;
    height: 14px;
    cursor: pointer;
  }

  .globe-slider {
    height: 4px;
    border-radius: 2px;
    cursor: pointer;
  }
</style>
