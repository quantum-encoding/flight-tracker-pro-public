<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    flightId: string;
    onClose: () => void;
  }

  let { flightId, onClose }: Props = $props();

  let researching = $state(false);
  let result: any = $state(null);
  let error = $state<string | null>(null);

  // Research options
  let researchDeparture = $state(true);
  let researchDestination = $state(true);
  let researchNews = $state(true);
  let researchEvents = $state(true);
  let researchWeather = $state(true);
  let researchPassengers = $state(false);

  async function performResearch() {
    researching = true;
    error = null;
    result = null;

    try {
      const res = await invoke('research_flight_with_deepseek', {
        request: {
          flight_id: flightId,
          research_departure: researchDeparture,
          research_destination: researchDestination,
          research_news: researchNews,
          research_events: researchEvents,
          research_weather: researchWeather,
          research_passengers: researchPassengers,
        }
      });

      result = res;
    } catch (err) {
      console.error('Research failed:', err);
      error = err as string;
    } finally {
      researching = false;
    }
  }

  function getRelevanceColor(score: number): string {
    if (score >= 0.8) return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
    if (score >= 0.6) return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
    return 'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200';
  }
</script>

<!-- Modal Backdrop -->
<div
  class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4"
  onclick={(e) => { if (e.target === e.currentTarget) onClose(); }}
  role="button"
  tabindex="0"
