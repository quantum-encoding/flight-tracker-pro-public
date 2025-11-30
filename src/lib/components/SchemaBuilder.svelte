<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { translations } from '$lib/i18n';
  import DataEditor from './DataEditor.svelte';

  interface SchemaField {
    id: string;
    name: string;
    display_name: string;
    field_type: string;
    is_required: boolean;
    default_value: string;
    options: string;
  }

  interface CustomSchema {
    id: string;
    name: string;
    display_name: string;
    description: string;
    icon: string;
    color: string;
    is_active: boolean;
  }

  let { userId }: { userId: string } = $props();

  // Main view: 'custom-schemas' for the schema builder, 'data-editor' for editing flight/passenger data
  let mainView = $state<'custom-schemas' | 'data-editor'>('custom-schemas');

  interface CustomRecord {
    id: string;
    schema_id: string;
    user_id: string;
    data: string;
    created_at: string;
    updated_at: string;
  }

  let schemas: CustomSchema[] = $state([]);
  let selectedSchema: CustomSchema | null = $state(null);
  let schemaFields: SchemaField[] = $state([]);
  let records: CustomRecord[] = $state([]);
  let loading = $state(false);
  let activeTab = $state<'schemas' | 'builder' | 'records'>('schemas');

  // Record modal state
  let showRecordModal = $state(false);
  let editingRecord: CustomRecord | null = $state(null);
  let recordFormData: Record<string, any> = $state({});

  // Builder state
  let newSchemaName = $state('');
  let newSchemaDisplayName = $state('');
  let newSchemaDescription = $state('');
  let newSchemaIcon = $state('database');
  let newSchemaColor = $state('#6366f1');
  let builderFields: SchemaField[] = $state([]);
  let draggedIndex: number | null = $state(null);

  const fieldTypes = [
    { value: 'text', label: 'Text', icon: '📝' },
    { value: 'number', label: 'Number', icon: '🔢' },
    { value: 'date', label: 'Date', icon: '📅' },
    { value: 'datetime', label: 'Date & Time', icon: '🕐' },
    { value: 'boolean', label: 'Yes/No', icon: '✓' },
    { value: 'select', label: 'Dropdown', icon: '📋' },
    { value: 'email', label: 'Email', icon: '📧' },
    { value: 'url', label: 'URL', icon: '🔗' },
    { value: 'currency', label: 'Currency', icon: '💰' },
    { value: 'textarea', label: 'Long Text', icon: '📄' },
  ];

  const icons = ['database', 'folder', 'box', 'truck', 'users', 'briefcase', 'plane', 'ship', 'car', 'tools', 'clipboard', 'archive'];
  const colors = ['#6366f1', '#8b5cf6', '#ec4899', '#f43f5e', '#f97316', '#eab308', '#22c55e', '#14b8a6', '#0ea5e9', '#3b82f6'];

  onMount(async () => {
    await loadSchemas();
  });

  async function loadSchemas() {
    loading = true;
    try {
      schemas = await invoke('list_custom_schemas', { userId });
    } catch (e) {
      console.error('Failed to load schemas:', e);
    }
    loading = false;
  }

  async function selectSchema(schema: CustomSchema) {
    selectedSchema = schema;
    try {
      schemaFields = await invoke('get_schema_fields', { schemaId: schema.id });
      await loadRecords(schema.id);
      activeTab = 'records';
    } catch (e) {
      console.error('Failed to load fields:', e);
    }
  }

  async function loadRecords(schemaId: string) {
    try {
      records = await invoke('list_custom_records', { schemaId });
    } catch (e) {
      console.error('Failed to load records:', e);
      records = [];
    }
  }

  function openAddRecord() {
    editingRecord = null;
    recordFormData = {};
    schemaFields.forEach(f => {
      recordFormData[f.name] = f.default_value || '';
    });
    showRecordModal = true;
  }

  function openEditRecord(record: CustomRecord) {
    editingRecord = record;
    try {
      recordFormData = JSON.parse(record.data);
    } catch {
      recordFormData = {};
    }
    showRecordModal = true;
  }

  async function saveRecord() {
    if (!selectedSchema) return;
    loading = true;
    try {
      const dataJson = JSON.stringify(recordFormData);
      if (editingRecord) {
        await invoke('update_custom_record', { recordId: editingRecord.id, data: dataJson });
      } else {
        await invoke('create_custom_record', { schemaId: selectedSchema.id, userId, data: dataJson });
      }
      await loadRecords(selectedSchema.id);
      showRecordModal = false;
    } catch (e) {
      console.error('Failed to save record:', e);
      alert('Failed to save: ' + e);
    }
    loading = false;
  }

  async function deleteRecord(record: CustomRecord) {
    if (!confirm('Delete this record?')) return;
    if (!selectedSchema) return;
    try {
      await invoke('delete_custom_record', { recordId: record.id });
      await loadRecords(selectedSchema.id);
    } catch (e) {
      console.error('Failed to delete:', e);
    }
  }

  function getFieldValue(record: CustomRecord, fieldName: string): string {
    try {
      const data = JSON.parse(record.data);
      return data[fieldName] ?? '';
    } catch {
      return '';
    }
  }

  function addField() {
    const id = crypto.randomUUID();
    builderFields = [...builderFields, {
      id,
      name: '',
      display_name: '',
      field_type: 'text',
      is_required: false,
      default_value: '',
      options: ''
    }];
  }

  function removeField(index: number) {
    builderFields = builderFields.filter((_, i) => i !== index);
  }

  function updateFieldName(index: number, displayName: string) {
    builderFields[index].display_name = displayName;
    builderFields[index].name = displayName.toLowerCase().replace(/[^a-z0-9]/g, '_');
  }

  function handleDragStart(index: number) {
    draggedIndex = index;
  }

  function handleDragOver(e: DragEvent, index: number) {
    e.preventDefault();
    if (draggedIndex === null || draggedIndex === index) return;

    const newFields = [...builderFields];
    const [removed] = newFields.splice(draggedIndex, 1);
    newFields.splice(index, 0, removed);
    builderFields = newFields;
    draggedIndex = index;
  }

  function handleDragEnd() {
    draggedIndex = null;
  }

  async function createSchema() {
    if (!newSchemaDisplayName.trim() || builderFields.length === 0) {
      alert('Please provide a name and at least one field');
      return;
    }

    const schemaName = newSchemaDisplayName.toLowerCase().replace(/[^a-z0-9]/g, '_');

    loading = true;
    try {
      await invoke('create_custom_schema', {
        userId,
        input: {
          name: schemaName,
          display_name: newSchemaDisplayName,
          description: newSchemaDescription || null,
          icon: newSchemaIcon,
          color: newSchemaColor,
          fields: builderFields.map(f => ({
            name: f.name,
            display_name: f.display_name,
            field_type: f.field_type,
            is_required: f.is_required,
            default_value: f.default_value || null,
            options: f.options || null,
            validation_rules: null
          }))
        }
      });

      // Reset and reload
      newSchemaName = '';
      newSchemaDisplayName = '';
      newSchemaDescription = '';
      builderFields = [];
      await loadSchemas();
      activeTab = 'schemas';
    } catch (e) {
      console.error('Failed to create schema:', e);
      alert('Failed to create schema: ' + e);
    }
    loading = false;
  }

  async function deleteSchema(schema: CustomSchema) {
    if (!confirm(`Delete "${schema.display_name}"? This will hide the schema but preserve data.`)) return;

    try {
      await invoke('delete_custom_schema', { schemaId: schema.id });
      await loadSchemas();
      if (selectedSchema?.id === schema.id) {
        selectedSchema = null;
      }
    } catch (e) {
      console.error('Failed to delete schema:', e);
    }
  }

  function startNewSchema() {
    newSchemaName = '';
    newSchemaDisplayName = '';
    newSchemaDescription = '';
    newSchemaIcon = 'database';
    newSchemaColor = '#6366f1';
    builderFields = [];
    activeTab = 'builder';
  }
