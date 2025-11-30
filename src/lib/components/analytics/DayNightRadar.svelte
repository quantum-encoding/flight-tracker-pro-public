<!-- DayNightRadar.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import type { DayNightStats } from '$lib/types/analytics';

  interface Props {
    stats?: DayNightStats;
  }

  let { stats }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);
  let error = $state<string | null>(null);
  // Safely validate stats
  let validStats = $derived(() => {
    try {
      if (!stats) return null;
      if (typeof stats.total_day_flights !== 'number' || typeof stats.total_night_flights !== 'number') return null;
      return stats;
    } catch (err) {
      console.error('Error validating day/night stats:', err);
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

      const safeStats = validStats();
      if (!safeStats) {
        error = 'No day/night statistics available';
        return;
      }

      // Destroy existing chart
      if (chart) {
        chart.destroy();
        chart = null;
      }

      chart = new Chart(ctx, {
        type: 'radar',
        data: {
          labels: ['Day Flights', 'Night Flights', 'Day Hours', 'Night Hours', 'Day Landings', 'Night Landings'],
          datasets: [{
            label: 'Day (Sun)',
            data: [safeStats.total_day_flights || 0, 0, safeStats.day_hours || 0, 0, safeStats.day_landings || 0, 0],
            backgroundColor: 'rgba(251,191,36,0.3)',
            borderColor: '#f59e0b',
            borderWidth: 2
          }, {
            label: 'Night (Moon)',
            data: [0, safeStats.total_night_flights || 0, 0, safeStats.night_hours || 0, 0, safeStats.night_landings || 0],
            backgroundColor: 'rgba(99,102,241,0.3)',
            borderColor: '#6366f1',
            borderWidth: 2
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            title: {
              display: true,
              text: 'Day vs Night Flying',
              font: { size: 16, weight: 'bold' }
            }
          }
        }
      });

      error = null;
    } catch (err) {
      console.error('Error creating day/night radar chart:', err);
      error = `Failed to create chart: ${err}`;
    }
  }

  // Track if we've created the chart to avoid infinite loops
  let chartCreated = $state(false);

  // React to stats changes - critical since data is loaded async after mount
  $effect(() => {
    if (stats && canvas && !chartCreated) {
      chartCreated = true;
      setTimeout(() => createChart(), 0);
    }
  });

  onMount(() => {
    // Initial check - may not have stats yet
    if (stats) {
      chartCreated = true;
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
{:else if !stats}
  <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
    <div class="text-center">
      <div class="text-4xl mb-3">🌓</div>
      <p class="text-gray-600 dark:text-gray-400">No day/night statistics available</p>
      <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Add flights to see day vs night analysis</p>
    </div>
  </div>
{:else}
  <div class="h-[400px]">
    <canvas bind:this={canvas}></canvas>
  </div>
{/if}
