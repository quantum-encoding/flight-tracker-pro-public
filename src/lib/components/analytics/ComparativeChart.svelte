<!-- src/lib/components/analytics/ComparativeChart.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import type { PassengerMetrics } from '$lib/types/analytics';

  interface Props {
    data?: PassengerMetrics[];
    metric?: 'flights' | 'distance' | 'co2' | 'hours';
  }

  let { data = [], metric = 'distance' }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);
  let error = $state<string | null>(null);
  const metricLabels = {
    flights: 'Total Flights',
    distance: 'Total Distance (km)',
    co2: 'Total CO₂ (kg)',
    hours: 'Total Flight Hours'
  };

  // Safely validate data and extract values
  let validData = $derived(() => {
    try {
      if (!Array.isArray(data) || data.length === 0) return null;

      const filtered = data.filter(p => p && (p.full_name || p.abbreviation));
      if (filtered.length === 0) return null;

      const values = filtered.map(p => {
        switch (metric) {
          case 'flights': return p.total_flights || 0;
          case 'distance': return p.total_distance_km || 0;
          case 'co2': return p.total_co2_kg || 0;
          case 'hours': return p.total_flight_hours || 0;
        }
      });

      return {
        labels: filtered.map(p => p.full_name || p.abbreviation),
        values,
        passengers: filtered
      };
    } catch (err) {
      console.error('Error validating passenger data:', err);
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
        error = 'No passenger comparison data available';
        return;
      }

      // Destroy existing chart
      if (chart) {
        chart.destroy();
        chart = null;
      }

      const label = metricLabels[metric];

      chart = new Chart(ctx, {
        type: 'bar',
        data: {
          labels: safeData.labels,
          datasets: [{
            label,
            data: safeData.values,
            backgroundColor: 'rgba(139, 92, 246, 0.6)',
            borderColor: 'rgb(139, 92, 246)',
            borderWidth: 1
          }]
        },
        options: {
          indexAxis: 'y',
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            title: {
              display: true,
              text: `Passenger Comparison – ${label}`,
              font: { size: 16, weight: 'bold' }
            },
            tooltip: {
              callbacks: {
                afterLabel: (ctx) => {
                  const p = safeData.passengers[ctx.dataIndex];
                  return metric === 'distance'
                    ? `Avg: ${(p.avg_flight_distance_km || 0).toFixed(0)} km`
                    : `Unique airports: ${p.unique_airports || 0}`;
                }
              }
            }
          },
          scales: {
            x: { title: { display: true, text: label } }
          }
        }
      });

      error = null;
    } catch (err) {
      console.error('Error creating comparative chart:', err);
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
      <div class="text-4xl mb-3">📊</div>
      <p class="text-gray-600 dark:text-gray-400">No passenger data available</p>
      <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Add passengers to compare metrics</p>
    </div>
  </div>
{:else}
  <div class="h-[400px]">
    <canvas bind:this={canvas}></canvas>
  </div>
{/if}
