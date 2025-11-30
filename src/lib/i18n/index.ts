/**
 * i18n (Internationalization) System
 *
 * Modular translation system with JSON language files.
 * Each language is a separate JSON file in the locales directory.
 */

import { writable, derived } from 'svelte/store';

// Supported locales
export const SUPPORTED_LOCALES = ['en', 'es', 'de'] as const;
export type Locale = typeof SUPPORTED_LOCALES[number];

// Translation dictionary type
export type TranslationDict = Record<string, any>;

// Current locale store
const localeStore = writable<Locale>('en');

// Translations store
const translationsStore = writable<Record<Locale, TranslationDict>>({
  en: {},
  es: {},
  de: {}
});

// Load translation file for a specific locale
export async function loadLocale(locale: Locale): Promise<void> {
  try {
    const translations = await import(`./locales/${locale}.json`);
    translationsStore.update(store => ({
      ...store,
      [locale]: translations.default
    }));
  } catch (error) {
    console.error(`Failed to load locale: ${locale}`, error);
  }
}

// Set current locale
export async function setLocale(locale: Locale): Promise<void> {
  if (!SUPPORTED_LOCALES.includes(locale)) {
    console.warn(`Locale ${locale} not supported. Using 'en' as fallback.`);
    locale = 'en';
  }

  await loadLocale(locale);
  localeStore.set(locale);

  // Persist to localStorage
  if (typeof window !== 'undefined') {
    localStorage.setItem('locale', locale);
  }
}

// Get translation by key path (e.g., "analytics.charts.temporal.title")
function getNestedTranslation(obj: any, path: string): string {
  const keys = path.split('.');
  let current = obj;

  for (const key of keys) {
    if (current && typeof current === 'object' && key in current) {
      current = current[key];
    } else {
      return path; // Return key if not found (fallback)
    }
  }

  return typeof current === 'string' ? current : path;
}

// Translation function with interpolation support
export function translate(key: string, params?: Record<string, string | number>): string {
  let translation = key;

  const unsubscribe = derived(
    [localeStore, translationsStore],
    ([$locale, $translations]) => {
      const dict = $translations[$locale] || {};
      return getNestedTranslation(dict, key);
    }
  ).subscribe(value => {
    translation = value;
  });

  unsubscribe();

  // Handle interpolation: "Hello {name}" with params: { name: "World" }
  if (params) {
    Object.entries(params).forEach(([paramKey, paramValue]) => {
      translation = translation.replace(new RegExp(`\\{${paramKey}\\}`, 'g'), String(paramValue));
    });
  }

  return translation;
}

// Shorthand alias
export const t = translate;

// Reactive translation store (for use in Svelte components)
export const locale = {
  subscribe: localeStore.subscribe,
  set: setLocale
};

// Get current locale value
export function getCurrentLocale(): Locale {
  let current: Locale = 'en';
  localeStore.subscribe(value => { current = value; })();
  return current;
}

// Initialize i18n system
export async function initI18n(): Promise<void> {
  // Check localStorage for saved locale
  let savedLocale: Locale = 'en';

  if (typeof window !== 'undefined') {
    const stored = localStorage.getItem('locale') as Locale;
    if (stored && SUPPORTED_LOCALES.includes(stored)) {
      savedLocale = stored;
    }
  }

  await setLocale(savedLocale);
}

// Derived store for reactive translations in components
export function createTranslation(key: string) {
  return derived(
    [localeStore, translationsStore],
    ([$locale, $translations]) => {
      const dict = $translations[$locale] || {};
      return getNestedTranslation(dict, key);
    }
  );
}

// Reactive translation store for use in Svelte templates
// Usage: $t('common.loading') in template
export const translations = derived(
  [localeStore, translationsStore],
  ([$locale, $translations]) => {
    const dict = $translations[$locale] || {};
    return (key: string, params?: Record<string, string | number>) => {
      let translation = getNestedTranslation(dict, key);

      // Handle interpolation
      if (params) {
        Object.entries(params).forEach(([paramKey, paramValue]) => {
          translation = translation.replace(new RegExp(`\\{${paramKey}\\}`, 'g'), String(paramValue));
        });
      }

      return translation;
    };
  }
);
