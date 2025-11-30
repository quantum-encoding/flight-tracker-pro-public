<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    user: any;
    agentId: 'grok' | 'deepseek' | 'gemini';
    agentName: string;
    agentIcon: string;
    agentColor: string;
    onBack: () => void;
  }

  let { user, agentId, agentName, agentIcon, agentColor, onBack }: Props = $props();

  interface Message {
    role: 'user' | 'assistant';
    content: string;
    timestamp: Date;
    model?: string;
    processingTime?: number;
    error?: string;
  }

  let messages = $state<Message[]>([]);
  let inputMessage = $state('');
  let processing = $state(false);
  let grokModel = $state('grok-4-fast-non-reasoning');
  let geminiModel = $state('gemini-2.5-flash');

  // Research topics for context
  let enabledTools = $state({
    webSearch: true,
    flightData: true,
    dataVisualization: true,
  });

  async function sendMessage() {
    if (!inputMessage.trim() || processing) return;

    const userMessage: Message = {
      role: 'user',
      content: inputMessage.trim(),
      timestamp: new Date(),
    };

    messages = [...messages, userMessage];
    const query = inputMessage;
    inputMessage = '';
    processing = true;

    try {
      const startTime = Date.now();
      let response: any;
      let modelUsed: string | null = null;

      if (agentId === 'grok') {
        modelUsed = grokModel;

        // Use Grok's chat capabilities with custom query
        response = await invoke('research_flight_with_grok', {
          flightId: '',
          researchTopics: [],
          modelName: grokModel,
          customQuery: query,
        });

        const assistantMessage: Message = {
          role: 'assistant',
          content: response.summary || 'No response received',
          timestamp: new Date(),
          model: modelUsed,
          processingTime: Date.now() - startTime,
        };

        messages = [...messages, assistantMessage];
      } else if (agentId === 'deepseek') {
        modelUsed = 'deepseek-chat';

        response = await invoke('chat_with_deepseek', {
          query,
        });

        const assistantMessage: Message = {
          role: 'assistant',
          content: response.content || 'No response received',
          timestamp: new Date(),
          model: modelUsed,
          processingTime: Date.now() - startTime,
        };

        messages = [...messages, assistantMessage];
      } else if (agentId === 'gemini') {
        modelUsed = geminiModel;

        response = await invoke('chat_with_gemini', {
          query,
          model: geminiModel,
        });

        const assistantMessage: Message = {
          role: 'assistant',
          content: response.content || 'No response received',
          timestamp: new Date(),
          model: modelUsed,
          processingTime: Date.now() - startTime,
        };

        messages = [...messages, assistantMessage];
      }
    } catch (err: any) {
      const errorMessage: Message = {
        role: 'assistant',
        content: '',
        timestamp: new Date(),
        error: err.toString(),
      };

      messages = [...messages, errorMessage];
    } finally {
      processing = false;
    }
  }

  function clearChat() {
    messages = [];
  }

  function getColorClasses(color: string) {
    const colorMap: Record<string, { border: string; bg: string; text: string }> = {
      purple: {
        border: 'border-purple-500',
        bg: 'bg-purple-50 dark:bg-purple-900/20',
        text: 'text-purple-600 dark:text-purple-400',
      },
      blue: {
        border: 'border-blue-500',
        bg: 'bg-blue-50 dark:bg-blue-900/20',
        text: 'text-blue-600 dark:text-blue-400',
      },
      indigo: {
        border: 'border-indigo-500',
        bg: 'bg-indigo-50 dark:bg-indigo-900/20',
        text: 'text-indigo-600 dark:text-indigo-400',
      },
    };
    return colorMap[color] || colorMap.blue;
  }

  const colors = $derived(getColorClasses(agentColor));
</script>

