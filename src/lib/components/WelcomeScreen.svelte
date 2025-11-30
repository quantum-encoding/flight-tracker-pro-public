<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { theme, type Theme } from '$lib/theme';
  import { onMount } from 'svelte';

  interface Props {
    onComplete: () => void;
  }

  let { onComplete }: Props = $props();

  // Multi-step wizard
  let step = $state<'theme' | 'profile' | 'loading' | 'complete'>('theme');
  let selectedTheme = $state<Theme>('skynet');
  let loading = $state(false);
  let error = $state('');

  // Preload progress state
  let preloadProgress = $state(0);
  let preloadTotal = $state(0);
  let preloadMessage = $state('');

  // Profile fields
  let profileType = $state<'new' | 'test'>('new');
  let name = $state('');
  let email = $state('');
  let planeRegistration = $state('');
  let pilotLicense = $state('');

  // Test user data
  const testUsers = [
    {
      id: 'epstein',
      name: 'JEFFREY EPSTEIN',
      email: 'jeevacation@gmail.com',
      planeRegistration: 'N908JE',
      pilotLicense: '',
      description: 'Financier & Private Jet Owner'
    }
  ];

  // Animation state
  let showContent = $state(false);
  let scanlineActive = $state(true);

  onMount(() => {
    // Trigger entrance animation
    setTimeout(() => showContent = true, 100);

    // Scanline effect
    const interval = setInterval(() => {
      scanlineActive = !scanlineActive;
    }, 3000);

    return () => clearInterval(interval);
  });

  async function selectTheme(t: Theme) {
    selectedTheme = t;
    await theme.set(t);
  }

  function proceedToProfile() {
    step = 'profile';
  }

  function selectTestUser(userId: string) {
    const user = testUsers.find(u => u.id === userId);
    if (user) {
      profileType = 'test';
      name = user.name;
      email = user.email;
      planeRegistration = user.planeRegistration;
      pilotLicense = user.pilotLicense;
    }
  }

  async function preloadTestData(userId: string) {
    const batchSize = 100; // Import 100 records at a time
    let batchNumber = 0;
    let isComplete = false;

    preloadMessage = 'Initializing flight data...';

    while (!isComplete) {
      try {
        const result = await invoke<{
          batch_number: number;
          total_batches: number;
          imported_this_batch: number;
          total_imported: number;
          total_rows: number;
          is_complete: boolean;
          errors: string[];
        }>('preload_test_data_batch', {
          userId,
          batchNumber,
          batchSize
        });

        preloadProgress = result.total_imported;
        preloadTotal = result.total_rows;
        preloadMessage = `Loading flight records... ${result.total_imported} / ${result.total_rows}`;
        isComplete = result.is_complete;
        batchNumber++;

        // Small delay between batches to allow UI updates
        await new Promise(resolve => setTimeout(resolve, 50));
      } catch (e) {
        console.error('Preload batch error:', e);
        preloadMessage = `Warning: Some data could not be loaded`;
        break;
      }
    }

    preloadMessage = 'Flight data loaded successfully!';
  }

  async function completeSetup() {
    if (!name.trim()) {
      error = 'Codename required for field operation';
      return;
    }

    loading = true;
    error = '';

    try {
      const user = {
        id: '',
        name: name.trim(),
        email: email.trim() || null,
        pilot_license_number: pilotLicense.trim() || null,
        license_type: null,
        license_country: null,
        created_at: '',
        updated_at: ''
      };

      const userId = await invoke<string>('create_user', { user });

      // If this is the JE test profile, preload the flight data
      if (profileType === 'test' && name === 'JEFFREY EPSTEIN') {
        step = 'loading';
        await preloadTestData(userId);
      }

      step = 'complete';

      // Brief delay for animation, then complete
      setTimeout(() => {
        onComplete();
      }, 1500);
    } catch (e) {
      error = `Operation failed: ${e}`;
      loading = false;
    }
  }

  // Theme preview colors
  const themePreview: Record<Theme, { bg: string; accent: string; glow: string; name: string; icon: string }> = {
    light: { bg: 'bg-white', accent: 'border-blue-500', glow: '', name: 'DAYLIGHT', icon: '☀️' },
    dark: { bg: 'bg-gray-900', accent: 'border-purple-500', glow: '', name: 'NIGHTFALL', icon: '🌙' },
    cyberpunk: { bg: 'bg-black', accent: 'border-cyan-400', glow: 'shadow-[0_0_30px_rgba(0,217,255,0.5)]', name: 'NEON GRID', icon: '🌃' },
    skynet: { bg: 'bg-slate-950', accent: 'border-blue-400', glow: 'shadow-[0_0_30px_rgba(59,130,246,0.5)]', name: 'SKYNET', icon: '🤖' }
  };
