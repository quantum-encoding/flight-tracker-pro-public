<!-- RunwayRiskTreemap.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import { TreemapController, TreemapElement } from 'chartjs-chart-treemap';
  import type { RunwayRiskData } from '$lib/types/analytics';

  // Register the treemap plugin
  Chart.register(TreemapController, TreemapElement);

  interface Props {
    data?: RunwayRiskData[];
  }

  let { data = [] }: Props = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let chart = $state<Chart | null>(null);
  let error = $state<string | null>(null);
  const riskColor = (level: string) => {
    switch(level) {
      case 'safe': return 'rgba(34,197,94,0.8)';
      case 'marginal': return 'rgba(251,191,36,0.8)';
      case 'watch_out': return 'rgba(251,146,60,0.8)';
      case 'nope': return 'rgba(239,68,68,0.8)';
      default: return '#6b7280';
    }
  };

  // Safely validate data
  let validData = $derived(() => {
    try {
      if (!Array.isArray(data) || data.length === 0) return null;
      const filtered = data.filter(a => a && a.airport_code && typeof a.visits === 'number');
      if (filtered.length === 0) return null;
      return filtered;
    } catch (err) {
      console.error('Error validating runway risk data:', err);
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
        error = 'No runway risk data available';
        return;
      }

      // Destroy existing chart
      if (chart) {
        chart.destroy();
        chart = null;
      }

      chart = new Chart(ctx, {
        type: 'treemap',
        data: {
          datasets: [{
            tree: safeData.map(a => ({
              id: a.airport_code,
              label: `${a.airport_code}\n${a.visits} visits\n${a.runway_length_ft || 'Unknown'}ft`,
              value: a.visits || 1,
              risk: a
            })),
            backgroundColor: (ctx: any) => {
              if (!ctx.raw) return '#6b7280';
              return riskColor((ctx.raw as any).risk?.risk_level || 'unknown');
            },
            borderWidth: 2,
            borderColor: '#fff',
            spacing: -0.5
          } as any]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: {
            title: {
              display: true,
              text: 'Runway Length Risk Heatmap',
              font: { size: 16, weight: 'bold' }
            },
            tooltip: {
              callbacks: {
                label: (ctx: any) => {
                  const a = ctx.raw.risk as RunwayRiskData;
                  return [
                    `${a.airport_code}`,
                    `Visits: ${a.visits}`,
                    `Runway: ${a.runway_length_ft || 'Unknown'} ft`,
                    `Risk: ${a.risk_level || 'unknown'}`
                  ];
                }
              }
            }
          }
        }
      });

      error = null;
    } catch (err) {
      console.error('Error creating runway risk treemap:', err);
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
      <div class="text-4xl mb-3">🛬</div>
      <p class="text-gray-600 dark:text-gray-400">No runway risk data available</p>
      <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Add flights to airports to see runway analysis</p>
    </div>
  </div>
{:else}
  <div class="h-[400px]">
    <canvas bind:this={canvas}></canvas>
  </div>
{/if}
