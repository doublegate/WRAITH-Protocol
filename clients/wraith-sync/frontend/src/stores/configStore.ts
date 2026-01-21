// Config Store (Zustand) - Settings and configuration management

import { create } from 'zustand';
import type { AppSettings, DeviceInfo } from '../types';
import * as tauri from '../lib/tauri';

interface ConfigState {
  // Settings
  settings: AppSettings | null;
  loading: boolean;
  error: string | null;

  // Devices
  devices: DeviceInfo[];

  // Ignored patterns
  globalPatterns: string[];
  folderPatterns: Map<number, string[]>;

  // Actions - Settings
  loadSettings: () => Promise<void>;
  updateSettings: (settings: AppSettings) => Promise<void>;
  updateSetting: <K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K]
  ) => Promise<void>;

  // Actions - Devices
  loadDevices: () => Promise<void>;
  removeDevice: (deviceId: string) => Promise<void>;

  // Actions - Ignored Patterns
  loadIgnoredPatterns: (folderId?: number) => Promise<void>;
  addIgnoredPattern: (pattern: string, folderId?: number) => Promise<void>;
}

export const useConfigStore = create<ConfigState>((set, get) => ({
  // Initial state
  settings: null,
  loading: false,
  error: null,
  devices: [],
  globalPatterns: [],
  folderPatterns: new Map(),

  // Settings actions
  loadSettings: async () => {
    set({ loading: true, error: null });
    try {
      const settings = await tauri.getSettings();
      set({ settings, loading: false });
    } catch (error) {
      set({ error: (error as Error).message, loading: false });
    }
  },

  updateSettings: async (settings: AppSettings) => {
    try {
      await tauri.updateSettings(settings);
      set({ settings });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  updateSetting: async <K extends keyof AppSettings>(
    key: K,
    value: AppSettings[K]
  ) => {
    const currentSettings = get().settings;
    if (!currentSettings) return;

    const newSettings = { ...currentSettings, [key]: value };
    await get().updateSettings(newSettings);
  },

  // Device actions
  loadDevices: async () => {
    try {
      const devices = await tauri.listDevices();
      set({ devices });
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  removeDevice: async (deviceId: string) => {
    try {
      await tauri.removeDevice(deviceId);
      await get().loadDevices();
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  // Ignored patterns actions
  loadIgnoredPatterns: async (folderId?: number) => {
    try {
      const patterns = await tauri.getIgnoredPatterns(folderId);
      if (folderId === undefined) {
        set({ globalPatterns: patterns });
      } else {
        const folderPatterns = new Map(get().folderPatterns);
        folderPatterns.set(folderId, patterns);
        set({ folderPatterns });
      }
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },

  addIgnoredPattern: async (pattern: string, folderId?: number) => {
    try {
      await tauri.addIgnoredPattern(pattern, folderId);
      await get().loadIgnoredPatterns(folderId);
    } catch (error) {
      set({ error: (error as Error).message });
    }
  },
}));