</script>

<!-- Massive Crosshair Overlay - Vesica Pisces Right Position -->
<div class="fixed inset-0 z-[60] pointer-events-none overflow-hidden">
  <!-- Position the crosshair to the right, like the right circle of a vesica pisces -->
  <div class="absolute top-1/2 -translate-y-1/2 -right-[20vw]">
    <svg
      width="150vh"
      height="150vh"
      viewBox="0 0 15 15"
      class="opacity-20"
      style="{selectedTheme === 'cyberpunk' || selectedTheme === 'skynet' || selectedTheme === 'dark'
        ? 'filter: drop-shadow(0 0 30px ' + (selectedTheme === 'cyberpunk' ? '#00d9ff' : '#3b82f6') + ') drop-shadow(0 0 60px ' + (selectedTheme === 'cyberpunk' ? '#00d9ff' : '#3b82f6') + ');'
        : 'filter: drop-shadow(0 0 30px #dc2626) drop-shadow(0 0 60px #dc2626);'}"
    >
      <defs>
        <!-- Radial gradient for the black center fading to colored edge -->
        <radialGradient id="crosshairGradient" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stop-color="#000000" />
          <stop offset="60%" stop-color="#000000" />
          <stop offset="85%" stop-color="{selectedTheme === 'cyberpunk' ? '#003344' : selectedTheme === 'skynet' || selectedTheme === 'dark' ? '#1e3a5f' : '#3f1515'}" />
          <stop offset="100%" stop-color="{selectedTheme === 'cyberpunk' ? '#00d9ff' : selectedTheme === 'skynet' || selectedTheme === 'dark' ? '#3b82f6' : '#dc2626'}" />
        </radialGradient>
        <!-- Glow filter -->
        <filter id="crosshairGlow" x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="0.15" result="blur"/>
          <feMerge>
            <feMergeNode in="blur"/>
            <feMergeNode in="SourceGraphic"/>
          </feMerge>
        </filter>
      </defs>
      <!-- Main crosshair shape with gradient fill -->
      <path
        fill-rule="evenodd"
        clip-rule="evenodd"
        d="M7.5 0C7.77614 0 8 0.223858 8 0.5V1.80687C10.6922 2.0935 12.8167 4.28012 13.0068 7H14.5C14.7761 7 15 7.22386 15 7.5C15 7.77614 14.7761 8 14.5 8H12.9888C12.7094 10.6244 10.6244 12.7094 8 12.9888V14.5C8 14.7761 7.77614 15 7.5 15C7.22386 15 7 14.7761 7 14.5V13.0068C4.28012 12.8167 2.0935 10.6922 1.80687 8H0.5C0.223858 8 0 7.77614 0 7.5C0 7.22386 0.223858 7 0.5 7H1.78886C1.98376 4.21166 4.21166 1.98376 7 1.78886V0.5C7 0.223858 7.22386 0 7.5 0ZM8 12.0322V9.5C8 9.22386 7.77614 9 7.5 9C7.22386 9 7 9.22386 7 9.5V12.054C4.80517 11.8689 3.04222 10.1668 2.76344 8H5.5C5.77614 8 6 7.77614 6 7.5C6 7.22386 5.77614 7 5.5 7H2.7417C2.93252 4.73662 4.73662 2.93252 7 2.7417V5.5C7 5.77614 7.22386 6 7.5 6C7.77614 6 8 5.77614 8 5.5V2.76344C10.1668 3.04222 11.8689 4.80517 12.054 7H9.5C9.22386 7 9 7.22386 9 7.5C9 7.77614 9.22386 8 9.5 8H12.0322C11.7621 10.0991 10.0991 11.7621 8 12.0322Z"
        fill="url(#crosshairGradient)"
        filter="url(#crosshairGlow)"
      />
      <!-- Outer ring stroke for extra glow definition -->
      <circle
        cx="7.5"
        cy="7.5"
        r="6.1"
        fill="none"
        stroke="{selectedTheme === 'cyberpunk' ? '#00d9ff' : selectedTheme === 'skynet' || selectedTheme === 'dark' ? '#3b82f6' : '#dc2626'}"
        stroke-width="0.08"
        opacity="0.6"
      />
    </svg>
  </div>
