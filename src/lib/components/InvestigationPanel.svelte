<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    flight: any;
    onClose: () => void;
  }

  let { flight, onClose }: Props = $props();

  let investigating = $state(false);
  let result: any = $state(null);
  let error = $state('');
  let passengerNames = $state('');

  async function runInvestigation() {
    if (!passengerNames.trim()) {
      error = 'Please enter at least one passenger name';
      return;
    }

    investigating = true;
    error = '';
    result = null;

    try {
      const names = passengerNames
        .split(',')
        .map(n => n.trim())
        .filter(n => n.length > 0);

      console.log('Starting investigation for:', {
        flightId: flight.id,
        passengerNames: names,
      });

      const investigationResult = await invoke('investigate_flight', {
        flightId: flight.id,
        passengerNames: names,
      });

      console.log('Investigation complete:', investigationResult);
      result = investigationResult;
    } catch (err) {
      console.error('Investigation failed:', err);
      error = String(err);
    } finally {
      investigating = false;
    }
  }

  function getScoreColor(score: number): string {
    if (score >= 0.7) return 'text-green-600 dark:text-green-400';
    if (score >= 0.4) return 'text-yellow-600 dark:text-yellow-400';
    return 'text-red-600 dark:text-red-400';
  }

  function getScoreLabel(score: number): string {
    if (score >= 0.7) return 'Strong Evidence';
    if (score >= 0.4) return 'Circumstantial';
    return 'Weak/No Evidence';
  }

  // Auto-populate passenger names from flight notes if available
  $effect(() => {
    if (flight.notes && flight.notes.includes('Passengers:')) {
      const match = flight.notes.match(/Passengers:\s*(.+)/);
      if (match) {
        passengerNames = match[1];
      }
    }
  });
</script>

