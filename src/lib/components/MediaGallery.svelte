<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { convertFileSrc } from '@tauri-apps/api/core';

  interface Props {
    userId: string;
  }

  let { userId }: Props = $props();

  // Types
  interface MediaFile {
    id: string;
    user_id: string;
    filename: string;
    original_filename: string;
    file_type: string;
    mime_type: string;
    file_size: number;
    file_path: string;
    thumbnail_path: string | null;
    title: string | null;
    description: string | null;
    tags: string | null;
    flight_id: string | null;
    journey_id: string | null;
    captured_date: string | null;
    location: string | null;
    is_favorite: boolean;
    created_at: string;
    updated_at: string;
  }

  interface MediaStats {
    total_files: number;
    total_photos: number;
    total_documents: number;
    total_receipts: number;
    total_boarding_passes: number;
    total_size_bytes: number;
    favorites_count: number;
  }

  // State
  type GalleryTab = 'all' | 'photos' | 'documents' | 'receipts' | 'boarding_passes' | 'favorites';
  let activeTab = $state<GalleryTab>('all');
  let files = $state<MediaFile[]>([]);
  let stats = $state<MediaStats | null>(null);
  let loading = $state(false);
  let uploading = $state(false);
  let selectedFile = $state<MediaFile | null>(null);
  let showViewer = $state(false);
  let showUploadModal = $state(false);
  let showEditModal = $state(false);
  let searchQuery = $state('');

  // Upload form state
  let uploadFileType = $state<string>('photo');
  let uploadTitle = $state('');
  let uploadDescription = $state('');
  let uploadLocation = $state('');
  let uploadTags = $state('');

  // Edit form state
  let editTitle = $state('');
  let editDescription = $state('');
  let editFileType = $state('');
  let editLocation = $state('');
  let editTags = $state('');

  // File type mapping
  const fileTypeLabels: Record<string, string> = {
    photo: 'Photos',
    document: 'Documents',
    receipt: 'Receipts',
    boarding_pass: 'Boarding Passes',
    other: 'Other',
  };

  const fileTypeIcons: Record<string, string> = {
    photo: '📷',
    document: '📄',
    receipt: '🧾',
    boarding_pass: '🎫',
    other: '📁',
  };

  // Load files based on active tab
  async function loadFiles() {
    loading = true;
    try {
      const filter: any = {
        limit: 200,
        offset: 0,
      };

      if (activeTab === 'photos') filter.file_type = 'photo';
      else if (activeTab === 'documents') filter.file_type = 'document';
      else if (activeTab === 'receipts') filter.file_type = 'receipt';
      else if (activeTab === 'boarding_passes') filter.file_type = 'boarding_pass';
      else if (activeTab === 'favorites') filter.favorites_only = true;

      if (searchQuery) filter.search = searchQuery;

      files = await invoke<MediaFile[]>('list_media_files', { userId, filter });
    } catch (err) {
      console.error('Failed to load media files:', err);
    } finally {
      loading = false;
    }
  }

  // Load stats
  async function loadStats() {
    try {
      stats = await invoke<MediaStats>('get_media_stats', { userId });
    } catch (err) {
      console.error('Failed to load media stats:', err);
    }
  }

  // Handle tab change
  function handleTabChange(tab: GalleryTab) {
    activeTab = tab;
    loadFiles();
  }

  // Open file picker and upload
  async function handleUpload() {
    try {
      const selected = await open({
        multiple: true,
        filters: [
          { name: 'Images', extensions: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'heic'] },
          { name: 'Documents', extensions: ['pdf', 'doc', 'docx', 'txt'] },
          { name: 'All Files', extensions: ['*'] },
        ],
      });

      if (!selected) return;

      const paths = Array.isArray(selected) ? selected : [selected];
      if (paths.length === 0) return;

      // If single file, show modal for metadata
      if (paths.length === 1) {
        showUploadModal = true;
        // Store path for later
        (window as any).__pendingUploadPath = paths[0];
        return;
      }

      // Multiple files - upload with default metadata
      uploading = true;
      for (const path of paths) {
        await invoke('upload_media_file', {
          userId,
          sourcePath: path,
          input: {
            file_type: guessFileType(path),
            title: null,
            description: null,
            tags: null,
            flight_id: null,
            journey_id: null,
            captured_date: null,
            location: null,
          },
        });
      }

      await loadFiles();
      await loadStats();
    } catch (err) {
      console.error('Upload failed:', err);
      alert('Upload failed: ' + err);
    } finally {
      uploading = false;
    }
  }

  // Complete single file upload with metadata
  async function completeUpload() {
    const path = (window as any).__pendingUploadPath;
    if (!path) return;

    // Parse comma-separated tags into array
    const tagsArray = uploadTags
      .split(',')
      .map(t => t.trim())
      .filter(t => t.length > 0);

    uploading = true;
    try {
      await invoke('upload_media_file', {
        userId,
        sourcePath: path,
        input: {
          file_type: uploadFileType,
          title: uploadTitle || null,
          description: uploadDescription || null,
          tags: tagsArray.length > 0 ? tagsArray : null,
          flight_id: null,
          journey_id: null,
          captured_date: null,
          location: uploadLocation || null,
        },
      });

      showUploadModal = false;
      uploadTitle = '';
      uploadDescription = '';
      uploadLocation = '';
      uploadTags = '';
      uploadFileType = 'photo';
      (window as any).__pendingUploadPath = null;

      await loadFiles();
      await loadStats();
    } catch (err) {
      console.error('Upload failed:', err);
      alert('Upload failed: ' + err);
    } finally {
      uploading = false;
    }
  }

  // Guess file type from extension
  function guessFileType(path: string): string {
    const ext = path.split('.').pop()?.toLowerCase() || '';
    if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'heic'].includes(ext)) return 'photo';
    if (['pdf', 'doc', 'docx', 'txt'].includes(ext)) return 'document';
    return 'other';
  }

  // View file
  async function viewFile(file: MediaFile) {
    selectedFile = file;
    showViewer = true;
  }

  // Get file URL for display
  async function getFileUrl(file: MediaFile): Promise<string> {
    const path = await invoke<string>('get_media_file_path', { filename: file.filename });
    // convertFileSrc creates an asset:// URL for Tauri to serve the file
    const url = convertFileSrc(path);
    console.log(`[MediaGallery] File: ${file.filename}, Path: ${path}, URL: ${url}`);
    return url;
  }

  // Toggle favorite
  async function toggleFavorite(file: MediaFile, e: Event) {
    e.stopPropagation();
    try {
      const newState = await invoke<boolean>('toggle_media_favorite', { fileId: file.id });
      file.is_favorite = newState;
      files = [...files]; // Trigger reactivity
    } catch (err) {
      console.error('Failed to toggle favorite:', err);
    }
  }

  // Open edit modal
  function openEditModal(file: MediaFile, e: Event) {
    e.stopPropagation();
    selectedFile = file;
    editTitle = file.title || '';
    editDescription = file.description || '';
    editFileType = file.file_type;
    editLocation = file.location || '';
    // Parse tags from JSON string or display as comma-separated
    if (file.tags) {
      try {
        const tagsArray = JSON.parse(file.tags);
        editTags = Array.isArray(tagsArray) ? tagsArray.join(', ') : file.tags;
      } catch {
        editTags = file.tags;
      }
    } else {
      editTags = '';
    }
    showEditModal = true;
  }

  // Save edits
  async function saveEdit() {
    if (!selectedFile) return;

    // Parse comma-separated tags into array
    const tagsArray = editTags
      .split(',')
      .map(t => t.trim())
      .filter(t => t.length > 0);

    try {
      await invoke('update_media_file', {
        fileId: selectedFile.id,
        title: editTitle || null,
        description: editDescription || null,
        tags: tagsArray.length > 0 ? tagsArray : null,
        fileType: editFileType,
        flightId: null,
        journeyId: null,
        capturedDate: null,
        location: editLocation || null,
      });

      showEditModal = false;
      await loadFiles();
    } catch (err) {
      console.error('Failed to update:', err);
      alert('Failed to update: ' + err);
    }
  }

  // Delete file
  async function deleteFile(file: MediaFile, e: Event) {
    e.stopPropagation();
    if (!confirm(`Delete "${file.original_filename}"? This cannot be undone.`)) return;

    try {
      await invoke('delete_media_file', { fileId: file.id });
      files = files.filter(f => f.id !== file.id);
      await loadStats();
      if (showViewer && selectedFile?.id === file.id) {
        showViewer = false;
        selectedFile = null;
      }
    } catch (err) {
      console.error('Failed to delete:', err);
      alert('Failed to delete: ' + err);
    }
  }

  // Format file size
  function formatSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
  }

  // Format date
  function formatDate(dateStr: string): string {
    try {
      return new Date(dateStr).toLocaleDateString();
    } catch {
      return dateStr;
    }
  }

  // Check if file is an image
  function isImage(file: MediaFile): boolean {
    return file.mime_type.startsWith('image/');
  }

  // Parse tags from JSON string
  function parseTags(tagsStr: string | null): string[] {
    if (!tagsStr) return [];
    try {
      const parsed = JSON.parse(tagsStr);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      // If not valid JSON, try comma-separated
      return tagsStr.split(',').map(t => t.trim()).filter(t => t.length > 0);
    }
  }

  // Search with debounce
  let searchTimeout: ReturnType<typeof setTimeout>;
  function handleSearch(value: string) {
    searchQuery = value;
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => loadFiles(), 300);
  }

  // File URL cache
  let fileUrls = $state<Record<string, string>>({});
  let fileUrlErrors = $state<Record<string, boolean>>({});

  async function ensureFileUrl(file: MediaFile) {
    if (!fileUrls[file.id] && !fileUrlErrors[file.id]) {
      try {
        const url = await getFileUrl(file);
        console.log(`Media URL for ${file.filename}:`, url);
        fileUrls[file.id] = url;
        fileUrls = { ...fileUrls };
      } catch (err) {
        console.error(`Failed to get URL for ${file.filename}:`, err);
        fileUrlErrors[file.id] = true;
        fileUrlErrors = { ...fileUrlErrors };
      }
    }
  }

  // Load URLs for visible files
  $effect(() => {
    files.forEach(f => {
      if (isImage(f)) ensureFileUrl(f);
    });
  });

  onMount(() => {
    loadFiles();
    loadStats();
  });
