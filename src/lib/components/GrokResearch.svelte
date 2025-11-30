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
  let researchMode = $state<'grok' | 'multi'>('grok');
  let selectedGrokModel = $state('grok-4-fast-non-reasoning');

  // Research topics
  let topics = $state({
    news: true,
    events: true,
    weather: true,
    aviation: true,
  });

  async function performResearch() {
    researching = true;
    error = null;
    result = null;

    try {
      const researchTopics = Object.entries(topics)
        .filter(([_, enabled]) => enabled)
        .map(([topic, _]) => topic);

      if (researchTopics.length === 0) {
        error = 'Please select at least one research topic';
        researching = false;
        return;
      }

      if (researchMode === 'grok') {
        // Grok-only research
        const res = await invoke('research_flight_with_grok', {
          flightId,
          researchTopics,
          modelName: selectedGrokModel
        });
        result = { mode: 'grok', data: res };
      } else {
        // Multi-provider comparison
        const res = await invoke('multi_provider_flight_research', {
          flightId,
          researchTopics
        });
        result = { mode: 'multi', data: res };
      }
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

  function getCategoryIcon(category: string): string {
    const icons: Record<string, string> = {
      News: '📰',
      Events: '🎪',
      Weather: '☀️',
      Travel: '✈️',
      Aviation: '🛫',
      Other: '📌',
    };
    return icons[category] || '📌';
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
    <div class="border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between bg-gradient-to-r from-purple-50 via-pink-50 to-indigo-50 dark:from-purple-900/20 dark:via-pink-900/20 dark:to-indigo-900/20">
      <div>
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
          {researchMode === 'grok' ? '🧠 Grok Flight Research' : '🤖 Multi-Provider Analysis'}
        </h2>
        <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
          {researchMode === 'grok' ? 'AI-powered research with Grok' : 'Compare insights from Grok and DeepSeek'}
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
          <div class="bg-purple-50 dark:bg-purple-900/20 border border-purple-200 dark:border-purple-800 rounded-lg p-4">
            <p class="text-sm text-purple-800 dark:text-purple-200">
              💡 {researchMode === 'grok'
                ? 'Grok will use advanced web search to provide real-time contextual insights about this flight.'
                : 'Compare analyses from both Grok and DeepSeek for comprehensive insights and validation.'}
            </p>
          </div>

          <!-- Research Mode Selection -->
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">Research Mode</h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <button
                onclick={() => researchMode = 'grok'}
                class="p-4 rounded-lg border-2 transition text-left {researchMode === 'grok'
                  ? 'border-purple-600 bg-purple-50 dark:bg-purple-900/30'
                  : 'border-gray-300 dark:border-gray-600 hover:border-purple-400'}"
              >
                <div class="font-semibold text-gray-900 dark:text-white">🧠 Grok Only</div>
                <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">Fast, focused analysis with X.AI's Grok</p>
              </button>

              <button
                onclick={() => researchMode = 'multi'}
                class="p-4 rounded-lg border-2 transition text-left {researchMode === 'multi'
                  ? 'border-purple-600 bg-purple-50 dark:bg-purple-900/30'
                  : 'border-gray-300 dark:border-gray-600 hover:border-purple-400'}"
              >
                <div class="font-semibold text-gray-900 dark:text-white">🤖 Multi-Provider</div>
                <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">Compare Grok + DeepSeek for validation</p>
              </button>
            </div>
          </div>

          {#if researchMode === 'grok'}
            <!-- Grok Model Selection -->
            <div>
              <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">Grok Model</h3>
              <div class="space-y-2">
                <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition border-2 {selectedGrokModel === 'grok-4-fast-non-reasoning' ? 'border-purple-600' : 'border-transparent'}">
                  <input
                    type="radio"
                    value="grok-4-fast-non-reasoning"
                    bind:group={selectedGrokModel}
                    class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 focus:ring-purple-500"
                  />
                  <div class="ml-3">
                    <span class="text-sm font-medium text-gray-900 dark:text-white">grok-4-fast-non-reasoning</span>
                    <p class="text-xs text-gray-600 dark:text-gray-400">Fastest model, optimized for quick responses without reasoning chains.</p>
                  </div>
                </label>

                <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition border-2 {selectedGrokModel === 'grok-4-fast-reasoning' ? 'border-purple-600' : 'border-transparent'}">
                  <input
                    type="radio"
                    value="grok-4-fast-reasoning"
                    bind:group={selectedGrokModel}
                    class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 focus:ring-purple-500"
                  />
                  <div class="ml-3">
                    <span class="text-sm font-medium text-gray-900 dark:text-white">grok-4-fast-reasoning</span>
                    <p class="text-xs text-gray-600 dark:text-gray-400">Advanced model with reasoning capabilities for deeper analysis.</p>
                  </div>
                </label>

                <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition border-2 {selectedGrokModel === 'grok-code-fast-1' ? 'border-purple-600' : 'border-transparent'}">
                  <input
                    type="radio"
                    value="grok-code-fast-1"
                    bind:group={selectedGrokModel}
                    class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 focus:ring-purple-500"
                  />
                  <div class="ml-3">
                    <span class="text-sm font-medium text-gray-900 dark:text-white">grok-code-fast-1</span>
                    <p class="text-xs text-gray-600 dark:text-gray-400">Specialized for code analysis and technical research.</p>
                  </div>
                </label>
              </div>
            </div>
          {/if}

          <!-- Research Topics -->
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">Research Topics</h3>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={topics.news}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">📰 News Stories</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Breaking news from that time/location</p>
                </div>
              </label>

              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={topics.events}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">🎪 Events & Conferences</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Major gatherings and conferences</p>
                </div>
              </label>

              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={topics.weather}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">☀️ Weather Conditions</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Weather data and conditions</p>
                </div>
              </label>

              <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition">
                <input
                  type="checkbox"
                  bind:checked={topics.aviation}
                  class="mt-1 w-4 h-4 text-purple-600 bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-600 rounded focus:ring-purple-500"
                />
                <div class="ml-3">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">🛫 Aviation Context</span>
                  <p class="text-xs text-gray-600 dark:text-gray-400">Flight status, incidents, aviation news</p>
                </div>
              </label>
            </div>
          </div>

          <button
            onclick={performResearch}
            disabled={researching}
            class="w-full bg-purple-600 hover:bg-purple-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white px-6 py-3 rounded-lg font-medium transition"
          >
            {researching ? '🔄 Researching...' : '🚀 Start Research'}
          </button>
        </div>
      {:else if researching}
        <!-- Loading State -->
        <div class="flex flex-col items-center justify-center py-16">
          <div class="animate-spin rounded-full h-16 w-16 border-b-4 border-purple-600 mb-4"></div>
          <p class="text-gray-600 dark:text-gray-400 text-lg font-medium">
            {researchMode === 'grok' ? 'Analyzing with Grok AI...' : 'Running multi-provider analysis...'}
          </p>
          <p class="text-gray-500 dark:text-gray-500 text-sm mt-2">This may take 15-45 seconds</p>
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
      {:else if result && result.mode === 'grok'}
        <!-- Grok-Only Results -->
        <div class="space-y-6">
          <!-- Summary -->
          <div class="bg-gradient-to-r from-purple-50 to-pink-50 dark:from-purple-900/20 dark:to-pink-900/20 rounded-lg p-6 border border-purple-200 dark:border-purple-800">
            <div class="flex items-center justify-between mb-3">
              <h3 class="text-lg font-semibold text-gray-900 dark:text-white">📊 Summary</h3>
              <span class="px-3 py-1 rounded-full text-xs font-medium bg-purple-600 text-white">
                Confidence: {Math.round((result.data.confidence_score || 0) * 100)}%
              </span>
            </div>
            <p class="text-gray-700 dark:text-gray-300">{result.data.summary}</p>
          </div>

          <!-- Key Findings -->
          {#if result.data.key_findings && result.data.key_findings.length > 0}
            <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
              <div class="px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">🔍 Key Findings</h3>
              </div>
              <div class="p-4 space-y-3">
                {#each result.data.key_findings as finding}
                  <div class="p-4 bg-gray-50 dark:bg-gray-800 rounded-lg border-l-4 border-purple-500">
                    <div class="flex items-start justify-between mb-2">
                      <div class="flex items-center">
                        <span class="text-2xl mr-2">{getCategoryIcon(finding.category)}</span>
                        <span class="font-semibold text-gray-900 dark:text-white">{finding.category}</span>
                      </div>
                      <span class="px-2 py-0.5 rounded text-xs font-medium {getRelevanceColor(finding.relevance)}">
                        {Math.round(finding.relevance * 100)}%
                      </span>
                    </div>
                    <p class="text-sm text-gray-600 dark:text-gray-400 ml-10">{finding.description}</p>
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Sources -->
          {#if result.data.sources && result.data.sources.length > 0}
            <div class="bg-white dark:bg-gray-900 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden">
              <div class="px-4 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">📚 Sources</h3>
              </div>
              <div class="p-4 space-y-3">
                {#each result.data.sources as source}
                  <div class="p-3 bg-gray-50 dark:bg-gray-800 rounded-lg">
                    <h4 class="font-semibold text-gray-900 dark:text-white mb-1">{source.title}</h4>
                    <p class="text-sm text-gray-600 dark:text-gray-400 mb-2">{source.snippet}</p>
                    {#if source.url}
                      <a
                        href={source.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="text-xs text-purple-600 hover:text-purple-800 dark:text-purple-400 dark:hover:text-purple-300 hover:underline"
                      >
                        🔗 {source.url}
                      </a>
                    {/if}
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
      {:else if result && result.mode === 'multi'}
        <!-- Multi-Provider Results -->
        <div class="space-y-6">
          <!-- Consensus Summary -->
          <div class="bg-gradient-to-r from-blue-50 via-purple-50 to-pink-50 dark:from-blue-900/20 dark:via-purple-900/20 dark:to-pink-900/20 rounded-lg p-6 border border-blue-200 dark:border-blue-800">
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-3">🤝 Consensus Summary</h3>
            <p class="text-gray-700 dark:text-gray-300">{result.data.consensus_summary}</p>
          </div>

          <!-- Disagreements (if any) -->
          {#if result.data.disagreements && result.data.disagreements.length > 0}
            <div class="bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800 overflow-hidden">
              <div class="px-4 py-3 bg-yellow-100 dark:bg-yellow-900/30 border-b border-yellow-200 dark:border-yellow-800">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">⚠️ Noted Disagreements</h3>
              </div>
              <div class="p-4 space-y-2">
                {#each result.data.disagreements as disagreement}
                  <p class="text-sm text-gray-700 dark:text-gray-300">• {disagreement}</p>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Grok Analysis -->
          {#if result.data.grok_analysis}
            <div class="bg-purple-50 dark:bg-purple-900/20 rounded-lg border border-purple-200 dark:border-purple-800 overflow-hidden">
              <div class="px-4 py-3 bg-purple-100 dark:bg-purple-900/30 border-b border-purple-200 dark:border-purple-800">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">🧠 Grok Analysis</h3>
              </div>
              <div class="p-4">
                <p class="text-gray-700 dark:text-gray-300 mb-3">{result.data.grok_analysis.summary}</p>
                {#if result.data.grok_analysis.key_findings}
                  <div class="space-y-2">
                    {#each result.data.grok_analysis.key_findings.slice(0, 3) as finding}
                      <div class="text-sm text-gray-600 dark:text-gray-400">
                        • {finding.description}
                      </div>
                    {/each}
                  </div>
                {/if}
              </div>
            </div>
          {/if}

          <!-- DeepSeek Analysis -->
          {#if result.data.deepseek_analysis}
            <div class="bg-indigo-50 dark:bg-indigo-900/20 rounded-lg border border-indigo-200 dark:border-indigo-800 overflow-hidden">
              <div class="px-4 py-3 bg-indigo-100 dark:bg-indigo-900/30 border-b border-indigo-200 dark:border-indigo-800">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-white">🤖 DeepSeek Analysis</h3>
              </div>
              <div class="p-4">
                <p class="text-gray-700 dark:text-gray-300">{result.data.deepseek_analysis.summary}</p>
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
