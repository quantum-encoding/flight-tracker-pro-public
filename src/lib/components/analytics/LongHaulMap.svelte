<!-- LongHaulMap.svelte -->
<script lang="ts">
  import type { LongHaulFlight } from '$lib/types/analytics';

  interface Props {
    flights?: LongHaulFlight[];
  }

  let { flights = [] }: Props = $props();

  function formatDuration(minutes: number | null): string {
    if (!minutes) return 'N/A';
    const hours = Math.floor(minutes / 60);
    const mins = Math.round(minutes % 60);
    return `${hours}h ${mins}m`;
  }

  function formatDate(datetime: string): string {
    try {
      return new Date(datetime).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' });
    } catch {
      return 'Invalid Date';
    }
  }
</script>

{#if flights.length === 0}
  <div class="flex items-center justify-center h-[400px] bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
    <div class="text-center">
      <div class="text-4xl mb-3">🌍</div>
      <p class="text-gray-600 dark:text-gray-400">No long haul flights found</p>
      <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">Flights over 1,000 km will appear here</p>
    </div>
  </div>
{:else}
  <div class="w-full overflow-auto bg-white dark:bg-gray-800 rounded-lg shadow">
    <table class="w-full text-sm">
      <thead class="bg-gray-100 dark:bg-gray-700">
        <tr>
          <th class="px-3 py-2 text-left font-semibold">Route</th>
          <th class="px-3 py-2 text-right font-semibold">Distance</th>
          <th class="px-3 py-2 text-right font-semibold">Duration</th>
          <th class="px-3 py-2 text-left font-semibold">Date</th>
          <th class="px-3 py-2 text-left font-semibold">Aircraft</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-gray-200 dark:divide-gray-600">
        {#each flights as flight}
          <tr class="hover:bg-gray-50 dark:hover:bg-gray-800 transition">
            <td class="px-3 py-2 font-mono font-semibold">
              {flight.departure_airport || 'Unknown'} → {flight.arrival_airport || 'Unknown'}
            </td>
            <td class="px-3 py-2 text-right text-blue-600 dark:text-blue-400 font-bold">
              {Math.round(flight.distance_km || 0).toLocaleString()} km
            </td>
            <td class="px-3 py-2 text-right">
              {formatDuration(flight.flight_duration)}
            </td>
            <td class="px-3 py-2 text-gray-600 dark:text-gray-400">
              {formatDate(flight.departure_datetime)}
            </td>
            <td class="px-3 py-2 text-gray-600 dark:text-gray-400">
              {flight.aircraft_type || 'Unknown'}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}