</script>

<div class="h-full flex flex-col overflow-hidden bg-gray-50 dark:bg-gray-900">
  <!-- Header -->
  <div class="px-6 py-4 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex-shrink-0">
    <div class="flex items-center justify-between mb-4">
      <div>
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">Media Gallery</h2>
        <p class="text-gray-600 dark:text-gray-400 text-sm">
          Store and organize your travel photos, boarding passes, receipts, and documents.
        </p>
      </div>
      <button
        onclick={handleUpload}
        disabled={uploading}
        class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:opacity-50 flex items-center gap-2"
      >
        {#if uploading}
          <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
        {:else}
          <span>+</span>
        {/if}
        Upload Files
      </button>
    </div>

    <!-- Stats -->
    {#if stats}
      <div class="flex gap-4 text-sm">
        <span class="text-gray-500 dark:text-gray-400">
          {stats.total_files} files
        </span>
        <span class="text-gray-300 dark:text-gray-600">|</span>
        <span class="text-gray-500 dark:text-gray-400">
          {formatSize(stats.total_size_bytes)} total
        </span>
        {#if stats.favorites_count > 0}
          <span class="text-gray-300 dark:text-gray-600">|</span>
          <span class="text-amber-600 dark:text-amber-400">
            {stats.favorites_count} favorites
          </span>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Tab Navigation & Search -->
  <div class="px-6 py-3 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 flex items-center gap-4 flex-shrink-0">
    <nav class="flex gap-2 flex-1">
      <button
        onclick={() => handleTabChange('all')}
        class="px-3 py-1.5 rounded-lg text-sm font-medium transition {activeTab === 'all'
          ? 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300'
          : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        All
      </button>
      <button
        onclick={() => handleTabChange('photos')}
        class="px-3 py-1.5 rounded-lg text-sm font-medium transition {activeTab === 'photos'
          ? 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300'
          : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        📷 Photos
        {#if stats && stats.total_photos > 0}
          <span class="ml-1 text-xs opacity-70">({stats.total_photos})</span>
        {/if}
      </button>
      <button
        onclick={() => handleTabChange('documents')}
        class="px-3 py-1.5 rounded-lg text-sm font-medium transition {activeTab === 'documents'
          ? 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300'
          : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        📄 Documents
        {#if stats && stats.total_documents > 0}
          <span class="ml-1 text-xs opacity-70">({stats.total_documents})</span>
        {/if}
      </button>
      <button
        onclick={() => handleTabChange('receipts')}
        class="px-3 py-1.5 rounded-lg text-sm font-medium transition {activeTab === 'receipts'
          ? 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300'
          : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        🧾 Receipts
        {#if stats && stats.total_receipts > 0}
          <span class="ml-1 text-xs opacity-70">({stats.total_receipts})</span>
        {/if}
      </button>
      <button
        onclick={() => handleTabChange('boarding_passes')}
        class="px-3 py-1.5 rounded-lg text-sm font-medium transition {activeTab === 'boarding_passes'
          ? 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900 dark:text-indigo-300'
          : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        🎫 Boarding Passes
        {#if stats && stats.total_boarding_passes > 0}
          <span class="ml-1 text-xs opacity-70">({stats.total_boarding_passes})</span>
        {/if}
      </button>
      <button
        onclick={() => handleTabChange('favorites')}
        class="px-3 py-1.5 rounded-lg text-sm font-medium transition {activeTab === 'favorites'
          ? 'bg-amber-100 text-amber-700 dark:bg-amber-900 dark:text-amber-300'
          : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        ⭐ Favorites
      </button>
    </nav>

    <input
      type="text"
      placeholder="Search files..."
      value={searchQuery}
      oninput={(e) => handleSearch(e.currentTarget.value)}
      class="px-3 py-1.5 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white w-48 text-sm"
    />
  </div>

  <!-- Gallery Grid -->
  <div class="flex-1 overflow-auto p-6">
    {#if loading}
      <div class="flex items-center justify-center h-full">
        <div class="animate-spin rounded-full h-10 w-10 border-b-2 border-indigo-600"></div>
      </div>
    {:else if files.length === 0}
      <div class="flex flex-col items-center justify-center h-full text-gray-500">
        <div class="text-6xl mb-4">📁</div>
        <p class="text-lg font-medium">No files yet</p>
        <p class="text-sm mt-1">Upload photos, documents, and receipts to keep them organized</p>
        <button
          onclick={handleUpload}
          class="mt-4 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700"
        >
          Upload Your First File
        </button>
      </div>
    {:else}
      <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
        {#each files as file}
          <div
            class="group relative bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden cursor-pointer hover:shadow-lg transition"
            onclick={() => viewFile(file)}
          >
            <!-- Thumbnail / Icon -->
            <div class="aspect-square bg-gray-100 dark:bg-gray-700 flex items-center justify-center overflow-hidden relative">
              {#if isImage(file) && fileUrls[file.id] && !fileUrlErrors[file.id]}
                <img
                  src={fileUrls[file.id]}
                  alt={file.title || file.original_filename}
                  class="w-full h-full object-cover"
                  onerror={(e) => {
                    console.error('[MediaGallery] Image failed to load:', file.filename, fileUrls[file.id]);
                    fileUrlErrors[file.id] = true;
                    fileUrlErrors = { ...fileUrlErrors };
                  }}
                />
              {:else if isImage(file) && !fileUrls[file.id] && !fileUrlErrors[file.id]}
                <div class="animate-pulse w-full h-full bg-gray-200 dark:bg-gray-600"></div>
              {:else if isImage(file) && fileUrlErrors[file.id]}
                <div class="flex flex-col items-center justify-center text-gray-400 dark:text-gray-500 p-2">
                  <span class="text-3xl">📷</span>
                  <span class="text-xs mt-1">Preview unavailable</span>
                </div>
              {:else}
                <span class="text-5xl">{fileTypeIcons[file.file_type] || '📁'}</span>
              {/if}
            </div>

            <!-- Favorite button -->
            <button
              onclick={(e) => toggleFavorite(file, e)}
              class="absolute top-2 right-2 p-1.5 rounded-full bg-black/30 hover:bg-black/50 transition opacity-0 group-hover:opacity-100"
            >
              <span class="text-lg">{file.is_favorite ? '⭐' : '☆'}</span>
            </button>

            <!-- Info -->
            <div class="p-3">
              <p class="text-sm font-medium text-gray-900 dark:text-white truncate">
                {file.title || file.original_filename}
              </p>
              <div class="flex items-center justify-between mt-1">
                <span class="text-xs text-gray-500 dark:text-gray-400">
                  {formatSize(file.file_size)}
                </span>
                <span class="text-xs text-gray-400">
                  {formatDate(file.created_at)}
                </span>
              </div>
            </div>

            <!-- Hover actions -->
            <div class="absolute bottom-0 left-0 right-0 p-2 bg-gradient-to-t from-black/60 to-transparent opacity-0 group-hover:opacity-100 transition flex justify-end gap-2">
              <button
                onclick={(e) => openEditModal(file, e)}
                class="px-2 py-1 text-xs bg-white/20 hover:bg-white/30 text-white rounded"
              >
                Edit
              </button>
              <button
                onclick={(e) => deleteFile(file, e)}
                class="px-2 py-1 text-xs bg-red-500/60 hover:bg-red-500/80 text-white rounded"
              >
                Delete
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- File Viewer Modal -->
{#if showViewer && selectedFile}
  <div
    class="fixed inset-0 bg-black/90 flex items-center justify-center z-50"
    onclick={() => showViewer = false}
  >
    <button
      onclick={() => showViewer = false}
      class="absolute top-4 right-4 text-white text-3xl hover:text-gray-300"
    >
      ✕
    </button>

    <div class="max-w-4xl max-h-[90vh] overflow-auto" onclick={(e) => e.stopPropagation()}>
      {#if isImage(selectedFile) && fileUrls[selectedFile.id] && !fileUrlErrors[selectedFile.id]}
        <img
          src={fileUrls[selectedFile.id]}
          alt={selectedFile.title || selectedFile.original_filename}
          class="max-w-full max-h-[80vh] object-contain mx-auto rounded-lg"
          onerror={() => {
            if (!selectedFile) return;
            console.error('[MediaGallery] Viewer image failed to load:', selectedFile.filename, fileUrls[selectedFile.id]);
            fileUrlErrors[selectedFile.id] = true;
            fileUrlErrors = { ...fileUrlErrors };
          }}
        />
      {:else if isImage(selectedFile) && !fileUrls[selectedFile.id] && !fileUrlErrors[selectedFile.id]}
        <div class="bg-white dark:bg-gray-800 rounded-lg p-12 text-center">
          <div class="animate-spin rounded-full h-16 w-16 border-b-2 border-indigo-600 mx-auto"></div>
          <p class="mt-4 text-gray-500">Loading image...</p>
        </div>
      {:else if isImage(selectedFile) && fileUrlErrors[selectedFile.id]}
        <div class="bg-white dark:bg-gray-800 rounded-lg p-12 text-center">
          <span class="text-6xl">📷</span>
          <p class="mt-4 text-xl font-medium text-gray-900 dark:text-white">
            Preview Unavailable
          </p>
          <p class="mt-2 text-gray-500">{selectedFile.original_filename}</p>
          <p class="mt-1 text-sm text-gray-400">The file may have been moved or deleted.</p>
        </div>
      {:else}
        <div class="bg-white dark:bg-gray-800 rounded-lg p-12 text-center">
          <span class="text-8xl">{fileTypeIcons[selectedFile.file_type] || '📁'}</span>
          <p class="mt-4 text-xl font-medium text-gray-900 dark:text-white">
            {selectedFile.original_filename}
          </p>
          <p class="mt-2 text-gray-500">{formatSize(selectedFile.file_size)}</p>
        </div>
      {/if}

      <div class="mt-4 text-center text-white">
        <p class="text-lg font-medium">{selectedFile.title || selectedFile.original_filename}</p>
        {#if selectedFile.description}
          <p class="text-sm text-gray-300 mt-1">{selectedFile.description}</p>
        {/if}
        {#if selectedFile.location}
          <p class="text-sm text-gray-400 mt-1">📍 {selectedFile.location}</p>
        {/if}
        {#if selectedFile.tags && parseTags(selectedFile.tags).length > 0}
          <div class="flex flex-wrap gap-1 justify-center mt-2">
            {#each parseTags(selectedFile.tags) as tag}
              <span class="px-2 py-0.5 bg-indigo-500/30 text-indigo-200 text-xs rounded-full">
                {tag}
              </span>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<!-- Upload Modal -->
{#if showUploadModal}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-md">
      <div class="p-4 border-b dark:border-gray-700 flex items-center justify-between">
        <h3 class="font-semibold text-gray-900 dark:text-white">Upload File</h3>
        <button onclick={() => showUploadModal = false} class="text-gray-500 hover:text-gray-700">✕</button>
      </div>
      <div class="p-4 space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">File Type</label>
          <select bind:value={uploadFileType} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white">
            <option value="photo">📷 Photo</option>
            <option value="document">📄 Document</option>
            <option value="receipt">🧾 Receipt</option>
            <option value="boarding_pass">🎫 Boarding Pass</option>
            <option value="other">📁 Other</option>
          </select>
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Title (optional)</label>
          <input type="text" bind:value={uploadTitle} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" placeholder="Give it a name..." />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description (optional)</label>
          <textarea bind:value={uploadDescription} rows="2" class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" placeholder="Add notes..."></textarea>
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Location (optional)</label>
          <input type="text" bind:value={uploadLocation} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" placeholder="Where was this taken?" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Tags / Categories (optional)</label>
          <input
            type="text"
            bind:value={uploadTags}
            class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            placeholder="e.g. vacation, tokyo, 2024 (comma-separated)"
          />
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">Separate multiple tags with commas</p>
        </div>
      </div>
      <div class="p-4 border-t dark:border-gray-700 flex justify-end gap-2">
        <button onclick={() => showUploadModal = false} class="px-4 py-2 text-gray-600 hover:text-gray-800 dark:text-gray-400">Cancel</button>
        <button onclick={completeUpload} disabled={uploading} class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:opacity-50">
          {uploading ? 'Uploading...' : 'Upload'}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Edit Modal -->
{#if showEditModal && selectedFile}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-md">
      <div class="p-4 border-b dark:border-gray-700 flex items-center justify-between">
        <h3 class="font-semibold text-gray-900 dark:text-white">Edit File</h3>
        <button onclick={() => showEditModal = false} class="text-gray-500 hover:text-gray-700">✕</button>
      </div>
      <div class="p-4 space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">File Type</label>
          <select bind:value={editFileType} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white">
            <option value="photo">📷 Photo</option>
            <option value="document">📄 Document</option>
            <option value="receipt">🧾 Receipt</option>
            <option value="boarding_pass">🎫 Boarding Pass</option>
            <option value="other">📁 Other</option>
          </select>
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Title</label>
          <input type="text" bind:value={editTitle} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description</label>
          <textarea bind:value={editDescription} rows="2" class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"></textarea>
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Location</label>
          <input type="text" bind:value={editLocation} class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white" placeholder="Where was this taken?" />
        </div>
        <div>
          <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Tags / Categories</label>
          <input
            type="text"
            bind:value={editTags}
            class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            placeholder="e.g. vacation, tokyo, 2024 (comma-separated)"
          />
          <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">Separate multiple tags with commas</p>
        </div>
      </div>
      <div class="p-4 border-t dark:border-gray-700 flex justify-end gap-2">
        <button onclick={() => showEditModal = false} class="px-4 py-2 text-gray-600 hover:text-gray-800 dark:text-gray-400">Cancel</button>
        <button onclick={saveEdit} class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700">Save</button>
      </div>
    </div>
  </div>
{/if}
