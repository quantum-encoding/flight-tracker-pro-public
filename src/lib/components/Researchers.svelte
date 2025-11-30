<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import AgentChat from './AgentChat.svelte';

  interface Props {
    user: any;
  }

  let { user }: Props = $props();

  // Chat mode state
  let chatMode = $state<AgentType | null>(null);

  type AgentType = 'gemini' | 'grok' | 'deepseek' | 'local';
  type QueryMode = 'template' | 'manual';

  interface Agent {
    id: AgentType;
    name: string;
    description: string;
    icon: string;
    color: string;
    available: boolean;
  }

  interface QueryTemplate {
    id: string;
    name: string;
    description: string;
    template: string;
    systemPrompt: string;
    requiredVars: string[];
  }

  const queryTemplates: QueryTemplate[] = [
    {
      id: 'destination_news',
      name: 'Destination News & Events',
      description: 'Research what was happening at the destination on the travel date',
      template: 'What significant news, events, and developments occurred in {DESTINATION} on {DATE}? Include local news, major events, weather conditions, and any notable incidents.',
      systemPrompt: 'You are a travel intelligence researcher. Provide comprehensive information about the destination on the specified date, focusing on events that would be relevant to a traveler.',
      requiredVars: ['DATE', 'DESTINATION'],
    },
    {
      id: 'route_analysis',
      name: 'Route & Flight Analysis',
      description: 'Analyze the specific flight route for incidents, delays, and conditions',
      template: 'Analyze the flight route from {ORIGIN} to {DESTINATION} on {DATE}. What were the aviation conditions, any delays, weather impacts, or incidents affecting this route?',
      systemPrompt: 'You are an aviation analyst. Focus on flight operations, air traffic, weather patterns, and any incidents specific to this route and date.',
      requiredVars: ['DATE', 'ORIGIN', 'DESTINATION'],
    },
    {
      id: 'location_context',
      name: 'Travel Context Research',
      description: 'Research both origin and destination for comprehensive travel context',
      template: 'Research travel between {ORIGIN} and {DESTINATION} on {DATE}. What was the political climate, weather, major events, travel advisories, and general conditions at both locations?',
      systemPrompt: 'You are a travel safety and context analyst. Provide comprehensive information about conditions at both locations that would be relevant for travel planning and documentation.',
      requiredVars: ['DATE', 'ORIGIN', 'DESTINATION'],
    },
    {
      id: 'passenger_context',
      name: 'Passenger Research',
      description: 'Research passenger names in relation to the trip',
      template: 'Research any public information about {PASSENGERS} in relation to travel or events in {DESTINATION} around {DATE}.',
      systemPrompt: 'You are a research analyst. Find publicly available information linking these individuals to the location or date. Focus on business, conferences, public events, or news mentions.',
      requiredVars: ['DATE', 'DESTINATION', 'PASSENGERS'],
    },
    {
      id: 'custom',
      name: 'Custom Query',
      description: 'Write your own research query',
      template: '',
      systemPrompt: 'You are a helpful research assistant with access to real-time web information.',
      requiredVars: [],
    },
  ];

  const agents: Agent[] = [
    {
      id: 'gemini',
      name: 'Gemini',
      description: 'Google\'s advanced AI for OCR and analysis',
      icon: '🔮',
      color: 'blue',
      available: true,
    },
    {
      id: 'grok',
      name: 'Grok',
      description: 'X.AI\'s real-time web search and analysis',
      icon: '🧠',
      color: 'purple',
      available: true,
    },
    {
      id: 'deepseek',
      name: 'DeepSeek',
      description: 'Powerful research AI with web integration',
      icon: '🤖',
      color: 'indigo',
      available: true,
    },
    {
      id: 'local',
      name: 'Local AI',
      description: 'Run AI models locally (Coming Soon)',
      icon: '💻',
      color: 'green',
      available: false,
    },
  ];

  // State
  let queryMode: QueryMode = $state('template');
  let selectedTemplate: QueryTemplate | null = $state(queryTemplates[0]);
  let selectedAgents = $state<Set<AgentType>>(new Set());
  let selectedFlightId = $state<string | null>(null);
  let flights: any[] = $state([]);
  let loadingFlights = $state(false);

  // Derived selected flight from ID
  let selectedFlight = $derived(
    selectedFlightId ? flights.find(f => f.id === selectedFlightId) : null
  );
  let manualQuery = $state('');
  let researching = $state(false);
  let results = $state<Map<AgentType, any>>(new Map());
  let error = $state<string | null>(null);

  // Model selection for Grok
  let grokModel = $state('grok-4-fast-non-reasoning');

  // Research topics
  let topics = $state({
    news: true,
    events: true,
    weather: true,
    aviation: true,
  });

  onMount(async () => {
    await loadFlights();
  });

  async function loadFlights() {
    if (!user) return;

    loadingFlights = true;
    try {
      flights = await invoke('list_flights', {
        userId: user.id,
        limit: 100,
        offset: 0,
      });
    } catch (err) {
      console.error('Failed to load flights:', err);
    } finally {
      loadingFlights = false;
    }
  }

  // Derived count for reactivity
  let selectedAgentsCount = $derived(selectedAgents.size);

  function toggleAgent(agentId: AgentType) {
    const agent = agents.find(a => a.id === agentId);
    if (!agent?.available) return;

    if (selectedAgents.has(agentId)) {
      selectedAgents.delete(agentId);
    } else {
      selectedAgents.add(agentId);
    }
    selectedAgents = new Set(selectedAgents); // Create new Set to trigger reactivity
    results.clear();
    error = null;
  }

  function selectAllAgents() {
    selectedAgents = new Set(agents.filter(a => a.available).map(a => a.id));
  }

  function clearAgents() {
    selectedAgents = new Set();
  }

  function selectTemplate(template: QueryTemplate) {
    selectedTemplate = template;
    if (template.id === 'custom') {
      queryMode = 'manual';
    } else {
      queryMode = 'template';
    }
  }

  function getGeneratedQuery(): string {
    if (queryMode === 'manual') {
      return manualQuery;
    }

    if (!selectedTemplate || !selectedFlight) {
      return '';
    }

    let query = selectedTemplate.template;

    // Extract date from flight
    const date = selectedFlight.departure_datetime.split('T')[0];
    const origin = selectedFlight.departure_airport;
    const destination = selectedFlight.arrival_airport;

    // Get passengers if available
    let passengers = '';
    if (selectedFlight.notes && selectedFlight.notes.includes('Passengers:')) {
      passengers = selectedFlight.notes.replace('Passengers:', '').trim();
    }

    // Replace variables
    query = query.replace(/{DATE}/g, date);
    query = query.replace(/{ORIGIN}/g, origin);
    query = query.replace(/{DESTINATION}/g, destination);
    query = query.replace(/{PASSENGERS}/g, passengers || 'N/A');

    return query;
  }

  function canResearch(): boolean {
    if (selectedAgents.size === 0) return false;
    if (queryMode === 'manual') {
      return manualQuery.trim().length > 0;
    }
    return selectedFlight !== null && selectedTemplate !== null;
  }

  async function performResearch() {
    const searchQuery = getGeneratedQuery();

    if (selectedAgents.size === 0 || !searchQuery.trim()) {
      error = 'Please select at least one agent, template, and flight';
      return;
    }

    researching = true;
    error = null;
    results = new Map();

    const researchTopics = Object.entries(topics)
      .filter(([_, enabled]) => enabled)
      .map(([topic, _]) => topic);

    // Research with each selected agent
    for (const agentId of selectedAgents) {
      const startTime = Date.now();

      try {
        let researchResult: any;
        let agentModel: string | null = null;

        if (agentId === 'grok') {
          agentModel = grokModel;

          researchResult = await invoke('research_flight_with_grok', {
            flightId: selectedFlight?.id || '',
            researchTopics,
            modelName: grokModel,
          });
        } else if (agentId === 'deepseek') {
          agentModel = 'deepseek-chat';

          researchResult = {
            summary: 'DeepSeek research for general queries is not yet implemented. Please use flight-specific research.',
            key_findings: [],
          };
        } else if (agentId === 'gemini') {
          researchResult = {
            summary: 'Gemini is optimized for OCR and boarding pass analysis. For general research, please use Grok or DeepSeek.',
            key_findings: [],
          };
        }

        const processingTime = Date.now() - startTime;

        // Save the research report
        const reportInput = {
          agent_name: agentId.charAt(0).toUpperCase() + agentId.slice(1),
          agent_model: agentModel,
          search_query: searchQuery,
          research_topics: researchTopics,
          report_summary: researchResult.summary || 'Research completed',
          report_details: JSON.stringify(researchResult),
          sources: researchResult.sources ? researchResult.sources.map((s: any) => ({
            title: s.title,
            url: s.url || null,
            snippet: s.snippet || '',
          })) : [],
          confidence_score: researchResult.confidence_score || null,
          flight_id: selectedFlight?.id || null,
          report_type: queryMode === 'template' ? selectedTemplate?.id : 'custom',
          processing_time_ms: processingTime,
        };

        const reportId = await invoke('save_research_report', {
          userId: user.id,
          report: reportInput,
        });

        results.set(agentId, {
          ...researchResult,
          reportId,
          processingTime,
        });
        results = new Map(results); // Trigger reactivity
      } catch (err) {
        console.error(`Research failed for ${agentId}:`, err);
        results.set(agentId, {
          error: err as string,
        });
        results = new Map(results);
      }
    }

    researching = false;
  }

  function resetForm() {
    results = new Map();
    error = null;
    manualQuery = '';
  }

  function getAgentColorClass(color: string): string {
    const colors: Record<string, string> = {
      blue: 'border-blue-500 bg-blue-50 dark:bg-blue-900/20',
      purple: 'border-purple-500 bg-purple-50 dark:bg-purple-900/20',
      indigo: 'border-indigo-500 bg-indigo-50 dark:bg-indigo-900/20',
      green: 'border-green-500 bg-green-50 dark:bg-green-900/20',
    };
    return colors[color] || 'border-gray-500 bg-gray-50 dark:bg-gray-900/20';
  }

  function getAgentHoverClass(color: string): string {
    const colors: Record<string, string> = {
      blue: 'hover:border-blue-600',
      purple: 'hover:border-purple-600',
      indigo: 'hover:border-indigo-600',
      green: 'hover:border-green-600',
    };
    return colors[color] || 'hover:border-gray-600';
  }

  function formatFlightOption(flight: any): string {
    const date = flight.departure_datetime.split('T')[0];
    return `${date} - ${flight.departure_airport} → ${flight.arrival_airport}${flight.flight_number ? ` (${flight.flight_number})` : ''}`;
  }
