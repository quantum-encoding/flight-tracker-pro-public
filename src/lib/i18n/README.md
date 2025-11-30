# i18n (Internationalization) System

Modular translation system for Flight Tracker Pro with JSON language files.

## Directory Structure

```
src/lib/i18n/
├── index.ts              # Core i18n utilities and stores
├── locales/
│   ├── en.json          # English translations (complete)
│   ├── template.json    # Blank template for new languages
│   ├── es.json          # Spanish (add as needed)
│   ├── fr.json          # French (add as needed)
│   └── ...              # Add more languages
└── README.md            # This file
```

## Usage in Components

### Option 1: Direct translation function

```svelte
<script lang="ts">
  import { t } from '$lib/i18n';

  // Initialize i18n (usually done once in root layout)
  import { initI18n } from '$lib/i18n';
  initI18n();
</script>

<h1>{t('analytics.title')}</h1>
<button>{t('common.save')}</button>
```

### Option 2: Reactive translation store

```svelte
<script lang="ts">
  import { createTranslation } from '$lib/i18n';

  const title = createTranslation('analytics.title');
  const saveBtn = createTranslation('common.save');
</script>

<h1>{$title}</h1>
<button>{$saveBtn}</button>
```

### Option 3: With interpolation

```svelte
<script lang="ts">
  import { t } from '$lib/i18n';

  const count = 5;
</script>

<p>{t('flights.import.success', { count: count })}</p>
<!-- Output: "Successfully imported 5 flights" -->
```

## Translation Key Structure

Translations are organized hierarchically by feature:

- `common.*` - Shared UI elements (buttons, labels, etc.)
- `navigation.*` - Navigation menu items
- `analytics.*` - Analytics dashboard and charts
- `flights.*` - Flight management
- `logbook.*` - Pilot logbook
- `passengers.*` - Passenger management
- `journeys.*` - Journey tracking
- `airports.*` - Airport information
- `aircraft.*` - Aircraft types
- `frequentFlyer.*` - Frequent flyer programs
- `documents.*` - Document management
- `research.*` - Research tools
- `settings.*` - Application settings
- `errors.*` - Error messages
- `validation.*` - Form validation messages
- `time.*` - Time-related labels
- `units.*` - Measurement units

## Adding a New Language

### Step 1: Copy the template

```bash
cp src/lib/i18n/locales/template.json src/lib/i18n/locales/es.json
```

### Step 2: Fill in translations

Edit `es.json` and replace empty strings with Spanish translations.

You can use AI agents for this:
```
"Translate the following English JSON to Spanish, maintaining the exact structure and keys: [paste template.json content]"
```

### Step 3: Update supported locales

Edit `src/lib/i18n/index.ts`:

```typescript
export const SUPPORTED_LOCALES = ['en', 'es'] as const;
```

### Step 4: Test

```typescript
import { setLocale } from '$lib/i18n';
await setLocale('es');
```

## Language Switcher Component

Create a language switcher in your UI:

```svelte
<script lang="ts">
  import { locale, SUPPORTED_LOCALES, setLocale } from '$lib/i18n';

  async function handleChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    await setLocale(target.value as any);
  }
</script>

<select value={$locale} onchange={handleChange}>
  {#each SUPPORTED_LOCALES as lang}
    <option value={lang}>{lang.toUpperCase()}</option>
  {/each}
</select>
```

## Best Practices

1. **Use descriptive keys**: `analytics.charts.temporal.title` instead of `t1`
2. **Group related translations**: Keep all analytics keys under `analytics.*`
3. **Use interpolation for dynamic content**: `t('validation.min', { min: 5 })`
4. **Keep template.json in sync**: When adding new keys to `en.json`, add them to `template.json` too
5. **One language per file**: Each locale gets its own JSON file
6. **Initialize once**: Call `initI18n()` in your root layout component

## Interpolation Syntax

Use `{variableName}` in translation strings:

```json
{
  "validation": {
    "minLength": "Minimum length is {min} characters",
    "range": "Value must be between {min} and {max}"
  }
}
```

```typescript
t('validation.minLength', { min: 5 })
// Output: "Minimum length is 5 characters"

t('validation.range', { min: 1, max: 100 })
// Output: "Value must be between 1 and 100"
```

## Fallback Behavior

- If a translation key is not found, the key itself is returned
- If a locale is not supported, English ('en') is used as fallback
- Missing interpolation parameters are left as `{paramName}` in the output

## Example: Full Component Integration

```svelte
<script lang="ts">
  import { t } from '$lib/i18n';
  import { invoke } from '@tauri-apps/api/core';

  let loading = $state(false);
  let error = $state<string | null>(null);

  async function loadData() {
    loading = true;
    error = null;
    try {
      await invoke('some_command');
    } catch (err) {
      error = t('errors.serverError');
    } finally {
      loading = false;
    }
  }
</script>

<div>
  <h1>{t('analytics.title')}</h1>

  {#if loading}
    <p>{t('common.loading')}</p>
  {:else if error}
    <div class="error">
      <p>{error}</p>
      <button onclick={loadData}>{t('common.retry')}</button>
    </div>
  {:else}
    <button onclick={loadData}>{t('analytics.loadButton')}</button>
  {/if}
</div>
```

## Future Enhancements

- [ ] Add plural forms support (e.g., "1 flight" vs "5 flights")
- [ ] Add date/time formatting per locale
- [ ] Add number formatting per locale
- [ ] Add RTL (right-to-left) language support
- [ ] Add translation validation tool
- [ ] Add missing translation detector
