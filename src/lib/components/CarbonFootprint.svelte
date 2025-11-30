<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  // Stats and data
  let calculationStats = $state<any>(null);
  let passengerStats = $state<any[]>([]);
  let loading = $state(true);
  let calculating = $state(false);
  let calculationResult = $state<any>(null);
  let totalCO2 = $state(0);
  let totalFlights = $state(0);
  let avgCO2PerFlight = $state(0);

  // Progress tracking
  interface ProgressUpdate {
    phase: string;
    current: number;
    total: number;
    updated: number;
    skipped: number;
    failed: number;
    complete: boolean;
  }
  let progress = $state<ProgressUpdate | null>(null);
  let unlistenProgress: (() => void) | null = null;

  // Fun offset equivalents
  let treesNeeded = $derived(Math.ceil(totalCO2 / 21)); // ~21kg CO2 absorbed per tree per year
  let burgerEquivalent = $derived(Math.ceil(totalCO2 / 3.6)); // ~3.6kg CO2 per beef burger
  let carMiles = $derived(Math.ceil(totalCO2 / 0.41)); // ~0.41kg CO2 per mile driven
  let netflixHours = $derived(Math.ceil(totalCO2 / 0.036)); // ~36g CO2 per hour of streaming
  let bitcoinTransactions = $derived(Math.ceil(totalCO2 / 400)); // ~400kg CO2 per BTC transaction

  // Top CO2 offenders (passengers)
  let topOffenders = $state<any[]>([]);

  onMount(async () => {
    await loadData();

    // Listen for batch progress events
    unlistenProgress = await listen<ProgressUpdate>('batch-progress', (event) => {
      progress = event.payload;
    });
  });

  onDestroy(() => {
    if (unlistenProgress) {
      unlistenProgress();
    }
  });

  async function loadData() {
    loading = true;
    try {
      // Get calculation stats
      calculationStats = await invoke('get_calculation_stats', { userId });

      // Get flight statistics for total CO2
      const stats: any = await invoke('get_statistics', { userId });
      totalCO2 = stats.total_carbon_kg || 0;
      totalFlights = stats.total_flights || 0;
      avgCO2PerFlight = totalFlights > 0 ? totalCO2 / totalFlights : 0;

      // Get comparative metrics to find top CO2 passengers
      try {
        const metrics: any = await invoke('get_comparative_metrics', {
          request: {
            user_id: userId,
            rank_by: 'co2',
            limit: 10,
            start_date: null,
            end_date: null
          }
        });
        topOffenders = metrics || [];
      } catch (e) {
        console.warn('Could not load comparative metrics:', e);
      }
    } catch (e) {
      console.error('Failed to load carbon data:', e);
    } finally {
      loading = false;
    }
  }

  async function runBatchCalculations() {
    calculating = true;
    calculationResult = null;
    progress = null;
    try {
      // Use streaming version with progress events
      const result = await invoke('batch_calculate_streaming', {
        userId,
        batchSize: 25  // Emit progress every 25 flights
      });
      calculationResult = result;
      // Reload data after calculation
      await loadData();
    } catch (e) {
      console.error('Batch calculation failed:', e);
      calculationResult = { error: String(e) };
    } finally {
      calculating = false;
      progress = null;
    }
  }

  // Format large numbers
  function formatNumber(n: number): string {
    if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
    if (n >= 1000) return (n / 1000).toFixed(1) + 'K';
    return n.toFixed(0);
  }

  // Get a sarcastic eco-rating
  function getEcoRating(co2: number): { rating: string; emoji: string; message: string } {
    if (co2 === 0) return { rating: 'Pristine', emoji: '🌱', message: "Either you don't fly, or we haven't calculated yet!" };
    if (co2 < 1000) return { rating: 'Eco Warrior', emoji: '🌿', message: 'Greta would almost approve.' };
    if (co2 < 5000) return { rating: 'Weekend Warrior', emoji: '🌲', message: 'A few trees shed a tear.' };
    if (co2 < 20000) return { rating: 'Frequent Flyer', emoji: '🔥', message: 'The polar bears have noticed you.' };
    if (co2 < 100000) return { rating: 'Climate Enthusiast', emoji: '☠️', message: 'Captain Planet is drafting a cease & desist.' };
    if (co2 < 500000) return { rating: 'Carbon Baron', emoji: '💀', message: 'You personally melted a glacier. Congrats.' };
    return { rating: 'Extinction Event', emoji: '🌋', message: 'The dinosaurs called. They want their asteroid back.' };
  }

  const ecoRating = $derived(getEcoRating(totalCO2));
