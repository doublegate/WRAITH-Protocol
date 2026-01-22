import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface AppState {
  // Identity
  peerId: string | null;
  displayName: string;
  isInitialized: boolean;

  // UI state
  isSettingsOpen: boolean;
  searchQuery: string;
  selectedCategory: string | null;

  // Actions
  initialize: () => Promise<void>;
  setDisplayName: (name: string) => Promise<void>;
  setSettingsOpen: (open: boolean) => void;
  setSearchQuery: (query: string) => void;
  setSelectedCategory: (category: string | null) => void;
}

export const useAppStore = create<AppState>((set, _get) => ({
  peerId: null,
  displayName: 'Anonymous',
  isInitialized: false,
  isSettingsOpen: false,
  searchQuery: '',
  selectedCategory: null,

  initialize: async () => {
    try {
      const peerId = await invoke<string>('get_peer_id');
      const displayName = await invoke<string>('get_display_name');
      set({ peerId, displayName, isInitialized: true });
    } catch (error) {
      console.error('Failed to initialize app:', error);
      set({ isInitialized: true }); // Still mark as initialized to show UI
    }
  },

  setDisplayName: async (name) => {
    try {
      await invoke('set_display_name', { name });
      set({ displayName: name });
    } catch (error) {
      console.error('Failed to set display name:', error);
      throw error;
    }
  },

  setSettingsOpen: (open) => set({ isSettingsOpen: open }),
  setSearchQuery: (query) => set({ searchQuery: query }),
  setSelectedCategory: (category) => set({ selectedCategory: category }),
}));
