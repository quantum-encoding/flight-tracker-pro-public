<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { translations } from '$lib/i18n';

  interface AircraftType {
    id: string;
    manufacturer: string;
    model: string;
    type_designator: string | null;
    category: string | null;
    class: string | null;
    notes: string | null;
  }

  let aircraftTypes = $state<AircraftType[]>([]);
  let loading = $state(true);
  let showModal = $state(false);
  let editingType = $state<AircraftType | null>(null);

  // Form fields
  let formManufacturer = $state('');
  let formModel = $state('');
  let formTypeDesignator = $state('');
  let formCategory = $state('');
  let formClass = $state('');
  let formNotes = $state('');

  // Search and filter
  let searchQuery = $state('');
  let filterCategory = $state<string>('all');

  let categories = $derived(() => {
    const cats = new Set<string>();
    aircraftTypes.forEach(t => {
      if (t.category) cats.add(t.category);
    });
    return Array.from(cats).sort();
  });

  let filteredTypes = $derived(() => {
    let filtered = aircraftTypes;

    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(t =>
        t.manufacturer.toLowerCase().includes(query) ||
        t.model.toLowerCase().includes(query) ||
        t.type_designator?.toLowerCase().includes(query)
      );
    }

    if (filterCategory !== 'all') {
      filtered = filtered.filter(t => t.category === filterCategory);
    }

    return filtered;
  });

  onMount(async () => {
    await loadAircraftTypes();
  });

  async function loadAircraftTypes() {
    loading = true;
    try {
      aircraftTypes = await invoke('list_all_aircraft_types');
    } catch (error) {
      console.error('Failed to load aircraft types:', error);
    } finally {
      loading = false;
    }
  }

  function openCreateModal() {
    editingType = null;
    resetForm();
    showModal = true;
  }

  function openEditModal(type: AircraftType) {
    editingType = type;
    formManufacturer = type.manufacturer;
    formModel = type.model;
    formTypeDesignator = type.type_designator || '';
    formCategory = type.category || '';
    formClass = type.class || '';
    formNotes = type.notes || '';
    showModal = true;
  }

  function resetForm() {
    formManufacturer = '';
    formModel = '';
    formTypeDesignator = '';
    formCategory = '';
    formClass = '';
    formNotes = '';
  }

  async function saveAircraftType() {
    try {
      const aircraftType = {
        manufacturer: formManufacturer,
        model: formModel,
        type_designator: formTypeDesignator || null,
        category: formCategory || null,
        class: formClass || null,
        notes: formNotes || null,
      };

      if (editingType) {
        await invoke('update_aircraft_type', {
          typeId: editingType.id,
          aircraftType
        });
      } else {
        await invoke('create_aircraft_type', { aircraftType });
      }

      showModal = false;
      await loadAircraftTypes();
    } catch (error) {
      console.error('Failed to save aircraft type:', error);
      alert(`Failed to save aircraft type: ${error}`);
    }
  }

  async function deleteAircraftType(id: string) {
    if (!confirm('Are you sure you want to delete this aircraft type?')) return;

    try {
      await invoke('delete_aircraft_type', { typeId: id });
      await loadAircraftTypes();
    } catch (error) {
      console.error('Failed to delete aircraft type:', error);
      alert(`Failed to delete aircraft type: ${error}`);
    }
  }
</script>

