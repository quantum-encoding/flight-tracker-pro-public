<!-- src/lib/components/analytics/GeospatialChart.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { writeTextFile, writeFile } from '@tauri-apps/plugin-fs';
  import Chart from 'chart.js/auto';
  import { TreemapController, TreemapElement } from 'chartjs-chart-treemap';
  import type { AirportVisitData } from '$lib/types/analytics';

  // Register the treemap plugin
  Chart.register(TreemapController, TreemapElement);

  interface Props {
    userId: string;
    initialLimit?: number;
  }

  let { userId, initialLimit = 50 }: Props = $props();

  // Data state
  let airportData = $state<AirportVisitData[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Filter state
  let selectedYear = $state<number | null>(null);
  let selectedInterval = $state<string>('all');
  let sortBy = $state<'visits' | 'departures' | 'arrivals'>('visits');
  let limit = $state(initialLimit);

  // Available years - generate a wide range to cover historical data
  let availableYears = $derived(() => {
    const currentYear = new Date().getFullYear();
    const years: number[] = [];
    // Go back 50 years to cover historical flight data
    for (let y = currentYear; y >= 1970; y--) {
      years.push(y);
    }
    return years;
  });

  // Interval options
  const intervalOptions = [
    { value: 'all', label: 'All Time' },
    { value: '1y', label: '1 Year' },
    { value: '3y', label: '3 Years' },
    { value: '5y', label: '5 Years' },
    { value: '10y', label: '10 Years' }
  ];

  // Limit options
  const limitOptions = [10, 25, 50, 100];

  // Calculate date range based on filters
  let dateRange = $derived(() => {
    const now = new Date();
    let startDate: string | null = null;
    let endDate: string | null = null;

    if (selectedYear !== null) {
      startDate = `${selectedYear}-01-01`;
      endDate = `${selectedYear}-12-31`;
    } else if (selectedInterval !== 'all') {
      const years = parseInt(selectedInterval.replace('y', ''));
      const start = new Date(now);
      start.setFullYear(start.getFullYear() - years);
      startDate = start.toISOString().split('T')[0];
      endDate = now.toISOString().split('T')[0];
    }

    return { startDate, endDate };
  });

  // Canvas and chart refs
  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);

  // Sorted data based on sortBy
  let sortedData = $derived(() => {
    if (!airportData.length) return [];
    const sorted = [...airportData];
    switch (sortBy) {
      case 'departures':
        sorted.sort((a, b) => b.departure_count - a.departure_count);
        break;
      case 'arrivals':
        sorted.sort((a, b) => b.arrival_count - a.arrival_count);
        break;
      default:
        sorted.sort((a, b) => b.total_visits - a.total_visits);
    }
    return sorted;
  });

  // Statistics
  let stats = $derived(() => {
    if (!airportData.length) return null;
    const totalVisits = airportData.reduce((sum, a) => sum + a.total_visits, 0);
    const totalDepartures = airportData.reduce((sum, a) => sum + a.departure_count, 0);
    const totalArrivals = airportData.reduce((sum, a) => sum + a.arrival_count, 0);
    const uniqueAirports = airportData.length;
    const topAirport = sortedData()[0];
    return {
      totalVisits,
      totalDepartures,
      totalArrivals,
      uniqueAirports,
      topAirport
    };
  });

  async function loadData() {
    if (!userId) {
      error = 'User ID is required';
      loading = false;
      return;
    }

    loading = true;
    error = null;

    try {
      const { startDate, endDate } = dateRange();

      const result = await invoke<AirportVisitData[]>('get_geospatial_analysis', {
        request: {
          user_id: userId,
          limit: limit,
          start_date: startDate,
          end_date: endDate
        }
      });

      airportData = result || [];

      if (airportData.length === 0) {
        error = 'No airport visit data available for the selected period';
      }
    } catch (err) {
      console.error('Failed to load airport data:', err);
      error = `Failed to load data: ${err}`;
      airportData = [];
    } finally {
      loading = false;
    }
  }

  function createChart() {
    if (!canvas || sortedData().length === 0) return;

    try {
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        error = 'Could not get canvas context';
        return;
      }

      // Destroy existing chart
      if (chart) {
        chart.destroy();
        chart = null;
      }

      const data = sortedData();
      const maxVisits = Math.max(...data.map(a => a.total_visits), 1);

      // Generate gradient colors based on value
      function getColor(value: number, alpha: number = 1): string {
        const ratio = value / maxVisits;
        // Gradient from light blue to deep blue
        const r = Math.round(59 + (30 - 59) * ratio);
        const g = Math.round(130 + (64 - 130) * ratio);
        const b = Math.round(246 + (175 - 246) * ratio);
        return `rgba(${r}, ${g}, ${b}, ${alpha})`;
      }

      chart = new Chart(ctx, {
        type: 'treemap',
        data: {
          datasets: [{
            label: 'Airport Visits',
            tree: data.map(a => ({
              id: a.airport_code,
              label: `${a.airport_code} – ${a.total_visits} visits`,
              value: a.total_visits,
              airport: a
            })),
            backgroundColor(ctx: any) {
              if (!ctx.raw) return '#888';
              const value = ctx.raw.v as number;
              const alpha = 0.4 + 0.6 * (value / maxVisits);
              return getColor(value, alpha);
            },
            borderWidth: 2,
            borderColor: '#fff',
            spacing: 1,
            labels: {
              display: true,
              align: 'center',
              position: 'middle',
              formatter: (ctx: any) => {
                if (!ctx.raw || !ctx.raw.airport) return '';
                const airport = ctx.raw.airport as AirportVisitData;
                if (!airport || !airport.airport_code) return '';
                return [airport.airport_code, `${airport.total_visits || 0}`];
              },
              color: '#fff',
              font: {
                size: 11,
                weight: 'bold'
              }
            }
          } as any]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            title: {
              display: true,
              text: getChartTitle(),
              font: { size: 16, weight: 'bold' }
            },
            legend: { display: false },
            tooltip: {
              callbacks: {
                title: () => '',
                label(ctx: any) {
                  const a = ctx.raw.airport as AirportVisitData;
                  return [
                    `${a.airport_name || a.airport_code}`,
                    `Location: ${a.location || 'Unknown'}`,
                    `Total visits: ${a.total_visits}`,
                    `Departures: ${a.departure_count} | Arrivals: ${a.arrival_count}`
                  ];
                }
              },
              backgroundColor: 'rgba(0, 0, 0, 0.8)',
              titleFont: { size: 14, weight: 'bold' },
              bodyFont: { size: 12 },
              padding: 12,
              cornerRadius: 8
            }
          }
        }
      });

    } catch (err) {
      console.error('Error creating geospatial chart:', err);
      error = `Failed to create chart: ${err}`;
    }
  }

  function getChartTitle(): string {
    let title = 'Most Visited Airports';
    if (selectedYear !== null) {
      title += ` (${selectedYear})`;
    } else if (selectedInterval !== 'all') {
      title += ` (Last ${selectedInterval.replace('y', ' Year').replace('1 Year', '1 Year').replace('3 Year', '3 Years').replace('5 Year', '5 Years').replace('10 Year', '10 Years')})`;
    }
    return title;
  }

  // Export functions
  async function exportToCsv() {
    const data = sortedData();
    if (data.length === 0) return;

    try {
      const headers = ['Rank', 'Airport Code', 'Airport Name', 'Location', 'Total Visits', 'Departures', 'Arrivals', 'Dep/Arr Ratio'];
      const rows = data.map((a, i) => [
        i + 1,
        a.airport_code,
        `"${a.airport_name || 'Unknown'}"`,
        `"${a.location || 'Unknown'}"`,
        a.total_visits,
        a.departure_count,
        a.arrival_count,
        a.arrival_count > 0 ? (a.departure_count / a.arrival_count).toFixed(2) : 'N/A'
      ]);

      const csv = [headers.join(','), ...rows.map(r => r.join(','))].join('\n');

      const filePath = await save({
        filters: [{ name: 'CSV', extensions: ['csv'] }],
        defaultPath: `airport-visits-${selectedYear || selectedInterval}.csv`
      });

      if (filePath) {
        await writeTextFile(filePath, csv);
      }
    } catch (err) {
      console.error('Failed to export CSV:', err);
    }
  }

  async function exportToPng() {
    if (!canvas) return;

    try {
      const dataUrl = canvas.toDataURL('image/png');
      const base64Data = dataUrl.split(',')[1];
      const binaryString = atob(base64Data);
      const bytes = new Uint8Array(binaryString.length);
      for (let i = 0; i < binaryString.length; i++) {
        bytes[i] = binaryString.charCodeAt(i);
      }

      const filePath = await save({
        filters: [{ name: 'PNG Image', extensions: ['png'] }],
        defaultPath: `airport-concentration-${selectedYear || selectedInterval}.png`
      });

      if (filePath) {
        await writeFile(filePath, bytes);
      }
    } catch (err) {
      console.error('Failed to export PNG:', err);
    }
  }

  // Handle filter changes
  function handleYearChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const value = target.value;
    if (value === '') {
      selectedYear = null;
    } else {
      selectedYear = parseInt(value);
      selectedInterval = 'all'; // Clear interval when year is selected
    }
  }

  function handleIntervalClick(interval: string) {
    selectedInterval = interval;
    selectedYear = null; // Clear year when interval is selected
  }

  function handleLimitChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    limit = parseInt(target.value);
  }

  // Track data changes for chart updates
  let lastDataHash = $state('');

  $effect(() => {
    // Reload data when filters change
    const { startDate, endDate } = dateRange();
    const filterHash = `${userId}-${startDate}-${endDate}-${limit}`;
    if (filterHash !== lastDataHash && userId) {
      lastDataHash = filterHash;
      loadData();
    }
  });

  $effect(() => {
    // Recreate chart when sorted data changes
    if (sortedData().length > 0 && canvas) {
      setTimeout(() => createChart(), 0);
    }
  });

  onMount(() => {
    if (userId) {
      loadData();
    }
  });

  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });
