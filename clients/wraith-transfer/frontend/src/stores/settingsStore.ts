// WRAITH Transfer - Settings State Store

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Theme = 'light' | 'dark' | 'system';

export interface Settings {
  theme: Theme;
  downloadDir: string;
  maxConcurrentTransfers: number;
  port: number;
  autoAcceptTransfers: boolean;
}

interface SettingsState extends Settings {
  // Actions
  setTheme: (theme: Theme) => void;
  setDownloadDir: (dir: string) => void;
  setMaxConcurrentTransfers: (max: number) => void;
  setPort: (port: number) => void;
  setAutoAcceptTransfers: (autoAccept: boolean) => void;
  resetToDefaults: () => void;
}

const DEFAULT_SETTINGS: Settings = {
  theme: 'system',
  downloadDir: '',
  maxConcurrentTransfers: 3,
  port: 8337,
  autoAcceptTransfers: false,
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...DEFAULT_SETTINGS,

      setTheme: (theme) => set({ theme }),
      setDownloadDir: (dir) => set({ downloadDir: dir }),
      setMaxConcurrentTransfers: (max) => set({ maxConcurrentTransfers: max }),
      setPort: (port) => set({ port }),
      setAutoAcceptTransfers: (autoAccept) => set({ autoAcceptTransfers: autoAccept }),
      resetToDefaults: () => set(DEFAULT_SETTINGS),
    }),
    {
      name: 'wraith-settings',
    }
  )
);