<div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-5xl w-full mx-4 max-h-[90vh] overflow-y-auto">
    <!-- Header -->
    <div class="sticky top-0 bg-gradient-to-r from-indigo-600 to-purple-600 text-white px-6 py-4 flex items-center justify-between rounded-t-lg">
      <div>
        <h2 class="text-2xl font-bold">AI Investigation Engine</h2>
        <p class="text-sm text-indigo-100 mt-1">
          Flight: {flight.departure_airport} → {flight.arrival_airport} on {new Date(flight.departure_datetime).toLocaleDateString()}
        </p>
      </div>
      <button
        onclick={onClose}
        class="text-white hover:text-indigo-200 text-2xl font-bold"
      >
        ✕
      </button>
    </div>

    <!-- Content -->
    <div class="px-6 py-4">
      {#if !result}
        <!-- Input Form -->
        <div class="space-y-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
              Passenger Name(s) to Investigate
            </label>
            <input
              type="text"
              bind:value={passengerNames}
              placeholder="John Doe, Jane Smith (comma-separated)"
              class="w-full px-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
            />
            <p class="mt-2 text-sm text-gray-500 dark:text-gray-400">
              Enter the names of passengers you want to cross-reference. The AI will search for public records, news articles, and other evidence of their presence at this location on this date.
            </p>
          </div>

          {#if error}
            <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 px-4 py-3 rounded-lg">
              {error}
            </div>
          {/if}

          <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 text-blue-700 dark:text-blue-300 px-4 py-3 rounded-lg">
            <p class="font-semibold">How it works:</p>
            <ol class="list-decimal list-inside mt-2 space-y-1 text-sm">
              <li>AI generates targeted search queries based on the passenger, location, and date</li>
              <li>Automated web search for news articles, records, and public documents</li>
              <li>AI synthesizes findings into an evidence report with sources</li>
            </ol>
          </div>

          <button
            onclick={runInvestigation}
            disabled={investigating}
            class="w-full bg-indigo-600 hover:bg-indigo-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white px-6 py-3 rounded-lg font-medium transition flex items-center justify-center"
          >
            {#if investigating}
              <span class="animate-spin mr-2">⚙️</span>
              Investigating... (This may take 30-60 seconds)
            {:else}
              🔍 Start Investigation
            {/if}
          </button>
        </div>
      {:else}
        <!-- Investigation Results -->
        <div class="space-y-6">
          <!-- Corroboration Score -->
          <div class="bg-gradient-to-r from-gray-50 to-gray-100 dark:from-gray-700 dark:to-gray-800 rounded-lg p-6">
            <div class="flex items-center justify-between">
              <div>
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-1">
                  Corroboration Score
                </h3>
                <p class="text-sm text-gray-600 dark:text-gray-400">
                  AI confidence in evidence quality
                </p>
              </div>
              <div class="text-right">
                <div class="text-4xl font-bold {getScoreColor(result.corroboration_score)}">
                  {(result.corroboration_score * 100).toFixed(0)}%
                </div>
                <div class="text-sm {getScoreColor(result.corroboration_score)}">
                  {getScoreLabel(result.corroboration_score)}
                </div>
              </div>
            </div>
          </div>

          <!-- AI Summary -->
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">
              📋 Investigation Summary
            </h3>
            <div class="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-4">
              <p class="text-gray-800 dark:text-gray-200 whitespace-pre-wrap leading-relaxed">
                {result.ai_summary}
              </p>
            </div>
          </div>

          <!-- Sources -->
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">
              🔗 Sources & Evidence ({result.sources.length})
            </h3>
            <div class="space-y-3">
              {#each result.sources as source, i}
                <div class="bg-white dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:border-indigo-500 dark:hover:border-indigo-400 transition">
                  <div class="flex items-start justify-between mb-2">
                    <h4 class="font-semibold text-gray-900 dark:text-white flex-1">
                      [{i + 1}] {source.title}
                    </h4>
                    <span class="ml-2 px-2 py-1 text-xs rounded-full {
                      source.relevance_score >= 0.8 ? 'bg-green-100 dark:bg-green-900 text-green-800 dark:text-green-200' :
                      source.relevance_score >= 0.5 ? 'bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200' :
                      'bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200'
                    }">
                      {source.relevance_score >= 0.8 ? 'High' : source.relevance_score >= 0.5 ? 'Medium' : 'Low'} Relevance
                    </span>
                  </div>
                  <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">
                    {source.excerpt}
                  </p>
                  <a
                    href={source.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    class="text-sm text-indigo-600 dark:text-indigo-400 hover:underline break-all"
                  >
                    {source.url}
                  </a>
                </div>
              {/each}

              {#if result.sources.length === 0}
                <div class="bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-8 text-center">
                  <p class="text-gray-600 dark:text-gray-400">
                    No relevant sources found. Try different passenger names or check if the Gemini API key is configured correctly.
                  </p>
                </div>
              {/if}
            </div>
          </div>

          <!-- Generated Queries (Debug Info) -->
          <details class="bg-gray-50 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded-lg p-4">
            <summary class="cursor-pointer font-semibold text-gray-900 dark:text-white">
              🔎 Generated Search Queries ({result.generated_queries.length})
            </summary>
            <ul class="mt-3 space-y-1 text-sm text-gray-600 dark:text-gray-400">
              {#each result.generated_queries as query}
                <li class="font-mono bg-white dark:bg-gray-800 px-2 py-1 rounded">
                  {query}
                </li>
              {/each}
            </ul>
          </details>

          <!-- Meta Info -->
          <div class="text-xs text-gray-500 dark:text-gray-400 text-center">
            Investigation completed in {result.processing_time_ms}ms
          </div>

          <!-- Action Buttons -->
          <div class="flex gap-3">
            <button
              onclick={() => { result = null; passengerNames = ''; }}
              class="flex-1 bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              🔄 New Investigation
            </button>
            <button
              onclick={onClose}
              class="flex-1 bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              Close
            </button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