<div class="p-6 max-w-7xl mx-auto h-screen flex flex-col">
  <!-- Header -->
  <div class="mb-4 flex items-center gap-4">
    <button
      onclick={onBack}
      class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg transition"
    >
      ← Back
    </button>
    <div class="flex items-center gap-3 flex-1">
      <span class="text-4xl">{agentIcon}</span>
      <div>
        <h1 class="text-2xl font-bold text-gray-900 dark:text-white">
          {agentName} Chat
        </h1>
        <p class="text-sm text-gray-600 dark:text-gray-400">
          Direct conversation with AI research capabilities
        </p>
      </div>
    </div>
    <button
      onclick={clearChat}
      class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition text-sm"
    >
      Clear Chat
    </button>
  </div>

  <!-- Tools Configuration -->
  <div class="mb-4 bg-white dark:bg-gray-800 rounded-lg shadow-md p-4">
    <div class="flex items-center gap-6">
      <span class="text-sm font-semibold text-gray-700 dark:text-gray-300">Tools:</span>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={enabledTools.webSearch} class="rounded" />
        <span class="text-sm text-gray-700 dark:text-gray-300">🔍 Web Search</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={enabledTools.flightData} class="rounded" />
        <span class="text-sm text-gray-700 dark:text-gray-300">✈️ Flight Data</span>
      </label>
      <label class="flex items-center gap-2 cursor-pointer">
        <input type="checkbox" bind:checked={enabledTools.dataVisualization} class="rounded" />
        <span class="text-sm text-gray-700 dark:text-gray-300">📊 Data Viz</span>
      </label>
    </div>

    {#if agentId === 'grok'}
      <div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
        <span class="text-sm font-semibold text-gray-700 dark:text-gray-300 mr-4">Model:</span>
        <div class="inline-flex gap-3">
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="radio" bind:group={grokModel} value="grok-4-fast-non-reasoning" />
            <span class="text-sm text-gray-700 dark:text-gray-300">⚡ Fast</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="radio" bind:group={grokModel} value="grok-4-fast-reasoning" />
            <span class="text-sm text-gray-700 dark:text-gray-300">🧠 Reasoning</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="radio" bind:group={grokModel} value="grok-code-fast-1" />
            <span class="text-sm text-gray-700 dark:text-gray-300">💻 Code</span>
          </label>
        </div>
      </div>
    {/if}

    {#if agentId === 'gemini'}
      <div class="mt-3 pt-3 border-t border-gray-200 dark:border-gray-700">
        <span class="text-sm font-semibold text-gray-700 dark:text-gray-300 mr-4">Model:</span>
        <div class="inline-flex gap-3">
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="radio" bind:group={geminiModel} value="gemini-2.5-flash-lite" />
            <span class="text-sm text-gray-700 dark:text-gray-300">⚡ Flash Lite</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="radio" bind:group={geminiModel} value="gemini-2.5-flash" />
            <span class="text-sm text-gray-700 dark:text-gray-300">🚀 Flash</span>
          </label>
          <label class="flex items-center gap-2 cursor-pointer">
            <input type="radio" bind:group={geminiModel} value="gemini-2.5-pro" />
            <span class="text-sm text-gray-700 dark:text-gray-300">🧠 Pro</span>
          </label>
        </div>
      </div>
    {/if}
  </div>

  <!-- Messages -->
  <div class="flex-1 overflow-y-auto bg-white dark:bg-gray-800 rounded-lg shadow-md p-6 mb-4 space-y-4">
    {#if messages.length === 0}
      <div class="text-center py-12 text-gray-500 dark:text-gray-400">
        <div class="text-6xl mb-4">{agentIcon}</div>
        <p class="text-lg font-medium mb-2">Start a conversation with {agentName}</p>
        <p class="text-sm">Ask questions, request research, or generate data visualizations</p>
      </div>
    {:else}
      {#each messages as message}
        <div class="flex {message.role === 'user' ? 'justify-end' : 'justify-start'}">
          <div class="max-w-3xl {message.role === 'user' ? 'bg-primary-600 text-white' : `border-2 ${colors.border} ${colors.bg}`} rounded-lg p-4">
            {#if message.role === 'assistant'}
              <div class="flex items-center gap-2 mb-2">
                <span class="text-2xl">{agentIcon}</span>
                <span class="font-semibold {colors.text}">{agentName}</span>
                {#if message.model}
                  <span class="text-xs px-2 py-0.5 bg-gray-200 dark:bg-gray-700 rounded">
                    {message.model}
                  </span>
                {/if}
                {#if message.processingTime}
                  <span class="text-xs text-gray-500 dark:text-gray-400">
                    {message.processingTime}ms
                  </span>
                {/if}
              </div>
            {/if}

            {#if message.error}
              <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3">
                <p class="text-sm text-red-700 dark:text-red-300">❌ Error: {message.error}</p>
              </div>
            {:else}
              <div class="text-sm {message.role === 'user' ? 'text-white' : 'text-gray-800 dark:text-gray-200'} whitespace-pre-wrap">
                {message.content}
              </div>
            {/if}

            <div class="text-xs {message.role === 'user' ? 'text-white/70' : 'text-gray-500 dark:text-gray-400'} mt-2">
              {message.timestamp.toLocaleTimeString()}
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Input -->
  <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-4">
    <form onsubmit={(e) => { e.preventDefault(); sendMessage(); }} class="flex gap-3">
      <input
        type="text"
        bind:value={inputMessage}
        disabled={processing}
        placeholder="Ask {agentName} anything..."
        class="flex-1 bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-lg px-4 py-3 text-gray-900 dark:text-white disabled:opacity-50"
      />
      <button
        type="submit"
        disabled={!inputMessage.trim() || processing}
        class="px-6 py-3 bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-lg font-medium transition"
      >
        {#if processing}
          ⏳ Processing...
        {:else}
          Send
        {/if}
      </button>
    </form>
  </div>
</div>
