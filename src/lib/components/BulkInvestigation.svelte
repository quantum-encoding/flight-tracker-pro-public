<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    flights: any[];
    onClose: () => void;
  }

  let { flights, onClose }: Props = $props();

  type AIModel = 'gemini' | 'deepseek' | 'grok';
  type FlightStatus = 'pending' | 'processing' | 'completed' | 'failed' | 'retrying';

  interface FlightProgress {
    flight: any;
    status: FlightStatus;
    error?: string;
    retries: number;
    result?: any;
  }

  let selectedModel = $state<AIModel>('gemini');
  let isRunning = $state(false);
  let isPaused = $state(false);
  let currentIndex = $state(0);
  let flightProgress = $state<FlightProgress[]>(
    flights.map(flight => ({
      flight,
      status: 'pending',
      retries: 0,
    }))
  );

  // Retry configuration
  const MAX_RETRIES = 3;
  const RETRY_DELAYS = [5000, 15000, 30000]; // 5s, 15s, 30s exponential backoff

  async function sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  async function investigateSingleFlight(
    index: number,
    flight: any,
    retryCount: number = 0
  ): Promise<void> {
    flightProgress[index].status = retryCount > 0 ? 'retrying' : 'processing';
    flightProgress[index].retries = retryCount;

    try {
      // Extract passenger names from flight notes if available
      let passengerNames: string[] = [];
      if (flight.notes && flight.notes.includes('Passengers:')) {
        const match = flight.notes.match(/Passengers:\s*(.+)/);
        if (match) {
          passengerNames = match[1].split(',').map((n: string) => n.trim());
        }
      }

      // If no passengers found, use a generic query
      if (passengerNames.length === 0) {
        passengerNames = ['passenger'];
      }

      const result = await invoke('investigate_flight', {
        flightId: flight.id,
        passengerNames,
        model: selectedModel, // Pass model to backend (will need backend update)
      });

      flightProgress[index].status = 'completed';
      flightProgress[index].result = result;
    } catch (error) {
      const errorMessage = String(error);

      // Check if it's a 429 rate limit error
      if (errorMessage.includes('429') || errorMessage.includes('rate limit')) {
        if (retryCount < MAX_RETRIES) {
          flightProgress[index].status = 'retrying';
          const delayMs = RETRY_DELAYS[retryCount];

          console.log(
            `Rate limit hit for flight ${index + 1}. Retrying in ${delayMs / 1000}s...`
          );

          await sleep(delayMs);

          if (!isPaused) {
            await investigateSingleFlight(index, flight, retryCount + 1);
          }
          return;
        }
      }

      // Mark as failed if max retries exceeded or different error
      flightProgress[index].status = 'failed';
      flightProgress[index].error = errorMessage;
    }
  }

  async function startBulkInvestigation() {
    isRunning = true;
    isPaused = false;

    for (let i = currentIndex; i < flights.length; i++) {
      if (!isRunning || isPaused) {
        currentIndex = i;
        break;
      }

      currentIndex = i;
      await investigateSingleFlight(i, flights[i]);

      // Add a small delay between requests to be respectful
      if (i < flights.length - 1 && isRunning && !isPaused) {
        await sleep(2000); // 2 second delay between flights
      }
    }

    if (currentIndex >= flights.length - 1) {
      isRunning = false;
      currentIndex = 0;
    }
  }

  function pauseInvestigation() {
    isPaused = true;
  }

  function resumeInvestigation() {
    isPaused = false;
    startBulkInvestigation();
  }

  function stopInvestigation() {
    isRunning = false;
    isPaused = false;
  }

  function getStatusColor(status: FlightStatus): string {
    switch (status) {
      case 'pending':
        return 'text-gray-500 dark:text-gray-400';
      case 'processing':
        return 'text-blue-600 dark:text-blue-400';
      case 'retrying':
        return 'text-yellow-600 dark:text-yellow-400';
      case 'completed':
        return 'text-green-600 dark:text-green-400';
      case 'failed':
        return 'text-red-600 dark:text-red-400';
      default:
        return 'text-gray-500';
    }
  }

  function getStatusIcon(status: FlightStatus): string {
    switch (status) {
      case 'pending':
        return '⏳';
      case 'processing':
        return '🔄';
      case 'retrying':
        return '🔁';
      case 'completed':
        return '✅';
      case 'failed':
        return '❌';
      default:
        return '❓';
    }
  }

  const completedCount = $derived(
    flightProgress.filter(fp => fp.status === 'completed').length
  );
  const failedCount = $derived(
    flightProgress.filter(fp => fp.status === 'failed').length
  );
  const progressPercentage = $derived(
    Math.round(((completedCount + failedCount) / flights.length) * 100)
  );
</script>

