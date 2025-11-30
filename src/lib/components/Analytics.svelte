<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { writeTextFile } from '@tauri-apps/plugin-fs';
  import { translations } from '$lib/i18n';

  interface RouteStatistic {
    route: string;
    departure_airport: string;
    arrival_airport: string;
    flight_count: number;
    total_distance_km: number;
    avg_duration_minutes: number;
  }

  interface AirportStatistic {
    airport_code: string;
    visit_count: number;
    departure_count: number;
    arrival_count: number;
  }

  interface PassengerStatistic {
    abbreviation: string;
    full_name: string | null;
    total_flights: number;
    total_distance_km: number;
    total_co2_kg: number;
  }

  interface Analytics {
    top_routes: RouteStatistic[];
    most_visited_airports: AirportStatistic[];
    total_unique_routes: number;
    total_unique_airports: number;
  }

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  let currentTab = $state<'routes' | 'destinations' | 'passengers'>('routes');
  let analytics: Analytics | null = $state(null);
  let passengerStats: PassengerStatistic[] = $state([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let limit = $state(10);

  onMount(async () => {
    await loadAnalytics();
    await loadPassengerStats();
  });

  async function loadAnalytics() {
    loading = true;
    error = null;
    try {
      analytics = await invoke('get_analytics', { userId, limit });
    } catch (err) {
      console.error('Failed to load analytics:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  async function loadPassengerStats() {
    try {
      const allPassengers = await invoke<any[]>('get_all_passenger_names', { userId });

      const statsPromises = allPassengers.map(async (p) => {
        const details = await invoke<any>('get_passenger_details', {
          userId,
          abbreviation: p.abbreviation
        });
        return {
          abbreviation: p.abbreviation,
          full_name: p.full_name,
          total_flights: details.total_flights || 0,
          total_distance_km: details.total_distance_km || 0,
          total_co2_kg: details.total_co2_kg || 0,
        };
      });

      passengerStats = await Promise.all(statsPromises);
    } catch (err) {
      console.error('Failed to load passenger stats:', err);
    }
  }

  function exportAsMarkdown() {
    let content = `# ✈️ Flight Analytics Report\n\n`;
    content += `Generated on ${new Date().toLocaleDateString()}\n\n`;
    content += `---\n\n`;

    if (currentTab === 'routes' && analytics) {
      content += `## 🛫 Most Flown Routes\n\n`;
      content += `| Rank | Route | Flights | Total Distance | Avg Duration |\n`;
      content += `|------|-------|---------|----------------|-------------|\n`;
      analytics.top_routes.forEach((route, i) => {
        const medal = i === 0 ? '🥇' : i === 1 ? '🥈' : i === 2 ? '🥉' : `#${i + 1}`;
        content += `| ${medal} | ${route.route} | ${route.flight_count} | ${Math.round(route.total_distance_km).toLocaleString()} km | ${Math.round(route.avg_duration_minutes)} min |\n`;
      });
    } else if (currentTab === 'destinations' && analytics) {
      content += `## 🏛️ Favorite Destinations\n\n`;
      content += `| Rank | Airport | Total Visits | Departures | Arrivals |\n`;
      content += `|------|---------|--------------|------------|----------|\n`;
      analytics.most_visited_airports.forEach((airport, i) => {
        const medal = i === 0 ? '🥇' : i === 1 ? '🥈' : i === 2 ? '🥉' : `#${i + 1}`;
        content += `| ${medal} | ${airport.airport_code} | ${airport.visit_count} | ${airport.departure_count} | ${airport.arrival_count} |\n`;
      });
    } else if (currentTab === 'passengers') {
      const topByFlights = [...passengerStats].sort((a, b) => b.total_flights - a.total_flights).slice(0, 10);
      const topByCO2 = [...passengerStats].sort((a, b) => b.total_co2_kg - a.total_co2_kg).slice(0, 10);

      content += `## 👥 Passenger Analytics\n\n`;
      content += `### Most Frequent Flyers\n\n`;
      content += `| Rank | Name | Total Flights | Distance (km) | CO₂ (kg) |\n`;
      content += `|------|------|---------------|---------------|----------|\n`;
      topByFlights.forEach((p, i) => {
        const medal = i === 0 ? '🥇' : i === 1 ? '🥈' : i === 2 ? '🥉' : `#${i + 1}`;
        const name = p.full_name || p.abbreviation;
        content += `| ${medal} | ${name} | ${p.total_flights} | ${Math.round(p.total_distance_km).toLocaleString()} | ${Math.round(p.total_co2_kg).toLocaleString()} |\n`;
      });

      content += `\n### 🌍 Top CO₂ Generators\n\n`;
      content += `> Hey ${topByCO2[0]?.full_name || topByCO2[0]?.abbreviation}, you better plant some trees! 🌳\n\n`;
      content += `| Rank | Name | CO₂ Generated (kg) | Flights | Distance (km) |\n`;
      content += `|------|------|--------------------|---------|---------------|\n`;
      topByCO2.forEach((p, i) => {
        const medal = i === 0 ? '🥇' : i === 1 ? '🥈' : i === 2 ? '🥉' : `#${i + 1}`;
        const name = p.full_name || p.abbreviation;
        content += `| ${medal} | ${name} | ${Math.round(p.total_co2_kg).toLocaleString()} | ${p.total_flights} | ${Math.round(p.total_distance_km).toLocaleString()} |\n`;
      });
    }

    downloadFile(content, 'flight-analytics.md', 'text/markdown');
  }

  function exportAsJSON() {
    let data: any = {};

    if (currentTab === 'routes' && analytics) {
      data = {
        report_type: 'routes',
        generated_at: new Date().toISOString(),
        total_unique_routes: analytics.total_unique_routes,
        top_routes: analytics.top_routes
      };
    } else if (currentTab === 'destinations' && analytics) {
      data = {
        report_type: 'destinations',
        generated_at: new Date().toISOString(),
        most_visited_airports: analytics.most_visited_airports
      };
    } else if (currentTab === 'passengers') {
      const topByFlights = [...passengerStats].sort((a, b) => b.total_flights - a.total_flights);
      const topByCO2 = [...passengerStats].sort((a, b) => b.total_co2_kg - a.total_co2_kg);

      data = {
        report_type: 'passengers',
        generated_at: new Date().toISOString(),
        top_frequent_flyers: topByFlights.slice(0, 10),
        top_co2_generators: topByCO2.slice(0, 10),
        all_passengers: passengerStats
      };
    }

    downloadFile(JSON.stringify(data, null, 2), 'flight-analytics.json', 'application/json');
  }

  function exportAsCSV() {
    let csv = '';

    if (currentTab === 'routes' && analytics) {
      csv = 'Rank,Route,Flights,Total Distance (km),Avg Duration (min)\n';
      analytics.top_routes.forEach((route, i) => {
        csv += `${i + 1},"${route.route}",${route.flight_count},${Math.round(route.total_distance_km)},${Math.round(route.avg_duration_minutes)}\n`;
      });
    } else if (currentTab === 'destinations' && analytics) {
      csv = 'Rank,Airport,Total Visits,Departures,Arrivals\n';
      analytics.most_visited_airports.forEach((airport, i) => {
        csv += `${i + 1},${airport.airport_code},${airport.visit_count},${airport.departure_count},${airport.arrival_count}\n`;
      });
    } else if (currentTab === 'passengers') {
      csv = 'Rank,Name,Total Flights,Distance (km),CO2 (kg)\n';
      const topByFlights = [...passengerStats].sort((a, b) => b.total_flights - a.total_flights).slice(0, 20);
      topByFlights.forEach((p, i) => {
        const name = p.full_name || p.abbreviation;
        csv += `${i + 1},"${name}",${p.total_flights},${Math.round(p.total_distance_km)},${Math.round(p.total_co2_kg)}\n`;
      });
    }

    downloadFile(csv, 'flight-analytics.csv', 'text/csv');
  }

  async function downloadFile(content: string, filename: string, mimeType: string) {
    try {
      const filePath = await save({
        defaultPath: filename,
        filters: [{
          name: filename.split('.')[1].toUpperCase(),
          extensions: [filename.split('.')[1]]
        }]
      });

      if (filePath) {
        await writeTextFile(filePath, content);
        alert('Report exported successfully!');
      }
    } catch (err) {
      console.error('Export failed:', err);
      alert(`Export failed: ${err}`);
    }
  }

  $effect(() => {
    if (userId) {
      loadAnalytics();
      loadPassengerStats();
    }
  });
</script>

<div class="h-full flex flex-col overflow-y-auto">
  <!-- Header -->
  <div class="mb-6 flex items-center justify-between">
    <div>
      <h2 class="text-2xl font-bold text-gray-900 dark:text-white mb-2">{$translations('analytics.title')}</h2>
      <p class="text-gray-600 dark:text-gray-400">
        Your travel patterns, routes, destinations, and passenger statistics
      </p>
    </div>

    <!-- Export Buttons -->
    <div class="flex gap-2">
      <button
        onclick={exportAsMarkdown}
        class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-sm font-medium transition"
        title="Export as Markdown"
      >
        📄 MD
      </button>
      <button
        onclick={exportAsJSON}
        class="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg text-sm font-medium transition"
        title="Export as JSON"
      >
        📊 JSON
      </button>
      <button
        onclick={exportAsCSV}
        class="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg text-sm font-medium transition"
        title="Export as CSV"
      >
        📈 CSV
      </button>
    </div>
  </div>

  <!-- Tab Navigation -->
  <div class="mb-6">
    <div class="border-b border-gray-200 dark:border-gray-700">
      <nav class="-mb-px flex gap-8">
        <button
          onclick={() => currentTab = 'routes'}
          class="border-b-2 py-4 px-1 text-sm font-medium transition {
            currentTab === 'routes'
              ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
              : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400 dark:hover:border-gray-600 dark:hover:text-gray-300'
          }"
        >
          🛫 Routes
        </button>
        <button
          onclick={() => currentTab = 'destinations'}
          class="border-b-2 py-4 px-1 text-sm font-medium transition {
            currentTab === 'destinations'
              ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
              : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400 dark:hover:border-gray-600 dark:hover:text-gray-300'
          }"
        >
          🏛️ Destinations
        </button>
        <button
          onclick={() => currentTab = 'passengers'}
          class="border-b-2 py-4 px-1 text-sm font-medium transition {
            currentTab === 'passengers'
              ? 'border-primary-600 text-primary-600 dark:border-primary-400 dark:text-primary-400'
              : 'border-transparent text-gray-500 hover:border-gray-300 hover:text-gray-700 dark:text-gray-400 dark:hover:border-gray-600 dark:hover:text-gray-300'
          }"
        >
          👥 {$translations('navigation.passengers')}
        </button>
      </nav>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
    </div>
  {:else if error}
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-12 text-center">
      <div class="text-red-500 text-4xl mb-4">⚠️</div>
      <p class="text-red-600 dark:text-red-400 font-medium">Failed to load analytics</p>
      <p class="text-gray-600 dark:text-gray-400 text-sm mt-2">{error}</p>
      <button
        onclick={loadAnalytics}
        class="mt-4 bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg transition"
      >
        Try Again
      </button>
    </div>
  {:else if analytics}
    <div class="space-y-6">
      <!-- Routes Tab -->
      {#if currentTab === 'routes'}
        <!-- Summary Stats -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-900 dark:to-indigo-900 rounded-lg shadow p-6 border-2 border-blue-200 dark:border-blue-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-blue-700 dark:text-blue-300 mb-1">Unique Routes</p>
                <p class="text-3xl font-bold text-blue-900 dark:text-blue-100">
                  {analytics.total_unique_routes}
                </p>
              </div>
              <div class="text-blue-600 dark:text-blue-400 text-4xl">🛫</div>
            </div>
          </div>

          <div class="bg-gradient-to-br from-purple-50 to-pink-50 dark:from-purple-900 dark:to-pink-900 rounded-lg shadow p-6 border-2 border-purple-200 dark:border-purple-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-purple-700 dark:text-purple-300 mb-1">Most Frequent Route</p>
                <p class="text-xl font-bold text-purple-900 dark:text-purple-100">
                  {analytics.top_routes[0]?.route || 'N/A'}
                </p>
              </div>
              <div class="text-purple-600 dark:text-purple-400 text-4xl">⭐</div>
            </div>
          </div>

          <div class="bg-gradient-to-br from-amber-50 to-orange-50 dark:from-amber-900 dark:to-orange-900 rounded-lg shadow p-6 border-2 border-amber-200 dark:border-amber-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-amber-700 dark:text-amber-300 mb-1">Top Route Flights</p>
                <p class="text-3xl font-bold text-amber-900 dark:text-amber-100">
                  {analytics.top_routes[0]?.flight_count || 0}x
                </p>
              </div>
              <div class="text-amber-600 dark:text-amber-400 text-4xl">🏆</div>
            </div>
          </div>
        </div>

        <!-- Most Flown Routes Table -->
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">Most Flown Routes</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Rank
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Route
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Flights
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Total Distance
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Avg Duration
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each analytics.top_routes as route, index}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                    <td class="px-6 py-4 whitespace-nowrap">
                      <div class="flex items-center gap-2">
                        {#if index === 0}
                          <span class="text-2xl">🥇</span>
                        {:else if index === 1}
                          <span class="text-2xl">🥈</span>
                        {:else if index === 2}
                          <span class="text-2xl">🥉</span>
                        {:else}
                          <span class="text-sm font-medium text-gray-600 dark:text-gray-400">#{index + 1}</span>
                        {/if}
                      </div>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <div class="font-semibold text-gray-900 dark:text-white">
                        {route.route}
                      </div>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-lg font-bold text-primary-600 dark:text-primary-400">
                        {route.flight_count}
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {Math.round(route.total_distance_km).toLocaleString()} km
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {Math.round(route.avg_duration_minutes)} min
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>

        <!-- Least Flown Routes -->
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">Least Flown Routes</h3>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">Routes you've only flown once or twice</p>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Route
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Flights
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Distance
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each [...analytics.top_routes].reverse().slice(0, 10) as route}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                    <td class="px-6 py-4 whitespace-nowrap text-gray-900 dark:text-white">
                      {route.route}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-gray-600 dark:text-gray-400">
                      {route.flight_count}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {Math.round(route.total_distance_km).toLocaleString()} km
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- Destinations Tab -->
      {#if currentTab === 'destinations'}
        <!-- Summary Stats -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="bg-gradient-to-br from-green-50 to-emerald-50 dark:from-green-900 dark:to-emerald-900 rounded-lg shadow p-6 border-2 border-green-200 dark:border-green-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-green-700 dark:text-green-300 mb-1">Airports Visited</p>
                <p class="text-3xl font-bold text-green-900 dark:text-green-100">
                  {analytics.total_unique_airports}
                </p>
              </div>
              <div class="text-green-600 dark:text-green-400 text-4xl">🏛️</div>
            </div>
          </div>

          <div class="bg-gradient-to-br from-cyan-50 to-blue-50 dark:from-cyan-900 dark:to-blue-900 rounded-lg shadow p-6 border-2 border-cyan-200 dark:border-cyan-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-cyan-700 dark:text-cyan-300 mb-1">Favorite Airport</p>
                <p class="text-2xl font-bold text-cyan-900 dark:text-cyan-100 font-mono">
                  {analytics.most_visited_airports[0]?.airport_code || 'N/A'}
                </p>
              </div>
              <div class="text-cyan-600 dark:text-cyan-400 text-4xl">⭐</div>
            </div>
          </div>

          <div class="bg-gradient-to-br from-rose-50 to-pink-50 dark:from-rose-900 dark:to-pink-900 rounded-lg shadow p-6 border-2 border-rose-200 dark:border-rose-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-rose-700 dark:text-rose-300 mb-1">Most Visits</p>
                <p class="text-3xl font-bold text-rose-900 dark:text-rose-100">
                  {analytics.most_visited_airports[0]?.visit_count || 0}x
                </p>
              </div>
              <div class="text-rose-600 dark:text-rose-400 text-4xl">🏆</div>
            </div>
          </div>
        </div>

        <!-- Most Visited Airports -->
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">Most Visited Destinations</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Rank
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Airport
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Total Visits
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Departures
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Arrivals
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each analytics.most_visited_airports as airport, index}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                    <td class="px-6 py-4 whitespace-nowrap">
                      <div class="flex items-center gap-2">
                        {#if index === 0}
                          <span class="text-2xl">🥇</span>
                        {:else if index === 1}
                          <span class="text-2xl">🥈</span>
                        {:else if index === 2}
                          <span class="text-2xl">🥉</span>
                        {:else}
                          <span class="text-sm font-medium text-gray-600 dark:text-gray-400">#{index + 1}</span>
                        {/if}
                      </div>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-lg font-bold text-gray-900 dark:text-white font-mono">
                        {airport.airport_code}
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-2xl font-bold text-primary-600 dark:text-primary-400">
                        {airport.visit_count}
                      </span>
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
          </div>
        </div>

        <!-- Least Visited -->
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">Least Visited Destinations</h3>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">Airports you've only visited once or twice</p>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Airport
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Visits
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Departures
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Arrivals
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each [...analytics.most_visited_airports].reverse().slice(0, 10) as airport}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                    <td class="px-6 py-4 whitespace-nowrap text-gray-900 dark:text-white font-mono">
                      {airport.airport_code}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-gray-600 dark:text-gray-400">
                      {airport.visit_count}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-gray-600 dark:text-gray-400">
                      {airport.departure_count}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-gray-600 dark:text-gray-400">
                      {airport.arrival_count}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <!-- Passengers Tab -->
      {#if currentTab === 'passengers'}
        {@const topByFlights = [...passengerStats].sort((a, b) => b.total_flights - a.total_flights).slice(0, 10)}
        {@const topByDistance = [...passengerStats].sort((a, b) => b.total_distance_km - a.total_distance_km).slice(0, 10)}
        {@const topByCO2 = [...passengerStats].sort((a, b) => b.total_co2_kg - a.total_co2_kg).slice(0, 10)}
        {@const leastFlights = [...passengerStats].sort((a, b) => a.total_flights - b.total_flights).slice(0, 10)}

        <!-- Summary Stats -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="bg-gradient-to-br from-indigo-50 to-purple-50 dark:from-indigo-900 dark:to-purple-900 rounded-lg shadow p-6 border-2 border-indigo-200 dark:border-indigo-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-indigo-700 dark:text-indigo-300 mb-1">Total Passengers</p>
                <p class="text-3xl font-bold text-indigo-900 dark:text-indigo-100">
                  {passengerStats.length}
                </p>
              </div>
              <div class="text-indigo-600 dark:text-indigo-400 text-4xl">👥</div>
            </div>
          </div>

          <div class="bg-gradient-to-br from-yellow-50 to-amber-50 dark:from-yellow-900 dark:to-amber-900 rounded-lg shadow p-6 border-2 border-yellow-200 dark:border-yellow-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-yellow-700 dark:text-yellow-300 mb-1">Top Flyer</p>
                <p class="text-lg font-bold text-yellow-900 dark:text-yellow-100">
                  {topByFlights[0]?.full_name || topByFlights[0]?.abbreviation || 'N/A'}
                </p>
                <p class="text-xs text-yellow-700 dark:text-yellow-300">
                  {topByFlights[0]?.total_flights || 0} flights
                </p>
              </div>
              <div class="text-yellow-600 dark:text-yellow-400 text-4xl">🏆</div>
            </div>
          </div>

          <div class="bg-gradient-to-br from-red-50 to-orange-50 dark:from-red-900 dark:to-orange-900 rounded-lg shadow p-6 border-2 border-red-200 dark:border-red-700">
            <div class="flex items-center justify-between">
              <div>
                <p class="text-sm text-red-700 dark:text-red-300 mb-1">Top CO₂ Generator</p>
                <p class="text-lg font-bold text-red-900 dark:text-red-100">
                  {topByCO2[0]?.full_name || topByCO2[0]?.abbreviation || 'N/A'}
                </p>
                <p class="text-xs text-red-700 dark:text-red-300">
                  {Math.round(topByCO2[0]?.total_co2_kg || 0).toLocaleString()} kg
                </p>
              </div>
              <div class="text-red-600 dark:text-red-400 text-4xl">🌍</div>
            </div>
          </div>
        </div>

        <!-- Most Frequent Flyers -->
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">Most Frequent Flyers</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Rank
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Name
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Flights
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Distance (km)
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    CO₂ (kg)
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each topByFlights as passenger, index}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                    <td class="px-6 py-4 whitespace-nowrap">
                      {#if index === 0}
                        <span class="text-2xl">🥇</span>
                      {:else if index === 1}
                        <span class="text-2xl">🥈</span>
                      {:else if index === 2}
                        <span class="text-2xl">🥉</span>
                      {:else}
                        <span class="text-sm font-medium text-gray-600 dark:text-gray-400">#{index + 1}</span>
                      {/if}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-gray-900 dark:text-white">
                      {passenger.full_name || passenger.abbreviation}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-lg font-bold text-primary-600 dark:text-primary-400">
                        {passenger.total_flights}
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {Math.round(passenger.total_distance_km).toLocaleString()}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {Math.round(passenger.total_co2_kg).toLocaleString()}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>

        <!-- Top CO2 Generators -->
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-red-50 dark:bg-red-900/20">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">🌍 Top CO₂ Generators</h3>
            <p class="text-sm text-red-600 dark:text-red-400 mt-1">
              Hey {topByCO2[0]?.full_name || topByCO2[0]?.abbreviation}, you better plant some trees! 🌳
            </p>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Rank
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Name
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    CO₂ Generated (kg)
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Flights
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Distance (km)
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each topByCO2 as passenger, index}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                    <td class="px-6 py-4 whitespace-nowrap">
                      {#if index === 0}
                        <span class="text-2xl">🥇</span>
                      {:else if index === 1}
                        <span class="text-2xl">🥈</span>
                      {:else if index === 2}
                        <span class="text-2xl">🥉</span>
                      {:else}
                        <span class="text-sm font-medium text-gray-600 dark:text-gray-400">#{index + 1}</span>
                      {/if}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-gray-900 dark:text-white">
                      {passenger.full_name || passenger.abbreviation}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-lg font-bold text-red-600 dark:text-red-400">
                        {Math.round(passenger.total_co2_kg).toLocaleString()}
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {passenger.total_flights}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {Math.round(passenger.total_distance_km).toLocaleString()}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>

        <!-- Most Miles Traveled -->
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">Most Miles Traveled</h3>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Rank
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Name
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Distance (km)
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Flights
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each topByDistance as passenger, index}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                    <td class="px-6 py-4 whitespace-nowrap">
                      {#if index === 0}
                        <span class="text-2xl">🥇</span>
                      {:else if index === 1}
                        <span class="text-2xl">🥈</span>
                      {:else if index === 2}
                        <span class="text-2xl">🥉</span>
                      {:else}
                        <span class="text-sm font-medium text-gray-600 dark:text-gray-400">#{index + 1}</span>
                      {/if}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-gray-900 dark:text-white">
                      {passenger.full_name || passenger.abbreviation}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap">
                      <span class="text-lg font-bold text-blue-600 dark:text-blue-400">
                        {Math.round(passenger.total_distance_km).toLocaleString()}
                      </span>
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {passenger.total_flights}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>

        <!-- Least Frequent Flyers -->
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow overflow-hidden">
          <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
            <h3 class="text-xl font-semibold text-gray-900 dark:text-white">Least Frequent Flyers</h3>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">Passengers who rarely fly with you</p>
          </div>
          <div class="overflow-x-auto">
            <table class="w-full">
              <thead class="bg-gray-50 dark:bg-gray-900">
                <tr>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Name
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Flights
                  </th>
                  <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                    Distance (km)
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
                {#each leastFlights as passenger}
                  <tr class="hover:bg-gray-50 dark:hover:bg-gray-900 transition">
                    <td class="px-6 py-4 whitespace-nowrap text-gray-900 dark:text-white">
                      {passenger.full_name || passenger.abbreviation}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-gray-600 dark:text-gray-400">
                      {passenger.total_flights}
                    </td>
                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-600 dark:text-gray-400">
                      {Math.round(passenger.total_distance_km).toLocaleString()}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
