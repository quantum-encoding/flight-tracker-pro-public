<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import Chart from 'chart.js/auto';

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  interface PassengerName {
    abbreviation: string;
    full_name: string | null;
    usage_count: number;
  }

  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let passengers = $state<PassengerName[]>([]);

  // Display options
  let showTop = $state<number>(10);
  let sortBy = $state<'flights' | 'name'>('flights');
  let showOthers = $state(true);

  const topOptions = [5, 10, 15, 20, 25];

  // Color palette for pie chart
  const colorPalette = [
    '#3b82f6', '#ef4444', '#22c55e', '#f59e0b', '#8b5cf6',
    '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1',
    '#14b8a6', '#a855f7', '#eab308', '#0ea5e9', '#d946ef',
    '#10b981', '#f43f5e', '#7c3aed', '#0891b2', '#c026d3'
  ];

  // Processed data for chart
  let chartData = $derived(() => {
    if (passengers.length === 0) return { labels: [], values: [], colors: [] };

    let sorted = [...passengers];
    if (sortBy === 'flights') {
      sorted.sort((a, b) => b.usage_count - a.usage_count);
    } else {
      sorted.sort((a, b) => (a.full_name || a.abbreviation).localeCompare(b.full_name || b.abbreviation));
    }

    const top = sorted.slice(0, showTop);
    const others = sorted.slice(showTop);
    const othersTotal = others.reduce((sum, p) => sum + p.usage_count, 0);

    const labels = top.map(p => p.full_name || p.abbreviation);
    const values = top.map(p => p.usage_count);
    const colors = top.map((_, i) => colorPalette[i % colorPalette.length]);

    if (showOthers && othersTotal > 0) {
      labels.push(`Others (${others.length})`);
      values.push(othersTotal);
      colors.push('#6b7280');
    }

    return { labels, values, colors };
  });

  // Statistics
  let stats = $derived(() => {
    if (passengers.length === 0) return null;

    const totalFlights = passengers.reduce((sum, p) => sum + p.usage_count, 0);
    const avgFlightsPerPassenger = totalFlights / passengers.length;
    const topPassenger = [...passengers].sort((a, b) => b.usage_count - a.usage_count)[0];

    return {
      totalPassengers: passengers.length,
      totalFlights,
      avgFlightsPerPassenger: avgFlightsPerPassenger.toFixed(1),
      topPassenger: topPassenger?.full_name || topPassenger?.abbreviation || 'N/A',
      topPassengerFlights: topPassenger?.usage_count || 0
    };
  });

  async function loadData() {
    loading = true;
    error = null;

    try {
      const result = await invoke<PassengerName[]>('get_all_passenger_names', {
        userId
      });

      passengers = Array.isArray(result) ? result : [];

      if (passengers.length === 0) {
        error = 'No passenger data available';
      }
    } catch (err) {
      console.error('Failed to load passenger data:', err);
      error = `Failed to load data: ${err}`;
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

      const data = chartData();
      if (data.labels.length === 0) {
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

      chart = new Chart(ctx, {
        type: 'pie',
        data: {
          labels: data.labels,
          datasets: [{
            data: data.values,
            backgroundColor: data.colors,
            borderColor: '#1f2937',
            borderWidth: 2,
            hoverOffset: 8
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            title: {
              display: true,
              text: 'Passenger Flight Distribution',
              font: { size: 16, weight: 'bold' }
            },
            legend: {
              display: true,
              position: 'right',
              labels: {
                padding: 15,
                usePointStyle: true,
                font: { size: 11 }
              }
            },
            tooltip: {
              callbacks: {
                label: (ctx) => {
                  const total = data.values.reduce((a, b) => a + b, 0);
                  const percentage = ((ctx.parsed / total) * 100).toFixed(1);
                  return `${ctx.label}: ${ctx.parsed} flights (${percentage}%)`;
                }
              }
            }
          }
        }
      });

      error = null;
    } catch (err) {
      console.error('Error creating pie chart:', err);
      error = `Failed to create chart: ${err}`;
    }
  }

  // Export functions
  async function exportCSV() {
    if (passengers.length === 0) return;

    const headers = ['Name', 'Abbreviation', 'Flight Count'];
    const sorted = [...passengers].sort((a, b) => b.usage_count - a.usage_count);
    const rows = sorted.map(p => [
      `"${p.full_name || ''}"`,
      p.abbreviation,
      p.usage_count
    ]);

    const csv = [headers.join(','), ...rows.map(r => r.join(','))].join('\n');

    try {
      const filePath = await save({
        defaultPath: `passenger-stats-${new Date().toISOString().split('T')[0]}.csv`,
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
        defaultPath: `passenger-distribution-${new Date().toISOString().split('T')[0]}.png`,
        filters: [{ name: 'PNG Image', extensions: ['png'] }]
      });

      if (filePath) {
        await invoke('write_base64_file', { path: filePath, base64Content: base64 });
      }
    } catch (err) {
      console.error('Export failed:', err);
    }
  }

  function handleShowTopChange(value: number) {
    showTop = value;
    if (canvas && passengers.length > 0) {
      createChart();
    }
  }

  function handleSortChange(value: 'flights' | 'name') {
    sortBy = value;
    if (canvas && passengers.length > 0) {
      createChart();
    }
  }

  function handleShowOthersToggle() {
    showOthers = !showOthers;
    if (canvas && passengers.length > 0) {
      createChart();
    }
  }

  // Track data changes
  let lastDataLength = $state(0);

  $effect(() => {
    const currentLength = passengers.length;
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
    <!-- Show Top N -->
    <div class="flex items-center gap-2">
      <span class="text-sm font-medium text-gray-600 dark:text-gray-400">Show Top:</span>
      <select
        value={showTop}
        onchange={(e) => handleShowTopChange(parseInt(e.currentTarget.value))}
        class="px-3 py-1.5 text-sm rounded-lg border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
      >
        {#each topOptions as opt}
          <option value={opt}>{opt}</option>
        {/each}
      </select>
    </div>

    <!-- Sort By -->
    <div class="flex items-center gap-2">
      <span class="text-sm font-medium text-gray-600 dark:text-gray-400">Sort:</span>
      <div class="flex rounded-lg overflow-hidden border border-gray-300 dark:border-gray-600">
        <button
          onclick={() => handleSortChange('flights')}
          class="px-3 py-1.5 text-sm font-medium transition {sortBy === 'flights'
            ? 'bg-blue-600 text-white'
            : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-600'}"
        >
          By Flights
        </button>
        <button
          onclick={() => handleSortChange('name')}
          class="px-3 py-1.5 text-sm font-medium transition {sortBy === 'name'
            ? 'bg-blue-600 text-white'
            : 'bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-600'}"
        >
          By Name
        </button>
      </div>
    </div>

    <!-- Show Others Toggle -->
    <label class="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        checked={showOthers}
        onchange={handleShowOthersToggle}
        class="w-4 h-4 rounded border-gray-300 dark:border-gray-600 text-blue-600 focus:ring-blue-500"
      />
      <span class="text-sm text-gray-600 dark:text-gray-400">Group others</span>
    </label>

    <!-- Spacer -->
    <div class="flex-1"></div>

    <!-- Export Buttons -->
    <div class="flex items-center gap-2">
      <button
        onclick={exportCSV}
        disabled={passengers.length === 0}
        class="flex items-center gap-1 px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed transition"
      >
        <span>📄</span> CSV
      </button>
      <button
        onclick={exportPNG}
        disabled={!canvas}
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
        <div class="text-2xl font-bold text-blue-600 dark:text-blue-400">{s?.totalPassengers}</div>
        <div class="text-xs text-blue-600/70 dark:text-blue-400/70">Total Passengers</div>
      </div>
      <div class="bg-green-50 dark:bg-green-900/20 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-green-600 dark:text-green-400">{s?.totalFlights}</div>
        <div class="text-xs text-green-600/70 dark:text-green-400/70">Total Flights</div>
      </div>
      <div class="bg-amber-50 dark:bg-amber-900/20 rounded-lg p-3 text-center">
        <div class="text-2xl font-bold text-amber-600 dark:text-amber-400">{s?.avgFlightsPerPassenger}</div>
        <div class="text-xs text-amber-600/70 dark:text-amber-400/70">Avg Flights/Person</div>
      </div>
      <div class="bg-purple-50 dark:bg-purple-900/20 rounded-lg p-3 text-center col-span-2 md:col-span-2">
        <div class="text-lg font-bold text-purple-600 dark:text-purple-400 truncate">{s?.topPassenger}</div>
        <div class="text-xs text-purple-600/70 dark:text-purple-400/70">Top Passenger ({s?.topPassengerFlights} flights)</div>
      </div>
    </div>
  {/if}

  <!-- Chart Area -->
  {#if loading}
    <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg">
      <div class="text-center">
        <div class="animate-spin rounded-full h-10 w-10 border-b-2 border-blue-600 mx-auto mb-3"></div>
        <p class="text-gray-500 dark:text-gray-400 text-sm">Loading passenger data...</p>
      </div>
    </div>
  {:else if error && passengers.length === 0}
    <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
      <div class="text-center">
        <div class="text-4xl mb-3">👥</div>
        <p class="text-gray-600 dark:text-gray-400">{error}</p>
        <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Add passengers to your flights to see distribution</p>
      </div>
    </div>
  {:else}
    <div class="h-[400px] bg-white dark:bg-gray-800 rounded-lg p-4">
      <canvas bind:this={canvas}></canvas>
    </div>
  {/if}

  <!-- Passenger Table -->
  {#if passengers.length > 0}
    <div class="bg-white dark:bg-gray-800 rounded-lg overflow-hidden border border-gray-200 dark:border-gray-700">
      <div class="px-4 py-3 bg-gray-50 dark:bg-gray-900 border-b border-gray-200 dark:border-gray-700">
        <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300">All Passengers</h3>
      </div>
      <div class="max-h-[300px] overflow-y-auto">
        <table class="w-full text-sm">
          <thead class="sticky top-0 bg-gray-50 dark:bg-gray-900">
            <tr class="text-left text-xs uppercase text-gray-500 dark:text-gray-400">
              <th class="px-4 py-2">#</th>
              <th class="px-4 py-2">Name</th>
              <th class="px-4 py-2">Abbreviation</th>
              <th class="px-4 py-2 text-right">Flights</th>
              <th class="px-4 py-2 text-right">Share</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
            {#each [...passengers].sort((a, b) => b.usage_count - a.usage_count) as passenger, i}
              {@const totalFlights = passengers.reduce((sum, p) => sum + p.usage_count, 0)}
              {@const share = ((passenger.usage_count / totalFlights) * 100).toFixed(1)}
              <tr class="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                <td class="px-4 py-2 text-gray-500">{i + 1}</td>
                <td class="px-4 py-2 text-gray-900 dark:text-gray-100 font-medium">
                  {passenger.full_name || '-'}
                </td>
                <td class="px-4 py-2 font-mono text-blue-600 dark:text-blue-400">{passenger.abbreviation}</td>
                <td class="px-4 py-2 text-right font-semibold">{passenger.usage_count}</td>
                <td class="px-4 py-2 text-right text-gray-500">{share}%</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    </div>
  {/if}
</div>
