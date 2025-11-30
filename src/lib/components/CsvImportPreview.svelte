<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';

  interface Props {
    user: any;
    onImportComplete?: () => void;
  }

  let { user, onImportComplete }: Props = $props();

  // Import mode: 'choose' | 'interactive' | 'quick'
  let importMode = $state<'choose' | 'interactive' | 'quick'>('choose');
  let csvPath = $state<string | null>(null);
  let previewData = $state<any>(null);
  let columnMapping = $state<any>(null);
  let editedRows = $state<Map<number, any>>(new Map());
  let loading = $state(false);
  let error = $state<string | null>(null);
  let importing = $state(false);
  let importResult = $state<any>(null);

  // Filter state: 'all' | 'clean' | 'review'
  let rowFilter = $state<'all' | 'clean' | 'review'>('all');

  // Computed filtered rows based on filter state
  let filteredRows = $derived(() => {
    if (!previewData?.all_rows) return [];
    switch (rowFilter) {
      case 'clean':
        return previewData.all_rows.filter((row: any) => !row.needs_review);
      case 'review':
        return previewData.all_rows.filter((row: any) => row.needs_review);
      default:
        return previewData.all_rows;
    }
  });

  async function selectCsvFile(mode: 'interactive' | 'quick') {
    importMode = mode;

    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'CSV Files',
          extensions: ['csv']
        }]
      });

      if (selected && typeof selected === 'string') {
        csvPath = selected;

        if (mode === 'quick') {
          // Quick import - directly import without preview
          await performQuickImport(selected);
        } else {
          // Interactive mode - load preview
          await loadPreview();
        }
      } else {
        // User cancelled file selection
        importMode = 'choose';
      }
    } catch (err) {
      console.error('Failed to select file:', err);
      error = `Failed to select file: ${err}`;
    }
  }

  async function performQuickImport(filePath: string) {
    if (!user) return;

    importing = true;
    error = null;

    try {
      const result = await invoke('import_flights_from_csv', {
        userId: user.id,
        csvPath: filePath
      });

      importResult = result;

      if (onImportComplete) {
        onImportComplete();
      }
    } catch (err) {
      console.error('Quick import failed:', err);
      error = `Import failed: ${err}`;
    } finally {
      importing = false;
    }
  }

  async function loadPreview() {
    if (!csvPath) return;

    loading = true;
    error = null;
    editedRows.clear();

    try {
      const preview = await invoke<{
        headers: string[];
        all_rows: any[];
        total_rows: number;
        clean_rows?: number;
        valid_rows?: number;
        review_rows?: number;
        invalid_rows?: number;
        detected_mapping: any;
      }>('preview_csv_import', {
        csvPath,
        maxPreviewRows: 20,
      });

      previewData = preview;
      columnMapping = preview.detected_mapping;
    } catch (err) {
      console.error('Failed to load preview:', err);
      error = `Failed to load preview: ${err}`;
    } finally {
      loading = false;
    }
  }

  function editRow(rowNumber: number, field: string, value: string) {
    const existing = editedRows.get(rowNumber) || {
      row_number: rowNumber,
      date: '',
      departure_airport: '',
      arrival_airport: '',
      passengers: null,
      flight_number: null,
      aircraft_registration: null,
    };

    // Merge the edit
    const updated = { ...existing, [field]: value || null };
    editedRows.set(rowNumber, updated);
    editedRows = editedRows; // Trigger reactivity
  }

  function getEditedValue(rowNumber: number, field: string, originalValue: any): any {
    const edited = editedRows.get(rowNumber);
    return edited && edited[field] !== undefined ? edited[field] : originalValue;
  }

  function updateColumnMapping(field: string, value: number) {
    columnMapping = { ...columnMapping, [field]: value };
  }

  async function performImport() {
    if (!csvPath || !user) return;

    importing = true;
    error = null;

    try {
      const editedRowsArray = Array.from(editedRows.values());

      const result = await invoke('import_flights_from_csv_with_mapping', {
        userId: user.id,
        csvPath,
        columnMapping,
        editedRows: editedRowsArray.length > 0 ? editedRowsArray : null,
      });

      importResult = result;

      if (onImportComplete) {
        onImportComplete();
      }
    } catch (err) {
      console.error('Import failed:', err);
      error = `Import failed: ${err}`;
    } finally {
      importing = false;
    }
  }

  function reset() {
    importMode = 'choose';
    csvPath = null;
    previewData = null;
    columnMapping = null;
    editedRows.clear();
    importResult = null;
    error = null;
    rowFilter = 'all';
  }

  // Helper to get validation warnings (was validation_errors)
  function getWarnings(row: any): string[] {
    return row.validation_warnings || row.validation_errors || [];
  }

  function isRowEdited(rowNumber: number): boolean {
    return editedRows.has(rowNumber);
  }