</div>

<!-- Full screen welcome with theme-responsive styling -->
<div class="fixed inset-0 z-50 overflow-hidden
  {selectedTheme === 'cyberpunk' ? 'bg-black' :
   selectedTheme === 'skynet' ? 'bg-slate-950' :
   selectedTheme === 'dark' ? 'bg-gray-900' : 'bg-gradient-to-br from-slate-100 to-slate-200'}">

  <!-- Animated background grid for dark themes -->
  {#if selectedTheme === 'cyberpunk' || selectedTheme === 'skynet' || selectedTheme === 'dark'}
    <div class="absolute inset-0 opacity-20">
      <div class="absolute inset-0"
           style="background-image: linear-gradient(rgba(255,255,255,0.03) 1px, transparent 1px),
                  linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px);
                  background-size: 50px 50px;">
      </div>
    </div>

    <!-- Scanline effect -->
    {#if scanlineActive}
      <div class="absolute inset-0 pointer-events-none overflow-hidden">
        <div class="absolute w-full h-[2px] bg-gradient-to-r from-transparent via-cyan-400/30 to-transparent animate-[scan_4s_linear_infinite]"
             style="animation: scan 4s linear infinite;"></div>
      </div>
    {/if}
  {/if}

  <!-- Content container -->
  <div class="relative z-10 h-full flex flex-col items-center justify-start pt-16 p-8 transition-all duration-700 {showContent ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-8'}">

    {#if step === 'theme'}
      <!-- THEME SELECTION -->
      <div class="text-center max-w-4xl mx-auto">
        <!-- ASCII Art Logo -->
        <pre class="text-xs md:text-sm font-mono mb-6 {selectedTheme === 'light' ? 'text-gray-800' : 'text-cyan-400'}" style="{selectedTheme === 'cyberpunk' ? 'text-shadow: 0 0 10px #00d9ff;' : ''}">
   ███████╗██╗     ██╗ ██████╗ ██╗  ██╗████████╗
   ██╔════╝██║     ██║██╔════╝ ██║  ██║╚══██╔══╝
█████╗  ██║     ██║██║  ███╗███████║   ██║
██╔══╝  ██║     ██║██║   ██║██╔══██║   ██║
██║     ███████╗██║╚██████╔╝██║  ██║   ██║
╚═╝     ╚══════╝╚═╝ ╚═════╝ ╚═╝  ╚═╝   ╚═╝
████████╗██████╗  █████╗  ██████╗██╗  ██╗███████╗██████╗
╚══██╔══╝██╔══██╗██╔══██╗██╔════╝██║ ██╔╝██╔════╝██╔══██╗
   ██║   ██████╔╝███████║██║     █████╔╝ █████╗  ██████╔╝
   ██║   ██╔══██╗██╔══██║██║     ██╔═██╗ ██╔══╝  ██╔══██╗
   ██║   ██║  ██║██║  ██║╚██████╗██║  ██╗███████╗██║  ██║
   ╚═╝   ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
        </pre>

        <div class="mb-6">
          <span class="inline-block px-4 py-1 rounded-full text-xs font-mono uppercase tracking-widest
            {selectedTheme === 'light' ? 'bg-gray-200 text-gray-700' : 'bg-cyan-900/30 text-cyan-400 border border-cyan-500/30'}">
            Intelligence Analysis System v2.0
          </span>
        </div>

        <h1 class="text-4xl md:text-5xl font-bold mb-4 font-mono tracking-tight
          {selectedTheme === 'light' ? 'text-gray-900' : 'text-white'}">
          SELECT VISUAL MODE
        </h1>

        <p class="text-lg mb-12 font-mono
          {selectedTheme === 'light' ? 'text-gray-600' : 'text-gray-400'}">
          > CHOOSE YOUR OPERATIONAL INTERFACE_
        </p>

        <!-- Theme Grid -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-6 mb-12">
          {#each Object.entries(themePreview) as [themeKey, preview]}
            <button
              onclick={() => selectTheme(themeKey as Theme)}
              class="group relative py-4 px-6 rounded-lg border-2 transition-all duration-300
                {preview.bg} {selectedTheme === themeKey ? preview.accent + ' ' + preview.glow : 'border-gray-600 hover:border-gray-400'}"
            >
              <p class="{themeKey === 'light' ? 'text-gray-900' : 'text-white'} text-sm font-mono text-center">
                {preview.name}
              </p>

              <!-- Selection indicator -->
              {#if selectedTheme === themeKey}
                <div class="absolute top-1 right-1 w-5 h-5 rounded-full bg-green-500 flex items-center justify-center text-white text-xs">
                  ✓
                </div>
              {/if}
            </button>
          {/each}
        </div>

        <button
          onclick={proceedToProfile}
          class="px-8 py-4 rounded-lg font-mono font-bold text-lg uppercase tracking-wider transition-all duration-300
            {selectedTheme === 'cyberpunk'
              ? 'bg-gradient-to-r from-cyan-500 to-pink-500 text-white hover:shadow-[0_0_30px_rgba(0,217,255,0.5)]'
              : selectedTheme === 'skynet'
              ? 'bg-blue-600 text-white hover:bg-blue-500 hover:shadow-[0_0_30px_rgba(59,130,246,0.5)]'
              : 'bg-primary-600 text-white hover:bg-primary-700'}"
        >
          [ ENTER ]
        </button>
      </div>

    {:else if step === 'profile'}
      <!-- PROFILE SETUP -->
      <div class="w-full max-w-xl mx-auto scale-95 -mt-10">
        <div class="text-center mb-8">
          <h1 class="text-3xl md:text-4xl font-bold mb-2 font-mono
            {selectedTheme === 'light' ? 'text-gray-900' : 'text-white'}">
            AGENT IDENTIFICATION
          </h1>
          <p class="font-mono {selectedTheme === 'light' ? 'text-gray-600' : 'text-gray-400'}">
            > ENTER CREDENTIALS FOR SYSTEM ACCESS_
          </p>
        </div>

        <!-- Profile card -->
        <div class="rounded-xl p-8 border-2
          {selectedTheme === 'light'
            ? 'bg-white border-gray-300 shadow-xl'
            : selectedTheme === 'cyberpunk'
            ? 'bg-black/80 border-cyan-500/50 shadow-[0_0_30px_rgba(0,217,255,0.2)]'
            : selectedTheme === 'skynet'
            ? 'bg-slate-900/80 border-blue-500/50 shadow-[0_0_30px_rgba(59,130,246,0.2)]'
            : 'bg-gray-800/80 border-gray-600'}">

          {#if error}
            <div class="mb-6 p-4 bg-red-500/20 border border-red-500/50 rounded-lg">
              <p class="text-red-400 font-mono text-sm">{error}</p>
            </div>
          {/if}

          <!-- Test User Quick Select -->
          <div class="mb-6">
            <label class="block text-sm font-mono mb-2 {selectedTheme === 'light' ? 'text-gray-700' : 'text-gray-300'}">
              QUICK LOAD: TEST PROFILE
            </label>
            <select
              onchange={(e) => selectTestUser((e.target as HTMLSelectElement).value)}
              class="w-full px-4 py-3 rounded-lg font-mono border-2 transition-all
                {selectedTheme === 'light'
                  ? 'bg-gray-100 border-gray-300 text-gray-900 focus:border-blue-500'
                  : selectedTheme === 'cyberpunk'
                  ? 'bg-black border-cyan-500/30 text-cyan-400 focus:border-cyan-400'
                  : selectedTheme === 'skynet'
                  ? 'bg-slate-900 border-blue-500/30 text-blue-400 focus:border-blue-400'
                  : 'bg-gray-900 border-gray-600 text-white focus:border-purple-500'}"
            >
              <option value="">-- SELECT TEST SUBJECT --</option>
              {#each testUsers as testUser}
                <option value={testUser.id}>{testUser.name} ({testUser.description})</option>
              {/each}
            </select>
          </div>

          <div class="text-center mb-6">
            <span class="text-xs font-mono {selectedTheme === 'light' ? 'text-gray-500' : 'text-gray-500'}">
              ─── OR CREATE NEW PROFILE ───
            </span>
          </div>

          <div class="space-y-4">
            <div>
              <label class="block text-sm font-mono mb-2 {selectedTheme === 'light' ? 'text-gray-700' : 'text-gray-300'}">
                CODENAME / NAME *
              </label>
              <input
                type="text"
                bind:value={name}
                placeholder="e.g., JOHN DOE"
                class="w-full px-4 py-3 rounded-lg font-mono border-2 transition-all uppercase
                  {selectedTheme === 'light'
                    ? 'bg-gray-100 border-gray-300 text-gray-900 placeholder-gray-400 focus:border-blue-500'
                    : selectedTheme === 'cyberpunk'
                    ? 'bg-black border-cyan-500/30 text-cyan-400 placeholder-cyan-700 focus:border-cyan-400'
                    : selectedTheme === 'skynet'
                    ? 'bg-slate-900 border-blue-500/30 text-blue-400 placeholder-blue-700 focus:border-blue-400'
                    : 'bg-gray-900 border-gray-600 text-white placeholder-gray-500 focus:border-purple-500'}"
              />
            </div>

            <div>
              <label class="block text-sm font-mono mb-2 {selectedTheme === 'light' ? 'text-gray-700' : 'text-gray-300'}">
                CONTACT EMAIL
              </label>
              <input
                type="email"
                bind:value={email}
                placeholder="agent@classified.gov"
                class="w-full px-4 py-3 rounded-lg font-mono border-2 transition-all
                  {selectedTheme === 'light'
                    ? 'bg-gray-100 border-gray-300 text-gray-900 placeholder-gray-400 focus:border-blue-500'
                    : selectedTheme === 'cyberpunk'
                    ? 'bg-black border-cyan-500/30 text-cyan-400 placeholder-cyan-700 focus:border-cyan-400'
                    : selectedTheme === 'skynet'
                    ? 'bg-slate-900 border-blue-500/30 text-blue-400 placeholder-blue-700 focus:border-blue-400'
                    : 'bg-gray-900 border-gray-600 text-white placeholder-gray-500 focus:border-purple-500'}"
              />
            </div>

            <div>
              <label class="block text-sm font-mono mb-2 {selectedTheme === 'light' ? 'text-gray-700' : 'text-gray-300'}">
                AIRCRAFT REGISTRATION
              </label>
              <input
                type="text"
                bind:value={planeRegistration}
                placeholder="e.g., N908JE"
                class="w-full px-4 py-3 rounded-lg font-mono border-2 transition-all uppercase
                  {selectedTheme === 'light'
                    ? 'bg-gray-100 border-gray-300 text-gray-900 placeholder-gray-400 focus:border-blue-500'
                    : selectedTheme === 'cyberpunk'
                    ? 'bg-black border-cyan-500/30 text-cyan-400 placeholder-cyan-700 focus:border-cyan-400'
                    : selectedTheme === 'skynet'
                    ? 'bg-slate-900 border-blue-500/30 text-blue-400 placeholder-blue-700 focus:border-blue-400'
                    : 'bg-gray-900 border-gray-600 text-white placeholder-gray-500 focus:border-purple-500'}"
              />
            </div>
          </div>

          <div class="mt-8 flex gap-4">
            <button
              onclick={() => step = 'theme'}
              class="flex-1 px-6 py-3 rounded-lg font-mono font-bold uppercase tracking-wider transition-all
                {selectedTheme === 'light'
                  ? 'bg-gray-200 text-gray-700 hover:bg-gray-300'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
            >
              [ BACK ]
            </button>
            <button
              onclick={completeSetup}
              disabled={loading || !name.trim()}
              class="flex-1 px-6 py-3 rounded-lg font-mono font-bold uppercase tracking-wider transition-all disabled:opacity-50 disabled:cursor-not-allowed
                {selectedTheme === 'cyberpunk'
                  ? 'bg-gradient-to-r from-cyan-500 to-pink-500 text-white hover:shadow-[0_0_20px_rgba(0,217,255,0.5)]'
                  : selectedTheme === 'skynet'
                  ? 'bg-blue-600 text-white hover:bg-blue-500'
                  : 'bg-green-600 text-white hover:bg-green-500'}"
            >
              {loading ? '[ INITIALIZING... ]' : '[ ENTER ]'}
            </button>
          </div>
        </div>

      </div>

    {:else if step === 'loading'}
      <!-- LOADING DATA ANIMATION -->
      <div class="text-center max-w-lg mx-auto">
        <div class="mb-8">
          <div class="inline-block animate-spin text-6xl">⟳</div>
        </div>
        <h1 class="text-3xl font-bold font-mono mb-4
          {selectedTheme === 'cyberpunk' ? 'text-cyan-400' : selectedTheme === 'skynet' ? 'text-blue-400' : 'text-white'}">
          LOADING FLIGHT DATA
        </h1>
        <p class="font-mono text-gray-400 mb-6">
          {preloadMessage}
        </p>

        <!-- Progress bar -->
        {#if preloadTotal > 0}
          <div class="w-full h-3 rounded-full overflow-hidden
            {selectedTheme === 'light' ? 'bg-gray-300' : 'bg-gray-800'}">
            <div
              class="h-full transition-all duration-300 rounded-full
                {selectedTheme === 'cyberpunk' ? 'bg-gradient-to-r from-cyan-500 to-pink-500' :
                 selectedTheme === 'skynet' ? 'bg-blue-500' : 'bg-green-500'}"
              style="width: {(preloadProgress / preloadTotal) * 100}%"
            ></div>
          </div>
          <p class="text-sm font-mono mt-2 {selectedTheme === 'light' ? 'text-gray-600' : 'text-gray-500'}">
            {Math.round((preloadProgress / preloadTotal) * 100)}% complete
          </p>
        {/if}
      </div>

    {:else if step === 'complete'}
      <!-- COMPLETION ANIMATION -->
      <div class="text-center">
        <div class="mb-8 animate-pulse">
          <span class="text-8xl">✓</span>
        </div>
        <h1 class="text-4xl font-bold font-mono mb-4
          {selectedTheme === 'cyberpunk' ? 'text-cyan-400' : selectedTheme === 'skynet' ? 'text-blue-400' : 'text-green-400'}">
          ACCESS GRANTED
        </h1>
        <p class="font-mono text-gray-400">
          > INITIALIZING FLIGHT TRACKER INTERFACE...
        </p>
      </div>
    {/if}
  </div>

  <!-- Introduction text box - left side -->
  {#if step === 'theme'}
    <div class="absolute left-8 top-1/2 -translate-y-1/2 max-w-xs text-left">
      <div class="p-4 rounded-lg border {selectedTheme === 'light' ? 'border-gray-300 bg-white/80' : 'border-cyan-500/30 bg-black/50'}">
        <p class="text-sm font-mono mb-3 {selectedTheme === 'light' ? 'text-gray-700' : 'text-cyan-400'}">
          A gift to the truth seekers.
        </p>
        <p class="text-xs font-mono mb-3 {selectedTheme === 'light' ? 'text-gray-600' : 'text-gray-400'}">
          Knowledge is power. This is about getting it out of our heads and doing the numbers—where the truth lives, not in speculation.
        </p>
        <p class="text-xs font-mono {selectedTheme === 'light' ? 'text-gray-600' : 'text-gray-400'}">
          Bringing credibility and accountability.
        </p>
        <p class="text-xs font-mono mt-3 italic {selectedTheme === 'light' ? 'text-gray-500' : 'text-gray-500'}">
          — Quantum Encoding Ltd
        </p>
      </div>
    </div>
  {/if}

  <!-- Classification footer -->
  <div class="absolute bottom-8 left-0 right-0 text-center">
    <p class="text-lg font-mono {selectedTheme === 'light' ? 'text-gray-600' : 'text-gray-400'}">
      CLASSIFICATION: OPEN SOURCE // INTELLIGENCE NETWORKING SYSTEM
    </p>
    <p class="text-lg font-mono {selectedTheme === 'light' ? 'text-gray-600' : 'text-gray-400'}">
      FLIGHT TRACKER PRO - INTELLIGENCE, ANALYSIS, AUTOMATION
    </p>
    <a
      href="https://quantumencoding.io"
      target="_blank"
      rel="noopener noreferrer"
      class="text-sm font-mono mt-1 inline-block transition-all hover:opacity-80
        {selectedTheme === 'light' ? 'text-gray-900' : 'text-cyan-400'}"
      style="{selectedTheme !== 'light' ? 'text-shadow: 0 0 10px #00d9ff;' : ''}"
    >
      quantumencoding.io
    </a>
  </div>
</div>

<style>
  @keyframes scan {
    0% { top: -2px; }
    100% { top: 100%; }
  }
</style>