<div class="aircraft-types-container">
  <div class="header">
    <h2>{$translations('aircraft.title')}</h2>
    <button onclick={openCreateModal} class="btn-primary">
      ➕ {$translations('aircraft.addAircraft')}
    </button>
  </div>

  <div class="filters">
    <input
      type="text"
      bind:value={searchQuery}
      placeholder="{$translations('common.search')}..."
      class="search-input"
    />

    <select bind:value={filterCategory} class="category-filter">
      <option value="all">{$translations('flights.filters.all')}</option>
      {#each categories() as category}
        <option value={category}>{category}</option>
      {/each}
    </select>
  </div>

  {#if loading}
    <div class="loading">{$translations('common.loading')}...</div>
  {:else if aircraftTypes.length === 0}
    <div class="empty-state">
      <p>{$translations('aircraft.title')}</p>
      <button onclick={openCreateModal} class="btn-primary">
        ➕ {$translations('aircraft.addAircraft')}
      </button>
    </div>
  {:else}
    <div class="types-table">
      <table>
        <thead>
          <tr>
            <th>{$translations('aircraft.manufacturer')}</th>
            <th>{$translations('aircraft.model')}</th>
            <th>{$translations('aircraft.registration')}</th>
            <th>{$translations('aircraft.category')}</th>
            <th>{$translations('aircraft.class')}</th>
            <th>{$translations('common.actions')}</th>
          </tr>
        </thead>
        <tbody>
          {#each filteredTypes() as type (type.id)}
            <tr>
              <td class="manufacturer">{type.manufacturer}</td>
              <td class="model">{type.model}</td>
              <td class="designator">
                {#if type.type_designator}
                  <span class="badge">{type.type_designator}</span>
                {:else}
                  <span class="text-muted">—</span>
                {/if}
              </td>
              <td class="category">
                {#if type.category}
                  <span class="category-badge">{type.category}</span>
                {:else}
                  <span class="text-muted">—</span>
                {/if}
              </td>
              <td class="class">
                {type.class || '—'}
              </td>
              <td class="actions">
                <button onclick={() => openEditModal(type)} class="btn-icon" title="Edit">
                  ✏️
                </button>
                <button onclick={() => deleteAircraftType(type.id)} class="btn-icon" title="Delete">
                  🗑️
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>

      {#if filteredTypes().length === 0 && (searchQuery || filterCategory !== 'all')}
        <div class="no-results">
          No aircraft types match your search criteria.
        </div>
      {/if}
    </div>
  {/if}
</div>

{#if showModal}
  <div class="modal-overlay" onclick={() => showModal = false}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h3>{editingType ? 'Edit Aircraft Type' : 'Add New Aircraft Type'}</h3>
        <button onclick={() => showModal = false} class="btn-close">✕</button>
      </div>

      <div class="modal-body">
        <form onsubmit={(e) => { e.preventDefault(); saveAircraftType(); }}>
          <div class="form-grid">
            <div class="form-group">
              <label for="manufacturer">Manufacturer *</label>
              <input
                type="text"
                id="manufacturer"
                bind:value={formManufacturer}
                placeholder="Boeing"
                required
              />
            </div>

            <div class="form-group">
              <label for="model">Model *</label>
              <input
                type="text"
                id="model"
                bind:value={formModel}
                placeholder="737-800"
                required
              />
            </div>

            <div class="form-group">
              <label for="designator">Type Designator (ICAO)</label>
              <input
                type="text"
                id="designator"
                bind:value={formTypeDesignator}
                placeholder="B738"
                maxlength="4"
                style="text-transform: uppercase;"
              />
              <small>4-character ICAO code (e.g., B738, A320)</small>
            </div>

            <div class="form-group">
              <label for="category">Category</label>
              <input
                type="text"
                id="category"
                bind:value={formCategory}
                placeholder="Airplane"
                list="categories-list"
              />
              <datalist id="categories-list">
                <option value="Airplane"></option>
                <option value="Helicopter"></option>
                <option value="Glider"></option>
                <option value="Gyroplane"></option>
                <option value="Airship"></option>
              </datalist>
            </div>

            <div class="form-group">
              <label for="class">Class</label>
              <input
                type="text"
                id="class"
                bind:value={formClass}
                placeholder="Multi-Engine Land"
                list="classes-list"
              />
              <datalist id="classes-list">
                <option value="Single-Engine Land"></option>
                <option value="Single-Engine Sea"></option>
                <option value="Multi-Engine Land"></option>
                <option value="Multi-Engine Sea"></option>
              </datalist>
            </div>

            <div class="form-group full-width">
              <label for="notes">Notes</label>
              <textarea
                id="notes"
                bind:value={formNotes}
                placeholder="Additional information about this aircraft type..."
                rows="3"
              ></textarea>
            </div>
          </div>

          <div class="modal-footer">
            <button type="button" onclick={() => showModal = false} class="btn-secondary">
              Cancel
            </button>
            <button type="submit" class="btn-primary">
              {editingType ? 'Update' : 'Create'} Aircraft Type
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
{/if}

<style>
  .aircraft-types-container {
    padding: 1rem;
    max-width: 1400px;
    margin: 0 auto;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  .header h2 {
    margin: 0;
    font-size: 1.5rem;
  }

  .filters {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .search-input {
    flex: 1;
    padding: 0.75rem;
    border: 1px solid #475569;
    border-radius: 4px;
    font-size: 1rem;
    background: #0f172a;
    color: white;
  }

  .category-filter {
    padding: 0.75rem;
    border: 1px solid #475569;
    border-radius: 4px;
    font-size: 1rem;
    background: #0f172a;
    color: white;
    min-width: 200px;
  }

  .loading,
  .empty-state {
    text-align: center;
    padding: 3rem;
    color: #666;
  }

  .empty-state p {
    margin-bottom: 1rem;
  }

  .types-table {
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 8px;
    overflow: hidden;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  thead {
    background: #0f172a;
  }

  th {
    text-align: left;
    padding: 1rem;
    font-weight: 600;
    color: #e2e8f0;
    border-bottom: 2px solid #334155;
  }

  td {
    padding: 1rem;
    border-bottom: 1px solid #334155;
    color: #e2e8f0;
  }

  tr:hover {
    background: #334155;
  }

  .manufacturer,
  .model {
    font-weight: 500;
  }

  .badge {
    display: inline-block;
    padding: 0.25rem 0.5rem;
    background: #e3f2fd;
    color: #1976d2;
    border-radius: 4px;
    font-weight: bold;
    font-size: 0.85rem;
    font-family: monospace;
  }

  .category-badge {
    display: inline-block;
    padding: 0.25rem 0.75rem;
    background: #f3e5f5;
    color: #7b1fa2;
    border-radius: 12px;
    font-size: 0.85rem;
  }

  .text-muted {
    color: #999;
  }

  .actions {
    display: flex;
    gap: 0.25rem;
  }

  .no-results {
    text-align: center;
    padding: 2rem;
    color: #666;
  }

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: #1e293b;
    color: white;
    border-radius: 8px;
    width: 90%;
    max-width: 600px;
    max-height: 90vh;
    overflow-y: auto;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid #334155;
  }

  .modal-header h3 {
    margin: 0;
  }

  .modal-body {
    padding: 1.5rem;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .form-group {
    display: flex;
    flex-direction: column;
  }

  .form-group.full-width {
    grid-column: 1 / -1;
  }

  .form-group label {
    margin-bottom: 0.25rem;
    font-weight: 500;
    font-size: 0.9rem;
    color: #e2e8f0;
  }

  .form-group input,
  .form-group textarea {
    padding: 0.5rem;
    border: 1px solid #475569;
    border-radius: 4px;
    font-size: 1rem;
    font-family: inherit;
    background: #0f172a;
    color: white;
  }

  .form-group small {
    margin-top: 0.25rem;
    font-size: 0.8rem;
    color: #94a3b8;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.5rem;
  }

  .btn-primary,
  .btn-secondary,
  .btn-icon,
  .btn-close {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    transition: background-color 0.2s;
  }

  .btn-primary {
    background: #1976d2;
    color: white;
  }

  .btn-primary:hover {
    background: #1565c0;
  }

  .btn-secondary {
    background: #475569;
    color: white;
  }

  .btn-secondary:hover {
    background: #334155;
  }

  .btn-icon {
    background: transparent;
    padding: 0.25rem 0.5rem;
    font-size: 1rem;
  }

  .btn-icon:hover {
    background: #f5f5f5;
  }

  .btn-close {
    background: transparent;
    padding: 0.25rem 0.5rem;
    font-size: 1.25rem;
    color: white;
  }

  .btn-close:hover {
    background: #475569;
  }
</style>