</script>

<div class="p-6 max-w-full mx-auto">
  <div class="mb-6">
    <h1 class="text-3xl font-bold text-gray-900 dark:text-white mb-2">
      📥 Import Flights
    </h1>
    <p class="text-gray-600 dark:text-gray-400">
      Import flight data from CSV files
    </p>
  </div>

  {#if importMode === 'choose'}
    <!-- Mode Selection -->
    <div class="grid md:grid-cols-2 gap-6">
      <!-- Interactive Import Option -->
      <button
        onclick={() => selectCsvFile('interactive')}
        class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-8 text-left hover:ring-2 hover:ring-primary-500 transition group"
      >
        <div class="text-5xl mb-4">🔍</div>
        <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-2 group-hover:text-primary-600 dark:group-hover:text-primary-400">
          Interactive Import
        </h2>
        <p class="text-gray-600 dark:text-gray-400 text-sm mb-4">
          Preview your data, customize column mapping, and edit rows before importing.
          Best for complex CSV files or when you need to verify data.
        </p>
        <ul class="text-xs text-gray-500 dark:text-gray-500 space-y-1">
          <li>✓ Preview data before import</li>
          <li>✓ Custom column mapping</li>
          <li>✓ Edit individual rows</li>
          <li>✓ See validation errors</li>
        </ul>
        <div class="mt-4 text-primary-600 dark:text-primary-400 font-medium text-sm">
          Select CSV File →
        </div>
      </button>

      <!-- Quick Import Option -->
      <button
        onclick={() => selectCsvFile('quick')}
        class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-8 text-left hover:ring-2 hover:ring-green-500 transition group"
      >
        <div class="text-5xl mb-4">⚡</div>
        <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-2 group-hover:text-green-600 dark:group-hover:text-green-400">
          Quick Import
        </h2>
        <p class="text-gray-600 dark:text-gray-400 text-sm mb-4">
          Auto-detect columns and import directly. Fast and efficient for
          well-formatted CSV files with standard column headers.
        </p>
        <ul class="text-xs text-gray-500 dark:text-gray-500 space-y-1">
          <li>✓ Auto-detects columns</li>
          <li>✓ One-click import</li>
          <li>✓ Fastest option</li>
          <li>✓ Skips invalid rows</li>
        </ul>
        <div class="mt-4 text-green-600 dark:text-green-400 font-medium text-sm">
          Select CSV File →
        </div>
      </button>
    </div>

    <!-- Supported Formats Info -->
    <div class="mt-8 bg-gray-50 dark:bg-gray-900 rounded-lg p-6">
      <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
        Supported CSV Formats
      </h3>
      <div class="grid md:grid-cols-2 gap-4 text-xs text-gray-600 dark:text-gray-400">
        <div>
          <p class="font-medium text-gray-700 dark:text-gray-300 mb-1">Required columns:</p>
          <ul class="space-y-0.5">
            <li>• Date (date, departure_date, flight_date)</li>
            <li>• Origin (from, origin, departure, departure_airport)</li>
            <li>• Destination (to, destination, arrival, arrival_airport)</li>
          </ul>
        </div>
        <div>
          <p class="font-medium text-gray-700 dark:text-gray-300 mb-1">Optional columns:</p>
          <ul class="space-y-0.5">
            <li>• Passengers (passengers, pax)</li>
            <li>• Flight Number (flight_number, flight, flight_no)</li>
            <li>• Aircraft (aircraft, aircraft_registration, registration)</li>
          </ul>
        </div>
      </div>
    </div>
  {:else if importing && importMode === 'quick'}
    <!-- Quick Import Loading -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-lg p-8 text-center">
      <div class="animate-spin rounded-full h-16 w-16 border-b-2 border-green-600 mx-auto mb-4"></div>
      <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-2">
        Importing Flights...
      </h2>
      <p class="text-gray-600 dark:text-gray-400">
        Auto-detecting columns and importing your data
      </p>
    </div>
  {:else if loading}
    <!-- Loading -->
    <div class="flex items-center justify-center py-16">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
    </div>
  {:else if error}
    <!-- Error -->
    <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-6">
      <div class="flex items-start">
        <span class="text-4xl">⚠️</span>
        <div class="ml-4 flex-1">
          <h3 class="text-lg font-semibold text-red-900 dark:text-red-200 mb-2">Import Error</h3>
          <p class="text-red-700 dark:text-red-300 text-sm">{error}</p>
          <div class="flex gap-3 mt-4">
            <button
              onclick={reset}
              class="bg-gray-600 hover:bg-gray-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              Start Over
            </button>
            <button
              onclick={loadPreview}
              class="bg-red-600 hover:bg-red-700 text-white px-4 py-2 rounded-lg font-medium transition"
            >
              Try Again
            </button>
          </div>
        </div>
      </div>
    </div>
  {:else if importResult}
    <!-- Import Complete -->
    <div class="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-6">
      <div class="flex items-start">
        <span class="text-4xl">✅</span>
        <div class="ml-4 flex-1">
          <h3 class="text-lg font-semibold text-green-900 dark:text-green-200 mb-2">
            Import Complete!
          </h3>
          <div class="text-green-700 dark:text-green-300 space-y-1">
            <p>✓ Successfully imported: <strong>{importResult.success_count}</strong> flights</p>
            {#if importResult.error_count > 0}
              <p class="text-orange-600 dark:text-orange-400">
                ⚠ Skipped {importResult.error_count} rows with errors
              </p>
            {/if}
          </div>
          {#if importResult.errors && importResult.errors.length > 0}
            <details class="mt-4">
              <summary class="cursor-pointer text-sm font-medium text-green-800 dark:text-green-300">
                View Errors ({importResult.errors.length})
              </summary>
              <div class="mt-2 bg-white dark:bg-gray-800 rounded p-3 text-xs max-h-40 overflow-y-auto">
                {#each importResult.errors as err}
                  <div class="text-red-600 dark:text-red-400">{err}</div>
                {/each}
              </div>
            </details>
          {/if}
          <button
            onclick={reset}
            class="mt-4 bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded-lg font-medium transition"
          >
            Import Another File
          </button>
        </div>
      </div>
    </div>
  {:else if previewData}
    <!-- Preview & Edit -->
    <div class="space-y-6">
      <!-- Summary Stats - Clickable Filters -->
      <div class="grid grid-cols-3 gap-4">
        <button
          onclick={() => rowFilter = 'all'}
          class="bg-blue-50 dark:bg-blue-900/20 border-2 rounded-lg p-4 text-left transition hover:shadow-md {rowFilter === 'all' ? 'border-blue-500 ring-2 ring-blue-500/50' : 'border-blue-200 dark:border-blue-800'}"
        >
          <div class="text-2xl font-bold text-blue-900 dark:text-blue-200">{previewData.total_rows}</div>
          <div class="text-sm text-blue-700 dark:text-blue-300">Total Rows {rowFilter === 'all' ? '(showing)' : ''}</div>
        </button>
        <button
          onclick={() => rowFilter = 'clean'}
          class="bg-green-50 dark:bg-green-900/20 border-2 rounded-lg p-4 text-left transition hover:shadow-md {rowFilter === 'clean' ? 'border-green-500 ring-2 ring-green-500/50' : 'border-green-200 dark:border-green-800'}"
        >
          <div class="text-2xl font-bold text-green-900 dark:text-green-200">{previewData.clean_rows ?? previewData.valid_rows}</div>
          <div class="text-sm text-green-700 dark:text-green-300">Clean Rows {rowFilter === 'clean' ? '(showing)' : ''}</div>
        </button>
        <button
          onclick={() => rowFilter = 'review'}
          class="bg-orange-50 dark:bg-orange-900/20 border-2 rounded-lg p-4 text-left transition hover:shadow-md {rowFilter === 'review' ? 'border-orange-500 ring-2 ring-orange-500/50' : 'border-orange-200 dark:border-orange-800'}"
        >
          <div class="text-2xl font-bold text-orange-900 dark:text-orange-200">{previewData.review_rows ?? previewData.invalid_rows}</div>
          <div class="text-sm text-orange-700 dark:text-orange-300">Needs Review {rowFilter === 'review' ? '(showing)' : ''}</div>
        </button>
      </div>

      <!-- All rows will import notice -->
      <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 text-sm">
        <span class="font-medium text-blue-900 dark:text-blue-200">All rows will be imported.</span>
        <span class="text-blue-700 dark:text-blue-300">Rows needing review have unusual data that you can fix before or after import.</span>
      </div>

      <!-- Column Mapping -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          🗺️ Column Mapping
        </h3>
        <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Date</label>
            <select
              bind:value={columnMapping.date_column}
              class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded px-3 py-2 text-gray-900 dark:text-white"
            >
              {#each previewData.headers as header, idx}
                <option value={idx}>{header}</option>
              {/each}
            </select>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">From (Origin)</label>
            <select
              bind:value={columnMapping.from_column}
              class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded px-3 py-2 text-gray-900 dark:text-white"
            >
              {#each previewData.headers as header, idx}
                <option value={idx}>{header}</option>
              {/each}
            </select>
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">To (Destination)</label>
            <select
              bind:value={columnMapping.to_column}
              class="w-full bg-gray-50 dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded px-3 py-2 text-gray-900 dark:text-white"
            >
              {#each previewData.headers as header, idx}
                <option value={idx}>{header}</option>
              {/each}
            </select>
          </div>
        </div>
      </div>

      <!-- Data Table -->
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md overflow-hidden">
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex justify-between items-center">
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
              {#if rowFilter === 'all'}
                All Rows ({filteredRows().length})
              {:else if rowFilter === 'clean'}
                Clean Rows ({filteredRows().length})
              {:else}
                Needs Review ({filteredRows().length}) - Check source pages for manual verification
              {/if}
            </h3>
            <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">
              Click any cell to edit. Changes are highlighted in blue.
              {#if rowFilter === 'review'}
                <span class="text-orange-500 font-medium">Source page shown for reference to original PDF.</span>
              {/if}
            </p>
          </div>
          {#if filteredRows().length > 100}
            <div class="text-sm text-gray-500 dark:text-gray-400">
              Showing first 100 of {filteredRows().length} rows
            </div>
          {/if}
        </div>
        <div class="overflow-x-auto max-h-[600px] overflow-y-auto">
          <table class="w-full text-sm">
            <thead class="bg-gray-50 dark:bg-gray-900 sticky top-0">
              <tr>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Row</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Page</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Date</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">From</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">To</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Aircraft</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Passengers</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
              {#each filteredRows().slice(0, 100) as row}
                <tr class="{isRowEdited(row.row_number) ? 'bg-blue-50 dark:bg-blue-900/20' : row.needs_review ? 'bg-orange-50/50 dark:bg-orange-900/10' : ''} hover:bg-gray-50 dark:hover:bg-gray-900">
                  <td class="px-4 py-3 text-gray-900 dark:text-white font-medium whitespace-nowrap">
                    {row.row_number}
                    {#if isRowEdited(row.row_number)}
                      <span class="text-blue-600 dark:text-blue-400" title="Edited">✏️</span>
                    {/if}
                  </td>
                  <td class="px-4 py-3 text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {#if row.source_page}
                      <span class="text-xs bg-gray-200 dark:bg-gray-700 px-2 py-0.5 rounded">pg {row.source_page}</span>
                    {:else}
                      -
                    {/if}
                  </td>
                  <td class="px-4 py-3">
                    <input
                      type="text"
                      value={getEditedValue(row.row_number, 'date', row.date)}
                      oninput={(e) => editRow(row.row_number, 'date', e.currentTarget.value)}
                      class="w-full bg-transparent border-b border-transparent hover:border-gray-300 dark:hover:border-gray-600 focus:border-primary-500 px-1 py-0.5 text-gray-900 dark:text-white"
                    />
                  </td>
                  <td class="px-4 py-3">
                    <input
                      type="text"
                      value={getEditedValue(row.row_number, 'departure_airport', row.departure_airport)}
                      oninput={(e) => editRow(row.row_number, 'departure_airport', e.currentTarget.value)}
                      class="w-32 bg-transparent border-b border-transparent hover:border-gray-300 dark:hover:border-gray-600 focus:border-primary-500 px-1 py-0.5 text-gray-900 dark:text-white uppercase"
                      maxlength="4"
                    />
                  </td>
                  <td class="px-4 py-3">
                    <input
                      type="text"
                      value={getEditedValue(row.row_number, 'arrival_airport', row.arrival_airport)}
                      oninput={(e) => editRow(row.row_number, 'arrival_airport', e.currentTarget.value)}
                      class="w-32 bg-transparent border-b border-transparent hover:border-gray-300 dark:hover:border-gray-600 focus:border-primary-500 px-1 py-0.5 text-gray-900 dark:text-white uppercase"
                      maxlength="4"
                    />
                  </td>
                  <td class="px-4 py-3">
                    <input
                      type="text"
                      value={getEditedValue(row.row_number, 'aircraft_registration', row.aircraft_registration || '')}
                      oninput={(e) => editRow(row.row_number, 'aircraft_registration', e.currentTarget.value)}
                      class="w-24 bg-transparent border-b border-transparent hover:border-gray-300 dark:hover:border-gray-600 focus:border-primary-500 px-1 py-0.5 text-gray-900 dark:text-white uppercase"
                      maxlength="10"
                    />
                  </td>
                  <td class="px-4 py-3">
                    <input
                      type="text"
                      value={getEditedValue(row.row_number, 'passengers', row.passengers)}
                      oninput={(e) => editRow(row.row_number, 'passengers', e.currentTarget.value)}
                      class="w-full bg-transparent border-b border-transparent hover:border-gray-300 dark:hover:border-gray-600 focus:border-primary-500 px-1 py-0.5 text-gray-900 dark:text-white"
                    />
                  </td>
                  <td class="px-4 py-3">
                    {#if !row.needs_review}
                      <span class="text-green-600 dark:text-green-400 whitespace-nowrap">✓ Clean</span>
                    {:else}
                      <details class="text-orange-600 dark:text-orange-400">
                        <summary class="cursor-pointer whitespace-nowrap">! {getWarnings(row).length} note(s)</summary>
                        <div class="mt-1 text-xs bg-orange-50 dark:bg-orange-900/30 p-2 rounded">
                          {#each getWarnings(row) as warning}
                            <div>• {warning}</div>
                          {/each}
                          {#if row.raw_values && row.raw_values.length > 0}
                            <div class="mt-2 pt-2 border-t border-orange-200 dark:border-orange-800">
                              <span class="font-medium">Raw values:</span>
                              {#each row.raw_values as val, i}
                                <span class="text-gray-600 dark:text-gray-400">{i > 0 ? ', ' : ''}{val || '(empty)'}</span>
                              {/each}
                            </div>
                          {/if}
                        </div>
                      </details>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>

      <!-- Actions -->
      <div class="flex gap-4">
        <button
          onclick={reset}
          class="px-6 py-3 bg-gray-600 hover:bg-gray-700 text-white rounded-lg font-medium transition"
        >
          Cancel
        </button>
        <button
          onclick={performImport}
          disabled={importing || previewData.total_rows === 0}
          class="flex-1 px-6 py-3 bg-primary-600 hover:bg-primary-700 disabled:bg-gray-400 disabled:cursor-not-allowed text-white rounded-lg font-medium transition"
        >
          {#if importing}
            Importing...
          {:else}
            Import All {previewData.total_rows} Flights
          {/if}
        </button>
      </div>
    </div>
  {/if}
</div>