</script>

<div class="space-y-6">
  <!-- Header -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white flex items-center gap-2">
          <span class="text-3xl">{ecoRating.emoji}</span>
          Carbon Footprint Dashboard
        </h2>
        <p class="text-gray-500 dark:text-gray-400 mt-1">
          Track your environmental impact (and feel appropriately guilty)
        </p>
      </div>
      <div class="flex flex-col items-end gap-2">
        <button
          onclick={runBatchCalculations}
          disabled={calculating}
          class="px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-green-800 text-white rounded-lg font-medium transition flex items-center gap-2"
        >
          {#if calculating}
            <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            {#if progress}
              {progress.phase === 'distance' ? 'Calculating Distances...' : 'Calculating CO2...'}
            {:else}
              Starting...
            {/if}
          {:else}
            🔄 Recalculate All
          {/if}
        </button>

        <!-- Progress Bar -->
        {#if calculating && progress}
          <div class="w-64">
            <div class="flex justify-between text-xs text-gray-500 dark:text-gray-400 mb-1">
              <span class="capitalize">{progress.phase}</span>
              <span>{progress.current} / {progress.total}</span>
            </div>
            <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2 overflow-hidden">
              <div
                class="h-full rounded-full transition-all duration-300 {progress.phase === 'distance' ? 'bg-blue-500' : 'bg-green-500'}"
                style="width: {progress.total > 0 ? (progress.current / progress.total) * 100 : 0}%"
              ></div>
            </div>
            <div class="flex justify-between text-xs text-gray-400 mt-1">
              <span class="text-green-500">✓ {progress.updated}</span>
              <span class="text-yellow-500">⏭ {progress.skipped}</span>
              {#if progress.failed > 0}
                <span class="text-red-500">✗ {progress.failed}</span>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-green-600"></div>
    </div>
  {:else}
    <!-- Eco Rating Card -->
    <div class="bg-gradient-to-r from-green-500 to-emerald-600 dark:from-green-600 dark:to-emerald-700 rounded-lg shadow p-6 text-white">
      <div class="flex items-center justify-between">
        <div>
          <p class="text-green-100 text-sm uppercase tracking-wide">Your Eco Rating</p>
          <h3 class="text-3xl font-bold mt-1">{ecoRating.rating}</h3>
          <p class="text-green-100 mt-2 italic">"{ecoRating.message}"</p>
        </div>
        <div class="text-6xl">{ecoRating.emoji}</div>
      </div>
    </div>

    <!-- Main Stats Grid -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
      <!-- Total CO2 -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-gray-500 dark:text-gray-400 text-sm">Total CO2 Emissions</p>
            <p class="text-3xl font-bold text-gray-900 dark:text-white mt-1">
              {formatNumber(totalCO2)} kg
            </p>
            <p class="text-gray-400 text-xs mt-1">
              ({(totalCO2 / 1000).toFixed(2)} tonnes)
            </p>
          </div>
          <div class="text-4xl">💨</div>
        </div>
      </div>

      <!-- Average per Flight -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-gray-500 dark:text-gray-400 text-sm">Avg CO2 per Flight</p>
            <p class="text-3xl font-bold text-gray-900 dark:text-white mt-1">
              {formatNumber(avgCO2PerFlight)} kg
            </p>
            <p class="text-gray-400 text-xs mt-1">
              Across {totalFlights} flights
            </p>
          </div>
          <div class="text-4xl">✈️</div>
        </div>
      </div>

      <!-- Calculation Coverage -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-gray-500 dark:text-gray-400 text-sm">Calculation Coverage</p>
            <p class="text-3xl font-bold text-gray-900 dark:text-white mt-1">
              {calculationStats ? Math.round((calculationStats.flights_with_co2 / calculationStats.total_flights) * 100) : 0}%
            </p>
            <p class="text-gray-400 text-xs mt-1">
              {calculationStats?.flights_with_co2 || 0} / {calculationStats?.total_flights || 0} flights
            </p>
          </div>
          <div class="text-4xl">📊</div>
        </div>
      </div>
    </div>

    <!-- Fun Equivalents -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
        🎭 Your Carbon Footprint Expressed As...
      </h3>
      <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
        <div class="text-center p-4 bg-green-50 dark:bg-green-900/20 rounded-lg">
          <div class="text-3xl mb-2">🌳</div>
          <p class="text-2xl font-bold text-green-600 dark:text-green-400">{formatNumber(treesNeeded)}</p>
          <p class="text-xs text-gray-500 dark:text-gray-400">Trees needed to offset (per year)</p>
        </div>
        <div class="text-center p-4 bg-red-50 dark:bg-red-900/20 rounded-lg">
          <div class="text-3xl mb-2">🍔</div>
          <p class="text-2xl font-bold text-red-600 dark:text-red-400">{formatNumber(burgerEquivalent)}</p>
          <p class="text-xs text-gray-500 dark:text-gray-400">Beef burgers equivalent</p>
        </div>
        <div class="text-center p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
          <div class="text-3xl mb-2">🚗</div>
          <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{formatNumber(carMiles)}</p>
          <p class="text-xs text-gray-500 dark:text-gray-400">Miles driven in a car</p>
        </div>
        <div class="text-center p-4 bg-purple-50 dark:bg-purple-900/20 rounded-lg">
          <div class="text-3xl mb-2">📺</div>
          <p class="text-2xl font-bold text-purple-600 dark:text-purple-400">{formatNumber(netflixHours)}</p>
          <p class="text-xs text-gray-500 dark:text-gray-400">Hours of Netflix streaming</p>
        </div>
        <div class="text-center p-4 bg-orange-50 dark:bg-orange-900/20 rounded-lg">
          <div class="text-3xl mb-2">₿</div>
          <p class="text-2xl font-bold text-orange-600 dark:text-orange-400">{formatNumber(bitcoinTransactions)}</p>
          <p class="text-xs text-gray-500 dark:text-gray-400">Bitcoin transactions</p>
        </div>
      </div>
    </div>

    <!-- How It's Calculated -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
        🧮 How We Calculate Your Carbon Guilt
      </h3>
      <div class="space-y-4">
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4 font-mono text-sm">
          <p class="text-gray-600 dark:text-gray-300 mb-2">// The Haversine Formula (Great Circle Distance)</p>
          <code class="text-green-600 dark:text-green-400">
            Distance (km) = 2 × R × arcsin(√(sin²(Δlat/2) + cos(lat1) × cos(lat2) × sin²(Δlon/2)))
          </code>
          <p class="text-gray-500 dark:text-gray-400 mt-2 text-xs">Where R = 6,371 km (Earth's radius)</p>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <h4 class="font-semibold text-gray-800 dark:text-gray-200 mb-2">📐 Step 1: Distance</h4>
            <p class="text-sm text-gray-600 dark:text-gray-400">
              We calculate the great-circle distance between airports using their coordinates.
              This is the shortest path over Earth's surface - like how a plane actually flies!
            </p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <h4 class="font-semibold text-gray-800 dark:text-gray-200 mb-2">⛽ Step 2: Fuel Burn</h4>
            <p class="text-sm text-gray-600 dark:text-gray-400">
              Different aircraft burn fuel at different rates. A 747 guzzles ~12 kg/km while a
              Cessna sips ~0.4 kg/km. We add 10% overhead for takeoff/landing.
            </p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <h4 class="font-semibold text-gray-800 dark:text-gray-200 mb-2">🔥 Step 3: CO2 Conversion</h4>
            <p class="text-sm text-gray-600 dark:text-gray-400">
              <span class="font-mono bg-gray-200 dark:bg-gray-700 px-1 rounded">1 kg Jet-A1 fuel = 3.15 kg CO2</span>
              <br/>This is the ICAO standard emission factor for aviation fuel.
            </p>
          </div>
          <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
            <h4 class="font-semibold text-gray-800 dark:text-gray-200 mb-2">👥 Step 4: Per Passenger</h4>
            <p class="text-sm text-gray-600 dark:text-gray-400">
              <span class="font-mono bg-gray-200 dark:bg-gray-700 px-1 rounded">Per Passenger CO2 = Total CO2 ÷ Passengers</span>
              <br/>Crew excluded - they're tools, like the plane itself! 🛠️
            </p>
          </div>
        </div>

        <div class="bg-blue-50 dark:bg-blue-900/30 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
          <h4 class="font-semibold text-blue-800 dark:text-blue-200 mb-2">🛩️ Aircraft Fuel Burn Rates</h4>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs">
            <div class="bg-white dark:bg-gray-800 rounded p-2">
              <p class="font-semibold">747 / A380</p>
              <p class="text-gray-500">~12 kg/km</p>
            </div>
            <div class="bg-white dark:bg-gray-800 rounded p-2">
              <p class="font-semibold">777 / 787</p>
              <p class="text-gray-500">~8 kg/km</p>
            </div>
            <div class="bg-white dark:bg-gray-800 rounded p-2">
              <p class="font-semibold">737 / A320</p>
              <p class="text-gray-500">~3.5 kg/km</p>
            </div>
            <div class="bg-white dark:bg-gray-800 rounded p-2">
              <p class="font-semibold">Gulfstream / Bizjet</p>
              <p class="text-gray-500">~1.8 kg/km</p>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Top CO2 Offenders -->
    {#if topOffenders.length > 0}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          🏆 Hall of Shame: Top CO2 Offenders
        </h3>
        <p class="text-gray-500 dark:text-gray-400 text-sm mb-4">
          These passengers have contributed the most to your carbon footprint. Send them a tree. Or a strongly-worded letter.
        </p>
        <div class="overflow-x-auto">
          <table class="min-w-full">
            <thead>
              <tr class="text-left text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider border-b dark:border-gray-700">
                <th class="pb-2 pr-4">Rank</th>
                <th class="pb-2 pr-4">Passenger</th>
                <th class="pb-2 pr-4">Total CO2</th>
                <th class="pb-2 pr-4">Flights</th>
                <th class="pb-2">Suggested Penance</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-100 dark:divide-gray-700">
              {#each topOffenders.slice(0, 10) as passenger, i}
                {@const penances = [
                  'Plant a forest',
                  'Give up beef for a year',
                  'Bike to work forever',
                  'Stop breathing (briefly)',
                  'Adopt a polar bear',
                  'Buy carbon credits',
                  'Write an apology to Earth',
                  'Hug a tree daily',
                  'Live in a cave',
                  'Invent cold fusion'
                ]}
                <tr class="text-sm">
                  <td class="py-3 pr-4">
                    {#if i === 0}
                      <span class="text-2xl">🥇</span>
                    {:else if i === 1}
                      <span class="text-2xl">🥈</span>
                    {:else if i === 2}
                      <span class="text-2xl">🥉</span>
                    {:else}
                      <span class="text-gray-500 dark:text-gray-400">#{i + 1}</span>
                    {/if}
                  </td>
                  <td class="py-3 pr-4 font-medium text-gray-900 dark:text-white">
                    {passenger.full_name || passenger.abbreviation}
                  </td>
                  <td class="py-3 pr-4 text-red-600 dark:text-red-400 font-semibold">
                    {formatNumber(passenger.total_co2_kg)} kg
                  </td>
                  <td class="py-3 pr-4 text-gray-600 dark:text-gray-300">
                    {passenger.total_flights}
                  </td>
                  <td class="py-3 text-gray-500 dark:text-gray-400 italic">
                    {penances[i % penances.length]}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {/if}

    <!-- Calculation Stats -->
    {#if calculationStats}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          📈 Data Quality Stats
        </h3>
        <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
          <div class="text-center p-3 bg-gray-50 dark:bg-gray-900 rounded-lg">
            <p class="text-2xl font-bold text-gray-900 dark:text-white">{calculationStats.total_flights}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400">Total Flights</p>
          </div>
          <div class="text-center p-3 bg-green-50 dark:bg-green-900/20 rounded-lg">
            <p class="text-2xl font-bold text-green-600 dark:text-green-400">{calculationStats.flights_with_distance}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400">With Distance</p>
          </div>
          <div class="text-center p-3 bg-red-50 dark:bg-red-900/20 rounded-lg">
            <p class="text-2xl font-bold text-red-600 dark:text-red-400">{calculationStats.flights_missing_distance}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400">Missing Distance</p>
          </div>
          <div class="text-center p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
            <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{calculationStats.flights_with_co2}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400">With CO2 Calculated</p>
          </div>
          <div class="text-center p-3 bg-purple-50 dark:bg-purple-900/20 rounded-lg">
            <p class="text-2xl font-bold text-purple-600 dark:text-purple-400">{calculationStats.flights_with_per_passenger_co2}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400">With Per-Passenger CO2</p>
          </div>
          <div class="text-center p-3 bg-orange-50 dark:bg-orange-900/20 rounded-lg">
            <p class="text-2xl font-bold text-orange-600 dark:text-orange-400">{calculationStats.flights_with_passengers}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400">With Passenger Data</p>
          </div>
        </div>
      </div>
    {/if}

    <!-- Calculation Result -->
    {#if calculationResult}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          ✅ Last Calculation Result
        </h3>
        {#if calculationResult.error}
          <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4 text-red-700 dark:text-red-300">
            Error: {calculationResult.error}
          </div>
        {:else}
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
              <h4 class="font-semibold mb-2">Distance Calculation</h4>
              <p class="text-sm text-gray-600 dark:text-gray-400">
                ✅ Updated: {calculationResult.distance_result?.updated_count || 0}<br/>
                ⏭️ Skipped: {calculationResult.distance_result?.skipped_count || 0}<br/>
                ❌ Failed: {calculationResult.distance_result?.failed_count || 0}
              </p>
            </div>
            <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4">
              <h4 class="font-semibold mb-2">CO2 Calculation</h4>
              <p class="text-sm text-gray-600 dark:text-gray-400">
                ✅ Updated: {calculationResult.co2_result?.updated_count || 0}<br/>
                ⏭️ Skipped: {calculationResult.co2_result?.skipped_count || 0}
              </p>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Offset Suggestions (Satirical) -->
    <div class="bg-gradient-to-r from-yellow-100 to-orange-100 dark:from-yellow-900/30 dark:to-orange-900/30 rounded-lg shadow p-6">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
        🌍 Questionable Ways to Offset Your Carbon
      </h3>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
        <div class="bg-white/50 dark:bg-gray-800/50 rounded-lg p-4">
          <div class="text-2xl mb-2">🌱</div>
          <h4 class="font-semibold">Plant Trees</h4>
          <p class="text-gray-600 dark:text-gray-400">The classic. Plant {formatNumber(treesNeeded)} trees and wait 40 years. Easy!</p>
        </div>
        <div class="bg-white/50 dark:bg-gray-800/50 rounded-lg p-4">
          <div class="text-2xl mb-2">💸</div>
          <h4 class="font-semibold">Buy Carbon Credits</h4>
          <p class="text-gray-600 dark:text-gray-400">Pay someone else to feel guilty for you. Capitalism at its finest!</p>
        </div>
        <div class="bg-white/50 dark:bg-gray-800/50 rounded-lg p-4">
          <div class="text-2xl mb-2">🚲</div>
          <h4 class="font-semibold">Never Fly Again</h4>
          <p class="text-gray-600 dark:text-gray-400">Bike everywhere. Even across oceans. Good luck!</p>
        </div>
        <div class="bg-white/50 dark:bg-gray-800/50 rounded-lg p-4">
          <div class="text-2xl mb-2">🥗</div>
          <h4 class="font-semibold">Go Vegan</h4>
          <p class="text-gray-600 dark:text-gray-400">Skip {formatNumber(burgerEquivalent)} burgers. Your colon will thank you.</p>
        </div>
        <div class="bg-white/50 dark:bg-gray-800/50 rounded-lg p-4">
          <div class="text-2xl mb-2">🏠</div>
          <h4 class="font-semibold">Live Off-Grid</h4>
          <p class="text-gray-600 dark:text-gray-400">Solar panels, rain collection, composting toilets. Very Instagram-able.</p>
        </div>
        <div class="bg-white/50 dark:bg-gray-800/50 rounded-lg p-4">
          <div class="text-2xl mb-2">🤷</div>
          <h4 class="font-semibold">Denial</h4>
          <p class="text-gray-600 dark:text-gray-400">Just pretend this dashboard doesn't exist. We won't judge. (We will.)</p>
        </div>
      </div>
      <p class="text-xs text-gray-500 dark:text-gray-400 mt-4 italic text-center">
        * This is satirical. Please do consider actual carbon offsetting programs if you're concerned about your environmental impact.
      </p>
    </div>
  {/if}
</div>