</script>

{#if chatMode}
  {@const agent = agents.find(a => a.id === chatMode)}
  {#if agent && chatMode !== 'local'}
    <AgentChat
      user={user}
      agentId={chatMode}
      agentName={agent.name}
      agentIcon={agent.icon}
      agentColor={agent.color}
      onBack={() => chatMode = null}
    />
  {/if}
{:else}
  <div class="p-6 max-w-7xl mx-auto">
    <div class="mb-6">
      <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">
        AI Researchers
      </h1>
      <p class="text-gray-600 dark:text-gray-400">
        Use template-based queries to research your flights with multiple AI agents
      </p>
    </div>

    <div class="space-y-6">
    <!-- Agent Selection - Click to Chat or Select for Universal Prompt -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
      <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">
        AI Agents
      </h2>
      <p class="text-sm text-gray-600 dark:text-gray-400 mb-4">
        Click an agent to open individual chat, or select agents for universal prompt below
      </p>
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        {#each agents as agent}
          <div class="relative">
            <button
              onclick={() => { if (agent.available) chatMode = agent.id; }}
              disabled={!agent.available}
              class="w-full p-6 border-2 rounded-lg transition {selectedAgents.has(agent.id) ? getAgentColorClass(agent.color) : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'} {!agent.available ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer hover:shadow-lg'}"
            >
              <h3 class="font-semibold text-gray-900 dark:text-white mb-1 text-lg">
                {agent.name}
              </h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">{agent.description}</p>
              {#if !agent.available}
                <span class="text-xs text-gray-500 dark:text-gray-500 mt-2 block">Coming Soon</span>
              {:else}
                <span class="text-xs text-primary-600 dark:text-primary-400 mt-2 block">Click to chat →</span>
              {/if}
            </button>
            {#if agent.available}
              <div class="absolute top-2 right-2">
                <input
                  type="checkbox"
                  checked={selectedAgents.has(agent.id)}
                  onchange={(e) => { e.stopPropagation(); toggleAgent(agent.id); }}
                  class="rounded w-5 h-5"
                  title="Select for universal prompt"
                />
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <!-- Selected Agents for Universal Prompt -->
    {#if selectedAgentsCount > 0}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Selected for Universal Prompt ({selectedAgentsCount} {selectedAgentsCount === 1 ? 'agent' : 'agents'})
        </h3>
        <div class="flex flex-wrap gap-2">
          {#each Array.from(selectedAgents) as agentId}
            {@const agent = agents.find(a => a.id === agentId)}
            {#if agent}
              <div class="flex items-center gap-2 px-3 py-2 rounded-lg border-2 {getAgentColorClass(agent.color)}">
                <span class="font-medium text-gray-900 dark:text-white">{agent.name}</span>
                <button
                  onclick={() => toggleAgent(agent.id)}
                  class="ml-2 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                >
                  ✕
                </button>
              </div>
            {/if}
          {/each}
        </div>
      </div>
    {/if}

    <!-- Query Template Selection -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
        Query Template
      </h3>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        {#each queryTemplates as template}
          <button
            onclick={() => selectTemplate(template)}
            class="p-4 text-left border-2 rounded-lg transition {selectedTemplate?.id === template.id ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20' : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'}"
          >
            <h4 class="font-semibold text-gray-900 dark:text-white mb-1">{template.name}</h4>
            <p class="text-sm text-gray-600 dark:text-gray-400">{template.description}</p>
            {#if template.requiredVars.length > 0}
              <div class="mt-2 flex flex-wrap gap-1">
                {#each template.requiredVars as variable}
                  <span class="text-xs px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded">
                    {variable}
                  </span>
                {/each}
              </div>
            {/if}
          </button>
        {/each}
      </div>
    </div>

    {#if queryMode === 'template'}
      <!-- Flight Selection -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Select Flight
        </h3>
        {#if loadingFlights}
          <div class="text-center py-4 text-gray-600 dark:text-gray-400">
            Loading flights...
          </div>
        {:else if flights.length === 0}
          <div class="text-center py-4 text-gray-600 dark:text-gray-400">
            No flights found. Add some flights first.
          </div>
        {:else}
          <select
            bind:value={selectedFlightId}
            class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-lg px-4 py-3 text-gray-900 dark:text-white"
          >
            <option value={null}>Select a flight...</option>
            {#each flights as flight}
              <option value={flight.id}>
                {formatFlightOption(flight)}
              </option>
            {/each}
          </select>

          {#if selectedFlight}
            <div class="mt-4 p-4 bg-gray-50 dark:bg-gray-900 rounded-lg">
              <div class="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                <div>
                  <span class="text-gray-500 dark:text-gray-400">Date:</span>
                  <div class="font-medium text-gray-900 dark:text-white">
                    {selectedFlight.departure_datetime.split('T')[0]}
                  </div>
                </div>
                <div>
                  <span class="text-gray-500 dark:text-gray-400">Origin:</span>
                  <div class="font-medium text-gray-900 dark:text-white">
                    {selectedFlight.departure_airport}
                  </div>
                </div>
                <div>
                  <span class="text-gray-500 dark:text-gray-400">Destination:</span>
                  <div class="font-medium text-gray-900 dark:text-white">
                    {selectedFlight.arrival_airport}
                  </div>
                </div>
                <div>
                  <span class="text-gray-500 dark:text-gray-400">Distance:</span>
                  <div class="font-medium text-gray-900 dark:text-white">
                    {selectedFlight.distance_nm ? `${Math.round(selectedFlight.distance_nm)} NM` : 'N/A'}
                  </div>
                </div>
              </div>
            </div>
          {/if}
        {/if}
      </div>

      <!-- Query Preview -->
      {#if selectedFlight && selectedTemplate}
        <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
            Query Preview
          </h3>
          <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
            <div class="text-xs font-semibold text-blue-700 dark:text-blue-300 mb-2">
              SYSTEM PROMPT:
            </div>
            <div class="text-sm text-blue-900 dark:text-blue-200 mb-4 italic">
              {selectedTemplate.systemPrompt}
            </div>
            <div class="text-xs font-semibold text-blue-700 dark:text-blue-300 mb-2">
              QUERY:
            </div>
            <div class="text-sm text-blue-900 dark:text-blue-200 whitespace-pre-wrap">
              {getGeneratedQuery()}
            </div>
          </div>
        </div>
      {/if}
    {:else}
      <!-- Manual Query Input -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Custom Query
        </h3>
        <textarea
          bind:value={manualQuery}
          placeholder="Enter your research query..."
          rows="6"
          class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-lg px-4 py-3 text-gray-900 dark:text-white resize-none"
        ></textarea>
      </div>
    {/if}

    {#if selectedAgents.has('grok')}
      <!-- Grok Model Selection -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Grok Model
        </h3>
        <div class="space-y-2">
          <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition border-2 {grokModel === 'grok-4-fast-non-reasoning' ? 'border-purple-500' : 'border-transparent'}">
            <input type="radio" bind:group={grokModel} value="grok-4-fast-non-reasoning" class="mt-1" />
            <div class="ml-3">
              <span class="text-sm font-medium text-gray-900 dark:text-white">⚡ grok-4-fast-non-reasoning</span>
              <p class="text-xs text-gray-600 dark:text-gray-400">Fast, cost-effective (best for quick queries)</p>
            </div>
          </label>

          <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition border-2 {grokModel === 'grok-4-fast-reasoning' ? 'border-purple-500' : 'border-transparent'}">
            <input type="radio" bind:group={grokModel} value="grok-4-fast-reasoning" class="mt-1" />
            <div class="ml-3">
              <span class="text-sm font-medium text-gray-900 dark:text-white">grok-4-fast-reasoning</span>
              <p class="text-xs text-gray-600 dark:text-gray-400">Advanced with reasoning (best for complex analysis)</p>
            </div>
          </label>

          <label class="flex items-start p-3 bg-gray-50 dark:bg-gray-900 rounded-lg cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition border-2 {grokModel === 'grok-code-fast-1' ? 'border-purple-500' : 'border-transparent'}">
            <input type="radio" bind:group={grokModel} value="grok-code-fast-1" class="mt-1" />
            <div class="ml-3">
              <span class="text-sm font-medium text-gray-900 dark:text-white">💻 grok-code-fast-1</span>
              <p class="text-xs text-gray-600 dark:text-gray-400">Optimized for code and technical content</p>
            </div>
          </label>
        </div>
      </div>
    {/if}

    <!-- Research Topics -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
        Research Topics
      </h3>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={topics.news} class="rounded" />
          <span class="text-sm text-gray-700 dark:text-gray-300">News</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={topics.events} class="rounded" />
          <span class="text-sm text-gray-700 dark:text-gray-300">Events</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={topics.weather} class="rounded" />
          <span class="text-sm text-gray-700 dark:text-gray-300">Weather</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" bind:checked={topics.aviation} class="rounded" />
          <span class="text-sm text-gray-700 dark:text-gray-300">Aviation</span>
        </label>
      </div>
    </div>

    <!-- Action Buttons -->
    <div class="flex gap-4">
      <button
        onclick={performResearch}
        disabled={!canResearch() || researching}
        class="flex-1 bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white px-6 py-3 rounded-lg font-medium transition"
      >
        {#if researching}
          🔄 Researching...
        {:else}
          🔬 Start Research
        {/if}
      </button>
      <button
        onclick={resetForm}
        class="px-6 py-3 bg-gray-600 hover:bg-gray-700 text-white rounded-lg font-medium transition"
      >
        Reset
      </button>
    </div>

    <!-- Error Display -->
    {#if error}
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-700 dark:text-red-300">{error}</p>
      </div>
    {/if}

    <!-- Results from All Agents -->
    {#if results.size > 0}
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Research Results ({results.size} {results.size === 1 ? 'agent' : 'agents'})
        </h3>
        <div class="space-y-4">
          {#each Array.from(results.entries()) as [agentId, result]}
            {@const agent = agents.find(a => a.id === agentId)}
            {#if agent}
              <div class="border-2 {getAgentColorClass(agent.color)} rounded-lg p-4">
                <div class="flex items-center gap-3 mb-3">
                  <div class="flex-1">
                    <h4 class="font-semibold text-gray-900 dark:text-white">{agent.name}</h4>
                    {#if result.processingTime}
                      <span class="text-xs text-gray-600 dark:text-gray-400">
                        {result.processingTime}ms
                      </span>
                    {/if}
                  </div>
                </div>
                {#if result.error}
                  <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded p-3">
                    <p class="text-sm text-red-700 dark:text-red-300">Error: {result.error}</p>
                  </div>
                {:else}
                  <div class="space-y-2">
                    <p class="text-sm text-gray-800 dark:text-gray-200">{result.summary}</p>
                    {#if result.confidence_score}
                      <div class="text-xs text-gray-600 dark:text-gray-400">
                        Confidence: {Math.round(result.confidence_score * 100)}%
                      </div>
                    {/if}
                    <div class="pt-2 border-t border-gray-200 dark:border-gray-700">
                      <p class="text-xs text-gray-600 dark:text-gray-400">
                        Report saved! View in <strong>Reports</strong> tab.
                      </p>
                    </div>
                  </div>
                {/if}
              </div>
            {/if}
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>
{/if}
