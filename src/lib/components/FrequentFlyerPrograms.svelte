<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { translations } from '$lib/i18n';

  interface FrequentFlyerProgram {
    id: string;
    user_id: string;
    program_name: string;
    airline: string | null;
    alliance: string | null;
    member_number: string | null;
    tier_status: string | null;
    current_miles: number;
    lifetime_miles: number;
    tier_miles: number;
    tier_expiry_date: string | null;
    notes: string | null;
  }

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  let programs: FrequentFlyerProgram[] = $state([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedProgram: FrequentFlyerProgram | null = $state(null);

  // Form state
  let showProgramForm = $state(false);
  let editingProgram: FrequentFlyerProgram | null = $state(null);
  let formProgramName = $state('');
  let formAirline = $state('');
  let formAlliance = $state('');
  let formMemberNumber = $state('');
  let formTierStatus = $state('');
  let formCurrentMiles = $state(0);
  let formLifetimeMiles = $state(0);
  let formTierMiles = $state(0);
  let formTierExpiryDate = $state('');
  let formNotes = $state('');
  let saving = $state(false);

  onMount(async () => {
    await loadPrograms();
  });

  async function loadPrograms() {
    loading = true;
    error = null;
    try {
      programs = await invoke('list_user_ffps', { userId });
    } catch (err) {
      console.error('Failed to load FFP:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  function openCreateForm() {
    editingProgram = null;
    resetForm();
    showProgramForm = true;
  }

  function openEditForm(program: FrequentFlyerProgram) {
    editingProgram = program;
    formProgramName = program.program_name;
    formAirline = program.airline || '';
    formAlliance = program.alliance || '';
    formMemberNumber = program.member_number || '';
    formTierStatus = program.tier_status || '';
    formCurrentMiles = program.current_miles;
    formLifetimeMiles = program.lifetime_miles;
    formTierMiles = program.tier_miles;
    formTierExpiryDate = program.tier_expiry_date ? program.tier_expiry_date.split('T')[0] : '';
    formNotes = program.notes || '';
    showProgramForm = true;
  }

  function resetForm() {
    formProgramName = '';
    formAirline = '';
    formAlliance = '';
    formMemberNumber = '';
    formTierStatus = '';
    formCurrentMiles = 0;
    formLifetimeMiles = 0;
    formTierMiles = 0;
    formTierExpiryDate = '';
    formNotes = '';
  }

  async function saveProgram() {
    if (!formProgramName.trim()) {
      alert('Please enter a program name');
      return;
    }

    saving = true;
    try {
      const ffp = {
        user_id: userId,
        program_name: formProgramName.trim(),
        airline: formAirline.trim() || null,
        alliance: formAlliance.trim() || null,
        member_number: formMemberNumber.trim() || null,
        tier_status: formTierStatus.trim() || null,
        current_miles: formCurrentMiles,
        lifetime_miles: formLifetimeMiles,
        tier_miles: formTierMiles,
        tier_expiry_date: formTierExpiryDate || null,
        notes: formNotes.trim() || null,
      };

      if (editingProgram) {
        await invoke('update_ffp', {
          ffpId: editingProgram.id,
          ffp
        });
      } else {
        await invoke('create_ffp', { ffp });
      }

      showProgramForm = false;
      await loadPrograms();
    } catch (err) {
      console.error('Failed to save FFP:', err);
      alert(`Failed to save: ${err}`);
    } finally {
      saving = false;
    }
  }

  async function deleteProgram(ffpId: string) {
    if (!confirm('Are you sure you want to delete this FFP?')) {
      return;
    }

    try {
      await invoke('delete_ffp', { ffpId });
      await loadPrograms();
      if (selectedProgram?.id === ffpId) {
        selectedProgram = null;
      }
    } catch (err) {
      console.error('Failed to delete FFP:', err);
      alert(`Failed to delete: ${err}`);
    }
  }

  function formatDate(dateStr: string): string {
    const date = new Date(dateStr);
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  }

  function getTotalMiles(): number {
    return programs.reduce((sum, p) => sum + p.current_miles, 0);
  }
</script>

<div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
      🎫 {$translations('frequentFlyer.title')}
    </h2>
    <button
      onclick={openCreateForm}
      class="px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white rounded-lg font-medium transition"
    >
      + {$translations('frequentFlyer.addProgram')}
    </button>
  </div>

  <!-- Total Miles Summary -->
  {#if programs.length > 0}
    <div class="mb-6 p-4 bg-gradient-to-r from-blue-50 to-purple-50 dark:from-gray-900 dark:to-gray-800 rounded-lg">
      <h3 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{$translations('frequentFlyer.miles')}</h3>
      <p class="text-3xl font-bold text-blue-600 dark:text-blue-400">{getTotalMiles().toLocaleString()}</p>
    </div>
  {/if}

  {#if loading}
    <div class="text-center py-12">
      <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
      <p class="mt-2 text-gray-600 dark:text-gray-400">{$translations('common.loading')}...</p>
    </div>
  {:else if error}
    <div class="text-center py-12 text-red-600 dark:text-red-400">
      <p>{$translations('common.error')}: {error}</p>
    </div>
  {:else if programs.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-600 dark:text-gray-400">{$translations('frequentFlyer.title')}</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      {#each programs as program}
        <div class="border border-gray-200 dark:border-gray-700 rounded-lg p-6 hover:shadow-lg transition">
          <div class="flex items-start justify-between mb-4">
            <div>
              <h3 class="text-lg font-bold text-gray-900 dark:text-white">{program.program_name}</h3>
              {#if program.airline}
                <p class="text-sm text-gray-600 dark:text-gray-400">{program.airline}</p>
              {/if}
              {#if program.alliance}
                <p class="text-xs text-gray-500 dark:text-gray-500">{program.alliance}</p>
              {/if}
            </div>
            {#if program.tier_status}
              <span class="px-3 py-1 bg-yellow-100 dark:bg-yellow-900 text-yellow-800 dark:text-yellow-200 text-xs font-semibold rounded-full">
                {program.tier_status}
              </span>
            {/if}
          </div>

          <div class="space-y-3 mb-4">
            {#if program.member_number}
              <div>
                <p class="text-xs text-gray-500 dark:text-gray-500">{$translations('frequentFlyer.memberNumber')}</p>
                <p class="text-sm font-mono text-gray-900 dark:text-white">{program.member_number}</p>
              </div>
            {/if}

            <div class="grid grid-cols-2 gap-3">
              <div>
                <p class="text-xs text-gray-500 dark:text-gray-500">{$translations('frequentFlyer.miles')}</p>
                <p class="text-lg font-bold text-blue-600 dark:text-blue-400">{program.current_miles.toLocaleString()}</p>
              </div>
              <div>
                <p class="text-xs text-gray-500 dark:text-gray-500">{$translations('frequentFlyer.miles')}</p>
                <p class="text-lg font-bold text-purple-600 dark:text-purple-400">{program.lifetime_miles.toLocaleString()}</p>
              </div>
            </div>

            {#if program.tier_miles > 0}
              <div>
                <p class="text-xs text-gray-500 dark:text-gray-500 mb-1">{$translations('frequentFlyer.tier')}</p>
                <p class="text-sm font-semibold text-gray-900 dark:text-white">{program.tier_miles.toLocaleString()}</p>
              </div>
            {/if}

            {#if program.tier_expiry_date}
              <div>
                <p class="text-xs text-gray-500 dark:text-gray-500">{$translations('frequentFlyer.expiryDate')}</p>
                <p class="text-sm text-gray-900 dark:text-white">{formatDate(program.tier_expiry_date)}</p>
              </div>
            {/if}

            {#if program.notes}
              <div>
                <p class="text-xs text-gray-500 dark:text-gray-500">{$translations('flights.notes')}</p>
                <p class="text-sm text-gray-700 dark:text-gray-300">{program.notes}</p>
              </div>
            {/if}
          </div>

          <div class="flex gap-2 pt-4 border-t border-gray-200 dark:border-gray-700">
            <button
              onclick={() => openEditForm(program)}
              class="flex-1 px-3 py-2 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded transition"
            >
              Edit
            </button>
            <button
              onclick={() => deleteProgram(program.id)}
              class="flex-1 px-3 py-2 text-sm bg-red-600 hover:bg-red-700 text-white rounded transition"
            >
              Delete
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Program Form Modal -->
{#if showProgramForm}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-lg max-w-2xl w-full p-6">
      <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-4">
        {editingProgram ? 'Edit Program' : 'Add New Program'}
      </h3>

      <div class="space-y-4 max-h-[70vh] overflow-y-auto pr-2">
        <div>
          <label for="ffp-program-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Program Name *
          </label>
          <input
            id="ffp-program-name"
            type="text"
            bind:value={formProgramName}
            placeholder="e.g., AAdvantage, SkyMiles"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          />
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label for="ffp-airline" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Airline
            </label>
            <input
              id="ffp-airline"
              type="text"
              bind:value={formAirline}
              placeholder="e.g., American Airlines"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
          <div>
            <label for="ffp-alliance" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Alliance
            </label>
            <input
              id="ffp-alliance"
              type="text"
              bind:value={formAlliance}
              placeholder="e.g., Oneworld, Star Alliance"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label for="ffp-member-number" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Member Number
            </label>
            <input
              id="ffp-member-number"
              type="text"
              bind:value={formMemberNumber}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
          <div>
            <label for="ffp-tier-status" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Tier Status
            </label>
            <input
              id="ffp-tier-status"
              type="text"
              bind:value={formTierStatus}
              placeholder="e.g., Gold, Platinum"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <label for="ffp-current-miles" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Current Miles
            </label>
            <input
              id="ffp-current-miles"
              type="number"
              step="1"
              bind:value={formCurrentMiles}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
          <div>
            <label for="ffp-lifetime-miles" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Lifetime Miles
            </label>
            <input
              id="ffp-lifetime-miles"
              type="number"
              step="1"
              bind:value={formLifetimeMiles}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
          <div>
            <label for="ffp-tier-miles" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
              Tier Miles
            </label>
            <input
              id="ffp-tier-miles"
              type="number"
              step="1"
              bind:value={formTierMiles}
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
        </div>

        <div>
          <label for="ffp-tier-expiry" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Tier Expiry Date
          </label>
          <input
            id="ffp-tier-expiry"
            type="date"
            bind:value={formTierExpiryDate}
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          />
        </div>

        <div>
          <label for="ffp-notes" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Notes
          </label>
          <textarea
            id="ffp-notes"
            bind:value={formNotes}
            rows="3"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          ></textarea>
        </div>
      </div>

      <div class="flex gap-3 mt-6">
        <button
          onclick={() => showProgramForm = false}
          class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition"
        >
          Cancel
        </button>
        <button
          onclick={saveProgram}
          disabled={saving}
          class="flex-1 px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white rounded-lg font-medium transition disabled:opacity-50"
        >
          {saving ? 'Saving...' : 'Save Program'}
        </button>
      </div>
    </div>
  </div>
{/if}
