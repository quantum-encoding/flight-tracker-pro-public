<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface User {
    id: string;
    name: string;
    email: string | null;
    pilot_license_number: string | null;
    license_type: string | null;
    license_country: string | null;
    created_at: string;
    updated_at: string;
  }

  interface Props {
    currentUser: User;
    onSwitch: (user: User) => void;
    onCreateNew: () => void;
  }

  let { currentUser, onSwitch, onCreateNew }: Props = $props();

  let showDropdown = $state(false);
  let allUsers: User[] = $state([]);
  let loading = $state(false);
  let editingId: string | null = $state(null);
  let editName = $state('');

  async function loadAllUsers() {
    loading = true;
    try {
      allUsers = await invoke('list_all_users');
    } catch (err) {
      console.error('Failed to load users:', err);
    } finally {
      loading = false;
    }
  }

  async function startEdit(user: User) {
    editingId = user.id;
    editName = user.name;
  }

  async function saveEdit(userId: string) {
    if (!editName.trim()) {
      alert('Dataset name cannot be empty');
      return;
    }

    try {
      await invoke('update_user_name', { userId, newName: editName });
      await loadAllUsers();
      if (userId === currentUser.id) {
        currentUser.name = editName;
      }
      editingId = null;
    } catch (err) {
      console.error('Failed to update name:', err);
      alert(`Failed to update name: ${err}`);
    }
  }

  function cancelEdit() {
    editingId = null;
    editName = '';
  }

  async function toggleDropdown() {
    showDropdown = !showDropdown;
    if (showDropdown && allUsers.length === 0) {
      await loadAllUsers();
    }
  }

  function handleBackdropClick() {
    showDropdown = false;
  }
</script>

<div class="relative">
  <!-- Selector Button -->
  <button
    onclick={toggleDropdown}
    class="flex items-center gap-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg px-4 py-2 hover:bg-gray-50 dark:hover:bg-gray-700 transition"
  >
    <span class="text-sm font-medium text-gray-700 dark:text-gray-300">Dataset:</span>
    <span class="text-sm font-semibold text-gray-900 dark:text-white">{currentUser.name}</span>
    <svg class="w-4 h-4 text-gray-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
    </svg>
  </button>

  <!-- Dropdown -->
  {#if showDropdown}
    <!-- Backdrop -->
    <div
      class="fixed inset-0 z-40"
      onclick={handleBackdropClick}
      role="button"
      tabindex="0"
    ></div>

    <!-- Dropdown Menu -->
    <div class="absolute top-full left-0 mt-2 w-80 bg-white dark:bg-gray-800 rounded-lg shadow-xl border border-gray-200 dark:border-gray-700 z-50">
      <div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700">
        <h3 class="text-sm font-semibold text-gray-900 dark:text-white">Your Datasets</h3>
        <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
          Switch between or create new flight datasets
        </p>
      </div>

      {#if loading}
        <div class="px-4 py-6 text-center">
          <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-primary-600 mx-auto"></div>
        </div>
      {:else}
        <!-- User List -->
        <div class="max-h-64 overflow-y-auto">
          {#each allUsers as user}
            <div
              class="px-4 py-3 hover:bg-gray-50 dark:hover:bg-gray-700 transition {user.id === currentUser.id ? 'bg-primary-50 dark:bg-primary-900/20' : ''}"
            >
              {#if editingId === user.id}
                <!-- Edit Mode -->
                <div class="flex items-center gap-2">
                  <input
                    type="text"
                    bind:value={editName}
                    class="flex-1 px-2 py-1 border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-900 text-sm text-gray-900 dark:text-white"
                    onkeydown={(e) => {
                      if (e.key === 'Enter') saveEdit(user.id);
                      if (e.key === 'Escape') cancelEdit();
                    }}
                  />
                  <button
                    onclick={() => saveEdit(user.id)}
                    class="text-green-600 hover:text-green-700 text-xs"
                    title="Save"
                  >
                    ✓
                  </button>
                  <button
                    onclick={cancelEdit}
                    class="text-red-600 hover:text-red-700 text-xs"
                    title="Cancel"
                  >
                    ✗
                  </button>
                </div>
              {:else}
                <!-- Display Mode -->
                <div class="flex items-center justify-between">
                  <button
                    onclick={() => {
                      onSwitch(user);
                      showDropdown = false;
                    }}
                    class="flex-1 text-left"
                  >
                    <div class="flex items-center gap-2">
                      {#if user.id === currentUser.id}
                        <span class="text-primary-600 dark:text-primary-400">●</span>
                      {/if}
                      <div>
                        <p class="text-sm font-medium text-gray-900 dark:text-white">
                          {user.name}
                        </p>
                        <p class="text-xs text-gray-500 dark:text-gray-400">
                          Created {new Date(user.created_at).toLocaleDateString()}
                        </p>
                      </div>
                    </div>
                  </button>
                  <button
                    onclick={() => startEdit(user)}
                    class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 text-xs ml-2"
                    title="Rename dataset"
                  >
                    ✏️
                  </button>
                </div>
              {/if}
            </div>
          {/each}
        </div>

        <!-- Create New Button -->
        <div class="px-4 py-3 border-t border-gray-200 dark:border-gray-700">
          <button
            onclick={() => {
              onCreateNew();
              showDropdown = false;
            }}
            class="w-full bg-primary-600 hover:bg-primary-700 text-white px-4 py-2 rounded-lg transition text-sm font-medium"
          >
            + Create New Dataset
          </button>
        </div>
      {/if}
    </div>
  {/if}
</div>