</script>

<div class="h-full flex flex-col overflow-hidden">
  <!-- Main View Toggle -->
  <div class="px-6 pt-4 pb-2 border-b border-gray-200 dark:border-gray-700 flex-shrink-0">
    <div class="flex gap-2">
      <button
        onclick={() => mainView = 'data-editor'}
        class="px-4 py-2 rounded-lg text-sm font-medium transition {mainView === 'data-editor'
          ? 'bg-indigo-600 text-white'
          : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'}"
      >
        Data Editor
      </button>
      <button
        onclick={() => mainView = 'custom-schemas'}
        class="px-4 py-2 rounded-lg text-sm font-medium transition {mainView === 'custom-schemas'
          ? 'bg-indigo-600 text-white'
          : 'bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-600'}"
      >
        Custom Schemas
      </button>
    </div>
  </div>

  {#if mainView === 'data-editor'}
    <DataEditor {userId} />
  {:else}
    <div class="p-6 space-y-6 flex-1 overflow-auto">
      <div class="flex items-center justify-between">
        <h2 class="text-2xl font-bold text-gray-900 dark:text-white">{$translations('schemaBuilder.title')}</h2>
        <button
          onclick={startNewSchema}
          class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition flex items-center gap-2"
        >
          <span>+</span> {$translations('schemaBuilder.newSchema')}
        </button>
      </div>

      <!-- Tabs -->
      <div class="border-b border-gray-200 dark:border-gray-700">
        <nav class="flex gap-4">
          <button
            onclick={() => activeTab = 'schemas'}
            class="pb-2 px-1 text-sm font-medium border-b-2 transition {activeTab === 'schemas' ? 'border-indigo-500 text-indigo-600' : 'border-transparent text-gray-500 hover:text-gray-700'}"
          >
            {$translations('schemaBuilder.tabs.schemas')}
          </button>
          <button
            onclick={() => activeTab = 'builder'}
            class="pb-2 px-1 text-sm font-medium border-b-2 transition {activeTab === 'builder' ? 'border-indigo-500 text-indigo-600' : 'border-transparent text-gray-500 hover:text-gray-700'}"
          >
            {$translations('schemaBuilder.tabs.builder')}
          </button>
          {#if selectedSchema}
            <button
              onclick={() => activeTab = 'records'}
              class="pb-2 px-1 text-sm font-medium border-b-2 transition {activeTab === 'records' ? 'border-indigo-500 text-indigo-600' : 'border-transparent text-gray-500 hover:text-gray-700'}"
            >
              {selectedSchema.display_name} {$translations('schemaBuilder.tabs.records')}
            </button>
          {/if}
        </nav>
      </div>

  <!-- Schemas List -->
  {#if activeTab === 'schemas'}
    {#if loading}
      <div class="text-center py-8 text-gray-500">{$translations('common.loading')}</div>
    {:else if schemas.length === 0}
      <div class="text-center py-12 text-gray-500">
        <div class="text-4xl mb-4">🗄️</div>
        <p class="mb-4">{$translations('schemaBuilder.schema.noSchemas')}</p>
        <button onclick={startNewSchema} class="text-indigo-600 hover:underline">
          {$translations('schemaBuilder.schema.createFirst')}
        </button>
      </div>
    {:else}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        {#each schemas as schema}
          <div
            class="p-4 rounded-lg border-2 cursor-pointer transition hover:shadow-lg"
            style="border-color: {schema.color}20; background: {schema.color}10"
            onclick={() => selectSchema(schema)}
          >
            <div class="flex items-center gap-3 mb-2">
              <span class="text-2xl">{schema.icon === 'database' ? '🗄️' : schema.icon === 'plane' ? '✈️' : schema.icon === 'truck' ? '🚚' : '📁'}</span>
              <div>
                <h3 class="font-semibold text-gray-900 dark:text-white">{schema.display_name}</h3>
                <p class="text-xs text-gray-500">{schema.name}</p>
              </div>
            </div>
            {#if schema.description}
              <p class="text-sm text-gray-600 dark:text-gray-400 mb-3">{schema.description}</p>
            {/if}
            <div class="flex justify-end">
              <button
                onclick={(e) => { e.stopPropagation(); deleteSchema(schema); }}
                class="text-xs text-red-500 hover:text-red-700"
              >
                {$translations('schemaBuilder.schema.delete')}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  <!-- Schema Builder -->
  {#if activeTab === 'builder'}
    <div class="space-y-6">
      <!-- Schema Info -->
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6 shadow">
        <h3 class="font-semibold mb-4 text-gray-900 dark:text-white">Schema Details</h3>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{$translations('schemaBuilder.schema.displayName')} *</label>
            <input
              type="text"
              bind:value={newSchemaDisplayName}
              placeholder="e.g. Cargo Manifest"
              class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            />
          </div>
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{$translations('schemaBuilder.schema.description')}</label>
            <input
              type="text"
              bind:value={newSchemaDescription}
              placeholder="Track cargo shipments..."
              class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            />
          </div>
        </div>
        <div class="mt-4 flex gap-6">
          <div>
            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{$translations('schemaBuilder.schema.color')}</label>
            <div class="flex gap-2">
              {#each colors as color}
                <button
                  onclick={() => newSchemaColor = color}
                  class="w-6 h-6 rounded-full transition {newSchemaColor === color ? 'ring-2 ring-offset-2 ring-gray-400' : ''}"
                  style="background: {color}"
                ></button>
              {/each}
            </div>
          </div>
        </div>
      </div>

      <!-- Fields -->
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6 shadow">
        <div class="flex items-center justify-between mb-4">
          <h3 class="font-semibold text-gray-900 dark:text-white">{$translations('schemaBuilder.fields.title')}</h3>
          <button
            onclick={addField}
            class="px-3 py-1 text-sm bg-gray-100 dark:bg-gray-700 rounded hover:bg-gray-200 dark:hover:bg-gray-600 transition"
          >
            + {$translations('schemaBuilder.fields.add')}
          </button>
        </div>

        {#if builderFields.length === 0}
          <div class="text-center py-8 text-gray-500 border-2 border-dashed rounded-lg">
            <p>{$translations('schemaBuilder.fields.noFields')}</p>
          </div>
        {:else}
          <div class="space-y-3">
            {#each builderFields as field, i}
              <div
                class="p-4 bg-gray-50 dark:bg-gray-700 rounded-lg border-l-4 transition-all {draggedIndex === i ? 'opacity-50' : ''}"
                style="border-color: {newSchemaColor}"
                draggable="true"
                ondragstart={() => handleDragStart(i)}
                ondragover={(e) => handleDragOver(e, i)}
                ondragend={handleDragEnd}
              >
                <div class="flex items-start gap-4">
                  <span class="cursor-move text-gray-400 pt-2">⋮⋮</span>

                  <div class="flex-1 grid grid-cols-1 md:grid-cols-4 gap-3">
                    <div>
                      <label class="block text-xs text-gray-500 mb-1">{$translations('schemaBuilder.fields.name')}</label>
                      <input
                        type="text"
                        value={field.display_name}
                        oninput={(e) => updateFieldName(i, e.currentTarget.value)}
                        placeholder="Field name"
                        class="w-full px-2 py-1.5 text-sm border rounded dark:bg-gray-600 dark:border-gray-500 dark:text-white"
                      />
                    </div>

                    <div>
                      <label class="block text-xs text-gray-500 mb-1">{$translations('schemaBuilder.fields.type')}</label>
                      <select
                        bind:value={field.field_type}
                        class="w-full px-2 py-1.5 text-sm border rounded dark:bg-gray-600 dark:border-gray-500 dark:text-white"
                      >
                        {#each fieldTypes as ft}
                          <option value={ft.value}>{ft.icon} {ft.label}</option>
                        {/each}
                      </select>
                    </div>

                    <div>
                      <label class="block text-xs text-gray-500 mb-1">{$translations('schemaBuilder.fields.default')}</label>
                      <input
                        type="text"
                        bind:value={field.default_value}
                        placeholder="Default value"
                        class="w-full px-2 py-1.5 text-sm border rounded dark:bg-gray-600 dark:border-gray-500 dark:text-white"
                      />
                    </div>

                    <div class="flex items-end gap-2">
                      <label class="flex items-center gap-2 text-sm">
                        <input type="checkbox" bind:checked={field.is_required} class="rounded" />
                        {$translations('schemaBuilder.fields.required')}
                      </label>
                      <button
                        onclick={() => removeField(i)}
                        class="ml-auto text-red-500 hover:text-red-700 p-1"
                      >
                        ✕
                      </button>
                    </div>
                  </div>
                </div>

                {#if field.field_type === 'select'}
                  <div class="mt-2 ml-8">
                    <label class="block text-xs text-gray-500 mb-1">{$translations('schemaBuilder.fields.options')}</label>
                    <input
                      type="text"
                      bind:value={field.options}
                      placeholder="Option 1, Option 2, Option 3"
                      class="w-full px-2 py-1.5 text-sm border rounded dark:bg-gray-600 dark:border-gray-500 dark:text-white"
                    />
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Create Button -->
      <div class="flex justify-end">
        <button
          onclick={createSchema}
          disabled={loading || !newSchemaDisplayName.trim() || builderFields.length === 0}
          class="px-6 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:opacity-50 disabled:cursor-not-allowed transition"
        >
          {loading ? $translations('schemaBuilder.actions.creating') : $translations('schemaBuilder.actions.create')}
        </button>
      </div>
    </div>
  {/if}

  <!-- Records View -->
  {#if activeTab === 'records' && selectedSchema}
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 shadow">
      <div class="flex items-center justify-between mb-4">
        <h3 class="font-semibold text-gray-900 dark:text-white">{selectedSchema.display_name}</h3>
        <button
          onclick={openAddRecord}
          class="px-3 py-1 text-sm bg-indigo-600 text-white rounded hover:bg-indigo-700 transition"
        >
          + {$translations('schemaBuilder.records.add')}
        </button>
      </div>

      {#if records.length === 0}
        <div class="text-center py-12 text-gray-500 border-2 border-dashed rounded-lg">
          <div class="text-3xl mb-2">📝</div>
          <p>{$translations('schemaBuilder.records.noRecords')}</p>
          <button onclick={openAddRecord} class="mt-2 text-indigo-600 hover:underline">
            {$translations('schemaBuilder.records.addFirst')}
          </button>
        </div>
      {:else}
        <div class="overflow-x-auto">
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b dark:border-gray-700">
                {#each schemaFields.slice(0, 5) as field}
                  <th class="text-left py-2 px-3 font-medium text-gray-700 dark:text-gray-300">
                    {field.display_name}
                  </th>
                {/each}
                <th class="text-right py-2 px-3 font-medium text-gray-700 dark:text-gray-300">{$translations('common.actions')}</th>
              </tr>
            </thead>
            <tbody>
              {#each records as record}
                <tr class="border-b dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700">
                  {#each schemaFields.slice(0, 5) as field}
                    <td class="py-2 px-3 text-gray-900 dark:text-white">
                      {#if field.field_type === 'boolean'}
                        {getFieldValue(record, field.name) === 'true' ? '✓' : '✗'}
                      {:else if field.field_type === 'currency'}
                        ${getFieldValue(record, field.name)}
                      {:else}
                        {getFieldValue(record, field.name) || '-'}
                      {/if}
                    </td>
                  {/each}
                  <td class="py-2 px-3 text-right">
                    <button
                      onclick={() => openEditRecord(record)}
                      class="text-indigo-600 hover:text-indigo-800 mr-2"
                    >
                      {$translations('schemaBuilder.records.edit')}
                    </button>
                    <button
                      onclick={() => deleteRecord(record)}
                      class="text-red-500 hover:text-red-700"
                    >
                      {$translations('schemaBuilder.records.delete')}
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <div class="mt-4 text-sm text-gray-500">
          {records.length} record{records.length !== 1 ? 's' : ''}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Record Modal -->
  {#if showRecordModal && selectedSchema}
    <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div class="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-lg max-h-[80vh] overflow-y-auto">
        <div class="p-4 border-b dark:border-gray-700 flex items-center justify-between">
          <h3 class="font-semibold text-gray-900 dark:text-white">
            {editingRecord ? 'Edit' : 'Add'} {selectedSchema.display_name}
          </h3>
          <button onclick={() => showRecordModal = false} class="text-gray-500 hover:text-gray-700">✕</button>
        </div>
        <div class="p-4 space-y-4">
          {#each schemaFields as field}
            <div>
              <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                {field.display_name}
                {#if field.is_required}<span class="text-red-500">*</span>{/if}
              </label>

              {#if field.field_type === 'textarea'}
                <textarea
                  bind:value={recordFormData[field.name]}
                  rows="3"
                  class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                ></textarea>
              {:else if field.field_type === 'boolean'}
                <label class="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={recordFormData[field.name] === 'true'}
                    onchange={(e) => recordFormData[field.name] = e.currentTarget.checked ? 'true' : 'false'}
                    class="rounded"
                  />
                  <span class="text-sm text-gray-600 dark:text-gray-400">Yes</span>
                </label>
              {:else if field.field_type === 'select' && field.options}
                <select
                  bind:value={recordFormData[field.name]}
                  class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                >
                  <option value="">Select...</option>
                  {#each field.options.split(',').map(o => o.trim()) as opt}
                    <option value={opt}>{opt}</option>
                  {/each}
                </select>
              {:else if field.field_type === 'date'}
                <input
                  type="date"
                  bind:value={recordFormData[field.name]}
                  class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
              {:else if field.field_type === 'datetime'}
                <input
                  type="datetime-local"
                  bind:value={recordFormData[field.name]}
                  class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
              {:else if field.field_type === 'number' || field.field_type === 'currency'}
                <input
                  type="number"
                  step={field.field_type === 'currency' ? '0.01' : '1'}
                  bind:value={recordFormData[field.name]}
                  class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
              {:else if field.field_type === 'email'}
                <input
                  type="email"
                  bind:value={recordFormData[field.name]}
                  class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
              {:else if field.field_type === 'url'}
                <input
                  type="url"
                  bind:value={recordFormData[field.name]}
                  placeholder="https://"
                  class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
              {:else}
                <input
                  type="text"
                  bind:value={recordFormData[field.name]}
                  class="w-full px-3 py-2 border rounded-lg dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
              {/if}
            </div>
          {/each}
        </div>
        <div class="p-4 border-t dark:border-gray-700 flex justify-end gap-2">
          <button
            onclick={() => showRecordModal = false}
            class="px-4 py-2 text-gray-600 hover:text-gray-800 dark:text-gray-400"
          >
            {$translations('common.cancel')}
          </button>
          <button
            onclick={saveRecord}
            disabled={loading}
            class="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:opacity-50"
          >
            {loading ? $translations('schemaBuilder.actions.saving') : $translations('schemaBuilder.actions.save')}
          </button>
        </div>
      </div>
    </div>
  {/if}
    </div>
  {/if}
</div>
