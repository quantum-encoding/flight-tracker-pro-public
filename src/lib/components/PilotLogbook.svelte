<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { translations } from '$lib/i18n';

  interface PilotLogbookEntry {
    id: string;
    flight_id: string;
    pic_time: number;
    sic_time: number;
    dual_time: number;
    instructor_time: number;
    solo_time: number;
    cross_country_time: number;
    day_time: number;
    night_time: number;
    actual_instrument_time: number;
    simulated_instrument_time: number;
    ground_trainer_time: number;
    day_takeoffs: number;
    day_landings: number;
    night_takeoffs: number;
    night_landings: number;
    ils_approaches: number;
    vor_approaches: number;
    ndb_approaches: number;
    gps_approaches: number;
    visual_approaches: number;
    ifr_time: number;
    vfr_time: number;
    pilot_name: string | null;
    copilot_name: string | null;
    instructor_name: string | null;
    route: string | null;
    remarks: string | null;
    endorsements: string | null;
  }

  interface PilotLogbookTotals {
    total_pic_time: number;
    total_sic_time: number;
    total_dual_time: number;
    total_instructor_time: number;
    total_solo_time: number;
    total_cross_country_time: number;
    total_day_time: number;
    total_night_time: number;
    total_actual_instrument_time: number;
    total_simulated_instrument_time: number;
    total_ground_trainer_time: number;
    total_day_takeoffs: number;
    total_day_landings: number;
    total_night_takeoffs: number;
    total_night_landings: number;
    total_ils_approaches: number;
    total_vor_approaches: number;
    total_ndb_approaches: number;
    total_gps_approaches: number;
    total_visual_approaches: number;
    total_ifr_time: number;
    total_vfr_time: number;
  }

  interface Flight {
    id: string;
    departure_airport: string;
    arrival_airport: string;
    departure_datetime: string;
    flight_number: string | null;
  }

  let entries: PilotLogbookEntry[] = $state([]);
  let totals: PilotLogbookTotals | null = $state(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedEntry: PilotLogbookEntry | null = $state(null);

  // Form state
  let showEntryForm = $state(false);
  let editingEntry: PilotLogbookEntry | null = $state(null);
  let formFlightId = $state('');
  let formPicTime = $state(0);
  let formSicTime = $state(0);
  let formDualTime = $state(0);
  let formInstructorTime = $state(0);
  let formSoloTime = $state(0);
  let formCrossCountryTime = $state(0);
  let formDayTime = $state(0);
  let formNightTime = $state(0);
  let formActualInstrumentTime = $state(0);
  let formSimulatedInstrumentTime = $state(0);
  let formGroundTrainerTime = $state(0);
  let formDayTakeoffs = $state(0);
  let formDayLandings = $state(0);
  let formNightTakeoffs = $state(0);
  let formNightLandings = $state(0);
  let formIlsApproaches = $state(0);
  let formVorApproaches = $state(0);
  let formNdbApproaches = $state(0);
  let formGpsApproaches = $state(0);
  let formVisualApproaches = $state(0);
  let formIfrTime = $state(0);
  let formVfrTime = $state(0);
  let formPilotName = $state('');
  let formCopilotName = $state('');
  let formInstructorName = $state('');
  let formRoute = $state('');
  let formRemarks = $state('');
  let formEndorsements = $state('');
  let saving = $state(false);

  // Flight selector
  let availableFlights: Flight[] = $state([]);
  let loadingFlights = $state(false);

  onMount(async () => {
    await loadEntries();
    await loadTotals();
  });

  async function loadEntries() {
    loading = true;
    error = null;
    try {
      entries = await invoke('list_all_pilot_logbook_entries');
    } catch (err) {
      console.error('Failed to load logbook entries:', err);
      error = err as string;
    } finally {
      loading = false;
    }
  }

  async function loadTotals() {
    try {
      totals = await invoke('get_pilot_logbook_totals');
    } catch (err) {
      console.error('Failed to load totals:', err);
    }
  }

  function openCreateForm() {
    editingEntry = null;
    resetForm();
    showEntryForm = true;
  }

  function openEditForm(entry: PilotLogbookEntry) {
    editingEntry = entry;
    formFlightId = entry.flight_id;
    formPicTime = entry.pic_time;
    formSicTime = entry.sic_time;
    formDualTime = entry.dual_time;
    formInstructorTime = entry.instructor_time;
    formSoloTime = entry.solo_time;
    formCrossCountryTime = entry.cross_country_time;
    formDayTime = entry.day_time;
    formNightTime = entry.night_time;
    formActualInstrumentTime = entry.actual_instrument_time;
    formSimulatedInstrumentTime = entry.simulated_instrument_time;
    formGroundTrainerTime = entry.ground_trainer_time;
    formDayTakeoffs = entry.day_takeoffs;
    formDayLandings = entry.day_landings;
    formNightTakeoffs = entry.night_takeoffs;
    formNightLandings = entry.night_landings;
    formIlsApproaches = entry.ils_approaches;
    formVorApproaches = entry.vor_approaches;
    formNdbApproaches = entry.ndb_approaches;
    formGpsApproaches = entry.gps_approaches;
    formVisualApproaches = entry.visual_approaches;
    formIfrTime = entry.ifr_time;
    formVfrTime = entry.vfr_time;
    formPilotName = entry.pilot_name || '';
    formCopilotName = entry.copilot_name || '';
    formInstructorName = entry.instructor_name || '';
    formRoute = entry.route || '';
    formRemarks = entry.remarks || '';
    formEndorsements = entry.endorsements || '';
    showEntryForm = true;
  }

  function resetForm() {
    formFlightId = '';
    formPicTime = 0;
    formSicTime = 0;
    formDualTime = 0;
    formInstructorTime = 0;
    formSoloTime = 0;
    formCrossCountryTime = 0;
    formDayTime = 0;
    formNightTime = 0;
    formActualInstrumentTime = 0;
    formSimulatedInstrumentTime = 0;
    formGroundTrainerTime = 0;
    formDayTakeoffs = 0;
    formDayLandings = 0;
    formNightTakeoffs = 0;
    formNightLandings = 0;
    formIlsApproaches = 0;
    formVorApproaches = 0;
    formNdbApproaches = 0;
    formGpsApproaches = 0;
    formVisualApproaches = 0;
    formIfrTime = 0;
    formVfrTime = 0;
    formPilotName = '';
    formCopilotName = '';
    formInstructorName = '';
    formRoute = '';
    formRemarks = '';
    formEndorsements = '';
  }

  async function saveEntry() {
    if (!formFlightId) {
      alert('Please select a flight');
      return;
    }

    saving = true;
    try {
      const entry = {
        flight_id: formFlightId,
        pic_time: formPicTime,
        sic_time: formSicTime,
        dual_time: formDualTime,
        instructor_time: formInstructorTime,
        solo_time: formSoloTime,
        cross_country_time: formCrossCountryTime,
        day_time: formDayTime,
        night_time: formNightTime,
        actual_instrument_time: formActualInstrumentTime,
        simulated_instrument_time: formSimulatedInstrumentTime,
        ground_trainer_time: formGroundTrainerTime,
        day_takeoffs: formDayTakeoffs,
        day_landings: formDayLandings,
        night_takeoffs: formNightTakeoffs,
        night_landings: formNightLandings,
        ils_approaches: formIlsApproaches,
        vor_approaches: formVorApproaches,
        ndb_approaches: formNdbApproaches,
        gps_approaches: formGpsApproaches,
        visual_approaches: formVisualApproaches,
        ifr_time: formIfrTime,
        vfr_time: formVfrTime,
        pilot_name: formPilotName || null,
        copilot_name: formCopilotName || null,
        instructor_name: formInstructorName || null,
        route: formRoute || null,
        remarks: formRemarks || null,
        endorsements: formEndorsements || null,
      };

      if (editingEntry) {
        await invoke('update_pilot_logbook_entry', {
          entryId: editingEntry.id,
          entry
        });
      } else {
        await invoke('create_pilot_logbook_entry', { entry });
      }

      showEntryForm = false;
      await loadEntries();
      await loadTotals();
    } catch (err) {
      console.error('Failed to save entry:', err);
      alert(`Failed to save entry: ${err}`);
    } finally {
      saving = false;
    }
  }

  async function deleteEntry(entryId: string) {
    if (!confirm('Are you sure you want to delete this logbook entry?')) {
      return;
    }

    try {
      await invoke('delete_pilot_logbook_entry', { entryId });
      await loadEntries();
      await loadTotals();
      if (selectedEntry?.id === entryId) {
        selectedEntry = null;
      }
    } catch (err) {
      console.error('Failed to delete entry:', err);
      alert(`Failed to delete: ${err}`);
    }
  }
</script>

<div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-6">
  <div class="flex items-center justify-between mb-6">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-white">
      ✈️ {$translations('logbook.title')}
    </h2>
    <button
      onclick={openCreateForm}
      class="px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white rounded-lg font-medium transition"
    >
      + {$translations('common.add')}
    </button>
  </div>

  <!-- Totals Dashboard -->
  {#if totals}
    <div class="mb-6 p-6 bg-gradient-to-r from-blue-50 to-indigo-50 dark:from-gray-900 dark:to-gray-800 rounded-lg border border-blue-200 dark:border-gray-700">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">{$translations('logbook.totalHours')}</h3>
      <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4">
        <div class="text-center">
          <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{totals.total_pic_time.toFixed(1)}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.pic')}</p>
        </div>
        <div class="text-center">
          <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{totals.total_sic_time.toFixed(1)}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.sic')}</p>
        </div>
        <div class="text-center">
          <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{totals.total_dual_time.toFixed(1)}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.dual')}</p>
        </div>
        <div class="text-center">
          <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{totals.total_night_time.toFixed(1)}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.nightHours')}</p>
        </div>
        <div class="text-center">
          <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{totals.total_ifr_time.toFixed(1)}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">IFR</p>
        </div>
        <div class="text-center">
          <p class="text-2xl font-bold text-blue-600 dark:text-blue-400">{totals.total_cross_country_time.toFixed(1)}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.crossCountry')}</p>
        </div>
      </div>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mt-4">
        <div class="text-center">
          <p class="text-xl font-bold text-green-600 dark:text-green-400">{totals.total_day_landings}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.dayLandings')}</p>
        </div>
        <div class="text-center">
          <p class="text-xl font-bold text-green-600 dark:text-green-400">{totals.total_night_landings}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.nightLandings')}</p>
        </div>
        <div class="text-center">
          <p class="text-xl font-bold text-purple-600 dark:text-purple-400">{totals.total_ils_approaches}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.approaches')}</p>
        </div>
        <div class="text-center">
          <p class="text-xl font-bold text-purple-600 dark:text-purple-400">{totals.total_gps_approaches}</p>
          <p class="text-xs text-gray-600 dark:text-gray-400">{$translations('logbook.approaches')}</p>
        </div>
      </div>
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
  {:else if entries.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-600 dark:text-gray-400">{$translations('logbook.title')}</p>
    </div>
  {:else}
    <div class="overflow-x-auto">
      <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
        <thead class="bg-gray-50 dark:bg-gray-900">
          <tr>
            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">{$translations('navigation.flights')}</th>
            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">{$translations('logbook.pic')}</th>
            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">{$translations('logbook.sic')}</th>
            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">{$translations('logbook.dual')}</th>
            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">{$translations('logbook.nightHours')}</th>
            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">IFR</th>
            <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">{$translations('logbook.landings')}</th>
            <th class="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">{$translations('common.actions')}</th>
          </tr>
        </thead>
        <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
          {#each entries as entry}
            <tr class="hover:bg-gray-50 dark:hover:bg-gray-700 transition">
              <td class="px-4 py-3 text-sm text-gray-900 dark:text-white font-medium">
                {entry.flight_id.substring(0, 8)}...
              </td>
              <td class="px-4 py-3 text-sm text-gray-700 dark:text-gray-300">{entry.pic_time.toFixed(1)}</td>
              <td class="px-4 py-3 text-sm text-gray-700 dark:text-gray-300">{entry.sic_time.toFixed(1)}</td>
              <td class="px-4 py-3 text-sm text-gray-700 dark:text-gray-300">{entry.dual_time.toFixed(1)}</td>
              <td class="px-4 py-3 text-sm text-gray-700 dark:text-gray-300">{entry.night_time.toFixed(1)}</td>
              <td class="px-4 py-3 text-sm text-gray-700 dark:text-gray-300">{entry.ifr_time.toFixed(1)}</td>
              <td class="px-4 py-3 text-sm text-gray-700 dark:text-gray-300">
                {entry.day_landings + entry.night_landings}
              </td>
              <td class="px-4 py-3 text-sm text-right space-x-2">
                <button
                  onclick={() => openEditForm(entry)}
                  class="text-blue-600 hover:text-blue-800 dark:text-blue-400"
                >
                  Edit
                </button>
                <button
                  onclick={() => deleteEntry(entry.id)}
                  class="text-red-600 hover:text-red-800 dark:text-red-400"
                >
                  Delete
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<!-- Entry Form Modal -->
{#if showEntryForm}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4 overflow-y-auto">
    <div class="bg-white dark:bg-gray-800 rounded-lg max-w-4xl w-full p-6 my-8">
      <h3 class="text-xl font-bold text-gray-900 dark:text-white mb-4">
        {editingEntry ? 'Edit Logbook Entry' : 'New Logbook Entry'}
      </h3>

      <div class="space-y-6 max-h-[70vh] overflow-y-auto pr-2">
        <!-- Flight Selection -->
        <div>
          <label for="flight-id-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Flight ID *
          </label>
          <input
            id="flight-id-input"
            type="text"
            bind:value={formFlightId}
            placeholder="Flight UUID"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
          />
        </div>

        <!-- Flight Times -->
        <div class="border-t pt-4">
          <h4 class="font-semibold text-gray-900 dark:text-white mb-3">Flight Times (Hours)</h4>
          <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
            <div>
              <label for="pic-time-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">PIC Time</label>
              <input id="pic-time-input" type="number" step="0.1" bind:value={formPicTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="sic-time-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">SIC Time</label>
              <input id="sic-time-input" type="number" step="0.1" bind:value={formSicTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="dual-time-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Dual Time</label>
              <input id="dual-time-input" type="number" step="0.1" bind:value={formDualTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="solo-time-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Solo Time</label>
              <input id="solo-time-input" type="number" step="0.1" bind:value={formSoloTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="day-time-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Day Time</label>
              <input id="day-time-input" type="number" step="0.1" bind:value={formDayTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="night-time-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Night Time</label>
              <input id="night-time-input" type="number" step="0.1" bind:value={formNightTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="ifr-time-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">IFR Time</label>
              <input id="ifr-time-input" type="number" step="0.1" bind:value={formIfrTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="vfr-time-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">VFR Time</label>
              <input id="vfr-time-input" type="number" step="0.1" bind:value={formVfrTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="cross-country-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Cross Country</label>
              <input id="cross-country-input" type="number" step="0.1" bind:value={formCrossCountryTime} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
          </div>
        </div>

        <!-- Takeoffs/Landings -->
        <div class="border-t pt-4">
          <h4 class="font-semibold text-gray-900 dark:text-white mb-3">Takeoffs & Landings</h4>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <label for="day-takeoffs-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Day T/O</label>
              <input id="day-takeoffs-input" type="number" bind:value={formDayTakeoffs} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="day-landings-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Day Ldg</label>
              <input id="day-landings-input" type="number" bind:value={formDayLandings} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="night-takeoffs-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Night T/O</label>
              <input id="night-takeoffs-input" type="number" bind:value={formNightTakeoffs} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="night-landings-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Night Ldg</label>
              <input id="night-landings-input" type="number" bind:value={formNightLandings} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
          </div>
        </div>

        <!-- Approaches -->
        <div class="border-t pt-4">
          <h4 class="font-semibold text-gray-900 dark:text-white mb-3">Approaches</h4>
          <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
            <div>
              <label for="ils-approaches-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">ILS</label>
              <input id="ils-approaches-input" type="number" bind:value={formIlsApproaches} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="vor-approaches-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">VOR</label>
              <input id="vor-approaches-input" type="number" bind:value={formVorApproaches} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="ndb-approaches-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">NDB</label>
              <input id="ndb-approaches-input" type="number" bind:value={formNdbApproaches} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="gps-approaches-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">GPS</label>
              <input id="gps-approaches-input" type="number" bind:value={formGpsApproaches} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="visual-approaches-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Visual</label>
              <input id="visual-approaches-input" type="number" bind:value={formVisualApproaches} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
          </div>
        </div>

        <!-- Crew & Notes -->
        <div class="border-t pt-4">
          <h4 class="font-semibold text-gray-900 dark:text-white mb-3">Crew & Notes</h4>
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
            <div>
              <label for="pilot-name-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Pilot Name</label>
              <input id="pilot-name-input" type="text" bind:value={formPilotName} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="copilot-name-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Co-Pilot</label>
              <input id="copilot-name-input" type="text" bind:value={formCopilotName} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="instructor-name-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Instructor</label>
              <input id="instructor-name-input" type="text" bind:value={formInstructorName} class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
          </div>
          <div class="space-y-3">
            <div>
              <label for="route-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Route</label>
              <input id="route-input" type="text" bind:value={formRoute} placeholder="e.g., KJFK-KLAX" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white" />
            </div>
            <div>
              <label for="remarks-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Remarks</label>
              <textarea id="remarks-input" bind:value={formRemarks} rows="2" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"></textarea>
            </div>
            <div>
              <label for="endorsements-input" class="block text-sm text-gray-700 dark:text-gray-300 mb-1">Endorsements</label>
              <textarea id="endorsements-input" bind:value={formEndorsements} rows="2" class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"></textarea>
            </div>
          </div>
        </div>
      </div>

      <div class="flex gap-3 mt-6">
        <button
          onclick={() => showEntryForm = false}
          class="flex-1 px-4 py-2 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition"
        >
          Cancel
        </button>
        <button
          onclick={saveEntry}
          disabled={saving}
          class="flex-1 px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white rounded-lg font-medium transition disabled:opacity-50"
        >
          {saving ? 'Saving...' : 'Save Entry'}
        </button>
      </div>
    </div>
  </div>
{/if}
