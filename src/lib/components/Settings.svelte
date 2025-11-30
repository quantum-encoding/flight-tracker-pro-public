<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { save } from '@tauri-apps/plugin-dialog';
  import { locale, setLocale, getCurrentLocale, translations, type Locale } from '$lib/i18n';
  import { theme, getCurrentTheme, type Theme } from '$lib/theme';
  import {
    developerMode as devModeStore,
    setDeveloperMode,
    userLocation,
    locationLoading,
    locationError,
    initUserLocation,
    detectLocationFromIP,
    setUserLocation,
    type UserLocation
  } from '$lib/stores/settings';
  import AirportEnrichment from './AirportEnrichment.svelte';
  import DonateModal from './DonateModal.svelte';

  // AI Model types
  interface ModelInfo {
    provider: string;
    model: string;
    input_cost_per_1m: number;
    output_cost_per_1m: number;
    cache_write_cost_per_1m: number;
    cache_read_cost_per_1m: number;
    websearch_cost_per_query: number;
    requests_per_minute: number;
    tokens_per_minute: number;
    context_window: number;
    region: string;
  }

  interface ProviderModels {
    provider: string;
    display_name: string;
    models: ModelInfo[];
  }

  interface Props {
    user: any;
    onReset: () => void;
  }

  let { user, onReset }: Props = $props();

  // Tab state
  let activeTab = $state<'appearance' | 'ai' | 'location' | 'data' | 'support'>('appearance');

  // Settings state
  let geminiApiKey = $state('');
  let selectedGeminiModel = $state('gemini-2.5-flash-lite');
  let deepseekApiKey = $state('');
  let selectedDeepseekModel = $state('deepseek-chat');
  let grokApiKey = $state('');
  let selectedGrokModel = $state('grok-4-fast-reasoning');
  let anthropicApiKey = $state('');
  let selectedAnthropicModel = $state('claude-sonnet-4-5-20250929');
  let exporting = $state(false);

  // AI Models data
  let aiProviders = $state<ProviderModels[]>([]);
  let loadingModels = $state(false);
  let resetting = $state(false);
  let showResetConfirm = $state(false);
  let developerMode = $state($devModeStore);
  let availableLocales = $state<string[]>(['en', 'es', 'de']);
  let currentLocale = $state<Locale>(getCurrentLocale());
  let currentTheme = $state<Theme>(getCurrentTheme());

  // Location settings
  let currentLocation = $state<UserLocation | null>(null);
  let isLocationLoading = $state(false);
  let locationErrorMsg = $state<string | null>(null);
  let locationTrackingEnabled = $state(true);
  let manualLocationMode = $state(false);
  let customLat = $state('');
  let customLng = $state('');
  let customCity = $state('');
  let customCountry = $state('');
  let showCryptoModal = $state(false);

  // Predefined cities
  const predefinedCities: { name: string; country: string; lat: number; lng: number }[] = [
    { name: 'New York', country: 'USA', lat: 40.7128, lng: -74.0060 },
    { name: 'Los Angeles', country: 'USA', lat: 34.0522, lng: -118.2437 },
    { name: 'London', country: 'UK', lat: 51.5074, lng: -0.1278 },
    { name: 'Paris', country: 'France', lat: 48.8566, lng: 2.3522 },
    { name: 'Berlin', country: 'Germany', lat: 52.5200, lng: 13.4050 },
    { name: 'Tokyo', country: 'Japan', lat: 35.6762, lng: 139.6503 },
    { name: 'Sydney', country: 'Australia', lat: -33.8688, lng: 151.2093 },
    { name: 'Dubai', country: 'UAE', lat: 25.2048, lng: 55.2708 },
    { name: 'Singapore', country: 'Singapore', lat: 1.3521, lng: 103.8198 },
    { name: 'Hong Kong', country: 'China', lat: 22.3193, lng: 114.1694 },
  ];

  function openCryptoModal() {
    showCryptoModal = true;
  }

  async function loadSettings() {
    try {
      // Load AI models from CSV
      await loadAiModels();

      // Load API keys
      const apiKey = await invoke('get_setting', { key: 'gemini_api_key' });
      if (apiKey) geminiApiKey = apiKey as string;

      const deepseekKey = await invoke('get_setting', { key: 'deepseek_api_key' });
      if (deepseekKey) deepseekApiKey = deepseekKey as string;

      const grokKey = await invoke('get_setting', { key: 'grok_api_key' });
      if (grokKey) grokApiKey = grokKey as string;

      const anthropicKey = await invoke('get_setting', { key: 'anthropic_api_key' });
      if (anthropicKey) anthropicApiKey = anthropicKey as string;

      // Load selected models
      const geminiModel = await invoke('get_setting', { key: 'selected_gemini_model' });
      if (geminiModel) selectedGeminiModel = geminiModel as string;

      const deepseekModel = await invoke('get_setting', { key: 'selected_deepseek_model' });
      if (deepseekModel) selectedDeepseekModel = deepseekModel as string;

      const grokModel = await invoke('get_setting', { key: 'selected_grok_model' });
      if (grokModel) selectedGrokModel = grokModel as string;

      const anthropicModel = await invoke('get_setting', { key: 'selected_anthropic_model' });
      if (anthropicModel) selectedAnthropicModel = anthropicModel as string;

      const devMode = await invoke('get_setting', { key: 'developer_mode' });
      developerMode = devMode === 'true';

      const locationEnabled = await invoke('get_setting', { key: 'location_tracking_enabled' });
      locationTrackingEnabled = locationEnabled !== 'false';

      if (developerMode) {
        await loadAllLocales();
      }
    } catch (error) {
      console.error('Error loading settings:', error);
    }
  }

  async function loadAiModels() {
    loadingModels = true;
    try {
      aiProviders = await invoke<ProviderModels[]>('get_ai_models');
    } catch (error) {
      console.error('Error loading AI models:', error);
      // Fallback to empty array - UI will handle gracefully
      aiProviders = [];
    } finally {
      loadingModels = false;
    }
  }

  // Helper to get models for a specific provider
  function getModelsForProvider(providerKey: string): ModelInfo[] {
    const provider = aiProviders.find(p => p.provider === providerKey);
    return provider?.models || [];
  }

  // Format model cost for display
  function formatCost(cost: number): string {
    if (cost === 0) return 'Free';
    if (cost < 0.01) return `$${cost.toFixed(4)}`;
    if (cost < 1) return `$${cost.toFixed(2)}`;
    return `$${cost.toFixed(2)}`;
  }

  // Format context window for display
  function formatContextWindow(tokens: number): string {
    if (tokens >= 1000000) return `${(tokens / 1000000).toFixed(1)}M`;
    if (tokens >= 1000) return `${(tokens / 1000).toFixed(0)}K`;
    return tokens.toString();
  }

  async function loadAllLocales() {
    try {
      const localeFiles = await import.meta.glob('$lib/i18n/locales/*.json');
      const locales = Object.keys(localeFiles)
        .map(path => {
          const match = path.match(/\/([^/]+)\.json$/);
          return match ? match[1] : null;
        })
        .filter(name => name && name !== 'template');
      availableLocales = locales as string[];
    } catch (error) {
      availableLocales = ['en', 'es', 'de'];
    }
  }

  async function loadLocationSettings() {
    const unsubLocation = userLocation.subscribe(loc => {
      currentLocation = loc;
      if (loc.source === 'manual') {
        customLat = loc.lat.toString();
        customLng = loc.lng.toString();
        customCity = loc.city || '';
        customCountry = loc.country || '';
      }
    });
    const unsubLoading = locationLoading.subscribe(val => isLocationLoading = val);
    const unsubError = locationError.subscribe(val => locationErrorMsg = val);

    if (locationTrackingEnabled) {
      await initUserLocation();
    }

    return () => {
      unsubLocation();
      unsubLoading();
      unsubError();
    };
  }

  async function handleDetectFromIP() {
    if (!locationTrackingEnabled) return;
    await detectLocationFromIP();
    manualLocationMode = false;
  }

  async function handleSelectPredefinedCity(event: Event) {
    const target = event.target as HTMLSelectElement;
    const cityName = target.value;
    if (!cityName) return;

    const city = predefinedCities.find(c => c.name === cityName);
    if (city) {
      await setUserLocation(city.lat, city.lng, city.name, city.country);
      manualLocationMode = false;
    }
  }

  async function handleSaveCustomLocation() {
    const lat = parseFloat(customLat);
    const lng = parseFloat(customLng);

    if (isNaN(lat) || isNaN(lng) || lat < -90 || lat > 90 || lng < -180 || lng > 180) {
      alert('Please enter valid coordinates');
      return;
    }

    await setUserLocation(lat, lng, customCity || undefined, customCountry || undefined);
    manualLocationMode = false;
  }

  async function toggleLocationTracking() {
    locationTrackingEnabled = !locationTrackingEnabled;
    await invoke('set_setting', { key: 'location_tracking_enabled', value: locationTrackingEnabled ? 'true' : 'false' });
  }

  async function saveSettings() {
    try {
      // Save API keys
      await invoke('set_setting', { key: 'gemini_api_key', value: geminiApiKey });
      await invoke('set_setting', { key: 'deepseek_api_key', value: deepseekApiKey });
      await invoke('set_setting', { key: 'grok_api_key', value: grokApiKey });
      await invoke('set_setting', { key: 'anthropic_api_key', value: anthropicApiKey });

      // Save selected models
      await invoke('set_setting', { key: 'selected_gemini_model', value: selectedGeminiModel });
      await invoke('set_setting', { key: 'selected_deepseek_model', value: selectedDeepseekModel });
      await invoke('set_setting', { key: 'selected_grok_model', value: selectedGrokModel });
      await invoke('set_setting', { key: 'selected_anthropic_model', value: selectedAnthropicModel });

      await invoke('set_setting', { key: 'developer_mode', value: developerMode ? 'true' : 'false' });
      alert('Settings saved successfully!');
    } catch (error) {
      alert(`Failed to save settings: ${error}`);
    }
  }

  async function handleLanguageChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const newLocale = target.value as Locale;
    currentLocale = newLocale;
    await setLocale(newLocale);
  }

  async function handleThemeChange(newTheme: Theme) {
    currentTheme = newTheme;
    await theme.set(newTheme);
  }

  async function handleDeveloperModeToggle() {
    developerMode = !developerMode;
    await setDeveloperMode(developerMode);
    if (developerMode) {
      await loadAllLocales();
    } else {
      availableLocales = ['en', 'es', 'de'];
    }
  }

  async function exportData() {
    if (!user) return;
    try {
      const filePath = await save({
        defaultPath: `flight-tracker-backup-${new Date().toISOString().split('T')[0]}.csv`,
        filters: [{ name: 'CSV', extensions: ['csv'] }]
      });

      if (filePath) {
        exporting = true;
        const count = await invoke('export_data_to_csv', { userId: user.id, exportPath: filePath });
        alert(`Successfully exported ${count} flights to ${filePath}`);
      }
    } catch (error) {
      alert(`Export failed: ${error}`);
    } finally {
      exporting = false;
    }
  }

  async function confirmReset() {
    if (resetting) return;
    resetting = true;
    try {
      await invoke('reset_database');
      onReset();
    } catch (error) {
      alert(`Reset failed: ${error}`);
      resetting = false;
      showResetConfirm = false;
    }
  }

  function cancelReset() {
    showResetConfirm = false;
  }

  $effect(() => {
    loadSettings();
    loadLocationSettings();
  });

  // Theme preview data
  const themes: { key: Theme; name: string; icon: string; bg: string; accent: string; glow?: string }[] = [
    { key: 'light', name: 'Light', icon: '☀️', bg: 'bg-white', accent: 'border-blue-500' },
    { key: 'dark', name: 'Dark', icon: '🌙', bg: 'bg-gray-900', accent: 'border-purple-500' },
    { key: 'cyberpunk', name: 'Cyberpunk', icon: '🌃', bg: 'bg-black', accent: 'border-cyan-400', glow: 'shadow-[0_0_20px_rgba(0,217,255,0.3)]' },
    { key: 'skynet', name: 'Skynet', icon: '🤖', bg: 'bg-slate-950', accent: 'border-blue-400', glow: 'shadow-[0_0_20px_rgba(59,130,246,0.3)]' }
  ];
