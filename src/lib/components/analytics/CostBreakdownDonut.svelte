<!-- CostBreakdownDonut.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import type { CostBreakdown } from '$lib/types/analytics';

  interface Props {
    data?: CostBreakdown[];
  }

  let { data = [] }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);
  let error = $state<string | null>(null);
  const colors = ['#3b82f6', '#8b5cf6', '#ec4899', '#f59e0b', '#10b981', '#6b7280'];

  // Safely validate data
  let validData = $derived(() => {
    try {
      if (!Array.isArray(data) || data.length === 0) return null;
      const filtered = data.filter(c => c && c.category && typeof c.total_cost === 'number');
      if (filtered.length === 0) return null;
      return filtered;
    } catch (err) {
      console.error('Error validating cost data:', err);
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
        error = 'No cost breakdown data available';
        return;
      }

      // Destroy existing chart
      if (chart) {
        chart.destroy();
        chart = null;
      }

      const total = safeData.reduce((sum, c) => sum + (c.total_cost || 0), 0);

      chart = new Chart(ctx, {
        type: 'doughnut',
        data: {
          labels: safeData.map(c => (c.category || 'Unknown').charAt(0).toUpperCase() + (c.category || 'Unknown').slice(1)),
          datasets: [{
            data: safeData.map(c => c.total_cost || 0),
            backgroundColor: colors
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            title: {
              display: true,
              text: 'Operating Cost Breakdown',
              font: { size: 16, weight: 'bold' }
            },
            tooltip: {
              callbacks: {
                label: ctx => {
                  const cost = safeData[ctx.dataIndex].total_cost || 0;
                  const percentage = total > 0 ? (cost / total * 100).toFixed(1) : '0.0';
                  const currency = safeData[ctx.dataIndex].currency || 'USD';
                  return `${ctx.label}: ${cost.toFixed(0)} ${currency} (${percentage}%)`;
                }
              }
            }
          }
        }
      });

      error = null;
    } catch (err) {
      console.error('Error creating cost breakdown chart:', err);
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
      <div class="text-4xl mb-3">💰</div>
      <p class="text-gray-600 dark:text-gray-400">No cost breakdown data available</p>
      <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Add cost entries to see breakdown</p>
    </div>
  </div>
{:else}
  <div class="h-[400px]">
    <canvas bind:this={canvas}></canvas>
  </div>
{/if}
