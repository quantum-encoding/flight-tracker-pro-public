<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';
  import { theme } from '$lib/theme';

  interface Props {
    visible: boolean;
    onClose: () => void;
    contextMessage?: string;
  }

  let { visible, onClose, contextMessage = '' }: Props = $props();

  // Donation config
  interface DonationConfig {
    btc_address: string;
    eth_address: string;
    sol_address: string;
    usdt_address: string;
    xrp_address: string;
    stripe_link: string;
    github_sponsors: string;
    message: string;
  }

  let config = $state<DonationConfig | null>(null);
  let selectedCrypto = $state<'btc' | 'eth' | 'sol' | 'usdt' | 'xrp'>('eth');
  let qrCodeSvg = $state('');
  let copied = $state(false);
  let loading = $state(true);

  // Load donation config on mount
  onMount(async () => {
    try {
      config = await invoke('get_donation_config') as DonationConfig;
      await generateQrCode();
    } catch (error) {
      console.error('Failed to load donation config:', error);
    } finally {
      loading = false;
    }
  });

  // Generate QR code when crypto selection changes
  async function generateQrCode() {
    if (!config) return;

    const addressMap: Record<string, string> = {
      btc: config.btc_address,
      eth: config.eth_address,
      sol: config.sol_address,
      usdt: config.usdt_address,
      xrp: config.xrp_address,
    };
    const address = addressMap[selectedCrypto];

    try {
      // Use themed colors based on current theme
      const fgColor = $theme === 'skynet' ? '#00b4ff'
        : $theme === 'cyberpunk' ? '#00d9ff'
        : '#ffffff';
      const bgColor = '#00000000'; // Transparent

      qrCodeSvg = await invoke('generate_qr_code_themed', {
        data: address,
        fgColor,
        bgColor,
      }) as string;
    } catch (error) {
      console.error('Failed to generate QR code:', error);
    }
  }

  // Watch for crypto selection changes
  $effect(() => {
    if (config && selectedCrypto) {
      generateQrCode();
    }
  });

  // Get current address based on selection
  function getCurrentAddress(): string {
    if (!config) return '';
    const addressMap: Record<string, string> = {
      btc: config.btc_address,
      eth: config.eth_address,
      sol: config.sol_address,
      usdt: config.usdt_address,
      xrp: config.xrp_address,
    };
    return addressMap[selectedCrypto] || '';
  }

  // Get color class for selected crypto
  function getCryptoColor(): string {
    const colors: Record<string, string> = {
      btc: 'text-orange-400',
      eth: 'text-purple-400',
      sol: 'text-green-400',
      usdt: 'text-emerald-400',
      xrp: 'text-blue-400',
    };
    return colors[selectedCrypto] || 'text-gray-400';
  }

  // Copy address to clipboard
  async function copyAddress() {
    const address = getCurrentAddress();
    try {
      await navigator.clipboard.writeText(address);
      copied = true;
      setTimeout(() => copied = false, 2000);
      await invoke('record_donation_click', { method: `copy_${selectedCrypto}` });
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  }

  // Track external link clicks and open URL via Tauri
  async function trackClick(method: string, url: string) {
    await invoke('record_donation_click', { method });
    try {
      await openUrl(url);
    } catch (error) {
      // Fallback for web or if opener fails
      window.open(url, '_blank');
    }
  }

  // Theme-based colors
  const themeColors = $derived(() => {
    switch ($theme) {
      case 'skynet':
        return {
          bg: 'bg-black',
          border: 'border-blue-500',
          glow: 'rgba(0, 180, 255, 0.3)',
          primary: 'text-blue-400',
          secondary: 'text-blue-300',
          accent: '#00b4ff',
        };
      case 'cyberpunk':
        return {
          bg: 'bg-gray-950',
          border: 'border-cyan-400',
          glow: 'rgba(0, 217, 255, 0.3)',
          primary: 'text-cyan-400',
          secondary: 'text-pink-400',
          accent: '#00d9ff',
        };
      default:
        return {
          bg: 'bg-gray-900',
          border: 'border-gray-700',
          glow: 'rgba(99, 102, 241, 0.2)',
          primary: 'text-indigo-400',
          secondary: 'text-gray-300',
          accent: '#6366f1',
        };
    }
  });
</script>

{#if visible}
  <div
    class="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-50 p-4"
    onclick={onClose}
  >
    <div
      class="{themeColors().bg} {themeColors().border} border rounded-xl shadow-2xl max-w-md w-full overflow-hidden"
      style="box-shadow: 0 0 30px {themeColors().glow}"
      onclick={(e) => e.stopPropagation()}
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b {themeColors().border}/30">
        <div class="flex items-center justify-between">
          <div>
            <h3 class="text-lg font-bold {themeColors().primary}">Support Development</h3>
            <p class="text-xs text-gray-500">Open Source | Community Driven</p>
          </div>
          <button
            onclick={onClose}
            class="p-2 hover:bg-gray-800 rounded-lg transition text-gray-500"
          >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      <!-- Content -->
      <div class="p-6 space-y-5">
        {#if loading}
          <div class="text-center py-8">
            <div class="animate-spin rounded-full h-10 w-10 border-2 border-t-transparent mx-auto" style="border-color: {themeColors().accent}"></div>
            <p class="mt-3 text-sm text-gray-500">Loading...</p>
          </div>
        {:else if config}
          <!-- Context Message -->
          {#if contextMessage}
            <div class="p-3 rounded-lg bg-gray-800/50 border border-gray-700/50 text-sm text-gray-300">
              {contextMessage}
            </div>
          {/if}

          <!-- Main Message -->
          <p class="text-sm text-gray-400 text-center">
            {config.message}
          </p>

          <!-- Crypto Selection -->
          <div class="flex flex-wrap gap-2 justify-center">
            <button
              onclick={() => selectedCrypto = 'btc'}
              class="px-3 py-1.5 rounded-lg font-mono text-xs transition-all {selectedCrypto === 'btc' ? 'bg-orange-500/20 border-orange-500 text-orange-400' : 'bg-gray-800/50 border-gray-700 text-gray-500 hover:border-gray-600'} border"
            >
              BTC
            </button>
            <button
              onclick={() => selectedCrypto = 'eth'}
              class="px-3 py-1.5 rounded-lg font-mono text-xs transition-all {selectedCrypto === 'eth' ? 'bg-purple-500/20 border-purple-500 text-purple-400' : 'bg-gray-800/50 border-gray-700 text-gray-500 hover:border-gray-600'} border"
            >
              ETH
            </button>
            <button
              onclick={() => selectedCrypto = 'sol'}
              class="px-3 py-1.5 rounded-lg font-mono text-xs transition-all {selectedCrypto === 'sol' ? 'bg-green-500/20 border-green-500 text-green-400' : 'bg-gray-800/50 border-gray-700 text-gray-500 hover:border-gray-600'} border"
            >
              SOL
            </button>
            <button
              onclick={() => selectedCrypto = 'usdt'}
              class="px-3 py-1.5 rounded-lg font-mono text-xs transition-all {selectedCrypto === 'usdt' ? 'bg-emerald-500/20 border-emerald-500 text-emerald-400' : 'bg-gray-800/50 border-gray-700 text-gray-500 hover:border-gray-600'} border"
            >
              USDT
            </button>
            <button
              onclick={() => selectedCrypto = 'xrp'}
              class="px-3 py-1.5 rounded-lg font-mono text-xs transition-all {selectedCrypto === 'xrp' ? 'bg-blue-500/20 border-blue-500 text-blue-400' : 'bg-gray-800/50 border-gray-700 text-gray-500 hover:border-gray-600'} border"
            >
              XRP
            </button>
          </div>

          <!-- QR Code -->
          <div class="flex justify-center">
            <div class="p-4 bg-gray-800/30 rounded-xl border border-gray-700/50">
              {#if qrCodeSvg}
                <div class="w-48 h-48 flex items-center justify-center">
                  {@html qrCodeSvg}
                </div>
              {:else}
                <div class="w-48 h-48 flex items-center justify-center text-gray-600">
                  Generating...
                </div>
              {/if}
            </div>
          </div>

          <!-- Address with Copy -->
          <div class="flex gap-2">
            <code class="flex-1 bg-black/50 px-3 py-2 rounded-lg text-xs font-mono truncate {getCryptoColor()}">
              {getCurrentAddress()}
            </code>
            <button
              onclick={copyAddress}
              class="px-3 py-2 bg-gray-800 hover:bg-gray-700 rounded-lg transition text-sm font-medium {copied ? 'text-green-400' : 'text-gray-300'}"
            >
              {copied ? '✓' : 'Copy'}
            </button>
          </div>

          <!-- Divider -->
          <div class="flex items-center gap-3">
            <div class="flex-1 h-px bg-gray-700/50"></div>
            <span class="text-xs text-gray-600">or</span>
            <div class="flex-1 h-px bg-gray-700/50"></div>
          </div>

          <!-- Alternative Methods -->
          <div class="grid grid-cols-2 gap-3">
            <button
              onclick={() => trackClick('stripe', config!.stripe_link)}
              class="flex items-center justify-center gap-2 px-4 py-3 bg-indigo-600/20 hover:bg-indigo-600/30 border border-indigo-500/50 rounded-lg transition text-indigo-300 text-sm font-medium"
            >
              <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                <path d="M13.976 9.15c-2.172-.806-3.356-1.426-3.356-2.409 0-.831.683-1.305 1.901-1.305 2.227 0 4.515.858 6.09 1.631l.89-5.494C18.252.975 15.697 0 12.165 0 9.667 0 7.589.654 6.104 1.872 4.56 3.147 3.757 4.992 3.757 7.218c0 4.039 2.467 5.76 6.476 7.219 2.585.92 3.445 1.574 3.445 2.583 0 .98-.84 1.545-2.354 1.545-1.875 0-4.965-.921-6.99-2.109l-.9 5.555C5.175 22.99 8.385 24 11.714 24c2.641 0 4.843-.624 6.328-1.813 1.664-1.305 2.525-3.236 2.525-5.732 0-4.128-2.524-5.851-6.591-7.305z"/>
              </svg>
              Card (Stripe)
            </button>
            <button
              onclick={() => trackClick('github', config!.github_sponsors)}
              class="flex items-center justify-center gap-2 px-4 py-3 bg-gray-800/50 hover:bg-gray-700/50 border border-gray-600/50 rounded-lg transition text-gray-300 text-sm font-medium"
            >
              <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
              </svg>
              GitHub Sponsors
            </button>
          </div>

          <!-- Footer Note -->
          <p class="text-center text-xs text-gray-600">
            All donations go directly to development and server costs
          </p>
        {/if}
      </div>
    </div>
  </div>
{/if}
