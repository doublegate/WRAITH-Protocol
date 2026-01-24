// WRAITH Recon - Settings Store

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { AppSettings } from '../types';

interface SettingsState extends AppSettings {
  // Actions
  setTheme: (theme: 'light' | 'dark' | 'system') => void;
  setNotificationsEnabled: (enabled: boolean) => void;
  setRefreshInterval: (ms: number) => void;
  resetSettings: () => void;
}

const defaultSettings: AppSettings = {
  theme: 'dark',
  notificationsEnabled: true,
  refreshIntervalMs: 1000,
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...defaultSettings,

      setTheme: (theme) => set({ theme }),

      setNotificationsEnabled: (notificationsEnabled) => set({ notificationsEnabled }),

      setRefreshInterval: (refreshIntervalMs) => set({ refreshIntervalMs }),

      resetSettings: () => set(defaultSettings),
    }),
    {
      name: 'wraith-recon-settings',
    }
  )
);