>
  <!-- Modal Container -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-6xl w-full max-h-[90vh] overflow-hidden flex flex-col">
    <!-- Header -->
    <div class="border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between bg-gradient-to-r from-purple-50 to-indigo-50 dark:from-purple-900/20 dark:to-indigo-900/20">
      <div>
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
          🔍 DeepSeek Flight Research
        </h2>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
          AI-powered context research for this flight
        </p>
      </div>
      <button
        onclick={onClose}
        class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-2xl font-bold w-8 h-8 flex items-center justify-center rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition"
        title="Close"
      >
        ×
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      {#if !result && !researching}
        <!-- Research Options -->
        <div class="space-y-6">
          <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
            <p class="text-sm text-blue-800 dark:text-blue-200">
              💡 DeepSeek will search the web and analyze results to provide contextual information about this flight.
            </p>
          </div>

          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">Research Scope</h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={researchDeparture}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">Departure Location</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Research departure airport area</p>
                </div>
              </label>

              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={researchDestination}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">Destination Location</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Research arrival airport area</p>
                </div>
              </label>
            </div>
          </div>

          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">Research Topics</h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={researchNews}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">📰 Top News Stories</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Major news from that date and location</p>
                </div>
              </label>

              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={researchEvents}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">🎪 Events & Conferences</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Major gatherings, conferences, shows</p>
                </div>
              </label>

              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={researchWeather}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">☀️ Weather Conditions</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Historical weather data for that date</p>
                </div>
              </label>

              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={researchPassengers}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">👤 Passenger Mentions</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Search for passenger names in news/events</p>
                </div>
              </label>
            </div>
          </div>

          <button
            onclick={performResearch}
            disabled={researching || (!researchDeparture && !researchDestination)}
            class="w-full bg-purple-600 hover:bg-purple-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white px-6 py-3 rounded-lg font-medium transition"
          >
            {researching ? '🔄 Researching...' : '🚀 Start Research'}
          </button>
        </div>
      {:else if researching}
        <!-- Loading State -->
        <div class="flex flex-col items-center justify-center py-16">
          <div class="animate-spin rounded-full h-16 w-16 border-b-4 border-purple-600 mb-4"></div>
          <p class="text-gray-600 dark:text-gray-400 text-lg font-medium">Analyzing web search results with DeepSeek AI...</p>
          <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">This may take 15-30 seconds</p>
        </div>
      {:else if error}
        <!-- Error State -->
        <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-6">
          <div class="flex items-start">
            <span class="text-4xl">⚠️</span>
            <div class="ml-4">
              <h3 class="text-lg font-semibold text-red-900 dark:text-red-200 mb-2">Research Failed</h3>
              <p class="text-red-700 dark:text-red-300 text-sm">{error}</p>
              <button
                onclick={() => { result = null; error = null; }}
                class="mt-4 bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-lg font-medium transition"
              >
                Try Again
              </button>
            </div>
          </div>
        </div>
      {:else if result}
        <!-- Research Results -->
        <div class="space-y-6">
          <!-- Summary -->
          <div class="bg-gradient-to-r from-purple-50 to-indigo-50 dark:from-purple-900/20 dark:to-indigo-900/20 rounded-lg p-6 border border-purple-200 dark:border-purple-800">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">📊 Summary</h3>
            <p class="text-gray-700 dark:text-gray-300">{result.summary}</p>
          </div>

          <!-- News Stories -->
          {#if result.news_stories && result.news_stories.length > 0}
            <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
              <div class="px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">📰 News Stories</h3>
              </div>
              <div class="p-4 space-y-3">
                {#each result.news_stories as story}
                  <div class="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                    <div class="flex items-start justify-between mb-2">
                      <h4 class="font-semibold text-gray-900 dark:text-white flex-1">{story.title}</h4>
                      <span class="ml-2 px-2 py-0.5 rounded text-xs font-medium {getRelevanceColor(story.relevance)}">
                        {Math.round(story.relevance * 100)}%
                      </span>
                    </div>
                    <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">{story.summary}</p>
                    <p class="text-xs text-gray-500 dark:text-gray-500">Source: {story.source}</p>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Events -->
          {#if result.events && result.events.length > 0}
            <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
              <div class="px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">🎪 Events & Conferences</h3>
              </div>
              <div class="p-4 space-y-3">
                {#each result.events as event}
                  <div class="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                    <h4 class="font-semibold text-gray-900 dark:text-white mb-1">{event.name}</h4>
                    <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">{event.description}</p>
                    <p class="text-xs text-gray-500 dark:text-gray-500">📍 {event.venue}</p>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Weather -->
          {#if result.weather}
            <div class="bg-gradient-to-br from-blue-50 to-cyan-50 dark:from-blue-900/20 dark:to-cyan-900/20 rounded-lg p-4 border border-blue-200 dark:border-blue-800">
              <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">☀️ Weather Conditions</h3>
              <div class="space-y-1">
                <p class="text-gray-700 dark:text-gray-300"><strong>Condition:</strong> {result.weather.condition}</p>
                <p class="text-gray-700 dark:text-gray-300"><strong>Temperature:</strong> {result.weather.temperature}</p>
                <p class="text-gray-600 dark:text-gray-400 text-sm">{result.weather.description}</p>
              </div>
            </div>
          {/if}

          <!-- Passenger Mentions -->
          {#if result.passenger_mentions && result.passenger_mentions.length > 0}
            <div class="bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800 overflow-hidden">
              <div class="px-4 py-3 bg-yellow-100 dark:bg-yellow-900/30 border-b border-yellow-200 dark:border-yellow-800">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">👤 Passenger Mentions</h3>
              </div>
              <div class="p-4 space-y-3">
                {#each result.passenger_mentions as mention}
                  <div class="p-3 bg-white dark:bg-gray-900 rounded-lg">
                    <h4 class="font-semibold text-gray-900 dark:text-white mb-1">{mention.passenger_name}</h4>
                    <p class="text-sm text-gray-600 dark:text-gray-400 mb-1">{mention.context}</p>
                    <p class="text-xs text-gray-500 dark:text-gray-500">Source: {mention.source}</p>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <button
            onclick={() => { result = null; error = null; }}
            class="w-full bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-lg font-medium transition"
          >
            🔄 New Research
          </button>
        </div>
      {/if}
    </div>

    <!-- Footer -->
    <div class="border-t border-gray-200 dark:border-gray-700 px-6 py-4 bg-gray-50 dark:bg-gray-900">
      <button
        onclick={onClose}
        class="w-full bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-lg font-medium transition"
      >
        Close
      </button>
    </div>
  </div>
</div>
