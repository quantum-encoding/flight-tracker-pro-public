<!-- src/lib/components/analytics/AircraftUtilizationChart.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import type { AircraftUtilization } from '$lib/types/analytics';

  interface Props {
    data?: AircraftUtilization[];
  }

  let { data = [] }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);
  let error = $state<string | null>(null);
  // Safely validate and transform data
  let validData = $derived(() => {
    try {
      if (!Array.isArray(data) || data.length === 0) return null;

      const filtered = data.filter(a => a && a.tail_number);
      if (filtered.length === 0) return null;

      const labels = filtered.map(a => a.tail_number);
      const hours = filtered.map(a => a.total_hours || 0);
      const flights = filtered.map(a => a.total_flights || 0);
      const idleColor = filtered.map(a => {
        const days = a.days_since_last_flight ?? 0;
        return days > 90 ? 'rgba(239,68,68,0.8)' : days > 30 ? 'rgba(251,146,60,0.8)' : 'rgba(34,197,94,0.8)';
      });

      return { labels, hours, flights, idleColor };
    } catch (err) {
      console.error('Error validating aircraft data:', err);
      return null;
    }
  });

  function createChart() {
    if (!canvas) return;

    try {
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        error = 'Could not get canvas context';
        return;
      }

      const safeData = validData();
      if (!safeData) {
        error = 'No aircraft utilization data available';
        return;
      }

      // Destroy existing chart
      if (chart) {
        chart.destroy();
        chart = null;
      }

      chart = new Chart(ctx, {
        type: 'bar',
        data: {
          labels: safeData.labels,
          datasets: [{
            label: 'Flight Hours',
            data: safeData.hours,
            backgroundColor: safeData.idleColor,
            yAxisID: 'y'
          }, {
            label: 'Number of Flights',
            data: safeData.flights,
            type: 'line',
            borderColor: '#818cf8',
            backgroundColor: 'rgba(129,140,248,0.2)',
            borderWidth: 3,
            fill: true,
            tension: 0.4,
            yAxisID: 'y1'
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            title: {
              display: true,
              text: 'Aircraft Fleet Utilization',
              font: { size: 16, weight: 'bold' }
            },
            tooltip: {
              callbacks: {
                afterLabel: (ctx) => {
                  const aircraft = data[ctx.dataIndex];
                  const days = aircraft?.days_since_last_flight ?? 0;
                  return days > 0 ? `Last flight: ${days} days ago` : 'Active';
                }
              }
            }
          },
          scales: {
            y: {
              position: 'left',
              title: { display: true, text: 'Hours' },
              beginAtZero: true
            },
            y1: {
              position: 'right',
              title: { display: true, text: 'Flights' },
              grid: { drawOnChartArea: false },
              beginAtZero: true
            }
          }
        }
      });

      error = null;
    } catch (err) {
      console.error('Error creating aircraft utilization chart:', err);
      error = `Failed to create chart: ${err}`;
    }
  }

  // Track previous data length to avoid infinite loops
  let lastDataLength = $state(0);

  // React to data changes - critical since data is loaded async after mount
  $effect(() => {
    const len = data?.length ?? 0;
    if (len > 0 && canvas && len !== lastDataLength) {
      lastDataLength = len;
      setTimeout(() => createChart(), 0);
    }
  });

  onMount(() => {
    // Initial check - may not have data yet
    if (data && data.length > 0) {
      lastDataLength = data.length;
      createChart();
    }
  });
  onDestroy(() => {
    if (chart) {
      chart.destroy();
      chart = null;
    }
  });
</script>

{#if error}
  <div class="flex items-center justify-center h-[400px] bg-red-50 dark:bg-red-900/20 rounded-lg p-6">
    <div class="text-center">
      <p class="text-red-600 dark:text-red-400 font-semibold mb-2">Chart Error</p>
      <p class="text-red-500 dark:text-red-300 text-sm">{error}</p>
    </div>
  </div>
{:else if !data || data.length === 0}
  <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
    <div class="text-center">
      <div class="text-4xl mb-3">✈️</div>
      <p class="text-gray-600 dark:text-gray-400">No aircraft utilization data available</p>
      <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Add aircraft and flights to see utilization</p>
    </div>
  </div>
{:else}
  <div class="h-[400px]">
    <canvas bind:this={canvas}></canvas>
  </div>
{/if}
