<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import Chart from 'chart.js/auto';

  interface Props {
    userId: string;
    initialGranularity?: 'month' | 'quarter' | 'year';
  }

  let { userId, initialGranularity = 'month' }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let data = $state<any[]>([]);

  // Filter states
  let granularity = $state<'month' | 'quarter' | 'year'>(initialGranularity);
  let yearFilter = $state<number | null>(null);
  let intervalFilter = $state<'all' | '1y' | '3y' | '5y' | '10y'>('all');
  let excludeOutliers = $state(false);
  let availableYears = $state<number[]>([]);

  // Track the latest date in the dataset for smart interval calculation
  let latestDataDate = $state<Date | null>(null);
  let yearsLoaded = $state(false);

  // Computed date range based on filters - uses dataset's latest date for intervals
  let dateRange = $derived(() => {
    let startDate: string | null = null;
    let endDate: string | null = null;

    if (yearFilter !== null) {
      // Specific year filter
      startDate = `${yearFilter}-01-01`;
      endDate = `${yearFilter}-12-31`;
    } else if (intervalFilter !== 'all' && latestDataDate) {
      // Interval filter - base on latest data date, not current date
      const years = parseInt(intervalFilter.replace('y', ''));
      const end = latestDataDate;
      const start = new Date(end.getFullYear() - years, end.getMonth(), end.getDate());
      startDate = start.toISOString().split('T')[0];
      endDate = end.toISOString().split('T')[0];
    }

    return { startDate, endDate };
  });

  // Safely sort and validate data
  let sortedData = $derived(() => {
    try {
      if (!Array.isArray(data) || data.length === 0) return [];
      return [...data]
        .filter(d => d && d.period && typeof d.flight_count === 'number')
        .sort((a, b) => (a.period_start || a.period).localeCompare(b.period_start || b.period));
    } catch (err) {
      console.error('Error sorting temporal data:', err);
      return [];
    }
  });

  // Calculate outlier threshold using IQR method
  function calculateOutlierThreshold(values: number[]): number {
    if (values.length < 4) return Math.max(...values) * 1.5;

    const sorted = [...values].sort((a, b) => a - b);
    const q1Index = Math.floor(sorted.length * 0.25);
    const q3Index = Math.floor(sorted.length * 0.75);
    const q1 = sorted[q1Index];
    const q3 = sorted[q3Index];
    const iqr = q3 - q1;

    // Upper fence: Q3 + 1.5 * IQR
    return q3 + 1.5 * iqr;
  }

  // Get chart max value, optionally excluding outliers
  let chartMaxFlights = $derived(() => {
    const values = sortedData().map(d => d.flight_count || 0);
    if (values.length === 0) return 10;

    if (excludeOutliers) {
      const threshold = calculateOutlierThreshold(values);
      // Use threshold as max, but ensure it's at least as high as the median * 2
      const sorted = [...values].sort((a, b) => a - b);
      const median = sorted[Math.floor(sorted.length / 2)];
      return Math.ceil(Math.max(threshold, median * 2) * 1.1);
    }

    return Math.ceil(Math.max(...values) * 1.15);
  });

  let chartMaxDistance = $derived(() => {
    const values = sortedData().map(d => d.total_distance_km || 0);
    if (values.length === 0) return 1000;

    if (excludeOutliers) {
      const threshold = calculateOutlierThreshold(values);
      const sorted = [...values].sort((a, b) => a - b);
      const median = sorted[Math.floor(sorted.length / 2)];
      return Math.ceil(Math.max(threshold, median * 2) * 1.1);
    }

    return Math.ceil(Math.max(...values) * 1.15);
  });

  // Summary statistics
  let stats = $derived(() => {
    const filtered = sortedData();
    if (filtered.length === 0) return null;

    const totalFlights = filtered.reduce((sum, d) => sum + (d.flight_count || 0), 0);
    const totalDistance = filtered.reduce((sum, d) => sum + (d.total_distance_km || 0), 0);
    const totalCO2 = filtered.reduce((sum, d) => sum + (d.total_co2_kg || 0), 0);
    const avgFlightsPerPeriod = totalFlights / filtered.length;

    return {
      totalFlights,
      totalDistance: Math.round(totalDistance),
      totalCO2: Math.round(totalCO2),
      avgFlightsPerPeriod: avgFlightsPerPeriod.toFixed(1),
      periodCount: filtered.length
    };
  });

  async function loadData() {
    loading = true;
    error = null;

    try {
      const range = dateRange();
      console.log('[TemporalChart] Loading data with range:', range);

      const result = await invoke<any[]>('get_temporal_analysis', {
        request: {
          user_id: userId,
          granularity,
          start_date: range.startDate,
          end_date: range.endDate
        }
      });

      data = Array.isArray(result) ? result : [];
      console.log('[TemporalChart] Loaded', data.length, 'items');

      // On first unfiltered load, extract available years and find latest date
      if (!yearsLoaded && range.startDate === null && range.endDate === null && data.length > 0) {
        const years = new Set<number>();
        let maxDate: Date | null = null;

        data.forEach(d => {
          const dateStr = d.period_start || d.period;
          if (dateStr) {
            // Parse year
            const year = parseInt(dateStr.split('-')[0]);
            if (!isNaN(year)) years.add(year);

            // Track latest date
            const parsed = new Date(dateStr);
            if (!isNaN(parsed.getTime())) {
              if (!maxDate || parsed > maxDate) {
                maxDate = parsed;
              }
            }
          }
        });

        availableYears = Array.from(years).sort((a, b) => b - a);
        latestDataDate = maxDate;
        yearsLoaded = true;

        console.log('[TemporalChart] Available years:', availableYears);
        console.log('[TemporalChart] Latest data date:', latestDataDate);
      }

    } catch (err) {
      console.error('Failed to load temporal data:', err);
      error = String(err);
    } finally {
      loading = false;
    }
  }

  function createChart() {
    if (!canvas) return;

    try {
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        error = 'Could not get canvas context';
        return;
      }

      const safeData = sortedData();
      if (safeData.length === 0) {
        if (chart) {
          chart.destroy();
          chart = null;
        }
        return;
      }

      // Destroy existing chart
      if (chart) {
        chart.destroy();
        chart = null;
      }

      const maxFlights = chartMaxFlights();
      const maxDistance = chartMaxDistance();

      chart = new Chart(ctx, {
        type: 'bar',
        data: {
          labels: safeData.map(d => d.period || 'Unknown'),
          datasets: [
            {
              label: 'Number of Flights',
              data: safeData.map(d => d.flight_count || 0),
              backgroundColor: 'rgba(59, 130, 246, 0.6)',
              borderColor: 'rgb(59, 130, 246)',
              borderWidth: 1,
              yAxisID: 'y'
            },
            {
              label: 'Total Distance (km)',
              data: safeData.map(d => d.total_distance_km || 0),
              type: 'line',
              borderColor: '#f59e0b',
              backgroundColor: 'rgba(245, 158, 11, 0.2)',
              borderWidth: 3,
              fill: true,
              tension: 0.4,
              yAxisID: 'y1'
            }
          ]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          interaction: { mode: 'index', intersect: false },
          plugins: {
            title: {
              display: true,
              text: getChartTitle(),
              font: { size: 16, weight: 'bold' }
            },
            tooltip: {
              callbacks: {
                afterLabel: (ctx) => {
                  if (ctx.datasetIndex === 0) {
                    const d = safeData[ctx.dataIndex];
                    const co2 = d?.total_co2_kg || 0;
                    const lines = [`CO₂: ${co2.toFixed(0)} kg`];
                    // Indicate if value exceeds chart max (outlier)
                    if (excludeOutliers && d.flight_count > maxFlights) {
                      lines.push(`⚠️ Outlier (actual: ${d.flight_count})`);
                    }
                    return lines;
                  }
                  return '';
                }
              }
            },
            legend: {
              display: true,
              position: 'top'
            }
          },
          scales: {
            y: {
              position: 'left',
              title: { display: true, text: excludeOutliers ? 'Flights (outliers capped)' : 'Flights' },
              beginAtZero: true,
              max: maxFlights,
              ticks: {
                stepSize: Math.ceil(maxFlights / 5) || 1
              }
            },
            y1: {
              position: 'right',
              title: { display: true, text: 'Distance (km)' },
              grid: { drawOnChartArea: false },
              beginAtZero: true,
              max: maxDistance
            }
          }
        }
      });

      error = null;
    } catch (err) {
      console.error('Error creating temporal chart:', err);
      error = `Failed to create chart: ${err}`;
    }
  }

  function getChartTitle(): string {
    let title = 'Flight Activity';

    if (yearFilter !== null) {
      title += ` - ${yearFilter}`;
    } else if (intervalFilter !== 'all' && latestDataDate) {
      const years = parseInt(intervalFilter.replace('y', ''));
      const endYear = latestDataDate.getFullYear();
      const startYear = endYear - years;
      title += ` - ${startYear} to ${endYear}`;
    }

    title += ` (${granularity === 'month' ? 'Monthly' : granularity === 'quarter' ? 'Quarterly' : 'Yearly'})`;

    return title;
  }

  // Export functions
  async function exportCSV() {
    const safeData = sortedData();
    if (safeData.length === 0) return;

    const headers = ['Period', 'Flight Count', 'Total Distance (km)', 'CO2 (kg)'];
    const rows = safeData.map(d => [
      d.period,
      d.flight_count,
      d.total_distance_km?.toFixed(2) || '0',
      d.total_co2_kg?.toFixed(2) || '0'
    ]);

    const csv = [headers.join(','), ...rows.map(r => r.join(','))].join('\n');

    try {
      const filePath = await save({
        defaultPath: `flight-patterns-${granularity}-${new Date().toISOString().split('T')[0]}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }]
      });

      if (filePath) {
        await invoke('write_file', { path: filePath, content: csv });
      }
    } catch (err) {
      console.error('Export failed:', err);
    }
  }

  async function exportPNG() {
    if (!canvas) return;

    try {
      const dataUrl = canvas.toDataURL('image/png');
      const base64 = dataUrl.split(',')[1];

      const filePath = await save({
        defaultPath: `flight-patterns-${granularity}-${new Date().toISOString().split('T')[0]}.png`,
        filters: [{ name: 'PNG Image', extensions: ['png'] }]
      });

      if (filePath) {
        await invoke('write_base64_file', { path: filePath, base64Content: base64 });
      }
    } catch (err) {
      console.error('Export failed:', err);
    }
  }

  function handleGranularityChange(newGranularity: 'month' | 'quarter' | 'year') {
    granularity = newGranularity;
    loadData();
  }

  function handleYearChange(year: number | null) {
    yearFilter = year;
    if (year !== null) intervalFilter = 'all';
    loadData();
  }

  function handleIntervalChange(interval: 'all' | '1y' | '3y' | '5y' | '10y') {
    intervalFilter = interval;
    if (interval !== 'all') yearFilter = null;
    loadData();
  }

  function handleOutlierToggle() {
    excludeOutliers = !excludeOutliers;
    // Just recreate chart with new scale, no data reload needed
    setTimeout(() => {
      if (canvas && data.length > 0) {
        createChart();
      }
    }, 0);
  }

  // Track data array reference for effect
  let lastDataLength = $state(0);

  // React to data changes only - recreate chart
  $effect(() => {
    // Only react to actual data changes, not derived value changes
    const currentLength = data.length;
    if (currentLength !== lastDataLength) {
      lastDataLength = currentLength;
      if (currentLength > 0 && canvas) {
        setTimeout(() => createChart(), 0);
      }
    }
  });

  onMount(() => {
    loadData();
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
  <div class="flex flex-wrap items-center gap-4 p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg">
    <!-- Granularity -->
    <div class="flex items-center gap-2">
      <span class="text-sm font-medium text-gray-600 dark:text-gray-400">View:</span>
      <div class="flex rounded-lg overflow-hidden border border-gray-300 dark:border-gray-600">
        {#each ['month', 'quarter', 'year'] as g}
          <button
            onclick={() => handleGranularityChange(g as any)}
            class="px-3 py-1.5 text-sm font-medium transition {granularity === g
              ? 'bg-blue-600 text-white'
              : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-600'}"
          >
            {g === 'month' ? 'Monthly' : g === 'quarter' ? 'Quarterly' : 'Yearly'}
          </button>
        {/each}
      </div>
    </div>

    <!-- Year Filter -->
    <div class="flex items-center gap-2">
      <span class="text-sm font-medium text-gray-600 dark:text-gray-400">Year:</span>
      <select
        value={yearFilter ?? ''}
        onchange={(e) => handleYearChange(e.currentTarget.value ? parseInt(e.currentTarget.value) : null)}
        class="px-3 py-1.5 text-sm rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
      >
        <option value="">All Years</option>
        {#each availableYears as year}
          <option value={year}>{year}</option>
        {/each}
      </select>
    </div>

    <!-- Interval Filter -->
    <div class="flex items-center gap-2">
      <span class="text-sm font-medium text-gray-600 dark:text-gray-400">Range:</span>
      <div class="flex rounded-lg overflow-hidden border border-gray-300 dark:border-gray-600">
        {#each [['all', 'All'], ['1y', '1Y'], ['3y', '3Y'], ['5y', '5Y'], ['10y', '10Y']] as [val, label]}
          <button
            onclick={() => handleIntervalChange(val as any)}
            class="px-3 py-1.5 text-sm font-medium transition {intervalFilter === val
              ? 'bg-green-600 text-white'
              : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-600'}"
          >
            {label}
          </button>
        {/each}
      </div>
      {#if latestDataDate && intervalFilter !== 'all'}
        <span class="text-xs text-gray-500 dark:text-gray-400">
          (ends {latestDataDate.getFullYear()})
        </span>
      {/if}
    </div>

    <!-- Exclude Outliers -->
    <label class="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        checked={excludeOutliers}
        onchange={handleOutlierToggle}
        class="w-4 h-4 rounded border-gray-300 dark:border-gray-600 text-blue-600 focus:ring-blue-500"
      />
      <span class="text-sm text-gray-600 dark:text-gray-400">Exclude outliers</span>
    </label>

    <!-- Spacer -->
    <div class="flex-1"></div>

    <!-- Export Buttons -->
    <div class="flex items-center gap-2">
      <button
        onclick={exportCSV}
        disabled={sortedData().length === 0}
        class="flex items-center gap-1 px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed transition"
      >
        <span>📄</span> CSV
      </button>
      <button
        onclick={exportPNG}
        disabled={sortedData().length === 0}
        class="flex items-center gap-1 px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed transition"
      >
        <span>🖼️</span> PNG
      </button>
    </div>
  </div>

  <!-- Stats Summary -->
  {#if stats()}
    {@const s = stats()}
    <div class="grid grid-cols-2 md:grid-cols-5 gap-3">
      <div class="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-blue-600 dark:text-blue-400">{s?.totalFlights}</div>
        <div class="text-xs text-blue-600/70 dark:text-blue-400/70">Total Flights</div>
      </div>
      <div class="bg-amber-50 dark:bg-amber-900/20 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-amber-600 dark:text-amber-400">{s?.totalDistance.toLocaleString()}</div>
        <div class="text-xs text-amber-600/70 dark:text-amber-400/70">Total km</div>
      </div>
      <div class="bg-green-50 dark:bg-green-900/20 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-green-600 dark:text-green-400">{s?.totalCO2.toLocaleString()}</div>
        <div class="text-xs text-green-600/70 dark:text-green-400/70">CO₂ (kg)</div>
      </div>
      <div class="bg-purple-50 dark:bg-purple-900/20 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-purple-600 dark:text-purple-400">{s?.avgFlightsPerPeriod}</div>
        <div class="text-xs text-purple-600/70 dark:text-purple-400/70">Avg/Period</div>
      </div>
      <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-gray-600 dark:text-gray-400">{s?.periodCount}</div>
        <div class="text-xs text-gray-500 dark:text-gray-500">Periods</div>
      </div>
    </div>
  {/if}

  <!-- Chart Area -->
  {#if loading}
    <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg">
      <div class="text-center">
        <div class="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600 mx-auto mb-3"></div>
        <p class="text-gray-500 dark:text-gray-400 text-sm">Loading flight data...</p>
      </div>
    </div>
  {:else if error}
    <div class="flex items-center justify-center h-[400px] bg-red-50 dark:bg-red-900/20 rounded-lg p-6">
      <div class="text-center">
        <p class="text-red-600 dark:text-red-400 font-semibold mb-2">Chart Error</p>
        <p class="text-red-500 dark:text-red-300 text-sm">{error}</p>
        <button
          onclick={loadData}
          class="mt-4 px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg text-sm"
        >
          Retry
        </button>
      </div>
    </div>
  {:else if sortedData().length === 0}
    <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
      <div class="text-center">
        <div class="text-4xl mb-3">📊</div>
        <p class="text-gray-600 dark:text-gray-400">No flight data for selected filters</p>
        <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Try adjusting the date range or granularity</p>
      </div>
    </div>
  {:else}
    <div class="h-[400px] bg-white dark:bg-gray-800 rounded-lg p-4">
      <canvas bind:this={canvas}></canvas>
    </div>
  {/if}
</div>
