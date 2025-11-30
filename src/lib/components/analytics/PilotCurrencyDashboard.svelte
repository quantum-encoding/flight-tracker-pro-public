<!-- PilotCurrencyDashboard.svelte -->
<script lang="ts">
  import type { CurrencyItem } from '$lib/types/analytics';

  interface Props {
    items?: CurrencyItem[];
  }

  let { items = [] }: Props = $props();

  const color = (s: string) => s === 'good' ? 'bg-green-500' : s === 'warning' ? 'bg-yellow-500' : 'bg-red-500';
  const percentage = (current: number, required: number) => {
    if (!required || required === 0) return 0;
    return Math.min(100, (current / required * 100));
  };
</script>

{#if items.length === 0}
  <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
    <div class="text-center">
      <div class="text-4xl mb-3">📋</div>
      <p class="text-gray-600 dark:text-gray-400">No currency requirements tracked</p>
      <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Add pilot currency requirements to monitor status</p>
    </div>
  </div>
{:else}
  <div class="space-y-4 p-4">
    {#each items as item}
      <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-4 border border-gray-200 dark:border-gray-700">
        <div class="flex justify-between items-center mb-2">
          <h3 class="font-semibold text-gray-900 dark:text-gray-100">{item.requirement || 'Unknown Requirement'}</h3>
          <span class="text-sm {item.expires_in_days !== null && item.expires_in_days < 30 ? 'text-red-500 dark:text-red-400 font-semibold' : 'text-gray-600 dark:text-gray-400'}">
            {item.expires_in_days !== null ? `${item.expires_in_days} days left` : 'Never expires'}
          </span>
        </div>
        <div class="w-full bg-gray-300 dark:bg-gray-700 rounded-full h-6 overflow-hidden">
          <div
            class="{color(item.status || 'unknown')} h-6 rounded-full transition-all duration-300"
            style="width: {percentage(item.current_count || 0, item.required || 1).toFixed(0)}%"
          ></div>
        </div>
        <p class="text-sm mt-2 text-gray-700 dark:text-gray-300">
          {item.current_count || 0} / {item.required || 0}
          {#if item.current_count >= item.required}
            <span class="ml-2 text-green-600 dark:text-green-400 font-semibold">✓ Current</span>
          {:else if item.status === 'warning'}
            <span class="ml-2 text-yellow-600 dark:text-yellow-400 font-semibold">⚠ Warning</span>
          {:else if item.status === 'expired'}
            <span class="ml-2 text-red-600 dark:text-red-400 font-semibold">✗ Expired</span>
          {/if}
        </p>
      </div>
    {/each}
  </div>
{/if}
