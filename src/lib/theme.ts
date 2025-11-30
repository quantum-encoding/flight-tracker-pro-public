import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export type Theme = 'light' | 'dark' | 'cyberpunk' | 'skynet';

export const SUPPORTED_THEMES: Theme[] = ['light', 'dark', 'cyberpunk', 'skynet'];

const themeStore = writable<Theme>('skynet');

export const theme = {
  subscribe: themeStore.subscribe,
  set: async (newTheme: Theme) => {
    themeStore.set(newTheme);

    // Apply theme to document
    if (typeof document !== 'undefined') {
      document.documentElement.setAttribute('data-theme', newTheme);

      // Also maintain dark mode class for compatibility
      if (newTheme === 'dark' || newTheme === 'cyberpunk' || newTheme === 'skynet') {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    }

    // Persist to backend
    try {
      await invoke('set_setting', { key: 'theme', value: newTheme });
    } catch (error) {
      console.error('Failed to save theme:', error);
    }
  }
};

export async function initTheme(): Promise<void> {
  try {
    // Try to load from backend
    const savedTheme = await invoke('get_setting', { key: 'theme' }) as string;
    if (savedTheme && SUPPORTED_THEMES.includes(savedTheme as Theme)) {
      await theme.set(savedTheme as Theme);
      return;
    }
  } catch (error) {
    console.error('Failed to load theme:', error);
  }

  // Fallback: default to skynet theme
  await theme.set('skynet');
}

export function getCurrentTheme(): Theme {
  let currentTheme: Theme = 'skynet';
  themeStore.subscribe(value => currentTheme = value)();
  return currentTheme;
}
