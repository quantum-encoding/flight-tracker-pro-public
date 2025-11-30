<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  // Component props
  interface Props {
    isOpen?: boolean;
    onClose?: () => void;
    onComplete?: (results: any[]) => void;
  }

  let { isOpen = false, onClose, onComplete }: Props = $props();

  // OCR result type
  interface OcrResult {
    flight_number?: string;
    departure_airport?: string;
    arrival_airport?: string;
    departure_datetime?: string;
    arrival_datetime?: string;
    passenger_name?: string;
    seat_number?: string;
    booking_reference?: string;
    confidence_score?: number;
  }

  interface FileItem {
    path: string;
    name: string;
    status: 'pending' | 'processing' | 'success' | 'error';
    result?: OcrResult;
    error?: string;
  }

  interface ProgressEvent {
    current: number;
    total: number;
    status: 'processing' | 'success' | 'error';
    path: string;
  }

  // State
  let files = $state<FileItem[]>([]);
  let isProcessing = $state(false);
  let isDragging = $state(false);
  let currentStep = $state<'upload' | 'processing' | 'review'>('upload');
  let unlisten: UnlistenFn | null = null;

  // Computed
  let processedCount = $derived(files.filter(f => f.status === 'success' || f.status === 'error').length);
  let successCount = $derived(files.filter(f => f.status === 'success').length);
  let errorCount = $derived(files.filter(f => f.status === 'error').length);
  let canProceed = $derived(successCount > 0);

  // File selection
  async function selectFiles() {
    const selected = await open({
      multiple: true,
      filters: [{
        name: 'Images',
        extensions: ['png', 'jpg', 'jpeg', 'pdf', 'heic']
      }]
    });

    if (selected && Array.isArray(selected)) {
      const newFiles: FileItem[] = selected.map(path => ({
        path,
        name: path.split('/').pop() || path,
        status: 'pending'
      }));
      files = [...files, ...newFiles];
    }
  }

  // Drag and drop handlers
  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;

    if (!e.dataTransfer?.files) return;

    const droppedFiles: FileItem[] = Array.from(e.dataTransfer.files).map(file => ({
      path: (file as any).path || file.name, // In Tauri, files have a path property
      name: file.name,
      status: 'pending'
    }));

    files = [...files, ...droppedFiles];
  }

  // Process batch
  async function processBatch() {
    if (files.length === 0) return;

    isProcessing = true;
    currentStep = 'processing';

    const filePaths = files.map(f => f.path);

    try {
      const results = await invoke<Array<{ Ok?: OcrResult; Err?: string }>>('batch_analyze_boarding_passes', {
        filePaths
      });

      // Update files with results
      files.forEach((file, index) => {
        const result = results[index];
        if (result.Ok) {
          file.result = result.Ok;
          file.status = 'success';
        } else {
          file.error = result.Err || 'Unknown error';
          file.status = 'error';
        }
      });

      files = [...files];
      currentStep = 'review';
    } catch (error: any) {
      console.error('Batch processing failed:', error);
      alert(`Batch processing failed: ${error}`);
    } finally {
      isProcessing = false;
    }
  }

  // Handle progress events
  function handleProgress(event: ProgressEvent) {
    const fileIndex = files.findIndex(f => f.path === event.path);
    if (fileIndex !== -1) {
      files[fileIndex].status = event.status === 'error' ? 'error' :
                                  event.status === 'success' ? 'success' : 'processing';
      files = [...files];
    }
  }

  // Complete and close
  function completeAndClose() {
    const successfulResults = files
      .filter(f => f.status === 'success' && f.result)
      .map(f => ({ path: f.path, result: f.result! }));

    if (onComplete) {
      onComplete(successfulResults);
    }

    resetAndClose();
  }

  // Reset and close
  function resetAndClose() {
    files = [];
    currentStep = 'upload';
    isProcessing = false;
    if (onClose) onClose();
  }

  // Remove file
  function removeFile(index: number) {
    files.splice(index, 1);
    files = [...files];
  }

  // Edit result
  function editResult(index: number, field: keyof OcrResult, value: string) {
    if (files[index].result) {
      (files[index].result as any)[field] = value || undefined;
      files = [...files];
    }
  }

  // Setup progress listener
  onMount(async () => {
    unlisten = await listen<ProgressEvent>('batch-ocr:progress', (event) => {
      handleProgress(event.payload);
    });
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  // Get status icon
  function getStatusIcon(status: FileItem['status']): string {
    switch (status) {
      case 'pending': return '○';
      case 'processing': return '⟳';
      case 'success': return '✓';
      case 'error': return '✕';
      default: return '○';
    }
  }

  // Get status color
  function getStatusColor(status: FileItem['status']): string {
    switch (status) {
      case 'pending': return 'text-slate-400';
      case 'processing': return 'text-blue-400 animate-spin';
      case 'success': return 'text-green-400';
      case 'error': return 'text-red-400';
      default: return 'text-slate-400';
    }
  }
</script>

{#if isOpen}
  <!-- Modal Backdrop -->
  <div class="fixed inset-0 bg-black/80 z-50 flex items-center justify-center p-4">
    <!-- Modal Container -->
    <div class="bg-slate-900 rounded-lg shadow-2xl w-full max-w-4xl max-h-[90vh] flex flex-col border border-slate-700">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-slate-800">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-blue-600/20 flex items-center justify-center">
            <svg class="w-6 h-6 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
            </svg>
          </div>
          <div>
            <h2 class="text-xl font-bold text-white">Batch OCR Import</h2>
            <p class="text-xs text-slate-400">Import multiple boarding passes at once</p>
          </div>
        </div>
        <button
          onclick={resetAndClose}
          class="text-slate-400 hover:text-white transition-colors"
          disabled={isProcessing}
          aria-label="Close"
        >
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <!-- Progress Steps -->
      <div class="px-6 py-4 bg-slate-950/50 border-b border-slate-800">
        <div class="flex items-center justify-between">
          {#each [
            { key: 'upload', label: 'Upload Files', icon: '📁' },
            { key: 'processing', label: 'Processing', icon: '⚡' },
            { key: 'review', label: 'Review', icon: '✓' }
          ] as step, i}
            <div class="flex items-center gap-2">
              <div class={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold border-2 ${
                currentStep === step.key ? 'bg-blue-600 border-blue-600 text-white' :
                i < ['upload', 'processing', 'review'].indexOf(currentStep) ? 'bg-green-600 border-green-600 text-white' :
                'bg-slate-800 border-slate-700 text-slate-400'
              }`}>
                {step.icon}
              </div>
              <span class={`text-sm font-medium ${
                currentStep === step.key ? 'text-white' : 'text-slate-400'
              }`}>{step.label}</span>
            </div>
            {#if i < 2}
              <div class={`flex-1 h-0.5 mx-4 ${
                i < ['upload', 'processing', 'review'].indexOf(currentStep) ? 'bg-green-600' : 'bg-slate-700'
              }`}></div>
            {/if}
          {/each}
        </div>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-y-auto p-6">
        {#if currentStep === 'upload'}
          <!-- Upload Step -->
          <div class="space-y-4">
            <!-- Drag-Drop Zone -->
            <div
              class={`border-2 border-dashed rounded-lg p-12 text-center transition-all ${
                isDragging
                  ? 'border-blue-500 bg-blue-500/10'
                  : 'border-slate-700 bg-slate-950/50 hover:border-slate-600'
              }`}
              ondragenter={handleDragEnter}
              ondragleave={handleDragLeave}
              ondragover={handleDragOver}
              ondrop={handleDrop}
              role="button"
              tabindex="0"
            >
              <div class="text-6xl mb-4">📄</div>
              <h3 class="text-lg font-bold text-white mb-2">Drag & Drop Boarding Passes</h3>
              <p class="text-sm text-slate-400 mb-4">
                Or click to browse files (PNG, JPG, PDF, HEIC)
              </p>
              <button
                onclick={selectFiles}
                class="bg-blue-600 hover:bg-blue-500 text-white px-6 py-2 rounded-lg font-medium transition-colors"
                aria-label="Select files to upload"
              >
                Select Files
              </button>
            </div>

            <!-- File List -->
            {#if files.length > 0}
              <div class="space-y-2">
                <div class="flex items-center justify-between mb-2">
                  <h4 class="text-sm font-bold text-white">Selected Files ({files.length})</h4>
                  <button
                    onclick={() => files = []}
                    class="text-xs text-red-400 hover:text-red-300"
                    aria-label="Clear all files"
                  >
                    Clear All
                  </button>
                </div>
                {#each files as file, i}
                  <div class="flex items-center gap-3 bg-slate-950/50 border border-slate-800 rounded-lg p-3">
                    <div class={`text-xl ${getStatusColor(file.status)}`}>
                      {getStatusIcon(file.status)}
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="text-sm font-medium text-white truncate">{file.name}</div>
                      <div class="text-xs text-slate-400 truncate">{file.path}</div>
                    </div>
                    <button
                      onclick={() => removeFile(i)}
                      class="text-slate-500 hover:text-red-400 transition-colors"
                      aria-label="Remove file"
                    >
                      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    </button>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

        {:else if currentStep === 'processing'}
          <!-- Processing Step -->
          <div class="text-center py-12">
            <div class="text-6xl mb-4 animate-pulse">⚡</div>
            <h3 class="text-xl font-bold text-white mb-2">Processing {files.length} Files...</h3>
            <p class="text-sm text-slate-400 mb-6">
              Completed: {processedCount} / {files.length} ({successCount} success, {errorCount} errors)
            </p>

            <!-- Progress Bar -->
            <div class="max-w-md mx-auto mb-8">
              <div class="h-2 bg-slate-800 rounded-full overflow-hidden">
                <div
                  class="h-full bg-blue-600 transition-all duration-300"
                  style:width="{(processedCount / files.length) * 100}%"
                ></div>
              </div>
            </div>

            <!-- File List with Status -->
            <div class="max-w-2xl mx-auto space-y-2 text-left">
              {#each files as file}
                <div class="flex items-center gap-3 bg-slate-950/50 border border-slate-800 rounded p-2">
                  <div class={`text-lg ${getStatusColor(file.status)}`}>
                    {getStatusIcon(file.status)}
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="text-xs font-medium text-white truncate">{file.name}</div>
                    {#if file.error}
                      <div class="text-xs text-red-400">{file.error}</div>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          </div>

        {:else}
          <!-- Review Step -->
          <div class="space-y-4">
            <div class="bg-blue-950/30 border border-blue-800/50 rounded-lg p-4 mb-4">
              <div class="flex items-start gap-3">
                <svg class="w-5 h-5 text-blue-400 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7-4a1 1 0 11-2 0 1 1 0 012 0zM9 9a1 1 0 000 2v3a1 1 0 001 1h1a1 1 0 100-2v-3a1 1 0 00-1-1H9z" clip-rule="evenodd"/>
                </svg>
                <div>
                  <h4 class="text-sm font-bold text-blue-200 mb-1">Review & Edit Results</h4>
                  <p class="text-xs text-blue-300">
                    Successfully processed {successCount} of {files.length} files. Review and edit before importing.
                  </p>
                </div>
              </div>
            </div>

            {#each files.filter(f => f.status === 'success') as file, i}
              <div class="bg-slate-950/50 border border-slate-800 rounded-lg p-4">
                <div class="flex items-center gap-2 mb-3">
                  <span class="text-green-400 text-lg">✓</span>
                  <h5 class="text-sm font-bold text-white">{file.name}</h5>
                </div>

                {#if file.result}
                  <div class="grid grid-cols-2 gap-3">
                    {#each Object.entries(file.result) as [key, value]}
                      {#if value !== undefined && key !== 'confidence_score'}
                        <div>
                          <label class="block text-xs text-slate-400 mb-1 capitalize">
                            {key.replace(/_/g, ' ')}
                          </label>
                          <input
                            type="text"
                            value={value || ''}
                            oninput={(e) => editResult(files.indexOf(file), key as keyof OcrResult, e.currentTarget.value)}
                            class="w-full bg-slate-900 border border-slate-700 rounded px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                          />
                        </div>
                      {/if}
                    {/each}
                  </div>

                  {#if file.result.confidence_score}
                    <div class="mt-3 text-xs text-slate-500">
                      Confidence: {Math.round(file.result.confidence_score * 100)}%
                    </div>
                  {/if}
                {/if}
              </div>
            {/each}

            {#if errorCount > 0}
              <div class="bg-red-950/30 border border-red-800/50 rounded-lg p-4">
                <h5 class="text-sm font-bold text-red-200 mb-2">Failed Files ({errorCount})</h5>
                {#each files.filter(f => f.status === 'error') as file}
                  <div class="text-xs text-red-300 mb-1">
                    • {file.name}: {file.error}
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- Footer Actions -->
      <div class="flex items-center justify-between px-6 py-4 border-t border-slate-800 bg-slate-950/50">
        <button
          onclick={resetAndClose}
          class="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white transition-colors"
          disabled={isProcessing}
        >
          Cancel
        </button>

        <div class="flex items-center gap-3">
          {#if currentStep === 'upload'}
            <button
              onclick={processBatch}
              disabled={files.length === 0}
              class="px-6 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-slate-700 disabled:text-slate-500 text-white font-medium rounded-lg transition-colors"
            >
              Process {files.length} File{files.length !== 1 ? 's' : ''}
            </button>
          {:else if currentStep === 'review'}
            <button
              onclick={completeAndClose}
              disabled={!canProceed}
              class="px-6 py-2 bg-green-600 hover:bg-green-500 disabled:bg-slate-700 disabled:text-slate-500 text-white font-medium rounded-lg transition-colors"
            >
              Import {successCount} Flight{successCount !== 1 ? 's' : ''}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}