</script>

<div class="space-y-4">
  <!-- Filter Controls -->
  <div class="flex flex-wrap items-center gap-4 p-4 bg-gray-50 dark:bg-gray-800 rounded-lg">
    <!-- Year Filter -->
    <div class="flex items-center gap-2">
      <label for="year-select" class="text-sm font-medium text-gray-700 dark:text-gray-300">Year:</label>
      <select
        id="year-select"
        class="px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500"
        value={selectedYear?.toString() ?? ''}
        onchange={handleYearChange}
      >
        <option value="">All Years</option>
        {#each availableYears() as year}
          <option value={year.toString()}>{year}</option>
        {/each}
      </select>
    </div>

    <!-- Interval Filters -->
    <div class="flex items-center gap-1">
      {#each intervalOptions as opt}
        <button
          type="button"
          class="px-3 py-1.5 text-sm rounded-md transition-colors {selectedInterval === opt.value && selectedYear === null
            ? 'bg-blue-600 text-white'
            : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-600'}"
          onclick={() => handleIntervalClick(opt.value)}
        >
          {opt.label}
        </button>
      {/each}
    </div>

    <!-- Limit Filter -->
    <div class="flex items-center gap-2">
      <label for="limit-select" class="text-sm font-medium text-gray-700 dark:text-gray-300">Show:</label>
      <select
        id="limit-select"
        class="px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500"
        value={limit.toString()}
        onchange={handleLimitChange}
      >
        {#each limitOptions as opt}
          <option value={opt.toString()}>Top {opt}</option>
        {/each}
      </select>
    </div>

    <!-- Sort By -->
    <div class="flex items-center gap-2">
      <label for="sort-select" class="text-sm font-medium text-gray-700 dark:text-gray-300">Sort by:</label>
      <select
        id="sort-select"
        class="px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 focus:ring-2 focus:ring-blue-500"
        bind:value={sortBy}
      >
        <option value="visits">Total Visits</option>
        <option value="departures">Departures</option>
        <option value="arrivals">Arrivals</option>
      </select>
    </div>

    <!-- Export Buttons -->
    <div class="flex items-center gap-2 ml-auto">
      <button
        type="button"
        class="flex items-center gap-1 px-3 py-1.5 text-sm bg-green-600 text-white rounded-md hover:bg-green-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={exportToCsv}
        disabled={sortedData().length === 0}
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 10v6m0 0l-3-3m3 3l3-3m2 8H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
        </svg>
        CSV
      </button>
      <button
        type="button"
        class="flex items-center gap-1 px-3 py-1.5 text-sm bg-purple-600 text-white rounded-md hover:bg-purple-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        onclick={exportToPng}
        disabled={!canvas}
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
        </svg>
        PNG
      </button>
    </div>
  </div>

  <!-- Statistics Summary -->
  {#if stats()}
    {@const s = stats()}
    {#if s}
      <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
        <div class="p-3 bg-blue-50 dark:bg-blue-900/30 rounded-lg">
          <div class="text-2xl font-bold text-blue-600 dark:text-blue-400">{s.uniqueAirports}</div>
          <div class="text-xs text-gray-600 dark:text-gray-400">Unique Airports</div>
        </div>
        <div class="p-3 bg-green-50 dark:bg-green-900/30 rounded-lg">
          <div class="text-2xl font-bold text-green-600 dark:text-green-400">{s.totalVisits.toLocaleString()}</div>
          <div class="text-xs text-gray-600 dark:text-gray-400">Total Visits</div>
        </div>
        <div class="p-3 bg-orange-50 dark:bg-orange-900/30 rounded-lg">
          <div class="text-2xl font-bold text-orange-600 dark:text-orange-400">{s.totalDepartures.toLocaleString()}</div>
          <div class="text-xs text-gray-600 dark:text-gray-400">Departures</div>
        </div>
        <div class="p-3 bg-purple-50 dark:bg-purple-900/30 rounded-lg">
          <div class="text-2xl font-bold text-purple-600 dark:text-purple-400">{s.totalArrivals.toLocaleString()}</div>
          <div class="text-xs text-gray-600 dark:text-gray-400">Arrivals</div>
        </div>
        <div class="p-3 bg-yellow-50 dark:bg-yellow-900/30 rounded-lg">
          <div class="text-lg font-bold text-yellow-600 dark:text-yellow-400 truncate" title={s.topAirport?.airport_name}>
            {s.topAirport?.airport_code || 'N/A'}
          </div>
          <div class="text-xs text-gray-600 dark:text-gray-400">Top Airport</div>
        </div>
      </div>
    {/if}
  {/if}

  <!-- Chart Area -->
  {#if loading}
    <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg">
      <div class="text-center">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
        <p class="text-gray-600 dark:text-gray-400">Loading airport data...</p>
      </div>
    </div>
  {:else if error && sortedData().length === 0}
    <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
      <div class="text-center">
        <div class="text-4xl mb-3">✈️</div>
        <p class="text-gray-600 dark:text-gray-400">{error}</p>
        <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Try adjusting filters or add more flights</p>
      </div>
    </div>
  {:else}
    <div class="h-[400px] bg-white dark:bg-gray-900 rounded-lg p-2">
      <canvas bind:this={canvas}></canvas>
    </div>
  {/if}

  <!-- Airport Table -->
  {#if sortedData().length > 0}
    <div class="bg-white dark:bg-gray-800 rounded-lg overflow-hidden border border-gray-200 dark:border-gray-700">
      <div class="px-4 py-3 bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700">
        <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300">Airport Details</h3>
      </div>
      <div class="max-h-[300px] overflow-y-auto">
        <table class="w-full text-sm">
          <thead class="sticky top-0 bg-gray-50 dark:bg-gray-900">
            <tr class="text-left text-xs uppercase text-gray-500 dark:text-gray-400">
              <th class="px-4 py-2">#</th>
              <th class="px-4 py-2">Code</th>
              <th class="px-4 py-2">Airport Name</th>
              <th class="px-4 py-2">Location</th>
              <th class="px-4 py-2 text-right">Visits</th>
              <th class="px-4 py-2 text-right">Dep</th>
              <th class="px-4 py-2 text-right">Arr</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
            {#each sortedData() as airport, i}
              <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                <td class="px-4 py-2 text-gray-500">{i + 1}</td>
                <td class="px-4 py-2 font-mono font-semibold text-blue-600 dark:text-blue-400">{airport.airport_code}</td>
                <td class="px-4 py-2 text-gray-900 dark:text-gray-100">{airport.airport_name || 'Unknown'}</td>
                <td class="px-4 py-2 text-gray-600 dark:text-gray-400">{airport.location || 'Unknown'}</td>
                <td class="px-4 py-2 text-right font-semibold">{airport.total_visits}</td>
                <td class="px-4 py-2 text-right text-green-600 dark:text-green-400">{airport.departure_count}</td>
                <td class="px-4 py-2 text-right text-purple-600 dark:text-purple-400">{airport.arrival_count}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>