</script>

<div class="h-full flex flex-col">
  <!-- Header -->
  <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-white">{$translations('settings.title')}</h1>
  </div>

  <!-- Tab Navigation -->
  <div class="px-6 border-b border-gray-200 dark:border-gray-700">
    <nav class="flex gap-1" aria-label="Settings tabs">
      <button
        onclick={() => activeTab = 'appearance'}
        class="px-4 py-3 text-sm font-medium rounded-t-lg transition-colors {activeTab === 'appearance'
          ? 'bg-white dark:bg-gray-800 text-primary-600 dark:text-primary-400 border-t border-x border-gray-200 dark:border-gray-700 -mb-px'
          : 'text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
      >
        🎨 Appearance
      </button>
      <button
        onclick={() => activeTab = 'ai'}
        class="px-4 py-3 text-sm font-medium rounded-t-lg transition-colors {activeTab === 'ai'
          ? 'bg-white dark:bg-gray-800 text-primary-600 dark:text-primary-400 border-t border-x border-gray-200 dark:border-gray-700 -mb-px'
          : 'text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
      >
        🤖 AI Providers
      </button>
      <button
        onclick={() => activeTab = 'location'}
        class="px-4 py-3 text-sm font-medium rounded-t-lg transition-colors {activeTab === 'location'
          ? 'bg-white dark:bg-gray-800 text-primary-600 dark:text-primary-400 border-t border-x border-gray-200 dark:border-gray-700 -mb-px'
          : 'text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
      >
        📍 Location
      </button>
      <button
        onclick={() => activeTab = 'data'}
        class="px-4 py-3 text-sm font-medium rounded-t-lg transition-colors {activeTab === 'data'
          ? 'bg-white dark:bg-gray-800 text-primary-600 dark:text-primary-400 border-t border-x border-gray-200 dark:border-gray-700 -mb-px'
          : 'text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
      >
        💾 Data
      </button>
      <button
        onclick={() => activeTab = 'support'}
        class="px-4 py-3 text-sm font-medium rounded-t-lg transition-colors {activeTab === 'support'
          ? 'bg-white dark:bg-gray-800 text-pink-600 dark:text-pink-400 border-t border-x border-gray-200 dark:border-gray-700 -mb-px'
          : 'text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'}"
      >
        ❤️ Support
      </button>
    </nav>
  </div>

  <!-- Tab Content -->
  <div class="flex-1 overflow-y-auto p-6">
    {#if activeTab === 'appearance'}
      <!-- APPEARANCE TAB -->
      <div class="max-w-3xl space-y-8">
        <!-- Theme Selection -->
        <section>
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Visual Theme</h2>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            {#each themes as t}
              <button
                onclick={() => handleThemeChange(t.key)}
                class="relative group rounded-xl border-2 transition-all overflow-hidden
                  {currentTheme === t.key ? t.accent + ' ' + (t.glow || '') : 'border-gray-300 dark:border-gray-600 hover:border-gray-400'}"
              >
                <div class="aspect-video {t.bg} p-3">
                  <div class="space-y-2">
                    <div class="h-2 rounded w-3/4 {t.key === 'light' ? 'bg-gray-300' : t.key === 'cyberpunk' ? 'bg-gradient-to-r from-cyan-500 to-pink-500' : t.key === 'skynet' ? 'bg-blue-500' : 'bg-gray-700'}"></div>
                    <div class="h-2 rounded w-1/2 {t.key === 'light' ? 'bg-gray-200' : 'bg-gray-800'}"></div>
                    <div class="h-2 rounded w-2/3 {t.key === 'light' ? 'bg-blue-500' : t.key === 'cyberpunk' ? 'bg-cyan-500' : t.key === 'skynet' ? 'bg-blue-400' : 'bg-purple-500'}"></div>
                  </div>
                </div>
                {#if currentTheme === t.key}
                  <div class="absolute top-2 right-2 w-5 h-5 rounded-full bg-green-500 flex items-center justify-center text-white text-xs">✓</div>
                {/if}
                <div class="p-2 bg-white dark:bg-gray-800 text-center">
                  <span class="text-sm font-medium text-gray-900 dark:text-white">{t.icon} {t.name}</span>
                </div>
              </button>
            {/each}
          </div>
        </section>

        <!-- Language -->
        <section>
          <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Language</h2>
          <select
            value={currentLocale}
            onchange={handleLanguageChange}
            class="w-full max-w-xs px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          >
            {#each availableLocales as lang}
              <option value={lang}>
                {lang === 'en' ? 'English' : lang === 'es' ? 'Español' : lang === 'de' ? 'Deutsch' : lang.toUpperCase()}
              </option>
            {/each}
          </select>
        </section>

        <!-- Developer Mode -->
        <section class="p-4 bg-purple-50 dark:bg-purple-900/20 border border-purple-200 dark:border-purple-800 rounded-lg">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="font-semibold text-gray-900 dark:text-white">Developer Mode</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400">Enable experimental features</p>
            </div>
            <button
              onclick={handleDeveloperModeToggle}
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {developerMode ? 'bg-purple-600' : 'bg-gray-300 dark:bg-gray-600'}"
            >
              <span class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {developerMode ? 'translate-x-6' : 'translate-x-1'}"></span>
            </button>
          </div>
        </section>
      </div>

    {:else if activeTab === 'ai'}
      <!-- AI PROVIDERS TAB -->
      <div class="max-w-3xl space-y-6">
        {#if loadingModels}
          <div class="flex items-center justify-center py-8">
            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
            <span class="ml-3 text-gray-600 dark:text-gray-400">Loading AI models...</span>
          </div>
        {:else}
          <!-- Anthropic (Claude) -->
          <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-center gap-3 mb-4">
              <div class="w-10 h-10 rounded-lg bg-amber-100 dark:bg-amber-900/30 flex items-center justify-center">
                <span class="text-xl">🧡</span>
              </div>
              <div>
                <h3 class="font-semibold text-gray-900 dark:text-white">Anthropic (Claude)</h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">Advanced reasoning & analysis</p>
              </div>
            </div>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">API Key</label>
                <input
                  type="password"
                  bind:value={anthropicApiKey}
                  placeholder="Enter Anthropic API key..."
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Model</label>
                <select
                  bind:value={selectedAnthropicModel}
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                >
                  {#each getModelsForProvider('anthropic') as model}
                    <option value={model.model}>
                      {model.model} - In: {formatCost(model.input_cost_per_1m)}/1M, Out: {formatCost(model.output_cost_per_1m)}/1M ({formatContextWindow(model.context_window)} ctx)
                    </option>
                  {/each}
                  {#if getModelsForProvider('anthropic').length === 0}
                    <option value="">No models available</option>
                  {/if}
                </select>
              </div>
            </div>
          </section>

          <!-- Google Gemini -->
          <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-center gap-3 mb-4">
              <div class="w-10 h-10 rounded-lg bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
                <span class="text-xl">🔮</span>
              </div>
              <div>
                <h3 class="font-semibold text-gray-900 dark:text-white">Google Gemini</h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">OCR & Boarding Pass Analysis</p>
              </div>
            </div>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">API Key</label>
                <input
                  type="password"
                  bind:value={geminiApiKey}
                  placeholder="Enter Gemini API key..."
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Model</label>
                <select
                  bind:value={selectedGeminiModel}
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                >
                  {#each getModelsForProvider('google') as model}
                    <option value={model.model}>
                      {model.model} - In: {formatCost(model.input_cost_per_1m)}/1M, Out: {formatCost(model.output_cost_per_1m)}/1M ({formatContextWindow(model.context_window)} ctx)
                    </option>
                  {/each}
                  {#if getModelsForProvider('google').length === 0}
                    <option value="">No models available</option>
                  {/if}
                </select>
              </div>
            </div>
          </section>

          <!-- DeepSeek -->
          <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-center gap-3 mb-4">
              <div class="w-10 h-10 rounded-lg bg-green-100 dark:bg-green-900/30 flex items-center justify-center">
                <span class="text-xl">🧠</span>
              </div>
              <div>
                <h3 class="font-semibold text-gray-900 dark:text-white">DeepSeek AI</h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">Flight Research & Context Analysis</p>
              </div>
            </div>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">API Key</label>
                <input
                  type="password"
                  bind:value={deepseekApiKey}
                  placeholder="Enter DeepSeek API key..."
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Model</label>
                <select
                  bind:value={selectedDeepseekModel}
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                >
                  {#each getModelsForProvider('deepseek') as model}
                    <option value={model.model}>
                      {model.model} - In: {formatCost(model.input_cost_per_1m)}/1M, Out: {formatCost(model.output_cost_per_1m)}/1M ({formatContextWindow(model.context_window)} ctx)
                    </option>
                  {/each}
                  {#if getModelsForProvider('deepseek').length === 0}
                    <option value="">No models available</option>
                  {/if}
                </select>
              </div>
            </div>
          </section>

          <!-- Grok (xAI) -->
          <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
            <div class="flex items-center gap-3 mb-4">
              <div class="w-10 h-10 rounded-lg bg-orange-100 dark:bg-orange-900/30 flex items-center justify-center">
                <span class="text-xl">⚡</span>
              </div>
              <div>
                <h3 class="font-semibold text-gray-900 dark:text-white">Grok (xAI)</h3>
                <p class="text-xs text-gray-500 dark:text-gray-400">Advanced Web Search & Research</p>
              </div>
            </div>
            <div class="space-y-4">
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">API Key</label>
                <input
                  type="password"
                  bind:value={grokApiKey}
                  placeholder="Enter Grok API key..."
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                />
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Model</label>
                <select
                  bind:value={selectedGrokModel}
                  class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                >
                  {#each getModelsForProvider('xai') as model}
                    <option value={model.model}>
                      {model.model} - In: {formatCost(model.input_cost_per_1m)}/1M, Out: {formatCost(model.output_cost_per_1m)}/1M ({formatContextWindow(model.context_window)} ctx)
                    </option>
                  {/each}
                  {#if getModelsForProvider('xai').length === 0}
                    <option value="">No models available</option>
                  {/if}
                </select>
                {#if getModelsForProvider('xai').find(m => m.model === selectedGrokModel)?.websearch_cost_per_query}
                  <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                    Web search: {formatCost(getModelsForProvider('xai').find(m => m.model === selectedGrokModel)?.websearch_cost_per_query || 0)}/query
                  </p>
                {/if}
              </div>
            </div>
          </section>

          <!-- Info Card -->
          <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
            <div class="flex items-start gap-3">
              <span class="text-blue-500 text-lg">ℹ️</span>
              <div class="text-sm text-blue-800 dark:text-blue-200">
                <p class="font-medium mb-1">Model Pricing Information</p>
                <p class="text-blue-700 dark:text-blue-300">
                  Costs shown are per 1 million tokens. Context window indicates maximum conversation length.
                  Models are loaded from <code class="bg-blue-100 dark:bg-blue-800 px-1 rounded">model_costs.csv</code> and can be updated without rebuilding the app.
                </p>
              </div>
            </div>
          </div>

          <button
            onclick={saveSettings}
            class="w-full bg-primary-600 hover:bg-primary-700 text-white px-4 py-3 rounded-lg font-medium transition"
          >
            Save API Settings
          </button>
        {/if}
      </div>

    {:else if activeTab === 'location'}
      <!-- LOCATION TAB -->
      <div class="max-w-3xl space-y-6">
        <!-- Location Tracking Toggle -->
        <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
          <div class="flex items-center justify-between">
            <div>
              <h3 class="font-semibold text-gray-900 dark:text-white">Location Services</h3>
              <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
                Used for network visualization and flight route display.
                <span class="text-xs text-gray-500 block mt-1">We do not track or store your location history.</span>
              </p>
            </div>
            <button
              onclick={toggleLocationTracking}
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors {locationTrackingEnabled ? 'bg-green-600' : 'bg-gray-300 dark:bg-gray-600'}"
            >
              <span class="inline-block h-4 w-4 transform rounded-full bg-white transition-transform {locationTrackingEnabled ? 'translate-x-6' : 'translate-x-1'}"></span>
            </button>
          </div>
        </section>

        {#if locationTrackingEnabled}
          <!-- Current Location Display -->
          {#if currentLocation}
            <section class="bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20 rounded-lg border border-blue-200 dark:border-blue-800 p-6">
              <div class="flex items-start justify-between">
                <div class="flex items-center gap-4">
                  <div class="w-12 h-12 rounded-full bg-blue-500 flex items-center justify-center text-white text-2xl">
                    📍
                  </div>
                  <div>
                    <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
                      {currentLocation.city || 'Unknown City'}{currentLocation.country ? `, ${currentLocation.country}` : ''}
                    </h3>
                    <div class="text-sm text-gray-600 dark:text-gray-400 space-y-1 mt-1">
                      <p>Latitude: {currentLocation.lat.toFixed(4)}°</p>
                      <p>Longitude: {currentLocation.lng.toFixed(4)}°</p>
                    </div>
                    <span class="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium mt-2
                      {currentLocation.source === 'auto' ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' :
                       currentLocation.source === 'manual' ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200' :
                       'bg-gray-100 text-gray-800 dark:bg-gray-700 dark:text-gray-200'}">
                      {currentLocation.source === 'auto' ? 'Detected from IP' : currentLocation.source === 'manual' ? 'Manually set' : 'Default'}
                    </span>
                  </div>
                </div>
                <button
                  onclick={handleDetectFromIP}
                  disabled={isLocationLoading}
                  class="text-sm text-blue-600 hover:text-blue-700 dark:text-blue-400 disabled:opacity-50"
                >
                  {isLocationLoading ? '⟳ Detecting...' : '🔄 Re-detect'}
                </button>
              </div>
            </section>
          {/if}

          {#if locationErrorMsg}
            <div class="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-3">
              <p class="text-sm text-yellow-800 dark:text-yellow-200">⚠️ {locationErrorMsg}</p>
            </div>
          {/if}

          <!-- Quick Select City -->
          <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
            <h3 class="font-semibold text-gray-900 dark:text-white mb-4">Quick Select City</h3>
            <select
              onchange={handleSelectPredefinedCity}
              disabled={isLocationLoading}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            >
              <option value="">Select a city...</option>
              {#each predefinedCities as city}
                <option value={city.name}>{city.name}, {city.country}</option>
              {/each}
            </select>
          </section>

          <!-- Manual Entry -->
          <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
            <button
              onclick={() => manualLocationMode = !manualLocationMode}
              class="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400 font-medium"
            >
              {manualLocationMode ? '▼ Hide custom coordinates' : '▶ Enter custom coordinates'}
            </button>

            {#if manualLocationMode}
              <div class="mt-4 space-y-4">
                <div class="grid grid-cols-2 gap-4">
                  <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Latitude</label>
                    <input type="number" step="0.0001" bind:value={customLat} placeholder="e.g., 40.4168"
                      class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Longitude</label>
                    <input type="number" step="0.0001" bind:value={customLng} placeholder="e.g., -3.7038"
                      class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
                  </div>
                </div>
                <div class="grid grid-cols-2 gap-4">
                  <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">City (optional)</label>
                    <input type="text" bind:value={customCity} placeholder="e.g., Madrid"
                      class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
                  </div>
                  <div>
                    <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Country (optional)</label>
                    <input type="text" bind:value={customCountry} placeholder="e.g., Spain"
                      class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
                  </div>
                </div>
                <button
                  onclick={handleSaveCustomLocation}
                  disabled={isLocationLoading || !customLat || !customLng}
                  class="bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg font-medium transition"
                >
                  Save Custom Location
                </button>
              </div>
            {/if}
          </section>
        {:else}
          <!-- Location disabled message -->
          <div class="bg-gray-50 dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6 text-center">
            <span class="text-4xl mb-4 block">🔒</span>
            <p class="text-gray-600 dark:text-gray-400">Location services are disabled. Enable above to use location-based features.</p>
          </div>
        {/if}
      </div>

    {:else if activeTab === 'data'}
      <!-- DATA TAB -->
      <div class="max-w-3xl space-y-6">
        <!-- Export -->
        <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
          <div class="flex items-center gap-3 mb-4">
            <div class="w-10 h-10 rounded-lg bg-green-100 dark:bg-green-900/30 flex items-center justify-center">
              <span class="text-xl">📥</span>
            </div>
            <div>
              <h3 class="font-semibold text-gray-900 dark:text-white">Export Data</h3>
              <p class="text-sm text-gray-500 dark:text-gray-400">Download all flight data as CSV</p>
            </div>
          </div>
          <button
            onclick={exportData}
            disabled={exporting}
            class="bg-green-600 hover:bg-green-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg font-medium transition"
          >
            {exporting ? 'Exporting...' : 'Export All Data'}
          </button>
        </section>

        <!-- Airport Enrichment -->
        <AirportEnrichment />

        <!-- Danger Zone -->
        <section class="bg-red-50 dark:bg-red-900/20 rounded-lg border-2 border-red-200 dark:border-red-800 p-6">
          <div class="flex items-center gap-3 mb-4">
            <div class="w-10 h-10 rounded-lg bg-red-100 dark:bg-red-900/30 flex items-center justify-center">
              <span class="text-xl">⚠️</span>
            </div>
            <div>
              <h3 class="font-semibold text-red-900 dark:text-red-200">Danger Zone</h3>
              <p class="text-sm text-red-700 dark:text-red-300">Irreversible destructive actions</p>
            </div>
          </div>

          {#if !showResetConfirm}
            <p class="text-sm text-red-700 dark:text-red-300 mb-4">
              Reset the entire database. This will permanently delete all users, flights, passengers, and settings.
            </p>
            <button
              onclick={() => showResetConfirm = true}
              disabled={resetting}
              class="bg-red-600 hover:bg-red-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              Reset Database
            </button>
          {:else}
            <!-- Confirmation Step -->
            <div class="bg-red-100 dark:bg-red-900/40 border border-red-300 dark:border-red-700 rounded-lg p-4 mb-4">
              <p class="text-red-900 dark:text-red-100 font-semibold mb-2">PERMANENT DATA LOSS</p>
              <p class="text-sm text-red-800 dark:text-red-200 mb-2">This will permanently delete:</p>
              <ul class="text-sm text-red-700 dark:text-red-300 list-disc list-inside mb-3 space-y-1">
                <li>All users</li>
                <li>All flights</li>
                <li>All passengers</li>
                <li>All settings</li>
              </ul>
              <p class="text-sm text-red-900 dark:text-red-100 font-medium">This action CANNOT be undone!</p>
            </div>
            <div class="flex gap-3">
              <button
                onclick={cancelReset}
                disabled={resetting}
                class="bg-gray-500 hover:bg-gray-600 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg font-medium transition"
              >
                Cancel
              </button>
              <button
                onclick={confirmReset}
                disabled={resetting}
                class="bg-red-600 hover:bg-red-700 disabled:bg-gray-400 text-white px-4 py-2 rounded-lg font-medium transition"
              >
                {resetting ? 'Resetting...' : 'Yes, Delete Everything'}
              </button>
            </div>
          {/if}
        </section>
      </div>

    {:else if activeTab === 'support'}
      <!-- SUPPORT TAB -->
      <div class="max-w-3xl space-y-6">
        <!-- About -->
        <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
          <div class="flex items-center gap-4 mb-4">
            <div class="w-14 h-14 rounded-xl bg-gradient-to-br from-pink-500 to-purple-600 flex items-center justify-center">
              <span class="text-3xl">✈️</span>
            </div>
            <div>
              <h3 class="font-bold text-xl text-gray-900 dark:text-white">Flight Tracker Pro</h3>
              <p class="text-sm text-gray-500 dark:text-gray-400">Open Source Flight Management</p>
            </div>
          </div>
          <p class="text-gray-600 dark:text-gray-300 text-sm leading-relaxed">
            Flight Tracker Pro is a free, open-source flight management application built for aviation enthusiasts,
            pilots, and frequent travelers. Track your flights, analyze patterns, manage passengers, and explore
            advanced analytics - all with a beautiful, privacy-first design.
          </p>
        </section>

        <!-- Donate Section -->
        <section class="bg-gradient-to-br from-pink-50 to-purple-50 dark:from-pink-900/20 dark:to-purple-900/20 rounded-lg border border-pink-200 dark:border-pink-800 p-6">
          <div class="flex items-center gap-3 mb-4">
            <div class="w-10 h-10 rounded-lg bg-pink-100 dark:bg-pink-900/30 flex items-center justify-center">
              <span class="text-xl">❤️</span>
            </div>
            <div>
              <h3 class="font-semibold text-gray-900 dark:text-white">Support Development</h3>
              <p class="text-sm text-pink-600 dark:text-pink-400">Help keep Flight Tracker Pro free and improving</p>
            </div>
          </div>
          <p class="text-gray-600 dark:text-gray-300 text-sm mb-4">
            If you find Flight Tracker Pro useful, consider supporting its development. Your contributions help pay for
            server costs, domain renewals, and fund new features. Every donation, no matter how small, makes a difference!
          </p>
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <a
              href="https://github.com/sponsors/quantum-encoding"
              target="_blank"
              rel="noopener noreferrer"
              class="flex items-center justify-center gap-2 px-4 py-3 bg-gray-900 hover:bg-gray-800 text-white rounded-lg transition font-medium"
            >
              <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
              </svg>
              GitHub Sponsors
            </a>
            <a
              href="https://buy.stripe.com/8x25kD0dD8hP8wK63G4ko00"
              target="_blank"
              rel="noopener noreferrer"
              class="flex items-center justify-center gap-2 px-4 py-3 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg transition font-medium"
            >
              <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M13.976 9.15c-2.172-.806-3.356-1.426-3.356-2.409 0-.831.683-1.305 1.901-1.305 2.227 0 4.515.858 6.09 1.631l.89-5.494C18.252.975 15.697 0 12.165 0 9.667 0 7.589.654 6.104 1.872 4.56 3.147 3.757 4.992 3.757 7.218c0 4.039 2.467 5.76 6.476 7.219 2.585.92 3.445 1.574 3.445 2.583 0 .98-.84 1.545-2.354 1.545-1.875 0-4.965-.921-6.99-2.109l-.9 5.555C5.175 22.99 8.385 24 11.714 24c2.641 0 4.843-.624 6.328-1.813 1.664-1.305 2.525-3.236 2.525-5.732 0-4.128-2.524-5.851-6.591-7.305z"/>
              </svg>
              Card (Stripe)
            </a>
            <button
              onclick={openCryptoModal}
              class="flex items-center justify-center gap-2 px-4 py-3 bg-amber-500 hover:bg-amber-600 text-white rounded-lg transition font-medium"
            >
              <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
                <path d="M23.638 14.904c-1.602 6.43-8.113 10.34-14.542 8.736C2.67 22.05-1.244 15.525.362 9.105 1.962 2.67 8.475-1.243 14.9.358c6.43 1.605 10.342 8.115 8.738 14.546z"/>
                <path fill="#fff" d="M17.01 10.486c.236-1.572-.96-2.417-2.596-2.98l.531-2.13-1.294-.322-.516 2.072c-.34-.085-.69-.165-1.038-.244l.52-2.083-1.294-.322-.531 2.13c-.281-.064-.558-.127-.825-.194l.001-.006-1.785-.447-.345 1.382s.96.22.94.234c.524.131.618.478.602.753l-1.447 5.806c-.063.156-.224.39-.588.3.013.019-.941-.235-.941-.235l-.644 1.483 1.686.42c.314.079.622.161.924.238l-.536 2.155 1.293.322.531-2.132c.353.096.696.184 1.03.267l-.529 2.12 1.294.323.537-2.152c2.21.418 3.871.249 4.571-1.75.564-1.609-.028-2.538-1.191-3.143.847-.195 1.485-.753 1.656-1.904z"/>
              </svg>
              Crypto (BTC/ETH/SOL)
            </button>
          </div>
        </section>

        <!-- Links Section -->
        <section class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
          <h3 class="font-semibold text-gray-900 dark:text-white mb-4">Resources</h3>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <a
              href="https://github.com/quantumencoding/flight-tracker-pro"
              target="_blank"
              rel="noopener noreferrer"
              class="flex items-center gap-3 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 transition"
            >
              <div class="w-9 h-9 rounded bg-gray-100 dark:bg-gray-700 flex items-center justify-center">
                <svg class="w-5 h-5 text-gray-700 dark:text-gray-300" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
                </svg>
              </div>
              <div>
                <p class="font-medium text-gray-900 dark:text-white text-sm">Source Code</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">View on GitHub</p>
              </div>
            </a>
            <a
              href="https://github.com/quantumencoding/flight-tracker-pro/issues"
              target="_blank"
              rel="noopener noreferrer"
              class="flex items-center gap-3 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 transition"
            >
              <div class="w-9 h-9 rounded bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
                <span class="text-lg">🐛</span>
              </div>
              <div>
                <p class="font-medium text-gray-900 dark:text-white text-sm">Report Issues</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">Bug reports & feature requests</p>
              </div>
            </a>
            <a
              href="https://quantumencoding.io"
              target="_blank"
              rel="noopener noreferrer"
              class="flex items-center gap-3 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 transition"
            >
              <div class="w-9 h-9 rounded bg-purple-100 dark:bg-purple-900/30 flex items-center justify-center">
                <span class="text-lg">🌐</span>
              </div>
              <div>
                <p class="font-medium text-gray-900 dark:text-white text-sm">Website</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">quantumencoding.io</p>
              </div>
            </a>
            <a
              href="mailto:support@quantumencoding.io"
              class="flex items-center gap-3 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 transition"
            >
              <div class="w-9 h-9 rounded bg-green-100 dark:bg-green-900/30 flex items-center justify-center">
                <span class="text-lg">✉️</span>
              </div>
              <div>
                <p class="font-medium text-gray-900 dark:text-white text-sm">Contact</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">support@quantumencoding.io</p>
              </div>
            </a>
          </div>
        </section>

        <!-- Version Info -->
        <section class="text-center text-sm text-gray-500 dark:text-gray-400">
          <p>Flight Tracker Pro v0.1.0</p>
          <p class="mt-1">Made with ❤️ by Quantum Encoding LTD</p>
        </section>
      </div>
    {/if}
  </div>
</div>

<!-- Crypto Donate Modal -->
<DonateModal
  visible={showCryptoModal}
  onClose={() => showCryptoModal = false}
  contextMessage="Support Flight Tracker Pro development with cryptocurrency"
/>
