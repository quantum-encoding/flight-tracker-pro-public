import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

export const developerMode = writable<boolean>(false);

export async function initDeveloperMode(): Promise<void> {
  try {
    const devMode = await invoke('get_setting', { key: 'developer_mode' });
    developerMode.set(devMode === 'true');
  } catch (error) {
    console.error('Failed to load developer mode:', error);
    developerMode.set(false);
  }
}

export async function setDeveloperMode(enabled: boolean): Promise<void> {
  try {
    await invoke('set_setting', { key: 'developer_mode', value: enabled ? 'true' : 'false' });
    developerMode.set(enabled);
  } catch (error) {
    console.error('Failed to save developer mode:', error);
  }
}

// User location store for network visualization
export interface UserLocation {
  lat: number;
  lng: number;
  city?: string;
  country?: string;
  countryCode?: string;
  source: 'auto' | 'manual' | 'default';
}

const defaultLocation: UserLocation = {
  lat: 40.4168,
  lng: -3.7038,
  city: 'Madrid',
  country: 'Spain',
  countryCode: 'ES',
  source: 'default'
};

export const userLocation = writable<UserLocation>(defaultLocation);
export const locationLoading = writable<boolean>(false);
export const locationError = writable<string | null>(null);

// Initialize location from saved settings or detect from IP
export async function initUserLocation(): Promise<void> {
  locationLoading.set(true);
  locationError.set(null);

  try {
    // First check if user has manually set a location
    const savedLat = await invoke<string | null>('get_setting', { key: 'user_location_lat' });
    const savedLng = await invoke<string | null>('get_setting', { key: 'user_location_lng' });
    const savedCity = await invoke<string | null>('get_setting', { key: 'user_location_city' });
    const savedCountry = await invoke<string | null>('get_setting', { key: 'user_location_country' });
    const savedSource = await invoke<string | null>('get_setting', { key: 'user_location_source' });

    if (savedLat && savedLng && savedSource === 'manual') {
      userLocation.set({
        lat: parseFloat(savedLat),
        lng: parseFloat(savedLng),
        city: savedCity || undefined,
        country: savedCountry || undefined,
        source: 'manual'
      });
      locationLoading.set(false);
      return;
    }

    // Otherwise, try to detect from IP
    await detectLocationFromIP();
  } catch (error) {
    console.error('Failed to initialize user location:', error);
    locationError.set('Using default location');
    userLocation.set(defaultLocation);
  } finally {
    locationLoading.set(false);
  }
}

// Detect location from IP address
export async function detectLocationFromIP(): Promise<void> {
  locationLoading.set(true);
  locationError.set(null);

  try {
    const result = await invoke<{
      lat: number;
      lng: number;
      city?: string;
      country?: string;
      country_code?: string;
    }>('detect_location_from_ip');

    const location: UserLocation = {
      lat: result.lat,
      lng: result.lng,
      city: result.city,
      country: result.country,
      countryCode: result.country_code,
      source: 'auto'
    };

    userLocation.set(location);

    // Save to settings
    await invoke('set_setting', { key: 'user_location_lat', value: result.lat.toString() });
    await invoke('set_setting', { key: 'user_location_lng', value: result.lng.toString() });
    await invoke('set_setting', { key: 'user_location_city', value: result.city || '' });
    await invoke('set_setting', { key: 'user_location_country', value: result.country || '' });
    await invoke('set_setting', { key: 'user_location_source', value: 'auto' });

    console.log('Location detected:', location);
  } catch (error) {
    console.error('Failed to detect location from IP:', error);
    locationError.set('Could not detect location. Using default.');
    // Keep current location or use default
    if (get(userLocation).source === 'default') {
      userLocation.set(defaultLocation);
    }
  } finally {
    locationLoading.set(false);
  }
}

// Manually set user location
export async function setUserLocation(lat: number, lng: number, city?: string, country?: string): Promise<void> {
  const location: UserLocation = {
    lat,
    lng,
    city,
    country,
    source: 'manual'
  };

  userLocation.set(location);

  try {
    await invoke('set_setting', { key: 'user_location_lat', value: lat.toString() });
    await invoke('set_setting', { key: 'user_location_lng', value: lng.toString() });
    await invoke('set_setting', { key: 'user_location_city', value: city || '' });
    await invoke('set_setting', { key: 'user_location_country', value: country || '' });
    await invoke('set_setting', { key: 'user_location_source', value: 'manual' });
  } catch (error) {
    console.error('Failed to save user location:', error);
  }
}

// Reset to auto-detected location
export async function resetToAutoLocation(): Promise<void> {
  await detectLocationFromIP();
}