<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
  <div
    class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-6xl w-full mx-4 max-h-[90vh] overflow-hidden flex flex-col"
  >
    <!-- Header -->
    <div
      class="sticky top-0 bg-gradient-to-r from-purple-600 to-indigo-600 text-white px-6 py-4 flex items-center justify-between"
    >
      <div>
        <h2 class="text-2xl font-bold">Bulk AI Investigation</h2>
        <p class="text-sm text-purple-100 mt-1">
          Investigating {flights.length} flights with intelligent retry handling
        </p>
      </div>
      <button
        onclick={onClose}
        class="text-white hover:text-purple-200 text-2xl font-bold"
      >
        ✕
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6 space-y-6">
      <!-- Configuration Section -->
      <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4 space-y-4">
        <div class="grid grid-cols-2 gap-4">
          <!-- Model Selector -->
          <div>
            <label
              class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
            >
              AI Model
            </label>
            <select
              bind:value={selectedModel}
              disabled={isRunning}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-purple-500 focus:border-transparent disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <option value="gemini">🤖 Google Gemini (Fast, Recommended)</option>
              <option value="deepseek">🧠 DeepSeek (Advanced Research)</option>
              <option value="grok">⚡ Grok (X.AI with Web Search)</option>
            </select>
          </div>

          <!-- Progress Stats -->
          <div class="flex items-center gap-4 text-sm">
            <div class="text-center">
              <div class="text-2xl font-bold text-green-600 dark:text-green-400">
                {completedCount}
              </div>
              <div class="text-xs text-gray-600 dark:text-gray-400">Completed</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-red-600 dark:text-red-400">
                {failedCount}
              </div>
              <div class="text-xs text-gray-600 dark:text-gray-400">Failed</div>
            </div>
            <div class="text-center">
              <div class="text-2xl font-bold text-gray-600 dark:text-gray-400">
                {flights.length - completedCount - failedCount}
              </div>
              <div class="text-xs text-gray-600 dark:text-gray-400">Remaining</div>
            </div>
          </div>
        </div>

        <!-- Progress Bar -->
        <div>
          <div class="flex items-center justify-between text-sm mb-2">
            <span class="font-medium text-gray-700 dark:text-gray-300">Overall Progress</span>
            <span class="text-gray-600 dark:text-gray-400">{progressPercentage}%</span>
          </div>
          <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-3 overflow-hidden">
            <div
              class="bg-gradient-to-r from-purple-600 to-indigo-600 h-full transition-all duration-300"
              style="width: {progressPercentage}%"
            ></div>
          </div>
        </div>

        <!-- Control Buttons -->
        <div class="flex gap-2">
          {#if !isRunning}
            <button
              onclick={startBulkInvestigation}
              class="flex-1 bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-700 hover:to-indigo-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              {currentIndex > 0 ? '▶️ Resume Investigation' : '🚀 Start Investigation'}
            </button>
          {:else if isPaused}
            <button
              onclick={resumeInvestigation}
              class="flex-1 bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              ▶️ Resume
            </button>
          {:else}
            <button
              onclick={pauseInvestigation}
              class="flex-1 bg-yellow-600 hover:bg-yellow-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              ⏸️ Pause
            </button>
          {/if}

          {#if isRunning || isPaused}
            <button
              onclick={stopInvestigation}
              class="bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              ⏹️ Stop
            </button>
          {/if}
        </div>

        <!-- Retry Info -->
        <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded p-3">
          <p class="text-xs text-blue-800 dark:text-blue-200">
            <strong>💡 Smart Retry:</strong> Rate limit errors (429) automatically retry with exponential
            backoff (5s → 15s → 30s). Max {MAX_RETRIES} retries per flight.
          </p>
        </div>
      </div>

      <!-- Flight List -->
      <div class="space-y-2">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">
          Flight Investigation Status
        </h3>

        <div class="space-y-2 max-h-96 overflow-y-auto">
          {#each flightProgress as fp, index}
            <div
              class="bg-white dark:bg-gray-700 rounded-lg p-3 border-2 transition-all"
              class:border-blue-500={index === currentIndex && isRunning}
              class:border-gray-200={index !== currentIndex || !isRunning}
              class:dark:border-gray-600={index !== currentIndex || !isRunning}
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-3 flex-1">
                  <span class="text-2xl">{getStatusIcon(fp.status)}</span>
                  <div class="flex-1">
                    <div class="font-medium text-gray-900 dark:text-white">
                      {fp.flight.departure_airport} → {fp.flight.arrival_airport}
                    </div>
                    <div class="text-xs text-gray-600 dark:text-gray-400">
                      {new Date(fp.flight.departure_datetime).toLocaleDateString()} •
                      {fp.flight.airline || 'Unknown Airline'}
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-3">
                  {#if fp.retries > 0}
                    <span
                      class="text-xs bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200 px-2 py-1 rounded"
                    >
                      Retry {fp.retries}/{MAX_RETRIES}
                    </span>
                  {/if}

                  <span class="{getStatusColor(fp.status)} font-medium capitalize">
                    {fp.status}
                  </span>
                </div>
              </div>

              {#if fp.error}
                <div class="mt-2 text-xs text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20 p-2 rounded">
                  Error: {fp.error}
                </div>
              {/if}

              {#if fp.result}
                <div class="mt-2 text-xs text-green-600 dark:text-green-400">
                  ✓ Investigation completed • Score: {(fp.result.corroboration_score * 100).toFixed(
                    0
                  )}%
                </div>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>
