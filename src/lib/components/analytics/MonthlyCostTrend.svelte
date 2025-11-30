<!-- MonthlyCostTrend.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import type { MonthlyCostData } from '$lib/types/analytics';

  interface Props {
    data?: MonthlyCostData[];
  }

  let { data = [] }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);
  let error = $state<string | null>(null);
  // Safely validate data
  let validData = $derived(() => {
    try {
      if (!Array.isArray(data) || data.length === 0) return null;
      const filtered = data.filter(d => d && d.period && typeof d.total_cost === 'number');
      if (filtered.length === 0) return null;
      return filtered;
    } catch (err) {
      console.error('Error validating cost trend data:', err);
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
        error = 'No monthly cost data available';
        return;
      }

      // Destroy existing chart
      if (chart) {
        chart.destroy();
        chart = null;
      }

      chart = new Chart(ctx, {
        type: 'line',
        data: {
          labels: safeData.map(d => d.period),
          datasets: [
            {
              label: `Total Cost (${safeData[0]?.currency || 'USD'})`,
              data: safeData.map(d => d.total_cost || 0),
              borderColor: '#ef4444',
              backgroundColor: 'rgba(239, 68, 68, 0.1)',
              fill: true,
              tension: 0.4,
              borderWidth: 3,
              yAxisID: 'y'
            },
            {
              label: 'Cost per Hour',
              data: safeData.map(d => d.cost_per_hour || 0),
              borderColor: '#f59e0b',
              backgroundColor: 'rgba(245, 158, 11, 0.1)',
              fill: false,
              tension: 0.4,
              borderWidth: 3,
              yAxisID: 'y1'
            }
          ]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            title: {
              display: true,
              text: 'Monthly Operating Cost Trend',
              font: { size: 16, weight: 'bold' }
            },
            tooltip: {
              callbacks: {
                afterLabel: (ctx) => {
                  const hours = (safeData[ctx.dataIndex].total_hours || 0).toFixed(1);
                  return `Total Hours: ${hours}`;
                }
              }
            }
          },
          scales: {
            y: {
              beginAtZero: true,
              position: 'left',
              title: { display: true, text: 'Total Cost' }
            },
            y1: {
              beginAtZero: true,
              position: 'right',
              title: { display: true, text: 'Cost/Hour' },
              grid: { drawOnChartArea: false }
            }
          }
        }
      });

      error = null;
    } catch (err) {
      console.error('Error creating monthly cost trend chart:', err);
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
      <div class="text-4xl mb-3">📈</div>
      <p class="text-gray-600 dark:text-gray-400">No monthly cost data available</p>
      <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Add cost entries to see trends</p>
    </div>
  </div>
{:else}
  <div class="h-[400px]">
    <canvas bind:this={canvas}></canvas>
  </div>
{/if}
